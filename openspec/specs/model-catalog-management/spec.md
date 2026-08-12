# model-catalog-management Specification

## Purpose

Keep Codex's official model metadata current while allowing each provider to add or override models safely in the native picker without changing inference routing, duplicating ChatGPT OAuth ownership, or exposing credentials.
## Requirements
### Requirement: Trusted official catalog baseline
The system MUST obtain the official catalog through a platform-verified official Codex CLI embedded in the configured target installation and a valid live ChatGPT identity. It MUST NOT give credentials to an untrusted target, treat a relay provider's `/v1/models` ID list or a provider-ambiguous live cache as the official catalog, or modify Codex-owned `models_cache.json` to inject custom entries.

#### Scenario: Official refresh succeeds
- **WHEN** the target-matched Codex CLI returns a valid rich model catalog through an isolated official-provider context
- **THEN** the system records a new official baseline with its fetch time, ETag when available, target client version, model count, and content identity

#### Scenario: Mixed provider is active
- **WHEN** the active relay uses the live ChatGPT identity together with a provider-scoped API key
- **THEN** official refresh leaves the active relay provider, Base URL, API key, live auth, and live configuration unchanged

#### Scenario: Access token is unavailable or rejected
- **WHEN** a usable access token cannot be projected from the live ChatGPT identity or the official request rejects it
- **THEN** the system performs no catalog or auth replacement, retains the last validated baseline, and requires refresh or sign-in through the official client

#### Scenario: Target CLI trust cannot be verified
- **WHEN** the target CLI escapes its application bundle, has an unsupported publisher, fails platform signature validation, or runs on a platform without a supported trust verifier
- **THEN** the system performs no credential-bearing refresh and does not expose an access token to that executable

#### Scenario: Relay model discovery succeeds
- **WHEN** a provider's `/v1/models` endpoint returns model IDs
- **THEN** the system records the IDs only as non-authoritative provider evidence and custom-model candidates and does not replace official metadata

#### Scenario: Official identity or target changes
- **WHEN** the current live account or workspace scope or verified target CLI identity differs from the recorded official baseline
- **THEN** the system marks the baseline scope-stale, retains existing effective catalogs for continuity, and blocks new composition or materialization until refresh succeeds under the current scope

### Requirement: Explicit provider catalog modes

The system SHALL assign every catalog-capable relay profile one catalog mode: `native-official`, `official-plus-custom`, `custom-only`, or `external`. A server-side composite relay SHALL be classified separately from local member aggregation, SHALL present one Responses-compatible upstream to Codex, and SHALL participate in managed catalogs independently of whether its one provider credential is represented by `PureApi` or `Official` plus `officialMixApiKey`. Local member aggregation and Chat Completions proxying SHALL remain unavailable.

#### Scenario: Unmodified official profile

- **WHEN** an official profile has no managed overlays or external catalog pointer
- **THEN** it defaults to `native-official`, writes no managed `model_catalog_json` pointer, and allows Codex to manage its dynamic official catalog

#### Scenario: Official mixed profile

- **WHEN** an official profile mixes a provider-scoped API key and has no external catalog pointer
- **THEN** it defaults to `official-plus-custom`

#### Scenario: Pure API profile

- **WHEN** a direct pure API profile has no external catalog pointer or prior managed mode
- **THEN** it defaults to `custom-only` and can be changed explicitly to `official-plus-custom`

#### Scenario: Server-side composite profile

- **WHEN** a `PureApi` profile or an `Official` profile with `officialMixApiKey = true` is explicitly classified as a server-side composite relay with one Base URL, one provider key, and the Responses protocol
- **THEN** it is applied as one custom-provider upstream without local aggregation, defaults to `official-plus-custom` unless its mode was explicitly chosen, and has managed catalog controls available

#### Scenario: Compatible API or official-mixed profile is reclassified

- **WHEN** the user explicitly reclassifies an existing Responses-compatible pure API or official-mixed profile as server-side composite
- **THEN** the system preserves its relay representation, Base URL, provider key, default model, OAuth ownership, and explicit catalog mode while changing only Manager-owned classification and any implicit catalog-mode default

#### Scenario: External catalog is detected

- **WHEN** a profile points to a catalog path not owned by Codex--
- **THEN** the system marks that profile `external`, does not modify its pointer or file, and requires explicit adoption before managed behavior begins

#### Scenario: Aggregate profile is encountered

- **WHEN** a profile contains local aggregate members or depends on the removed local aggregation proxy
- **THEN** managed catalog controls and live application remain unavailable and the system does not materialize or apply a catalog for that profile

