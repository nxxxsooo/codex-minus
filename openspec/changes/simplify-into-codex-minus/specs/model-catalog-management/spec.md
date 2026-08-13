## ADDED Requirements

### Requirement: Bundled official catalog baseline

The official model baseline SHALL ship inside the application as a versioned bundled asset — slug, display name, context window, effective-context percent, reasoning levels, and visibility — authored from verified official client output at release time and labeled with its source client version. Composition, readiness classification, activation-scope validation, and commit planning SHALL treat the bundled baseline as always available for the running app version; account-scope staleness SHALL NOT apply to it. The system MUST NOT perform runtime credential-bearing official refreshes and MUST NOT project access tokens into any external process for catalog purposes. Display names shown to the user come from the bundled asset, not from a runtime cache.

#### Scenario: A managed profile composes without any runtime fetch

- **WHEN** an `official-plus-custom` profile is saved or activated with no runtime catalog cache present
- **THEN** composition and commit planning succeed against the bundled baseline and no external process or network request is made for the baseline

#### Scenario: A leftover runtime cache is superseded

- **WHEN** state written by the retired runtime refresh exists on disk
- **THEN** the bundled baseline is used, the leftover state causes no error, and nothing rewrites or deletes it

#### Scenario: Relay model discovery stays evidence-only

- **WHEN** a provider's `/v1/models` endpoint reports models
- **THEN** the system records them only as provider evidence and custom-model candidates and does not replace bundled baseline metadata

#### Scenario: A bundled update retires an active default model

- **WHEN** an application update's bundled baseline no longer carries an active profile's default model and no custom row preserves it
- **THEN** the system keeps the existing effective catalog for continuity and reports that a replacement default is required instead of silently invalidating the profile

### Requirement: A context override implies the catalog it needs

Native official mode generates no managed catalog and points at none, so a per-model context override cannot take effect there. Supplying one SHALL therefore be treated as choosing a managed catalog: the profile's mode becomes `official-plus-custom`, the save generates the catalog, and the active profile's live configuration points at it. The system MUST state, before the save, that a catalog will be generated and that Codex needs a full restart. An overlay that becomes empty again SHALL NOT silently return the profile to native mode. The system MUST NOT persist a per-model override in a mode that would ignore it.

#### Scenario: A larger context window on a native profile takes effect

- **WHEN** the user sets a context window against an official model on a profile in native official mode and saves
- **THEN** the profile becomes managed, a catalog carrying that window is generated, the active profile's live configuration points at it, and the outcome reports that Codex must be restarted

#### Scenario: The consequence is stated before the save

- **WHEN** a context override has been supplied against a profile whose persisted mode is native official
- **THEN** the editor states that saving will generate a catalog for this provider and that Codex must be fully quit and reopened

#### Scenario: Clearing the override keeps catalog ownership

- **WHEN** every override on a promoted profile is cleared again
- **THEN** the profile stays managed and no mode change happens without the user's action

### Requirement: The model list is the list Codex offers

The editor's model list SHALL show exactly the models the profile's catalog makes selectable in Codex: an official model the bundled baseline hides MUST NOT appear as a row, and an override the user supplies decides visibility where it exists. Every listed model SHALL be removable, and every removal SHALL be reversible from the same screen — removing an official model records a hidden override against the baseline entry rather than dropping it, and the model returns to the list of models that can be added. A removal that would leave nothing selectable SHALL be reported in the editor before the save is attempted. Removing the model the profile starts on SHALL clear that selection rather than leave the profile pointing at a model its catalog no longer offers.

#### Scenario: A retired official model is not offered

- **WHEN** the bundled baseline marks an official model hidden and the profile supplies no override for it
- **THEN** the editor does not list that model as a row and offers it only as a model that can be added

#### Scenario: An unwanted official model is removed and restored

- **WHEN** the user removes an official model from the list and then adds it back
- **THEN** the generated catalog hides it while removed, the baseline entry and its fields survive, and adding it back returns the row with its original metadata

#### Scenario: A model already listed is not offered a second time

- **WHEN** the provider reports a model slug the list already carries
- **THEN** the slug is not offered as an addition, so no second row can collide with it in the generated catalog

#### Scenario: An empty list is refused where it was typed

- **WHEN** every model in a managed list has been removed
- **THEN** the editor states that at least one model must remain and the save is not attempted

### Requirement: A mangled model list can be repaired in one action

The editor SHALL offer a single action that restores the shipped list of models a ChatGPT Pro account routes, so a list left wrong by an earlier version or by mistaken removals does not have to be rebuilt from memory. The action SHALL name every model it would remove and require confirmation before removing any. Context windows already supplied SHALL survive the repair, and the startup model SHALL be moved into the restored list if the repair would leave it outside.

#### Scenario: The Pro list is restored without losing a context window

- **WHEN** the user restores the Pro list on a profile carrying a per-model context override for a model the Pro list contains
- **THEN** the list becomes exactly the Pro models and the override is still present

#### Scenario: Removals are named before they happen

- **WHEN** restoring the Pro list would remove models the list currently shows
- **THEN** those models are named and the restore does not proceed without confirmation

### Requirement: Catalog status and provenance

The system SHALL expose enough status to distinguish the bundled baseline's source version, custom-row state, provider-reporting evidence, effective catalog state, and runtime activation without exposing secret, identity, or prompt content. Status SHALL NOT present refresh controls or refresh freshness; the baseline's identity is the running application version plus its recorded source client version.

#### Scenario: Catalog status is viewed

- **WHEN** the user opens provider management
- **THEN** the system shows the model list with bundled display names, the bundled baseline's source version, provider evidence age, and whether the effective catalog is current, with no refresh control

#### Scenario: Generated catalog changes

- **WHEN** a save produces a different effective catalog for the active profile
- **THEN** the system reports the change through the committed generation and restart guidance without exposing secrets

## REMOVED Requirements

### Requirement: Trusted official catalog baseline

**Reason**: The baseline no longer comes from a runtime shell-out to a platform-verified target CLI with projected access tokens; it ships inside the application. Its replacement is "Bundled official catalog baseline", which keeps `/v1/models` evidence-only and keeps live auth untouched by making catalog work credential-free.

### Requirement: Transactional and credential-safe refresh

**Reason**: The runtime official-refresh machinery (isolated CODEX_HOME, access-token projection, publisher-signature verification, refresh command and screen) is removed; the baseline ships with the application, so no credential-bearing refresh exists to constrain. Save/commit transaction safety remains governed by "Safe managed catalog materialization".

### Requirement: Catalog status and update evidence

**Reason**: Status no longer reports refresh freshness, refresh diffs, or pending credential actions because runtime refresh is removed. Its non-refresh obligations continue as "Catalog status and provenance".
