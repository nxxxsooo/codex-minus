## MODIFIED Requirements

### Requirement: Canonical actor-authorized provider contract

The system MUST materialize each native-capability-priority profile as one coherent custom provider contract. The selected provider identifier MUST remain a non-built-in profile-scoped identifier; an explicit upgrade MUST preserve an existing non-reserved, non-legacy identifier unchanged so session references stay valid; a brand-new draft SHALL default its identifier to `OpenAI`. The provider entry MUST use the exact friendly name `name = "OpenAI"`, use `wire_api = "responses"`, project the provider-scoped key through the provider bearer field, and include the manager-owned non-empty `x-openai-actor-authorization = "local-image-extension"` header. A new mixed-auth profile MUST default to `requires_openai_auth = true`. Existing mixed profiles with either `true` or `false` are valid, and ordinary saves MUST preserve the persisted value. The system MUST write `requires_openai_auth = false` only for an explicit exit to pure API. The system MUST NOT route the profile through Codex's reserved built-in `openai` provider identifier, and exactly one generator SHALL own this materialization.

#### Scenario: Native-capability profile is materialized

- **WHEN** a valid native-capability-priority draft is saved
- **THEN** the resulting provider table contains the complete actor-authorized Responses contract and the root `model_provider` selects that profile's existing custom provider identifier

#### Scenario: Existing mixed authentication requirement is preserved

- **WHEN** an existing actor-authorized Responses provider has either `requires_openai_auth = true` or `requires_openai_auth = false` and the user performs an ordinary edit or save
- **THEN** the system preserves that exact persisted value and does not treat either boolean as a compatibility mode

#### Scenario: A new draft defaults its identifier

- **WHEN** the user creates a brand-new provider draft
- **THEN** its provider identifier defaults to `OpenAI`, which round-trips core normalization, storage sanitize, and staging unchanged

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

## ADDED Requirements

### Requirement: Capability copy stays truthful without an evidence surface

The product SHALL NOT display a capability-evidence panel, capability matrix, or capability inspection command. UI copy MUST NOT claim a plan upgrade, a subscription change, or a blanket grant of native or Pro capabilities from configuration alone; copy MAY state only what the contract configures. Failures SHALL name their cause in plain language.

#### Scenario: A canonical save promises nothing it cannot verify

- **WHEN** a profile saves with the complete canonical contract
- **THEN** the interface confirms the configuration without asserting that any specific native capability is thereby unlocked

#### Scenario: No inspection surface exists

- **WHEN** the user explores provider management
- **THEN** no capability-evidence rows, ledger, or evidence-refresh affordances are presented

## REMOVED Requirements

### Requirement: Evidence-based native capability status

**Reason**: The capability-evidence surface and its inspection command are removed. The truthfulness obligation it carried is retained by "Capability copy stays truthful without an evidence surface"; runtime capability observation is no longer a product feature.

### Requirement: Row-scoped provider-routable capability acceptance

**Reason**: The per-capability acceptance matrix is removed with its surface. The prohibition on claiming subscription upgrades or blanket Pro entitlements is retained by "Capability copy stays truthful without an evidence surface".
