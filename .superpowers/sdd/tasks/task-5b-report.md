# Task 5b implementation report

## Changes

- OpenSpec 5.8 is complete through `0189c3b` (`6f77132`, `133391b`, `0189c3b`). Existing protocol／mode exits are revisioned backend transformations with in-memory capability-loss or destructive previews; synchronous TypeScript rebuilding is not used.
- Only the exact backend blocker for the requested exit creates a confirmation. Structured-key／provider-bearer conflicts, actor conflicts, unknown blockers, and multi-blocker responses remain blocked, cannot loop through the wrong confirmation, and prevent Save／SetCurrent.
- Chat Completions confirmation removes only the Manager-owned actor marker, preserves the bearer and unowned provider／header／TOML fields, and drops the managed catalog draft from the eventual commit envelope. Pure API／legacy exits preserve unowned fields; pure OAuth requires a destructive preview and removes the complete selected custom-provider table only after confirmation.
- The production Tauri transform command derives catalog ownership from persisted settings plus catalog state. Persisted external ownership overrides a forged managed request mode and blocks all ordinary exits before transformation; missing, invalid, newer-version, or profile-mismatched ownership returns a sanitized typed failure. The pure transformer remains filesystem-free.

## Verification

- Fresh fix review: Spec PASS／Quality PASS; Critical 0／Important 0／Minor 0.
- Frontend suite: 89／89; `npm run check` passes.
- Provider native-capability Rust suite: 24／24; `cargo check`, `cargo fmt --check`, and scoped diff checks pass.
- Full Rust library: 128 passed／1 approved live-OAuth test ignored.
- Regressions cover preview and cancel without persistence, pending confirmation commit gates, typed blocker matching, external-mode forgery, absent／invalid／newer／mismatched catalog state, valid managed exits, unowned-field preservation, raw-source trust, and secret placement.

## Not verified

- No Tauri bundle build, package, install, deployment, release, network request, live OAuth refresh, image-generation probe, or real auth／config mutation was performed.
- No manual GUI rapid-click or full external／key-conflict confirmation flow was executed.

## Remaining risks

- No known Task 5 implementation risk remains. Task 6 still owns the explicit upgrade／exit capability-status UX and product policy; this slice did not change defaults or evaluator semantics.
- A read-only preview can observe settings and catalog state on adjacent atomic generations, but every eventual write remains subject to the existing ProviderCommit fingerprint／generation CAS and external-adoption boundary.
