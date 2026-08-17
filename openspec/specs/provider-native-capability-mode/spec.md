# provider-native-capability-mode Specification

## Purpose
Define a safe, explicit mixed-provider mode that keeps ChatGPT OAuth under the official client's ownership, routes inference through a provider-scoped Responses API key, and preserves every native Codex capability that the observable account, model, upstream, and runtime prerequisites actually permit.
## Requirements
### Requirement: Derived native-capability-priority state

The system SHALL represent native-capability priority as a derived provider state rather than a separately persisted feature boolean. The derived state SHALL be based on the profile's relay mode, protocol, catalog ownership and mode, provider selection, provider authentication fields, and managed actor-authorization header. A profile SHALL be reported as native-capability priority only when the complete contract is present.

#### Scenario: New ordinary provider is created

- **WHEN** the user creates an ordinary provider after provider-onboarding defaults are available
- **THEN** its empty draft selects official OAuth plus a provider-scoped API key, the Responses protocol, an implicit `official-plus-custom` catalog draft, and native-capability priority as its transient target without persisting a separate feature flag

#### Scenario: New provider inputs are incomplete

- **WHEN** a new native-capability-priority target is missing its Base URL, provider key, model, or another required input
- **THEN** the system reports the draft as incomplete or degraded and does not claim that the effective native-capability contract is active

#### Scenario: New provider inputs become complete

- **WHEN** a new native-capability-priority target receives every required provider input
- **THEN** the draft materializes the complete canonical provider TOML and can be validated for Save or Set-as-current

#### Scenario: Complete native-capability contract is loaded

- **WHEN** a saved profile contains every field required by the native-capability-priority contract
- **THEN** the system derives the profile as native-capability priority without reading a separate feature flag

#### Scenario: Eligible legacy mixed profile is loaded

- **WHEN** an existing official-OAuth-plus-provider-key Responses profile with no unadopted external catalog lacks one or more native-capability-priority fields
- **THEN** the system leaves the saved and live profile unchanged and reports that an explicit upgrade is available

#### Scenario: External mixed profile is loaded

- **WHEN** an existing mixed profile has an unadopted external catalog pointer
- **THEN** external ownership takes precedence, the profile is not offered the ordinary one-click native-priority upgrade, and any later adoption remains governed by the catalog capability

#### Scenario: Partially matching profile is loaded

- **WHEN** a profile has only some native-capability-priority fields or has conflicting values
- **THEN** the system reports the concrete mismatches and does not claim that native-capability priority is active

#### Scenario: Pure OAuth profile is loaded

- **WHEN** a profile uses only the official client's ChatGPT identity, has no provider-scoped API key, and has no external catalog pointer
- **THEN** it remains true `native-official`, keeps Codex's dynamic official catalog behavior, and is not converted to the mixed-provider contract

#### Scenario: External pure OAuth profile is loaded

- **WHEN** a pure OAuth profile has an unadopted external catalog pointer
- **THEN** it remains `external` until explicit catalog adoption or removal and is not silently reclassified as `native-official`

#### Scenario: Advanced compatibility profile is loaded

- **WHEN** a profile uses pure API, Chat Completions, a client-side aggregate provider, or another incompatible relay mode
- **THEN** the system keeps that mode available as an advanced path and does not classify or silently convert it as native-capability priority

### Requirement: Canonical actor-authorized provider contract

The system MUST materialize each native-capability-priority profile as one coherent custom provider contract. The selected provider identifier MUST remain a non-built-in profile-scoped identifier; its provider entry MUST use the exact friendly name `name = "OpenAI"`, use `wire_api = "responses"`, set `requires_openai_auth = false`, project the provider-scoped key through the provider bearer field, and include the manager-owned non-empty `x-openai-actor-authorization = "local-image-extension"` header. The system MUST NOT route the profile through Codex's reserved built-in `openai` provider identifier.

#### Scenario: Native-capability profile is materialized

- **WHEN** a valid native-capability-priority draft is saved
- **THEN** the resulting provider table contains the complete actor-authorized Responses contract and the root `model_provider` selects that profile's existing custom provider identifier

#### Scenario: Existing provider identifier is upgraded

- **WHEN** an eligible mixed profile with a non-reserved, non-legacy provider identifier is explicitly upgraded
- **THEN** the system preserves its provider identifier and references while changing only the fields required by the selected contract

