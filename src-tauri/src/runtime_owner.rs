//! Ownership is established before any domain loader or migration runs. This is the same guard
//! used by the legacy manager, so Electron's own single-instance lock is not the only protection.
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

pub struct RuntimeOwner {
    #[cfg(unix)]
    _activation: ActivationListener,
    // Fields drop in declaration order: remove the activation socket while the guard is held.
    _guard: codex_plus_core::ports::LoopbackPortGuard,
}

#[derive(Debug)]
pub enum OwnershipError {
    IsolationInvalid,
    AlreadyRunning,
    Unavailable,
}

impl OwnershipError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::IsolationInvalid => "IsolationInvalid",
            Self::AlreadyRunning => "AlreadyRunning",
            Self::Unavailable => "OwnershipUnavailable",
        }
    }
}

fn resolved_existing_path(path: &Path) -> Option<PathBuf> {
    if path.exists() {
        return path.canonicalize().ok();
    }
    let parent = resolved_existing_path(path.parent()?)?;
    Some(parent.join(path.file_name()?))
}

pub fn validate_isolation() -> Result<(), OwnershipError> {
    let Some(requested) = std::env::var_os("CODEX_MINUS_TEST_HOME") else {
        return Ok(());
    };
    let root = PathBuf::from(requested)
        .canonicalize()
        .map_err(|_| OwnershipError::IsolationInvalid)?;
    let settings = codex_plus_core::paths::default_settings_path();
    let codex = codex_plus_core::codex_sqlite::default_codex_home_dir();
    if resolved_existing_path(&settings) != Some(root.join(".codex-session-delete/settings.json"))
        || codex.canonicalize().ok() != Some(root.join(".codex"))
    {
        return Err(OwnershipError::IsolationInvalid);
    }
    Ok(())
}

pub fn acquire(focus: mpsc::Sender<()>) -> Result<RuntimeOwner, OwnershipError> {
    validate_isolation()?;
    let port = codex_plus_core::ports::manager_guard_port();
    // Port zero bypasses the pinned core's persistent exclusive lock.
    if port == 0 {
        return Err(OwnershipError::Unavailable);
    }
    let guard =
        codex_plus_core::ports::acquire_resilient_loopback_port_guard(port).map_err(|error| {
            if matches!(
                error.kind(),
                std::io::ErrorKind::AddrInUse | std::io::ErrorKind::WouldBlock
            ) {
                // The legacy manager's activation socket belongs to its owner. The failed core
                // cannot create one, and Electron's own instance lock handles new-to-new focus.
                OwnershipError::AlreadyRunning
            } else {
                OwnershipError::Unavailable
            }
        })?;
    #[cfg(unix)]
    let activation = ActivationListener::start(focus).map_err(|_| OwnershipError::Unavailable)?;
    #[cfg(not(unix))]
    let _ = focus;
    Ok(RuntimeOwner {
        #[cfg(unix)]
        _activation: activation,
        _guard: guard,
    })
}

#[cfg(unix)]
fn activation_path() -> PathBuf {
    codex_plus_core::paths::default_app_state_dir().join("manager-activate.sock")
}

#[cfg(unix)]
struct ActivationListener {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    path: PathBuf,
}

#[cfg(unix)]
impl ActivationListener {
    fn start(focus: mpsc::Sender<()>) -> std::io::Result<Self> {
        use std::os::unix::{fs::PermissionsExt, net::UnixListener};
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let path = activation_path();
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let listener = UnixListener::bind(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread = std::thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok(_) => {
                        let _ = focus.try_send(());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(25));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            stop,
            thread: Some(thread),
            path,
        })
    }
}

#[cfg(unix)]
impl Drop for ActivationListener {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}
