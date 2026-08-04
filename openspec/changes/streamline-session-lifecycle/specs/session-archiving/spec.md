## ADDED Requirements

### Requirement: Consented inactivity policy
The system SHALL offer automatic session archiving with a default inactivity threshold of 30 days, and SHALL NOT archive any session automatically until the user has reviewed a candidate preview and enabled the policy.

#### Scenario: First-run preview
- **WHEN** automatic archiving has never been enabled and inactive eligible sessions exist
- **THEN** the system shows the cutoff, candidate count, and native archive destination without modifying any session

#### Scenario: User enables the policy
- **WHEN** the user confirms the first-run preview
- **THEN** the system persists automatic archiving as enabled with a 30-day threshold and may process the previewed candidates

#### Scenario: User declines the policy
- **WHEN** the user declines the first-run preview
- **THEN** the system leaves automatic archiving disabled and performs no archive writes

#### Scenario: User changes the threshold
- **WHEN** the user saves a valid inactivity threshold
- **THEN** future previews and maintenance runs use that threshold without changing already archived sessions

### Requirement: Safe archive eligibility
The system SHALL consider only non-archived sessions whose last activity is older than the configured cutoff, and SHALL skip any session that is loaded, running, locked, recently updated, or missing a trustworthy activity timestamp.

#### Scenario: Inactive session is eligible
- **WHEN** a non-archived session has a trustworthy last-activity timestamp older than the cutoff and is not in use
- **THEN** the system includes it as an archive candidate

#### Scenario: Session changes during a batch
- **WHEN** a candidate receives activity or becomes loaded before its archive operation begins
- **THEN** the system rechecks its state, skips it, and reports the skip reason

#### Scenario: Session state cannot be proven idle
- **WHEN** the system cannot determine whether a candidate is loaded or running
- **THEN** the automatic archive operation skips or defers that candidate without mutating it

#### Scenario: Already archived or timestamp-less session
- **WHEN** a session is already archived or lacks a trustworthy last-activity timestamp
- **THEN** the system excludes it from automatic archive candidates

### Requirement: Codex-native archive storage
The system MUST use the target Codex installation's native archive operation and effective `CODEX_HOME`, and MUST NOT directly emulate archive file moves or state-database writes.

#### Scenario: Native archive succeeds
- **WHEN** an eligible session is archived
- **THEN** its rollout is located under the effective `$CODEX_HOME/archived_sessions` and its Codex state metadata identifies it as archived with the native archive path

#### Scenario: Native capability is unavailable
- **WHEN** the target-matched Codex installation does not expose a compatible archive operation
- **THEN** the system disables archive mutation, reports an actionable compatibility error, and performs no direct SQL or filesystem workaround

#### Scenario: Post-operation state is inconsistent
- **WHEN** the native operation returns but the rollout location and state metadata do not agree
- **THEN** the system reports the session as inconsistent and does not conceal the failure with an ad hoc metadata edit

#### Scenario: Partial batch failure
- **WHEN** one session fails during a multi-session archive batch
- **THEN** successful native archives remain archived, remaining eligible sessions may continue, and the final summary identifies each failure

### Requirement: Non-blocking maintenance schedule
The system SHALL run archive maintenance only after the manager is usable, no more than once per 24-hour interval while open, and SHALL catch up on the next launch without requiring an operating-system daemon.

#### Scenario: Manager launches with maintenance due
- **WHEN** the manager starts and the enabled policy is due
- **THEN** it renders its usable interface before starting maintenance asynchronously

#### Scenario: Manager remains open
- **WHEN** 24 hours have elapsed since the last completed maintenance check
- **THEN** the system schedules another asynchronous check without blocking navigation or session listing

#### Scenario: Manager was closed during a due interval
- **WHEN** the manager next launches after a missed interval
- **THEN** it schedules one catch-up check after the interface is usable

#### Scenario: Maintenance is already running
- **WHEN** another schedule trigger fires during an active maintenance operation
- **THEN** the system coalesces the trigger and does not run overlapping archive batches

### Requirement: Native restoration
The system SHALL allow an archived session to be restored through the target Codex installation's native unarchive operation and SHALL keep restored sessions from being immediately re-archived.

#### Scenario: Restore succeeds
- **WHEN** the user restores an archived session
- **THEN** the native rollout returns to the active sessions hierarchy, Codex state marks it active, and its refreshed activity state starts a new retention interval

#### Scenario: Restore fails
- **WHEN** the native unarchive operation fails or its postconditions are inconsistent
- **THEN** the system reports the failure and does not apply a direct SQL or filesystem workaround

### Requirement: Bounded session listing
The system SHALL query active and archived sessions separately with backend pagination, SHALL show active sessions by default, and SHALL load archived history only on request.

#### Scenario: Active page opens
- **WHEN** the user enters session management
- **THEN** the system requests a bounded first page of active sessions and lightweight active and archived counts without loading all archived rows

#### Scenario: Archived page opens
- **WHEN** the user selects archived sessions
- **THEN** the system requests a bounded archived page without reloading the full active history

#### Scenario: More results are requested
- **WHEN** the user requests the next page
- **THEN** the backend returns the next bounded result using a stable cursor and ordering

### Requirement: Archive operation evidence
The system SHALL report archive candidates, successful archives, skips, failures, consistency results, and elapsed time without exposing session content.

#### Scenario: Maintenance completes
- **WHEN** an archive maintenance run finishes
- **THEN** the interface and diagnostic log expose aggregate counts, duration, cutoff, and last-completed time without titles, prompts, or rollout contents in diagnostics

#### Scenario: External client performance is evaluated
- **WHEN** launch performance is compared before and after archival
- **THEN** the result is recorded as measured evidence and the system does not claim an improvement when the measurement is inconclusive
