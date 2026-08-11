## MODIFIED Requirements

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
