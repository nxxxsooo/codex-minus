# Tasks — add-active-only-session-adaptation

## 1. Backend

- [x] 1.1 `session_adaptation.rs`: per-session rollout rewrite + per-id sqlite update, backup-first with manifest and marker-scoped pruning, mtime preservation, locked-file skip, restore-on-partial-failure, encrypted_content detection. Tests prove: active mismatched sessions adapt, an archived mismatched session's file and row are byte-for-byte untouched, backups carry the originals, and the encrypted flag surfaces. (rusqlite moved from dev-deps to deps — same 0.32 the pinned core links.)
- [x] 1.2 `adapt_active_sessions_to_current_provider` becomes real behind the existing scan-generation CAS; the compatibility payload reports availability and a message describing the active-only write; old settings state without the new toggle field still loads (test).

## 2. Frontend

- [x] 2.1 Compatibility card: enabled adapt button (existing gating flips on), toggle row「切换供应商后自动适配」saving immediately through the lifecycle settings command; i18n entries. (`saveSessionLifecycle` refactored to take a partial patch so the feature fits inside the 3500-line shell ceiling — App.tsx landed at 3490.)
- [x] 2.2 After a successful provider switch with the toggle on: fresh scan → adapt with that scan's generation → passive notice with the outcome; a failure never blocks the switch.

## 3. Verification

- [x] 3.1 `npm run verify` (206 tests), Rust suites (172+17+21), `cargo fmt --check` green; `openspec validate --strict` passes; three-platform CI green on the PR.
- [ ] 3.2 On-screen: with two mismatched active sessions, click adapt — count drops to zero, backup dir exists, archived count untouched; switch providers with the toggle on and watch the automatic pass report.
