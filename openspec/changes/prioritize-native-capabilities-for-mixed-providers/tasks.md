## 1. Establish the integration and unified-save base

- [x] 1.1 Create an isolated implementation worktree from the intended integration branch and record the prerequisite commit IDs before changing shared provider or catalog files.
- [x] 1.2 Semantically integrate the raw-JSON model-catalog and provider/live-config ownership work, preserving the provider-owned root allowlist, global live re-graft, Context guard, and OAuth ownership tests.
- [x] 1.3 Transplant the provider-onboarding helper and its tests so a new ordinary provider defaults to `relayMode = "official"`, `officialMixApiKey = true`, and `protocol = "responses"` without merging competing `App.tsx` changes wholesale.
- [x] 1.4 Add failing frontend/backend contract tests for the provider-owned commit envelope: canonical topology draft, catalog drafts, action, focused and previous active IDs, Context confirmation, draft revision, and expected provider fingerprint for new/existing detail saves plus enable, reorder, copy, delete, aggregate-cleanup, and test-model mutations.
- [x] 1.5 Implement thin detail/topology request adapters over one compare-and-swap schema and planner; make ordinary nested catalog controls draft-only, echo revision without treating it as concurrency control, and prevent `model_catalog_status` from creating a new profile's missing catalog state as a post-save side effect while preserving versioned catalog migrations.
- [x] 1.6 Add a draft-aware catalog planner that can create first-save state, persist an inactive action-required state when environmental readiness is unavailable, and plan active provider/catalog output from the request draft rather than previously persisted catalog state.
- [x] 1.7 Verify that the existing target-verified official baseline and `official-plus-custom` composition remain authoritative; add no standard-Pro, signed-update, or 372k readiness dependency in this change.
- [x] 1.8 Run the existing frontend and Rust test suites on the integrated prerequisite baseline and fix or record every pre-existing failure before starting native-capability changes.

## 2. Build the backend contract evaluator test-first

- [x] 2.1 Add failing Rust fixtures for canonical native-priority, eligible legacy mixed, partial, conflicting actor-header, external mixed, external pure OAuth, ordinary pure OAuth, pure API, Chat Completions, aggregate, missing-input, structured-key/bearer conflict, malformed TOML, reserved lowercase `openai`, and legacy alias IDs.
- [x] 2.2 Add failing tests proving classification is derived from relay/catalog fields and parsed TOML, with no serialized native-capability boolean added to settings.
- [x] 2.3 Add failing tests for both inline `http_headers` and explicit header-table forms, including preservation of unrelated headers and rejection of ambiguous or duplicate structures.
- [x] 2.4 Implement `provider_native_capability.rs` with the managed header constants, sanitized state/reason types, selected-provider lookup, eligibility rules, and complete-contract validation.
- [x] 2.5 Add tests that manually authored complete contracts are recognized, a header alone is insufficient, provider IDs remain case-sensitive, reserved built-in `openai` is rejected, and `CodexPlusPlus`/`CodexPP` require an explicit stable-ID rename.
- [x] 2.6 Expose a read-only bulk/per-profile inspection command that returns derived state and field-level reasons without returning provider keys, OAuth tokens, account identifiers, or raw auth JSON.

## 3. Build the pure draft transformer test-first

- [x] 3.1 Add failing tests that enabling native priority preserves a non-legacy provider ID, Base URL, matching bearer, comments where supported, arbitrary provider keys, and unrelated headers while setting only the canonical owned fields.
- [x] 3.2 Add failing tests that enablement produces `name = "OpenAI"`, `wire_api = "responses"`, `requires_openai_auth = false`, a nonblank provider bearer, and `x-openai-actor-authorization = "local-image-extension"`.
- [x] 3.3 Add failing tests that legacy alias IDs move to `custom` only when absent or semantically identical; a different existing `custom` table blocks migration until the user chooses an unused non-reserved ID, with all references/table content preserved.
- [x] 3.4 Add failing conflict tests proving custom actor-header values and structured-key/raw-bearer mismatches remain redacted blockers until an explicit synchronization or replacement decision.
- [x] 3.5 Add failing exit tests proving pure API/legacy preserve unowned custom-provider fields, while true pure OAuth previews and removes the complete custom provider table with no dormant copy; preserve unadopted external catalog ownership in every case.
- [x] 3.6 Implement minimal `toml_edit` transformations for inspect, enable, legacy-ID migration, conflict-confirmed replacement, and exit without touching global live configuration.
- [x] 3.7 Expose the transformer as a revision-echoing pure draft command so a later TypeScript consumer can discard stale responses, and add an audited read-only-boundary regression proving the command path writes no settings, catalog files, live `config.toml`, or `auth.json`.

## 4. Enforce the contract at the unified backend commit boundary

