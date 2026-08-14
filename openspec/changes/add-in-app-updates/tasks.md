# Tasks — add-in-app-updates

## 1. Signing

- [x] 1.1 Generate the minisign keypair; private key at `~/.tauri/codex-minus-updater.key` (600, no password) and in the `TAURI_SIGNING_PRIVATE_KEY` repository secret; public key pinned in `tauri.conf.json`. (Done 2026-08-14 before authoring — the secret must exist before any CI run of this branch.)

## 2. App

- [x] 2.1 Register `tauri-plugin-updater` and `tauri-plugin-process`; grant their capabilities; add `createUpdaterArtifacts` and the `plugins.updater` block (pubkey, latest.json endpoint, Windows passive install mode).
- [x] 2.2 Leaf module for banner/progress presentation (phase reducer over download events, banner label + action mapping) with tests; App.tsx wires the startup check, banner, download-install-relaunch, and failure notice; i18n entries.

## 3. CI

- [x] 3.1 Build jobs sign with the secret and upload `.sig` files beside the installers; release job renames installer+signature pairs, asserts one signature per slot, composes `latest.json`, uploads it; release notes state the NSIS exe is the auto-update path on Windows. (README's "no auto-updater" paragraph rewritten too.)
- [x] 3.2 Drop the MSI target end-to-end — bundle targets, WiX language config, build/upload/packaging/assertion lines, README table, release notes (D4;「只留一个」). The NSIS installer carries SimpChinese and English.

## 4. Verification

- [ ] 4.1 `npm run verify`, Rust suite, `cargo fmt --check` green; three-platform CI green on the PR.
- [ ] 4.2 After the first release carrying this change: build a lower-versioned app locally, see the banner, install through it, land in the released version — on macOS and the Windows VM.
