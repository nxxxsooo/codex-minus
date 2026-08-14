# Design — add-active-only-session-adaptation

## D1. Own the write path; the upstream sync is unusable by construction

`codex_plus_data::provider_sync` cannot be scoped: `SESSION_DIRS` hard-codes `archived_sessions`
into the rollout walk and the sqlite update carries no WHERE beyond "differs from target", so any
call rewrites the full history. The manager therefore implements the narrow counterpart itself in
`session_adaptation.rs`, reusing the same on-disk semantics the core defined (rewrite
`session_meta` payload `model_provider` lines in the rollout JSONL; update `threads.model_provider`
by id) but addressed per session through the `rollout_path` and `db_path` that
`LocalSession` already carries. The scope guarantee is structural: the module receives only the
active mismatched sessions and has no directory walk and no table-wide statement at all.

## D2. Backup first, restore on partial failure, skip locked

Before any write, the run copies every rollout it will touch and the sqlite sidecar files of every
db it will touch into `backups_state/codex-minus-session-adapt/<timestamp>/` with a metadata
manifest (`managedBy: "codex-minus session adapt"`); pruning keeps the newest five runs and only
ever deletes directories carrying that exact marker, so it cannot eat the core's provider-sync
backups sharing `backups_state/`. Rollout writes preserve mtime (the inventory sorts by it). A
sqlite failure after a rollout rewrite restores that file's original text. Files locked by a
running Codex (Windows sharing violations, permission-denied) are skipped and counted, never
errored through — matching the archive flow's posture.

## D3. Manual stays CAS-guarded; automatic scans inside the same call

The button keeps the existing scan-generation compare-and-swap: what the user saw counted is what
gets adapted, or the command asks for a re-check. The on-switch automatic path runs scan and adapt
inside one blocking backend call-chain driven by the frontend (fresh scan → adapt with that scan's
generation), so it can never act on a stale count either. Auto-adaptation reports through a
passive notice — a failure there never blocks or un-does the provider switch itself.

## D4. The toggle lives in the session-lifecycle settings

`SessionLifecycleSettings` gains `auto_adapt_provider_on_switch: bool`, default **on** — the
fleet's stated want is "switching follows sessions automatically", the write is backed up and
active-only, and the toggle is right on the compatibility card to turn off. Serde-default keeps
old settings files loading; flipping the toggle saves immediately through the existing settings
command rather than adding a second save button.

## D5. encrypted_content is adapted but flagged

Sessions whose rollouts carry `encrypted_content` produced under another provider adapt like any
other (metadata follows the switch), but the outcome message carries the same warning the core
established: continuing or compacting those conversations can fail with
`invalid_encrypted_content`, and a reliable continuation needs the original provider or a new
session. Detection happens during the rewrite pass — no extra scan.

## D6. What this does not do

No archived row or file is ever read for writing, no `has_user_event`/`cwd` normalization (that
was desktop-app bookkeeping the core's sync also carried), no startup auto-run (the old app's
"before launch" hook belonged to its launcher; this manager adapts on switch and on demand).
