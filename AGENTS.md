# codex-minus (Codex-- Manager)

## Overview
Trimmed fork of upstream Codex++ Manager (`BigPizzaV3/CodexPlusPlus`, `apps/codex-plus-manager`): relay provider switching + model catalog management + session management + config doctor only. Tauri 2 + React 19 + Vite. No renderer injection, no launcher. AGPL-3.0-only. Installed as `/Applications/Codex-- Manager.app`; replaced Codex++ (2026-07-15).

## Architecture
- **Frontend**: `src/App.tsx` — single-file SPA, dual-mounted screens (relay / sessions / doctor), v1.2.35 green theme
- **Presets**: `src/presets.ts` — provider presets consumed by `src/components/ProviderPresetSelector.tsx` (App.tsx:~2166). NOT dead code.
- **Backend**: `src-tauri/src/commands.rs` — all Tauri commands; IO-heavy commands are async (off main thread)
- **Live state**: `src-tauri/src/live_state.rs` — process-wide coordinator, owner-only multi-file transaction journal, and crash recovery for settings/config/catalog generations
- **Model catalogs**: `src-tauri/src/model_catalog.rs` — verified target-CLI official refresh, four catalog modes, overlays, provider evidence, materialization, and external ownership
- **Upstream deps**: `codex-plus-core` / `codex-plus-data` as git deps pinned to rev `59a2f90` in `src-tauri/Cargo.toml`. Upgrade = bump the rev; do NOT vendor or fork provider logic.
- **Settings store**: `~/.codex-session-delete/settings.json` (inherited from Codex++, now owned by codex-minus; relay profiles live here)

## Hard Constraints
- **Context 保护罩** (commands.rs): provider switch/apply/clear, active saves, and catalog-pointer writes use the process-wide coordinator plus a fail-closed Context transaction — snapshot `mcp_servers` / `skills` / `plugins` tables of `~/.codex/config.toml` before write, re-graft verbatim via toml_edit after, and roll back the full prior generation on failure. Never bypass it when adding write paths to config.toml. Root cause: Codex++'s managed context copies ate `[mcp_servers.memory]` (2026-07-15 incident).
- **OAuth ownership**: ChatGPT OAuth remains owned by the official Codex/ChatGPT client. Provider profiles must not persist, apply, or restore `authContents`; backend raw `auth.json` writes remain rejected. Official catalog refresh may project only non-refreshable access/ID tokens into a private temporary `CODEX_HOME` and must leave live auth byte-for-byte unchanged.
- **Model catalog ownership**: managed profiles use `native-official`, `official-plus-custom`, `custom-only`, or explicit `external` mode. Provider `/v1/models` is evidence only; official baselines require a platform-verified target CLI. External catalog paths and files remain untouched until explicit adoption.
- **Chat Completions / aggregator providers are a dead path**: they depend on the removed launcher's local proxy at `127.0.0.1:57321`. codex-minus does not ship that proxy. Decision: keep the protocol option with the in-editor warning (App.tsx `relay-protocol-hint`); do not remove, do not implement the proxy.
- **toml_edit gotcha**: implicit tables (containing only sub-tables) render as empty string when `to_string()`-ed alone; to compare table contents, graft into a temp `DocumentMut` and render whole.

## Commands
- `npm run check` — tsc --noEmit
- `npm run vite:build` — frontend build
- `npm run build` — full tauri build (macOS app bundle + icns)
- `cargo test` in `src-tauri/` — includes Context, live-state transaction, and model-catalog tests; the live OAuth test remains ignored unless explicitly enabled

## Role Separation
- Completed-work history → `BOARD.md` (append-only changelog)
- Active tasks → OmniFocus (`Personal/Projects-Dev`)
