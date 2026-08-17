# Reset Unowned Legacy Model Overlay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove only unowned `legacy-model-list` catalog state, restore the bundled official model list, and replace a removed legacy startup model such as `gpt-5` with `gpt-5.6-terra` without changing the mixed Sub2API provider contract or any user-owned catalog state.

**Architecture:** Add a pure backend planner in a focused module, then persist its settings/catalog/live-config generation through the existing process-wide coordinator and Context-protected transaction. Catalog state carries a one-time migration marker; the frontend receives one explicit success notice, while ordinary profiles remain a byte-identical no-op.

**Tech Stack:** Rust, `serde`, `serde_json`, `toml_edit`, Tauri 2, React 19, TypeScript, Codex Plus Core settings types, OpenSpec, Node test runner.

## Global Constraints

- Reset only ordinary Responses mixed profiles: `RelayMode::Official`, `official_mix_api_key = true`, protocol `Responses`.
- Remove only custom rows whose exact `template_provenance` is `legacy-model-list`; unknown, empty, preset, and `user-created` provenance is preserved.
- Preserve explicit catalog modes/overlays, external pointers/catalogs, pure OAuth, pure API, aggregate, and Chat Completions profiles.
- Catalog mode remains `official-plus-custom`; “official reset” changes model content, not provider/auth mode.
- Change the startup model only when it names a row removed by this migration; the replacement is exactly `gpt-5.6-terra`.
- Preserve provider ID, endpoint, bearer, actor header, unrelated provider fields, `requires_openai_auth`, MCP, skills, plugins, and live `auth.json` bytes.
- Clear consumed legacy `modelList` and `modelWindows` fields only after a successful plan.
- Affected settings, catalog state, generated catalog, and active live config commit as one owner-only generation; any failure rolls back all targets.
- Do not create recovery artifacts until the 0.4.14 auth scrub has removed copied OAuth residue.
- Empty/no-legacy profiles are byte-identical no-ops and gain no save/switch gate.
- Do not force-quit Codex, send a provider request, alter session history, or special-case Eva/profile names in production code.
- Use `apply_patch` for authored file changes; formatter-generated mechanical rewrites may use the repository formatter.

---

### Task 1: Pure Eva reset planner and one-time state marker

**Files:**
- Create: `src-tauri/src/legacy_model_reset.rs`
- Modify: `src-tauri/src/lib.rs:1-8`
- Modify: `src-tauri/src/model_catalog.rs:20, 175-208`
- Test: `src-tauri/src/legacy_model_reset.rs` (`#[cfg(test)]` module)

**Interfaces:**
- Consumes: `BackendSettings`, `RelayProfile`, `CatalogState`, `CatalogMode`, `ProfileCatalogState`, and bundled visible official slugs.
- Produces:

```rust
pub(crate) const LEGACY_MODEL_RESET_VERSION: u32 = 1;
pub(crate) const CANONICAL_MIXED_DEFAULT_MODEL: &str = "gpt-5.6-terra";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResetProfileSummary {
    pub profile_id: String,
    pub removed_slugs: Vec<String>,
    pub previous_model: String,
    pub next_model: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct LegacyModelResetPlan {
    pub settings: BackendSettings,
    pub state: CatalogState,
    pub reset_profiles: Vec<ResetProfileSummary>,
}

pub(crate) fn plan_legacy_model_reset(
    settings: &BackendSettings,
    state: &CatalogState,
    official_visible_slugs: &std::collections::BTreeSet<String>,
) -> anyhow::Result<Option<LegacyModelResetPlan>>;

pub(crate) fn top_level_model(config: &str) -> Option<String>;
pub(crate) fn set_top_level_model(config: &str, model: &str) -> anyhow::Result<String>;
```

- [ ] **Step 1: Add the Eva failing regression before the planner exists**

Add a test fixture with an official-mixed Responses profile whose TOML default and legacy list contain `gpt-5`, whose state is implicit, and whose custom row has exact legacy provenance:

