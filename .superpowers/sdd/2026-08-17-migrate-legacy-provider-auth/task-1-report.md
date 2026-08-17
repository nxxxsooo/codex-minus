## Task 1 implementation report

### Files changed
- `src-tauri/src/commands.rs` (migration function + regression test)

### RED command and exact failure observed
- Command:
  - `cd /Users/mingjian/Documents/sync/GitHub/codex-minus/.worktrees/legacy-provider-auth-migration/src-tauri && CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target cargo test load_time_legacy_migration_repairs_api_key_plus_oauth_residue --lib -- --nocapture`
- Initial failure after adding the new regression:
  - `thread 'commands::context_guard_tests::load_time_legacy_migration_repairs_api_key_plus_oauth_residue' ... assertion failed: left: "" right: "provider-key-sentinel"`

### Implementation decisions
- Replaced `load_time_legacy_migration_rejects_mixed_oauth_payload_without_writing` with `load_time_legacy_migration_repairs_api_key_plus_oauth_residue` to exercise the mixed official profile case with both legacy auth residue fields present.
- Reworked `migrate_persisted_legacy_api_key_auth` to:
  - Ignore unrelated fields in `auth_contents` and only extract `OPENAI_API_KEY` as the candidate legacy key.
  - Require ownership only for pure-API and official mixed profiles (`official_mix_api_key == true`).
  - Compare the candidate key against existing key destinations (`profile.api_key` and config bearer) when present, using consistent byte comparison for conflict checks.
  - Install the legacy key into `profile.config_contents` via `set_provider_config_bearer`.
  - Keep `auth_contents` cleared after migration.
- Added two tiny private helpers beside the migration function:
  - `profile_owns_provider_key`
  - `non_empty_provider_key`
- Kept the function behavior local and did not introduce any new public API.
- The test asserts migration success via persisted `experimental_bearer_token` plus removal of `authContents` and OAuth residue.
  - Note: this repo’s `RelayProfile` serialization intentionally skips `api_key`, so asserting `profile.api_key` after reload is not possible from persisted JSON.

### GREEN commands and results
- `cd /Users/mingjian/Documents/sync/GitHub/codex-minus/.worktrees/legacy-provider-auth-migration/src-tauri && CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target cargo test load_time_legacy_migration_repairs_api_key_plus_oauth_residue --lib -- --nocapture`
  - Passed (`1 passed; 0 failed`)
- `cd /Users/mingjian/Documents/sync/GitHub/codex-minus/.worktrees/legacy-provider-auth-migration/src-tauri && CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target cargo test load_time_legacy_migration --lib -- --nocapture`
  - Passed (`1 passed; 0 failed`, including the new mixed migration regression by filter prefix)
- `cd /Users/mingjian/Documents/sync/GitHub/codex-minus/.worktrees/legacy-provider-auth-migration/src-tauri && CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target cargo test normalization_projects_api_key_only_legacy_copy_to_provider_config --lib -- --nocapture`
  - Passed (`1 passed; 0 failed`)

### Self-review
- The migration no longer rejects mixed-residue JSON payloads and now deterministically reconciles legacy keys with existing structured/configed keys.
- The existing API-key-only path remains green.
- No other file was modified.

### Concerns
- The new test verifies the migrated bearer value and residue removal rather than a persisted `api_key` field because `RelayProfile` has `#[serde(skip_serializing)]` on `api_key` in `codex-plus-core`.
- No Task 2/3 work was touched; pure-OAuth behavior remains unimplemented as requested.

### Round 1/5 follow-up: pure-API startup migration contract preservation

#### Files changed
- `src-tauri/src/commands.rs`

#### RED command and exact failure observed
- Command:
  - `cd /Users/mingjian/Documents/sync/GitHub/codex-minus/.worktrees/legacy-provider-auth-migration/src-tauri && CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target cargo test load_time_legacy_migration_preserves_pure_api_contract_field_semantics --lib -- --nocapture`
- Expected/observed failure before fix (with `sanitize_profile_after_core_normalize_fallible` still called during startup migration):
  - `thread 'commands::context_guard_tests::load_time_legacy_migration_preserves_pure_api_contract_field_semantics' (16345521) panicked at src/commands.rs:6359:9:`
  - `assertion 'left == right' failed`
  - `left: Some(false)`
  - `right: Some(true)`

#### Implementation decisions
- Added `load_time_legacy_migration_preserves_pure_api_contract_field_semantics` to assert a pure-API legacy payload with `requires_openai_auth = true` retains noncanonical contract fields when startup migration runs.
- Startup migration still performs `migrate_persisted_legacy_api_key_auth` and ownership filtering, then writes via `serialize_settings_without_profile_auth`.
- Removed the startup invocation of `sanitize_profile_after_core_normalize_fallible` from `migrate_legacy_profile_auth_locked_at` so startup relocation does not mutate provider contract fields (notably `requires_openai_auth`).

#### GREEN commands and results
- `cd /Users/mingjian/Documents/sync/GitHub/codex-minus/.worktrees/legacy-provider-auth-migration/src-tauri && CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target cargo test load_time_legacy_migration_preserves_pure_api_contract_field_semantics --lib -- --nocapture`
  - `1 passed; 0 failed`
- `cd /Users/mingjian/Documents/sync/GitHub/codex-minus/.worktrees/legacy-provider-auth-migration/src-tauri && CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target cargo test load_time_legacy_migration --lib -- --nocapture`
  - `2 passed; 0 failed; 0 ignored` (includes both migration regressions)
- `cd /Users/mingjian/Documents/sync/GitHub/codex-minus/.worktrees/legacy-provider-auth-migration/src-tauri && CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target cargo test normalization_projects_api_key_only_legacy_copy_to_provider_config --lib -- --nocapture`
  - `1 passed; 0 failed`

#### Self-review
- Migration now removes `authContents` and relays `OPENAI_API_KEY` into provider config while preserving unrelated contract semantics for pure-API profiles.
- Existing Task 1 migration coverage still passes, and the pre-existing normalization migration regression was not regressed by this change.

#### Concerns
- This fix intentionally does not implement pure-OAuth behavior, which remains unimplemented per task boundary.
- Any future startup path that again calls normalize in bulk would risk reintroducing hidden contract changes.
