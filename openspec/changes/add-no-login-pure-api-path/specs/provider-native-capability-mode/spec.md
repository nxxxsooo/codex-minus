# provider-native-capability-mode Delta: add-no-login-pure-api-path

## ADDED Requirements

### Requirement: Pure-API exit is a reachable action

The system SHALL offer the explicit pure-API exit as a visible action in the provider detail editor for every eligible existing profile whose selected provider table can adopt the pure-API contract (native-capability priority, upgrade-available, or degraded mixed Responses profiles without an unadopted external catalog). Offering the action SHALL NOT change any persisted or live state; only the existing explicit, previewed, confirmed, revisioned draft transition followed by Save or Set-as-current applies it. The applied exit SHALL write `requires_openai_auth = false`, remove only the manager-owned actor-authorization header, preserve unrelated provider fields, and plan a `custom-only` catalog so the profile's relay models remain representable without official OAuth.

#### Scenario: Eligible mixed profile offers the exit

- **WHEN** the user opens an eligible mixed Responses profile in the provider detail editor
- **THEN** an explicit pure-API exit action is visible, and invoking it presents the capability-loss confirmation before any draft change

#### Scenario: Confirmed exit produces a runnable no-login contract

- **WHEN** the user confirms the pure-API exit and saves or activates the profile
- **THEN** the persisted provider table carries `requires_openai_auth = false`, no manager-owned actor-authorization header, the provider bearer key, and `wire_api = "responses"`, and the profile's catalog state is a valid `custom-only` generation representing its model list

#### Scenario: External profile does not offer the exit

- **WHEN** a profile has an unadopted external catalog pointer
- **THEN** the pure-API exit action is not offered and external ownership rules remain governed by the catalog capability

#### Scenario: Declined confirmation changes nothing

- **WHEN** the user invokes the exit action but cancels the confirmation
- **THEN** the draft, persisted settings, catalog state, and live configuration are unchanged

### Requirement: Explicit pure-API target for a new provider

The new-provider flow SHALL let the user explicitly choose a pure-API target as an alternative to the default mixed native-priority target. The mixed target SHALL remain the default selection. A pure-API draft SHALL materialize a provider contract with `requires_openai_auth = false`, no actor-authorization header, the provider bearer key, `wire_api = "responses"`, a non-reserved provider identifier, and SHALL plan a `custom-only` catalog from the draft's model list so the saved profile is runnable without any ChatGPT login. The pure-API target SHALL NOT claim native-capability priority, actor-marker eligibility, or OAuth-derived capabilities.

#### Scenario: User selects the pure-API target at creation

- **WHEN** the user creates a provider and explicitly selects the pure-API target before first save
- **THEN** the materialized draft contains the pure-API contract, reports only Base URL and provider key as missing inputs, and carries the built-in model list with a default model from it

#### Scenario: First save of a pure-API provider is complete without OAuth

- **WHEN** a valid new pure-API provider is saved while no ChatGPT context is authenticated
- **THEN** one transaction persists the provider and a complete `custom-only` catalog state together, the save is not blocked or marked action-required for missing official OAuth, and no auth write occurs

#### Scenario: Activation of a pure-API provider needs no sign-in

- **WHEN** the user sets a valid pure-API profile as current while no ChatGPT context is authenticated
- **THEN** activation is not blocked by the official sign-in gate, and the committed live configuration lets the target CLI run with the provider bearer alone

#### Scenario: Default target remains mixed

- **WHEN** the user creates a provider without explicitly selecting the pure-API target
- **THEN** the draft keeps the existing default mixed native-priority target and every existing new-provider behavior is unchanged

### Requirement: Pure-OAuth key entry starts an explicit enablement transition

Entering a provider-scoped API key on a pure-OAuth profile SHALL be accepted into the draft instead of being rejected. The system SHALL NOT reject the edit with guidance that references an unavailable control, and SHALL NOT silently convert the profile: before the change persists, the editor SHALL present the explicit native-priority enablement transition (preview plus confirmation), and the transition applies only after the user confirms and then saves or activates. The enablement SHALL succeed for a pure-OAuth profile whose configuration has no provider table yet by materializing the canonical contract from the draft's structured inputs, reporting any missing required input as a named blocker. Cancelling SHALL leave the profile a pure-OAuth profile with no persisted key.

#### Scenario: Key entry is accepted and gated before persisting

- **WHEN** the user enters a provider key on a pure-OAuth profile in the detail editor
- **THEN** the draft accepts the key without an error toast, and the native-priority enablement preview and confirmation are presented before the change can persist

#### Scenario: Confirmed enablement produces the mixed contract

- **WHEN** the user confirms the enablement and saves
- **THEN** the profile adopts the canonical actor-authorized mixed contract through the existing explicit upgrade lifecycle, subject to its existing OAuth and catalog gates

#### Scenario: Cancelled enablement preserves pure OAuth

- **WHEN** the user cancels the enablement confirmation
- **THEN** the profile remains pure OAuth, no key is retained in the draft or persisted state, and no settings, catalog, live-configuration, or auth write occurs