#### Scenario: Legacy provider identifier is upgraded

- **WHEN** an otherwise eligible profile selects a legacy provider alias that the pinned core would rewrite, including `CodexPlusPlus` or `CodexPP`
- **THEN** the upgrade preview discloses one stable non-reserved identifier rename, preserves the provider table and references under that identifier, and applies the rename only through the explicit Save or Set-as-current action

#### Scenario: Legacy identifier target collides

- **WHEN** the default migration identifier `custom` already names a different provider table
- **THEN** the system blocks automatic migration and requires the user to choose an unused non-reserved identifier rather than overwriting, merging, or discarding either table

#### Scenario: Provider contains unrelated headers

- **WHEN** the provider already has `http_headers` entries other than `x-openai-actor-authorization`
- **THEN** the system preserves their keys and semantic TOML values unchanged and adds or updates only the manager-owned actor-authorization entry

#### Scenario: Actor-authorization header has the managed value

- **WHEN** the provider has a non-empty `x-openai-actor-authorization` value equal to `local-image-extension` and all other contract fields match
- **THEN** the header satisfies the actor-authorization part of native-capability priority

#### Scenario: Actor-authorization header has a custom value

- **WHEN** the provider has a non-empty `x-openai-actor-authorization` value different from the manager-owned value
- **THEN** the system treats the profile as custom or conflicting, preserves that value until an explicit user action, and does not silently claim ownership of it

#### Scenario: Required provider input is missing

- **WHEN** the provider Base URL, provider-scoped API key, selected model, or another required contract input is blank or invalid
- **THEN** the system blocks activation, identifies the missing input, and leaves the previous saved and live generations unchanged

#### Scenario: Structured key and provider bearer conflict

- **WHEN** the structured provider-key draft and a non-empty `experimental_bearer_token` in provider TOML differ
- **THEN** the system reports a redacted conflict and requires an explicit synchronization choice before Save or activation rather than silently preserving or overwriting either value

#### Scenario: Built-in provider identifier would be selected

- **WHEN** materialization would select the reserved lowercase `openai` provider identifier for the custom relay
- **THEN** validation fails before persistence or live application because the custom relay must not impersonate Codex's built-in provider entry

### Requirement: Stable ownership across structured edits

The system MUST preserve the native-capability-priority contract across subsequent edits to model, Base URL, provider key, and other structured provider fields. Provider transformations MUST edit the parsed provider table rather than reconstructing it from a lossy subset. Provider profile operations MUST NOT copy, apply, or overwrite unrelated global live settings.

#### Scenario: Base URL is edited

- **WHEN** the user changes the Base URL of a native-capability-priority draft
- **THEN** the draft retains `wire_api = "responses"`, `requires_openai_auth = false`, the actor-authorization header, the provider identifier, and all unrelated provider fields

#### Scenario: Provider key is edited

- **WHEN** the user changes the provider-scoped API key of a native-capability-priority draft
- **THEN** only the provider bearer projection changes and the rest of the provider contract remains intact

#### Scenario: Default model is edited

- **WHEN** the user changes the default model of a native-capability-priority draft
- **THEN** the provider contract and unrelated root and provider fields remain intact

#### Scenario: Protocol changes to Chat Completions

- **WHEN** the user changes a native-capability-priority draft from Responses to Chat Completions
- **THEN** the system treats the change as an explicit exit to compatibility mode, previews the native-capability loss, and does not preserve a false native-priority classification

#### Scenario: Global live fields differ from profile content

- **WHEN** live `config.toml` contains global settings such as reasoning effort, response-storage policy, sandbox or network policy, acknowledgements, or feature flags
- **THEN** provider save, upgrade, switch, and apply operations preserve those live settings and do not introduce, remove, or replay them from the provider profile

#### Scenario: Protected Context tables exist

- **WHEN** live `config.toml` contains `mcp_servers`, `skills`, or `plugins` tables
- **THEN** every provider write preserves and verifies the complete protected tables through the existing fail-closed Context transaction

#### Scenario: Provider TOML cannot be transformed safely

- **WHEN** the provider table is invalid, ambiguous, or contains a structure the system cannot preserve
- **THEN** the operation fails before any saved or live write and reports the blocking field without normalizing away user content

