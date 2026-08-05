# codex-minus — Changelog

<!-- Append-only. Newest date first. -->
<!-- Completed work only: what / why / verified / refs. -->
<!-- Todos, next steps, priorities, and status live in OmniFocus. -->

## Changelog

### 2026-08-05

- **launch**: announced Codex-- Manager to the FIT product group and directly notified Eva
  - why: introduce the simplified Codex++ workflow to the FIT team as a lower-friction, less lag-prone option for provider switching and session management
  - verified: the FIT Mingjian bot delivered the message to `AI产品数据库（飞特卡车配件）`, message `om_x100b681d61edccacc4c327205b074a4`; the user confirmed it was visible
  - refs: https://mjshao.fun/codex-minus/

### 2026-08-04

- **release**: published Codex-- Manager `v0.2.0` with Windows x86_64 support
  - why: make the manager available on Windows; the only release was macOS-only v0.1.0
  - verified: CI built and released macOS .app.zip (6.7 MB), Windows .msi (6.2 MB), and Windows .exe (4.3 MB) with SHA256SUMS; landing page, portfolio, and README all updated to reflect dual-platform; all 12 Rust tests pass on both platforms
  - refs: https://github.com/nxxxsooo/codex-minus/releases/tag/v0.2.0, `.github/workflows/build.yml`

- **infra**: added Windows x86_64 build target and cross-platform CI/CD
  - why: make Codex-- Manager available to Windows users; the existing codebase was macOS-only
  - verified: tauri.conf.json targets include `msi` and `nsis`; windows-app-manifest relaxed from `requireAdministrator` to `asInvoker`; CI workflow builds both macOS (aarch64) and Windows (x86_64), runs Rust tests on both, and attaches artifacts to tagged releases
  - refs: `.github/workflows/build.yml`, `src-tauri/tauri.conf.json`, `src-tauri/windows-app-manifest.xml`

- **launch**: published the bilingual public project surface with real application imagery, install guidance, portfolio entry, and a dedicated landing page
  - why: make the first release understandable and verifiable outside the repository while keeping the presentation faithful to the shipped macOS app
  - verified: GitHub README renders at desktop and mobile widths in light and dark modes; `mjshao.fun/codex-minus` and `mjshao.fun/work/codex-minus` are live from portfolio commit `6cf99bb`; the landing passed HTML validation, responsive browser QA, and Lighthouse scores of 98 performance with 100 accessibility, best practices, and SEO
  - refs: https://mjshao.fun/codex-minus/, https://mjshao.fun/work/codex-minus, https://github.com/nxxxsooo/codex-minus
- **release**: published Codex-- Manager `v0.1.0` for Apple Silicon macOS and refreshed the maintainer installation
  - why: establish the first immutable public application release with consumer-facing install and update paths
  - verified: annotated tag resolves to `38d24cb`; GitHub Release is public with a 6.6 MB app zip and `SHA256SUMS`; the remote-downloaded archive passed SHA-256 and strict deep codesign verification; `/Applications/Codex-- Manager.app` reports version `0.1.0`, bundle ID `fun.mjshao.codex-minus`, architecture `arm64`, and launches successfully
  - refs: https://github.com/nxxxsooo/codex-minus/releases/tag/v0.1.0, SHA-256 `982f90a54db29a354472146253786952b640f178ef88aa51d500d27a4a85e12f`

### 2026-07-25

- **feat**: added consented 30-day session lifecycle management with active / archived cursor pagination, target-matched Codex native archive and restore, delayed 24-hour maintenance, per-operation postcondition checks, and a dedicated private settings sidecar
  - why: keep the active session set manageable without inventing archive semantics or touching real session state directly
  - verified: 12 Rust tests, TypeScript check, Vite production build, full Tauri macOS bundle build, mocked Tauri browser QA in Chinese and English at desktop and mobile widths, and a real target-CLI archive / unarchive round trip in a disposable `CODEX_HOME`
  - refs: OpenSpec `streamline-session-lifecycle`, `src-tauri/src/commands.rs`, `src/App.tsx`
