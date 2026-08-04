## Context

Codex-- currently exposes provider switching on the relay screen and a separate provider-sync target selector on the sessions screen. The selector contains low-level Codex `model_provider` identifiers such as `openai` or `custom`, not the user-facing relay profiles. The current upstream sync operation rewrites session metadata in both `sessions` and `archived_sessions`, so even a small provider change can scan the full history.

Session listing also reads, sorts, and deduplicates every active and archived row before the frontend paginates the rendered list. There is no inactivity policy. Codex itself has native archive semantics: it moves an active rollout from `$CODEX_HOME/sessions/...` to `$CODEX_HOME/archived_sessions/`, updates state-database metadata, and supports the inverse unarchive operation.

The implementation must preserve the project's trimmed architecture. It must not restore the Codex++ launcher, watcher, or local protocol proxy; vendor or fork provider logic; or mutate Codex configuration outside the existing context-protected paths.

## Goals / Non-Goals

**Goals:**

- Keep the active session set bounded with a consented 30-day inactivity policy.
- Use Codex-native archive storage and restoration so archived sessions remain compatible with Codex.
- Keep application startup and the first active-session page independent of archive maintenance and archived-history loading.
- Eliminate manual low-level provider-target selection from the normal workflow.
- Avoid provider scans entirely when two relay profiles resolve to the same effective `model_provider`.
- Make provider adaptation scale with mismatched active sessions, not all historical sessions.
- Preserve explicit confirmation, backups, rollback behavior, and warnings for provider-sensitive encrypted content.

**Non-Goals:**

- Reclaim disk space, compress archives, or permanently delete archived sessions.
- Run a LaunchAgent, daemon, or maintenance process while Codex-- is closed.
- Promise that archiving improves ChatGPT or Codex client launch time without measured evidence.
- Automatically rewrite every historical session as part of a provider-switch transaction.
- Add an arbitrary provider-id editor or reintroduce removed Codex++ management features.

## Decisions

### Use Codex-native archive and unarchive operations

Codex-- will discover the CLI that belongs to the target Codex installation and invoke its supported archive and unarchive interface by UUID. It will not trust an unrelated executable found first on `PATH`, directly edit archive columns, or recreate Codex's file-move rules.

The adapter must use the effective `CODEX_HOME`. It must verify that the native operation produced a consistent file location and state-database record. If the target client does not expose a compatible native operation, the feature remains unavailable and reports an actionable error without writing anything.

Alternative considered: implement file moves and SQL updates in Codex--. Rejected because current Codex semantics include state metadata, archive timestamps, restored paths, and spawned descendants that can change with the client schema.

### Archive by inactivity after first-run consent

The default threshold is 30 exact 24-hour periods since the last recorded activity. Unknown timestamps, already archived sessions, and loaded or running sessions are ineligible. The first maintenance run shows a count and cutoff preview and requires confirmation; declining leaves automatic archiving disabled.

After enablement, Codex-- schedules a catch-up check after the UI is usable and no more than once per 24 hours while it remains open. A missed interval is handled on the next launch. Maintenance runs asynchronously and never gates window creation, navigation, or the first active-session page.

Immediately before each native archive call, the backend re-reads the session state and timestamp. If the session became active, changed after the cutoff, or cannot be proven idle, it is skipped. If native status cannot prove that candidates are not loaded while the Codex client is running, the automatic batch is deferred.

Alternative considered: install a system scheduler for strict wall-clock execution. Rejected because it adds a background service to a deliberately minimal manager.

### Treat archive as organization, not backup or deletion

The canonical archive remains `$CODEX_HOME/archived_sessions`. Existing deletion backups under the manager state directory remain a separate recovery mechanism. Archiving does not create a duplicate copy and does not claim to reduce disk usage.

Each session is an independent operation. A partial batch keeps successful native archives, reports failures, and performs an inventory consistency check. Rollback of the feature disables future automation; existing native archives remain visible and recoverable through unarchive.

### Query active and archived sessions separately

The Tauri API will accept an archive filter, cursor, and page size and return lightweight counts plus one requested page. The active page is the default; archived history is loaded only when selected. Sorting and deduplication happen within the requested bounded result rather than loading the full history into the frontend.