#### Scenario: Provider topology is changed outside the detail editor

- **WHEN** the user enables or disables provider routing, reorders, copies, or deletes a provider, changes provider test-model state, or performs aggregate-reference cleanup
- **THEN** the operation uses the same provider-owned validation, catalog planning, compare-and-swap, Context protection, OAuth observation, and transaction engine as provider-detail Save rather than generic settings persistence

#### Scenario: Generic settings save carries provider changes

- **WHEN** a stale, custom, or unconverted caller submits any provider-owned difference through generic `save_settings`
- **THEN** the backend rejects the request before writing and directs the caller to the provider-owned commit boundary

### Requirement: Explicit and atomic upgrade lifecycle

The system SHALL make legacy-profile native-capability migration an explicit draft operation. Loading, inspecting, diagnosing, or starting the manager MUST NOT mutate an eligible profile for native-capability migration. Saving or activating an upgrade MUST use the unified provider-detail transaction and MUST preserve one complete prior generation on any failure.

#### Scenario: New profile receives its initial catalog draft

- **WHEN** the new-provider page creates an unsaved native-capability-priority target
- **THEN** it creates an in-memory implicit `official-plus-custom` catalog draft without calling a post-save status operation to create persisted catalog state

#### Scenario: New inactive profile is saved for the first time

- **WHEN** the user saves a valid new native-capability-priority provider without activating it
- **THEN** one transaction creates the provider and its complete catalog state together, materializes an inactive catalog when current catalog evidence permits, and performs no live-provider or auth write

#### Scenario: First save lacks current catalog readiness

- **WHEN** a new inactive provider is structurally valid but the catalog capability cannot currently produce a valid effective generation
- **THEN** the same transaction persists the provider plus a complete action-required catalog state without changing live configuration or claiming readiness

#### Scenario: First save transaction fails

- **WHEN** provider persistence, catalog-state creation, permitted inactive materialization, or transaction verification fails during first Save
- **THEN** neither a partial provider nor a partial catalog state remains persisted

#### Scenario: Provider draft is stale

- **WHEN** the expected provider-state fingerprint no longer matches persisted provider-owned state
- **THEN** the commit fails before mutation and returns a sanitized conflict for reload or merge; the frontend draft revision is used only to correlate responses and never overrides the compare-and-swap failure

#### Scenario: User previews an upgrade

- **WHEN** the user invokes the native-capability upgrade action for an eligible existing profile
- **THEN** the editor updates only its draft, presents the resulting contract and capability caveats, and performs no settings, catalog, live-configuration, or auth write

#### Scenario: User cancels an upgrade

- **WHEN** the user cancels, navigates away, or closes the editor before saving the upgraded draft
- **THEN** the Manager performs no settings, live-configuration, catalog, restart-state, or auth write

#### Scenario: Inactive upgraded profile is saved

- **WHEN** the user saves a valid upgraded draft for an inactive profile
- **THEN** one backend operation persists the complete normalized provider and catalog state without changing the active live provider or live auth

#### Scenario: Upgraded profile is set as current

- **WHEN** the user sets a valid upgraded profile as current, or saves equivalent changes to the active profile
- **THEN** one transaction commits settings, provider configuration, managed catalog and pointer, activation state, and restart state as one verified generation without writing or restoring live auth

#### Scenario: Official OAuth is unavailable during inactive Save

- **WHEN** a native-capability-priority draft is valid but the official client has no current authenticated ChatGPT context
- **THEN** inactive Save persists it as action-required, but the system does not claim that the OAuth-plus-provider path is ready

#### Scenario: Official OAuth is unavailable during activation

- **WHEN** the user attempts to activate a native-capability-priority profile without a current authenticated ChatGPT context
- **THEN** activation is blocked with official-client sign-in guidance while the prior active generation remains unchanged

#### Scenario: Authenticated Free account is used

- **WHEN** a current authenticated ChatGPT context is Free and all provider and catalog activation prerequisites otherwise pass
- **THEN** the profile may activate as the OAuth-plus-provider path and the local plan alone does not mark provider-routed image generation blocked; capability status follows the verified target path and its own model, upstream, and runtime evidence

#### Scenario: Catalog identity or target scope is stale