```rust
#[test]
fn eva_implicit_legacy_gpt5_resets_to_official_terra() {
    let settings = eva_settings(
        "gpt-5",
        "gpt-5.6-terra\ngpt-5",
        r#"{"gpt-5":"272000"}"#,
    );
    let state = state_with_profile(
        "eva",
        CatalogMode::OfficialPlusCustom,
        false,
        vec![custom("gpt-5", "legacy-model-list")],
    );
    let official = std::collections::BTreeSet::from([
        "gpt-5.6-terra".to_string(),
        "gpt-5.6-luna".to_string(),
        "gpt-5.6-sol".to_string(),
        "gpt-5.5".to_string(),
    ]);

    let plan = plan_legacy_model_reset(&settings, &state, &official)
        .unwrap()
        .expect("Eva legacy state must produce a reset plan");

    let profile = &plan.settings.relay_profiles[0];
    assert_eq!(top_level_model(&profile.config_contents).as_deref(), Some("gpt-5.6-terra"));
    assert_eq!(profile.model, "gpt-5.6-terra");
    assert!(profile.model_list.is_empty());
    assert!(profile.model_windows.is_empty());
    assert!(plan.state.profiles["eva"].overlay.custom.is_empty());
    assert_eq!(plan.state.profiles["eva"].mode, CatalogMode::OfficialPlusCustom);
    assert_eq!(plan.reset_profiles[0].removed_slugs, vec!["gpt-5"]);
}
```

The fixture's provider TOML must also carry `model_provider = "OpenAI"`, endpoint, bearer, actor header, `requires_openai_auth = true`, and one unrelated custom field for later preservation assertions.

Use these concrete test helpers in the Task 1 test module:

```rust
fn custom(slug: &str, provenance: &str) -> CustomModel {
    CustomModel {
        slug: slug.to_string(),
        display_name: slug.to_string(),
        context_window: 272_000,
        effective_context_window_percent: 95,
        visible: true,
        template_provenance: provenance.to_string(),
        ..CustomModel::default()
    }
}

fn eva_settings(model: &str, model_list: &str, model_windows: &str) -> BackendSettings {
    let config_contents = format!(
        r#"model = "{model}"
model_provider = "OpenAI"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://example.test/v1"
wire_api = "responses"
requires_openai_auth = true
experimental_bearer_token = "provider-key-sentinel"
http_headers = {{ "x-openai-actor-authorization" = "local-image-extension" }}
fit_marker = "keep"
"#,
    );
    BackendSettings {
        relay_profiles_enabled: true,
        active_relay_id: "eva".to_string(),
        relay_profiles: vec![RelayProfile {
            id: "eva".to_string(),
            name: "Eva|Codex".to_string(),
            model: model.to_string(),
            relay_mode: RelayMode::Official,
            official_mix_api_key: true,
            protocol: RelayProtocol::Responses,
            config_contents,
            model_list: model_list.to_string(),
            model_windows: model_windows.to_string(),
            ..RelayProfile::default()
        }],
        ..BackendSettings::default()
    }
}

fn state_with_profile(
    id: &str,
    mode: CatalogMode,
    mode_explicit: bool,
    custom_models: Vec<CustomModel>,
) -> CatalogState {
    let mut state = CatalogState::default();
    state.official = Some(crate::model_catalog::bundled_official_snapshot().unwrap());
    state.profiles.insert(id.to_string(), ProfileCatalogState {
        mode,
        mode_explicit,
        overlay: CatalogOverlay {
            official: std::collections::BTreeMap::new(),
            custom: custom_models,
        },
        ..ProfileCatalogState::default()
    });
    state
}

fn eva_legacy_fixture() -> (BackendSettings, CatalogState, std::collections::BTreeSet<String>) {
    let settings = eva_settings("gpt-5", "gpt-5.6-terra\ngpt-5", r#"{"gpt-5":"272000"}"#);
    let state = state_with_profile(
        "eva",
        CatalogMode::OfficialPlusCustom,
        false,
        vec![custom("gpt-5", "legacy-model-list")],
    );
    let official = std::collections::BTreeSet::from([
        "gpt-5.6-terra".to_string(),
        "gpt-5.6-luna".to_string(),
        "gpt-5.6-sol".to_string(),
        "gpt-5.5".to_string(),
    ]);
    (settings, state, official)
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cd src-tauri
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test eva_implicit_legacy_gpt5_resets_to_official_terra --lib -- --nocapture
```

Expected: compilation/test failure because `legacy_model_reset`, `plan_legacy_model_reset`, and the state marker do not exist.

- [ ] **Step 3: Add the catalog-state marker and pure planner**

Register the module in `lib.rs`:

```rust
mod legacy_model_reset;
```

Bump `STATE_VERSION` from `3` to `4` and add this serde-defaulted field to `ProfileCatalogState`:

```rust
pub legacy_model_reset_version: u32,
```

Initialize it to `0` in `Default`.

Implement eligibility and exact-provenance removal in `legacy_model_reset.rs`:

