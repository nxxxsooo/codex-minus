use std::ffi::OsStr;
use std::process::Command;

#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_the_requested_program() {
        let command = background_command("codex-minus-test-command");
        assert_eq!(command.get_program(), "codex-minus-test-command");
    }
}
