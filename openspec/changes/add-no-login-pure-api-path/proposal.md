# add-no-login-pure-api-path

## Why

A user without a ChatGPT login cannot use Codex Minus at all today. Field report (2026-08): a tester who cannot sign in hit two dead ends. First, the core default "默认中转" profile (pure OAuth, no key) rejects any API-key entry — typing a key auto-flips `officialMixApiKey`, `providerTransitionDecisionForStructuredPatch` answers `requiresExplicitUpgrade`, and the editor drops the edit with a toast pointing at an upgrade control that does not exist for the `notApplicable`/pure-OAuth state. Second, every brand-new provider materializes only the mixed native-priority contract (`requires_openai_auth = true` plus actor header), which the Codex CLI refuses to run without ChatGPT OAuth. The backend `ExitPureApi` transform (writes `requires_openai_auth = false`, removes the manager actor header, switches the catalog to `custom-only`) is fully implemented and spec-covered ("User selects pure API"), but no UI entry reaches it since the 接入模式 picker was retired. Hand-pasting a config.toml outside the app fails too: the pasted contract still demands OAuth, removing `model_catalog_json` makes relay-only models unreadable, and the app cannot adopt a hand-edited live file as a profile.

## What Changes

- **A — Explicit pure-API exit entry**: the provider detail editor for an existing mixed (native-priority / upgrade-available / degraded) profile offers an explicit "切换到纯 API" action that routes the already-implemented `exitPureApi` backend transform, with the existing capability-loss confirmation, preview, and revisioned draft flow. No new backend transform semantics.
- **B — Pure-API option at provider creation**: the new-provider flow gains an explicit choice between the default mixed native-priority target and a pure-API target. The pure-API target materializes `requires_openai_auth = false`, no actor-authorization header, provider bearer key, `wire_api = "responses"`, and plans a `custom-only` catalog so relay models remain readable without OAuth. The mixed target remains the default; nothing changes for logged-in users who keep the default.
- **C — Default pure-OAuth profile key entry becomes a real transition**: entering a provider key on a pure-OAuth profile no longer dead-ends. Instead of a toast referencing a nonexistent control, the editor routes the change through the explicit native-priority enablement transition (preview + confirmation), or the user can decline and keep the profile untouched.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `provider-native-capability-mode`: three requirement-level changes. (1) The "Explicit authentication-mode exit behavior" requirement gains a reachability guarantee: the pure-API exit is an offered action on eligible existing profiles, not merely an accepted transform. (2) A new-provider draft may select an explicit pure-API target with its own materialized contract and `custom-only` catalog planning, alongside the existing default mixed target ("Built-in Pro model list" and "Derived native-capability-priority state" scenarios extended). (3) Entering a provider key on a pure-OAuth profile is defined as an explicit enablement transition with preview/confirmation instead of a rejected edit.

## Impact

- Frontend: `src/provider-onboarding.ts` (second materialization target), `src/provider-native-capability-view.ts` (structured-patch decision for pure-OAuth + key), `src/App.tsx` wiring (exit action, creation choice, key-entry transition), `src/provider-transition-confirmation.ts` (messages), i18n strings, `src/relay-settings.ts`/`backend-types.ts` if the draft shape grows.
- Backend: `src-tauri/src/provider_native_capability.rs` — reuse `ExitPureApi` and `EnableNativePriority`; possible small addition for a brand-new pure-API materialization path and its catalog-mode default (`custom-only`). `src-tauri/src/model_catalog.rs` default-mode mapping for new pure-API profiles.
- Hard constraints respected: OAuth ownership untouched; `requires_openai_auth = false` written only by the explicit pure-API paths; the contract still changes only through explicit, previewed, revisioned commits focused on the edited profile; no Chat Completions or removed provider shapes reintroduced.
- Docs: `AGENTS.md` hard-constraint paragraph gets one sentence acknowledging the new-provider pure-API target once implemented.
