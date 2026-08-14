# Tasks — add-known-relay-model-presets

## 1. Preset data

- [ ] 1.1 Author `src/known-relay-models.ts` with the four Claude cards copied field-for-field from the owner's validated catalog (D2), and a contract test that pins every field of every card so a revision is a reviewed diff.

## 2. Backend description field

- [ ] 2.1 Add serde-defaulted `description: String` to `CustomModel`; write a non-empty description into the composed entry; a compose test proves a preset-shaped row (1M context, four levels, default `medium`, 95%, description) round-trips into the generated catalog, and old persisted state without the field still loads.

## 3. Editor

- [ ] 3.1 Offer preset chips in the add strip for every preset slug not already a row, deduplicated against provider-reported candidates; clicking either form of the slug prefills the full card. Tests cover offer/dedup/already-present.
- [ ] 3.2 Give custom rows a display-name input with follow-until-touched semantics (D5); slug edits keep an edited display name, hand-typed rows keep the follow behavior; startup-selection rename behavior is preserved. Tests cover both halves.
- [ ] 3.3 i18n entries for any new strings; `npm run verify`, Rust suites, and `cargo fmt --check` green.

## 4. Verification

- [ ] 4.1 On-screen: add Fable 5 from the strip on the fleet profile, save, confirm the generated catalog carries the card (description, levels, 95%), select it as startup model, restart Codex, and see "Fable 5" in the picker.
