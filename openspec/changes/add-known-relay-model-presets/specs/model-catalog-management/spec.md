## ADDED Requirements

### Requirement: Known relay models are offered as complete cards

The application SHALL ship a reviewed preset table of known relay-served models — slug, display name, description, context window, effective-context percent, supported reasoning levels with descriptions, and default reasoning level — pinned by a contract test so a revision is a deliberate diff. The editor's add-model strip SHALL offer every preset whose slug is not already a row, beside provider-reported candidates, deduplicated by slug; a provider-reported candidate whose slug matches a preset SHALL use the preset card. Adding a preset SHALL create a custom row carrying the full card rather than template defaults, and the row SHALL participate in startup-model selection, removal, and the empty-catalog guard like any other custom row. Preset metadata SHALL only prefill the draft: a saved profile owns its copy, and a later preset revision MUST NOT rewrite an already-saved profile.

#### Scenario: One click mixes a known Claude model into a mixed catalog

- **WHEN** the user adds `claude-fable-5` from the add strip on an `official-plus-custom` profile and saves
- **THEN** the row shows display name "Fable 5" with context window 1000000, and the generated catalog entry carries the card's description, its four reasoning levels with default `medium`, and 95 percent effective context

#### Scenario: A preset the catalog already carries is not offered again

- **WHEN** a custom row with the slug `claude-opus-5` already exists
- **THEN** the add strip does not offer the `claude-opus-5` preset, and a provider report of the same slug does not add a second chip

#### Scenario: A preset revision leaves saved profiles alone

- **WHEN** an application update revises a preset card and a profile saved under the previous card is loaded
- **THEN** the profile's catalog draft and generated catalog are unchanged until the user edits them

### Requirement: A custom row's display name is editable and survives slug edits

A custom row SHALL carry an editable display name. The display name SHALL follow the slug only while it has never been edited independently; once edited, a slug change MUST NOT overwrite it. An empty display name SHALL remain rejected before save.

#### Scenario: Correcting a slug keeps the card's name

- **WHEN** the user corrects the slug of a row whose display name is "Fable 5"
- **THEN** the display name stays "Fable 5" and the catalog entry keeps it

#### Scenario: A hand-typed row still names itself

- **WHEN** the user types a new row's slug without ever touching its display name
- **THEN** the display name follows the slug as it is typed