- [x] 4.1 Add failing tests that first and later inactive Save atomically persist one coherent provider/catalog state, including action-required catalog state when appropriate, without changing live configuration, active provider, restart state, or live auth.
- [x] 4.2 Add failing tests that Set-as-current and active Save atomically commit settings, provider config, catalog state, generated catalog, pointer, activation state, and restart state as one generation.
- [x] 4.3 Implement a fallible provider-detail normalizer that rejects missing inputs, structured/raw conflicts, reserved/ambiguous provider selection, invalid TOML, and structural catalog errors before any mutation; prevent provider-detail callers from using the old log-and-continue settings path.
- [x] 4.4 Re-parse and assert the canonical actor-authorized contract after the pinned core generates staged configuration so upstream defaults cannot silently restore `requires_openai_auth = true` or drop headers.
- [x] 4.5 Add transaction-failure injection tests at normalization, catalog materialization, settings persistence, live-config commit, Context verification, and post-commit verification; prove every persisted artifact returns to the prior generation.
- [x] 4.6 Extend success and failure tests to prove protected Context tables remain semantically identical and live `auth.json` remains byte-identical when no official-client update races the transaction.
- [x] 4.7 Add regression tests proving unrelated live roots and tables—including review model, reasoning effort, sandbox/network policy, acknowledgements, and feature flags—are preserved from live state and never introduced from profile content.
- [x] 4.8 Add a concurrent official-auth update test proving the provider transaction aborts and rolls back only Manager-owned mutations while preserving the newer official `auth.json` bytes.
- [x] 4.9 Reject every new Save/Set-as-current request with non-empty `authContents`, whether OAuth or API-key-only; permit only controlled migration of an already persisted API-key-only legacy copy into the provider bearer, then delete that copy.
- [x] 4.10 Block Set-as-current and active Save without current official ChatGPT authentication or with scope-stale catalog identity/target state, while permitting a valid inactive action-required Save.
- [x] 4.11 Verify rollback journals, staging directories, logs, errors, diagnostics, settings, and catalog artifacts contain no ChatGPT OAuth payload, and verify owner-only permissions for every provider-key-bearing path.
- [x] 4.12 Route provider-detail and provider-list callers—including enablement, reorder, copy, delete, aggregate cleanup, and provider test-model changes—through the shared provider-owned transaction engine; then harden backend `save_settings` by comparing the incoming provider-owned settings snapshot with persisted state and allowing unrelated settings changes only when relay profiles, active IDs, relay common/context state, enablement, aggregate/legacy projections, and related provider fields are unchanged.
- [x] 4.13 Add direct-invoke bypass and stale-fingerprint tests proving generic `save_settings` rejects invalid provider TOML, actor/auth changes, non-empty `authContents`, active-ID changes, topology mutations, and other provider-bearing differences with persisted and live bytes unchanged; prove draft revision alone cannot bypass compare-and-swap.

## 5. Make frontend provider generation and editing mode-aware

- [x] 5.1 Add failing TypeScript tests that a new empty provider selects a transient native-priority target but remains incomplete, then emits the exact canonical TOML only after Base URL, key, and model are complete.
- [x] 5.2 Refactor `buildRelayConfigToml`, `withGeneratedRelayFiles`, `applyRelayProfilePatchToFiles`, and provider-default helpers to accept an explicit transient target contract instead of assuming `requires_openai_auth = true`.
- [x] 5.3 Allow the synchronous TypeScript builder to create the canonical header only for a brand-new empty profile; route every add/replace/remove on existing TOML through the revisioned backend transformer.
- [x] 5.4 Add regression tests that model, Base URL, provider key, context-window, and auto-compact edits never clobber the native-priority provider table; keep managed-catalog context-conflict reporting and cleanup under the catalog capability.
- [x] 5.5 Add a specific regression proving a Base-URL-only edit can no longer turn `requires_openai_auth = false` back into `true`.
- [x] 5.6 Remove the provider-scoped goals control or relocate it to its true live/global owner, and add a regression proving provider edits cannot introduce or change `[features].goals`.
- [x] 5.7 Load backend-derived inspection state into the provider-detail draft as response-only UI metadata and prove it is never serialized into `settings.json`.
- [x] 5.8 Treat a protocol change from Responses to Chat Completions as an explicit compatibility exit with a capability-loss preview rather than an ordinary contract-preserving edit.

## 6. Add explicit upgrade, exit, and capability-status UX

