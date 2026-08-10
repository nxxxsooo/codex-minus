# Raw JSON Model Catalog Editor Design

**Date:** 2026-08-10
**Status:** Confirmed in conversation; awaiting written-spec review

## Problem

The current managed catalog editor decomposes custom models into a fixed set of GUI fields and later rebuilds each model from a conservative template. That makes common values such as `context_window = 372000` visible, but it does not preserve a complete imported model object.

The observed `gpt-5.6-sol` migration retained the 372k window while dropping fields such as `availability_nux`, `multi_agent_version`, `tool_mode`, and `use_responses_lite`, and changing other fields such as `base_instructions`, `model_messages`, and `service_tiers`. The resulting catalog is therefore not semantically equivalent to the user's original `models_372k.json` model block.

The mode transition is also unsafe as a product flow: a user can choose a managed mode before adopting an existing external catalog, which makes it possible to replace the active pointer without first preserving the user's model objects.

## Goals

- Make a complete JSON model object the source of truth for every custom model.
- Replace the wide field-by-field custom-model form with a standard JSON editing experience.
- Preserve all known and unknown fields and values when importing, editing, saving, switching providers, and refreshing the official catalog.
- Make `official-plus-custom` and `custom-only` use the same custom JSON-block editor and differ only in composition behavior.
- Require safe adoption of an existing external catalog before a managed mode can replace its pointer.
- Preserve the existing Context transaction, rollback, OAuth ownership, catalog validation, and external ownership guarantees.

## Non-goals

- The Manager will not explain or provide dedicated controls for every Codex model field.
- The Manager will not maintain bidirectional synchronization between a structured form and raw JSON.
- Provider `/v1/models` evidence will not become a source of rich model metadata.
- The Manager will not modify or delete the source external catalog during adoption.
- This change will not add the removed local proxy or alter aggregate or Chat Completions behavior.
- Agent-assisted generation or repair is tracked separately in [Agentic Model Catalog Assistant Exploration](./2026-08-10-agentic-model-catalog-assistant-exploration.md) and is not required for this editor design.

## Chosen Interaction

### Mode control

The existing catalog modes remain:

- `native-official`: use the official Codex dynamic catalog without a Manager-generated pointer.
- `official-plus-custom`: use the verified official baseline plus user-owned JSON model blocks.
- `custom-only`: use only user-owned JSON model blocks.
- `external`: keep the existing user-owned catalog pointer untouched.

### Official models

In `official-plus-custom`, the official section is read-only and collapsed by default. Its summary shows the number of models, target client version, source, and refresh time. It no longer exposes a row of per-field overrides.

The user may inspect official model summaries, but customization happens by adding a complete JSON model block with the same `slug`. Such a block is clearly labeled `Overrides official`.

### Custom JSON blocks

Each custom model is displayed as a collapsed summary card containing:

- `slug`
- `display_name`, when present
- `context_window`, when present
- validation state
- whether it overrides an official model

Expanding the card reveals a standard JSON editor for the complete model object. Long values such as `base_instructions` and `model_messages` remain inside the editor and do not expand the page while the card is collapsed.

Each block provides these actions:

- Format JSON
- Duplicate
- Delete

The primary add action is `Add JSON model`. It creates a minimal valid model-object template that the user can replace with a complete object. A separate `Import catalog JSON` action accepts a catalog object containing a `models` array and creates one raw JSON block per model.

### Save state

Editing marks the catalog as unsaved. Syntax or semantic errors appear on the affected block with a useful line or field description. `Save catalog` is disabled while any block is invalid.

A successful materialization preserves the existing restart-required behavior. Destructive choices—deleting a model, replacing an official model, or abandoning an unadopted external catalog—require explicit confirmation.

## Data Ownership and Representation

### Custom models

Each custom model is stored as a complete JSON object rather than as the current reduced `CustomModel` structure. Parsing is required for validation and composition, but the Manager must not project the object through a fixed schema before persistence.

Field order and whitespace may be normalized when formatting or materializing JSON. Field presence, field names, and JSON values must remain semantically identical unless the user edits them.

The Manager may derive summaries such as slug, display name, and context window for display. Derived summaries are not authoritative and are never written back into the object.

### Catalog wrapper

The Manager owns the generated catalog wrapper and model ordering. In `official-plus-custom`, wrapper metadata comes from the verified official catalog. In `custom-only`, the Manager produces the smallest target-compatible wrapper needed by the Codex CLI.

Adoption preserves every model object. Catalog-level fields outside `models` are validated and reported in the preview, but are not blindly merged into an official wrapper because their ownership and version semantics may conflict with the verified target catalog.

### Same-slug composition

Composition is deterministic by `slug`:

1. Start with the official models in official order for `official-plus-custom`, or an empty list for `custom-only`.
2. For each user JSON model block:
   - If its slug is not present, append it in user order.
   - If its slug matches an official model, replace the entire official model object with the user's complete object at that position.
3. Reject duplicate slugs among user blocks before materialization.

Whole-object replacement is intentional. It guarantees that a user who adopts a complete 372k model block gets that exact semantic model definition rather than an undocumented field merge.

