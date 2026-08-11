## Context

Codex-- currently models the desired routing topology as `relayMode = "official"` plus `officialMixApiKey = true`: the official client retains its ChatGPT sign-in while inference is sent through a custom provider and provider-scoped bearer token. The provider-onboarding branch makes that topology the default for new ordinary providers, and the catalog capability classifies it as `official-plus-custom`.

The generated provider configuration does not yet express the newer Codex actor-authorization contract. It uses a custom provider whose friendly name is `custom`, sets `requires_openai_auth = true`, and has no `x-openai-actor-authorization` header. Current Codex therefore does not treat that custom Responses provider as actor-authorized for OpenAI local extensions. The existing frontend also has multiple line-oriented provider setters; `ensureCodexProviderDefaults` can turn `requires_openai_auth = false` back into `true` during an unrelated Base URL edit.

The backend already provides the hard safety boundaries this change needs:

- provider profiles persist complete raw `configContents`;
- the whole selected `[model_providers.*]` table is provider-owned;
- unrelated live root items are re-grafted from live `config.toml`;
- `mcp_servers`, `skills`, and `plugins` are protected fail-closed;
- active provider and catalog changes use the process-wide coordinator and owner-only transaction journal;
- provider API keys can be projected through `experimental_bearer_token`; and
- live `auth.json` is never an allowed mutation target; its generation is observed so concurrent official-client changes can abort provider commits without restoring older auth.

The implementation must integrate provider-onboarding defaults and provider/live configuration separation that exist on active worktrees. The unified provider-detail Save/Set-as-current boundary exists only as a design document, so this change implements the required request schema and transaction instead of treating it as a landed dependency. The authoritative catalog remains the existing `official-plus-custom` composition from a target-verified official baseline; this change does not wait for or invent a standard-Pro readiness API.

The older provider-onboarding design intentionally generated `requires_openai_auth = true`. Once this change lands, its native-priority new-provider path is superseded by the canonical actor-authorized contract defined here; the compatibility path remains available only as an explicit advanced exit.

The phrase “OAuth plus API key” describes ownership and capability context, not dual credentials on one inference request. ChatGPT OAuth remains live and owned by the official client; the custom provider request uses only its provider-scoped bearer token and static actor marker. Configuration can make Codex eligible to register native extensions, but actual availability can still be denied by the ChatGPT plan, selected model metadata, upstream group policy, image-model allowance, target Codex version, or an old task registry.

## Goals / Non-Goals

**Goals:**

- Make native-capability priority the coherent default contract for newly created ordinary mixed providers after provider-onboarding is integrated.
- Represent the mode through the actual provider TOML and existing relay/catalog fields, with no drift-prone persisted feature boolean.
- Give existing eligible mixed profiles a previewable, explicit upgrade that is draft-only until the normal Save or Set-as-current action.
- Preserve arbitrary provider fields and headers while enforcing the exact fields owned by the selected contract whenever the target mode retains a custom provider table; true pure OAuth is the explicit, previewed destructive exception.
- Keep structured model, Base URL, and key edits from silently breaking an enabled contract; a protocol change away from Responses is an explicit compatibility exit.
- Implement and use the unified provider-detail transaction, Context guard, owner-only storage, rollback journal, and OAuth-generation concurrency check for every persisted or live change.
- Reuse `official-plus-custom` and its validated managed catalog generation for mixed profiles while leaving pure OAuth `native-official`.
- Preserve unadopted external catalog ownership and require the catalog capability's explicit adoption flow before native-priority classification can consume `official-plus-custom`.
- Report configuration readiness separately from account, model, upstream, and runtime evidence.
- Give precise restart and new-task guidance after an active change that affects the tool registry or static catalog.

**Non-Goals:**

- Persisting, refreshing, restoring, or otherwise taking ownership of ChatGPT OAuth.
- Sending ChatGPT OAuth to a relay or combining it with the provider bearer on the inference request.
- Guaranteeing image generation, web search, or every current or future native Codex capability.
- Circumventing Codex's Free-plan or model capability gates.
- Implementing Sub2API server behavior, provider group policy, or `gpt-image-2` allowance.
- Adding a local Chat Completions/aggregate proxy or reviving the removed launcher.
- Defining the standard-Pro catalog contents, signature/update mechanism, or optional Sol 372k overlay.
- Moving global live settings such as `review_model`, reasoning effort, storage, sandbox/network policy, acknowledgements, or `[features]` into provider ownership.
- Automatically restarting Codex or silently migrating existing providers.
- Adding a quota-consuming image-generation smoke test in the first implementation slice.

