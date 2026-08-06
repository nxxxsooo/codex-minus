## Context

See `proposal.md` for the motivation. Codex-- currently exposes `RelayProfile.modelList` and `modelWindows`, and `/v1/models` discovery appends IDs to those rows. The pinned `codex-plus-core` generates a per-profile catalog only when at least one context-window override exists; that generated catalog contains only profile models cloned from a bundled template. A user-authored `model_catalog_json` pointer is preserved and prevents regeneration.

Current Codex versions construct a static catalog manager whenever `model_catalog_json` is configured. Without that pointer, the active provider can request `/models` and write the shared `models_cache.json` under either OAuth or API authentication when its configuration supports model discovery. A mixed OAuth/API profile sends that request through the active custom provider and provider-scoped bearer token. The upstream cache does not yet bind entries to provider identity, so neither a live mixed-provider cache nor a relay `/v1/models` response is an independent official-catalog source.

Existing official and mixed profiles can persist full ChatGPT OAuth payloads in `authContents`. Those copies can outlive refresh-token rotation and later overwrite the current live identity during a provider switch. The current backend also returns raw live auth to the profile editor, which can recreate a copy after migration. The raw live-file save bypasses `with_context_tables_protected`, the guard reports success when snapshot or restoration fails, and upstream writes can leave secret-bearing files with group/world-readable modes. Target discovery accepts a configured path and checks only that a CLI file exists; that is insufficient before giving a subprocess an access token. These are prerequisites for safe catalog orchestration, not independent cleanup.

The implementation must preserve the trimmed architecture: no renderer injection, launcher, watcher, local protocol proxy, provider-logic fork, or background daemon. The target Codex Desktop CLI, rather than the first executable on `PATH`, owns catalog schema and official endpoint behavior.

## Goals / Non-Goals

**Goals:**

- Keep one validated official baseline independent of the active inference provider.
- Represent provider customization as a small overlay and compile deterministic effective catalogs.
- Preserve complete official metadata emitted by the target CLI, including hidden and newly introduced fields, across refreshes.
- Keep ChatGPT OAuth under the live target auth store and prevent Codex-- from owning or rotating refresh tokens.
- Make settings, authentication, provider configuration, materialization, and live-pointer writes permission-safe, serialized, fail-closed, and recoverable.
- Keep official-only users on Codex's native dynamic catalog until they opt into managed overlays.
- Retain compatibility with existing relay profiles and the pinned upstream dependency without vendoring provider logic.

**Non-Goals:**

- Discover GPT releases from announcements, scrape product pages, or predict model availability.
- Prove that a model advertised by the official catalog is served by a custom relay account or group.
- Hide official models merely because a relay `/v1/models` response omits them.
- Modify Sub2API account mappings, quotas, group eligibility, or `/v1/models` behavior.
- Add managed catalog behavior to aggregate profiles or restore the removed Chat Completions proxy path.
- Add arbitrary editing for instructions, tool capabilities, reasoning metadata, or service tiers in the first version.
- Store or apply live-auth payloads through provider profiles.
- Hot-reload a static catalog inside a running Codex process or restart Codex automatically.
- Run scheduled work while Codex-- is closed.

## Decisions

### Keep catalog state separate from upstream relay settings

Codex-- will persist an app-owned catalog state file under the existing application state directory. It contains the official snapshot metadata, target CLI identity, catalog modes, provider overlays, migrations, provider-discovery evidence, and operation state. It contains no auth or API credentials. `RelayProfile.modelList` and `modelWindows` remain compatibility projections so existing profiles, presets, and upstream calls continue to work.

Generated catalogs live under `$CODEX_HOME/model-catalogs/` with a `codex-minus-<profile-id>.json` filename. The root pointer is relative to `CODEX_HOME` where supported. Manager ownership requires both the generated namespace and matching state metadata; filename similarity alone is insufficient. Any other pointer is external to Codex--.

Alternative considered: add fields to `codex-plus-core::RelayProfile`. Rejected because catalog ownership is app-specific, would force an upstream schema change for local orchestration state, and risks coupling this feature to provider logic.

### Keep all provider credentials out of live auth

No provider profile persists or applies an `authContents` payload. The live target auth store remains owned by the official client and is never replaced by provider switching. Mixed and pure API profiles retain provider-specific API credentials in owner-only settings and materialize them through the supported custom-provider `experimental_bearer_token` configuration path. A pure API profile keeps `custom-only` catalog semantics and does not require a ChatGPT login.

