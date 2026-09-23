# Design / Build Contract

**Status: confirmed for implementation.** On 2026-09-22 the user explicitly selected “确认，开始实现” for this contract, including the visual baseline, Electron/Rust private transport, data protection, signed updates and platform packaging. The first implemented provider workflow remains the agreed review milestone before wider UI migration.

**Scope update (2026-09-23):** The user requested that open PR #48 (permanent session cleanup) and PR #49 (long-context and image-tool controls) be integrated into `codex/electron-redesign` before finishing the migration. The two PR commits have been locally merged after safeguarding the existing uncommitted Electron/model work; public PR merges, release and installed-app replacement remain separate.

**Release update (2026-09-23):** After authorizing publication of the completed Electron application, the user explicitly chose **v0.5.0 with automatic upgrade from the old Tauri version**. Add a tested legacy feed bridge; replacing the maintainer's installed app remains outside this authority.

## Context

See `proposal.md` for scope and approval history. The source baseline is v0.4.18 (`60fa8ab`). The user has requested continued development after delivery of the Stitch suite; the material implementation choices are presented here for confirmation. The existing renderer is React 19/Vite/Tailwind; `App.tsx` is a 3212-line-budget wiring shell. Its domain calls currently cross Tauri IPC. The Rust domain core owns validation, config protection, journals, catalogs and sessions.

The current macOS bundle is ad-hoc signed and the update trust root is a pinned minisign public key. Electron/Squirrel's ordinary macOS updater requires a signed app; replacing the existing signed-artifact contract with an assumed working Squirrel updater would be a regression.

## Goals / Non-Goals

**Goals:** a locally reviewable Electron build with the selected Bauhaus direction, existing domain behavior, a bounded IPC/lifecycle boundary, preserved data locations and a verified signed-artifact update path. First deliver a real provider edit/save/current workflow for the agreed milestone review.

**Non-goals:** new provider protocols, an inference proxy, backend reimplementation, new account ownership, replacement of the maintainer's installed app, or silently changing unrelated settings/catalog contracts.

## Decisions

### Desktop and backend boundary

- Electron main owns the window, tray/menu, single-instance activation, restricted local asset protocol, native lifecycle and update orchestration.
- A sandboxed, context-isolated preload exposes a versioned, allowlisted API to the app renderer; `nodeIntegration` remains off. Main validates the sending `WebContents`, main frame and allowed origin for every privileged call. External navigation/new windows and unsolicited permissions are denied; explicit http/https links use the existing approved action.
- The Rust crate becomes a standalone `codex-minus-core` executable. Retain its current `src-tauri/` directory initially to avoid mixing a repository-wide path migration into behavioral changes; the historical directory name does not imply a remaining Tauri runtime dependency.
- Replace Tauri wrappers/runtime calls with Tokio and a static command dispatcher, preserving domain implementations and DTOs. The exposed command allowlist remains the current registered commands, with only narrow handshake/recovery and update-verification facilities added. Legacy bypass methods stay unregistered.
- Use private stdin/stdout NDJSON, not an HTTP listener for application RPC. Request `{id, command, args}`; response `{id, result}` or `{id, error: {code, message}}`; lifecycle events are separate typed frames. Request IDs correlate responses only and never replace provider generation CAS/draft revisions. Envelope/command validation fails before execution.
- Bound frame size, pending requests and startup/request deadlines. Never log raw frames, arguments, stdout or stderr. Panic/transport errors use static messages. A backend death rejects all pending promises and does not replay requests. Main controls executable path; renderer cannot choose a binary or invoke arbitrary shell commands.

**Alternatives:** a Node rewrite would duplicate the hard-won transaction/auth rules; an HTTP service would create another local attack/lifecycle surface. A native addon would couple backend lifetime/crashes to the Electron process and add ABI-specific packaging. Private pipes preserve the existing Rust ownership with a smaller boundary.

### Single writer, shutdown and recovery

- Electron's native instance lock focuses a second Electron launch. The Rust process also acquires the pinned core's legacy manager guard, so an old Tauri build and a new Electron build cannot write concurrently.
- Ownership failures are fail-closed, including unexpected errors: do not use the old emergency random-port fallback. The legacy guard port is an ownership primitive, not the removed protocol proxy and not a general RPC endpoint.
- Preserve legacy activation compatibility where supported through the existing owner-only activation socket; relay focus events to Electron. Windows keeps the installed executable identity so native activation still works.
- Acquire ownership before domain startup/migration. Keep one backend per main session. Normal quit closes its input and permits accepted work to settle; forced termination is bounded and relies on the existing recovery journal, never a fabricated successful response.
- Explicit reconnection starts a new owned process, runs existing recovery and obtains an authoritative baseline. In-flight mutation outcome remains unknown until reconciliation. The UI retains the draft and never automatically retries a mutation.

