use anyhow::{Context, ensure};
use std::ffi::OsStr;
use std::fs;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;

/// Upper bound for a helper process that runs inside a live-state transaction.
///
/// The coordinator lock is held across the whole transaction, so a child that never exits is
/// indistinguishable from a hung application: the editor keeps its pending state with nothing to
/// report. Ten seconds is far above the observed cost of these helpers and matches the bound the
/// target-CLI probes already use.
pub(crate) const HELPER_TIMEOUT: Duration = Duration::from_secs(10);

const MAX_HELPER_OUTPUT_BYTES: u64 = 1024 * 1024;

/// Distinguishes concurrent captures inside one process.
///
/// The wall clock cannot: on Windows it advances in ~15.6 ms steps, so two helpers started in the
/// same tick derive the same temp-file name, and whichever finishes first deletes the file the
/// other is still reading.
static HELPER_CAPTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Spawns a child with no window and no console of its own.
///
/// Only use this when the child's output is discarded or redirected to a file: a Windows
/// console host such as `powershell.exe` writes nothing to an inherited stdout pipe when it
/// starts detached, and still exits successfully, so captured output silently comes back empty.
pub(crate) fn background_command(program: impl AsRef<OsStr>) -> Command {
    let command = Command::new(program);
    #[cfg(windows)]
    let command = {
        use std::os::windows::process::CommandExt;

        let mut command = command;
        command.creation_flags(codex_plus_core::windows_create_no_window() | DETACHED_PROCESS);
        command
    };
    command
}

/// Spawns a child with no visible window while keeping its standard streams usable, so piped
/// stdout can be read back.
pub(crate) fn captured_output_command(program: impl AsRef<OsStr>) -> Command {
    let command = Command::new(program);
    #[cfg(windows)]
    let command = {
        use std::os::windows::process::CommandExt;

        let mut command = command;
        command.creation_flags(codex_plus_core::windows_create_no_window());
        command
    };
    command
}

/// Waits for an already-configured child, killing it once `timeout` elapses.
fn wait_bounded(
    command: &mut Command,
    timeout: Duration,
    what: &str,
) -> anyhow::Result<ExitStatus> {
    let mut child = command
        .spawn()
        .with_context(|| format!("cannot start {what}"))?;
    let started = Instant::now();
    let mut interval = Duration::from_millis(1);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("{what} did not finish within {} seconds", timeout.as_secs());
        }
        std::thread::sleep(interval);
        if interval < Duration::from_millis(25) {
            interval *= 2;
        }
    }
}

/// Runs a helper whose output is irrelevant, with a hard upper bound on its runtime.
///
/// Standard streams are closed rather than piped so no inherited handle can outlive the child.
pub(crate) fn status_bounded(
    command: &mut Command,
    timeout: Duration,
    what: &str,
) -> anyhow::Result<ExitStatus> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    wait_bounded(command, timeout, what)
}

/// The output half of this module is Windows-only in practice: `verify_windows_acl` in
/// `live_state` is its one caller, and it is `#[cfg(windows)]`.
///
/// A macOS build therefore reports `captured_output_command`, `output_bounded`, `capture_paths`,
/// `BoundedHelperOutput`, `MAX_HELPER_OUTPUT_BYTES`, and `HELPER_CAPTURE_SEQUENCE` as dead. They
/// are not. Deleting them on that evidence breaks the Windows build and nothing else, so it gets
/// found by CI rather than by a compiler — which is how it was found once already.
pub(crate) struct BoundedHelperOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

/// Names one capture's stdout and stderr files.
///
/// The clock alone does not separate them. `SystemTime::now` reports nanoseconds but advances in
/// ~15.6 ms steps on Windows, and the pid is shared by every thread, so helpers started together
/// derive one name — and the first to finish deletes the files the others are still reading. The
/// sequence is what makes each capture its own.
fn capture_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = HELPER_CAPTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir();
    let stem = format!(
        "codex-minus-helper-{}-{nonce}-{sequence}",
        std::process::id()
    );
    (
        root.join(format!("{stem}.out")),
        root.join(format!("{stem}.err")),
    )
}

