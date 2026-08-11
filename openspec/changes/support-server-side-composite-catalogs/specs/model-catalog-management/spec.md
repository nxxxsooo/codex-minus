## MODIFIED Requirements

### Requirement: Explicit provider catalog modes
The system SHALL assign every catalog-capable relay profile one catalog mode: `native-official`, `official-plus-custom`, `custom-only`, or `external`. A server-side composite relay SHALL be classified separately from local member aggregation, SHALL present one Responses-compatible upstream to Codex, and SHALL participate in managed catalogs independently of inference credentials. Local member aggregation and Chat Completions proxying SHALL remain unavailable.

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
- **WHEN** a profile is explicitly classified as a server-side composite relay with one Base URL, one provider key, and the Responses protocol
- **THEN** it is applied as one pure API upstream, defaults to `official-plus-custom` unless its mode was explicitly chosen, and has managed catalog controls available

#### Scenario: Compatible API profile is reclassified
- **WHEN** the user explicitly reclassifies an existing Responses-compatible pure API profile as server-side composite
- **THEN** the system preserves its Base URL, provider key, default model, live-auth independence, and explicit catalog mode while changing only Manager-owned classification and any implicit catalog-mode default

#### Scenario: External catalog is detected
- **WHEN** a profile points to a catalog path not owned by Codex--
- **THEN** the system marks that profile `external`, does not modify its pointer or file, and requires explicit adoption before managed behavior begins

#### Scenario: Aggregate profile is encountered
- **WHEN** a profile contains local aggregate members or depends on the removed local aggregation proxy
- **THEN** managed catalog controls and live application remain unavailable and the system does not materialize or apply a catalog for that profile

#### Scenario: Chat Completions profile is encountered
- **WHEN** a profile depends on the removed Chat Completions conversion proxy
- **THEN** managed catalog controls and live application remain unavailable even if the upstream provider performs server-side aggregation

### Requirement: Deterministic official and custom composition
The system SHALL compose each managed catalog deterministically from the latest validated official baseline and that profile's overlay. Official entries SHALL retain their full target-emitted metadata except for explicitly allowed display-name, visibility, ordering, context, reasoning, and tool-capability overlay fields, and hidden official entries SHALL remain present in `official-plus-custom` catalogs.

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
- **THEN** the effective catalog contains one visible custom entry with validated display name, context window, effective context percentage, reasoning levels and default, and explicitly configured tool capabilities without adding unrequested official-backend-only claims

#### Scenario: Custom reasoning metadata is configured
- **WHEN** a custom model declares supported reasoning levels and a default reasoning level
- **THEN** the system requires unique valid levels, requires the default to be one of those levels, preserves their configured order, and materializes them without deriving them from the model slug

#### Scenario: Custom tool capabilities are configured
- **WHEN** a custom model explicitly declares supported tools or tool capabilities
- **THEN** the system validates and preserves those declarations while continuing to strip service tiers, upgrade prompts, availability notices, and any unrequested official-backend-only fields

#### Scenario: Official model is overridden
- **WHEN** an overlay changes an allowed display-name, visibility, order, context-window, effective-context-percentage, reasoning, or tool-capability field for an official slug
- **THEN** the effective catalog preserves all other fields from the current official entry and applies only the explicit overrides

#### Scenario: Context window changes without a percentage override
- **WHEN** an official overlay changes the context window but does not set an effective context percentage
- **THEN** the system preserves the official entry's existing effective context percentage instead of silently replacing it with `100`

#### Scenario: Custom slug becomes official
- **WHEN** a later official baseline contains a slug previously stored as custom
- **THEN** the official entry becomes the metadata baseline while compatible user display-name, visibility, order, context, reasoning, and tool-capability overrides remain applied once

#### Scenario: Duplicate or invalid slug exists
- **WHEN** composition encounters duplicate, empty, or structurally invalid model entries
- **THEN** validation fails before materialization and the previous effective catalog remains unchanged

## ADDED Requirements

### Requirement: Managed per-model context precedence
The system MUST make per-model catalog metadata authoritative whenever a managed multi-model catalog is active and MUST NOT silently retain profile-wide context settings that clamp or distort models with different context windows.

#### Scenario: Managed profile has no global context values
- **WHEN** a managed profile is saved or applied without `model_context_window` or `model_auto_compact_token_limit`
- **THEN** the system materializes and applies the catalog using each model entry's context metadata without adding either global key

#### Scenario: Existing globals conflict with a managed catalog
- **WHEN** a profile entering or already using managed mode contains `model_context_window` or `model_auto_compact_token_limit`
- **THEN** the system reports an action-required warning that identifies both keys, explains that they override per-model metadata, and requests confirmation before changing live configuration

#### Scenario: User confirms global context cleanup
- **WHEN** the user confirms activation or repair of a managed catalog that conflicts with profile-wide context values
- **THEN** one context-protected transaction removes both keys from the saved profile and live configuration, preserves the prior generation for rollback, and activates the per-model catalog generation

#### Scenario: User declines global context cleanup
- **WHEN** the user declines removal of conflicting profile-wide context values
- **THEN** the system leaves the saved profile, live configuration, catalog pointer, and current generation unchanged and does not report the managed catalog as current

#### Scenario: Native or external catalog uses global context values
- **WHEN** a `native-official` or unadopted `external` profile contains profile-wide context values
- **THEN** the system preserves those values and does not remove them solely because multiple models may be present

### Requirement: Explicit external catalog version compatibility
The system SHALL distinguish authoritative official-refresh version validation from user-owned external-catalog adoption. Official refresh MUST remain exact to the verified target CLI version, while external adoption MUST use structural and target-offline validation and SHALL surface a declared client-version mismatch as evidence rather than an automatic rejection.

#### Scenario: Official refresh version mismatches
- **WHEN** an official refresh result declares a client version whose major, minor, or patch differs from the verified target CLI
- **THEN** the system rejects the refresh result and retains the previous validated official baseline

#### Scenario: External catalog version matches
- **WHEN** an external catalog declares the verified target CLI version and passes structure and target-offline validation
- **THEN** adoption presents no version warning and can proceed after the normal diff review

#### Scenario: External catalog version differs
- **WHEN** an external catalog declares a different client version but passes structure and target-offline validation
- **THEN** adoption shows the declared and target versions, records a compatibility warning, and allows the user to adopt after reviewing the warning and model diff

#### Scenario: External catalog fails structural or target validation
- **WHEN** an external catalog is empty, malformed, duplicate-bearing, has no visible models, or is rejected or semantically changed by the verified target CLI
- **THEN** adoption fails regardless of its declared client version and leaves the external pointer and file unchanged

#### Scenario: External catalog omits a client version
- **WHEN** an external catalog has no declared client version but passes structure and target-offline validation
- **THEN** adoption labels the version as unknown and allows the user to proceed after the normal diff review
