## Why

Codex-- currently stores provider model IDs and optional context windows, but it does not own a reliable official-catalog refresh path. A `model_catalog_json` override freezes Codex's dynamic catalog, mixed OAuth/API profiles send model discovery to the custom provider rather than independently refreshing the official catalog, and the existing `/v1/models` import supplies only IDs. Users therefore cannot keep official model metadata current while retaining provider-specific custom models in the native Codex picker.

## What Changes

- Add a managed official catalog baseline sourced through a platform-verified official CLI embedded in the target Codex installation, without changing the user's active provider configuration.
- Refresh the official baseline with a non-refreshable, access-token-only authentication projection; Codex-- never copies, rotates, persists, or replaces a ChatGPT refresh token.
- Add per-provider catalog modes: native official, official plus custom overlays, custom only, and externally managed catalog.
- Treat official catalog entries as an immutable upstream baseline and provider model rows as overlays for custom models and explicitly allowed visibility or context-window changes.
- Keep the complete official list in official-plus-custom mode while labeling provider availability as unknown unless `/v1/models` supplies non-authoritative evidence.
- Materialize validated, provider-specific catalogs under the Codex home and write only manager-owned `model_catalog_json` pointers through the existing context-protected configuration paths.
- Keep provider `/v1/models` discovery as a custom-model candidate source; it never replaces the rich official catalog.
- Show catalog source, target Codex version, last refresh, model counts, pending additions/updates/removals, refresh failures, and restart-required state.
- Preserve the last validated official baseline and generated catalogs on refresh, merge, validation, or configuration failures.
- Migrate existing relay `modelList` and `modelWindows` data into overlays without silently taking ownership of a user-authored external catalog.
- **BREAKING**: Stop storing or applying `authContents` from provider profiles. The live target auth store becomes the sole OAuth owner, legacy OAuth copies are removed, pure API keys migrate to the profile's provider configuration, and provider switching no longer replaces live auth or switches ChatGPT accounts.
- Harden every settings, auth, live-config, and catalog write behind user-only permissions, a shared operation coordinator, fail-closed Context protection, and atomic rollback.

## Capabilities

### New Capabilities

- `model-catalog-management`: Official catalog refresh, provider-specific catalog composition, managed materialization, migration, validation, and operational status for the native Codex model picker.

### Modified Capabilities

None. This repository does not yet have a main model-catalog capability.

## Impact

- Frontend relay management gains a global official-catalog status surface and provider-level catalog mode, overlay editor, effective-list preview, update diff, and restart state.
- Tauri commands gain target-Codex CLI discovery and publisher verification, non-refreshing isolated catalog retrieval, catalog parsing and validation, deterministic merge/materialization, OAuth-copy migration, permission repair, locking, rollback, and redacted diagnostics.
- Manager-owned catalog state is persisted separately from upstream relay settings while retaining `modelList` and `modelWindows` as compatibility projections.
- Provider profiles never persist a live-auth payload. Pure API credentials remain profile-specific in owner-only settings and materialize through the supported provider-config bearer path.
- Active-profile apply and live-config save paths become one crash-recoverable backend transaction that integrates managed catalog pointers, treats Context snapshot or restore failure as command failure, and preserves unrelated `mcp_servers`, `skills`, and `plugins` tables.
- No launcher, renderer injection, local protocol proxy, provider-logic fork, backend account change, or operating-system daemon is introduced.
