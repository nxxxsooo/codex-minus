# Design: add-no-login-pure-api-path

## Context

See proposal.md — Why. Current state relevant to the approach:

- Backend `ExitPureApi` (src-tauri/src/provider_native_capability.rs `compatibility_exit_draft`) already writes `requires_openai_auth = false`, removes the manager actor header, and returns catalog mode `CustomOnly`. It is unreachable from the UI.
- `default_mode` (src-tauri/src/model_catalog.rs:1387) already maps `PureApi` → `CustomOnly` (direct topology), and the activation OAuth gate (`managed_provider_catalog_scope_current`, commands.rs:3691) only applies to `OfficialPlusCustom` — a `CustomOnly` profile activates without sign-in. No commit-layer changes are needed for the no-login path.
- The frontend router (src/provider-config-transform-router.ts) has exactly one synchronous materialization path (brand-new-empty → `materializeNewProviderConfig`) and routes `relayMode`/`officialMixApiKey` patches on existing profiles as revisioned backend transitions. `editDraft` (src/App.tsx:2414) already converts `relayMode: "pureApi"` into an `exitPureApi` transition; nothing dispatches it.
- `enable_native_priority_draft` blocks with `MissingProviderSelection` on a profile whose `config_contents` is empty — which is exactly the shape of the core default "默认中转" profile.
- `updateDraft` (src/App.tsx:2727) flips `officialMixApiKey: true` on the first keystroke of a key on a pure-OAuth profile, which `providerTransitionDecisionForStructuredPatch` rejects as `requiresExplicitUpgrade` per keystroke.

## Goals / Non-Goals

Goals
- Make the existing `ExitPureApi` transform reachable from the provider detail editor (A).
- Add a second brand-new materialization target `pureApi` beside `nativePriority` (B).
- Turn pure-OAuth + key entry into the explicit `EnableNativePriority` transition instead of a dead-end toast, including the empty-config default profile (C).

Non-Goals
- No new commit semantics, no changes to the Context 保护罩, OAuth isolation, catalog ownership, or the Responses-only contract.
- No re-introduction of the retired four-target 接入模式 picker as a persistent mode switcher; the creation choice is a one-time target selection, and the exit is a single explicit action.
- No auto-detection of "cannot log in"; the user chooses explicitly.

## Decisions

1. **A: exit action dispatches the existing transition through `editDraft`.**
   The detail editor shows a "切换到纯 API（无需官方登录）" action for eligible profiles (mixed Responses presentation `nativePriority`/`advancedCompatibility` from a mixed contract, not external, not pure OAuth, not already pure API). Clicking calls `editDraft({ relayMode: "pureApi", officialMixApiKey: false })` — the decision function already maps this to `exitPureApi`; the backend returns `ConfirmationRequired`/`CapabilityLossConfirmationRequired`, and the existing `pendingConfirmation` flow plus `providerTransitionConfirmationMessage` presents the capability-loss dialog. Only new code: the button, its eligibility predicate, and a pure-API-specific confirmation message branch (the current generic "切换到兼容模式…" text does not say 无需登录/失去哪些能力).
   *Alternative considered*: a separate command path bypassing the draft router — rejected, it would duplicate the revisioned transform contract.

2. **B: second frontend materialization target, no backend generator.**
   Extend `NewProviderTransientTarget` to `"nativePriority" | "pureApi"` and give `materializeNewProviderConfig` a pure-API branch emitting `requires_openai_auth = false`, no `http_headers`, same provider id `OpenAI`, `wire_api = "responses"`, bearer key. The new-provider draft gains a target selector (default `nativePriority`, radio/segment with one line of guidance: 能登录 ChatGPT 选默认；无法登录选纯 API). Selecting pure API patches `{ transientTarget: "pureApi", relayMode: "pureApi", officialMixApiKey: false }`; on a brand-new empty draft the router already treats this as a synchronous edit (transition decisions only apply to `source: "existing"`), so no router change beyond the widened type. Catalog planning needs no change: `default_mode` derives `CustomOnly` from `relayMode: PureApi`, and the built-in model list rows materialize as custom rows.
   *Alternative considered*: creating mixed then auto-exiting — rejected, two transactions and a misleading intermediate contract.

3. **C: defer the mixed-flip to an explicit gated step; extend `EnableNativePriority` to materialize a missing provider table.**
   - Frontend: `updateDraft` stops auto-flipping `officialMixApiKey` on keystroke for pure-OAuth profiles; the key lands in the draft as an ordinary field edit. `saveDraft` (and Set-as-current) detects "pure OAuth + nonblank structured key" and, before committing, presents the enablement confirmation (new message in provider-transition-confirmation.ts explaining the resulting mixed contract and the login requirement); on confirm it dispatches the `enableNativePriority` transition (same `beginProviderDetailNativePriorityUpgrade` machinery saveDraft already uses for legacy upgrades) and then continues the save; on cancel it aborts the save, clears the draft key, and leaves the profile pure OAuth.
   - Backend: `enable_native_priority_draft` gains a materialization branch: when the parsed document has no `model_provider` selection, build the canonical contract from the structured draft (root `model` + `model_provider = "OpenAI"` + provider table with base URL, bearer, actor header), reusing the same field validation; missing base URL/model/key surface as the existing named blockers (`MissingBaseUrl`, `MissingModel`, `MissingProviderBearer`). Existing root keys in a non-empty config are preserved (toml_edit in-place inserts).
   *Alternative considered*: treating existing-but-empty configs as `brand-new-empty` in the frontend router — rejected, it would bypass the revisioned backend transform that the contract requires for `officialMixApiKey` changes.

4. **Copy discipline.** All new UI copy states facts only: pure API = 不用 ChatGPT 登录、失去 OAuth 派生的原生能力声明; enablement = 混合契约需要官方登录. No capability or plan-upgrade claims (provider-capability-claims.test.ts patterns stay green).

## Risks / Trade-offs

- [Pinned-core normalizer rewrites the pure-API table on unrelated saves] → The contract confinement already in place ("A contract changes only for the profile the user is editing") covers `requires_openai_auth = false`; add a regression test that saving a neighbouring profile leaves a pure-API profile byte-identical.
- [`CustomOnly` materialization gap for a brand-new profile] → First-save transaction already handles catalog creation for new profiles; add a Rust test covering first save of a `PureApi` profile producing a valid `CustomOnly` generation from the built-in model list.
- [Keystroke-deferred flip changes mixed-profile behavior] → The deferral is scoped to profiles whose current state is pure OAuth (`relayMode === "official" && !officialMixApiKey`); existing mixed profiles keep today's behavior (`officialMixApiKey` already true).
- [Exit confirmation reuses a message that undersells consequences] → Add a dedicated message branch keyed on `transition.action === "exitPureApi"`.
- [App.tsx budget test] → New wiring stays in App.tsx but eligibility predicates, target-selector rules, and enablement-gate rules go into the module files beside their tests (`provider-native-capability-view.ts`, `provider-onboarding.ts`), keeping the budget cap satisfiable.

## Migration Plan

Pure additive feature work behind explicit user actions; no data migration. Existing profiles, catalogs, and live files are untouched until a user invokes one of the three new actions. Rollback = revert the commits; persisted pure-API profiles remain valid because the schema (`RelayMode::PureApi`, `CustomOnly`) predates this change.

## Open Questions

None.
