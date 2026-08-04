<p align="right"><a href="README.md">中文</a></p>

<p align="center">
  <img src="docs/assets/codex-minus-hero.webp" alt="Codex-- Manager provider configuration screen" width="960">
</p>

<h1 align="center">Codex-- Manager</h1>

<p align="center">Switch providers and manage local sessions without surrendering your Context configuration.</p>

<p align="center">
  <a href="https://github.com/nxxxsooo/codex-minus/releases/latest"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/nxxxsooo/codex-minus?style=flat-square&color=197547"></a>
  <img alt="macOS arm64" src="https://img.shields.io/badge/macOS-arm64-202720?style=flat-square&logo=apple&logoColor=white">
  <img alt="Windows x86_64" src="https://img.shields.io/badge/Windows-x86__64-0078D4?style=flat-square&logo=windows&logoColor=white">
  <a href="LICENSE"><img alt="AGPL-3.0-only" src="https://img.shields.io/badge/license-AGPL--3.0--only-197547?style=flat-square"></a>
</p>

Codex-- Manager is a trimmed fork of [Codex++ Manager](https://github.com/BigPizzaV3/CodexPlusPlus). It keeps provider switching, local session lifecycle, and configuration diagnostics while removing renderer injection, launchers, marketplaces, and auto-update.

## Download

Supports Apple Silicon (`arm64`) and Windows (`x86_64`).

| Platform | Architecture | Format | Latest Release |
|----------|-------------|--------|----------------|
| macOS    | arm64       | .app.zip | [v0.2.0](https://github.com/nxxxsooo/codex-minus/releases/latest) |
| Windows  | x86_64      | .msi / .exe | [v0.2.0](https://github.com/nxxxsooo/codex-minus/releases/latest) |

- [Go to the Release page](https://github.com/nxxxsooo/codex-minus/releases)
- [Open the project website](https://mjshao.fun/codex-minus/)

```bash
# Verify macOS
shasum -a 256 -c SHA256SUMS
```

After verification, unzip the archive and move `Codex-- Manager.app` to `/Applications`. The current build uses an ad-hoc signature and is not signed with Developer ID or notarized by Apple. If macOS blocks the first launch, use Open Anyway in System Settings, Privacy & Security.

## Why it exists

Provider switching should only change provider configuration. Before every write path, Codex-- snapshots three Context tables from `~/.codex/config.toml` and grafts their original TOML content back after the upstream write:

```toml
[mcp_servers]
[skills]
[plugins]
```

This protection follows a real failure where a stale managed context copy overwrote valid MCP configuration during a provider switch. Codex-- removes that management feature and pins the protection contract with Rust tests.

## Scope

### Provider switching

- Manage relay profiles that use OAuth, API keys, or mixed authentication.
- Write `config.toml` and `auth.json` with rollback on failure.
- Read the effective `model_provider` after switching and skip session scans when it did not change.
- Detect `OPENAI_*` environment variables that may override provider configuration.

### Session lifecycle

- Browse active and archived sessions with backend pagination.
- Use the target Codex CLI for native `archive` and `unarchive` operations.
- Default to a 30-day retention threshold, with candidate preview and consent before enablement.
- Run maintenance after the interface is usable and no more than once every 24 hours.
- Create a local backup before session deletion.

### Context protection

- Route provider switch, apply, and clear operations through `with_context_tables_protected`.
- Never store or merge managed context copies.
- Do not restore the upstream Tools and Plugins management screen.

## Update and uninstall

Quit Codex-- Manager, download the latest release, and replace `/Applications/Codex-- Manager.app`. The app has no auto-updater, and a GitHub Release does not update an installed copy automatically.

User settings live under `~/.codex-session-delete/` and survive app replacement. You can decide separately whether to retain that directory when uninstalling.

## Known limitations

- There is no Intel build, Developer ID signature, or Apple notarization yet.
- Windows builds are produced by CI and have not been manually tested on a real Windows machine.
- Chat Completions and aggregator profiles depend on the upstream launcher's proxy at `127.0.0.1:57321`. Codex-- does not ship that proxy, so these profiles should not be used.
- The pinned upstream revision does not expose active-only provider-sync writes. Adapt to current provider remains disabled and never falls back to rewriting full history.
- Archiving organizes sessions but does not compress data or free disk space.

## Architecture

- Frontend: React 19, Vite, and TypeScript.
- Desktop and backend: Tauri 2 and Rust.
- Upstream logic: `codex-plus-core` and `codex-plus-data`, pinned to an explicit git revision instead of vendored locally.
- Bundle identifier: `fun.mjshao.codex-minus`.
- State directory: `~/.codex-session-delete/`.

## Development

```bash
npm install
npm run check
npm run vite:build
cd src-tauri && cargo test
npm run build
```

The full Tauri build generates:

- macOS: `src-tauri/target/release/bundle/macos/Codex-- Manager.app`
- Windows: `src-tauri/target/release/bundle/msi/*.msi` or `src-tauri/target/release/bundle/nsis/*.exe`

## Version note

The public website and README on `master` were added after the `v0.1.0` application tag. The `v0.1.0` macOS binary and its SHA-256 remain unchanged.

## License

AGPL-3.0-only, inherited from the upstream project.