```rust
fn ordinary_mixed_responses(profile: &RelayProfile) -> bool {
    profile.relay_mode == RelayMode::Official
        && profile.official_mix_api_key
        && profile.protocol == RelayProtocol::Responses
}

fn has_legacy_signal(profile: &RelayProfile, state: &ProfileCatalogState) -> bool {
    !profile.model_list.trim().is_empty()
        || !profile.model_windows.trim().is_empty()
        || state.overlay.custom.iter().any(|model| {
            model.template_provenance == "legacy-model-list"
        })
}

fn exact_legacy_slugs(state: &ProfileCatalogState) -> std::collections::BTreeSet<String> {
    state.overlay.custom.iter()
        .filter(|model| model.template_provenance == "legacy-model-list")
        .map(|model| model.slug.trim().to_string())
        .filter(|slug| !slug.is_empty())
        .collect()
}
```

For each profile with a legacy signal and marker below `LEGACY_MODEL_RESET_VERSION`:

```rust
let state_entry = next_state.profiles.entry(profile.id.clone()).or_default();
if !ordinary_mixed_responses(profile)
    || state_entry.mode == CatalogMode::External
    || state_entry.external_pointer.is_some()
    || state_entry.mode_explicit
{
    state_entry.legacy_model_reset_version = LEGACY_MODEL_RESET_VERSION;
    continue;
}

let removed = exact_legacy_slugs(state_entry);
state_entry.overlay.custom.retain(|model| {
    model.template_provenance != "legacy-model-list"
});
state_entry.overlay.official.clear();
state_entry.legacy_model_reset_version = LEGACY_MODEL_RESET_VERSION;
```

Clear `model_list`/`model_windows`. Read the selected model from top-level TOML with structured fallback. If and only if `removed` contains it, write `gpt-5.6-terra` to both structured `profile.model` and top-level TOML via `toml_edit`. Before writing, require the canonical model to be in `official_visible_slugs`.

Return `None` when no state/settings byte would change; otherwise increment `state.operation_generation` once and return the cloned next generation plus summaries.

- [ ] **Step 4: Run the Eva test and verify GREEN**

Run:

```bash
cd src-tauri
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test eva_implicit_legacy_gpt5_resets_to_official_terra --lib -- --nocapture
```

Expected: `1 passed; 0 failed`.

- [ ] **Step 5: Commit the planner slice**

```bash
git add src-tauri/src/lib.rs src-tauri/src/model_catalog.rs src-tauri/src/legacy_model_reset.rs
git commit -m "feat: plan unowned legacy model resets"
```

---

### Task 2: Ownership matrix, idempotency, and effective catalog proof

**Files:**
- Modify: `src-tauri/src/legacy_model_reset.rs`
- Modify: `src-tauri/src/model_catalog.rs:39-62, 1837-2012, 2185-2255`
- Test: `src-tauri/src/legacy_model_reset.rs`
- Test: `src-tauri/src/model_catalog.rs`

**Interfaces:**
- Consumes: `LegacyModelResetPlan` and planner from Task 1.
- Produces:

```rust
pub(crate) fn visible_official_slugs(state: &CatalogState) -> anyhow::Result<BTreeSet<String>>;
pub(crate) fn materialize_profile(
    state: &mut CatalogState,
    profile: &RelayProfile,
    home: &Path,
) -> anyhow::Result<Option<FileMutation>>;
pub(crate) fn plan_active_profile_with_state(
    home: &Path,
    settings: &BackendSettings,
    provider_config: &str,
    state: &mut CatalogState,
    confirm_context_cleanup: bool,
) -> anyhow::Result<ActiveCatalogPlan>;
```

The latter two functions already exist privately; this task raises them only to `pub(crate)` for the shared migration orchestrator.

- [ ] **Step 1: Add failing ownership and idempotency tests**

Add one table-driven test whose cases assert exact preservation/reset behavior:

```rust
fn matrix_fixture(
    id: &str,
    mode: CatalogMode,
    explicit: bool,
    pointer: Option<&str>,
    provenance: &str,
) -> (BackendSettings, CatalogState, std::collections::BTreeSet<String>) {
    let mut settings = eva_settings("claude-or-legacy", "claude-or-legacy", "{}");
    settings.relay_profiles[0].id = id.to_string();
    settings.active_relay_id = id.to_string();
    let mut state = state_with_profile(
        id,
        mode,
        explicit,
        vec![custom("claude-or-legacy", provenance)],
    );
    state.profiles.get_mut(id).unwrap().external_pointer = pointer.map(ToString::to_string);
    let official = std::collections::BTreeSet::from(["gpt-5.6-terra".to_string()]);
    (settings, state, official)
}

#[test]
fn reset_matrix_preserves_every_explicit_or_ambiguous_owner() {
    let cases = [
        ("explicit", CatalogMode::OfficialPlusCustom, true, None, "legacy-model-list", false),
        ("external", CatalogMode::External, false, Some("C:/models.json"), "legacy-model-list", false),
        ("user-created", CatalogMode::OfficialPlusCustom, false, None, "user-created", false),
        ("unknown", CatalogMode::OfficialPlusCustom, false, None, "", false),
        ("legacy", CatalogMode::OfficialPlusCustom, false, None, "legacy-model-list", true),
    ];

    for (id, mode, explicit, pointer, provenance, reset_expected) in cases {
        let (settings, state, official) = matrix_fixture(id, mode, explicit, pointer, provenance);
        let plan = plan_legacy_model_reset(&settings, &state, &official).unwrap();
        assert_eq!(plan.as_ref().is_some_and(|plan| !plan.reset_profiles.is_empty()), reset_expected, "{id}");
        if !reset_expected {
            let next = plan.map(|plan| plan.state).unwrap_or(state);
            assert_eq!(next.profiles[id].overlay.custom[0].slug, "claude-or-legacy");
        }
    }
}
```