### Data and verification isolation

- Keep `~/.codex-session-delete/` and the existing Codex-home resolution. Renderer-local theme/language/route preferences are independent of domain data; no new settings schema or ownership migration is introduced.
- A development/test launch supplies an isolated home and Codex home. The backend verifies that both resolved state and Codex locations are inside the requested root before acquiring guards, opening settings or running startup maintenance. If platform path resolution ignores the requested test root, reject that test launch instead of touching live state.
- Automated visual checks run headlessly when possible. Isolated native Electron tests position an inactive window on a detected non-primary display before showing it; without one they fail closed and report native-only gaps instead of using the primary display. This follows the workspace-wide `AGENTS.md` rule.
- Use temporary fixture profiles, auth sentinels, external-catalog sentinels and protected Context tables for real IPC workflow tests. Test network calls use a controlled local Responses fixture when needed. Do not copy live credentials into fixtures.

### Update and packaging contract

- Preserve minisign verification against the existing pinned key. Electron updates use an explicitly Electron-marked manifest/feed so a local v0.4.18 review build cannot accidentally install a Tauri artifact from the legacy feed. A check does not install anything.
- Main downloads a platform/architecture-matched newer artifact to private staging. Rust validates signatures; no archive extraction or installer execution precedes that verification. Invalid signatures, malformed metadata, wrong runtime/target and downgrades fail closed.
- Authenticate `electron-latest.json` itself with the same minisign trust root, binding runtime, version, architecture and artifact digest to the publisher. Artifact-only signatures cannot prevent relabeling an older signed payload.
- macOS uses a staged external installation helper with bounded parent-exit waiting, candidate-bundle identity checks, path-safe extraction, replacement backup and verified rollback on replacement failure. The helper must survive app-directory replacement. Writable-location errors produce actionable guidance; no speculative privilege escalation.
- Windows uses the verified NSIS artifact with existing product identity and preserved data directories. The manager exits only after the helper/installer handoff is established. Codex host restart remains a separate explicit user action.
- Package macOS arm64 and Windows x64/arm64, bundling the matching Rust binary outside ASAR. Pin Electron and builder versions in the lockfile. Maintain product name `Codex Minus`, executable `codex-minus` and app id `fun.mjshao.codex-minus`.
- The external helper runs from a full copied runtime, acknowledges readiness before main may quit, and copies candidate bundles onto the destination volume. macOS uses `RENAME_SWAP` through a private non-RPC Rust entry point; interruption leaves both complete generations accessible, and post-check failure exchanges them back. Electron's original filesystem API is required when inspecting/copying physical `app.asar` bytes.
- The authorized publication is v0.5.0 through the protected PR/CI/merge/tag process, after final acceptance. The old `latest.json` points to the same signed Electron artifacts for one-time migration; future Electron checks use their independent authenticated feed. Verify the old macOS updater's actual extraction/replacement code using its pinned upstream library in an isolated headless test harness. Windows NSIS must recognize the old `/P /R /UPDATE /ARGS` invocation, reuse the registered install location, retire only the old application uninstaller record after success, preserve data, and restart the manager when requested. Native Windows x64 CI must exercise an actual old installer followed by this handoff; ARM64 build evidence remains explicitly distinct from native execution.

### UI translation and review milestone

- Visual source: user-selected SCREEN_8 / Light Mode Bauhaus and the 36 IDs in `Designs/personal/codex-minus/bauhaus/manifest.json`. Generated screens provide composition/tokens, not business logic or unverified runtime facts.
- Rebuild with the existing React stack and shared primitives. Keep pure rules beside their tests, reduce the App shell budget as code moves out, and keep its rule allowlist shrinking. Do not copy generated CDN HTML into the production renderer.
- Default new appearance to the selected light palette, respecting existing explicit light/dark preferences. Both themes and Chinese/English remain supported; bundle fonts locally. Use paper/near-black surfaces, compact black boundaries and restrained yellow/red/blue accents, with semantic status text rather than color alone.
- Preserve two primary work areas, providers and sessions. Preferences expose existing appearance/language/version/update/restart controls. Separate draft/saved/current/restart states and preserve the valid existing-draft atomic Set-current flow.
- Session deletion now uses PR #48's explicit, backup-free permanent command for single/selected entries and all archived pages; the renderer and RPC allowlist must not re-expose the retired backup-based delete path. The clear-all preview scans all pages before confirmation and rechecks archived-only eligibility during deletion. PR #49's per-provider image flag owns only `features.image_generation` and uses the explicit pure-API transform; long-context controls edit only eligible visible model rows, preserving the catalog and Context transaction boundaries. The later model-list correction adds source-marked Sol/Luna cards where the verified bundled official baseline does not yet contain those slugs.
- An unsaved new provider remains a brand-new draft after its structured fields generate TOML. A mixed→pure API→mixed target round trip must discard only model copies created by that target switch, carrying edited names/windows as field-scoped official overrides rather than full custom rows that erase official reasoning metadata. Manually added or unknown-provenance model rows remain owned. Cleanup notices render after their status refresh so partial failure IDs cannot be replaced by a later refresh error.
- Start with B01–B06 plus model editing and the minimal meaningful transport/error states. Verify on the actual Electron runtime and isolated backend. Present this implemented workflow for the agreed user review before migrating the remaining screen families.
- Target 1180 × 820 and 960 × 720 initially, with 1280 × 820 reference comparison, native platform controls, keyboard focus, dialog focus return, reduced motion and readable long translated labels. UI inspection images belong in Designs.

