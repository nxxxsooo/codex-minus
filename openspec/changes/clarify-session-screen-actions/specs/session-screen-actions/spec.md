## ADDED Requirements

### Requirement: Session-screen cards present one primary action and save policy inputs themselves

Each card on the session screen SHALL present at most one filled primary button — the action the card exists for. Inputs that configure policy (the retention-days field) SHALL persist themselves without a dedicated save button, debounced and silent on success, and the archive preview SHALL refresh automatically when the screen loads and after a days change rather than by manual request. The bulk-selection toolbar SHALL show a single entry point normally and reveal 全选/清空/删除/取消 only in selection mode, where 取消 exits it.

#### Scenario: Changing the retention days needs no further clicks

- **WHEN** the user edits the retention-days field and stops typing
- **THEN** the value persists on its own, the archive preview updates to the new cutoff, and no save-button click is required

#### Scenario: Selection tools appear only when selecting

- **WHEN** the user is not in selection mode
- **THEN** only 多选 is offered, and entering selection mode reveals the full toolbar including a working 取消 that leaves the mode and clears the selection

### Requirement: A manual archive check always runs and the result line reports the run's real disposition

A user-initiated 立即检查 SHALL run the archive check regardless of the automatic pass's daily interval, while still refusing when archiving is disabled or unreviewed. The result line SHALL show candidate/archived counts only for a run that actually evaluated sessions; a deferred run (target client running, operation in progress) or a run with failures SHALL show a warning with the reason, and a not-due automatic pass SHALL show its message — never zero counts under a success mark.

#### Scenario: Clicking the button inside the daily interval still checks

- **WHEN** the automatic pass completed recently and the user clicks 立即检查
- **THEN** the check runs anyway and reports real counts instead of "not due yet" with zeros

#### Scenario: A deferred run does not masquerade as a clean one

- **WHEN** the check finds candidates but defers because Codex is running
- **THEN** the line shows a warning and the deferral reason instead of a green check with zero archived

### Requirement: The update banner acknowledges the install click immediately and exactly once

Clicking the update banner's install action SHALL change the banner's visible state before any network response arrives, and further clicks during download or install SHALL have no effect — a single download runs no matter how often the user clicks.

#### Scenario: The first click is visibly acknowledged

- **WHEN** the user clicks 更新并重启
- **THEN** the banner switches to the downloading state at once, the action button disappears, and the real download events take over the progress display from there