#### Scenario: Chat Completions profile is encountered

- **WHEN** a profile depends on the removed Chat Completions conversion proxy
- **THEN** managed catalog controls and live application remain unavailable even if the upstream provider performs server-side aggregation

#### Scenario: Unsupported topology claims server-side composition

- **WHEN** an Aggregate, Chat Completions, pure-OAuth, missing-credential, or otherwise non-single-upstream profile is marked `server-side-composite`
- **THEN** validation fails before catalog materialization or live mutation and the prior provider/catalog generation remains unchanged

### Requirement: Deterministic official and custom composition
The system SHALL compose each managed catalog deterministically from the latest validated official baseline and that profile's overlay. Official entries SHALL retain their full target-emitted metadata except for explicitly allowed overlay fields, and hidden official entries SHALL remain present in `official-plus-custom` catalogs.

#### Scenario: No overlay exists
- **WHEN** an `official-plus-custom` profile has no custom models or official overrides
- **THEN** its effective catalog contains the complete validated official baseline without reconstructed or dropped metadata

#### Scenario: Provider omits an official model
- **WHEN** provider discovery does not report a slug in the official baseline
- **THEN** `official-plus-custom` retains the official entry and labels its provider availability as unknown or not reported instead of hiding it or claiming it is unsupported

#### Scenario: Provider reports an official model
- **WHEN** provider discovery reports a slug in the official baseline
- **THEN** the system labels that slug as provider-reported evidence without treating the report as a guarantee of account, group, quota, or request availability

#### Scenario: Custom model is added
- **WHEN** a profile adds a slug that is absent from the official baseline
- **THEN** the effective catalog contains one visible custom entry with validated required metadata, the configured context window, and no unrequested official-only capability claims

#### Scenario: Official model is overridden
- **WHEN** an overlay changes an allowed visibility or context-window field for an official slug
- **THEN** the effective catalog preserves all other fields from the current official entry and applies only the allowed overrides

#### Scenario: Custom slug becomes official
- **WHEN** a later official baseline contains a slug previously stored as custom
- **THEN** the official entry becomes the metadata baseline while compatible user visibility, order, and context-window overrides remain applied once

#### Scenario: Duplicate or invalid slug exists
- **WHEN** composition encounters duplicate, empty, or structurally invalid model entries
- **THEN** validation fails before materialization and the previous effective catalog remains unchanged

### Requirement: Live-auth independence and protected secret storage
The system MUST leave live target auth under the official client's ownership and MUST NOT persist or apply `authContents` from any provider profile. Catalog state, generated catalogs, diagnostics, and backups MUST NOT persist ChatGPT OAuth payloads. Secret-bearing settings and auth state MUST use owner-only access or the platform-equivalent ACL.

#### Scenario: Provider profile is saved
- **WHEN** an official, mixed, or pure API provider profile is created, edited, or switched
- **THEN** the profile stores no live-auth payload and provider operations leave live target auth byte-for-byte unchanged

#### Scenario: Provider profile is loaded in the editor
- **WHEN** the frontend requests live or saved auth data for a provider profile
- **THEN** the backend returns sanitized auth status and required actions without returning token-bearing auth JSON

#### Scenario: Manual live-auth save is requested
- **WHEN** a frontend or stale client requests that Codex-- write `auth.json`
- **THEN** the backend rejects the request and directs authentication changes to the official client

#### Scenario: Legacy OAuth profile copies exist
- **WHEN** migration finds ChatGPT OAuth payloads in provider profiles
- **THEN** the system removes the profile copies without creating a credential backup and preserves live auth unchanged

#### Scenario: Live identity is unavailable after migration
- **WHEN** official catalog work requires ChatGPT auth but no valid live identity exists
- **THEN** the system requires official sign-in and never offers a removed or stored profile payload as recovery

#### Scenario: Pure API profile is saved
- **WHEN** a pure API profile owns an API key
- **THEN** the system retains it only in owner-readable provider configuration, materializes it through the provider-config bearer path with no ChatGPT-auth requirement, and does not write it to live auth, catalog state, or diagnostics

#### Scenario: Existing secret permissions are too broad
- **WHEN** the application detects manager state, auth state, or their parent directories with broader-than-owner access
- **THEN** it repairs and verifies owner-only access before credential migration, profile switching, catalog refresh, or other secret-bearing writes can succeed