Backend payloads expose only sanitized live-auth status, mode, account-generation identity, and required actions. They never return token-bearing live auth JSON to the renderer or profile draft. Backend normalization clears `authContents` from every provider on every settings save, so older or modified frontends cannot recreate a live-auth copy.

Migration parses legacy auth structurally. It extracts pure API keys into the profile's provider configuration, clears every profile `authContents`, and never copies ChatGPT OAuth into a replacement location or credential backup. If no usable live ChatGPT identity exists, official catalog refresh becomes action-required until the user signs in through the official client; provider switching still never promotes a stored profile copy to live auth. This intentionally removes profile-driven ChatGPT account switching.

The pinned core's `PureApi` projection normally moves the provider key into `auth.json`, while its official-mixed projection supports `experimental_bearer_token`. Codex-- therefore creates a non-persisted compatibility profile for the upstream apply call: user-facing pure API mode and `custom-only` catalog state remain unchanged, but the staged provider write uses the existing mixed config-bearer path, explicitly sets `requires_openai_auth = false`, and supplies empty `authContents`. The private staging home is never seeded with live auth. Codex-- commits only the staged provider config and discards the staged auth output, leaving live auth byte-for-byte unchanged. This adapter is covered by target-version integration tests and does not vendor provider generation logic.

The application state directory uses owner-only access and every secret-bearing settings or auth file uses owner-only permissions, with the platform-equivalent ACL on non-Unix systems. Permission repair runs before credential migration or live writes. Every upstream write path verifies and repairs the final mode before returning success; on Unix, the owner-only parent directory prevents exposure during replacement.

Alternative considered: synchronize each rotated live token into every profile copy. Rejected because it multiplies the credential, makes stale generations recoverable by later switches, and turns catalog refresh into an OAuth-account manager. Keeping pure API keys in `auth.json` was also rejected because switching to pure API would replace the only live ChatGPT identity with no permitted recovery copy.

### Refresh through a non-refreshing target-matched Codex CLI

The refresh adapter resolves the CLI bundled with the configured target Codex or ChatGPT installation, canonicalizes the bundle and CLI paths, rejects symlink escape, verifies the official platform publisher and bundle relationship, then capability-checks `debug models`. On macOS the app and embedded CLI must pass code-signature validation and a supported OpenAI Team Identifier; Windows uses the platform trust chain and supported OpenAI publisher. A platform without an implemented trust verifier can perform credential-free probes but cannot receive an access-token projection. The adapter does not use the shell's `codex` wrapper or a Homebrew binary.

For each refresh it snapshots a valid file-backed live ChatGPT identity, creates a private owner-only temporary `CODEX_HOME`, and writes a minimal official-provider configuration without `model_catalog_json`. Its temporary `auth.json` is a deliberately non-refreshable `chatgptAuthTokens` projection containing only the current ID/access token, account metadata, and a synthetic load timestamp needed by the target CLI. It contains no API key or usable refresh-token credential; a target-required `refresh_token` schema field is an empty string. The child starts from an empty environment and receives only an explicit locale, network/TLS, and process-safety allowlist plus the temporary `CODEX_HOME`; credential, provider, base-URL, and auth-endpoint overrides are never inherited. The temporary home starts with no model cache, so `debug models` must request the official catalog.

Codex-- never propagates temporary auth to live state. An expired or rejected access token fails closed and tells the user to let the official client refresh or re-establish login before retrying. The target client may remain running because the adapter cannot rotate credentials; however, a live-auth generation or account change before catalog commit invalidates the result. Keyring-only or otherwise unreadable live credentials remain unsupported until the target exposes a non-mutating export or model-catalog command that can use them safely.

Alternative considered: copy managed OAuth with its refresh token and propagate any rotation. Rejected because a routine catalog read could consume a single-use refresh token, race the official client, and be undone by stale profile copies. Temporarily switching the live provider was rejected because it mutates routing and expands rollback scope. Calling the private models endpoint directly was rejected because it would duplicate endpoint selection, request headers, and catalog schema handling owned by Codex.

### Treat the official cache as evidence, not a customization file

The isolated command output and its `models_cache.json` are cross-checked, parsed, and validated, then recorded as an app-owned official snapshot with source metadata, a content hash, target CLI identity, and a local non-reversible account/workspace scope hash. Codex's live `models_cache.json` is never patched or used as the materialized custom catalog.

Validation requires valid JSON, a non-empty model array, unique non-empty slugs, at least one visible model, the target CLI's whole client version, and the rich fields required by that target. Official entries are retained as raw JSON values after target-compatible validation so fields unknown to Codex-- are not normalized away. A refresh result that fails validation cannot replace the previous snapshot.

