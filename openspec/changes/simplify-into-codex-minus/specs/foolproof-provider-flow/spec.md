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

### Requirement: Provider secrets never render in plaintext

The provider bearer key MUST NOT be displayed, exported, or logged in plaintext by any screen, including configuration previews; the masked provider-detail draft IPC remains the only carrier. The raw provider-configuration text editor SHALL NOT be offered.

#### Scenario: The stored contract is previewed

- **WHEN** the user views any preview of the stored or live provider configuration
- **THEN** the bearer value appears masked and no copyable plaintext of it exists in the page
