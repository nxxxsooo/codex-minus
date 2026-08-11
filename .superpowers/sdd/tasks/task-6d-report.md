# Task 6d implementation report

## Changes

- OpenSpec 6.9 is complete through `7d638d3`. A pure presentation classifier derives only from response-only native-capability inspection metadata: external ownership has first precedence, ordinary non-external pure OAuth displays `native-official`, and external pure OAuth or mixed profiles display `external`.
- Pure API, Chat Completions, aggregate, unsupported relay modes, legacy provider aliases, and the classic legacy contract display as advanced compatibility paths. External mixed profiles keep the existing unavailable／null ordinary upgrade action; native-priority, eligible-upgrade, and degraded native-family states keep their existing state labels.
- The App adds badges and Chinese／English explanatory copy only. It does not modify catalog defaults or drafts, profiles, settings, ProviderCommit, transforms, Save／SetCurrent, migration, or persistence behavior, and it does not rewrite existing profiles.

## Verification

- Fresh final scoped review: Spec PASS／Quality PASS; Critical 0／Important 0／Minor 0.
- Frontend suite: 134／134 across 25 suites; focused mode-presentation／wiring: 15／15; `npm run check` and scoped diff checks pass.
- Rust native-capability evaluator integration: 14／14; draft-transformer integration: 24／24; full Rust library: 128 passed／1 approved live-OAuth ignore; `cargo check` and `cargo fmt --check` pass.
- Tests cover external-first precedence across apparent states, ordinary pure OAuth, pure API, Chat Completions, aggregate, unsupported relay mode, legacy alias, classic legacy contract, external no-upgrade behavior, response-only App wiring, and aggregate label-only presentation.

## Not verified

- No Tauri bundle, package, install, deployment, release, rendered GUI interaction, real OAuth／upstream request, image request, or live config／auth mutation was performed.
- The display was verified through pure state and source-wiring tests rather than a mounted React or manual visual test.

## Remaining risks

- No known implementation risk remains within OpenSpec 6.9; Task 6 is complete.
- These labels are presentation only. Catalog readiness, active／inactive commit behavior, restart semantics, and runtime capability proof remain independently governed by Task 7 and later evidence gates.
