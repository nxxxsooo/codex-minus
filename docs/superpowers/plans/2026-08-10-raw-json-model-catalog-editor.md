# Raw JSON Model Catalog Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace lossy field-by-field catalog overlays with complete raw JSON model blocks, preserve existing external catalogs during adoption, and provide a compact JSON editor for both managed modes.

**Architecture:** A focused Rust module owns raw model-object validation, target-compatible template creation, and deterministic whole-object composition. `model_catalog.rs` continues to own state, target verification, transactions, and external adoption, while a focused TypeScript module owns editor drafts and fast validation; `App.tsx` only renders cards and orchestrates Tauri commands.

**Tech Stack:** Rust, Serde/serde_json, Tauri 2, React 19, TypeScript, Node test runner, existing CSS and component primitives. Add no editor or JSON-schema dependency.

## Global Constraints

- A custom model's complete JSON object is authoritative; field order and whitespace may normalize, but field names, presence, and values must remain semantically identical unless the user edits them.
- In `official-plus-custom`, a same-slug user object replaces the entire official object exactly once; in `custom-only`, no official model is emitted.
- Official refresh never rewrites user JSON objects.
- External catalog files remain read-only and their live pointer remains active until a validated adoption or explicit discard transaction commits.
- Active `config.toml` writes must continue through the process-wide coordinator and fail-closed Context transaction; `[mcp_servers]`, `[skills]`, and `[plugins]` remain byte-for-byte protected.
- Provider profiles never persist, apply, or restore `authContents`; live `auth.json` remains byte-for-byte unchanged.
- Generated catalogs must pass structure checks and target Codex CLI offline validation before the active pointer changes.
- Provider `/v1/models` remains evidence and candidate discovery only.
- The Agentic Skill + Pi assistant stays outside this plan; its boundary is documented in `docs/superpowers/specs/2026-08-10-agentic-model-catalog-assistant-exploration.md`.
- The worktree already contains unrelated user changes. Before each commit, inspect `git diff` and use explicit paths plus `git add -p` where a file contains mixed work; never stage unrelated hunks.

## File Map

- Create `src-tauri/src/model_catalog_json.rs`: raw custom model type, model inspection, validation, import, template creation, and deterministic composition.
- Modify `src-tauri/src/lib.rs`: register the new module and template command.
- Modify `src-tauri/src/model_catalog.rs`: state v3 migration, raw overlay payloads, adoption, status, materialization, and transaction tests.
- Create `src/model-catalog-json.ts`: editor draft parsing, summaries, import parsing, semantic validation, and serialization.
- Create `src/model-catalog-json.test.ts`: frontend behavior tests for raw JSON drafts.
- Modify `src/model-catalog-ui.ts` and `src/model-catalog-ui.test.ts`: managed-mode transition decisions and existing catalog UI helpers.
- Modify `src/App.tsx`: raw JSON draft lifecycle, cards, import, template request, external adoption gate, and save orchestration.
- Modify `src/styles.css`: compact summary cards and JSON editor layout.
- Modify `src/i18n-en.ts`: new catalog editor, validation, adoption, and discard copy.
- Modify `README.md`: describe complete JSON ownership and safe adoption.
- Modify `BOARD.md`: append the completed change and exact verification evidence only after all gates pass.

---

### Task 1: Raw JSON Model Domain

**Files:**
- Create: `src-tauri/src/model_catalog_json.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/model_catalog_json.rs` (`#[cfg(test)]` module)

**Interfaces:**
- Produces: `RawCustomModel { model: Value, provenance: String }`.
- Produces: `ModelIdentity { slug: String, display_name: Option<String>, context_window: Option<u64> }`.
- Produces: `validate_custom_models(&[RawCustomModel]) -> anyhow::Result<Vec<ModelIdentity>>`.
- Produces: `import_catalog_models(&Value, &str) -> anyhow::Result<Vec<RawCustomModel>>`.
- Produces: `compose_catalog(Option<&Value>, &[RawCustomModel], Option<&str>) -> anyhow::Result<Value>`.
- Produces: `new_custom_model_template_blocking(String) -> CommandResult<RawCustomModel>` through the Tauri command `new_custom_model_template`.
- Consumes: `codex_plus_core::model_suffix::build_model_catalog_json` only to create a new explicit template; imported and edited objects never pass through it.

