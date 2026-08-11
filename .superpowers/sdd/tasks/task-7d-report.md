# Task 7d implementation report

## Changes

- OpenSpec 7.5 is complete through `06c9bf5`. `adopt_external_model_catalog` remains the sole specialized external-ownership transfer path, with commit bound to the exact reviewed source hash, target client version, and version status; a mismatch additionally requires explicit acceptance.
- The existing adoption checks were extracted into one private pure predicate without changing preview, collision, Context confirmation, backup, materialization, or commit ordering.
- Direct unified ProviderCommit requests cannot forge an external-to-managed transition. Rejection is typed `InvalidDraft` and introduces no adoption backup, managed artifact, pointer change, or other side effect.

## Verification

- Focused reviewed-tuple binding passes 1／1; external-boundary tests pass 7／7; ProviderCommit transactions pass 40／40; full Rust library tests pass 138 with one approved live-OAuth test ignored; integration suites pass 2／2, 14／14, and 24／24; frontend tests pass 136／136.
- The transaction regression captures settings, catalog state, live config, auth, and the external source file and proves complete byte identity after rejected ordinary Save.
- `npm run check`, `cargo check`, `cargo fmt --check`, handler allowlist and cumulative diff checks, and strict OpenSpec validation pass. Fresh review: Spec PASS／Quality PASS; Critical 0／Important 0／Minor 0.

## Not verified

- The approved ignored test requiring a real OAuth catalog-refresh request was not enabled. No Tauri bundle, package, install, deployment, manual GUI flow, real network request, or live config／auth mutation was performed.

## Remaining risks

- No known Task 7.5 implementation risk remains. Runtime restart fingerprint semantics remain separately scoped Task 7.6–7.9 work.
