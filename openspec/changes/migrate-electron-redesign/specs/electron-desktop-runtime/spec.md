# Electron desktop runtime

## Purpose

Provide a consistent Chromium desktop runtime while preserving Codex Minus's existing local-data ownership, configuration transactions, native lifecycle and authenticated update guarantees.

## ADDED Requirements

### Requirement: Restricted desktop command bridge

The application SHALL expose only its registered domain commands and narrowly scoped desktop lifecycle APIs to its own main application frame. Untrusted frames, arbitrary command names, filesystem paths for executable selection and arbitrary process execution MUST NOT acquire privileged access through the renderer bridge.

#### Scenario: An existing provider command crosses the new bridge
- **WHEN** the application invokes provider commit with an expected fingerprint and draft revision
- **THEN** the original domain validation, transaction and response semantics apply, including typed failures and correlation with the current draft

#### Scenario: An untrusted caller sends a command
- **WHEN** the command originates outside the trusted main frame, is not registered, or has an invalid transport envelope
- **THEN** it is rejected before domain mutation, without echoing credential-bearing arguments in an error

### Requirement: One owned backend and truthful transport failures

The application SHALL maintain one managed backend process for its command session. Connection loss, startup failure, malformed responses and request timeouts SHALL settle pending calls with an actionable transport error. Mutating requests MUST NOT be automatically replayed after an uncertain outcome.

#### Scenario: A backend exits during a provider save
- **WHEN** the renderer has submitted a provider commit and the backend exits before its response is received
- **THEN** the interface retains the unsaved draft, reports an unconfirmed outcome, and requires recovery and a fresh authoritative baseline before a subsequent write

#### Scenario: The user reconnects
- **WHEN** a new backend is started after a failed session
- **THEN** it establishes single-writer ownership and performs existing transaction recovery before settings become writable, without replaying the prior request

### Requirement: Existing state and exclusive writer ownership

The application MUST retain existing settings, catalog and Codex-home locations and the existing configuration/auth ownership rules. A second current or legacy manager MUST NOT create a concurrent live writer. Failure to acquire ownership SHALL fail closed.

#### Scenario: A legacy manager is running
- **WHEN** the new manager starts while the legacy manager owns the same data generation
- **THEN** the new process does not perform credential migration, configuration writes or journal recovery as a competing writer

#### Scenario: Existing data is opened
- **WHEN** the new manager loads saved profiles and catalogs
- **THEN** it consumes the existing domain loader and narrow migrations, preserving protected Context tables, external catalog files, unrelated profiles and official-client-owned auth

### Requirement: Native desktop lifecycle

The application SHALL retain a single main window, tray/menu access, close-to-hide behavior, explicit quit, second-launch activation, macOS reopen activation, version display and bilingual native labels. The window SHALL support the existing 960 × 720 minimum content size.

#### Scenario: The window is closed and reopened
- **WHEN** the user closes the window and subsequently selects Show from the tray or reopens the app
- **THEN** the existing window and its editing session are restored without restarting the backend or applying a draft

#### Scenario: The manager exits normally
- **WHEN** the user explicitly quits the manager
- **THEN** its owned backend is given a bounded opportunity to finish accepted work and exit, and any interrupted multi-file transaction remains recoverable through the existing journal

### Requirement: Signed updates preserve the trust boundary

Update checks SHALL retain silent startup failure and explicit manual-check feedback. Installation SHALL require an explicit update action, a newer compatible Electron artifact and verification of both manifest and artifact against the existing pinned minisign trust key before extraction or execution. Manager updates MUST NOT restart the official Codex host. The v0.5.0 release SHALL provide the legacy feed bridge so existing Tauri installations can upgrade automatically to Electron using the same signed artifacts.

#### Scenario: An update signature or runtime identity is invalid
- **WHEN** an artifact has an invalid signature, the wrong platform/architecture/runtime, or an older version
- **THEN** installation is rejected and the existing application remains usable

#### Scenario: Installation cannot finish
- **WHEN** the installation location is readonly or replacement fails
- **THEN** the update reports the actual failure and any uncertain state honestly, retaining a recoverable prior application rather than asserting unverified success

#### Scenario: An existing Tauri installation upgrades
- **WHEN** v0.4.18 requests its existing update feed and the user accepts v0.5.0
- **THEN** its updater verifies and installs the Electron artifact in the existing application location, preserving settings, sessions, Context and official authentication; the Windows handoff retires the old application registration only after a successful replacement

#### Scenario: A helper fails before becoming ready
- **WHEN** a copied helper cannot validate or prepare its candidate
- **THEN** the manager remains running and no replacement or installer launch occurs

#### Scenario: A macOS post-check rejects the new bundle
- **WHEN** atomic exchange succeeded but validation of the installed candidate fails
- **THEN** the valid retained previous generation is exchanged back without requiring the rejected candidate to pass validation

#### Scenario: Stage cleanup is interrupted or deferred
- **WHEN** a helper or installer still holds files, cleanup partially fails, or another update begins
- **THEN** ownership evidence and per-attempt receipts permit later cleanup of obsolete stages while unconfirmed rollback backups remain recoverable

### Requirement: Review builds use isolated verification data

Automated destructive workflow verification MUST use an explicitly isolated data root and verify resolved settings and Codex-home paths before running domain operations. An unverifiable isolation environment SHALL be rejected before application-state writes.

#### Scenario: A verification root cannot be honored
- **WHEN** the resolved settings or Codex-home location escapes the requested isolated root
- **THEN** the test launch fails before loading or migrating real user data