/// Runs a helper whose output is needed, with a hard upper bound on its runtime.
///
/// Output lands in temporary files instead of pipes: waiting on a pipe means waiting for every
/// inherited handle to close, which a process-creation interceptor can hold open long after the
/// child itself has exited.
pub(crate) fn output_bounded(
    command: &mut Command,
    timeout: Duration,
    what: &str,
) -> anyhow::Result<BoundedHelperOutput> {
    let (stdout_path, stderr_path) = capture_paths();
    let result = (|| -> anyhow::Result<BoundedHelperOutput> {
        let stdout_file = fs::File::create(&stdout_path)
            .with_context(|| format!("cannot capture output of {what}"))?;
        let stderr_file = fs::File::create(&stderr_path)
            .with_context(|| format!("cannot capture output of {what}"))?;
        command
            .stdin(Stdio::null())
            .stdout(stdout_file)
            .stderr(stderr_file);
        let status = wait_bounded(command, timeout, what)?;
        let measure = |path: &std::path::Path| -> anyhow::Result<u64> {
            Ok(fs::metadata(path)
                .with_context(|| format!("cannot measure the output of {what}"))?
                .len())
        };
        ensure!(
            measure(&stdout_path)? <= MAX_HELPER_OUTPUT_BYTES
                && measure(&stderr_path)? <= MAX_HELPER_OUTPUT_BYTES,
            "{what} produced more output than expected"
        );
        Ok(BoundedHelperOutput {
            status,
            stdout: fs::read(&stdout_path)
                .with_context(|| format!("cannot read the output of {what}"))?,
            stderr: fs::read(&stderr_path)
                .with_context(|| format!("cannot read the output of {what}"))?,
        })
    })();
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_the_requested_program() {
        let command = background_command("codex-minus-test-command");
        assert_eq!(command.get_program(), "codex-minus-test-command");
    }

    #[test]
    fn captured_output_command_preserves_the_requested_program() {
        let command = captured_output_command("codex-minus-test-command");
        assert_eq!(command.get_program(), "codex-minus-test-command");
    }

    #[cfg(unix)]
    #[test]
    fn a_helper_that_never_finishes_is_killed_instead_of_waited_on() {
        let started = Instant::now();
        let error = status_bounded(
            Command::new("sleep").arg("30"),
            Duration::from_millis(300),
            "the test helper",
        )
        .expect_err("an unbounded child must not be waited on forever");

        assert!(
            error.to_string().contains("did not finish"),
            "the failure names the timeout: {error}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the wait returns near its bound rather than at the child's own pace"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_bounded_helper_returns_stdout_without_an_inherited_pipe() {
        let output = output_bounded(
            Command::new("echo").arg("BOUNDED"),
            Duration::from_secs(5),
            "the test helper",
        )
        .expect("the helper should run");

        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "BOUNDED");
    }

    #[test]
    fn captures_taken_inside_one_clock_tick_still_get_their_own_files() {
        // Windows advances the wall clock in ~15.6 ms steps, so a burst of helpers reads the same
        // nanosecond value, and every thread reports the same pid. Names must still differ.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..64 {
            let (stdout_path, stderr_path) = capture_paths();
            assert_ne!(stdout_path, stderr_path);
            assert!(
                seen.insert(stdout_path),
                "each capture named its own stdout"
            );
            assert!(
                seen.insert(stderr_path),
                "each capture named its own stderr"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_bounded_helper_keeps_its_output_while_others_run_beside_it() {
        let outputs: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|index| {
                    scope.spawn(move || {
                        let output = output_bounded(
                            Command::new("echo").arg(format!("BOUNDED-{index}")),
                            Duration::from_secs(5),
                            "the test helper",
                        )
                        .expect("a helper running beside others should still be readable");
                        String::from_utf8_lossy(&output.stdout).trim().to_string()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("the helper thread should finish"))
                .collect()
        });

        for index in 0..8 {
            assert!(
                outputs.contains(&format!("BOUNDED-{index}")),
                "each helper read back its own output: {outputs:?}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn captured_output_command_reads_console_host_stdout() {
        let output = captured_output_command("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Write-Output CAPTURED",
            ])
            .output()
            .expect("powershell should run");

        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "CAPTURED",
            "a detached console host reports success while returning no output"
        );
    }
}
