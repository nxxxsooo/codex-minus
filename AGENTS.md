# codex-minus (Codex Minus)

## Overview
Trimmed fork of upstream Codex++ Manager (`BigPizzaV3/CodexPlusPlus`, `apps/codex-plus-manager`): relay provider switching + model catalog management + session management + config doctor only. Tauri 2 + React 19 + Vite. No renderer injection, no launcher. AGPL-3.0-only. Installed as `/Applications/Codex Minus.app`; replaced Codex++ (2026-07-15) and renamed from `Codex-- Manager` (2026-08-12), so an older bundle can remain until it is removed by hand.

## Architecture
- **Frontend**: `src/App.tsx` — single-file SPA, dual-mounted screens (relay / sessions / doctor), v1.2.35 green theme
- **Backend**: `src-tauri/src/commands.rs` — all Tauri commands; IO-heavy commands are async (off main thread)
- **Live state**: `src-tauri/src/live_state.rs` — process-wide coordinator, owner-only multi-file transaction journal, and crash recovery for settings/config/catalog generations
- **Model catalogs**: `src-tauri/src/model_catalog.rs` — verified target-CLI official refresh, four catalog modes, overlays, provider evidence, materialization, and external ownership
- **Upstream deps**: `codex-plus-core` / `codex-plus-data` as git deps pinned to rev `59a2f90` in `src-tauri/Cargo.toml`. Upgrade = bump the rev; do NOT vendor or fork provider logic.
- **Settings store**: `~/.codex-session-delete/settings.json` (inherited from Codex++, now owned by codex-minus; relay profiles live here)

## Hard Constraints
- **Context 保护罩** (commands.rs): provider switch/apply/clear, active saves, and catalog-pointer writes use the process-wide coordinator plus a fail-closed Context transaction — snapshot `mcp_servers` / `skills` / `plugins` tables of `~/.codex/config.toml` before write, re-graft verbatim via toml_edit after, and roll back the full prior generation on failure. Never bypass it when adding write paths to config.toml. Root cause: Codex++'s managed context copies ate `[mcp_servers.memory]` (2026-07-15 incident).
- **OAuth ownership**: ChatGPT OAuth remains owned by the official Codex/ChatGPT client. Provider profiles must not persist, apply, or restore `authContents`; backend raw `auth.json` writes remain rejected. Official catalog refresh may project only non-refreshable access/ID tokens into a private temporary `CODEX_HOME` and must leave live auth byte-for-byte unchanged.
- **Model catalog ownership**: managed profiles use `native-official`, `official-plus-custom`, `custom-only`, or explicit `external` mode. Provider `/v1/models` is evidence only; official baselines require a platform-verified target CLI. External catalog paths and files remain untouched until explicit adoption.
- **Native-capability contract ownership**: the canonical mixed contract is provider name `OpenAI`, `wire_api = "responses"`, `requires_openai_auth = false`, provider bearer, and one exact actor-authorization header. It changes only through an explicit, previewed, revisioned commit on that one profile. Startup, inspection, and a commit focused on another profile must never rewrite it — in particular, do not run `normalize_relay_profile_for_storage` outside the focused profile: the pinned core rewrites a legacy alias (`CodexPlusPlus`, `CodexPP`) or reserved id to its own `custom` shape, drops the replaced table's actor header, and defaults `requires_openai_auth` back to `true`. The actor marker asserts client eligibility only; capability rows (text Responses, model discovery, image generation, image editing, remote compaction, web search) are row-scoped, independently observed, and default to unknown. Never claim a plan upgrade or a blanket grant of Pro capabilities in UI copy.
- **Chat Completions / client-side aggregate providers are a dead path**: member rotation and protocol conversion depend on the removed launcher's local proxy at `127.0.0.1:57321`. codex-minus does not ship that proxy. Keep those options with in-editor warnings; do not remove or implement the proxy. A server-side composite that exposes one Responses Base URL and key is an ordinary pure API upstream and may use managed catalogs.
- **toml_edit gotcha**: implicit tables (containing only sub-tables) render as empty string when `to_string()`-ed alone; to compare table contents, graft into a temp `DocumentMut` and render whole.

## Commands
- `npm run check` — tsc --noEmit
- `npm run vite:build` — frontend build
- `npm run build` — full tauri build (macOS app bundle + icns)
- `cargo test` in `src-tauri/` — includes Context, live-state transaction, and model-catalog tests; the live OAuth test remains ignored unless explicitly enabled

## Role Separation
- Completed-work history → `BOARD.md` (append-only changelog)
- Active tasks → OmniFocus (`Personal/Projects-Dev`)
