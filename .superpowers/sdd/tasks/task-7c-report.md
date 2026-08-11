# Task 7c implementation report

## Changes

- OpenSpec 7.4 is complete through `7050de7`. Catalog readiness now uses one stable internal action code and an exact equality predicate; recovery clears only that catalog-readiness action and preserves unrelated actions.
- A later valid official-refresh materialization clears readiness even when the effective artifact hash is unchanged, without changing its bytes, generation, or existing restart marker. Production continues to use the real target-CLI offline validator; the synthetic validator is private to module tests.
- A later valid ProviderCommit materializes the inactive profile and clears readiness while leaving the active provider and live config unchanged; activation happens only after a separate explicit SetCurrent retry passes the remaining gates.

## Verification

- Focused refresh-materializer and ProviderCommit recovery regressions pass 2／2; ProviderCommit transactions pass 39／39; planner tests pass 24／24; catalog tests pass 27 with one approved live-OAuth test ignored.
- Full Rust library tests pass 136 with one approved live-OAuth test ignored; integration suites pass 2／2, 14／14, and 24／24; frontend tests pass 136／136.
- `npm run check`, `cargo check`, `cargo fmt --check`, cumulative diff checks, and strict OpenSpec validation pass. Fresh review: Spec PASS／Quality PASS; Critical 0／Important 0／Minor 0.

## Not verified

- The approved ignored test requiring a real OAuth catalog-refresh request was not enabled. No Tauri bundle, package, install, deployment, manual GUI flow, real network request, or live config／auth mutation was performed.

## Remaining risks

- No known Task 7.4 implementation risk remains. External adoption and runtime restart fingerprint semantics remain separately scoped Task 7.5–7.9 work.