- [ ] **Step 1: Write failing behavior tests for semantic preservation and composition**

Add literal fixtures and tests inside the new module. The expected model object must be a hand-written literal, not output from the function under test:

```rust
fn rich_372_model() -> Value {
    json!({
        "slug": "gpt-5.6-sol",
        "display_name": "GPT-5.6-Sol",
        "context_window": 372000,
        "max_context_window": 372000,
        "effective_context_window_percent": 100,
        "base_instructions": "keep exactly",
        "model_messages": { "instructions_template": "keep nested" },
        "service_tiers": [{ "id": "priority", "name": "Fast" }],
        "future_field": { "nested": [1, true, "x"] }
    })
}

#[test]
fn import_and_compose_preserve_rich_custom_object_semantically() {
    let expected = rich_372_model();
    let imported = import_catalog_models(
        &json!({ "client_version": "0.147.0", "models": [expected.clone()] }),
        "external:/tmp/models_372k.json",
    ).unwrap();
    let output = compose_catalog(None, &imported, Some("gpt-5.6-sol")).unwrap();
    assert_eq!(output["models"][0], expected);
}

#[test]
fn same_slug_custom_replaces_the_complete_official_object_once() {
    let official = json!({
        "client_version": "0.147.0",
        "models": [
            { "slug": "gpt-5.6-sol", "display_name": "Official", "context_window": 272000, "official_only": true },
            { "slug": "gpt-5.6-terra", "display_name": "Terra", "context_window": 272000 }
        ]
    });
    let expected = rich_372_model();
    let custom = vec![RawCustomModel { model: expected.clone(), provenance: "user".into() }];
    let output = compose_catalog(Some(&official), &custom, Some("gpt-5.6-sol")).unwrap();
    let models = output["models"].as_array().unwrap();
    assert_eq!(models.iter().filter(|model| model["slug"] == "gpt-5.6-sol").count(), 1);
    assert_eq!(models[0], expected);
    assert_eq!(models[1]["slug"], "gpt-5.6-terra");
}
```

Also add separate tests that reject a non-object model, missing/blank slug, duplicate custom slugs, zero context windows, and an effective percentage outside `1..=100`.

- [ ] **Step 2: Run the focused Rust tests and verify RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml model_catalog_json -- --nocapture
```

Expected: compilation fails because `model_catalog_json` and its exported types/functions do not exist.

- [ ] **Step 3: Implement the raw domain and template command**

Create the types and validate known fields without rejecting unknown fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RawCustomModel {
    pub model: Value,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelIdentity {
    pub slug: String,
    pub display_name: Option<String>,
    pub context_window: Option<u64>,
}

pub fn inspect_model(model: &Value) -> anyhow::Result<ModelIdentity> {
    let object = model.as_object().context("model must be a JSON object")?;
    let slug = object.get("slug").and_then(Value::as_str).unwrap_or_default().trim();
    ensure!(!slug.is_empty(), "model slug must not be empty");
    for key in ["context_window", "max_context_window"] {
        if let Some(value) = object.get(key) {
            ensure!(value.as_u64().is_some_and(|number| number > 0), "{key} must be a positive integer");
        }
    }
    if let Some(value) = object.get("effective_context_window_percent") {
        ensure!(value.as_u64().is_some_and(|number| (1..=100).contains(&number)),
            "effective_context_window_percent must be 1-100");
    }
    Ok(ModelIdentity {
        slug: slug.to_string(),
        display_name: object.get("display_name").and_then(Value::as_str).map(ToString::to_string),
        context_window: object.get("context_window").and_then(Value::as_u64),
    })
}
```

Implement composition by cloning the official wrapper, indexing its model array by slug, replacing matching indices with `record.model.clone()`, and appending non-matching user objects in user order. Validate the default model against the final slug set.

Implement `template_model(slug)` by calling the pinned core builder for exactly one entry, extracting the single complete generated model object, and wrapping it in `RawCustomModel` with provenance `manager-template`. Register an async `new_custom_model_template` Tauri command in `src-tauri/src/lib.rs`; do not add network or auth access.

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml model_catalog_json -- --nocapture
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Expected: all new module tests pass and formatting is clean.

- [ ] **Step 5: Commit only the raw-domain slice**

