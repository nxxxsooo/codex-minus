use std::process::Command;

#[test]
fn update_verifier_has_a_read_only_entry_point_and_never_echoes_paths_or_signature() {
    let binary = env!("CARGO_BIN_EXE_codex-minus-core");
    let incomplete = Command::new(binary)
        .arg("--verify-update")
        .output()
        .unwrap();
    assert_eq!(incomplete.status.code(), Some(2));
    let temp = tempfile::tempdir().unwrap();
    let artifact = temp.path().join("private-sentinel-artifact");
    let signature = temp.path().join("private-sentinel-signature");
    std::fs::write(&artifact, b"test").unwrap();
    std::fs::write(&signature, b"malformed-signature").unwrap();
    let rejected = Command::new(binary)
        .args([
            "--verify-update",
            artifact.to_str().unwrap(),
            signature.to_str().unwrap(),
            &"a".repeat(64),
            "4",
        ])
        .output()
        .unwrap();
    assert_eq!(rejected.status.code(), Some(1));
    assert!(rejected.stdout.is_empty());
    assert!(rejected.stderr.is_empty());
    assert_eq!(std::fs::read(&artifact).unwrap(), b"test");
    assert_eq!(std::fs::read(&signature).unwrap(), b"malformed-signature");
}