If the current live account/workspace scope or verified target CLI identity differs from the snapshot, that baseline becomes scope-stale. Existing generated files remain active for continuity, but the stale baseline cannot drive new composition or materialization until a successful refresh under the current scope. Scope hashes remain owner-only state and never appear in frontend payloads or diagnostics.

Alternative considered: merge custom entries directly into live `models_cache.json`. Rejected because Codex treats it as a short-lived, versioned remote cache that may come from whichever provider is active and may replace it at any time.

### Compose a complete catalog from baseline and overlay

Catalog modes compile as follows:

- `native-official`: no generated catalog and no manager-owned pointer.
- `official-plus-custom`: all official entries, including hidden entries, followed by allowed official overrides and ordered custom entries.
- `custom-only`: ordered custom entries and their required catalog metadata only.
- `external`: no composition or manager write until explicit adoption.

Official entries are copied as complete JSON values. First-version official overrides are limited to visibility, ordering, and one context-window value that updates the active and maximum window consistently while leaving all other fields intact. A custom entry stores slug, optional display name, context window, visibility, order, and template provenance. Its generated metadata uses a deterministic compatible template but removes backend-only capability claims unless they are explicitly retained by the chosen template policy.

`official-plus-custom` never intersects the official baseline with provider discovery. A provider `/v1/models` result is timestamped, non-authoritative evidence: a reported slug may be labeled reported, an omitted slug is labeled unknown or not reported, and neither state silently adds, removes, or hides an official entry. Newly discovered non-official IDs remain explicit custom-model candidates.

When a custom slug appears in a later official snapshot, the new official JSON becomes its baseline and the allowed overlay fields remain. Duplicate output slugs are a validation error. A removed default model blocks materialization for the affected profile rather than silently selecting another model.

Alternative considered: regenerate every entry through the current bundled-template helper. Rejected because it discards official instructions, availability, tools, tiers, and newly added fields, and can advertise stale capabilities after a Codex update.

### Make all live writes one fail-closed transaction boundary

A process-wide live-state coordinator covers official refresh snapshots, catalog adoption, effective materialization, provider switching, apply/clear commands, and active-profile saves. The frontend no longer commits active `config.toml` and `auth.json` through separate calls; the raw auth editor and backend auth-save path are removed, and provider operations do not write live auth at all. One backend transaction validates settings, uses the pinned core's lower-level normalize and switch-rules apply primitives against a private home without live auth, discards staged auth, applies the target profile's catalog ownership, verifies permissions, and commits settings, config, generated catalog, and restart state as one generation. It does not call the upstream switch orchestrator that writes the real SettingsStore outside this transaction.

Context protection becomes fallible. The Context snapshot must parse successfully before any live mutation. After the staged provider write, `mcp_servers`, `skills`, and `plugins` are re-grafted with `toml_edit`; snapshot, graft, verification, or restoration failure fails the command and restores the prior files. Success is reported only after parse-back and protected-table verification.

Catalog files use write-to-temp, flush, owner-only permission verification, parse-back validation, and atomic rename. Because multiple files cannot share one filesystem rename, the coordinator writes an owner-only transaction journal containing the prior and target generations, staged paths, hashes, and commit phase. On startup or before the next write, an incomplete journal is deterministically rolled back or completed before profiles become writable. The journal can contain owner-only API-key-bearing settings needed for recovery but never contains ChatGPT OAuth, model instructions, or diagnostic text, and is deleted after verified completion.

`native-official` removes a manager-owned pointer when the same profile leaves managed mode. An external pointer belongs to its profile: saving or reapplying that external profile preserves its pointer and file byte-for-byte, while switching to a different target profile applies the target profile's own pointer state and never leaks the source profile's pointer. External files are never modified or deleted by a switch.

Alternative considered: modify or fork upstream provider logic. Rejected because upstream calls can run inside the local transaction boundary and the project explicitly avoids vendoring provider behavior.

### Serialize generations and contain partial regeneration

Long CLI and filesystem work executes off the Tauri main thread with bounded timeouts and cancellation-safe cleanup. Each operation records a generation based on target CLI identity, live-auth identity and generation where relevant, official snapshot hash, overlay hash, and provider ID. Results are discarded if any input generation changes before commit.

Per-profile composition failures do not invalidate a valid new official snapshot or unaffected inactive profile catalogs, but each failed profile retains its last validated effective file. The active profile is never partially advanced: its catalog, pointer, settings generation, and restart state commit together.

### Make migration lazy and ownership-explicit

After the security prerequisite completes, first catalog-state load classifies existing `modelList` rows against the validated official snapshot. Exact official rows without overrides are redundant; official rows with `modelWindows` become overlays; absent slugs become custom entries in their existing order. No API keys or auth contents are copied into catalog state.