```bash
git add src-tauri/src/model_catalog_json.rs
git add -p src-tauri/src/lib.rs
git diff --cached --check
git commit -m "feat: add raw model catalog domain"
```

### Task 2: State v3 Migration Without Rich-to-Template Loss

**Files:**
- Modify: `src-tauri/src/model_catalog.rs`
- Test: `src-tauri/src/model_catalog.rs`

**Interfaces:**
- Consumes: `RawCustomModel`, `inspect_model`, and `template_model` from Task 1.
- Produces: `CatalogOverlay { custom: Vec<RawCustomModel> }` with no structured official override map.
- Produces: `migrate_state_document(value: &mut Value, home: &Path) -> anyhow::Result<()>` before typed deserialization.
- Produces: state version `3` serialized only in raw-model form.

- [ ] **Step 1: Write failing migration tests**

Add a v2 JSON fixture directly as `serde_json::Value`. Use a temporary home containing a hash-matched generated catalog whose model includes unknown rich fields:

```rust
#[test]
fn v2_migration_prefers_hash_matched_generated_model_object() {
    let home = tempfile::tempdir().unwrap();
    let generated = json!({ "models": [{
        "slug": "gpt-5.6-sol",
        "display_name": "GPT-5.6-Sol",
        "context_window": 372000,
        "base_instructions": "rich source",
        "future_field": { "kept": true }
    }] });
    let bytes = serde_json::to_vec_pretty(&generated).unwrap();
    let relative = "model-catalogs/codex-minus-p-hash.json";
    std::fs::create_dir_all(home.path().join("model-catalogs")).unwrap();
    std::fs::write(home.path().join(relative), &bytes).unwrap();
    let mut state = json!({
        "version": 2,
        "profiles": { "p": {
            "overlay": { "official": {}, "custom": [{
                "slug": "gpt-5.6-sol", "displayName": "GPT-5.6-Sol",
                "contextWindow": 372000, "effectiveContextWindowPercent": 100,
                "visible": true, "order": 0, "supportedReasoningLevels": [],
                "defaultReasoningLevel": null, "supportedTools": [],
                "toolCapabilities": null, "templateProvenance": "external"
            }]},
            "generatedPath": relative,
            "generatedHash": content_hash(&bytes)
        }}
    });
    migrate_state_document(&mut state, home.path()).unwrap();
    assert_eq!(state["version"], 3);
    assert_eq!(state["profiles"]["p"]["overlay"]["custom"][0]["model"], generated["models"][0]);
    assert_eq!(state["profiles"]["p"]["overlay"]["custom"][0]["provenance"], "state-v2-generated");
    assert!(state["profiles"]["p"]["overlay"].get("official").is_none());
}
```

Add independent tests for these breaks:

- A generated-file hash mismatch is not trusted.
- A v2 legacy custom record falls back to a complete core template plus its explicit legacy values and records provenance `state-v2-fallback`.
- A v2 official override is applied to the matching full official model and becomes a raw replacement block.
- An official override without a matching official snapshot leaves the previous generation/mode intact and sets `actionRequired` instead of silently dropping the override.
- A v3 state document is unchanged except for normal defaulting.

- [ ] **Step 2: Run migration tests and verify RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml v2_migration -- --nocapture
```

Expected: tests fail because state version 3 and document migration do not exist.

- [ ] **Step 3: Implement document-first migration**

Change state loading from direct byte deserialization to explicit JSON migration:

```rust
let mut value: Value = serde_json::from_slice(&bytes)
    .context("model catalog state is invalid")?;
migrate_state_document(&mut value, home)?;
let mut state: CatalogState = serde_json::from_value(value)
    .context("migrated model catalog state is invalid")?;
```

Set `STATE_VERSION` to `3`, replace the overlay types, and keep the old structured structs private under `LegacyCustomModel` and `LegacyOfficialOverride` only for migration.

For each v2 profile, resolve richer data in this order:

1. Read `generatedPath` only when it resolves under the supplied Codex home and its bytes match `generatedHash`.
2. If `externalPointer` still exists, resolve it through the existing safe pointer resolver and read it without mutation.
3. Build a complete fallback object from `template_model(slug)` and apply every explicit v2 field.

Convert the union of legacy custom slugs and official override slugs into `RawCustomModel` records. Remove `overlay.official`, serialize only the v3 shape, and preserve `generatedPath`, `generatedHash`, generation, and last-valid action state.

- [ ] **Step 4: Run migration and existing catalog tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml v2_migration -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml model_catalog -- --nocapture
```

