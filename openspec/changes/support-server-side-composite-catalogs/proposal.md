## Why

Codex-- currently treats every multi-vendor relay as if it required the removed local aggregation proxy, even when an upstream service exposes one Responses-compatible Base URL and key. That prevents a working server-side composite relay from using managed catalogs and also exposes fidelity bugs when mixed official and custom model metadata is materialized.

## What Changes

- Distinguish a server-side composite relay from the unsupported local member-aggregation mode. A server-side composite is applied as one ordinary Responses API upstream and can use managed catalogs; local member rotation and Chat Completions proxying remain unavailable.
- Default newly classified server-side composite profiles to `official-plus-custom`, while preserving explicit catalog-mode choices and offering an explicit conversion path for compatible existing API profiles.
- Expand official and custom overlays to carry display names, effective context percentages, reasoning levels and defaults, and explicitly configured tool capabilities without losing unrelated official metadata.
- **BREAKING**: When a managed multi-model catalog is active, stop writing profile-wide `model_context_window` and `model_auto_compact_token_limit` values. Detect existing values, explain that they override per-model metadata, and remove them through the protected apply transaction after user confirmation.
- Make external-catalog adoption intentionally structure-compatible across target CLI patch versions: record a declared-version mismatch as a warning, but do not reject an otherwise valid external catalog. Official refresh remains target-version exact.
- Add diagnostics and regression coverage for topology classification, catalog defaults, metadata round trips, context precedence, version warnings, and preservation of the removed-proxy guard.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `model-catalog-management`: Extend managed catalogs to server-side composite relays, make rich per-model metadata authoritative, and define external-version compatibility behavior.

## Impact

- Frontend provider and catalog editors in `src/App.tsx`, including profile classification, context controls, overlay fields, warnings, migration/conversion UX, and status copy.
- Catalog state, composition, validation, adoption, and tests in `src-tauri/src/model_catalog.rs`.
- Protected provider staging and root-key handling in `src-tauri/src/commands.rs`; the Context transaction, OAuth ownership, and rollback invariants remain unchanged.
- Manager-owned settings/catalog-state schema gains backward-compatible metadata. No upstream dependency fork, local proxy restoration, or live-auth write path is introduced.
- Project documentation narrows “aggregator providers” to the unsupported client-side proxy topology and records server-side composite relay support.
