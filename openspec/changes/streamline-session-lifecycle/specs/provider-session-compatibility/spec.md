## ADDED Requirements

### Requirement: Effective provider is the only adaptation target
The system SHALL derive the adaptation target from the effective live `model_provider` after a successful provider switch and SHALL NOT ask the user to select a low-level provider target in the normal workflow.

#### Scenario: Provider switch succeeds
- **WHEN** a relay profile switch completes successfully
- **THEN** the system resolves the effective provider identity from the resulting live Codex configuration and uses it as the sole compatibility target

#### Scenario: Provider switch fails
- **WHEN** a relay profile switch fails or rolls back
- **THEN** the system does not start a compatibility scan for the failed target

#### Scenario: Legacy target settings exist
- **WHEN** saved or manual provider-sync targets are present in manager settings
- **THEN** the normal compatibility workflow ignores them as adaptation targets

### Requirement: Unchanged provider identity requires no repair
The system SHALL compare the effective provider identity before and after a relay profile switch and SHALL skip compatibility scanning and adaptation when the identity is unchanged.

#### Scenario: API profile changes but provider identity remains custom
- **WHEN** the user switches between relay profiles that both resolve to `custom`
- **THEN** the system completes the switch without scanning or rewriting session provider metadata

#### Scenario: Provider identity changes
- **WHEN** a successful switch changes the effective provider identity
- **THEN** the system starts a read-only compatibility scan for active sessions after the switch completes

### Requirement: Compatibility scans are read-only and active-only
The system SHALL scan only active session metadata for provider mismatches, SHALL NOT traverse archived rollouts, and SHALL discard results that became stale because the effective provider changed.

#### Scenario: Active mismatch exists
- **WHEN** an active session records a provider identity different from the current effective provider
- **THEN** the read-only scan counts it as requiring adaptation without modifying its rollout or index rows

#### Scenario: Archived history exists
- **WHEN** archived sessions contain any provider identity
- **THEN** the bulk compatibility scan neither opens nor counts their rollout files

#### Scenario: Provider changes during a scan
- **WHEN** the effective provider changes before a scan result is displayed or used
- **THEN** the system discards that result and does not enable adaptation from it

### Requirement: Active-session adaptation requires explicit confirmation
The system SHALL offer adaptation only after a fresh mismatch scan and SHALL require confirmation that identifies the effective target, affected active count, skipped in-use count, and any encrypted-content warning.

#### Scenario: User confirms adaptation
- **WHEN** the user confirms a fresh compatibility result
- **THEN** the system adapts eligible mismatched active sessions to the current effective provider

#### Scenario: User declines adaptation
- **WHEN** the user declines or dismisses the confirmation
- **THEN** the provider switch remains active and all session metadata remains unchanged

#### Scenario: Encrypted content is detected
- **WHEN** one or more affected active sessions contain provider-sensitive encrypted content
- **THEN** the confirmation explains the compatibility risk before adaptation can begin

#### Scenario: No mismatches exist
- **WHEN** a fresh scan finds no active provider mismatch
- **THEN** the system reports compatibility as current and does not offer a redundant adaptation action

### Requirement: Adaptation is scoped, recoverable, and concurrency-safe
The system MUST adapt only eligible active sessions, MUST preserve backups and rollback behavior for changed metadata, and MUST skip sessions that are loaded, running, locked, or concurrently modified.

#### Scenario: Scoped adaptation runs
- **WHEN** the user confirms adaptation
- **THEN** the implementation reads and writes active rollout metadata and dependent active index rows only, without accessing `archived_sessions`

#### Scenario: Session is in use
- **WHEN** an affected session is loaded, running, locked, or changes after the scan
- **THEN** the system skips it and includes the reason in the result

#### Scenario: A later write fails
- **WHEN** adaptation changes rollout metadata but a dependent index update fails
- **THEN** the system restores the changed rollout metadata and rolls back the associated state-database transaction where supported

#### Scenario: Scoped upstream operation is unavailable
- **WHEN** the pinned upstream dependency cannot restrict provider sync to active sessions
- **THEN** Codex-- does not fall back to the full-history operation or vendor provider logic and keeps adaptation unavailable until a supported upstream scope is integrated

### Requirement: Restored sessions are checked individually
The system SHALL check a restored session against the current effective provider after native unarchive and SHALL NOT trigger a bulk archived-history scan.

#### Scenario: Restored session already matches
- **WHEN** a restored session records the current effective provider
- **THEN** the system completes restoration without offering adaptation

#### Scenario: Restored session mismatches
- **WHEN** a restored session records a different provider
- **THEN** the system offers explicit adaptation for that session only and includes any encrypted-content warning

#### Scenario: User declines restored-session adaptation
- **WHEN** the user declines adaptation after restoring a mismatched session
- **THEN** the session remains restored with its original provider metadata and no other archived session is accessed

### Requirement: Compatibility operation evidence
The system SHALL report provider identity, active candidates, scanned sessions, changed sessions, skipped sessions, failures, and elapsed time so reduced repair work can be verified.

#### Scenario: Same-provider profile switch completes
- **WHEN** a profile switch leaves the effective provider unchanged
- **THEN** the operation evidence reports zero session scans and zero session writes

#### Scenario: Active adaptation completes
- **WHEN** an adaptation run finishes
- **THEN** the result reports active-only scan and change counts, duration, skips, failures, and confirmation that archived rollouts were not traversed

#### Scenario: Diagnostics are persisted
- **WHEN** compatibility diagnostics are logged
- **THEN** they exclude prompts, titles, API keys, auth contents, and rollout contents