This is the direct Codex-- responsiveness improvement. Any effect on ChatGPT or Codex client launch time must be measured separately and reported as evidence, not assumed.

### Derive compatibility from the effective provider identity

Relay profiles and Codex provider identities are different concepts. After a successful profile switch, the backend reads the effective live provider identity using Codex semantics. A saved profile ID, previous sync selection, or dropdown value is never the repair target.

If the effective provider identity did not change, no compatibility scan runs. If it changed, a read-only scan counts mismatched active sessions and reports warnings. It does not traverse archived rollouts. The provider switch remains complete regardless of whether the user later adapts sessions.

Alternative considered: automatically run provider sync after every profile switch. Rejected because API-to-API profiles commonly share `custom`, and automatic full-history rewrites increase latency, rollback scope, and encrypted-content risk without benefit.

### Adapt active sessions explicitly and archived sessions on restoration

The normal UI removes the provider target selector and offers `Adapt to current provider` only when a fresh scan found mismatches. The user confirms after seeing the effective target, affected active count, skipped in-use count, and encrypted-content warning.

Bulk adaptation reads and writes active session rollouts and their dependent index rows only. It uses a scoped upstream API; Codex-- must not copy or fork the existing full-history provider-sync implementation. If the required scope is unavailable, implementation must first add it upstream and bump the pinned revision.

Archived rollouts are not scanned or rewritten by bulk adaptation. When a session is restored, Codex-- checks that one restored session against the then-current provider and requests confirmation before adapting it.

### Make concurrency and stale results explicit

Compatibility scans carry the effective provider identity and a scan generation. Results are discarded if the provider changes before display or before adaptation begins. Provider switching, adaptation, archiving, unarchiving, and deletion use mutually compatible operation locks so the same rollout cannot be changed concurrently.

Archive and adaptation summaries include candidates, scanned items, changed items, skipped items, failures, and elapsed time. This evidence is the acceptance basis for reduced provider-repair work.

## Risks / Trade-offs

- [Native CLI behavior differs across Codex versions] -> Discover the target-matched CLI, capability-check archive commands, validate postconditions, and fail closed.
- [A session becomes active during maintenance] -> Re-read eligibility immediately before mutation and skip loaded, running, locked, or recently updated sessions.
- [Archive file move succeeds but state metadata is stale] -> Run a post-operation parity check and report the session as inconsistent rather than applying ad hoc SQL repair.
- [Archived history still consumes disk] -> State this explicitly; permanent deletion is a separate, destructive future change.
- [Encrypted session content is provider-sensitive] -> Keep adaptation explicit, show the upstream warning, and leave the original session untouched when declined.
- [Scoped provider sync is missing upstream] -> Add a scope boundary upstream and bump the pinned dependency; do not vendor the implementation locally.
- [Automatic maintenance is delayed while applications are closed or candidates are busy] -> Catch up on the next safe opportunity and show pending and last-run status.
- [No measurable external client launch improvement] -> Retain decluttering and repair-scope benefits; report benchmark results without claiming causation.

## Migration Plan

1. Add new settings with a 30-day default but automatic archiving disabled until the first preview is accepted.
2. Keep existing archived sessions untouched and discover them through Codex-native state.
3. Stop using `providerSyncLastSelectedProvider` and saved/manual provider targets as behavioral inputs; retain tolerant deserialization for existing settings during one compatibility period.
4. Ship read-only filtered listing and compatibility scans before enabling mutation controls.
5. Enable native archive/unarchive and scoped active-session adaptation after capability checks and integration tests pass.
6. Roll back by disabling automatic maintenance and restoring the previous UI. Native archives remain valid and can be restored; provider-adaptation backups retain their existing recovery semantics.

## Open Questions

- Which minimum Codex CLI version first provides the archive, unarchive, and redacted inventory capabilities required by the adapter?
- Can the target-matched CLI expose loaded or running state without connecting to a separately managed app-server, or must automatic batches defer whenever the client process is active?
- Should the legacy manual provider-target settings be removed after one release or retained indefinitely as ignored compatibility fields?
