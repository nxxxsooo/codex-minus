# Desktop workspace presentation

## Purpose

Present Codex Minus's provider, model, session and diagnostic workflows in the user-selected Light Mode Bauhaus design while preserving existing domain behavior and readable failure/recovery states.

## ADDED Requirements

### Requirement: One coherent desktop workspace

The application SHALL use the selected Stitch SCREEN_8 direction and the B01–B36 suite as visual references: light paper surfaces, strong near-black boundaries, compact geometry and restrained functional accents. Primary navigation SHALL contain providers and sessions. Existing language and dark-theme preferences SHALL remain supported.

#### Scenario: A user navigates between work areas
- **WHEN** the user switches between providers and sessions
- **THEN** shared navigation, typography, action hierarchy and status presentation remain consistent, and route changes do not implicitly apply provider drafts

### Requirement: Preserved provider and catalog workflow

Provider creation, editing, testing, ordering, copying, deletion, routing enablement and activation SHALL retain their existing domain rules. Model edits SHALL retain startup-model validity, ownership boundaries and explicit draft/commit semantics. Generated-design-only controls or invented runtime metadata MUST NOT become product features.

#### Scenario: An inactive provider draft is saved
- **WHEN** a valid inactive provider is saved
- **THEN** the interface adopts the committed baseline and distinguishes saved configuration from the current provider and Codex runtime adoption

#### Scenario: A valid existing dirty draft is activated
- **WHEN** the user selects Set as current on a valid existing draft
- **THEN** the existing atomic save/apply operation is used, without adding an artificial save-first requirement

#### Scenario: A new provider changes access targets before its first save
- **WHEN** a brand-new provider has already filled its Base URL and key, then switches between mixed auth and pure API
- **THEN** it remains a new draft, regenerates only its own provider contract, and preserves the edited fields without treating the draft as an existing persisted profile

#### Scenario: A new provider switches back from custom-only to official-plus-custom
- **WHEN** an official model was copied into a temporary custom-only draft and the user returns to mixed auth before saving
- **THEN** only that temporary model copy becomes field-scoped official edits; untouched reasoning and tool capabilities keep the official baseline, while manually added, edited and unknown-provenance rows remain owned

#### Scenario: Routing is disabled
- **WHEN** the routing switch is disabled
- **THEN** draft saving remains available while activation/live writes are disabled with a visible reason

#### Scenario: A catalog is externally owned or incomplete
- **WHEN** an external pointer is present or complete catalog state is unavailable
- **THEN** the UI reflects the existing ownership/readiness boundary, preserves the draft, and neither fabricates a remote-baseline requirement nor modifies an external file

### Requirement: Evidence-scoped diagnostics and authentication

Provider Doctor SHALL retain its four-step progress and row-scoped results. Authentication surfaces SHALL show sanitized official-client status only. A successful text request or a local configuration marker MUST NOT imply subscription upgrades or unrelated capabilities.

#### Scenario: Text works but model discovery fails
- **WHEN** Doctor receives a successful Responses result while model discovery returns no usable list
- **THEN** both results are shown independently with an actionable recommendation

### Requirement: Preserved session lifecycle and selection

Active/archive lists, paging, row archive/restore/delete, scoped multiselection, archive-policy preview and provider adaptation SHALL preserve existing behavior. Archive SHALL remain distinguishable from deletion and from freeing disk space.

#### Scenario: A batch partially fails
- **WHEN** some selected sessions are deleted and others fail
- **THEN** the result states both counts, failed entries remain inspectable, and successful entries are not automatically retried

#### Scenario: A post-deletion refresh fails
- **WHEN** a permanent cleanup returns a partial result and the following session-list refresh fails
- **THEN** the failed session IDs and reasons remain the final inspectable cleanup notice and the interface asks for a manual refresh before trusting the list

#### Scenario: Active-session adaptation is requested
- **WHEN** the user requests adaptation using a fresh scan
- **THEN** only the existing active-session provider adaptation runs, with its backup/locked-file behavior and archived-history preservation

### Requirement: Accessible, persistent state presentation

Dialogs SHALL have accessible titles, keyboard operation and managed focus. Failures SHALL remain inspectable until dismissed, with sanitized details separate from the primary sentence. Primary actions SHALL remain reachable at 960 × 720, with no unintended horizontal overflow or obscured controls.

#### Scenario: A submit fails
- **WHEN** a provider request returns a conflict or transport failure
- **THEN** the pending state ends, draft data remains available, and the user receives a persistent understandable message and an explicit next action

#### Scenario: The window is resized or reduced motion is requested
- **WHEN** the user uses a small supported window, keyboard navigation or reduced-motion preference
- **THEN** content reflows or scrolls within its work region, focus remains visible, and decorative motion does not block operation