## Decisions

### 1. Keep the existing mixed relay representation; derive a separate effective state

No new persisted `RelayMode` variant or boolean will be added. The existing topology remains:

- `relayMode = "official"`
- `officialMixApiKey = true`
- `protocol = "responses"`
- catalog mode `official-plus-custom`

A backend-owned evaluator will derive one sanitized effective state from those fields and the parsed selected provider table:

- `nativePriority`: the full managed actor-authorized contract is present;
- `upgradeAvailable`: the profile is an eligible mixed Responses profile but uses legacy-compatible fields;
- `degraded`: native priority is selected or substantially present, but a required provider/catalog input is missing or conflicting;
- `compatibility`: the profile deliberately uses a non-native-priority API contract;
- `notApplicable`: pure OAuth, aggregate, or another topology for which mixed-provider native priority does not apply.

Unadopted external catalog ownership is evaluated before these states. An external mixed profile is not eligible for the ordinary upgrade, and an external pure OAuth profile remains external; adoption or removal must occur through the catalog capability first. A new empty provider uses a frontend-only transient `nativePriority` target, but its derived effective state is `degraded` until Base URL, key, model, and canonical TOML are complete.

The evaluator also returns field-level reasons and an evidence ledger. These values are response-only and are never serialized into settings. A manually authored profile that exactly satisfies the complete contract is therefore recognized without migration metadata; merely having the actor header is insufficient.

This avoids a state where a checkbox says the feature is enabled while the TOML says otherwise. It also avoids changing the pinned core's serialized settings schema solely for a UI concept.

### 2. Make the backend parser and transformer the contract authority

Add a focused backend module at `src-tauri/src/provider_native_capability.rs`, using `toml_edit` to inspect, validate, and minimally transform the selected provider table. It owns these constants and invariants:

- managed header name: `x-openai-actor-authorization`;
- managed header value: `local-image-extension`;
- custom provider friendly name: `OpenAI`;
- wire API: `responses`;
- `requires_openai_auth = false`;
- nonblank `experimental_bearer_token` from the profile's provider-scoped key;
- a non-empty, case-sensitive, non-reserved custom provider identifier selected by root `model_provider`.

The transformer edits the existing selected provider table in place. It preserves the provider identifier, comments where `toml_edit` can preserve them, unrelated keys, unrelated headers, and valid table shape. It accepts both inline and explicit-table header representations, but rejects ambiguous or duplicate structures rather than flattening them. It never rewrites the identifier to lowercase `openai`.

The pinned core treats `CodexPlusPlus` and `CodexPP` as legacy aliases and would later rewrite them. They are therefore the only preservation exception for an otherwise custom identifier: inspection reports an explicit rename requirement, and the upgrade transformer moves the complete provider table and root reference to `custom` only when that target is absent or semantically identical. If `custom` already contains a different table, the draft remains blocked until the user chooses an unused non-reserved identifier; no table is overwritten or merged. That identity change participates in the restart fingerprint.

The persisted credential source of truth is the selected provider's `experimental_bearer_token`; the structured `profile.apiKey` is its editor projection and the seed for a new empty configuration. If both are non-empty and differ, inspection returns a redacted conflict and neither upgrade nor commit chooses a winner. An explicit key edit updates both draft representations together. The same conflict-before-normalization principle applies when structured Base URL or model values disagree with imported raw TOML.

The frontend receives two pure backend operations through the existing Tauri boundary:

1. bulk or per-profile inspection, returning derived state and redacted evidence;
2. draft transformation for explicit enable, conflict-confirmed replacement, or exit.

Neither operation writes settings, live config, catalog state, or auth. The transformed draft is returned to the detail editor. The normal combined commit remains the only persistence boundary.

Backend commit paths run the evaluator again after settings normalization and assert the selected contract again after the pinned core has generated the staged config. This double boundary prevents a stale frontend or upstream defaulting helper from silently reintroducing `requires_openai_auth = true`.

### 3. Centralize frontend provider editing around an explicit transient contract