Expected: migration tests pass; existing model-catalog tests either pass or have been updated only where their expected payload shape intentionally changed.

- [ ] **Step 5: Commit the state migration slice**

```bash
git add -p src-tauri/src/model_catalog.rs
git diff --cached --check
git commit -m "feat: migrate catalog state to raw model JSON"
```

### Task 3: Raw Adoption, Materialization, and Transaction Safety

**Files:**
- Modify: `src-tauri/src/model_catalog.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/model_catalog.rs`

**Interfaces:**
- Consumes: Task 1 composition and Task 2 state shape.
- Changes: `AdoptCatalogRequest` gains `target_mode: Option<CatalogMode>` serialized as `targetMode`.
- Changes: `AdoptionPreviewPayload.custom_models` becomes `Vec<RawCustomModel>`; `official_override_count` counts imported slugs colliding with the official baseline.
- Produces: adoption commit may target only `official-plus-custom` or `custom-only`.

- [ ] **Step 1: Write failing backend adoption and materialization tests**

Add tests using a rich external catalog and literal expected objects:

```rust
#[test]
fn raw_external_adoption_preserves_every_model_object() {
    let raw = json!({ "models": [
        { "slug": "official-a", "context_window": 372000, "future": { "kept": 1 } },
        { "slug": "custom-b", "context_window": 128000, "instructions": "keep" }
    ]});
    let official = OfficialSnapshot { raw_catalog: official_catalog(), ..OfficialSnapshot::default() };
    let (models, collisions) = raw_overlay_from_catalog(Some(&official), &raw, "external:test").unwrap();
    assert_eq!(models[0].model, raw["models"][0]);
    assert_eq!(models[1].model, raw["models"][1]);
    assert_eq!(collisions, vec!["official-a"]);
}
```

Add transaction-level tests for:

- `targetMode = custom-only` produces no official model after adoption.
- A commit target of `native-official` or `external` is rejected.
- Invalid JSON/model structure fails before any mutation is created.
- Offline validation failure retains the external pointer and source bytes.
- Successful adoption creates the non-secret backup and switches the pointer only in the same transaction as the generated file and state.
- `auth.json` is absent from every mutation.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml raw_external_adoption -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml adoption_target_mode -- --nocapture
```

Expected: failures show the old reduced overlay conversion and missing target mode.

- [ ] **Step 3: Replace lossy overlay paths with raw composition**

Delete the structured `apply_official_override`, conservative rebuild path, and `overlay_from_catalog` field projection. Replace them with:

```rust
fn raw_overlay_from_catalog(
    official: Option<&OfficialSnapshot>,
    catalog: &Value,
    provenance: &str,
) -> anyhow::Result<(Vec<RawCustomModel>, Vec<String>)> {
    let models = model_catalog_json::import_catalog_models(catalog, provenance)?;
    let official_slugs = official
        .map(|snapshot| catalog_slugs(&snapshot.raw_catalog))
        .transpose()?
        .unwrap_or_default();
    let collisions = models.iter()
        .filter_map(|record| inspect_model(&record.model).ok())
        .filter(|identity| official_slugs.contains(&identity.slug))
        .map(|identity| identity.slug)
        .collect();
    Ok((models, collisions))
}
```

In `compose_profile_catalog`, pass the official raw catalog only for `OfficialPlusCustom`, pass `None` for `CustomOnly`, and pass `profile_default_model(profile)` into Task 1's composer. Keep existing structure validation, offline CLI validation, generated hash, restart flag, and live-state transaction ordering unchanged.

On adoption commit, require `target_mode` to be one of the two managed modes, persist the complete raw records, materialize, validate, back up, and switch the pointer in the existing single transaction.

- [ ] **Step 4: Run catalog, live-state, Context, and auth regression tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml raw_external_adoption -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml adoption_target_mode -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml context_transaction_preserves_unrelated_root_settings -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml raw_auth_save_is_rejected -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml inactive_materialization_failure_keeps_last_valid_generation -- --nocapture
```

Expected: all pass; no test weakens an ownership or rollback assertion.

- [ ] **Step 5: Commit the backend integration slice**

