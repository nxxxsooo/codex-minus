# Legacy Provider Auth Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Codex Minus automatically repair Eva-shaped legacy provider `authContents` residue while preserving provider-key ownership, rejecting credential ambiguity, and never touching live ChatGPT OAuth.

**Architecture:** Keep `migrate_persisted_legacy_api_key_auth` as the single profile-level reconciliation rule used by startup and provider-commit loading. Extend it from an API-key-only parser into a mode-aware three-source reconciliation across legacy `OPENAI_API_KEY`, structured `apiKey`, and the selected provider bearer; keep the existing coordinator, owner-only settings file, auth-free serializer, and atomic replacement as the only persistence path.

**Tech Stack:** Rust, `anyhow`, `serde_json`, `toml_edit`, Codex Plus Core settings types, Rust unit/integration tests, OpenSpec Markdown, npm verification scripts.

## Global Constraints

- Live `%USERPROFILE%\.codex\auth.json` or `~/.codex/auth.json` remains byte-for-byte unchanged.
- OAuth fields from profile `authContents` are discarded, never migrated, backed up, logged, returned to the frontend, or written into live auth.
- A provider key may be written only for pure-API or official-mixed-with-key profiles.
- Any disagreement among legacy key, structured `apiKey`, and provider bearer fails without writing settings.
- Startup migration must not normalize, rename, or otherwise rewrite the provider contract beyond relocating the agreed bearer.
- Persisted migrated settings contain no `authContents` field and remain owner-only.
- Do not probe Sub2API, consume Eva's quota, or change models, catalog ownership, active profile, provider ID, or native-capability mode.
- Use `apply_patch` for source, test, specification, plan, and BOARD edits.

---

### Task 1: Repair Eva-shaped mixed auth residue

**Files:**
- Modify: `src-tauri/src/commands.rs:3070-3111`
- Test: `src-tauri/src/commands.rs:6255-6282`

**Interfaces:**
- Consumes: `RelayProfile`, `provider_bearer_token_from_config_exact(&str) -> Option<String>`, and `set_provider_config_bearer(&str, &str, Option<bool>) -> anyhow::Result<String>`.
- Produces: the existing `migrate_persisted_legacy_api_key_auth(&mut RelayProfile) -> anyhow::Result<()>` with broader, deterministic migration semantics; no new public API.

- [ ] **Step 1: Replace the rejection regression with Eva's expected successful migration**

Replace `load_time_legacy_migration_rejects_mixed_oauth_payload_without_writing` with a test shaped like the real official-mixed profile:

```rust
#[test]
fn load_time_legacy_migration_repairs_api_key_plus_oauth_residue() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let mut settings = BackendSettings::default();
    settings.relay_profiles = vec![RelayProfile {
        id: "eva".to_string(),
        name: "Eva|Codex".to_string(),
        model: "gpt-5.6-terra".to_string(),
        relay_mode: codex_plus_core::settings::RelayMode::Official,
        official_mix_api_key: true,
        protocol: codex_plus_core::settings::RelayProtocol::Responses,
        base_url: "https://example.test/v1".to_string(),
        upstream_base_url: "https://example.test/v1".to_string(),
        config_contents: r#"model = "gpt-5.6-terra"
model_provider = "OpenAI"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://example.test/v1"
wire_api = "responses"
requires_openai_auth = true
"#
        .to_string(),
        auth_contents: r#"{
            "OPENAI_API_KEY": "provider-key-sentinel",
            "auth_mode": "chatgpt",
            "tokens": {"access_token": "oauth-access-sentinel"}
        }"#
        .to_string(),
        ..RelayProfile::default()
    }];
    std::fs::write(&settings_path, serde_json::to_vec_pretty(&settings).unwrap()).unwrap();
    let _guard = live_state::lock().unwrap();

    assert_eq!(migrate_legacy_profile_auth_locked_at(&settings_path).unwrap(), 1);

    let bytes = std::fs::read(&settings_path).unwrap();
    let migrated: BackendSettings = serde_json::from_slice(&bytes).unwrap();
    let profile = &migrated.relay_profiles[0];
    assert_eq!(profile.api_key, "provider-key-sentinel");
    assert_eq!(
        provider_bearer_token_from_config_exact(&profile.config_contents).as_deref(),
        Some("provider-key-sentinel")
    );
    assert!(!String::from_utf8(bytes).unwrap().contains("authContents"));
    assert!(!profile.config_contents.contains("oauth-access-sentinel"));
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cd src-tauri
cargo test load_time_legacy_migration_repairs_api_key_plus_oauth_residue --lib -- --nocapture
```

Expected: FAIL because the current `object.len() == 1` gate returns `persisted provider auth copy is not API-key-only`.

- [ ] **Step 3: Implement the minimum mixed-residue reconciliation**