An unrecognized profile `model_catalog_json` path puts that profile in `external` mode. Adoption parses the external file, compares it with the official baseline, presents the resulting official overrides and custom entries, and backs up only non-secret configuration before changing the pointer. Migration alone never changes the active catalog pointer.

### Surface freshness, availability evidence, and activation separately

The relay screen receives one global official-catalog status band. Provider detail shows catalog mode, overlay rows, the complete effective list, and separately labeled provider-reporting evidence. Status distinguishes official baseline freshness, generated-file freshness, live-pointer state, credential action required, and restart requirement. `/v1/models` remains an explicit custom candidate import and availability-evidence action.

Refresh results report counts for added, updated, removed, and colliding slugs. Diagnostics include operation ID, target CLI version and path identity, hashes, counts, duration, and redacted errors. They exclude model instructions, prompts, credentials, provider tokens, auth file contents, and token-derived identity details.

Updating a generated file sets `restart_required` for the active profile because Codex loads `model_catalog_json` at startup. Codex-- does not restart or kill Codex.

## Risks / Trade-offs

- [The access token is expired or rejected] -> Fail without changing auth or catalogs and require the official client to refresh or re-establish login.
- [The official client changes auth during refresh] -> Re-check live-auth generation and account identity before commit and discard the catalog result on change.
- [Legacy profiles contain stale OAuth or API-key auth payloads] -> Secure settings first, stop applying all profile auth immediately, migrate pure API keys into provider config, and remove OAuth copies without credential backups.
- [A configured target path points to an untrusted executable] -> Canonicalize paths, verify the platform publisher and bundle relationship before projecting an access token, and disable credential-bearing refresh where trust verification is unavailable.
- [Codex changes catalog or auth schema] -> Use the verified target-matched CLI, preserve raw target output fields, capability-check at runtime, and keep the last validated state on mismatch.
- [A custom template advertises unsupported tools] -> Strip official-backend-only claims by default and keep advanced capability editing out of the first version.
- [An official model is unavailable through a relay] -> Keep it visible by user decision, label relay evidence as non-authoritative, and do not claim availability from omission.
- [Official removal invalidates a configured default] -> Block that profile's new materialization, retain its prior catalog, and require an explicit replacement default.
- [External catalogs contain intentional advanced edits] -> Never adopt or overwrite them implicitly; show a diff and retain per-profile external ownership.
- [Generated static catalogs do not hot-reload] -> Track and display restart-required state without restoring a launcher or automatic restart path.
- [A live write, process, or Context restoration fails mid-commit] -> Recover from the owner-only transaction journal, restore or finish one complete generation, and never report success after a protection failure.
- [The pinned upstream generator also writes a catalog] -> Stage the upstream result, materialize the namespaced manager-owned file afterward, and verify the committed target pointer.

## Migration Plan

1. Repair app-state and Codex-home permissions, install the shared live-write coordinator and crash-recovery journal, make Context protection fail-closed, and replace split active-file saves with one recoverable backend transaction that never writes live auth.
2. Stop applying all provider `authContents` immediately. Migrate pure API keys into owner-only provider configuration, remove legacy OAuth copies without creating credential backups, and require the official client whenever a live ChatGPT identity is needed.
3. Ship read-only target CLI discovery, live/cache inspection, catalog-state parsing, and migration previews without changing live pointers.
4. Add isolated manual refresh with a non-refreshable access-token projection and commit only validated official snapshots; keep existing catalogs active.
5. Add deterministic composition and offline validation for inactive profiles.
6. Enable managed materialization for newly opted-in profiles, then integrate target-profile pointer ownership into the unified switch/save transaction.
7. Offer explicit adoption for existing external pointers, including the current `models_372k.json` pattern; do not auto-adopt or modify the external file.
8. After successful adoption, project current official rows out of `modelList`, retain window overrides, and retain non-official rows as custom models.
9. Roll back or recover a catalog generation from its journal by restoring the prior settings, live configuration, generated catalog, pointer, and restart state. Live auth remains untouched. Official snapshots and overlays can remain inert while profiles return to `external` or `native-official` mode; removed profile auth copies are intentionally not restored.

## Open Questions

- What is the minimum verified embedded Codex CLI version that both exposes `debug models` and accepts a file-backed non-refreshable `chatgptAuthTokens` projection? Older or incompatible targets will remain capability-gated.
- Which conservative catalog template and capability-stripping rules produce the broadest custom Responses-provider compatibility across the supported target versions?
