## Why

Session maintenance is split across unrelated controls: provider switching and provider repair choose targets independently, while the session list grows without a retention policy. This makes provider changes confusing and causes repair work to scale with the full history instead of the small active set users still need.

## What Changes

- Add an opt-in automatic archive policy with a default 30-day inactivity threshold and a first-run preview.
- Use Codex's native archive and unarchive semantics under the effective `CODEX_HOME`; do not create a private archive format.
- Run archive maintenance asynchronously after the manager becomes usable, catch up on later launches, and never block application startup.
- Separate active and archived session loading so archived history is fetched only when requested.
- Remove manual provider-target selection from the normal repair workflow.
- After a successful provider switch, derive the effective `model_provider`, perform a read-only mismatch scan, and offer an explicit adaptation action only when the low-level provider identity changed.
- Restrict bulk provider adaptation to active sessions; adapt an archived session individually when it is restored.
- Record scan, archive, adaptation, skip, failure, and duration counts so the reduction in provider-repair work can be verified.

## Capabilities

### New Capabilities

- `session-archiving`: Inactivity-based native session archiving, restoration, scheduling, consistency checks, and non-blocking session-list behavior.
- `provider-session-compatibility`: Provider-switch compatibility detection and scoped adaptation of active or restored sessions without manual target selection.

### Modified Capabilities

None. This repository does not yet have main OpenSpec capabilities.

## Impact

- Frontend session-management controls, filters, status, confirmations, and provider-switch feedback.
- Tauri commands for session discovery, native archive/unarchive orchestration, compatibility scans, scoped adaptation, and operation summaries.
- `codex-plus-data` integration, which currently scans both active and archived rollout collections during provider sync and will need a supported scope boundary without vendoring or forking provider logic.
- Settings persistence for archive enablement, the 30-day threshold, first-run consent, and last successful maintenance time.
- Tests covering filesystem and SQLite consistency, concurrency, partial failures, provider changes, archived-session restoration, and performance evidence.