- [x] 6.1 Add frontend tests that an eligible non-external mixed profile shows upgrade availability while startup, load, inspection, Doctor, and evidence refresh do not add, remove, or change actor/native-capability fields; allow unrelated independently specified legacy maintenance.
- [x] 6.2 Implement the “upgrade to native-capability priority” draft action with a preview of owned field changes, preserved fields, capability caveats, and the normal unsaved-change state.
- [x] 6.3 Add tests that cancel, navigation, or editor close after preview performs no settings, catalog, live-config, restart-state, or auth write.
- [x] 6.4 Implement explicit confirmation for replacing a custom actor-header value or legacy provider ID and explicit exit choices; pure OAuth must preview destructive custom-provider removal, while pure API/legacy preserve unowned fields.
- [x] 6.5 Add a redacted evidence ledger with independent provider contract, OAuth session, local account plan, catalog/model, upstream, and runtime gates; missing OAuth permits inactive action-required Save but blocks activation and key-only routing is labeled pure API/compatibility.
- [x] 6.6 Add target-path truth-table tests proving a signed-in Free account may activate and is not itself an image block on an actor-marker path without a Free-plan rejection; report a plan-based block only for an exact verified target/path rule, keep unknown behavior unknown, and never treat a paid plan as success.
- [x] 6.7 Update Provider Doctor so a successful text Responses probe reports only text connectivity; preserve `compatibilityFallbackUsed` evidence without treating fallback success as native-extension, selected-model, provider-group, or catalog proof.
- [x] 6.8 Add Chinese and English copy that explains OAuth remains official-client-owned, the provider key authenticates inference, actor authorization only enables eligibility, and upstream/model/account gates still apply.
- [x] 6.9 Keep ordinary pure OAuth visibly `native-official`, preserve external pure OAuth as `external`, exclude unadopted external mixed profiles from one-click upgrade, and keep pure API, Chat Completions, aggregate, and legacy paths advanced and non-default.

## 7. Integrate catalog and restart semantics

- [x] 7.1 Add tests that non-external native-priority mixed profiles request `official-plus-custom`, ordinary pure OAuth requests `native-official`, explicit server-side-composite classification accepts Responses `PureApi` and `Official + officialMixApiKey` single-upstream profiles, external ownership wins before all defaults, and native-priority code defines no competing catalog baseline or update channel.
- [x] 7.2 Block a new active native-priority generation when its managed catalog prerequisite is missing, scope-stale, invalid, or cannot contain the selected default model; preserve the last valid active provider/catalog generation.
- [x] 7.3 Allow an inactive native-priority draft to remain saved as action-required when catalog readiness is unavailable, without changing live configuration or claiming runtime readiness.
- [x] 7.4 Add recovery tests that a later valid official refresh or provider-detail commit materializes an action-required inactive profile, clears only its catalog-readiness action, and permits activation retry under the remaining gates.
- [x] 7.5 Preserve `adopt_external_model_catalog` as the source/version-hash-bound specialized catalog transaction and make the combined provider-detail command reject any unreviewed external-to-managed ownership transition.
- [x] 7.6 Add an independent `appliedRuntimeFingerprint` to `ProfileCatalogState`, using provider ID/name, protocol, auth requirement, actor header, and stable catalog runtime identity (`generated_hash`, external source identity, or native sentinel); add migration/default tests.
- [x] 7.7 Add diff and transaction tests that update the runtime fingerprint and existing `restart_required` signal without incrementing catalog generation, including two identical consecutive active saves that make no second generation or restart transition.
- [x] 7.8 Present complete host quit/relaunch plus new-task guidance without terminating Codex, adding a second restart flag, or auto-clearing the marker without a trustworthy runtime observer.
- [x] 7.9 Prove inactive Save sets no restart marker and an unobserved post-restart runtime remains `unknown` rather than being reported as adopted.

## 8. Run end-to-end preservation and security regressions

- [x] 8.1 Add a golden regression for the supplied custom `OpenAI` provider configuration, excluding invalid/unrelated global keys, and prove normalization plus live staging retain the exact effective actor-authorized contract.
- [x] 8.2 Add golden regressions for an existing legacy mixed profile, a legacy provider-ID alias, and a custom-header profile, proving startup and inspection do not change actor/native-capability fields automatically.
- [x] 8.3 Test the explicit native-priority-to-pure-OAuth transition, proving the preview discloses custom-provider deletion, no dormant copy is retained, the catalog returns to `native-official` only when non-external, and official auth is never written.
- [x] 8.4 Test switching among native priority, pure API, and legacy compatibility, proving target-specific fields change only after commit and unowned provider headers survive every round trip.
- [x] 8.5 Run a pinned-core compatibility test against the fully staged TOML to detect changes in provider-name, actor-header, Responses, bearer, or reserved-provider semantics before future dependency upgrades are accepted.
- [x] 8.6 Audit transaction artifacts, application logs, telemetry, inspection/status/evidence/Doctor payloads, errors, and generated catalogs for provider-key or OAuth leakage using sentinel credentials; allow the local masked provider-detail draft/settings IPC to carry the bearer and prove it is never logged or copied into status surfaces.
- [ ] 8.7 Perform a controlled no-image-cost manual flow without concurrent auth refresh: create a provider, preview/save it inactive, activate it, inspect the live provider/catalog generation and restart guidance, and verify live `auth.json` hash equality.
- [ ] 8.8 Record image-generation permission, `gpt-image-2` allowance, and runtime tool registration as unverified in the manual result unless independently observed; do not convert a text probe into a success claim.
- [x] 8.9 Add the redacted provider-routable capability matrix for text Responses, model discovery, image generation, image editing, remote compaction, and web search; prove success, denial, fallback, and unknown are row-scoped and that no UI copy claims a local subscription upgrade or “all Pro capabilities.”
- [ ] 8.10 With separate explicit approval for quota-bearing operations, run selected image-generation/edit or other costly probes and record target version, provider/profile ID, model, observation time, and redacted result per row; without that approval, leave those rows unknown.