Official refresh replaces only the official baseline. It never rewrites user JSON objects. A same-slug user object continues to replace the refreshed official object and may receive a non-blocking `Official model updated` notice so the user can compare manually.

## External Catalog Adoption

When a profile has an external `model_catalog_json` pointer, selecting a managed mode cannot immediately clear or replace that pointer.

The Manager first opens an adoption preview that:

- Resolves and validates the external file.
- Hashes the source to protect the preview-to-commit boundary.
- Shows model count, custom count, official slug collisions, target version status, and validation errors.
- Preserves every model object as raw JSON data.
- Requires explicit acceptance of any allowed target-version mismatch.

On confirmation, one transaction:

1. Creates the existing non-secret config backup.
2. Persists the adopted raw model objects in Manager state.
3. Materializes and validates the candidate catalog.
4. Writes the generated catalog.
5. Switches `model_catalog_json` to the Manager-owned relative pointer.
6. Commits the catalog state and any confirmed global-context cleanup.

If any step fails, the live config, old pointer, last valid generated file, and prior state generation remain active. The source external catalog is never edited or deleted.

The UI also offers an explicit `Discard external catalog and start empty` path. It is visually secondary, describes the consequence, and requires confirmation.

## Validation

Before saving, the frontend provides fast feedback for:

- Valid JSON syntax.
- A top-level JSON object for each model block.
- A non-empty string `slug`.
- Duplicate user slugs.
- Obviously invalid known numeric fields, including non-positive context windows and percentages outside their supported range.
- A configured default model that would be absent from the effective catalog.

The backend repeats all trust-boundary validation and additionally:

- Rejects forbidden credential fields in persisted catalog state.
- Composes the final catalog deterministically.
- Validates catalog structure.
- Runs the existing target Codex CLI offline validation.
- Uses the process-wide coordinator and fail-closed Context transaction for active config writes.

The Manager treats unknown model fields as opaque JSON. It neither rejects nor rewrites them merely because the current application version does not recognize them.

## Error Handling

- Invalid block: show the local error and keep `Save catalog` disabled.
- Duplicate slug: mark every conflicting block and do not materialize.
- External source changed after preview: reject commit and require a fresh preview.
- Target CLI or version status changed after preview: reject commit and require a fresh preview.
- Generated catalog fails offline validation: retain the prior effective catalog and show the backend validation error.
- Transaction failure: use existing full-generation rollback and surface the required recovery action.
- Official refresh failure: retain the previous official snapshot and all custom JSON blocks.

## Migration of Existing Manager State

The catalog state version is incremented. Existing reduced custom models are converted once into complete model objects using the last valid Manager-generated catalog when available. This preserves richer materialized data that the reduced state could not represent.

Migration source priority is:

1. A valid Manager-owned generated catalog whose hash matches state.
2. A still-resolvable external catalog recorded by the profile state.
3. A deterministic conversion of the reduced custom model state as a compatibility fallback.

The fallback is clearly marked in state provenance so the UI can warn that the original rich object was unavailable. Migration is fail-closed: if a trusted richer source is expected but invalid or mismatched, the Manager retains the prior mode and requires user action instead of silently degrading to a template.

Existing official override entries are migrated into complete JSON replacement blocks by applying them to the matching official model from the state snapshot. If no matching official snapshot exists, the profile remains on its last valid generation and reports an actionable migration error.

## Testing

### Backend unit tests

- Import a representative 372k model containing instructions, model messages, tiers, nested fields, and unknown future fields; assert semantic object equality after adoption and materialization.
- Compose `official-plus-custom` and assert a same-slug user object replaces the complete official object exactly once.
- Compose `custom-only` and assert no official model is present.
- Refresh the official baseline and assert all user objects remain unchanged.
- Reject invalid objects, empty slugs, duplicate slugs, invalid known numeric fields, and a missing default model.
- Migrate each state source-priority case and assert no silent rich-to-template downgrade.

### Frontend unit tests

- Parse and summarize valid model JSON without mutating it.
- Report JSON syntax and required-field errors.
- Detect duplicate and official-collision slugs.
- Disable save while any block is invalid.
- Preserve editor text across collapse and expand.

### Transaction and regression tests

- Adoption failure leaves the external pointer and source file untouched.
- Validation failure produces no config, catalog, or state mutation.
- Successful adoption writes a backup and switches the pointer only after validated materialization.
- Context tables survive managed catalog saves byte-for-byte under the existing protection contract.
- Live `auth.json` remains byte-for-byte unchanged.
- Inactive-profile failures retain their last valid generated catalog.

## Acceptance Criteria

- A user can import the existing `models_372k.json`, inspect `gpt-5.6-sol` as one complete JSON block, save it, switch providers, and refresh official models without losing or changing any JSON field or value.
- The wide custom-model field grid is removed.
- Both managed modes use the same JSON-block editor.
- A managed-mode transition cannot bypass adoption of an existing external catalog without an explicit discard confirmation.
- Invalid input never replaces the last valid catalog or active pointer.
- Existing Context, OAuth, catalog ownership, and transaction invariants continue to pass their regression tests.