```bash
git add -p src-tauri/src/model_catalog.rs src-tauri/src/lib.rs
git diff --cached --check
git commit -m "feat: preserve raw JSON through catalog adoption"
```

### Task 4: Frontend Raw JSON Draft Model

**Files:**
- Create: `src/model-catalog-json.ts`
- Create: `src/model-catalog-json.test.ts`
- Modify: `src/model-catalog-ui.ts`
- Modify: `src/model-catalog-ui.test.ts`

**Interfaces:**
- Produces: `RawCustomModel`, `ModelJsonDraft`, `ModelDraftSummary`, and `ModelDraftIssue` TypeScript types matching Rust camel-case payloads.
- Produces: `draftFromStoredModel(record, id)`, `parseModelDraft(draft)`, `validateModelDrafts(...)`, `serializeValidDrafts(...)`, `parseCatalogImport(text, idFactory)`.
- Produces: `catalogModeTransition(currentMode, requestedMode, externalPointer)` returning `"select" | "adopt-external" | "confirm-discard-external"`.
- Consumes: no React or Tauri API, keeping all behavior runnable by Node tests.

- [ ] **Step 1: Write failing Node tests for parsing, semantic preservation, and transition decisions**

Create `src/model-catalog-json.test.ts` with hand-written JSON text containing nested and unknown fields:

```typescript
it("round-trips a rich 372k model without projecting fields", () => {
  const text = JSON.stringify({
    slug: "gpt-5.6-sol",
    display_name: "GPT-5.6-Sol",
    context_window: 372000,
    max_context_window: 372000,
    base_instructions: "keep exactly",
    model_messages: { instructions_template: "nested" },
    service_tiers: [{ id: "priority" }],
    future_field: { kept: true },
  }, null, 2);
  const draft = { id: "one", text, provenance: "external" };
  const parsed = parseModelDraft(draft);
  assert.equal(parsed.issue, null);
  assert.deepEqual(serializeValidDrafts([draft])[0].model, JSON.parse(text));
  assert.deepEqual(parsed.summary, {
    slug: "gpt-5.6-sol",
    displayName: "GPT-5.6-Sol",
    contextWindow: 372000,
  });
});

it("imports every models array object and rejects a non-catalog", () => {
  const drafts = parseCatalogImport('{"models":[{"slug":"a"},{"slug":"b","future":1}]}', (() => {
    let index = 0;
    return () => `id-${++index}`;
  })());
  assert.deepEqual(drafts.map((draft) => parseModelDraft(draft).summary?.slug), ["a", "b"]);
  assert.throws(() => parseCatalogImport('{"items":[]}', () => "id"), /models/);
});
```

Add independent tests for syntax line errors, non-object JSON, blank slug, duplicate slugs marking both drafts, invalid numeric fields, missing default model, official collision summary, custom-only effective slugs, and external-to-managed transition requiring adoption.

- [ ] **Step 2: Run focused Node tests and verify RED**

Run:

```bash
node --test --experimental-strip-types src/model-catalog-json.test.ts src/model-catalog-ui.test.ts
```

Expected: module-not-found or missing-export failures.

- [ ] **Step 3: Implement pure draft helpers and simplify catalog overlay types**

Use editor text as the draft source of truth and parse only for summary, validation, and save:

```typescript
export type RawCustomModel = {
  model: Record<string, unknown>;
  provenance: string;
};

export type ModelJsonDraft = {
  id: string;
  text: string;
  provenance: string;
};

export function parseModelDraft(draft: ModelJsonDraft): {
  record: RawCustomModel | null;
  summary: ModelDraftSummary | null;
  issue: ModelDraftIssue | null;
} {
  try {
    const value: unknown = JSON.parse(draft.text);
    if (!value || Array.isArray(value) || typeof value !== "object") {
      return { record: null, summary: null, issue: { draftId: draft.id, code: "model-not-object", message: "Model JSON must be an object." } };
    }
    const model = value as Record<string, unknown>;
    const slug = typeof model.slug === "string" ? model.slug.trim() : "";
    if (!slug) return { record: null, summary: null, issue: { draftId: draft.id, code: "empty-slug", message: "slug must be a non-empty string." } };
    return {
      record: { model, provenance: draft.provenance },
      summary: {
        slug,
        displayName: typeof model.display_name === "string" ? model.display_name : null,
        contextWindow: typeof model.context_window === "number" ? model.context_window : null,
      },
      issue: validateKnownModelNumbers(draft.id, model),
    };
  } catch (error) {
    return { record: null, summary: null, issue: jsonSyntaxIssue(draft.id, error) };
  }
}
```