Add separate tests proving:

```rust
#[test]
fn adjacent_user_custom_survives_legacy_row_removal() {
    let settings = eva_settings("gpt-5", "gpt-5\nclaude-opus-5", "{}");
    let mut state = state_with_profile(
        "eva",
        CatalogMode::OfficialPlusCustom,
        false,
        vec![custom("gpt-5", "legacy-model-list")],
    );
    state.profiles.get_mut("eva").unwrap().overlay.custom
        .push(custom("claude-opus-5", "user-created"));
    let official = std::collections::BTreeSet::from(["gpt-5.6-terra".to_string()]);

    let plan = plan_legacy_model_reset(&settings, &state, &official).unwrap().unwrap();

    let custom = &plan.state.profiles["eva"].overlay.custom;
    assert_eq!(custom.len(), 1);
    assert_eq!(custom[0].slug, "claude-opus-5");
    assert_eq!(custom[0].template_provenance, "user-created");
}

#[test]
fn valid_official_or_retained_custom_default_never_changes() {
    let official = std::collections::BTreeSet::from([
        "gpt-5.6-terra".to_string(),
        "gpt-5.6-luna".to_string(),
    ]);
    for default_model in ["gpt-5.6-luna", "claude-opus-5"] {
        let settings = eva_settings(default_model, "gpt-5\nclaude-opus-5", "{}");
        let mut state = state_with_profile(
            "eva",
            CatalogMode::OfficialPlusCustom,
            false,
            vec![custom("gpt-5", "legacy-model-list")],
        );
        state.profiles.get_mut("eva").unwrap().overlay.custom
            .push(custom("claude-opus-5", "user-created"));

        let plan = plan_legacy_model_reset(&settings, &state, &official).unwrap().unwrap();

        assert_eq!(
            top_level_model(&plan.settings.relay_profiles[0].config_contents).as_deref(),
            Some(default_model),
        );
    }
}

#[test]
fn second_reset_is_a_byte_identical_noop() {
    let (settings, state, official) = eva_legacy_fixture();
    let first = plan_legacy_model_reset(&settings, &state, &official).unwrap().unwrap();
    let settings_bytes = serde_json::to_vec(&first.settings).unwrap();
    let state_bytes = serde_json::to_vec(&first.state).unwrap();

    let second = plan_legacy_model_reset(&first.settings, &first.state, &official).unwrap();

    assert!(second.is_none());
    assert_eq!(serde_json::to_vec(&first.settings).unwrap(), settings_bytes);
    assert_eq!(serde_json::to_vec(&first.state).unwrap(), state_bytes);
}

#[test]
fn profile_without_legacy_signals_is_a_byte_identical_noop() {
    let settings = eva_settings("gpt-5.6-terra", "", "{}");
    let state = state_with_profile(
        "eva",
        CatalogMode::OfficialPlusCustom,
        false,
        vec![custom("claude-opus-5", "user-created")],
    );
    let official = std::collections::BTreeSet::from(["gpt-5.6-terra".to_string()]);

    assert!(plan_legacy_model_reset(&settings, &state, &official).unwrap().is_none());
}
```

Implement these tests with complete fixtures and concrete assertions: no empty test bodies or mock-only assertions.

- [ ] **Step 2: Run the matrix and verify RED where Task 1 is incomplete**

Run:

```bash
cd src-tauri
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test legacy_model_reset --lib -- --nocapture
```

Expected: at least the coexistence/idempotency cases fail until the planner distinguishes marker-only preservation from a real reset and retains nonlegacy rows/defaults exactly.

- [ ] **Step 3: Refine the planner and expose catalog helpers**

Use two outcome concepts internally:

