## 1. Compatibility Spikes

- [x] 1.1 Discover the target-matched Codex CLI on supported macOS installations and test capability detection for `archive` and `unarchive` without mutating real sessions.
- [x] 1.2 Determine capability support at runtime and add a safe loaded/running-state check; document when automatic archive batches must defer.
- [ ] 1.3 Add active-only scope to the upstream provider-sync API with tests, then bump the pinned `codex-plus-core` and `codex-plus-data` revision without vendoring provider logic.

## 2. Settings And Session Queries

- [x] 2.1 Add tolerant settings fields for archive enablement, retention days, first-run consent, last completed check, and compatibility migration from legacy provider-target fields.
- [x] 2.2 Replace the all-session response with archive-filtered, cursor-based backend pagination and lightweight active and archived counts.
- [ ] 2.3 Add read-only archive candidate discovery using last activity, archive state, and in-use state, including skip reasons and a preview result.
- [x] 2.4 Add read-only active-session provider mismatch discovery keyed by the effective live provider and a stale-result generation token.

## 3. Native Archive Lifecycle

- [x] 3.1 Implement the target-Codex archive adapter with effective `CODEX_HOME`, UUID-only invocation, capability checks, timeouts, and redacted diagnostics.
- [x] 3.2 Implement per-session eligibility rechecks, operation locking, native archive invocation, and rollout/state postcondition validation.
- [x] 3.3 Implement native unarchive with postcondition validation and refreshed retention activity.
- [ ] 3.4 Implement partial-batch summaries and an inventory parity check that reports inconsistencies without direct SQL repair.
- [x] 3.5 Schedule consented maintenance after UI readiness, coalesce overlapping triggers, enforce the 24-hour interval, and catch up on later launches.

## 4. Provider Compatibility Workflow

- [x] 4.1 Capture the effective provider before and after each successful relay switch and skip compatibility work when the identity is unchanged.
- [x] 4.2 Implement read-only active-only compatibility scans that never traverse archived rollouts and reject stale scan generations before adaptation.
- [ ] 4.3 Implement explicit active-session adaptation through the scoped upstream API with backups, rollback behavior, in-use skips, and encrypted-content warnings.
- [ ] 4.4 After native unarchive, check and optionally adapt only the restored session without scanning archived history.
- [x] 4.5 Remove manual and saved provider-target values from normal behavioral decisions while retaining backward-compatible settings reads.

## 5. Frontend Experience

- [x] 5.1 Split session management into active and archived views backed by independent pagination and stable counts.
- [x] 5.2 Add the 30-day policy preview, enablement control, retention setting, pending status, last-run status, and archive result summary.
- [x] 5.3 Add per-session native archive and restore actions with in-use, failure, and post-restore compatibility states.
- [x] 5.4 Remove the provider target selector and replace it with current-provider identity, mismatch count, warning state, and a capability-gated adaptation action.
- [x] 5.5 Ensure archive maintenance and compatibility scans do not resize, block, or delay the first usable session page.

## 6. Verification And Documentation

- [ ] 6.1 Add Rust tests for eligibility cutoffs, consent, target CLI discovery, native postconditions, partial failures, locks, stale activity, and archive parity failures.
- [ ] 6.2 Add provider tests proving same-provider switches perform zero session scans, active-only adaptation never opens archived rollouts, and restored-session adaptation is single-session scoped.
- [ ] 6.3 Add frontend tests for first-run consent, active and archived pagination, removed target selection, stale scan handling, confirmations, and partial-result rendering.
- [ ] 6.4 Benchmark first active-page readiness and provider scan/adaptation duration before and after the change; record counts and avoid unsupported client-launch claims.
- [x] 6.5 Run `npm run check`, `npm run vite:build`, `cargo test` in `src-tauri`, and a full Tauri build.
- [ ] 6.6 Exercise real archive, restore, same-provider switch, changed-provider scan, declined adaptation, and confirmed active-only adaptation on disposable sessions through the packaged app.
- [ ] 6.7 Update README and BOARD.md with the consent model, native archive location, restore behavior, provider-linkage workflow, measured performance evidence, and remaining compatibility limits.