The provider-onboarding helper remains the owner of new-profile topology defaults. After this change is integrated, its empty mixed draft selects native priority as a transient target. Once Base URL, key, and model are sufficient, the synchronous TypeScript builder may create the canonical TOML because no pre-existing provider table or custom header can be damaged. The backend commit still parses and validates that generated TOML authoritatively.

Frontend draft state may cache the backend inspection result for rendering, but it does not persist that result. `withGeneratedRelayFiles`, `buildRelayConfigToml`, `applyRelayProfilePatchToFiles`, and `ensureCodexProviderDefaults` are refactored so they no longer guess one universal authentication default. They receive the transient target contract derived for the draft:

- pure OAuth: no custom provider block;
- native-priority mixed: the canonical actor-authorized contract;
- pure API: provider bearer with no ChatGPT-auth requirement but no native-priority claim unless explicitly supported by a later design;
- legacy/compatibility: its explicit existing contract.

Line-oriented setters may continue to update simple root or provider string fields for immediate editing, but they must not reconstruct an existing provider table or author authentication/header defaults. Apart from the canonical construction of a brand-new empty profile, any operation that adds, replaces, or removes the actor header goes through the asynchronous backend `toml_edit` transformer. Draft-transform responses carry a request revision and are discarded if a newer edit exists, preventing stale asynchronous results from overwriting current input. Before Save or Set-as-current, the backend remains authoritative.

This design fixes the present Base-URL regression without introducing a second TOML parser into the browser. It also lets the effective-config preview show exactly the transformed draft the user is about to commit.

### 4. Treat upgrade and exit as explicit draft transitions

An eligible legacy mixed profile with no unadopted external pointer shows one action such as “Upgrade to native-capability priority.” Invoking it calls the pure transformer and displays a diff summary:

- `name` becomes `OpenAI` when needed;
- `wire_api` is `responses`;
- `requires_openai_auth` becomes `false`;
- the provider bearer is present;
- the managed actor header is added;
- unrelated provider fields and headers remain.

If the actor header already has a different value, the first action only reports the conflict. Replacement requires a second explicit confirmation because the manager cannot infer ownership. No startup, profile load, diagnosis, or evidence refresh invokes the transformer.

Exiting the mode is also draft-only. The transformer removes the actor header only when it still equals the manager-owned value. A different current value is preserved or requires an explicit conflict decision. Other target-mode fields are changed according to that mode's own contract. Pure API and legacy modes retain their custom provider table and preserve unowned fields. True pure OAuth is deliberately destructive: its preview lists the entire custom provider table and bearer that the pinned core will remove, no dormant copy is kept, and `native-official` is selected only when no external catalog remains. Returning later to mixed routing requires rebuilding or re-entering provider fields.

### 5. Implement one fallible provider-detail commit boundary

Introduce a combined request containing the complete provider draft, an explicit catalog draft, the intended action (`save` or `setCurrent`), the previous active profile ID, Context-cleanup confirmation, and the frontend draft revision. For a new profile, the page constructs an in-memory implicit `official-plus-custom` catalog draft before first Save. `model_catalog_status` cannot create a new profile's missing catalog state as a post-save side effect; existing versioned state migrations remain governed by the catalog capability.

The combined command uses a new fallible provider-detail normalizer; it must not call the current infallible path that logs TOML normalization errors and continues. Within the existing process-wide coordinator it:

1. rejects any new request carrying non-empty `authContents`, whether OAuth or API-key-only, and requires provider keys in the structured provider-key field;
2. permits the existing controlled migration of an already persisted API-key-only legacy auth copy into the provider bearer, then deletes that copy;
3. checks structured-field versus raw-TOML conflicts without exposing their values;
4. evaluates and minimally normalizes the requested provider contract;
5. creates or updates catalog state from the catalog draft supplied in this request, not previously persisted state;
6. for inactive Save, materializes only when current catalog evidence permits and otherwise persists one complete action-required provider/catalog state;
7. for active Save or Set-as-current, requires current official ChatGPT authentication and a catalog generation valid for the current identity/workspace/target scope;
8. generates staged live config through the pinned core and re-parses the staged selected provider to verify native-priority invariants;
9. applies the provider-owned/live-global boundary and Context re-graft;
10. computes the provider runtime fingerprint and complete file mutations;
11. commits settings, catalog state, generated catalog, pointer, live config when active, activation state, and restart state as one journal generation;
12. verifies protected Context tables and the observed auth generation before success.