```rust
let state_changed = marker_changed || overlay_changed;
let profile_reset = !removed_slugs.is_empty() || official_overrides_cleared || default_changed;
```

Only `profile_reset` produces a `ResetProfileSummary` and clears legacy profile fields. Explicit/external/ambiguous rows may receive the evaluated marker but remain semantically identical.

Make the existing model-catalog helpers `pub(crate)` and add:

```rust
pub(crate) fn visible_official_slugs(state: &CatalogState) -> anyhow::Result<BTreeSet<String>> {
    Ok(state.official.as_ref()
        .map(|snapshot| {
            catalog_models(&snapshot.raw_catalog).map(|models| models.iter()
                .filter(|model| model.get("visibility").and_then(Value::as_str) != Some("hide"))
                .filter_map(|model| model.get("slug").and_then(Value::as_str).map(ToString::to_string))
                .collect())
        })
        .transpose()?
        .unwrap_or_default())
}
```

- [ ] **Step 4: Add the effective catalog regression**

After planning Eva's reset, compose the resulting profile catalog and assert:

```rust
#[test]
fn eva_reset_catalog_contains_terra_and_not_gpt5() {
    let (settings, state, official) = eva_legacy_fixture();
    let plan = plan_legacy_model_reset(&settings, &state, &official).unwrap().unwrap();
    let catalog = compose_profile_catalog(
        &plan.state,
        &plan.settings.relay_profiles[0],
        &plan.state.profiles["eva"],
    ).unwrap();
    let slugs = catalog["models"].as_array().unwrap()
        .iter()
        .filter_map(|model| model["slug"].as_str())
        .collect::<Vec<_>>();
    assert!(slugs.contains(&"gpt-5.6-terra"));
    assert!(!slugs.contains(&"gpt-5"));
}
```

Also assert provider TOML excluding only the root `model` line is byte/semantically equivalent before and after.

- [ ] **Step 5: Run planner/catalog tests and verify GREEN**

Run:

```bash
cd src-tauri
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test legacy_model_reset --lib -- --nocapture
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test eva_reset_catalog_contains_terra_and_not_gpt5 --lib -- --nocapture
```

Expected: all focused tests pass with no failed assertions.

- [ ] **Step 6: Commit the ownership/catalog slice**

```bash
git add src-tauri/src/legacy_model_reset.rs src-tauri/src/model_catalog.rs
git commit -m "test: pin legacy model reset ownership"
```

---

### Task 3: Atomic settings, catalog, and live-config migration

**Files:**
- Modify: `src-tauri/src/commands.rs:493-509, 2564-2620, 5599-5757`
- Modify: `src-tauri/src/model_catalog.rs:771-859, 1217-1258`
- Test: `src-tauri/src/provider_commit_transaction_tests.rs`
- Test: `src-tauri/src/commands.rs`