- **WHEN** the effective catalog is stale for the current ChatGPT identity, workspace, or verified target Codex installation
- **THEN** a new activation is blocked according to the catalog capability even if the provider TOML is canonical

#### Scenario: Upgrade transaction fails

- **WHEN** validation, catalog materialization, Context protection, settings persistence, live configuration, or verification fails during an upgraded active save
- **THEN** the system restores the complete previous settings, catalog, pointer, live configuration, and activation generation and does not report success

#### Scenario: Manager starts with eligible profiles

- **WHEN** startup discovers one or more eligible existing mixed profiles
- **THEN** it may report upgrade availability but does not add, remove, or change native-capability or actor-authorization fields; independent previously specified legacy maintenance remains outside this guarantee

### Requirement: Existing catalog contract remains authoritative

The system SHALL use the existing `model-catalog-management` capability as the sole owner of catalog classification, composition, materialization, and updates. A native-capability-priority mixed profile with no external pointer SHALL use `official-plus-custom`; a pure OAuth profile with no external pointer SHALL remain `native-official`. This capability MUST consume the catalog capability's currently authoritative validated official baseline and MUST NOT define a competing baseline or update channel.

#### Scenario: Mixed profile satisfies catalog prerequisites

- **WHEN** a native-capability-priority profile has a valid current `official-plus-custom` managed catalog generation
- **THEN** activation applies that generation through the catalog capability's existing atomic pointer transaction

#### Scenario: Pure OAuth profile is active

- **WHEN** the active profile is true pure OAuth
- **THEN** no native-capability mixed-provider action adds a managed catalog pointer or replaces Codex's dynamic official catalog

#### Scenario: Managed catalog prerequisite is unavailable

- **WHEN** the validated official baseline or another required `official-plus-custom` catalog artifact is missing, stale for the current identity or target, or invalid
- **THEN** the system preserves the last valid generation, marks the profile action-required, and blocks a new activation that would produce an incomplete mixed-provider generation

#### Scenario: Selected model is absent from the effective generation

- **WHEN** an inactive profile is structurally valid but its selected default model cannot be represented by a valid current `official-plus-custom` generation
- **THEN** inactive Save may retain a complete action-required state, but Set-as-current is blocked and the prior active generation remains unchanged

#### Scenario: Catalog readiness recovers

- **WHEN** catalog refresh or a later valid provider-detail commit can materialize an action-required inactive profile for the current identity and target scope
- **THEN** the catalog capability writes a valid generation, clears the catalog readiness action, and allows activation to be retried subject to the remaining OAuth and provider gates

#### Scenario: Catalog policy changes independently

- **WHEN** the catalog capability later changes its authoritative baseline, composition, update rules, or optional overlays through its own specification
- **THEN** this capability consumes the resulting validated `official-plus-custom` generation without redefining or silently overriding those policies

#### Scenario: External catalog requires adoption

- **WHEN** a mixed or pure OAuth profile has an unadopted external catalog
- **THEN** inspection, exit, Save, and activation preserve the external pointer and file until the catalog capability's explicit source and diff review adopts or removes it

### Requirement: Evidence-based native capability status

The system SHALL distinguish configuration readiness from effective capability availability. It SHALL report official-client session state, local account plan, actor-marker eligibility, model metadata, provider-group permissions, upstream feature support, and runtime registration as independent evidence. It MUST NOT claim that image generation, web search, or all native capabilities are guaranteed merely because the actor-marker provider contract is present.

#### Scenario: Configuration contract is complete

- **WHEN** every native-capability-priority provider field and catalog prerequisite is valid
- **THEN** status reports the configuration as ready while separately reporting known, unknown, satisfied, or blocked runtime and upstream gates

#### Scenario: Free ChatGPT plan is observed

- **WHEN** the official client's sanitized account status identifies the current plan as Free and the verified target's actor-marker path does not enforce a Free-plan rejection for provider-routed image generation
- **THEN** status records the local plan as Free without marking image generation either successful or blocked; the image row remains governed by its model, Sub2API group/upstream, request, and runtime evidence

#### Scenario: Verified target path enforces a plan restriction

- **WHEN** the exact target version and selected capability path are verified to reject the observed local plan
- **THEN** status reports that row blocked with target-version-scoped evidence and does not generalize the restriction to other capability rows

#### Scenario: Account plan is unknown

