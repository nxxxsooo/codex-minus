## 1. Live State Security And Transactions

- [x] 1.1 Inventory every settings, `config.toml`, `auth.json`, and generated-catalog write path, then route switch, apply, clear, active save, adoption, refresh snapshot, and materialization through one process-wide live-state coordinator that prohibits provider-driven live-auth writes.
- [x] 1.2 Add cross-platform owner-only directory and file helpers; repair and verify the application state directory, settings file, Codex home, auth file, temporary refresh home, and generated catalogs before secret-bearing operations succeed.
- [x] 1.3 Refactor Context protection into a fallible transaction guard that requires a valid pre-write snapshot and treats graft, parse-back, protected-table verification, or restoration failure as command failure.
- [x] 1.4 Remove the raw auth editor and reject backend `auth.json` save requests; wrap the remaining raw config path and every root-config write path in the coordinator and fail-closed Context transaction, with regression tests for `mcp_servers`, `skills`, `plugins`, and unrelated root settings.
- [x] 1.5 Replace the frontend's split active `config.toml` and `auth.json` saves with one backend transaction that stages settings, provider config, catalog, pointer ownership, and restart state, leaves live auth unchanged, and rolls back the complete prior generation on failure; use pinned lower-level apply primitives rather than the upstream SettingsStore-writing switch orchestrator.
- [x] 1.6 Stop saving or applying `authContents` for every provider, return only sanitized live-auth status to profile editors, and enforce backend stripping on every settings save.
- [x] 1.7 Add guarded legacy auth migration that secures settings first, extracts pure API keys into owner-only provider config with `requires_openai_auth = false`, removes OAuth copies without credential backups, and never restores a profile payload to live auth.
- [x] 1.8 Add an owner-only multi-file transaction journal with staged and prior hashes, commit phases, startup recovery, verified roll-forward/rollback, and cleanup tests for interruption at every rename boundary without storing ChatGPT OAuth or model instructions.

## 2. Compatibility Spikes

- [x] 2.1 Exercise configured target CLI discovery and the offline `codex debug models --bundled` capability probe; record executable identity, whole client-version behavior, output shape, exit behavior, timeout behavior, and cleanup without reading credentials.
- [x] 2.2 Define and test official target trust verification: canonical bundle and CLI paths, symlink-escape rejection, macOS code signature and OpenAI Team Identifier, Windows trust chain and publisher, unsupported-platform gating, and swapped or unsigned CLI failure.
- [x] 2.3 Build disposable test auth fixtures using file-backed `chatgptAuthTokens` with an ID/access token, synthetic load timestamp, no API key, and an empty or absent refresh-token credential; prove supported target CLIs can fetch on cache miss and cannot persist or rotate auth.
- [x] 2.4 Define the supported target-Codex versions and credential-store modes, including fail-closed results for keyring-only, invalid, missing, expired, rejected, account-changed, and concurrently changed live auth.
- [x] 2.5 Verify the isolated child starts from an explicit locale, network/TLS, and process-safety environment allowlist and always selects the built-in official provider without inheriting credential, provider, base-URL, or auth-endpoint overrides.
- [x] 2.6 Build representative custom catalogs with a conservative template policy and verify required fields, picker visibility, context windows, and stripped official-only capabilities through target-matched offline `codex debug models` runs.

## 3. Catalog State And Migration

- [x] 3.1 Add a tolerant, versioned app-owned catalog state schema for official snapshot metadata, verified target CLI identity, local non-reversible account/workspace scope hash, operation generations, per-profile modes, overlays, external ownership, provider-discovery evidence, and restart state without credential fields.
- [x] 3.2 Add permission-safe state and generated-catalog path helpers with profile-ID sanitization, owner-only files, atomic writes, parse-back validation, unchanged-file elision, and recovery of interrupted temporary files.
- [x] 3.3 Implement read-only migration classification for existing `modelList`, `modelWindows`, configured defaults, manager-owned paths, per-profile external `model_catalog_json` pointers, and legacy OAuth copies.
- [x] 3.4 Preserve `modelList` and `modelWindows` as compatibility projections and add round-trip tests proving migration does not copy API keys, auth contents, provider bearer tokens, or unrelated relay configuration into catalog state.

## 4. Official Catalog Refresh

- [x] 4.1 Reuse target-application discovery, add canonical path and platform publisher verification, and version-check the embedded Codex CLI rather than the first executable on `PATH`.
- [x] 4.2 Implement a read-only stable live-auth snapshot that structurally validates ChatGPT mode and account/workspace identity, derives a non-refreshable access-token-only projection, computes an owner-only scope hash, and re-checks live generation and identity before catalog commit without any write-back path.
- [x] 4.3 Implement the private owner-only temporary `CODEX_HOME` adapter with minimal official configuration, explicit safe child-environment allowlist, empty initial cache, bounded CLI execution, redacted output, and cleanup on every exit path.
- [x] 4.4 Cross-check command output and the isolated cache, preserve raw target-emitted model JSON, reject empty, provider-ambiguous, non-rich, duplicate, or whole-version-incompatible results, bind the snapshot to target and account/workspace scope, and commit only after complete validation.
- [x] 4.5 Fail expired or unauthorized access tokens without live-auth writes and return an official-client refresh or sign-in action while retaining the prior baseline.
- [x] 4.6 Compute stable added, updated, removed, and collision diffs from snapshot hashes and metadata while retaining the previous baseline on failure or generation change.