**Interfaces:**
- Consumes: pure plan plus exposed model-catalog helpers from Tasks 1-2, `ProviderCommitPaths`, `FileMutation`, Context protection, and live-state journal.
- Precondition: the caller holds the coordinator and has already completed `migrate_legacy_profile_auth_locked_at`, so settings contain no copied OAuth residue before this function can create a recovery snapshot.
- Produces:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LegacyModelResetOutcome {
    reset_profiles: Vec<ResetProfileSummary>,
    active_restart_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyModelResetCheckpoint {
    Planned,
    CatalogMaterialized,
    BeforeCommit,
    PostCommitVerification,
}

fn migrate_legacy_model_state_locked_at(
    paths: &ProviderCommitPaths,
    observe: impl FnMut(LegacyModelResetCheckpoint) -> anyhow::Result<()>,
) -> anyhow::Result<LegacyModelResetOutcome>;
```

- [ ] **Step 1: Write the real Eva transaction test and observe RED**

Create a fixture with:

- owner-only settings containing Eva's legacy `gpt-5` default/list;
- implicit catalog state with `legacy-model-list` custom `gpt-5`;
- active live config selecting provider `OpenAI`, root `model = "gpt-5"`, Context tables, bearer, actor header, and unrelated provider fields;
- live `auth.json` sentinel bytes.

At `BeforeCommit`, inspect the staged transaction artifacts. After success, assert:

```rust
assert_eq!(stored_profile_model(&fixture.paths.settings_path, "eva"), "gpt-5.6-terra");
assert_eq!(live_root_model(&fixture.paths.codex_home), "gpt-5.6-terra");
assert!(!generated_catalog_text(&fixture.paths.codex_home, "eva").contains("\"gpt-5\""));
assert!(generated_catalog_text(&fixture.paths.codex_home, "eva").contains("\"gpt-5.6-terra\""));
assert_eq!(read_auth(&fixture.paths), auth_before);
assert_context_tables_equal(&fixture.paths, &context_before);
assert_provider_table_equal_except_root_model(&fixture.paths, &provider_before);
```

Run:

```bash
cd src-tauri
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test eva_legacy_model_reset_commits_one_protected_generation --lib -- --nocapture
```

Expected: FAIL because no migration entry point or transaction mutations exist.

- [ ] **Step 2: Implement the locked orchestrator**

The caller already owns the coordinator. Implement this order:

```rust
let raw_settings = std::fs::read(&paths.settings_path)?;
let settings: BackendSettings = serde_json::from_slice(&raw_settings)?;
let state = crate::model_catalog::load_and_migrate_state_from_path(
    &settings,
    &paths.codex_home,
    &paths.catalog_state_path,
)?;
let official = crate::model_catalog::visible_official_slugs(&state)?;
let Some(mut plan) = crate::legacy_model_reset::plan_legacy_model_reset(
    &settings,
    &state,
    &official,
)? else {
    return Ok(LegacyModelResetOutcome::default());
};
```

Build mutations from the planned generation:

```rust
let next_settings_bytes = serialize_settings_without_profile_auth(&plan.settings)?;
let mut mutations = Vec::new();
if next_settings_bytes != raw_settings {
    mutations.push(FileMutation::bytes(
        paths.settings_path.clone(),
        next_settings_bytes,
    ));
}
let mut context_snapshots = Vec::new();

for reset in &plan.reset_profiles {
    let profile = plan.settings.relay_profiles.iter()
        .find(|profile| profile.id == reset.profile_id)
        .context("reset profile disappeared")?;
    if reset.active && plan.settings.relay_profiles_enabled {
        let live = std::fs::read_to_string(paths.codex_home.join("config.toml"))?;
        let active_plan = crate::model_catalog::plan_active_profile_with_state(
            &paths.codex_home,
            &plan.settings,
            &crate::legacy_model_reset::set_top_level_model(&live, &reset.next_model)?,
            &mut plan.state,
            false,
        )?;
        mutations.extend(active_plan.mutations);
        let (protected, snapshot) = context_protected_config(
            &paths.codex_home,
            &active_plan.config_contents,
        )?;
        mutations.push(FileMutation::text(paths.codex_home.join("config.toml"), protected));
        context_snapshots.push(snapshot);
    } else if let Some(mutation) = crate::model_catalog::materialize_profile(
        &mut plan.state,
        profile,
        &paths.codex_home,
    )? {
        mutations.push(mutation);
    }
}
mutations.push(crate::model_catalog::state_mutation_at(
    &plan.state,
    &paths.catalog_state_path,
)?);
```

Snapshot live auth bytes and expected raw generations before staging. Commit via `live_state::commit_locked_verified_at_observed`, map the migration checkpoints, and verify:

- settings/state/catalog/config hashes match the planned generation;
- every Context snapshot re-verifies;
- live auth bytes equal the pre-commit bytes; and
- all profile markers are `LEGACY_MODEL_RESET_VERSION`.

If active live config no longer selects the persisted active provider ID or its current generation changed after planning, return an error before mutation.

- [ ] **Step 3: Wire both safe entry points**

In `load_settings_blocking`, after auth scrub and under the same coordinator lock:

```rust
let reset = migrate_legacy_model_state_locked_at(&ProviderCommitPaths::defaults(), |_| Ok(()))?;
```

In `commit_provider_detail_from_paths_observed`, run the same migration after auth scrub and before `load_provider_commit_settings`, so a direct invoke cannot snapshot legacy model state or judge CAS against a generation the UI should no longer see.

Map malformed input to existing static `InputUnavailable` reasons and transaction/storage failure to static `TransactionFailed`. Do not expose TOML, model catalog JSON, bearer, or auth contents.

- [ ] **Step 4: Add injected rollback and no-op tests**

For every `LegacyModelResetCheckpoint` after planning, inject one failure and assert byte identity for settings, catalog state, generated catalog, live config, auth, and transaction artifacts after recovery.

Add an ordinary profile fixture with empty legacy fields and a user-created overlay; run both load and direct commit entry points and assert no file mtime/content/generation changes and no new failure code.

- [ ] **Step 5: Run transaction and existing auth/context suites**

Run:

```bash
cd src-tauri
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test legacy_model_reset --lib -- --nocapture
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test eva_legacy_model_reset_commits_one_protected_generation --lib -- --nocapture
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test successful_active_commit_preserves_context_semantics_and_auth_bytes --lib -- --nocapture
```

Expected: all focused and existing security regressions pass.

- [ ] **Step 6: Commit the transaction slice**

```bash
git add src-tauri/src/commands.rs src-tauri/src/model_catalog.rs src-tauri/src/provider_commit_transaction_tests.rs
git commit -m "fix: reset legacy model state atomically"
```

---

### Task 4: One-time UI notice and cross-boundary Terra contract

**Files:**
- Modify: `src-tauri/src/commands.rs:56-62, 493-509, 5599-5645`
- Modify: `src/backend-types.ts:178-183`
- Modify: `src/App.tsx:286-303`
- Modify: `src/i18n-en.ts`
- Test: `src/provider-onboarding.test.ts`
- Test: `src/provider-save-liveness.test.ts`

**Interfaces:**
- Consumes: `LegacyModelResetOutcome` from Task 3.
- Produces one optional payload field:

```rust
pub legacy_model_reset_notice: Option<String>,
```

mirrored as:

```ts
legacy_model_reset_notice?: string | null;
```

- [ ] **Step 1: Add failing backend/frontend notice tests**

Add a Rust assertion that a reset outcome produces exactly:

```text
已丢弃旧版自动生成的模型列表，并恢复官方模型；启动模型已设为 5.6 Terra。请重启 Codex 后新建任务。
```

Add a frontend source/wiring test asserting `refreshSettings(true)` still calls `showNotice` when `legacy_model_reset_notice` is non-empty, while an ordinary silent successful load remains silent.

Run:

```bash
npm test -- --test-name-pattern="legacy model reset notice"
cd src-tauri
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test load_settings_returns_legacy_model_reset_notice --lib -- --nocapture
```

Expected: tests fail because the payload field and wiring do not exist.

- [ ] **Step 2: Add the payload field and show the one-time notice**

Initialize `legacy_model_reset_notice: None` in every fallback/settings payload constructor. In `load_settings_blocking`, attach the notice only when `reset.reset_profiles` is non-empty. The persisted migration marker ensures later loads return `None`.

In `refreshSettings` after adopting the valid baseline:

```ts
if (result.legacy_model_reset_notice) {
  showNotice(t("模型目录已恢复"), result.legacy_model_reset_notice, "ok");
} else if (!silent) {
  showResultNotice(t("设置已加载"), result, { silentSuccess: true });
}
```

Add the English title and notice translation without making a capability, plan, quota, or authentication claim.

- [ ] **Step 3: Pin the frontend/backend/default-catalog contract**

Extend `provider-onboarding.test.ts` to read the real Rust module and bundled asset:

```ts
const rust = readFileSync(new URL("../src-tauri/src/legacy_model_reset.rs", import.meta.url), "utf8");
const catalog = JSON.parse(readFileSync(new URL("../src-tauri/assets/official-model-catalog.json", import.meta.url), "utf8"));
assert.equal(PRO_MODEL_SLUGS[0], "gpt-5.6-terra");
assert.match(rust, /CANONICAL_MIXED_DEFAULT_MODEL:\s*&str\s*=\s*"gpt-5\.6-terra"/);
assert.equal(catalog.models.find((model: { slug: string }) => model.slug === "gpt-5.6-terra")?.visibility, "list");
```

- [ ] **Step 4: Run frontend/backend notice tests and verify GREEN**

Run:

```bash
npm run check
npm test
cd src-tauri
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target \
  cargo test load_settings_returns_legacy_model_reset_notice --lib -- --nocapture
```

Expected: TypeScript and all frontend/focused Rust tests pass.

- [ ] **Step 5: Commit the UI contract**

```bash
git add src-tauri/src/commands.rs src/backend-types.ts src/App.tsx src/i18n-en.ts src/provider-onboarding.test.ts src/provider-save-liveness.test.ts
git commit -m "feat: report official model reset"
```

---

### Task 5: Synchronize specifications, durable constraints, and full verification

**Files:**
- Modify: `openspec/specs/model-catalog-management/spec.md`
- Modify: `openspec/specs/provider-native-capability-mode/spec.md`
- Modify: `AGENTS.md`
- Modify after all gates pass: `BOARD.md`
- Modify: `docs/superpowers/plans/2026-08-17-reset-legacy-model-overlay.md` (checkboxes only)

**Interfaces:**
- Consumes: completed behavior and exact test evidence from Tasks 1-4.
- Produces: authoritative specification/constraint text and a completed-work record.

- [ ] **Step 1: Add the normative OpenSpec scenarios**

Add these scenario families with exact ownership language:

```markdown
#### Scenario: Unowned legacy model list is reset

- **WHEN** an ordinary mixed Responses profile has implicit catalog state containing exact `legacy-model-list` rows and no external ownership
- **THEN** the manager removes only those rows, restores the bundled official model content, clears consumed legacy list fields, and changes a removed legacy default to `gpt-5.6-terra`

#### Scenario: Explicit or ambiguous catalog ownership is preserved

- **WHEN** a profile has an explicit mode, external pointer, user-created/preset row, or unknown provenance
- **THEN** startup does not delete or rewrite that owned or ambiguous model state

#### Scenario: Active legacy reset is one protected generation

- **WHEN** the affected profile is active
- **THEN** settings, catalog state, generated catalog, and live config commit atomically under Context protection while live auth remains byte-for-byte unchanged

#### Scenario: Legacy reset is idempotent and does not gate ordinary profiles

- **WHEN** reset already ran or the profile has no eligible legacy state
- **THEN** subsequent startup and provider commits perform no write and add no save/switch rejection
```

Update the provider-native-capability no-startup-rewrite rule with one narrow exception for exact unowned legacy model-list reset; preserve the prohibition on core normalization and explicit/user-owned rewrites.

- [ ] **Step 2: Update the AGENTS hot-path constraint**

Under model-catalog/native-capability ownership, add one concise rule:

```markdown
Startup may reset only exact unowned `legacy-model-list` catalog state through the coordinator: remove only those rows, repair a removed default to `gpt-5.6-terra`, and preserve every explicit, external, preset, user-created, or unknown-provenance model state. This narrow migration is not permission to run the core normalizer or rewrite any other profile contract.
```

- [ ] **Step 3: Run focused validators and complete repository gates**

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
openspec validate model-catalog-management --type spec --strict --no-interactive
openspec validate provider-native-capability-mode --type spec --strict --no-interactive
git diff --check
cd src-tauri
CARGO_TARGET_DIR=/Users/mingjian/Documents/sync/GitHub/codex-minus/src-tauri/target cargo test
cd ..
npm run verify
npm run vite:build
```

Expected: every command exits `0`; capture the exact Rust/frontend counts from output.

- [ ] **Step 4: Append BOARD only after all gates pass**

Under `### 2026-08-17`, add:

```markdown
- **fix/catalog**: unowned legacy model-list overlays now reset to the official baseline, so Eva's stale GPT-5 row is discarded and Terra becomes the persistent startup default
  - why: the 0.4.14 auth repair revealed an older manager-owned `gpt-5` model/default; preserving it was safe but incomplete because the user never explicitly adopted that legacy row
  - verified: focused planner/ownership/transaction/notice regressions, full Rust and frontend gates, strict OpenSpec, formatting, diff, and production build passed with the exact command-reported counts
  - refs: `src-tauri/src/legacy_model_reset.rs`, `src-tauri/src/model_catalog.rs`, `src-tauri/src/commands.rs`, OpenSpec, design and implementation plan
```

Replace no text with estimated counts; add exact counts only when command output states them.

- [ ] **Step 5: Commit the authoritative docs and plan completion**

```bash
git add AGENTS.md BOARD.md openspec/specs/model-catalog-management/spec.md openspec/specs/provider-native-capability-mode/spec.md docs/superpowers/plans/2026-08-17-reset-legacy-model-overlay.md
git commit -m "docs: record official legacy model reset"
```

- [ ] **Step 6: Confirm the final branch is review-ready**

Run:

```bash
git status --short
git log --oneline --decorate -8
git diff --check master..HEAD
```

Expected: clean worktree, planned commits only, and no diff errors.

---

## Final Evidence Budget

| Claim | Owner | Minimum decisive evidence |
|---|---|---|
| Eva's stale GPT-5 row/default resets to Terra | Pure planner + transaction regression | RED on current behavior, then settings/live/catalog GREEN |
| Only unowned legacy rows are removed | Ownership matrix | Explicit/external/user/preset/unknown cases unchanged |
| User custom rows survive mixed overlays | Planner/catalog test | Adjacent legacy row removed, user row retained |
| Migration is one protected generation | Transaction fixture | Injected failure rollback across settings/state/catalog/config |
| Auth and Context remain untouched | Transaction fixture | Byte/semantic equality before and after |
| Ordinary saves/switches gain no gate | No-op entry-point test | No file/generation mutation and unchanged success result |
| Default contract stays Terra across languages | Cross-boundary source/asset test | TypeScript constant, Rust constant, visible bundled row agree |
| User sees one accurate notice | Backend payload + App wiring tests | First reset shows notice; later silent load stays silent |
| Durable contracts match behavior | Strict OpenSpec + AGENTS review | Both specs validate; narrow exception is explicit |
| Branch is shippable | Full gates | Rust, npm verify/build, formatting, diff, and later three-platform CI |