- **WHEN** the system cannot determine a sanitized account plan without exposing credentials
- **THEN** it reports the plan gate as unknown and does not infer either availability or unavailability

#### Scenario: Selected model lacks required capability metadata

- **WHEN** the effective catalog does not advertise a native capability required by the selected model path
- **THEN** status identifies the model or catalog gate and does not promise that capability

#### Scenario: Upstream image permission is not verified

- **WHEN** the system has not observed a real successful image-generation request for the provider account or group, including access to the required image model
- **THEN** it reports upstream image permission as unverified rather than successful

#### Scenario: Provider connectivity test succeeds

- **WHEN** Provider Doctor receives a successful text Responses result
- **THEN** it reports text connectivity only and does not treat that result as end-to-end verification of image generation or other local extensions

#### Scenario: Provider connectivity uses a compatibility fallback

- **WHEN** Provider Doctor reaches text Responses only after its existing compatibility fallback
- **THEN** it preserves and displays the fallback-used evidence while treating the result only as text connectivity and not as native-extension, model-availability, or catalog-readiness proof

### Requirement: Row-scoped provider-routable capability acceptance

The system MUST track text Responses, model discovery, image generation, image editing, remote compaction, and web search as separate capability rows. Each row MUST identify its configuration prerequisites and MUST require row-specific runtime or upstream evidence before reporting success. The system MUST NOT describe this contract as upgrading the local subscription or enabling all Pro entitlements.

#### Scenario: Text Responses succeeds

- **WHEN** a request succeeds through the selected provider and model over the Responses protocol
- **THEN** only the text Responses row reports success, with compatibility fallback recorded separately when used

#### Scenario: Provider model discovery succeeds

- **WHEN** the provider's model-discovery route reports a model
- **THEN** the model-discovery row records provider-reported evidence without treating it as proof that the selected account, group, quota, or tool request can use the model

#### Scenario: Image generation succeeds

- **WHEN** an explicit image-generation request succeeds for the selected target version, provider, group, and image model
- **THEN** the image-generation row records time-scoped success without upgrading image editing, web search, compaction, or subscription status

#### Scenario: Image editing is not tested

- **WHEN** image generation is verified but no edit request has succeeded
- **THEN** image editing remains unknown rather than inheriting image-generation success

#### Scenario: Remote compaction succeeds

- **WHEN** the target successfully performs remote compaction through the selected provider
- **THEN** only the remote-compaction row reports success; ordinary text reachability alone is insufficient

#### Scenario: Web search succeeds

- **WHEN** an observable web-search result succeeds through the selected provider/model path
- **THEN** only the web-search row reports success and the observation does not imply first-party subscription entitlements or unrelated tools

### Requirement: Restart and new-task activation guidance

The system MUST make runtime activation requirements explicit whenever a saved or active change can affect Codex's client-side tool registry or static model catalog. It MUST distinguish manager configuration success from Codex runtime adoption and MUST NOT terminate or restart Codex automatically.

#### Scenario: Active native-capability contract changes

- **WHEN** an active profile is newly upgraded or its actor-authorization, provider identity, provider friendly name, protocol, auth requirement, or catalog generation changes
- **THEN** the system records restart-required state and instructs the user to completely quit the relevant Codex or ChatGPT host and start a new task after relaunch

#### Scenario: Existing task remains open

- **WHEN** configuration is correct but the user continues an existing Codex task created before the change
- **THEN** status explains that the task can retain its previous tool registry and does not interpret a missing local extension as proof that the saved configuration failed

#### Scenario: Only an inactive draft changes

- **WHEN** the user saves a native-capability change to an inactive profile
- **THEN** no restart is requested until that profile or its materialized catalog becomes active

#### Scenario: Runtime adoption is not observed

- **WHEN** the manager cannot observe a post-restart task using the new configuration
- **THEN** it reports runtime adoption as unverified rather than completed

#### Scenario: Identical active configuration is saved again

- **WHEN** the user saves an active profile whose staged provider runtime fingerprint and effective catalog identity equal the last applied values
- **THEN** the operation does not create a new runtime generation, catalog generation, or additional restart transition

### Requirement: OAuth and provider-secret isolation

