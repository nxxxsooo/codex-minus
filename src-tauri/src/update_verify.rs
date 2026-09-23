//! Signature gate for Electron artifacts. This path is read-only and runs before any installer.
use std::path::Path;
use std::{fs, io::Read};

use anyhow::{Context, ensure};
use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use sha2::{Digest, Sha256};

pub const PINNED_MINISIGN_KEY: &str = "RWQPLOj53PNl7oHXovERPXrOO0S0PrjAb3xT/n8UCTygNgEIyE5XrFqm";

pub fn verify_staged_artifact(
    artifact: &Path,
    signature: &Path,
    sha256: &str,
    size: u64,
) -> anyhow::Result<()> {
    verify_with_key(artifact, signature, sha256, size, PINNED_MINISIGN_KEY)
}

fn verify_with_key(
    artifact: &Path,
    signature: &Path,
    sha256: &str,
    size: u64,
    key: &str,
) -> anyhow::Result<()> {
    ensure!(
        size > 0 && size <= 2 * 1024 * 1024 * 1024,
        "InvalidUpdateSize"
    );
    ensure!(
        sha256.len() == 64
            && sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "InvalidUpdateDigest"
    );
    let artifact_meta = fs::symlink_metadata(artifact)?;
    let signature_meta = fs::symlink_metadata(signature)?;
    ensure!(
        artifact_meta.is_file()
            && !artifact_meta.file_type().is_symlink()
            && artifact_meta.len() == size,
        "InvalidUpdateArtifact"
    );
    ensure!(
        signature_meta.is_file()
            && !signature_meta.file_type().is_symlink()
            && signature_meta.len() <= 4096,
        "InvalidUpdateSignature"
    );
    let encoded = fs::read_to_string(signature)?;
    let encoded = encoded.trim();
    let raw = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .context("InvalidUpdateSignature")?;
    ensure!(
        base64::engine::general_purpose::STANDARD.encode(&raw) == encoded,
        "InvalidUpdateSignature"
    );
    let signature =
        Signature::decode(std::str::from_utf8(&raw)?).context("InvalidUpdateSignature")?;
    let public_key = PublicKey::from_base64(key).context("InvalidUpdateKey")?;
    // Tauri signs prehashed update artifacts; the streaming verifier rejects legacy signatures.
    let mut verifier = public_key
        .verify_stream(&signature)
        .context("InvalidUpdateSignature")?;
    let mut digest = Sha256::new();
    let mut file = fs::File::open(artifact)?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut received = 0_u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        received += count as u64;
        ensure!(received <= size, "InvalidUpdateArtifact");
        digest.update(&buffer[..count]);
        verifier.update(&buffer[..count]);
    }
    ensure!(
        received == size && format!("{:x}", digest.finalize()) == sha256,
        "InvalidUpdateDigest"
    );
    verifier.finalize().context("InvalidUpdateSignature")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use sha2::{Digest, Sha256};
    use std::fs;

    const TEST_PUBLIC_KEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const TEST_SIGNATURE: &str = "untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1556193335\tfile:test\ny/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==\n";

    #[test]
    fn pinned_key_is_exactly_the_existing_tauri_updater_trust_root() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let encoded = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
        let raw = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        assert!(
            String::from_utf8(raw)
                .unwrap()
                .contains(PINNED_MINISIGN_KEY)
        );
    }

    #[test]
    fn signed_bytes_pass_and_tamper_wrong_key_or_wrong_size_fail() {
        let temp = tempfile::tempdir().unwrap();
        let artifact = temp.path().join("candidate.tar.gz");
        let signature = temp.path().join("candidate.sig");
        fs::write(&artifact, b"test").unwrap();
        fs::write(
            &signature,
            base64::engine::general_purpose::STANDARD.encode(TEST_SIGNATURE),
        )
        .unwrap();
        let hash = format!("{:x}", Sha256::digest(b"test"));
        verify_with_key(&artifact, &signature, &hash, 4, TEST_PUBLIC_KEY).unwrap();
        assert!(verify_with_key(&artifact, &signature, &hash, 4, PINNED_MINISIGN_KEY).is_err());
        assert!(verify_with_key(&artifact, &signature, &hash, 3, TEST_PUBLIC_KEY).is_err());
        fs::write(&artifact, b"Test").unwrap();
        assert!(verify_with_key(&artifact, &signature, &hash, 4, TEST_PUBLIC_KEY).is_err());
        let forged_hash = format!("{:x}", Sha256::digest(b"Test"));
        assert!(verify_with_key(&artifact, &signature, &forged_hash, 4, TEST_PUBLIC_KEY).is_err());
    }
}