Change `migrate_persisted_legacy_api_key_auth` so additional object fields are ignored, but the non-empty legacy key still must agree with existing provider-key destinations:

```rust
fn profile_owns_provider_key(profile: &RelayProfile) -> bool {
    profile.relay_mode == codex_plus_core::settings::RelayMode::PureApi
        || (profile.relay_mode == codex_plus_core::settings::RelayMode::Official
            && profile.official_mix_api_key)
}

fn non_empty_provider_key(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn migrate_persisted_legacy_api_key_auth(profile: &mut RelayProfile) -> anyhow::Result<()> {
    if profile.auth_contents.is_empty() {
        return Ok(());
    }
    let value: serde_json::Value = serde_json::from_str(&profile.auth_contents)
        .map_err(|_| anyhow::anyhow!("persisted provider auth copy is invalid"))?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("persisted provider auth copy is invalid"))?;
    anyhow::ensure!(
        profile_owns_provider_key(profile),
        "persisted provider auth copy has no provider-key owner"
    );
    let legacy_key = non_empty_provider_key(
        object.get("OPENAI_API_KEY").and_then(serde_json::Value::as_str),
    )
    .ok_or_else(|| anyhow::anyhow!("persisted provider API key is missing"))?;
    let structured_key = non_empty_provider_key(Some(&profile.api_key));
    let bearer_key = provider_bearer_token_from_config_exact(&profile.config_contents)
        .and_then(|value| non_empty_provider_key(Some(&value)));
    for existing in [structured_key.as_deref(), bearer_key.as_deref()]
        .into_iter()
        .flatten()
    {
        anyhow::ensure!(
            existing.as_bytes() == legacy_key.as_bytes(),
            "persisted provider key conflict"
        );
    }
    profile.api_key = legacy_key.clone();
    profile.config_contents =
        set_provider_config_bearer(&profile.config_contents, &legacy_key, None)?;
    profile.auth_contents.clear();
    Ok(())
}
```

Keep helper visibility private and place both helpers beside the migration function.

- [ ] **Step 4: Run the focused regression and nearby migration tests**

Run:

```bash
cd src-tauri
cargo test load_time_legacy_migration --lib -- --nocapture
cargo test normalization_projects_api_key_only_legacy_copy_to_provider_config --lib -- --nocapture
```

Expected: PASS. The new test proves the current failure is repaired; the existing API-key-only path remains green.

- [ ] **Step 5: Commit the working Eva migration slice**

```bash
git add src-tauri/src/commands.rs
git commit -m "fix: migrate mixed legacy provider auth copies"
```

---

### Task 2: Pin OAuth-only, missing-key, malformed, and conflict boundaries

**Files:**
- Modify: `src-tauri/src/commands.rs:3070-3115, 5574-5607`
- Test: `src-tauri/src/commands.rs` migration tests near the existing load-time regression

**Interfaces:**
- Consumes: `profile_owns_provider_key`, `non_empty_provider_key`, and `migrate_legacy_profile_auth_locked_at(&Path) -> anyhow::Result<usize>` from Task 1.
- Produces: complete migration decision matrix and safe profile identification on startup failures.

- [ ] **Step 1: Add failing tests for every remaining decision boundary**

Add focused tests with real serialized `BackendSettings` values:

```rust
#[test]
fn oauth_only_residue_uses_an_existing_provider_bearer() {
    let mut profile = RelayProfile {
        id: "mixed".to_string(),
        relay_mode: codex_plus_core::settings::RelayMode::Official,
        official_mix_api_key: true,
        config_contents: set_provider_config_bearer("", "existing-key", Some(true)).unwrap(),
        auth_contents: r#"{"auth_mode":"chatgpt","tokens":{"access_token":"oauth-sentinel"}}"#
            .to_string(),
        ..RelayProfile::default()
    };

    migrate_persisted_legacy_api_key_auth(&mut profile).unwrap();

    assert_eq!(profile.api_key, "existing-key");
    assert!(profile.auth_contents.is_empty());
}

#[test]
fn pure_oauth_discards_the_complete_legacy_copy_without_adopting_its_key() {
    let mut profile = RelayProfile {
        id: "official".to_string(),
        relay_mode: codex_plus_core::settings::RelayMode::Official,
        official_mix_api_key: false,
        auth_contents: r#"{"OPENAI_API_KEY":"orphan-key","auth_mode":"chatgpt"}"#.to_string(),
        ..RelayProfile::default()
    };

    migrate_persisted_legacy_api_key_auth(&mut profile).unwrap();

    assert!(profile.api_key.is_empty());
    assert!(provider_bearer_token_from_config_exact(&profile.config_contents).is_none());
    assert!(profile.auth_contents.is_empty());
}
```

Also add three filesystem-level tests using a shared local fixture helper to avoid repeated serialization:

