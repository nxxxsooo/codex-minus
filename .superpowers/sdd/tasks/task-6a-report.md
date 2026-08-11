# Task 6a implementation report

## Changes

- OpenSpec 6.1–6.4 are complete through `e7490fb`. Provider Detail consumes backend inspection only as response metadata and exposes explicit draft actions for native-priority upgrade, compatibility exits, custom Actor-header replacement, and legacy provider-ID migration.
- Ordinary Responses controls never synthesize native enablement. Existing TOML changes remain revisioned backend transforms with an in-memory preview; cancel, close, navigation, startup, inspection, Doctor, and evidence refresh produce no provider commit or live write.
- A conflicting custom Actor value requires the exact `replaceActorHeader` confirmation. A legacy alias moves to `custom` only when safe; a different existing `custom` table requires a user-chosen unused, non-reserved ID and never permits overwrite or merge. External ownership remains unavailable to the ordinary upgrade path.
- Save and SetCurrent fail closed during transforms, raw validation, confirmations, legacy resolution, or backend blockers. Matching current transport errors restore the explicit action／retry state without adopting transformed data; stale session／profile／revision errors remain ignored.

## Verification

- Fresh final review: Spec PASS／Quality PASS; Critical 0／Important 0／Minor 0.
- Frontend suite: 106／106; `npm run check` passes.
- Rust evaluator suite: 14／14; Rust draft-transformer suite: 24／24; `cargo fmt --check` passes.
- Scoped regressions cover actor replacement confirmation, legacy collision and chosen-ID retry／cancel, external precedence, pure OAuth destructive preview, pure API／legacy unowned-field preservation, zero-effect cancellation, stale response rejection, transport-error recovery, and independent Save／SetCurrent commit gates.

## Not verified

- No Tauri bundle build, package, install, deployment, release, real network request, OAuth refresh, image-generation probe, live config／auth mutation, or rendered GUI interaction was performed.
- Task 6.5–6.9 evidence-ledger, truth-table, Doctor semantics, explanatory copy, and final mode-presentation work remain pending.

## Remaining risks

- No known implementation risk remains within OpenSpec 6.1–6.4.
- Capability availability is still inspection evidence rather than proof of upstream, model, account-plan, or runtime success. Task 6.5–6.9 must preserve those independent unknown／blocked states and must not infer entitlement from the Actor marker or a paid plan.
