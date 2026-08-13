## ADDED Requirements

### Requirement: Seven-step switchless provider journey

The system SHALL let a user complete the whole provider journey — official OAuth sign-in in the official client, open the app, open the profile, keep the prefilled default model, fill Base URL and API key, save, take effect — while the editor shows only 名称, the model list, Base URL, API key, and Provider Doctor. The editor MUST NOT show access-mode, protocol, auth-mixing, catalog-mode, User-Agent, or context-size controls, and every save SHALL materialize the one canonical actor-authorized contract. Opening a legacy profile and saving SHALL upgrade that profile in place; no other profile changes.

#### Scenario: A new user finishes with only two inputs

- **WHEN** a signed-in user creates a profile and supplies only a Base URL and an API key
- **THEN** the save succeeds with the prefilled default model and the resulting provider table carries the complete canonical contract

#### Scenario: No switch can produce a different contract

- **WHEN** the user explores every visible editor control
- **THEN** no combination changes the saved contract's provider name, wire API, auth requirement, bearer projection, or actor header

#### Scenario: A legacy profile upgrades on its own save

- **WHEN** the user opens a profile stored with a legacy shape and presses save with valid inputs
- **THEN** that profile reaches the canonical contract in one revisioned commit while every other profile stays byte-identical

### Requirement: A save always settles with a human-readable outcome

Every save SHALL end, in bounded time, in exactly one of: a success notice, or a failure stated as one plain-language sentence naming what happened and the next action, with codes and raw detail behind a details affordance. The pending 保存中 state MUST clear on every path: subprocess waits in the save and status paths are bounded and fail typed on expiry; a panicked backend command returns a typed failure instead of dropping the reply; post-commit refreshes run after the pending state clears and cannot hold it.

#### Scenario: A blocked subprocess cannot hang the button

- **WHEN** a permission or verification subprocess in the save path never returns
- **THEN** the bounded wait expires, the transaction rolls back, and the user sees a plain-language failure instead of a perpetual 保存中

#### Scenario: A backend panic still answers the editor

- **WHEN** the blocking save closure panics
- **THEN** the frontend promise settles with a typed failure, the failure renders in plain language, and the next save attempt is not silently rejected

#### Scenario: Success is not delayed by refreshes

- **WHEN** the commit transaction has committed and the post-commit status refresh is slow
- **THEN** the success notice has already been shown and the button is already re-enabled

### Requirement: One table answers every model question

The provider editor SHALL present exactly one model list per profile, covering both official baseline models and the profile's own custom models, and the model Codex starts on SHALL be a selection on a row of that list rather than a name typed elsewhere. The editor MUST NOT offer a second surface that names a model — no free-text startup model, no per-profile or global test model — and a provider test SHALL use the model the profile starts on. Renaming or removing the selected row SHALL carry the selection with it, and a row with no identifier yet SHALL NOT be selectable. Diagnostic output that reports the models a provider offers SHALL be named so it cannot be read as the editable list.

#### Scenario: The startup model cannot name a model the catalog lacks

- **WHEN** the user chooses which model Codex starts on
- **THEN** the choice is a row of the profile's own model list, and no control accepts a model identifier that the list does not contain

#### Scenario: The selection survives an edit to its own row

- **WHEN** the user renames or deletes the row that is currently selected as the startup model
- **THEN** the selection follows the rename, or is cleared by the deletion, and never keeps pointing at a model the catalog no longer contains

#### Scenario: A retired test model stops outranking the startup model

- **WHEN** a profile or the settings file carries a test model written by an earlier version whose control no longer exists
- **THEN** the next save clears it, and the provider is tested with the model the profile starts on

### Requirement: A save states which generation it could not read

A save SHALL NOT proceed against an absent settings baseline, and SHALL NOT report an internal condition as the reason. A settings read that failed MUST NOT become the compare-and-swap baseline nor replace the profiles on screen; its failure is reported when it happens. When a save finds no baseline it SHALL read the persisted generation once more and then state, in plain language, whether the generation has just been read and the edit should be repeated, or whether it still cannot be read.

#### Scenario: A failed read does not become the baseline

- **WHEN** the settings read answers with a failure and placeholder settings
- **THEN** the previous baseline and the profiles on screen are kept and the failure's own reason is shown

#### Scenario: A save with no baseline does not publish placeholder settings

- **WHEN** the user saves before any settings generation has been read
- **THEN** the save stops without committing, the persisted generation is read and shown, and the user is asked to confirm and save again

### Requirement: Provider secrets never render in plaintext

The provider bearer key MUST NOT be displayed, exported, or logged in plaintext by any screen, including configuration previews; the masked provider-detail draft IPC remains the only carrier. The raw provider-configuration text editor SHALL NOT be offered.

#### Scenario: The stored contract is previewed

- **WHEN** the user views any preview of the stored or live provider configuration
- **THEN** the bearer value appears masked and no copyable plaintext of it exists in the page
