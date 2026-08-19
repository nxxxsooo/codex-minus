# Tasks: add-no-login-pure-api-path

## 1. Backend — enablement materialization and pure-API coverage

- [x] 1.1 Extend `enable_native_priority_draft` (src-tauri/src/provider_native_capability.rs) to materialize the canonical contract when the parsed config has no `model_provider` selection: insert root `model` (from draft) and `model_provider = "OpenAI"`, create the provider table with base URL, bearer, `wire_api = "responses"`, `requires_openai_auth = true`, and actor header; report missing base URL/model/key as the existing named blockers; preserve unrelated root keys. Unit tests: empty config, non-empty config without provider table, missing-input blockers.
- [x] 1.2 Add a Rust test that first save of a brand-new `PureApi` profile persists the provider plus a valid `CustomOnly` catalog generation from its model list, with no OAuth gate and no action-required state (provider_commit / model_catalog test files).
- [x] 1.3 Add a Rust regression test: with a persisted pure-API profile (`requires_openai_auth = false`, no actor header), a commit focused on a different profile leaves the pure-API profile byte-identical (contract-confinement coverage).
- [x] 1.4 Add a Rust test that Set-as-current for a valid `CustomOnly` pure-API profile succeeds while `auth.json` is absent/unauthenticated and never writes auth.

## 2. Frontend modules — targets, decisions, messages

- [x] 2.1 `src/provider-onboarding.ts`: widen `NewProviderTransientTarget` to `"nativePriority" | "pureApi"`; add the pure-API branch to `materializeNewProviderConfig` (`requires_openai_auth = false`, no `http_headers`, bearer key, `wire_api = "responses"`, provider id `OpenAI`); export a target-selector rule (labels/guidance) for the new-provider page. Tests beside the module.
- [x] 2.2 `src/provider-config-transform-router.ts`: accept the widened transient target in `brandNewProviderConfig` (pass draft target through instead of hardcoding `"nativePriority"`). Tests for both targets materializing on brand-new-empty.
- [x] 2.3 `src/provider-native-capability-view.ts`: add an exit-eligibility predicate (mixed Responses contract, not external, not pure OAuth, not already pure API) for the "切换到纯 API" action; change `providerTransitionDecisionForStructuredPatch` so pure-OAuth → mixed (key entry path) yields the `enableNativePriority` transition contract used by the save gate instead of bare `requiresExplicitUpgrade`. Tests updated.
- [x] 2.4 `src/provider-transition-confirmation.ts`: dedicated message for `exitPureApi` (无需官方登录、失去 OAuth 派生原生能力声明、目录切为 custom-only) and a new enablement confirmation message (混合契约、需要官方登录). No capability/plan-upgrade claims; keep provider-capability-claims tests green.

## 3. App wiring

- [x] 3.1 New-provider page: render the target selector (default `nativePriority`); selecting pure API patches `{ transientTarget, relayMode: "pureApi", officialMixApiKey: false }` through `editDraft`; hide the official-login guide and mixed-mode hint for the pure-API target.
- [x] 3.2 Detail editor: render the pure-API exit action gated by the 2.3 predicate; clicking dispatches `editDraft({ relayMode: "pureApi", officialMixApiKey: false })` and the existing pendingConfirmation flow shows the 2.4 message.
- [x] 3.3 `updateDraft`: stop auto-flipping `officialMixApiKey` on key entry when the profile is pure OAuth; keep current behavior for all other states.
- [x] 3.4 `saveDraft`/Set-as-current: when the draft is pure OAuth with a nonblank structured key, present the enablement confirmation; on confirm dispatch `beginProviderDetailNativePriorityUpgrade` then continue the save; on cancel abort the save, clear the draft key, leave the profile untouched. Remove the dead-end `requiresExplicitUpgrade` toast path for this case.
- [x] 3.5 Keep App.tsx within the app-shell budget: rules land in module files (2.1–2.4); update `app-shell-budget.test.ts` only by lowering/keeping the cap, never raising it.

## 4. Verification and docs

- [x] 4.1 `npm run verify` green (tsc, frontend tests, knip) and `cargo test` green in src-tauri.
- [ ] 4.2 Manual smoke: (a) new pure-API provider with relay Base URL + sk on a machine shape without auth.json → save, set current, confirm generated config.toml has `requires_openai_auth = false`, no actor header, `model_catalog_json` pointing at a materialized custom-only catalog; (b) mixed profile → 切换到纯 API → same outcome; (c) default 默认中转 profile + key → enablement confirmation → mixed contract or clean cancel.
- [x] 4.3 Update AGENTS.md hard-constraint paragraph (one sentence: new-provider pure-API target exists; `requires_openai_auth = false` written only by the explicit pure-API paths) and append the BOARD.md changelog entry.