## Evidence path and must-pass scenarios

| Claim | Owner / minimum evidence |
|---|---|
| Protocol preserves actual payloads and rejects unknown/bypass commands | Rust dispatcher tests plus real executable round trip; static registration audit updated to the real dispatcher |
| Every pending request settles on timeout/death/corruption | Node client tests with controlled fake child streams and one killed real fixture child; no request replay |
| Current and legacy managers cannot concurrently mutate data | Guard contention test using the actual pinned guard and isolated root; second instance rejected before maintenance |
| No live user data is used in acceptance | Resolved-path assertion before startup, temp fixture roots and outside-root sentinel verification |
| Save/Set-current preserve transactions and ownership | Existing Rust transaction/CAS/failure-injection suite, plus real IPC edit/save/activation fixture checking settings, catalog, Context, auth and external file bytes |
| Bauhaus workflow is usable in Electron | Actual launch and provider workflow at target sizes, light/dark, keyboard/dialog/error checks and first-slice review |
| Remaining UI actions retain semantics | Existing frontend rule/wiring regressions, PR #48's SQLite/rollout cleanup suite and all-page UI flow, PR #49's image/long-context and Context transaction suite, plus actual isolated Electron UI checks after migration |
| Updates reject invalid artifacts and recover interrupted replacement | Signature fixtures, wrong-runtime/target/downgrade cases, disposable app/installer staging and rollback tests |
| Packaged build resolves its own backend/assets | macOS review bundle smoke test and platform CI packaging checks; distinguish Windows build evidence from native Windows runtime evidence |

Required completion checks: `npm run verify`, frontend build, full Rust tests, relevant Electron/IPC/update tests and a local packaged launch. Do not repeat broad suites without changed evidence or an unresolved failure.

## Implementation slices

1. **Runtime and first provider workflow:** detach the core; add transport, ownership and Electron shell; implement the provider visual slice; test real isolated save/activation and show it for review.
2. **Remaining application workflows:** session lifecycle, model states, Doctor, preferences, shared accessible confirmations/errors, localization and theme completion.
3. **Distribution and update closure:** verified updater, matching packaged helper/core, CI/release/version changes, disposable install/rollback evidence and final review build.

## Risks / Trade-offs

- Chromium increases download/memory footprint → measure the actual packaged review build; do not substitute assumptions or DOM-query timings.
- Pipe loss cannot prove a mutation did not happen → retain uncertainty, serialize recovery and refresh a trusted baseline before new writes.
- Two runtime generations can coexist on disk → share the existing ownership guard and avoid touching live installation in tests.
- Generated drafts contain occasional invented controls or metadata → source contracts and the coverage notes take precedence during implementation.
- macOS ad-hoc signing does not fit the stock automatic updater → retain pinned artifact verification and test the bounded replacement helper.
- Native Windows runtime is unavailable on this host → perform cross-platform source/build checks and record the missing native lifecycle/install verification explicitly.

## Migration and rollback

Build in the dedicated worktree. Keep app identity and domain data unchanged. Review builds run with isolated data. Commit/version the runtime transition as a coherent package; an application-binary rollback keeps the same data schema and does not restore OAuth. Before the authorized v0.5.0 publication, test the old installation handoff and new signed feed using disposable packages and the ordinary release process. Do not replace the maintainer's installed app.

## Source references

- Electron security and IPC guidance: https://www.electronjs.org/docs/latest/tutorial/security
- Electron updater platform constraints: https://www.electronjs.org/docs/latest/api/auto-updater
- Existing `src-tauri/src/lib.rs`, command wrappers, `live_state.rs`, pinned core `ports.rs`, and the existing updater's minisign verification establish the project-specific preservation boundary.
