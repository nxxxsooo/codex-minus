# Proposal: add-in-app-updates

## Why

Every release today reaches the fleet by hand: the owner cuts a tag, downloads the installer from the GitHub release page, sends it over Lark, and each machine re-installs manually. With two active machines (owner's Mac, Eva's Windows) and a cadence of small fix releases — 0.4.6, 0.4.7, 0.4.8 within days, each carrying a diagnostic improvement the previous report needed — the manual loop is now the slowest link in the debug cycle: Eva is currently blocked on a save error whose 详情 payload only the next version can show her.

## What Changes

- Release artifacts are cryptographically signed (minisign via the Tauri updater toolchain) and every tagged release publishes a `latest.json` update manifest alongside the installers.
- The app checks the manifest once at startup, silently; when a newer version exists it shows a persistent banner naming the version, with a one-click "更新并重启" that downloads, verifies the signature, installs, and relaunches.
- Update failures (offline, signature mismatch, interrupted download) never block the app: the check is silent on failure and an explicit install attempt reports through the normal failure notice with 详情.
- The verification public key is pinned in the app config; the private key lives only on the owner's machine and in the repository's CI secret.

## Capabilities

### New Capabilities

- `app-distribution`: signed update manifests per release, and an in-app check-and-install path.

### Modified Capabilities

None.

## Impact

- `src-tauri/`: `tauri-plugin-updater` + `tauri-plugin-process` registered; `tauri.conf.json` gains the pinned public key, endpoint, and `createUpdaterArtifacts`; capability grants for the two plugins.
- `src/`: a leaf module for the banner/progress presentation with its test; App.tsx wiring for the startup check and banner.
- `.github/workflows/build.yml`: build jobs sign with the CI secret; the release job assembles and uploads `latest.json`.
- No provider, catalog, or state changes. The update path replaces the whole app bundle, so it composes with — and never partially applies — anything the release contains.