- **fix**: replaced independent provider target selection and full-history repair with effective-provider identity tracking plus read-only active-session mismatch scanning
  - why: provider repair belongs to a successful switch and must scale with active mismatches, not archived history
  - verified: same-provider switch payload reports no provider change; compatibility diagnostics report zero archived rollout traversal; legacy target fields remain tolerant but ignored
  - refs: OpenSpec `streamline-session-lifecycle`
- **safety**: added a fail-closed capability gate for provider adaptation because pinned upstream provider-sync has no active-only scope
  - why: the project constraint forbids vendoring provider logic, and falling back to the upstream full-history operation would reintroduce the original repair-time problem
  - verified: old full-history commands are absent from the Tauri command registry and frontend
  - refs: `src-tauri/src/lib.rs`, upstream pin `59a2f90`

### 2026-07-16 (late)

- **deploy**: published to GitHub as public repo https://github.com/nxxxsooo/codex-minus
  - why: local-only repo; AGPL derivative is fine to publish openly
  - verified: push succeeded, repo visible as PUBLIC
  - refs: none
- **chore**: rewrote git history with git-filter-repo to drop node_modules / src-tauri/target / dist blobs committed before .gitignore existed
  - why: history carried 255MB of build artifacts; unacceptable clone size for a public repo. No remote/collaborators existed, so hash rewrite was free
  - verified: .git 255MB → 1.2MB, all 9 commits preserved; pre-rewrite backup at /tmp/codex-minus-git-backup
  - refs: none
- **chore**: added AGPL-3.0 LICENSE full text
  - why: Cargo.toml declared AGPL-3.0-only but license text was missing; required for public distribution of a derivative work
  - refs: LICENSE

### 2026-07-16

- **docs**: added AGENTS.md (constraints: pinned-rev upstream deps, context 保护罩, dead 57321 protocol path, toml_edit implicit-table gotcha) and this BOARD.md
  - why: project had no on-disk constraints/history; resume flow depended on memory only
  - verified: not applicable
  - refs: AGENTS.md, memory `codex-minus` (2026-07-15)
- **chore**: confirmed `src/presets.ts` is live code (imported via `components/ProviderPresetSelector.tsx`, used at App.tsx:2166) after a false dead-code diagnosis; restored from git untouched
  - why: initial grep missed the `../presets` import path; deletion would have broken the preset picker
  - verified: `tsc --noEmit` clean, worktree clean
  - refs: src/presets.ts, src/components/ProviderPresetSelector.tsx

### 2026-07-15

- **feat**: codex-minus built and installed as `/Applications/Codex-- Manager.app`, replacing Codex++ (Codex++.app deleted)
  - why: Codex++'s "工具与插件" (context) feature stored managed MCP copies and merged them back into config.toml on provider switch — a stale copy ate `[mcp_servers.memory]` transport on 2026-07-15 morning; feature removed wholesale in the fork
  - verified: real launch log shows `store_scrubbed` fired; 4 unit tests pin the context guard
  - refs: src-tauri/src/commands.rs (`with_context_tables_protected`), commits dba979c..5ee5db4
- **feat**: sessions screen — provider-sync repair restored (async), all IO-heavy commands moved off the main thread, session list rendering paginated
  - refs: commit 5ee5db4
- **feat**: ux — dual-mount screens (no remount jank), dropped about/settings pages (test-model field moved to relay screen), restored v1.2.35 green theme; per-route data loading parallelized
  - refs: commits 72c197f, 2d14c62
- **feat**: macOS app bundle target + icns icon; frontend trimmed 7122→5024 lines (relay/sessions/context/doctor screens only)
  - verified: tsc clean, vite + cargo build green
  - refs: commits 5453bd1, 6b78c56