`serializeValidDrafts` must throw when any issue exists; it must return the parsed objects directly without rebuilding their fields. `parseCatalogImport` must preserve every parsed model object semantically and create one draft per array entry.

Remove the structured `CatalogOverlayDraft.official` and structured custom fields from `src/model-catalog-ui.ts`. Keep provider evidence, refresh, Context conflict, and diff helpers unchanged.

- [ ] **Step 4: Run frontend unit tests and TypeScript**

Run:

```bash
node --test --experimental-strip-types src/model-catalog-json.test.ts src/model-catalog-ui.test.ts
npm run check
```

Expected: all focused tests and TypeScript pass.

- [ ] **Step 5: Commit the frontend domain slice**

```bash
git add src/model-catalog-json.ts src/model-catalog-json.test.ts
git add -p src/model-catalog-ui.ts src/model-catalog-ui.test.ts
git diff --cached --check
git commit -m "feat: add raw catalog JSON draft model"
```

### Task 5: Compact JSON Block Editor

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/styles.css`
- Modify: `src/i18n-en.ts`
- Test: `src/model-catalog-json.test.ts`

**Interfaces:**
- Consumes: Task 4 draft helpers and Task 1 Tauri command `new_custom_model_template`.
- Changes: frontend `CatalogOverlay` payload contains only `custom: RawCustomModel[]`.
- Produces: collapsed official summary, custom summary cards, expandable JSON textareas, format/duplicate/delete, template creation, and catalog-file import.

- [ ] **Step 1: Add failing helper tests for editor actions**

Before touching JSX, add pure helper tests for the state changes the UI will call:

```typescript
it("formats and duplicates without changing model semantics", () => {
  const source = { id: "a", text: '{"slug":"gpt-5.6-sol","future":{"x":1}}', provenance: "user" };
  const formatted = formatModelDraft(source);
  assert.deepEqual(JSON.parse(formatted.text), JSON.parse(source.text));
  const duplicate = duplicateModelDraft(formatted, "b");
  assert.equal(duplicate.id, "b");
  assert.deepEqual(JSON.parse(duplicate.text), JSON.parse(formatted.text));
});
```

Add a test that formatting an invalid draft returns the original text plus the existing syntax issue instead of deleting content.

- [ ] **Step 2: Run the helper test and verify RED**

Run:

```bash
node --test --experimental-strip-types src/model-catalog-json.test.ts
```

Expected: missing `formatModelDraft` and `duplicateModelDraft` exports.

- [ ] **Step 3: Implement editor state and replace the field grid**

In `CatalogProfileEditor`, keep `drafts: ModelJsonDraft[]` separate from the last saved backend overlay. Reinitialize drafts only when `profile.id` or the backend generation changes; do not overwrite unsaved drafts on unrelated React renders.

Render official models read-only inside a collapsed `<details>` summary. Render each custom model as a card:

```tsx
<details className={`catalog-json-card ${issue ? "invalid" : ""}`} key={draft.id}>
  <summary>
    <span><strong>{summary?.displayName || summary?.slug || t("无效 JSON")}</strong><small>{summary?.slug || issue?.message}</small></span>
    <span className="catalog-json-meta">{summary?.contextWindow ? tf("{0} 上下文", [summary.contextWindow]) : t("未声明上下文")}</span>
    {summary && officialSlugSet.has(summary.slug) ? <UiBadge variant="secondary">{t("覆盖官方")}</UiBadge> : null}
  </summary>
  <Textarea
    className="catalog-json-editor"
    spellCheck={false}
    value={draft.text}
    onChange={(event) => updateDraftText(draft.id, event.currentTarget.value)}
  />
  {issue ? <div className="catalog-inline-error">{issue.message}</div> : null}
  <div className="catalog-json-actions">{/* Format, Duplicate, Delete */}</div>
