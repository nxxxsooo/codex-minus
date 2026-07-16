# codex-minus (Codex-- Manager)

## Overview
Trimmed fork of upstream Codex++ Manager (`BigPizzaV3/CodexPlusPlus`, `apps/codex-plus-manager`): relay provider switching + session management + config doctor only. Tauri 2 + React 19 + Vite. No renderer injection, no launcher. AGPL-3.0-only. Installed as `/Applications/Codex-- Manager.app`; replaced Codex++ (2026-07-15).

## Architecture
- **Frontend**: `src/App.tsx` — single-file SPA, dual-mounted screens (relay / sessions / doctor), v1.2.35 green theme
- **Presets**: `src/presets.ts` — provider presets consumed by `src/components/ProviderPresetSelector.tsx` (App.tsx:~2166). NOT dead code.
- **Backend**: `src-tauri/src/commands.rs` — all Tauri commands; IO-heavy commands are async (off main thread)
- **Upstream deps**: `codex-plus-core` / `codex-plus-data` as git deps pinned to rev `59a2f90` in `src-tauri/Cargo.toml`. Upgrade = bump the rev; do NOT vendor or fork provider logic.
- **Settings store**: `~/.codex-session-delete/settings.json` (inherited from Codex++, now owned by codex-minus; relay profiles live here)

## Hard Constraints
- **Context 保护罩** (commands.rs): provider switch/apply/clear are wrapped in `with_context_tables_protected` — snapshot `mcp_servers` / `skills` / `plugins` tables of `~/.codex/config.toml` before write, re-graft verbatim via toml_edit after. 4 unit tests pin this. Never bypass it when adding write paths to config.toml. Root cause: Codex++'s managed context copies ate `[mcp_servers.memory]` (2026-07-15 incident).
- **Chat Completions / aggregator providers are a dead path**: they depend on the removed launcher's local proxy at `127.0.0.1:57321`. codex-minus does not ship that proxy. Decision: keep the protocol option with the in-editor warning (App.tsx `relay-protocol-hint`); do not remove, do not implement the proxy.
- **toml_edit gotcha**: implicit tables (containing only sub-tables) render as empty string when `to_string()`-ed alone; to compare table contents, graft into a temp `DocumentMut` and render whole.

## Commands
- `npm run check` — tsc --noEmit
- `npm run vite:build` — frontend build
- `npm run build` — full tauri build (macOS app bundle + icns)
- `cargo test` in `src-tauri/` — includes the 4 context-guard tests

## Role Separation
- Completed-work history → `BOARD.md` (append-only changelog)
- Active tasks → OmniFocus (`Personal/Projects-Dev`)
