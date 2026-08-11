use std::cell::RefCell;
use std::collections::HashSet;
#[cfg(unix)]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const JOURNAL_FILE: &str = "live-state-transaction.json";
const TRANSACTION_ROOT: &str = "live-state-transactions";

static LIVE_STATE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

thread_local! {
    static ACTIVE_PERMISSION_CACHE: RefCell<Option<HashSet<PermissionTarget>>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PermissionTarget {
    Directory(PathBuf),
    File(PathBuf),
}

pub struct LiveStateGuard {
    _guard: MutexGuard<'static, ()>,
}

impl Drop for LiveStateGuard {
    fn drop(&mut self) {
        ACTIVE_PERMISSION_CACHE.with(|cache| *cache.borrow_mut() = None);
    }
}

#[derive(Debug, Clone)]
pub struct FileMutation {
    pub path: PathBuf,
    pub contents: Option<Vec<u8>>,
}

impl FileMutation {
    pub fn text(path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            contents: Some(contents.into().into_bytes()),
        }
    }

    pub fn bytes(path: impl Into<PathBuf>, contents: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            contents: Some(contents),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryOutcome {
    None,
    RolledForward,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionJournal {
    version: u32,
    transaction_id: String,
    phase: TransactionPhase,
    applied_count: usize,
    entries: Vec<JournalEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum TransactionPhase {
    Prepared,
    Applying,
    Committed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JournalEntry {
    target_path: PathBuf,
    target_stage_path: Option<PathBuf>,
    prior_stage_path: Option<PathBuf>,
    target_hash: Option<String>,
    prior_hash: Option<String>,
}

pub fn lock() -> anyhow::Result<LiveStateGuard> {
    let guard = LIVE_STATE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow::anyhow!("live-state coordinator lock is poisoned"))?;
    ACTIVE_PERMISSION_CACHE.with(|cache| {
        *cache.borrow_mut() = Some(HashSet::new());
    });
    Ok(LiveStateGuard { _guard: guard })
}

pub fn prepare_secret_paths(codex_home: &Path) -> anyhow::Result<()> {
    let app_state = codex_plus_core::paths::default_app_state_dir();
    ensure_owner_only_dir(&app_state)?;
    cleanup_interrupted_atomic_temps(&app_state)?;
    cleanup_private_workspaces(&app_state)?;
    let settings_path = codex_plus_core::paths::default_settings_path();
    if settings_path.exists() {
        ensure_owner_only_file(&settings_path)?;
    }
    ensure_owner_only_dir(codex_home)?;
    cleanup_interrupted_atomic_temps(codex_home)?;
    let generated = codex_home.join("model-catalogs");
    if generated.is_dir() {
        ensure_owner_only_dir(&generated)?;
        cleanup_interrupted_atomic_temps(&generated)?;
    }
    let auth_path = codex_home.join("auth.json");
    if auth_path.exists() {
        ensure_owner_only_file(&auth_path)?;
    }
    Ok(())
}

fn cleanup_interrupted_atomic_temps(directory: &Path) -> anyhow::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') && name.contains(".codex-minus-") && name.ends_with(".tmp") {
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_file() || metadata.file_type().is_symlink() {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}

fn cleanup_private_workspaces(app_state: &Path) -> anyhow::Result<()> {
    for name in [
        "private-staging",
        "catalog-refresh",
        "catalog-validation",
        "command-output",
    ] {
        let root = app_state.join(name);
        if !root.exists() {
            continue;
        }
        ensure_owner_only_dir(&root)?;
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
                fs::remove_dir_all(entry.path())?;
            } else {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}

pub fn ensure_owner_only_dir(path: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create private directory {}", path.display()))?;
    let target = PermissionTarget::Directory(path.to_path_buf());
    if permission_is_cached(&target) {
        return Ok(());
    }
    apply_owner_only_dir(path)?;
    verify_owner_only_dir(path)?;
    remember_permission(target);
    Ok(())
}

pub fn ensure_owner_only_file(path: &Path) -> anyhow::Result<()> {
    ensure!(
        path.is_file(),
        "private file is missing: {}",
        path.display()
    );
    let target = PermissionTarget::File(path.to_path_buf());
    if permission_is_cached(&target) {
        return Ok(());
    }
    apply_owner_only_file(path)?;
    verify_owner_only_file(path)?;
    remember_permission(target);
    Ok(())
}

fn permission_is_cached(target: &PermissionTarget) -> bool {
    ACTIVE_PERMISSION_CACHE.with(|cache| {
        cache
            .borrow()
            .as_ref()
            .is_some_and(|secured| secured.contains(target))
    })
}

fn remember_permission(target: PermissionTarget) {
    ACTIVE_PERMISSION_CACHE.with(|cache| {
        if let Some(secured) = cache.borrow_mut().as_mut() {
            secured.insert(target);
        }
    });
}

pub fn atomic_write_owner_only(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("private file has no parent: {}", path.display()))?;
    ensure_owner_only_dir(parent)?;
    let temp_path = unique_temp_path(path);
    let write_result = (|| -> anyhow::Result<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temp_path)
            .with_context(|| format!("failed to create temp file {}", temp_path.display()))?;
        file.write_all(bytes)?;
        file.sync_all()?;
        ensure_owner_only_file(&temp_path)?;
        fs::rename(&temp_path, path).with_context(|| {
            format!(
                "failed to atomically replace {} from {}",
                path.display(),
                temp_path.display()
            )
        })?;
        ensure_owner_only_file(path)?;
        sync_directory(parent)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

pub fn commit_locked(mutations: &[FileMutation]) -> anyhow::Result<()> {
    commit_locked_verified(mutations, || Ok(()))
}

pub fn commit_locked_verified(
    mutations: &[FileMutation],
    verify: impl FnOnce() -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    recover_locked()?;
    validate_mutations(mutations)?;
    if mutations.is_empty() {
        return Ok(());
    }

    let app_state = codex_plus_core::paths::default_app_state_dir();
    ensure_owner_only_dir(&app_state)?;
    let transaction_id = transaction_id();
    let transaction_dir = app_state.join(TRANSACTION_ROOT).join(&transaction_id);
    ensure_owner_only_dir(&transaction_dir)?;

    let entries = (|| -> anyhow::Result<Vec<JournalEntry>> {
        let mut entries = Vec::with_capacity(mutations.len());
        for (index, mutation) in mutations.iter().enumerate() {
            let prior = match fs::read(&mutation.path) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("failed to snapshot {}", mutation.path.display())
                    });
                }
            };
            let prior_stage_path = match prior.as_ref() {
                Some(bytes) => {
                    let path = transaction_dir.join(format!("{index}.prior"));
                    atomic_write_owner_only(&path, bytes)?;
                    Some(path)
                }
                None => None,
            };
            let target_stage_path = match mutation.contents.as_ref() {
                Some(bytes) => {
                    let path = transaction_dir.join(format!("{index}.target"));
                    atomic_write_owner_only(&path, bytes)?;
                    Some(path)
                }
                None => None,
            };
            entries.push(JournalEntry {
                target_path: mutation.path.clone(),
                target_stage_path,
                prior_stage_path,
                target_hash: mutation.contents.as_deref().map(content_hash),
                prior_hash: prior.as_deref().map(content_hash),
            });
        }
        Ok(entries)
    })();
    let entries = match entries {
        Ok(entries) => entries,
        Err(error) => {
            let _ = fs::remove_dir_all(&transaction_dir);
            return Err(error);
        }
    };

    let mut journal = TransactionJournal {
        version: 1,
        transaction_id,
        phase: TransactionPhase::Prepared,
        applied_count: 0,
        entries,
    };
    if let Err(error) = persist_journal(&journal) {
        let _ = fs::remove_dir_all(&transaction_dir);
        return Err(error);
    }

    let result = (|| -> anyhow::Result<()> {
        journal.phase = TransactionPhase::Applying;
        persist_journal(&journal)?;
        for index in 0..journal.entries.len() {
            apply_entry_target(&journal.entries[index])?;
            verify_entry_target(&journal.entries[index])?;
            journal.applied_count = index + 1;
            persist_journal(&journal)?;
        }
        verify().context("live-state post-commit verification failed")?;
        journal.phase = TransactionPhase::Committed;
        persist_journal(&journal)?;
        for entry in &journal.entries {
            verify_entry_target(entry)?;
        }
        cleanup_journal(&journal)
    })();

    if let Err(error) = result {
        let rollback_error = rollback_journal(&journal).err();
        return match rollback_error {
            Some(rollback_error) => Err(anyhow::anyhow!(
                "transaction failed: {error}; rollback failed: {rollback_error}"
            )),
            None => Err(error),
        };
    }
    Ok(())
}

pub fn recover_locked() -> anyhow::Result<RecoveryOutcome> {
    let journal_path = journal_path();
    let bytes = match fs::read(&journal_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RecoveryOutcome::None);
        }
        Err(error) => return Err(error).context("failed to read live-state transaction journal"),
    };
    ensure_owner_only_file(&journal_path)?;
    let journal: TransactionJournal =
        serde_json::from_slice(&bytes).context("live-state transaction journal is invalid")?;
    validate_journal(&journal)?;

    if journal
        .entries
        .iter()
        .all(|entry| verify_entry_target(entry).is_ok())
    {
        cleanup_journal(&journal)?;
        return Ok(RecoveryOutcome::RolledForward);
    }

    rollback_journal(&journal)?;
    Ok(RecoveryOutcome::RolledBack)
}