### Requirement: Safe managed catalog materialization
The system MUST materialize managed catalogs atomically with owner-only access and MUST update live `model_catalog_json` only through a fail-closed, context-protected configuration transaction. It MUST preserve unrelated root settings and the complete `mcp_servers`, `skills`, and `plugins` tables.

#### Scenario: Managed profile is applied
- **WHEN** a validated managed profile becomes active
- **THEN** one transaction atomically writes its manager-owned catalog, applies provider configuration, and sets one root `model_catalog_json` pointer to that file

#### Scenario: Managed profile returns to native official
- **WHEN** a managed profile changes to `native-official`
- **THEN** the system removes only that profile's manager-owned pointer and does not remove or rewrite unrelated configuration

#### Scenario: External profile is reapplied
- **WHEN** a target profile is `external` and has not been adopted
- **THEN** the system preserves that profile's external pointer and file byte-for-byte

#### Scenario: Switching away from an external profile
- **WHEN** the active source profile is `external` and a different target profile becomes active
- **THEN** the system applies the target profile's own pointer state without copying the source pointer and without modifying or deleting the source external file

#### Scenario: Active profile save succeeds
- **WHEN** the user saves provider, auth, or catalog changes for the active profile
- **THEN** one backend transaction commits settings, live configuration, generated catalog, pointer ownership, and restart state as one generation while leaving live auth unchanged

#### Scenario: Context protection cannot be established
- **WHEN** the system cannot snapshot, parse, restore, or verify the protected Context tables
- **THEN** the command fails, restores the prior live generation, and does not report a successful provider or catalog change

#### Scenario: Configuration write fails
- **WHEN** catalog materialization succeeds but the protected live configuration commit fails
- **THEN** the system restores the previous settings, live configuration, generated catalog, pointer, and activation state while live auth remains unchanged

#### Scenario: Process stops during multi-file commit
- **WHEN** the manager exits or crashes after committing only part of a live generation
- **THEN** startup or the next write uses an owner-only transaction journal to restore or finish one complete verified generation before profiles become writable

#### Scenario: Transaction recovery contains provider credentials
- **WHEN** staged or prior settings in an incomplete transaction contain a provider API key
- **THEN** every recovery artifact remains owner-only, is excluded from diagnostics, and is removed after verified recovery while containing no ChatGPT OAuth payload

### Requirement: Transactional and credential-safe refresh
The system MUST isolate official refresh work from live provider configuration, MUST use a non-refreshable access-token projection without an API key or usable refresh-token credential, MUST serialize conflicting live operations, and MUST never expose credentials or catalog instructions in diagnostics.

#### Scenario: Refresh runs while the official client is active
- **WHEN** the target Codex or ChatGPT client is running and the live auth generation remains unchanged
- **THEN** refresh can proceed without killing the client, changing live routing, or writing live auth

#### Scenario: Live auth changes during refresh
- **WHEN** the live auth generation or account identity changes after the isolated snapshot and before catalog commit
- **THEN** the system discards the result, retains the prior baseline, and performs no live-auth write

#### Scenario: Isolated token cannot be refreshed
- **WHEN** the isolated target CLI encounters an expired token or unauthorized response
- **THEN** it cannot rotate credentials, no usable refresh token is present to consume, and the system requires the official client to refresh or re-establish login

#### Scenario: Inherited credential override exists
- **WHEN** the manager process contains API-key, access-token, provider, or auth-endpoint environment overrides
- **THEN** the isolated child starts from an explicit safe environment allowlist and does not inherit those overrides before invoking the target CLI

#### Scenario: Concurrent live operation is requested
- **WHEN** provider switch, apply, clear, active save, adoption, materialization, or another refresh conflicts with an in-progress catalog operation
- **THEN** the system serializes or rejects the conflicting operation so neither transaction observes a partial generation

#### Scenario: Temporary refresh state is cleaned up
- **WHEN** an isolated refresh succeeds, fails, is discarded, or times out
- **THEN** private temporary configuration, projected access-token auth, output, and cache files are removed and their contents are not written to diagnostics

### Requirement: Failure containment and continuity
The system SHALL commit a new official baseline or effective catalog only after complete validation and SHALL retain the last validated version independently for each affected artifact when a later step fails.

#### Scenario: Remote response is malformed
- **WHEN** the target CLI returns empty, malformed, version-incompatible, provider-ambiguous, or non-rich model data
- **THEN** the official baseline and all generated catalogs remain unchanged and the refresh is reported as failed