```rust
// provider-key mode + OAuth-only + no structured key/bearer
assert!(error.to_string().contains("Eva|Codex"));
assert!(!error.to_string().contains("oauth-access-sentinel"));
assert_eq!(std::fs::read(&settings_path).unwrap(), before);

// legacy key disagrees with the structured key or bearer
assert!(error.to_string().contains("persisted provider key conflict"));
assert_eq!(std::fs::read(&settings_path).unwrap(), before);

// authContents is invalid JSON or a non-object
assert!(error.to_string().contains("persisted provider auth copy is invalid"));
assert_eq!(std::fs::read(&settings_path).unwrap(), before);
```

The fixture helper must write only under `tempfile::tempdir()`, return the exact pre-migration bytes, and never print credential sentinels.

- [ ] **Step 2: Run the new tests and verify RED for the unimplemented boundaries**

Run:

```bash
cd src-tauri
cargo test oauth_only_residue --lib -- --nocapture
cargo test pure_oauth_discards --lib -- --nocapture
cargo test legacy_profile_auth --lib -- --nocapture
```

Expected: at least the missing-profile-context test fails because the current filesystem loop propagates a context-free error. If any behavior test unexpectedly passes, strengthen its assertion to cover the missing observable state rather than weakening the requirement.

- [ ] **Step 3: Add safe profile context to startup migration failures**

Wrap the profile-level migration at the filesystem boundary, where dynamic context is permitted, while preserving static commit failure reasons:

```rust
let profile_label = if profile.name.trim().is_empty() {
    profile.id.trim().to_string()
} else {
    profile.name.trim().to_string()
};
migrate_persisted_legacy_api_key_auth(profile).with_context(|| {
    format!("provider profile {profile_label:?} failed auth migration")
})?;
```

Do not change `ProviderCommitPayload.reason` or `ProviderCommitFailure`: their static-string contract intentionally prevents dynamic credential-bearing error content from crossing IPC.

- [ ] **Step 4: Refine reconciliation only as required by the failing tests**

Keep these invariants in the implementation:

```rust
// Pure OAuth owns no provider key, so parsing proves object shape and then cleanup stops.
if !profile_owns_provider_key(profile) {
    profile.auth_contents.clear();
    return Ok(());
}

// A provider-key mode must end with exactly one agreed non-empty key.
let agreed_key = candidates
    .next()
    .map(ToString::to_string)
    .ok_or_else(|| anyhow::anyhow!("persisted provider API key is missing"))?;
anyhow::ensure!(
    candidates.all(|candidate| candidate.as_bytes() == agreed_key.as_bytes()),
    "persisted provider key conflict"
);
```

An empty or non-string legacy `OPENAI_API_KEY` is ignored only when a valid structured key or bearer already supplies the provider credential. Invalid JSON and non-object payloads always fail before mode-specific cleanup.

- [ ] **Step 5: Run all command-module migration tests**

Run:

```bash
cd src-tauri
cargo test legacy_profile_auth --lib -- --nocapture
cargo test oauth_only_residue --lib -- --nocapture
cargo test pure_oauth_discards --lib -- --nocapture
cargo test normalization_projects_api_key_only_legacy_copy_to_provider_config --lib -- --nocapture
```

Expected: PASS, with failed migrations retaining byte-identical settings and no error rendering any OAuth or provider-key sentinel.

- [ ] **Step 6: Commit the safety matrix**

```bash
git add src-tauri/src/commands.rs
git commit -m "test: pin provider auth migration boundaries"
```

---

### Task 3: Verify both entry points and synchronize the durable contract

**Files:**
- Modify: `src-tauri/src/commands.rs` tests near `load_provider_commit_settings`
- Modify: `openspec/specs/provider-native-capability-mode/spec.md:419-425, 525-534`
- Modify: `BOARD.md` at the top of the 2026-08-17 section

**Interfaces:**
- Consumes: the completed migration rule from Tasks 1-2, `load_provider_commit_settings(&Path)`, `serialize_settings_without_profile_auth`, and the existing provider transaction/live-auth invariants.
- Produces: direct evidence that startup and provider-commit loading agree, plus the updated normative specification and completed-work record.

- [ ] **Step 1: Add a failing/behavior-locking commit-load regression before any further production change**

Add a unit test that writes Eva-shaped settings, calls `load_provider_commit_settings`, and asserts both outputs deliberately:

```rust
#[test]
fn provider_commit_load_accepts_eva_residue_without_mutating_its_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let original = eva_legacy_settings_bytes();
    std::fs::write(&settings_path, &original).unwrap();

    let (snapshot, loaded) = load_provider_commit_settings(&settings_path).unwrap();

    assert_eq!(snapshot, original);
    assert_eq!(std::fs::read(&settings_path).unwrap(), original);
    let profile = &loaded.relay_profiles[0];
    assert!(profile.auth_contents.is_empty());
    assert_eq!(profile.api_key, "provider-key-sentinel");
    assert_eq!(
        provider_bearer_token_from_config_exact(&profile.config_contents).as_deref(),
        Some("provider-key-sentinel")
    );
}
```