fn validate_mutations(mutations: &[FileMutation]) -> anyhow::Result<()> {
    let mut seen = HashSet::new();
    for mutation in mutations {
        ensure!(
            mutation.path.is_absolute(),
            "transaction target must be absolute"
        );
        ensure!(
            mutation.path.file_name().and_then(|value| value.to_str()) != Some("auth.json"),
            "provider-driven auth.json writes are prohibited"
        );
        ensure!(
            seen.insert(mutation.path.clone()),
            "duplicate transaction target"
        );
    }
    Ok(())
}

fn validate_journal(journal: &TransactionJournal) -> anyhow::Result<()> {
    ensure!(
        journal.version == 1,
        "unsupported transaction journal version"
    );
    ensure!(
        !journal.transaction_id.trim().is_empty(),
        "missing transaction id"
    );
    let app_state = codex_plus_core::paths::default_app_state_dir();
    let expected_dir = app_state
        .join(TRANSACTION_ROOT)
        .join(&journal.transaction_id);
    for entry in &journal.entries {
        ensure!(
            entry.target_path.is_absolute(),
            "journal target must be absolute"
        );
        ensure!(
            entry
                .target_path
                .file_name()
                .and_then(|value| value.to_str())
                != Some("auth.json"),
            "journal attempts to modify auth.json"
        );
        for stage_path in [
            entry.target_stage_path.as_ref(),
            entry.prior_stage_path.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            ensure!(
                stage_path.starts_with(&expected_dir),
                "journal stage path escaped"
            );
        }
    }
    Ok(())
}