The system MUST leave ChatGPT OAuth exclusively under the official client's ownership. Native-capability-priority profile state, transactions, backups, diagnostics, and recovery artifacts MUST NOT persist `authContents`, refresh tokens, or token-bearing live auth. Provider API keys MAY exist only in owner-readable manager/provider configuration and owner-only transaction state and MUST be redacted from frontend status, logs, diagnostics, errors, and catalog artifacts.

#### Scenario: Native-capability profile is saved or activated

- **WHEN** any inactive or active native-capability-priority operation succeeds without a concurrent official-client auth update
- **THEN** the Manager has not targeted live `auth.json`, its bytes remain unchanged, and the saved profile contains no copied ChatGPT OAuth payload

#### Scenario: Native-capability operation fails

- **WHEN** validation, staging, commit, verification, or recovery fails without a concurrent official-client auth update
- **THEN** the Manager does not write or restore live `auth.json`, its bytes remain unchanged, and no OAuth payload is written to rollback or diagnostic artifacts

#### Scenario: Official client updates auth concurrently

- **WHEN** live auth changes after the Manager snapshots its generation and before provider transaction verification completes
- **THEN** the Manager aborts and rolls back only its provider, settings, catalog, pointer, and restart mutations while preserving the official client's newer auth bytes

#### Scenario: Frontend requests capability status

- **WHEN** the editor or Doctor requests auth-related evidence
- **THEN** the backend returns only sanitized plan, sign-in, expiry, or action-required status and never returns raw OAuth or provider-key values

#### Scenario: Provider key is staged or recovered

- **WHEN** an atomic transaction temporarily contains a provider-scoped API key
- **THEN** every containing file and directory has owner-only access, the key is excluded from diagnostics, and transient recovery material is removed after verified completion

#### Scenario: A stale client sends auth contents

- **WHEN** a new Save or Set-as-current request contains any non-empty `authContents`, whether OAuth or API-key-only
- **THEN** the backend rejects the request, directs OAuth changes to the official client, and directs provider keys to the provider-key field

#### Scenario: Legacy provider auth copy exists on disk

- **WHEN** controlled legacy migration finds an `OPENAI_API_KEY` together with OAuth or other legacy fields in a provider-key profile, and every existing provider-key destination agrees
- **THEN** it moves the agreed key into the owner-only provider bearer field, deletes the complete legacy copy, and does not read or write live auth

#### Scenario: Legacy OAuth copy has no provider-key owner

- **WHEN** a pure-OAuth profile contains a legacy `authContents` copy, whether or not that copy also contains `OPENAI_API_KEY`
- **THEN** it deletes the profile copy without creating a provider bearer and leaves live official auth untouched

#### Scenario: Legacy provider auth copy is ambiguous

- **WHEN** provider-key evidence conflicts, the legacy payload is malformed, or a provider-key profile has no usable key source
- **THEN** migration fails without changing persisted settings or exposing credential material

### Requirement: Explicit exit and compatibility behavior

The system SHALL allow the user to leave native-capability priority deliberately. When the target mode retains a custom provider table, it SHALL remove or replace only manager-owned contract fields that the target does not use and preserve unrelated headers and provider fields. True pure OAuth is an explicit destructive exception: it removes the custom provider configuration after preview and stores no dormant provider copy.

#### Scenario: User selects pure API

- **WHEN** the user explicitly changes a native-capability-priority draft to pure API
- **THEN** the draft adopts the pure-API contract, preserves unrelated provider fields, and explains that OAuth-derived native capability behavior is no longer claimed

#### Scenario: User selects pure OAuth

- **WHEN** the user explicitly changes a mixed profile to true pure OAuth
- **THEN** the preview identifies that the custom provider table, bearer, and unrelated custom-provider fields will be removed; after commit the profile stores no dormant custom-provider copy, returns to `native-official` only when no external pointer exists, and leaves official OAuth under the official client's ownership

#### Scenario: User selects legacy compatibility behavior

- **WHEN** the user explicitly chooses a supported compatibility path that requires `requires_openai_auth = true` or lacks actor authorization
- **THEN** the system previews the incompatible fields and capability consequences and changes them only after the normal Save or Set-as-current action

#### Scenario: Manager-owned actor header is removed

- **WHEN** an explicit target mode retains a custom provider table, no longer uses actor authorization, and the header still equals the manager-owned value
- **THEN** the system may remove that one entry while preserving every unrelated header

