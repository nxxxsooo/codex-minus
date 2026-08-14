# Proposal: add-active-only-session-adaptation

## Why

Switching providers strands active sessions: their records still name the previous provider, Codex greets each one with a repair prompt, and the manager's「适配到当前 provider」button is a hard-disabled stub. It was disabled for a real reason — the upstream `provider_sync` rewrites the whole history by construction (its rollout walk hard-codes `archived_sessions` beside `sessions`, and its sqlite update is a WHERE-less `UPDATE threads SET model_provider = …`), so with 10k+ archived sessions on the owner's machine, enabling it meant rewriting history the product promised never to touch. The original fat app had an auto-repair toggle riding exactly that all-or-nothing machinery; the user wants the capability back (「拉出来，并且切供应商自动」) without the history rewrite.

## What Changes

- The manager grows its own active-only adaptation: for each active session whose provider differs from the current one, rewrite the `session_meta` provider in that session's own rollout file and update that session's own sqlite row — addressed by the `rollout_path`/`db_path` the inventory already carries. Archived history is unreachable by construction, not by discipline.
- Every run backs up the exact files it will touch (rollout originals + sqlite sidecars) into a manager-owned backup namespace before writing, keeps the last runs, and restores on partial failure. Locked files are skipped and reported, never errored through.
- The「适配到当前 provider」button comes alive, still guarded by the existing scan-generation compare-and-swap.
- A toggle「切换供应商后自动适配活动会话」(default on, persisted with the session-lifecycle settings) runs the same adaptation automatically after a successful provider switch, reporting through a passive notice.
- Sessions carrying `encrypted_content` from another provider are adapted but flagged: metadata follows the switch, yet continuing those conversations may fail upstream, and the outcome says so.

## Capabilities

### New Capabilities

- `session-provider-adaptation`: active-only provider adaptation with per-run backups, manual and on-switch-automatic.

### Modified Capabilities

None.

## Impact

- `src-tauri/src/session_adaptation.rs` (new module + tests), `commands.rs` (stub becomes real; compatibility payload reports availability; lifecycle settings gain the toggle field, serde-defaulted so old state loads).
- `src/App.tsx` wiring (toggle row, adapt-after-switch), i18n entries.
- The pinned core is not touched; its whole-history `provider_sync` stays unused.