## 9. Validate, document, and hand off

- [x] 9.1 Run focused TypeScript tests for onboarding, provider draft editing, upgrade/exit UX, catalog integration, and restart presentation.
- [x] 9.2 Run focused Rust tests for evaluator/transformer behavior, fallible combined commands, live-state transactions, Context protection, model catalogs, permissions, recovery, OAuth byte equality without a race, and preservation of concurrent official auth updates.
- [x] 9.3 Run `npm run check`, `npm run vite:build`, and `cargo test` from their documented project directories; resolve every change-caused failure.
- [x] 9.4 Run `npm run build` to verify the complete Tauri application bundle when the local signing/build environment permits it, and record any environment-only gap explicitly.
- [x] 9.5 Update user-facing help and architecture documentation with the contract, truthfulness limits, external-catalog precedence, explicit migration/destructive-exit policy, restart/new-task requirement, and ownership distinction between profile and global live configuration; mark the older onboarding `requires_openai_auth = true` native-default text as superseded.
- [ ] 9.6 Append one completed-work entry to `BOARD.md` only after implementation and verification are complete; include the exact tests and the absence of bulk migration or OAuth writes.
- [x] 9.7 Run strict OpenSpec validation, reconcile implementation behavior against every scenario in `provider-native-capability-mode`, and leave no unchecked scenario without an explicit verification record.

## 10. Reconcile shipped behavior and reach the upgrade action

- [x] 10.1 Re-derive an implicit catalog mode on load and never re-derive an explicit one, so a stale derived mode cannot deadlock its profile (`0e21b44`).
- [x] 10.2 Grade the commit gate: refuse only unusable or ambiguous gaps, and accept a draft that is merely un-upgraded so a contract can be completed in steps (`c3948b0`).
- [x] 10.3 Treat the master routing switch as a live-write gate, committing a disabled switch as an inactive draft instead of refusing the commit (`31d5299`).
- [x] 10.4 Surface the typed commit failure code, an actionable hint, and the static rejecting reason, naming the specific contract field for a contract gap (`5e902c5`, `321e99f`, `4972cb4`).
- [x] 10.5 Prefill a new provider with the built-in Pro model list and a default the official bundled catalog carries, leaving only the Base URL and provider key to supply (`d8bd01d`, `070a3f1`).
- [x] 10.6 Restore coverage for `StagingRejected`, whose only exercised trigger was removed by 10.3, by adding a `Staging` checkpoint to the hidden observer — matching how catalog materialization is already injectable — and asserting the typed code, the static reason, and an unchanged prior generation.
- [x] 10.7 Offer the upgrade action whenever every unsatisfied contract field is one the upgrade transform writes, keeping it withheld for a missing model, missing endpoint, unusable identifier, or unparseable structure, and name the input to supply first.
- [x] 10.8 Prove the reachable upgrade path end to end at the real entry point (transform command boundary plus commit transaction; the on-screen flow remains 8.7): a legacy `custom` profile supplies its missing input, saves, upgrades in one explicit revisioned transform, and reaches the canonical contract without any automatic migration of other profiles.
- [x] 10.9 Add a maintenance check that the built-in Pro list contains no slug the official bundled catalog hides, so a retired model is caught instead of shipped.
- [x] 10.10 Confine core storage normalization to the focused profile, so startup, inspection, and a commit focused elsewhere never rewrite a provider contract, and let the startup credential migration relocate a key without deciding the official-auth requirement (`13932f0`).
- [x] 10.13 Keep a custom model whose slug the official baseline also carries when the mode is custom-only, so a profile whose default model is such a slug is representable instead of failing planning as catalog-unavailable (`8.4`).
- [x] 10.12 Delete the provider table the live pointer named when a pure OAuth exit is confirmed, so a provider staged under a non-core identifier cannot leave its bearer in live configuration (`8.3`).
- [x] 10.11 Decide compare-and-swap once against either the normalized baseline or the persisted form the editor was shown, so a profile that is not core-canonical is not permanently stale (`13932f0`).