#### Scenario: One provider cannot be regenerated
- **WHEN** a new official baseline is valid but one inactive provider overlay cannot produce a valid effective catalog
- **THEN** the system retains that provider's previous catalog, marks it action-required, and commits valid catalogs only for unaffected inactive providers

#### Scenario: Active profile regeneration fails
- **WHEN** a new official baseline cannot produce a valid effective catalog for the active profile
- **THEN** the system retains the active profile's prior catalog and pointer generation while recording the newer official baseline separately

#### Scenario: Active default model disappears
- **WHEN** the refreshed official baseline removes an active profile's default model and no custom overlay preserves it
- **THEN** the system does not activate an effective catalog that invalidates the default and reports that a replacement default is required

### Requirement: Compatible migration and adoption
The system SHALL migrate existing relay model rows without copying credentials and SHALL distinguish redundant official rows, official overrides, custom models, per-profile external catalogs, and OAuth copies before changing behavior.

#### Scenario: Existing model matches the official baseline
- **WHEN** a saved `modelList` entry matches an official slug and has no user override
- **THEN** migration treats it as redundant official data rather than a custom duplicate

#### Scenario: Existing context window differs
- **WHEN** `modelWindows` specifies a value for an official slug
- **THEN** migration records an official context-window overlay while preserving the official model's remaining metadata

#### Scenario: Existing model is not official
- **WHEN** a saved model slug is absent from the official baseline
- **THEN** migration records it as a provider-specific custom model in the same relative order

#### Scenario: User adopts an external catalog
- **WHEN** the user explicitly adopts an external catalog after reviewing its source and model diff
- **THEN** the system backs up non-secret configuration, imports compatible differences into managed overlay state, and switches ownership only after validation

### Requirement: Catalog status and update evidence
The system SHALL expose enough status to distinguish official freshness, overlay state, provider-reporting evidence, effective catalog state, pending credential actions, and runtime activation without exposing secret, identity, or prompt content.

#### Scenario: Catalog status is viewed
- **WHEN** the user opens provider management
- **THEN** the system shows catalog mode, source, target client version, last successful refresh, visible and total model counts, provider evidence age, and whether the effective catalog is current

#### Scenario: Official baseline changes
- **WHEN** a validated refresh detects model or metadata changes
- **THEN** the system reports added, updated, removed, and collision counts before or with materialization results

#### Scenario: Generated catalog changes
- **WHEN** the active profile's materialized catalog changes
- **THEN** the system reports that a Codex restart is required because static `model_catalog_json` is loaded at startup and does not restart Codex automatically

#### Scenario: Credential action is required
- **WHEN** live ChatGPT auth is missing, expired, changed during refresh, or cannot be read safely
- **THEN** status requires official-client sign-in or refresh without offering to restore OAuth from a provider profile

#### Scenario: Diagnostics are recorded
- **WHEN** refresh, merge, migration, validation, materialization, or live-write diagnostics are persisted
- **THEN** they contain operation IDs, sources, versions, hashes, counts, duration, status, and redacted errors only

### Requirement: An implicit catalog mode follows the current default

A catalog mode that was derived rather than chosen SHALL be re-derived when state is loaded, the same way an implicit external pointer is already re-derived. A mode the user chose explicitly SHALL never be re-derived.

A stale implicit mode otherwise deadlocks its profile: the provider contract rejects every commit while the mode disagrees, and correcting the mode requires a commit.

#### Scenario: A stale implicit mode is corrected on load

- **WHEN** state carries an implicit catalog mode that disagrees with the mode the current default rule derives for that profile
- **THEN** the loaded mode becomes the current default and the profile can be committed

#### Scenario: An explicit mode is preserved

- **WHEN** state carries a catalog mode the user selected explicitly
- **THEN** the loaded mode is unchanged regardless of what the default rule would derive

### Requirement: A custom-only catalog carries every custom model it declares

A custom-only catalog SHALL contain each model the profile's overlay declares, including a model whose slug the official baseline also carries. Deduplication against the official baseline applies only where an official model of that slug is actually present in the composed output.

A custom-only catalog contains no official models, so dropping such a slug removes the model outright. When that slug is the profile's default model, the model becomes unrepresentable and an active commit fails as catalog-unavailable, with no indication that a declared model was silently discarded.

#### Scenario: A custom-only overlay keeps a slug the official baseline shares

- **WHEN** a custom-only profile declares a custom model whose slug the official baseline also carries, and that slug is the profile's default model
- **THEN** the composed catalog contains exactly that model and planning succeeds

