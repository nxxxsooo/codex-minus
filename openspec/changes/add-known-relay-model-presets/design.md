# Design: add-known-relay-model-presets

## D1 — Cards over editors

The gap is authoring, not representation: `CustomModel` and `compose_profile_catalog` already carry and materialize display name, context, effective percent, and reasoning levels. The rejected alternative — expanding each row into a full metadata editor — restores exactly the surface 4.1/4.3 removed and asks the user to know answers (reasoning ladders, effective percents) that are facts about the model, not preferences. A preset card treats them like the bundled baseline treats official metadata: verified once, shipped as data, reviewed in a diff when they change.

## D2 — The preset data is the owner's validated production data

The four entries are copied field-for-field from the hand-authored catalog the fleet actually ran (`models_372k_claude.json`, template-identical to the server cache where the two overlap):

| slug | display | context | eff% | levels | default |
| --- | --- | --- | --- | --- | --- |
| `claude-fable-5` | Fable 5 | 1,000,000 | 95 | low/medium/high/xhigh | medium |
| `claude-opus-5` | Opus 5 | 1,000,000 | 95 | low/medium/high/xhigh | medium |
| `claude-sonnet-5` | Sonnet 5 | 1,000,000 | 95 | low/medium/high/xhigh | medium |
| `claude-haiku-4-5-20251001` | Haiku 4.5 | 200,000 | 95 | low/high | low |

Each entry also carries its one-line description ("… via the Sub2API Responses bridge."). A contract test pins every value, so a preset revision is a deliberate reviewed diff — the same discipline the bundled-baseline guard imposes on the Pro list.

## D3 — Prefill, not reference

Adding a preset copies the card into the profile's overlay draft; the saved profile owns its copy. The alternative — rows that reference the preset table and re-resolve on load — would make an app update silently rewrite the catalog of a profile the user already saved, which is the exact behavior the bundled-baseline continuity scenario (3.5) exists to prevent. Consequence accepted: a preset fix does not propagate to existing profiles; the restore-style repair for that can be added if it is ever needed.

## D4 — Presets live in the frontend

The overlay draft is authored in the editor and sent whole in the commit request; the backend validates and composes what it is sent. Preset data therefore belongs beside the other editor-owned lists (`PRO_MODEL_SLUGS`) in a frontend module the node test runner can import. The backend's only change is carrying `description` — a serde-defaulted field on `CustomModel` written into the composed entry — because that is the one card field the schema could not yet express.

## D5 — Display name follows the slug until touched

Today `renameCustom` syncs `displayName = slug` on every slug edit, which is right for a hand-typed row and wrong the moment a display name exists ("Fable 5" must survive a slug correction). Rule: the display name auto-follows the slug exactly while `displayName === slug`-as-it-was (the untouched state), and stops following once the user edits it independently. No extra state field; the comparison against the pre-edit slug decides.

## D6 — Offer rules

A preset chip is offered while its slug is neither an official row nor a custom row, in the same strip as provider-reported candidates, deduplicated slug-wise; a provider-reported candidate that matches a preset uses the card (the report proves reachability, the card supplies the metadata). Startup-model selection, removal, and the empty-catalog guard treat the row identically to any custom row — no new states.