## 5. Provider Catalog Composition

- [x] 5.1 Implement catalog modes and defaults for native official, official mixed, pure API, and per-profile externally managed catalogs while capability-gating aggregate and removed-proxy profiles out of managed catalog behavior.
- [x] 5.2 Implement deterministic official-plus-custom merging that preserves complete and hidden official entries while applying only allowed visibility, order, and context-window overrides.
- [x] 5.3 Store timestamped `/v1/models` results as non-authoritative reporting evidence; retain omitted official entries with unknown/not-reported status and offer non-official IDs only as explicit custom candidates.
- [x] 5.4 Implement custom-only composition and conservative custom-entry generation with unique slugs, validated context windows, template provenance, and no unrequested official-backend capability claims.
- [x] 5.5 Handle custom-to-official slug promotion, duplicates, malformed overlays, removed defaults, and inactive-profile partial regeneration without replacing the last valid effective catalog.
- [x] 5.6 Add offline effective-catalog validation through a private static target-CLI configuration so inactive profiles can be checked without a network request or credential read.
- [x] 5.7 Materialize namespaced effective catalogs with owner-only atomic replacement, content hashes, generation metadata, parse-back validation, and unchanged-file elision.

## 6. Live Catalog Ownership And Adoption

- [x] 6.1 Add manager-owned pointer recognition that requires namespaced generated paths plus matching state metadata and never infers ownership from filename similarity alone.
- [x] 6.2 Apply the target profile's catalog stage after pinned upstream provider logic in a private home without live auth; use an unpersisted mixed config-bearer compatibility projection for pure API, discard staged auth, and commit only config plus pointer inside the unified transaction.
- [x] 6.3 Implement target-profile pointer semantics: reapplying an external profile preserves its pointer and file, switching away applies the target profile's pointer state, and no switch modifies or deletes an external file.
- [x] 6.4 Add complete rollback and crash-recovery tests for settings, configuration, generated catalog, pointer, protected Context tables, active generation, and restart state across every commit failure point while asserting live auth remains byte-for-byte unchanged.
- [x] 6.5 Implement explicit external-catalog adoption with parse and diff preview, non-secret configuration backup, compatible overlay extraction, validation, and reversible ownership transfer.

## 7. Frontend Experience

- [x] 7.1 Add a global official-catalog status band showing source, target client version, last successful refresh, visible and total counts, freshness, operation progress, credential action, and a capability-gated refresh action.
- [x] 7.2 Add provider catalog mode controls with per-profile external ownership, current effective-catalog status, and a restart-required indicator for active static catalogs.
- [x] 7.3 Replace duplicated official rows with an official-list view plus provider overlay editor for custom models, visibility, ordering, context windows, validation, and default-model impact.
- [x] 7.4 Show provider-reported, unknown/not-reported, and custom-candidate states separately; keep `/v1/models` discovery non-authoritative and never hide an official entry from omission.
- [x] 7.5 Add update and adoption diff presentation for additions, metadata updates, removals, collisions, invalid defaults, partial profile failures, unchanged results, and legacy OAuth migration actions.
- [x] 7.6 Keep catalog loading, refresh, composition, and materialization asynchronous and prevent their loading or error states from resizing or blocking normal provider navigation.

## 8. Verification And Delivery

- [x] 8.1 Add Rust tests for OAuth-copy removal, pure API config-bearer projection without ChatGPT auth, stale profile-auth rejection, raw auth-save rejection, owner-only permissions, coordinator serialization, fail-closed Context protection, unified active saves, crash recovery, unchanged live auth, and absence of OAuth backups or credential diagnostics.
- [x] 8.2 Add Rust tests for state migration, mode defaults, account/target scope staleness, official-field preservation, hidden entries, provider evidence, overlays, slug promotion, duplicates, invalid defaults, partial failures, hashes, and external pointer ownership.
- [x] 8.3 Add refresh-adapter tests using fake signed/trusted and untrusted target fixtures for success, malformed output, timeout, missing cache, inherited environment overrides, empty refresh credential, expired access token, account mismatch, concurrent live-auth change, active target process, symlink escape, publisher failure, and cleanup.
- [x] 8.4 Add frontend tests for status, credential actions, mode changes, overlay editing, provider evidence, custom candidate import, update diffs, external adoption, invalid defaults, partial failures, and restart state.
- [x] 8.5 Run `npm run check`, `npm run vite:build`, `cargo test` in `src-tauri`, and the full Tauri build.
- [x] 8.6 With explicit live-request approval, exercise successful and unchanged official refreshes, expired-token failure without rotation, official-client re-login recovery, official-plus-custom materialization, provider restoration, external-profile switching, and packaged-app restart behavior using disposable catalog state.
- [x] 8.7 Update README and BOARD.md with OAuth ownership, catalog modes, provider-evidence semantics, refresh and restart behavior, migration results, verification evidence, unsupported credential stores, and remaining target-version limits.
