# Tasks — clarify-session-screen-actions

## 1. Backend

- [x] 1.1 `run_session_archive_maintenance` takes `force`; due-decision extracted into `archive_maintenance_due` with a test proving force bypasses the interval but not the policy.

## 2. Frontend

- [x] 2.1 Archive card: single 立即检查 (forced) primary button; days input auto-saves debounced + silent, preview auto-refreshes on screen load (chained after lifecycle fetch) and after a days save; 预览/保存策略 removed with their EN entries.
- [x] 2.2 Maintenance result line renders by disposition (counts+check / warning+message / info+message); the two deferral messages join `EN_BACKEND`; `data-tone="warn"` styling.
- [x] 2.3 Compatibility card: 适配到当前 provider is the filled primary; local sessions card drops 刷新 and gains progressive selection with 取消.
- [x] 2.4 Update banner: immediate downloading phase on click + re-entry guard; reducer test pins that a later `Started` supersedes the synthetic phase.

## 3. Verification

- [x] 3.1 `npm run verify` (207 tests), Rust 173+17+21, `cargo fmt --check`, `openspec validate --strict` green.
- [ ] 3.2 On-screen: archive card shows one button and the days field saves on its own; a manual 立即检查 inside the daily interval actually runs; with Codex running the line shows the deferral warning, not a green zero-count; the update banner reacts on first click.