First Save atomically creates both profile and catalog state; there is no successful half-state in which a profile exists but its catalog state does not. Structural draft errors fail both Save and activation. Environmental catalog unavailability is different: inactive Save may persist action-required state, while activation is blocked.

If official-client auth changes concurrently, verification fails and the journal rolls back only Manager-owned mutations. Because `auth.json` is not a mutation target, the newer official auth bytes remain. With no concurrent auth change, tests assert byte identity before and after. Any failure preserves the user's editor draft for retry.

No new direct `config.toml`, `auth.json`, standalone ordinary catalog-save, or standalone feature-save path is introduced. The existing source-hash-bound external-adoption transaction remains the catalog-owned exception described below.

The generic `save_settings` Tauri command is also hardened at the backend, not merely avoided by the detail page. It compares a provider-owned settings snapshot—including relay profiles, active IDs, common/context relay configuration, provider enablement, aggregate/provider legacy projections, and related relay fields—with the persisted snapshot. A request may save unrelated application settings only when that provider-owned snapshot is unchanged; any provider-bearing difference is rejected and directed to the fallible combined command. This prevents stale or custom frontend callers from bypassing provider TOML validation and atomic catalog/live planning.

### 6. Leave global live configuration outside the profile contract

The provider transformer is allowed to own only:

- root provider selectors and profile-scoped model/catalog/context-limit fields already recognized by the existing provider-owned allowlist; and
- the selected `[model_providers.<id>]` table.

It does not adopt `review_model`, `model_reasoning_effort`, obsolete response-storage fields, network/sandbox settings, Windows acknowledgements, `[features]`, or any other global root/table. The live-config protection pass continues to remove candidate-only global items and re-graft the current live values. The existing provider-detail goals control must be removed from this profile boundary or relocated to a true live/global settings surface; this change must not make it appear profile-scoped.

Owning the two profile-scoped context-limit keys for preservation does not make them valid in every catalog mode. The catalog capability remains authoritative: if `official-plus-custom` treats either key as a managed-catalog conflict, it reports or removes it only through that capability's existing confirmation flow while leaving the actor-authorized provider table intact.

This decision is important for the supplied example: only the model/provider block is part of native-capability priority. The current Codex version rejects several of the example's global keys under strict configuration, and `[features].goals` is valid but unrelated and already enabled by default.

### 7. Consume the managed catalog contract without redefining it

Native-priority mixed profiles use `official-plus-custom`, exactly as required by `model-catalog-management`. Pure OAuth remains `native-official` and writes no manager-owned `model_catalog_json` pointer.

This change consumes the catalog capability's currently authoritative target-verified official baseline and does not carry a competing bundled catalog or decide a signed-update channel. If that baseline is unavailable, scope-stale, invalid, or cannot materialize a profile whose default model is valid, inactive drafts may remain saved and action-required, but a new active generation is blocked. The previous active provider/catalog generation remains live.

Unadopted external pointers have higher precedence than the mixed/native defaults. This change neither changes their files nor converts their ownership. A user must complete the catalog capability's existing source and diff review before the profile can use `official-plus-custom`; a combined future flow may coordinate both confirmations, but the ordinary upgrade action does not.

The existing `adopt_external_model_catalog` preview/commit transaction remains the catalog capability's explicitly specialized exception to ordinary provider-detail Save. Its commit is bound to the reviewed source hash, target version, and version status and retains its rollback rules. The combined provider-detail command refuses an external-to-managed ownership transition, so native upgrade cannot bypass adoption.

### 8. Report a capability evidence ledger, not one success badge

The detail page and Provider Doctor consume a redacted ledger with independent gates:

| Gate | Possible evidence | Meaning |
|---|---|---|
| Provider contract | ready, upgrade available, conflicting, invalid | Whether the saved/draft TOML is actor-authorized and complete |
| OAuth context | signed in, sign-in required, plan Free, plan paid, unknown | Sanitized official-client identity context only; never a token or account identifier |
| Catalog/model | supported, missing metadata, stale, unknown | Whether the effective model metadata advertises the relevant capability |
| Upstream | text reachable, image permission verified, denied, unknown | Provider-observed evidence, scoped to provider/group/model and observation time |
| Runtime | restart required, new task required, adopted, unknown | Whether Codex could have rebuilt the tool registry/static catalog |