#### Scenario: Actor header was customized after enablement

- **WHEN** an explicit target mode retains a custom provider table and encounters an actor-authorization value different from the manager-owned value
- **THEN** the system preserves the custom value or requires an explicit conflict decision and never silently deletes it

### Requirement: Provider drafts remain repairable in steps

A provider contract is completed by editing several fields, so the system SHALL refuse a commit only for gaps that make the draft unusable or ambiguous, and SHALL accept a draft that is merely not yet upgraded. Refusing an incomplete-but-interpretable draft would strand the profile, because the fields that complete the contract can only be persisted by a commit.

The system SHALL refuse unparseable provider TOML, a missing or malformed selected provider table, a reserved or legacy provider identifier, a missing endpoint or default model, a malformed contract value, and conflicting or duplicated actor-authorization headers. The system SHALL accept a provider name mismatch, an unsatisfied official-auth requirement, a missing provider bearer, a missing actor-authorization header, a wire-API mismatch, and a catalog-mode mismatch.

#### Scenario: A legacy provider is repaired one field at a time

- **WHEN** a profile still carrying the legacy provider contract is saved with only some of the target fields corrected
- **THEN** the commit succeeds, the corrected fields persist, and the remaining gaps are reported without claiming the native-capability contract is active

#### Scenario: An uninterpretable draft is still refused

- **WHEN** a draft has unparseable TOML, no usable provider table, a reserved or legacy identifier, no endpoint, no default model, a malformed value, or two conflicting actor-authorization headers
- **THEN** the commit is refused, settings and live configuration are unchanged, and the failure names the specific rejecting rule

#### Scenario: A degraded contract never claims native-capability priority

- **WHEN** a saved draft is accepted while its contract remains incomplete
- **THEN** the derived state stays degraded or upgrade-available, the staged live configuration carries only the provider table the user authored, and no native-capability claim is made

### Requirement: Typed commit failures name their rejecting rule

The unified provider commit already returns a typed error code, and the commit failure path writes no log entry, so the system SHALL surface both the typed code and a static rejecting reason to the user. Every reason SHALL be a literal chosen at the failing call site, so dynamic content cannot reach the notice by construction.

#### Scenario: A rejected commit identifies its gate

- **WHEN** a provider commit is refused
- **THEN** the notice carries the typed code, an actionable hint for that code, and the static reason identifying the rejecting rule, including for a code the frontend does not yet map

#### Scenario: A contract gap names its field

- **WHEN** a commit is refused because the native contract is incomplete
- **THEN** the reason names the specific contract field rather than a generic contract message

#### Scenario: Failure payloads stay secret-free

- **WHEN** any typed failure payload is serialized
- **THEN** it contains no provider key, OAuth material, account identity, or endpoint value

### Requirement: Built-in Pro model list for a new provider

A brand-new provider SHALL be prefilled with a built-in list of models a ChatGPT Pro account can route, and with a default model drawn from that list, so that only a Base URL and a provider key remain for the user to supply. The list SHALL ship with the application rather than being discovered, so a new provider is usable before any upstream call.

The default SHALL be a slug the official bundled catalog carries; a default the catalog cannot represent fails the first commit at catalog planning. Slugs SHALL be taken verbatim from the official client rather than guessed, and retired models SHALL be removed once the official bundled catalog hides them.

#### Scenario: A new provider needs only an endpoint and a key

- **WHEN** the user creates a provider
- **THEN** the draft carries the built-in model list and a default model from it, reports only the Base URL and provider key as missing inputs, and keeps the canonical native-capability target it already generated

#### Scenario: The built-in default is representable

- **WHEN** a brand-new provider is committed with its prefilled default model
- **THEN** catalog planning represents that default and the commit is not refused as catalog-unavailable

### Requirement: Provider routing switch governs live writes only

The provider page states that a disabled master switch still saves configuration and simply does not write Codex's live file, so the system SHALL honor that meaning. A disabled switch SHALL commit as an inactive draft rather than refusing the commit.

#### Scenario: A disabled switch saves without touching live configuration

- **WHEN** a provider draft is committed while the master routing switch is disabled
- **THEN** settings and catalog state persist, live configuration is byte-identical, no restart is claimed, and the commit is not refused

