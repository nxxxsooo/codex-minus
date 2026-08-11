# Task 6c implementation report

## Changes

- OpenSpec 6.7–6.8 are complete through `509e387`. Provider Doctor maps only a successful Responses request with a structured final HTTP 2xx status to text reachability; compatibility fallback remains a distinct text-only observation, while image, native-extension, catalog/model, selected-model, provider-group, and every other trusted ledger axis remain unchanged.
- Doctor results and response-only evidence overlays are correlated to the latest request and the exact profile, model-window rows, detail session/revision, and catalog evidence fingerprint. A stale modal result or completed observation cannot cross an edit, catalog refresh, session change, or navigation; Doctor-first and base-ledger-first arrival orders both preserve the same-source text observation.
- Chinese and English copy is visible for new and existing non-aggregate providers. It states that OAuth remains owned by the official client, the provider API key authenticates relay inference only, the Actor marker establishes eligibility only, and upstream, model, account-plan, and runtime gates require independent evidence.

## Verification

- Fresh final scoped review: Spec PASS／Quality PASS; Critical 0／Important 0／Minor 0.
- Frontend suite: 129／129 across 24 suites; focused Doctor evidence／wiring: 13／13; `npm run check` and scoped diff checks pass.
- Backend Provider Doctor compatibility／redaction tests: 2／2; `cargo check` and `cargo fmt --check` pass.
- Tests cover Responses versus Chat protocol discrimination, final 2xx versus initial／failed／3xx status, compatibility fallback, secret-bearing dynamic-field exclusion, text-only ledger merging, exact protocol wiring, new-provider copy visibility, request/source invalidation, completed-observation invalidation, and Doctor／base-ledger response ordering.

## Not verified

- No Tauri bundle, package, install, deployment, release, rendered GUI interaction, real OAuth／upstream probe, image request, or live config／auth mutation was performed.
- No new network probe was added; the integration consumes only the existing Provider Doctor result. The quick-probe mapper remains conservative and read-only but is not a new production network path in this slice.

## Remaining risks

- No known implementation risk remains within OpenSpec 6.7–6.8.
- A successful text probe intentionally proves only text connectivity. Unknown image, native-extension, catalog/model, plan, provider-group, selected-model, and runtime states remain unknown until their own trusted evidence exists; OpenSpec 6.9 and later catalog/runtime policy work remain pending.