The backend emits only evidence it can support. A successful text `/responses` probe sets `text reachable`; it never sets image generation or web search verified. The first implementation reports image permission as unknown unless existing trusted evidence proves denial. A future user-initiated, quota-bearing capability test may set a time-scoped verified result, but that test is outside this slice.

If the existing Provider Doctor reaches text Responses only through its compatibility fallback, the ledger retains the fallback-used evidence and still sets only `text reachable`. It does not satisfy any native-extension, selected-model, provider-group, or catalog gate.

Free-plan reporting is conservative. The backend reports image generation blocked only when a sanitized observed plan is Free and the verified target Codex version is governed by the known Free-plan gate. If either fact is unknown, the gate is unknown. Paid plan status is not presented as proof of availability.

The UI's top-level summary may say “native-capability configuration ready,” “upgrade available,” or “action required”; it must not say “all native capabilities enabled.”

Native-priority activation requires a current sanitized ChatGPT-authenticated state because the product contract is OAuth plus provider routing. Missing or expired OAuth permits inactive Save as action-required but blocks Set-as-current. A Free account is still an authenticated native-priority context; only the target-version-scoped image gate is marked blocked. Users who want key-only text routing choose pure API or compatibility explicitly.

### 9. Persist restart-required state without coupling it to catalog generation

An active commit marks restart required when provider identity, friendly name, protocol, `requires_openai_auth`, actor header, or effective catalog artifact changes. `ProfileCatalogState.generation` remains owned exclusively by catalog materialization. The implementation adds an independent `applied_runtime_fingerprint` and, only if a sequence number is needed, an independent `applied_runtime_generation`. The fingerprint is computed from the provider fields plus a stable catalog runtime identity—managed `generated_hash`, external pointer/source identity, or a native-official sentinel—after catalog planning. It never includes the catalog generation counter that its own update could affect.

When the fingerprint differs from the last applied value, the unified active transaction stores it and sets the existing per-profile `restart_required` signal without incrementing catalog generation. Two identical consecutive active saves are idempotent: the second changes neither fingerprint, runtime generation, catalog generation, nor restart transition. The result explains that Codex/Desktop/IDE hosts must be fully quit and relaunched and that a new task is required to rebuild the local extension registry. Codex-- does not kill or restart those processes.

The restart marker belongs to the committed active generation, not the transient draft. An inactive Save does not set it. This slice has no trustworthy runtime-adoption observer, so it does not automatically clear the marker; runtime adoption remains unknown. A later change may define an explicit acknowledgment or observed-runtime clearing contract.

### 10. Integrate changes in dependency order rather than merging competing App.tsx edits wholesale

The implementation sequence is:

1. land or transplant the raw-JSON/live-config ownership boundaries and current catalog safety work;
2. transplant the provider-onboarding helper and tests, preserving its mixed Responses defaults;
3. implement the combined fallible provider/catalog Save/Set-as-current transaction in this change;
4. add the backend native-capability evaluator/transformer and staged assertions;
5. update provider generation/editing and add the upgrade/status UI;
6. enable the native-priority default only after the existing validated `official-plus-custom` path and all integrated tests pass together.

The active branches overlap in `App.tsx`, translations, styles, catalog UI, backend commands, and `BOARD.md`. Integration therefore uses small semantic transplants and tests, not branch-wide conflict resolution that accepts either side wholesale.

## Risks / Trade-offs

