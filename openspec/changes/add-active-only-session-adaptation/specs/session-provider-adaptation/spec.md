## ADDED Requirements

### Requirement: Active sessions adapt to the current provider without touching archived history

The manager SHALL be able to rewrite the provider marker of active sessions — in each session's own rollout file and its own sqlite row — to the current provider. The write path MUST address sessions individually by the paths the session inventory carries; it MUST NOT walk session directories or issue table-wide updates, so archived sessions are unreachable by construction. Every run SHALL back up the exact files it will modify into a manager-owned backup namespace before writing, SHALL restore a session's rollout if its sqlite update fails, and SHALL skip (and count) files locked by a running Codex rather than failing the run. Rollout modification times SHALL be preserved. Backup pruning MUST only delete backup directories carrying this feature's own marker.

#### Scenario: Adaptation repairs the active mismatches and only them

- **WHEN** two active sessions and one archived session name a previous provider and the user adapts
- **THEN** both active sessions' rollout meta and sqlite rows name the current provider, the archived session's file and row are byte-for-byte unchanged, and a backup of the two modified rollouts and the sqlite sidecars exists

#### Scenario: A session held open by Codex is skipped, not corrupted

- **WHEN** a mismatched active session's rollout is locked by another process during adaptation
- **THEN** that session is skipped and reported, the remaining sessions adapt, and the run reports success with the skip count

#### Scenario: Foreign encrypted content is flagged

- **WHEN** an adapted session's rollout carries `encrypted_content` produced under the previous provider
- **THEN** the outcome warns that continuing that conversation may fail and names the reliable options

### Requirement: Adaptation runs manually under a fresh scan and automatically after a provider switch

The manual adapt action SHALL remain guarded by the scan-generation compare-and-swap: it MUST refuse to act on a count the user is no longer looking at. A persisted toggle, default on, SHALL run the same adaptation automatically after a successful provider switch, using a scan taken after the switch; the automatic pass SHALL report through a passive notice and its failure MUST NOT block, revert, or fail the provider switch. Settings persisted before the toggle existed SHALL load with the default.

#### Scenario: Switching providers carries the active sessions along

- **WHEN** the user switches the active provider with the toggle on and mismatched active sessions exist
- **THEN** after the switch a scan and adaptation run automatically and a notice reports how many sessions followed

#### Scenario: The automatic pass stays out of the switch's way

- **WHEN** the automatic adaptation fails (for example every rollout is locked)
- **THEN** the provider switch itself remains committed and reported successful, and the adaptation failure surfaces as its own notice