fn apply_entry_target(entry: &JournalEntry) -> anyhow::Result<()> {
    match &entry.target_stage_path {
        Some(stage_path) => atomic_write_owner_only(&entry.target_path, &fs::read(stage_path)?),
        None => remove_exact_file(&entry.target_path),
    }
}

fn apply_entry_prior(entry: &JournalEntry) -> anyhow::Result<()> {
    match &entry.prior_stage_path {
        Some(stage_path) => atomic_write_owner_only(&entry.target_path, &fs::read(stage_path)?),
        None => remove_exact_file(&entry.target_path),
    }
}

fn verify_entry_target(entry: &JournalEntry) -> anyhow::Result<()> {
    verify_path_hash(&entry.target_path, entry.target_hash.as_deref())
}

fn verify_entry_prior(entry: &JournalEntry) -> anyhow::Result<()> {
    verify_path_hash(&entry.target_path, entry.prior_hash.as_deref())
}

fn verify_path_hash(path: &Path, expected: Option<&str>) -> anyhow::Result<()> {
    match expected {
        Some(expected) => {
            let bytes =
                fs::read(path).with_context(|| format!("failed to verify {}", path.display()))?;
            ensure!(
                content_hash(&bytes) == expected,
                "hash mismatch for {}",
                path.display()
            );
            ensure_owner_only_file(path)
        }
        None => {
            ensure!(!path.exists(), "expected {} to be absent", path.display());
            Ok(())
        }
    }
}