- **OpenAI-specific behavior expands beyond image generation.** `name = "OpenAI"` and actor authorization intentionally opt the custom provider into OpenAI-specific Codex paths, which may include remote compaction or OpenAI-backed web search. This is the point of native-capability priority, but a relay that supports basic `/responses` and not the additional routes may degrade. The evidence ledger and compatibility exit make this visible; the manager cannot promise upstream support.
- **Exact-contract derivation can recognize manually authored configuration as managed.** With no persisted ownership flag, a manually authored profile that exactly matches the contract is classified as native priority. This is acceptable because the classification describes effective behavior. Destructive removal still requires an explicit mode transition and only removes the exact managed header value.
- **The label “official mixed” can imply OAuth is sent upstream.** Product copy must state that OAuth remains an ambient official-client identity/capability context while inference authentication uses the provider key. No OAuth token is merged into relay requests.
- **A pinned-core update could reintroduce defaults after transformation.** Re-validating the fully staged config before commit turns this into a fail-closed error instead of silent drift. Upgrading `codex-plus-core` still requires regression tests for the exact actor-authorized table.
- **Plan and capability gates change across Codex releases.** Hard-coded optimistic claims would age badly. Status is target-version scoped and falls back to unknown when the installed target or policy cannot be verified.
- **Header representation can be structurally complex.** Inline tables, explicit subtables, comments, and custom values make line editing unsafe. The backend `toml_edit` transformer owns header changes and rejects ambiguous forms, accepting a slightly heavier preview round trip in exchange for preservation.
- **Default mixed profiles depend on a valid managed catalog generation.** Pure OAuth remains the route for Codex's dynamic official catalog, while mixed routing needs one deterministic `official-plus-custom` generation from the current target-verified baseline. Temporary refresh unavailability can still permit inactive action-required Save, but not activation.
- **API keys remain in provider configuration.** This follows the existing pure-provider bearer design and avoids live OAuth mutation, but it requires owner-only settings, staging, journal, and backup permissions. Diagnostics and frontend status must stay redacted.
- **The local provider-detail IPC carries the bearer.** The editor must load and change the persisted provider key, so settings/draft payloads inside the local Tauri process boundary can contain it and the UI masks it. Those payloads must never be logged, telemetered, copied into evidence/status/Doctor/catalog responses, or exposed to remote content; eliminating local plaintext IPC would require a separate secret-handle architecture.
- **No real image smoke test means upstream permission remains unknown.** Avoiding surprise quota/cost and generated artifacts is preferable for the first slice. The UI must communicate this limit prominently.

## Migration Plan

1. **Land prerequisites without native-capability migration.** Integrate provider/live ownership separation and provider onboarding. Existing profiles may still undergo independently specified legacy maintenance, but no actor/native-capability field changes automatically.
2. **Implement the combined provider-detail transaction.** Add the request schema, transient first-save catalog draft, draft-aware catalog planner, fallible normalization, inactive action-required persistence, active planning, and complete rollback before changing defaults.
3. **Introduce the backend evaluator in report-only mode.** Add fixtures for canonical, legacy, partial, conflicting-header, external mixed, external pure OAuth, pure OAuth, pure API, aggregate, reserved-ID, key-conflict, and malformed TOML cases. Surface no default behavior until evaluator results agree with staged live output.
4. **Add the pure draft transformer and UI preview.** Existing eligible non-external profiles receive an upgrade action. Cancel/navigation tests verify no provider/native-capability persistence. Conflict replacement and explicit exit require confirmation.
5. **Enforce the contract at the unified commit boundary.** Add staged postconditions, owner-only transaction coverage, full rollback fixtures, Context invariants, auth-generation concurrency tests, and byte-identity tests when auth is not concurrently changed.
6. **Switch new ordinary onboarding to native priority.** Reuse the provider-onboarding helper; empty drafts select the target, complete inputs generate canonical TOML, and first Save creates `official-plus-custom` state atomically. Activation waits only for the existing catalog and OAuth prerequisites.
7. **Add status and restart guidance.** Show independent provider, OAuth, catalog/model, upstream, and runtime gates. Existing active tasks are explicitly called stale until a full host restart and new task.
8. **Do not bulk migrate.** Legacy profiles remain usable in compatibility state indefinitely. Users upgrade one eligible profile at a time through the normal draft and commit flow.

Rollback of the application code does not require rewriting profiles: the canonical actor-authorized TOML remains valid raw Codex configuration. If a user needs behavioral rollback, they explicitly select compatibility, pure API, or pure OAuth and commit that target contract. Transaction recovery continues to restore the prior complete generation if any migration commit is interrupted.

## Deferred Follow-ups

- A quota-bearing image-generation verification action is not part of this change. If later desired, it requires its own proposal covering explicit consent, cost, artifact cleanup, and evidence expiry.
- A future standard-Pro baseline, signed-update channel, or optional Sol 372k policy requires its own catalog delta and is not a prerequisite for this change. Until a minimum target Codex version is verified, plan- and actor-policy evidence remains `unknown` rather than making optimistic claims.
