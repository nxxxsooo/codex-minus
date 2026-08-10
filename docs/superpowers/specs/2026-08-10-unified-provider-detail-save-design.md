# Unified Provider Detail Save Design

## Problem

The provider detail page currently exposes conflicting persistence boundaries:

- The top-level **Save** action persists the provider profile, but not the model-catalog draft owned by `CatalogProfileEditor`.
- **Set as current** applies the provider draft while the backend still plans `config.toml` from the previously persisted catalog state. A newly selected managed mode can therefore switch successfully without writing `model_catalog_json`.
- **Extract current provider config** updates the draft and then calls `saveSettingsValue`, persisting without the top-level Save action.
- Editing or clearing the shared common-config draft can make the effective `config.toml` preview empty, instead of leaving the provider-owned layer visible.
- Native-official mode correctly removes a manager-owned `model_catalog_json`, but the UI can still display the historical generated catalog path.

This makes the page appear to save edits without Save in one place and ignore unsaved edits in another.

## Desired Behavior

The provider detail page has one editable draft and two explicit commit actions:

1. **Save** atomically persists the complete provider-detail draft: provider settings, extracted common/context configuration, and model-catalog settings.
2. **Set as current** atomically persists the same complete draft and applies that provider to live `config.toml`.

All other controls are either draft-only or clearly non-configuration operations:

- Catalog mode, topology, overrides, and custom-model controls update the shared draft only.
- Extracting common configuration updates the shared draft only.
- Clearing common configuration removes only the shared layer from the preview. It never clears or rewrites the provider-owned `configContents` layer.
- Leaving the page without Save or Set as current does not persist those draft edits.
- Refreshing provider evidence may persist evidence cache metadata, but must not change provider settings, catalog mode/overlay, generated catalog ownership, or live `config.toml`.
- Native-official mode displays that Codex uses its native dynamic catalog and does not display a historical manager-generated path or restart badge as if it were active.

## Architecture

### Shared frontend draft

`RelayProfileDetail` owns both the provider draft and a `CatalogDraft` initialized from the persisted `ProfileCatalogSummary`. `CatalogProfileEditor` becomes a controlled editor: it receives the catalog draft and reports draft changes upward. It no longer owns a separate persistence action.

The catalog draft contains:

- `mode`
- `modeExplicit`
- `upstreamTopology`
- `overlay`

Catalog validation remains available before either commit action. The top Save and Set as current controls are disabled when either provider validation or catalog validation fails.

The catalog summary returned by the backend remains the persisted state. Refreshing evidence may replace evidence-related summary fields, but must not reset a dirty catalog draft. Draft initialization/reset occurs only when the edited profile changes or after a successful commit reloads persisted state.

### Atomic backend command boundary

The provider Save and Set as current requests carry an optional catalog draft for an existing catalog-capable profile. The backend applies the provider settings and catalog state under the existing process-wide live-state lock and commits every affected mutation through one journal transaction.

For Save on the active provider, the transaction may include:

- settings JSON
- model-catalog state JSON
- generated managed catalog JSON
- live `config.toml`

For Save on an inactive provider, it includes settings, catalog state, and any inactive generated catalog materialization, but does not replace live `config.toml`.

For Set as current, it includes the same persisted draft plus live provider application. Catalog planning must use the catalog draft supplied in that request, not the previously stored catalog state.

The transaction keeps the existing Context protection, owner-only mutation journal, rollback behavior, and OAuth byte-for-byte verification. No new direct `config.toml` write path is introduced.

### Extraction and evidence refresh

The extraction command remains a pure transformation. Its frontend handler updates the provider, common-config, and context-config drafts and removes the direct `saveSettingsValue` call.

Provider-owned configuration and shared configuration remain separate draft values after extraction. The effective preview is always composed in one direction:

1. provider-owned `configContents`
2. shared non-context configuration
3. enabled context entries

Changing layer 2 or 3 recomputes the preview but never reverse-parses or mutates layer 1. In particular, changing the common draft from non-empty to empty must produce the provider-owned bytes (plus enabled context entries), not an empty provider draft. Extraction must preserve provider identity fields such as `model`, `model_provider`, `model_catalog_json`, and `[model_providers.*]` in layer 1.

Provider evidence refresh remains an explicit network/cache action. The backend may record `provider_evidence`, and the frontend may refresh evidence fields. It must not adopt candidates automatically or overwrite any catalog draft fields.

### Catalog status presentation

The catalog header derives its active description from the selected draft mode:

- `native-official`: “Use Codex native dynamic catalog.”
- `external`: the external pointer.
- managed modes: persisted generated path when it corresponds to the persisted managed state; otherwise an unsaved-state label until commit.

The restart badge is shown only for the active persisted managed/external state, not when the current draft is native-official.

## Error Handling

- A failed combined transaction leaves settings, catalog state, generated files, and live config at their prior generation.
- The detail page stays open with the user's draft intact after failure.
- Context cleanup confirmation is collected once for the complete operation and passed to the backend transaction.
- A failed evidence refresh leaves both persisted configuration and the catalog draft unchanged.
- Save and Set as current are guarded against duplicate submission while the transaction is running.

## Testing

Frontend regression tests cover:

- Catalog changes remain draft-only until commit.
- Extraction produces the next draft without requesting persistence.
- Clearing an extracted common-config draft leaves provider identity roots and provider tables in the effective preview and in the provider draft.
- Repeated common-config edits do not mutate the provider-owned draft.
- Evidence refresh results merge evidence fields without replacing a dirty catalog draft.
- Native-official presentation does not expose a stale generated path.
- The complete provider and catalog drafts are included in Save and Set as current requests.

Backend regression tests cover:

- A managed catalog draft supplied during provider switching writes exactly one root `model_catalog_json` pointer.
- A native-official draft removes a manager-owned pointer.
- Saving an inactive provider persists/materializes its catalog without changing live `config.toml`.
- A forced transaction failure rolls back settings, catalog state, generated catalog, and live config together.
- Context tables and live OAuth remain unchanged across successful and failed combined commits.

## Scope

This change removes the separate **Save catalog** persistence boundary from the provider detail page. It does not change official catalog refresh, external catalog adoption confirmation, catalog composition rules, provider evidence semantics, or OAuth ownership.