</details>
```

`Add JSON model` calls `new_custom_model_template` with a collision-free draft slug such as `model-id`, converts the returned complete model object to formatted editor text, and appends it as an unsaved draft.

`Import catalog JSON` uses a hidden `<input type="file" accept="application/json,.json">`, reads `await file.text()`, parses through Task 4, and appends every imported block. If current drafts are non-empty, confirm the append and explain that duplicate slugs must be resolved before saving. Never pass a filesystem path to the backend.

On save, call `serializeValidDrafts`, build `{ custom: records }`, and send only parsed complete model objects. Disable save while `validateModelDrafts` reports any issue.

Add CSS for a single-column card layout, JetBrains Mono editor text, minimum 16 rows while expanded, responsive actions, invalid borders, and readable summaries at the existing 960px minimum window. Remove obsolete `.catalog-model-row` and `.catalog-custom-row` grid rules only after no JSX uses them.

Add every new Chinese string and deterministic error copy to `src/i18n-en.ts`.

- [ ] **Step 4: Verify frontend behavior, type checking, and production build**

Run:

```bash
npm test
npm run check
npm run vite:build
```

Expected: all tests pass, TypeScript is clean, and Vite builds without warnings introduced by this change.

- [ ] **Step 5: Commit the editor UI slice**

```bash
git add -p src/App.tsx src/styles.css src/i18n-en.ts src/model-catalog-json.test.ts
git diff --cached --check
git commit -m "feat: replace catalog field grid with JSON blocks"
```

### Task 6: External-to-Managed Adoption Gate

**Files:**
- Modify: `src/model-catalog-ui.ts`
- Modify: `src/model-catalog-ui.test.ts`
- Modify: `src/App.tsx`
- Modify: `src/i18n-en.ts`
- Test: `src/model-catalog-ui.test.ts`

**Interfaces:**
- Consumes: backend `targetMode` adoption contract from Task 3.
- Produces: `catalogModeTransition(currentMode, requestedMode, externalPointer)`.
- Produces: one UI path for `Preview and adopt`, and a separate confirmed `Discard external catalog and start empty` path.

- [ ] **Step 1: Write failing transition tests**

Add literal decision tests:

```typescript
it("never selects a managed mode directly over an external pointer", () => {
  assert.equal(catalogModeTransition("external", "official-plus-custom", "models_372k.json"), "adopt-external");
  assert.equal(catalogModeTransition("external", "custom-only", "models_372k.json"), "adopt-external");
  assert.equal(catalogModeTransition("external", "native-official", "models_372k.json"), "confirm-discard-external");
  assert.equal(catalogModeTransition("custom-only", "official-plus-custom", null), "select");
});
```

The production change this catches is a mode button assigning managed state directly while an external pointer still owns the catalog.

- [ ] **Step 2: Run the transition test and verify RED**

Run:

```bash
node --test --experimental-strip-types src/model-catalog-ui.test.ts
```

Expected: missing `catalogModeTransition` export.

- [ ] **Step 3: Implement the adoption gate and explicit discard**

Route managed mode buttons through one handler:

```typescript
const requestMode = async (requested: CatalogMode) => {
  const transition = catalogModeTransition(mode, requested, summary?.externalPointer);
  if (transition === "adopt-external") {
    await adopt(requested);
    return;
  }
  if (transition === "confirm-discard-external" && !window.confirm(
    tf("放弃外部目录 {0} 并切换模式？保存成功前旧目录仍保持有效。", [summary?.externalPointer || ""]),
  )) return;
  setMode(requested);
  setModeExplicit(true);
};
```

Change `adopt(targetMode)` to preview the unchanged external source, show source path/model count/official collisions/version state, and commit with `targetMode`. On success, reload backend status and drafts from the committed raw records.

Add a secondary `Discard external catalog and start empty` action only in external mode. It must show the external path and require confirmation; confirmation prepares an empty managed draft locally, but the pointer remains live until the user presses `Save catalog` and the backend transaction succeeds. Selecting `native-official` from external mode uses the same confirmed-discard boundary and does not clear the live pointer before save.

Do not allow a segmented-control click to clear `externalPointer` in frontend state. Keep preview hash, target version, mismatch acceptance, and Context cleanup confirmations in the existing commit request.

- [ ] **Step 4: Run transition, frontend, and backend adoption tests**

Run:

```bash
node --test --experimental-strip-types src/model-catalog-ui.test.ts src/model-catalog-json.test.ts
npm run check
cargo test --manifest-path src-tauri/Cargo.toml adoption_target_mode -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml raw_external_adoption -- --nocapture
```

Expected: all pass; managed selection over an external pointer cannot bypass preview/adoption.

- [ ] **Step 5: Commit the transition-safety slice**

```bash
git add -p src/model-catalog-ui.ts src/model-catalog-ui.test.ts src/App.tsx src/i18n-en.ts
git diff --cached --check
git commit -m "fix: gate managed catalogs behind external adoption"
```

### Task 7: Regression Evidence, Documentation, and Release-Grade Verification

**Files:**
- Modify: `README.md`
- Modify: `BOARD.md`

**Interfaces:**
- Consumes: all prior task outputs.
- Produces: recipient-facing behavior documentation and exact verification evidence.

- [ ] **Step 1: Update user-facing model catalog documentation**

Replace the README statement about structured overlays and conservative custom templates with exact behavior:

```markdown
- 「官方 + 自定义」和「仅自定义」以完整 JSON 模型对象为自定义数据源；未知字段、instructions、tiers 与嵌套能力不会被字段表单重建。与官方模型同 slug 的用户对象完整替换该官方对象。
- 检测到外部 `model_catalog_json` 时，切换托管模式必须先预览并采用，或明确确认放弃；生成目录通过离线校验并完成事务提交前，旧指针和源文件保持不变。
```

Do not mention the Agentic exploration as a shipped feature.

- [ ] **Step 2: Run formatting and complete automated suites**

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm test
npm run check
npm run vite:build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all commands pass. The intentionally ignored live OAuth test remains ignored and is reported as such; do not describe targeted tests as the full Rust suite.

- [ ] **Step 3: Build the real macOS application entry point**

Run:

```bash
npm run build
```

If the build lasts over 60 seconds, run it as a detached, traceable job with a log plus explicit done/error marker and poll no more frequently than the workspace long-job rules permit. Expected: the Tauri build produces `src-tauri/target/release/bundle/macos/Codex-- Manager.app`.

- [ ] **Step 4: Perform focused installed-app QA without modifying the user's source catalog**

Use a temporary catalog fixture, not `/Users/mingjian/.codex/models_372k.json`, for destructive-path tests. Verify:

1. The official section is collapsed and read-only.
2. A rich JSON model card shows slug/display/window summary and retains nested unknown fields after format, collapse, and expand.
3. Importing a catalog creates one card per model.
4. Invalid JSON and duplicate slugs disable save and leave the active pointer unchanged.
5. External-to-managed selection opens adoption preview instead of changing mode directly.
6. Successful adoption produces one generated catalog whose `gpt-5.6-sol` object is semantically equal to the fixture's 372k object.
7. Switching profiles and refreshing official models do not change the user object.
8. Restart-required state appears only after a new validated generation commits.

Compare fixture and generated objects with `jq -S` and `diff`; compare the live `auth.json` hash before and after. Restore the previous provider through the Manager UI if the QA changes it. Never edit or delete the user's original external catalog.

- [ ] **Step 5: Append exact completion evidence to BOARD.md**

Append this completed-work entry only after Steps 2-4 pass:

```markdown
- **feat/raw-json-model-catalogs**: replaced lossy structured overlays with complete JSON model blocks, added safe external adoption gating, and preserved whole-object official overrides
  - verified: the full Rust suite passes with the opt-in live OAuth test intentionally ignored; the full frontend suite, TypeScript, Vite production build, Rust formatting, macOS Tauri bundle build, and installed-app 372k semantic-preservation QA pass
  - refs: `docs/superpowers/specs/2026-08-10-raw-json-model-catalog-editor-design.md`, `src-tauri/src/model_catalog_json.rs`, `src-tauri/src/model_catalog.rs`, `src/model-catalog-json.ts`, `src/App.tsx`
```

If a gate does not pass, do not append or commit this completed-work entry.

- [ ] **Step 6: Commit documentation and any verification-only corrections**

```bash
git add -p README.md BOARD.md
git diff --cached --check
git commit -m "docs: document raw JSON model catalogs"
```

- [ ] **Step 7: Inspect final scope and report gaps**

Run:

```bash
git status --short
git log --oneline -8
git diff --check
```

Confirm every implementation commit contains only intended hunks. Report unrelated pre-existing worktree changes separately. If Windows installed-app validation was not performed, record it as not verified and retain the existing Windows risk statement; do not infer parity from macOS alone.
