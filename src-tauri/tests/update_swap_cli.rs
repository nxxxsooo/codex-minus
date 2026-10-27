#[cfg(target_os = "macos")]
#[test]
fn atomic_bundle_exchange_retains_both_generations_and_rejects_symlinks() {
    use std::{fs, process::Command};
    let temp = tempfile::tempdir().unwrap();
    let current = temp.path().join("current");
    let staged = temp.path().join("staged");
    fs::create_dir(&current).unwrap();
    fs::create_dir(&staged).unwrap();
    fs::write(current.join("marker"), "old").unwrap();
    fs::write(staged.join("marker"), "new").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_codex-minus-core"))
        .arg("--swap-update-bundles")
        .arg(&current)
        .arg(&staged)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(result.stdout.is_empty() && result.stderr.is_empty());
    assert_eq!(fs::read_to_string(current.join("marker")).unwrap(), "new");
    assert_eq!(fs::read_to_string(staged.join("marker")).unwrap(), "old");
    let link = temp.path().join("alias");
    std::os::unix::fs::symlink(&current, &link).unwrap();
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_codex-minus-core"))
            .arg("--swap-update-bundles")
            .arg(link)
            .arg(&staged)
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(fs::read_to_string(current.join("marker")).unwrap(), "new");
}