### Requirement: A contract changes only for the profile the user is editing

A provider contract SHALL change only through an explicit commit focused on that profile. Startup, inspection, and a commit focused on a different profile SHALL leave every actor and native-capability field exactly as persisted.

This is not automatic by default: the pinned core's storage normalizer rewrites a whole provider table on sight — it renames a legacy provider-ID alias to its own identity, discards the table it replaces along with the actor header that table carried, and restores `requires_openai_auth = true` where the value is absent. The system SHALL therefore confine that normalizer to the profile a commit is focused on, and SHALL NOT run it as part of reading, migrating credentials, or saving a neighbouring profile.

The startup credential migration SHALL remain permitted, for a provider-key profile, to move an agreed provider key out of `authContents` into the owner-only persisted provider bearer and to remove the unauthorized copied OAuth and legacy representation. Those operations repair credential ownership without changing the provider contract; migration SHALL NOT decide the official-auth requirement while doing so.

The only startup exception is a coordinator-protected reset of exact unowned `legacy-model-list` catalog rows for an ordinary mixed Responses profile with implicit `official-plus-custom` state: it may remove only those rows and official overrides exactly equal to their reconstruction from persisted legacy list/window fields, restore bundled official model content, clear consumed legacy fields, and repair a removed legacy default to `gpt-5.6-terra` only when that slug is absent from both the visible official baseline and retained custom rows. Reset bootstrap SHALL classify a missing catalog-state file or profile entry before parsing dormant legacy fields: ineligible profiles receive only preservation-safe state and a one-time marker, while only an eligible implicit `official-plus-custom` profile may enter strict reconstruction. This exception SHALL preserve every other catalog mode, explicit or external state, preset, user-created or unknown-provenance row, and modified or user-added official override; an explicit overlay edit SHALL establish explicit ownership. It SHALL NOT invoke the core normalizer or rewrite any provider contract.

#### Scenario: Startup leaves an unopened contract alone

- **WHEN** the manager starts with a legacy mixed profile, a legacy provider-ID alias, and a custom-header profile persisted, each carrying a legacy API-key auth copy
- **THEN** the credential copies are removed, and every provider identity, provider name, wire API, official-auth requirement, actor header, and unowned provider or header key is byte-identical to what was persisted

#### Scenario: Inspection reports without rewriting

- **WHEN** those profiles are inspected and classified
- **THEN** the reported state reflects each contract as authored and no persisted byte changes

#### Scenario: Saving one profile does not migrate another

- **WHEN** a commit focused on one profile succeeds
- **THEN** every other profile's persisted contract is byte-identical, including a legacy provider-ID alias and its actor header

### Requirement: Compare-and-swap accepts the generation the editor was shown

Compare-and-swap SHALL be decided once, at the commit boundary, against either the normalized baseline the transaction plans on or the persisted form the settings payload actually handed the editor. The two differ whenever a persisted profile is not already in core-canonical form.

Accepting only the normalized form strands such a profile: the expected fingerprint can never be produced, every save reports stale state, and reloading reads the same file and reports it again. Later validators SHALL compare against the one accepted generation rather than re-deciding staleness.

#### Scenario: A non-canonical persisted profile can still be saved

- **WHEN** a provider draft is committed while another persisted profile is not in core-canonical form
- **THEN** the commit is not refused as stale, and the focused profile persists

### Requirement: A confirmed pure OAuth exit leaves no dormant provider copy

When the user confirms the destructive exit to pure OAuth, the provider that was deleted SHALL NOT survive anywhere the system controls: not in the profile contract, not in any other persisted provider field, and not in Codex's live configuration.

Live cleanup SHALL cover the provider the live configuration was actually pointed at, not only the identifiers the pinned core recognizes as its own. A provider staged under any other identifier would otherwise keep its complete table, including `experimental_bearer_token`, in the file the user was told it had been removed from.

Official authentication SHALL NOT be written by this exit, and a non-external profile SHALL return to the native official catalog.

#### Scenario: The exit removes the staged provider from live configuration

- **WHEN** an active native-priority provider is staged into live configuration and the user then confirms the pure OAuth exit
- **THEN** live configuration no longer contains that provider's table, identifier, or bearer, persisted settings retain no copy of them, the profile's catalog mode returns to `native-official`, and `auth.json` is byte-identical