fn rollback_journal(journal: &TransactionJournal) -> anyhow::Result<()> {
    let mut failures = Vec::new();
    for entry in journal.entries.iter().rev() {
        if let Err(error) = apply_entry_prior(entry).and_then(|_| verify_entry_prior(entry)) {
            failures.push(format!("{}: {error}", entry.target_path.display()));
        }
    }
    ensure!(failures.is_empty(), "{}", failures.join("; "));
    cleanup_journal(journal)
}

fn persist_journal(journal: &TransactionJournal) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec_pretty(journal)?;
    atomic_write_owner_only(&journal_path(), &bytes)
}

fn cleanup_journal(journal: &TransactionJournal) -> anyhow::Result<()> {
    let path = journal_path();
    if path.exists() {
        fs::remove_file(&path)?;
    }
    let transaction_dir = codex_plus_core::paths::default_app_state_dir()
        .join(TRANSACTION_ROOT)
        .join(&journal.transaction_id);
    if transaction_dir.exists() {
        fs::remove_dir_all(&transaction_dir)?;
    }
    if let Some(parent) = path.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

fn journal_path() -> PathBuf {
    codex_plus_core::paths::default_app_state_dir().join(JOURNAL_FILE)
}

fn remove_exact_file(path: &Path) -> anyhow::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => {
            if let Some(parent) = path.parent() {
                sync_directory(parent)?;
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn content_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn unique_temp_path(path: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("state");
    path.with_file_name(format!(
        ".{name}.codex-minus-{}-{nonce}.tmp",
        std::process::id()
    ))
}

fn transaction_id() -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{nonce}", std::process::id())
}

#[cfg(unix)]
fn apply_owner_only_dir(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(unix)]
fn apply_owner_only_file(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(unix)]
fn verify_owner_only_dir(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)?.permissions().mode() & 0o777;
    ensure!(
        mode == 0o700,
        "directory is not owner-only: {} ({mode:o})",
        path.display()
    );
    Ok(())
}

#[cfg(unix)]
fn verify_owner_only_file(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)?.permissions().mode() & 0o777;
    ensure!(
        mode == 0o600,
        "file is not owner-only: {} ({mode:o})",
        path.display()
    );
    Ok(())
}

#[cfg(windows)]
fn apply_owner_only_dir(path: &Path) -> anyhow::Result<()> {
    apply_windows_acl(path, true)
}

#[cfg(windows)]
fn apply_owner_only_file(path: &Path) -> anyhow::Result<()> {
    apply_windows_acl(path, false)
}

#[cfg(windows)]
fn verify_owner_only_dir(path: &Path) -> anyhow::Result<()> {
    verify_windows_acl(path)
}

#[cfg(windows)]
fn verify_owner_only_file(path: &Path) -> anyhow::Result<()> {
    verify_windows_acl(path)
}

#[cfg(windows)]
fn apply_windows_acl(path: &Path, directory: bool) -> anyhow::Result<()> {
    let user = std::env::var("USERNAME").context("USERNAME is unavailable")?;
    let grant = if directory {
        format!("{user}:(OI)(CI)F")
    } else {
        format!("{user}:F")
    };
    let status = crate::platform_command::background_command("icacls")
        .arg(path)
        .args(["/inheritance:r", "/grant:r", &grant])
        .status()?;
    ensure!(status.success(), "icacls failed for {}", path.display());
    Ok(())
}

#[cfg(windows)]
fn verify_windows_acl(path: &Path) -> anyhow::Result<()> {
    let output = crate::platform_command::background_command("icacls")
        .arg(path)
        .output()?;
    ensure!(
        output.status.success(),
        "cannot verify ACL for {}",
        path.display()
    );
    let text = String::from_utf8_lossy(&output.stdout);
    ensure!(
        !text.contains("Everyone:"),
        "Everyone retains access to {}",
        path.display()
    );
    Ok(())
}

#[cfg(not(any(unix, windows)))]
compile_error!("owner-only permission enforcement is not implemented for this platform");

