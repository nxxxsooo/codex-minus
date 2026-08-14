# Proposal: add-known-relay-model-presets

## Why

The fleet's Sub2API composite group serves four Claude models (Fable 5, Opus 5, Sonnet 5, Haiku 4.5) through the same Responses Base URL and key an official-mixed provider already uses, and the owner has run Codex against them by hand-authoring an external catalog file. The manager's backend can already express that catalog — `official-plus-custom` composes custom rows with display name, context, effective percent, and reasoning levels over the official baseline — but the simplified editor cannot author it: a custom row edits only slug and context, the display name is forcibly synced to the slug, and reasoning metadata has no entry point at all. Re-opening a full per-row metadata editor would undo the foolproofing that removed it.

## What Changes

- Ship a reviewed preset table of known relay-served models — the four Claude entries with the exact metadata the owner validated in production (slug, display name, description, context window, effective percent, reasoning levels with descriptions, default level).
- Offer every preset whose slug is not already a row in the editor's add-model strip, beside provider-reported candidates; a provider-reported candidate whose slug matches a preset uses the preset card.
- Adding a preset creates a custom row carrying the whole card, not template defaults. The row is a startup-model candidate like any other.
- Custom rows gain an editable display name that follows the slug only until the user edits it independently. No other per-row metadata editing is added; everything else rides on the card.
- `CustomModel` gains an optional `description` that composition writes into the generated entry, so a preset row reads in Codex's picker the way the hand-authored file did. Old persisted state loads unchanged.
- Preset metadata prefills the draft and stops there: a later preset revision never silently rewrites an already-saved profile.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `model-catalog-management`: known relay models are offered as complete cards; custom rows carry an editable display name and an optional description.

## Impact

- `src/known-relay-models.ts` (new preset table + contract test), the model table and add strip in `src/App.tsx`, `src/model-catalog-ui.ts` helpers and tests.
- `src-tauri/src/model_catalog.rs`: `CustomModel.description` (serde-defaulted, backward compatible) and its composition; a compose test for a preset-shaped row.
- No provider-contract, staging, auth, or transaction changes. The stale `support-server-side-composite-catalogs` draft overlaps this ground (overlay metadata expansion) but predates the simplification and is not revived by this change; its reconciliation stays with the simplify workstream's archive step.
