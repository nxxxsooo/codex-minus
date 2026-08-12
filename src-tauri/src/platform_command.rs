use std::ffi::OsStr;
use std::process::Command;

#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;

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