#[cfg(unix)]
fn sync_directory(path: &Path) -> anyhow::Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(windows)]
fn sync_directory(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn atomic_write_repairs_owner_only_mode() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("secret.json");
        atomic_write_owner_only(&path, b"secret").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"secret");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn transaction_rejects_auth_target() {
        let mutation = FileMutation::text(PathBuf::from("/tmp/auth.json"), "{}");
        assert!(validate_mutations(&[mutation]).is_err());
    }

    #[test]
    fn journal_contains_hashes_and_paths_only() {
        let journal = TransactionJournal {
            version: 1,
            transaction_id: "test".to_string(),
            phase: TransactionPhase::Prepared,
            applied_count: 0,
            entries: vec![JournalEntry {
                target_path: PathBuf::from("/tmp/settings.json"),
                target_stage_path: Some(PathBuf::from("/tmp/stage/target")),
                prior_stage_path: None,
                target_hash: Some(content_hash(b"secret payload")),
                prior_hash: None,
            }],
        };
        let text = serde_json::to_string(&journal).unwrap();
        assert!(!text.contains("secret payload"));
        assert!(!text.contains("chatgptAuthTokens"));
    }

    #[test]
    fn every_partial_apply_boundary_rolls_back_complete_generation() {
        for boundary in 0..=3 {
            let temp = tempfile::tempdir().unwrap();
            let stage = temp.path().join("stage");
            ensure_owner_only_dir(&stage).unwrap();
            let mut entries = Vec::new();
            for index in 0..3 {
                let target = temp.path().join(format!("target-{index}"));
                let prior = format!("prior-{index}").into_bytes();
                let next = format!("next-{index}").into_bytes();
                atomic_write_owner_only(&target, &prior).unwrap();
                let prior_stage = stage.join(format!("{index}.prior"));
                let target_stage = stage.join(format!("{index}.target"));
                atomic_write_owner_only(&prior_stage, &prior).unwrap();
                atomic_write_owner_only(&target_stage, &next).unwrap();
                entries.push(JournalEntry {
                    target_path: target,
                    target_stage_path: Some(target_stage),
                    prior_stage_path: Some(prior_stage),
                    target_hash: Some(content_hash(&next)),
                    prior_hash: Some(content_hash(&prior)),
                });
            }
            for entry in entries.iter().take(boundary) {
                apply_entry_target(entry).unwrap();
            }
            for entry in entries.iter().rev() {
                apply_entry_prior(entry).unwrap();
                verify_entry_prior(entry).unwrap();
            }
            for (index, entry) in entries.iter().enumerate() {
                assert_eq!(
                    fs::read(&entry.target_path).unwrap(),
                    format!("prior-{index}").as_bytes()
                );
            }
        }
    }

    #[test]
    fn coordinator_serializes_conflicting_operations() {
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let threads = (0..8)
            .map(|_| {
                let active = Arc::clone(&active);
                let peak = Arc::clone(&peak);
                std::thread::spawn(move || {
                    let _guard = lock().unwrap();
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(current, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(2));
                    active.fetch_sub(1, Ordering::SeqCst);
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(peak.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn permission_cache_is_scoped_to_one_coordinator_guard() {
        let target = PermissionTarget::Directory(PathBuf::from("permission-cache-test"));
        assert!(!permission_is_cached(&target));
        {
            let _guard = lock().unwrap();
            remember_permission(target.clone());
            assert!(permission_is_cached(&target));
        }
        assert!(!permission_is_cached(&target));
    }

    #[test]
    fn interrupted_temp_cleanup_is_namespaced_and_preserves_user_files() {
        let temp = tempfile::tempdir().unwrap();
        let interrupted = temp.path().join(".state.codex-minus-1-2.tmp");
        let ordinary = temp.path().join("user.tmp");
        fs::write(&interrupted, "partial").unwrap();
        fs::write(&ordinary, "keep").unwrap();
        cleanup_interrupted_atomic_temps(temp.path()).unwrap();
        assert!(!interrupted.exists());
        assert_eq!(fs::read_to_string(ordinary).unwrap(), "keep");
    }
}
