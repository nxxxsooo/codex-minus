<p align="right"><a href="README.md">中文</a></p>

<p align="center">
  <img src="docs/assets/codex-minus-hero.webp" alt="Codex Minus provider configuration screen" width="960">
</p>

<h1 align="center">Codex Minus</h1>

<p align="center">Switch providers and manage model catalogs without surrendering OAuth or Context.</p>

<p align="center">
  <a href="https://github.com/nxxxsooo/codex-minus/releases/latest"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/nxxxsooo/codex-minus?style=flat-square&color=197547"></a>
  <img alt="macOS arm64" src="https://img.shields.io/badge/macOS-arm64-202720?style=flat-square&logo=apple&logoColor=white">
  <img alt="Windows x86_64" src="https://img.shields.io/badge/Windows-x86__64-0078D4?style=flat-square&logo=windows&logoColor=white">
  <a href="LICENSE"><img alt="AGPL-3.0-only" src="https://img.shields.io/badge/license-AGPL--3.0--only-197547?style=flat-square"></a>
</p>

Codex Minus is a trimmed fork of [Codex++ Manager](https://github.com/BigPizzaV3/CodexPlusPlus). It keeps provider switching, model catalogs, local session lifecycle, and configuration diagnostics, and ships signed in-app updates. There is no renderer injection, launcher, or marketplace.

## Download

Supports Apple Silicon (`arm64`) and Windows (`x86_64`).

| Platform | Architecture | Format | Latest Release |
|----------|-------------|--------|----------------|
| macOS    | arm64       | .app.zip | [latest](https://github.com/nxxxsooo/codex-minus/releases/latest) |
| Windows  | x64 / arm64 | -setup.exe (NSIS) | [latest](https://github.com/nxxxsooo/codex-minus/releases/latest) |

- [Go to the Release page](https://github.com/nxxxsooo/codex-minus/releases)
- [Open the project website](https://mjshao.fun/codex-minus/)

```bash
# Verify macOS
shasum -a 256 -c SHA256SUMS
```

After verification, unzip the archive and **drag `Codex Minus.app` into `/Applications` with Finder before launching** — running it from the unzip folder leaves it on a read-only translocated mount, and in-app updates then fail with `Read-only file system (os error 30)` (alternatively run `xattr -dr com.apple.quarantine "/Applications/Codex Minus.app"` and retry). The current build uses an ad-hoc signature and is not signed with Developer ID or notarized by Apple. If macOS blocks the first launch, use Open Anyway in System Settings, Privacy & Security.

## Why it exists

Provider switching should only change provider configuration. Before every write path, Codex Minus snapshots three Context tables from `~/.codex/config.toml` and grafts their original TOML content back after the upstream write:

```toml
[mcp_servers]
[skills]
[plugins]
```

This protection follows a real failure where a stale managed context copy overwrote valid MCP configuration during a provider switch. Codex Minus removes that management feature and pins the protection contract with Rust tests.

## Scope

### Provider switching

- ChatGPT OAuth remains owned by the official Codex/ChatGPT client. Provider profiles never persist, restore, or apply `authContents`.
- Pure API and mixed-provider keys live only in owner-only settings and the provider bearer configuration in `config.toml`. Provider operations never write live `auth.json`.
- Settings, provider configuration, model catalogs, and the live pointer commit through one recoverable transaction that restores the complete previous generation on failure.
- Read the effective `model_provider` after switching and skip session scans when it did not change.
- Detect `OPENAI_*` environment variables that may override provider configuration.

### Native capability priority

- A mixed provider (official login plus a custom Base URL and Key) can adopt one fixed contract: provider name `OpenAI`, `wire_api = "responses"`, your Key as the provider bearer, and one actor-authorization header. `requires_openai_auth` defaults to `true` so the ChatGPT-attached login and plugin surfaces stay available; both `true` and `false` are legal values, saves never rewrite the one you have, and only an explicit exit to pure-API mode writes `false`.
- The actor header only asserts that this client may issue requests as a local extension. It is not a plan upgrade and grants nothing by itself; each capability (text Responses, model discovery, image generation and editing, remote compaction, web search) is decided upstream and independently, and untested capabilities display as unknown.
- Upgrading to native capability priority is an explicit, previewed, confirmed action on one profile. Startup, inspection, and saves of other profiles never rewrite an existing contract.
- Exiting to pure OAuth is destructive by design: the preview lists the provider table and fields that will be removed, and confirmation deletes them from the profile, settings, and live `config.toml` with no dormant copy.

### Model catalogs

- Without a static `model_catalog_json`, either OAuth or API providers may update the shared `models_cache.json` through their own `/models` path. Mixed mode routes that request through the active custom provider, so the live cache is provider-ambiguous and cannot be the official baseline.
- Refresh the official list only through the platform-verified Codex CLI embedded in the configured target application. A `codex` found on `PATH` and a provider `/v1/models` response are not official sources.
- Run refresh in a private temporary `CODEX_HOME` with only the current access and ID tokens projected. The refresh token is empty, and temporary auth is never written back to live state.
- Each non-aggregate provider can use native official, official plus custom, custom only, or external mode. External files remain read-only until explicit preview and adoption.
- Preserve every target-emitted official field and hidden model. Overlays manage only visibility, ordering, context windows, and custom models.
- Treat provider `/v1/models` output as timestamped reported/not-reported evidence and custom candidates. An omission never hides an official model.
- Renumber every model's `priority` sequentially over the final list, so the Codex picker order always matches the manager's list order and custom models never interleave between officials. A custom model that declares no reasoning levels materializes `supported_reasoning_levels: []`, so Codex offers no Effort menu and sends no `effort` parameter.
- Write managed catalogs under `~/.codex/model-catalogs/codex-minus-<profile>-<hash>.json`. Active static-catalog changes prompt for a Codex restart; the restart prompt and the sidebar-foot Restart Codex button perform one graceful, confirmed restart (quit request only, never force-kill; macOS only). Nothing restarts the official client automatically.

### Session lifecycle

- Browse active and archived sessions with backend pagination.
- Use the target Codex CLI for native `archive` and `unarchive` operations.
- Default to a 30-day retention threshold, with candidate preview and consent before enablement.
- Run maintenance after the interface is usable and no more than once every 24 hours.
- Create a local backup before session deletion.
- Adapt to current provider rewrites active sessions only: per-session rollout header rewrites plus per-id sqlite updates keep archived history unreachable by construction, locked files are skipped whole, and provider switches can run it automatically (default on).

### Context protection

- Route switching, apply, clear, active save, and catalog-pointer writes through one coordinator and a fail-closed Context transaction.
- Fail the complete command if snapshot, TOML parsing, grafting, post-write verification, or restoration fails.
- Never store or merge managed context copies.
- Do not restore the upstream Tools and Plugins management screen.

## Update and uninstall

The app checks GitHub Releases once at startup. When a new version exists, a banner appears at the sidebar foot; Update and Restart downloads, verifies the signature, installs, and relaunches. The sidebar foot always shows the installed version with a manual Check for updates action that also reports "up to date" explicitly. A failed startup check (for example offline) stays silent. Windows machines that installed an older `.msi` should uninstall it once before installing `-setup.exe` to avoid a duplicate Apps entry.

User settings live under `~/.codex-session-delete/` and survive app replacement. You can decide separately whether to retain that directory when uninstalling.

## Known limitations

- There is no Intel build, Developer ID signature, or Apple notarization yet.
- Windows builds are produced by CI and have not been manually tested on a real Windows machine.
- Credential-bearing official refresh currently requires an embedded `codex-cli` at or above `0.147.0-alpha.1`. macOS verification covers OpenAI Team ID `2DC432GLL2`; keyring-only or otherwise unreadable credential stores are unsupported.
- Windows has an Authenticode/OpenAI publisher gate, but its live OAuth refresh path has not yet been verified on a physical Windows machine.
- Chat Completions and aggregator profiles depend on the upstream launcher's proxy at `127.0.0.1:57321`. Codex Minus does not ship that proxy, so these profiles should not be used. A server-side composite that exposes one Responses Base URL and Key to Codex is an ordinary pure API upstream and is not affected.
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

- macOS: `src-tauri/target/release/bundle/macos/Codex Minus.app`
- Windows: `src-tauri/target/release/bundle/nsis/*.exe`

## License

AGPL-3.0-only, inherited from the upstream project.