Extract `eva_legacy_settings_bytes() -> Vec<u8>` only inside the test module and reuse it in the startup regression. The helper contains sentinel credentials solely in temp-file test data.

- [ ] **Step 2: Run the entry-point test**

Run:

```bash
cd src-tauri
cargo test provider_commit_load_accepts_eva_residue_without_mutating_its_snapshot --lib -- --nocapture
```

Expected: PASS if Tasks 1-2 fully unified the rule. If it fails, change only the shared migration call or test fixture; do not add a second migration implementation.

- [ ] **Step 3: Synchronize the main OpenSpec scenarios**

Replace the API-key-only scenario with explicit mixed-residue behavior:

```markdown
#### Scenario: Legacy provider auth copy exists on disk

- **WHEN** controlled legacy migration finds an `OPENAI_API_KEY` together with OAuth or other legacy fields in a provider-key profile, and every existing provider-key destination agrees
- **THEN** it moves the agreed key into the owner-only provider bearer field, deletes the complete legacy copy, and does not read or write live auth

#### Scenario: Legacy OAuth copy has no provider-key owner

- **WHEN** a pure-OAuth profile contains a legacy `authContents` copy, whether or not that copy also contains `OPENAI_API_KEY`
- **THEN** it deletes the profile copy without creating a provider bearer and leaves live official auth untouched

#### Scenario: Legacy provider auth copy is ambiguous

- **WHEN** provider-key evidence conflicts, the legacy payload is malformed, or a provider-key profile has no usable key source
- **THEN** migration fails without changing persisted settings or exposing credential material
```

Update the later startup-credential-migration requirement so it permits removal of legacy OAuth fields as well as relocation of the provider key, while continuing to prohibit implicit provider-contract normalization.

- [ ] **Step 4: Run focused formatting and specification checks**

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
openspec validate provider-native-capability-mode --type spec --strict --no-interactive
git diff --check
```

Expected: Rust formatting is clean, the main provider specification passes strict validation, and no diff whitespace errors are reported.

- [ ] **Step 5: Run the complete repository evidence path**

Run:

```bash
cd src-tauri
cargo test
cd ..
npm run verify
```

Expected: all Rust tests, TypeScript checks, frontend tests, and knip pass. No live provider request is sent by these commands.

- [ ] **Step 6: Append the completed-work record only after all checks pass**

At the top of the `2026-08-17` section in `BOARD.md`, add one entry with:

```markdown
- **fix/providers**: legacy profile `authContents` containing a provider API key plus stale OAuth fields now migrates automatically instead of blocking settings load
  - why: older Codex Minus/Codex++ Manager versions left Eva's mixed credential copy behind; the strict API-key-only upgrade gate turned product-owned residue into a user repair task
  - verified: the focused migration regressions, full `cargo test`, `npm run verify`, strict OpenSpec validation, Rust formatting, and diff checks passed
  - refs: `src-tauri/src/commands.rs`, `openspec/specs/provider-native-capability-mode/spec.md`, design and implementation plan
```

Add exact test counts only when the command output reports them directly; do not estimate counts.

- [ ] **Step 7: Re-run document checks and commit the completed change**

Run:

```bash
git diff --check
git status --short
git add src-tauri/src/commands.rs openspec/specs/provider-native-capability-mode/spec.md BOARD.md docs/superpowers/plans/2026-08-17-migrate-legacy-provider-auth.md
git commit -m "docs: record legacy auth migration"
```

Expected: the commit contains only the planned migration, tests, normative spec, BOARD entry, and this plan. Any unrelated user-owned change remains unstaged.

---

## Final evidence budget

| Claim | Owner | Minimum decisive evidence |
|---|---|---|
| Eva-shaped `OPENAI_API_KEY` plus OAuth residue migrates | `commands.rs` unit regression | Focused RED before Task 1 implementation, then focused PASS |
| OAuth-only and pure-OAuth ownership rules are safe | `commands.rs` boundary tests | Focused PASS for existing-bearer, pure-OAuth, missing-key, malformed, and conflict cases |
| Failed migration does not mutate settings or leak sentinels | Filesystem-level unit tests | Byte equality plus negative error/string assertions |
| Startup and provider-commit loading share one rule | Two direct entry-point tests | Both return the same migrated profile semantics; commit-load keeps its raw snapshot unchanged |
| Live official auth remains untouched | Architectural boundary plus existing provider transaction tests | Migration accepts only a settings path; complete `cargo test` retains live-auth transaction regressions |
| Repository remains shippable | Project acceptance gates | `cargo test`, `npm run verify`, `cargo fmt --check`, and `git diff --check` |
