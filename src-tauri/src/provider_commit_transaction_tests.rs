use std::fs;

use crate::commands::{
    GenericSettingsSaveError, ProviderCommitCheckpoint, ProviderCommitErrorCode,
    ProviderCommitPaths, ProviderCommitPayload, assert_staged_native_provider_contract,
    commit_provider_detail_from_paths, commit_provider_detail_from_paths_observed,
    save_settings_with_provider_guard_at, save_settings_with_provider_guard_at_observed,
    settings_snapshot_for_ui_projection, ui_provider_topology_projection,
};
use crate::provider_commit::{
    CatalogMode, CatalogOverlay, CatalogState, CustomModel, OfficialSnapshot, ProfileCatalogDraft,
    ProviderCommitAction, ProviderCommitRequest, ProviderOwnedTopologyDraft, UpstreamTopology,
    provider_owned_fingerprint,
};
use base64::Engine;
use codex_plus_core::settings::{
    AggregateRelayMember, AggregateRelayProfile, AggregateRelayStrategy, BackendSettings,
    RelayMode, RelayProfile, RelayProtocol, SettingsStore,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn canonical_profile(id: &str, model: &str, base_url: &str, key: &str) -> RelayProfile {
    RelayProfile {
        id: id.to_string(),
        name: format!("Provider {id}"),
        model: model.to_string(),
        base_url: base_url.to_string(),
        upstream_base_url: base_url.to_string(),
        api_key: key.to_string(),
        protocol: RelayProtocol::Responses,
        relay_mode: RelayMode::Official,
        official_mix_api_key: true,
        config_contents: format!(
            r#"model = "{model}"
model_provider = "RelayOne"

[model_providers.RelayOne]
name = "OpenAI"
base_url = "{base_url}"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "{key}"
http_headers = {{ "x-openai-actor-authorization" = "local-image-extension", "x-keep" = "yes" }}
"#
        ),
        ..RelayProfile::default()
    }
}

fn pure_oauth_profile(id: &str) -> RelayProfile {
    RelayProfile {
        id: id.to_string(),
        name: "Official".to_string(),
        model: "official-a".to_string(),
        relay_mode: RelayMode::Official,
        protocol: RelayProtocol::Responses,
        config_contents: "model = \"official-a\"\n".to_string(),
        ..RelayProfile::default()
    }
}

fn legacy_pure_api_profile(id: &str, auth_contents: &str) -> RelayProfile {
    let mut profile = canonical_profile(
        id,
        "official-a",
        "https://legacy.example/v1",
        "legacy-provider-key",
    );
    profile.relay_mode = RelayMode::PureApi;
    profile.official_mix_api_key = false;
    profile.auth_contents = auth_contents.to_string();
    profile
}

fn settings_with(profiles: Vec<RelayProfile>, active: &str) -> BackendSettings {
    BackendSettings {
        relay_profiles_enabled: true,
        relay_profiles: profiles,
        active_relay_id: active.to_string(),
        ..BackendSettings::default()
    }
}

fn official_catalog() -> Value {
    json!({
        "models": [{
            "slug": "official-a",
            "display_name": "Official A",
            "description": "A",
            "visibility": "list",
            "priority": 1,
            "context_window": 100000,
            "max_context_window": 100000,
            "effective_context_window_percent": 95,
            "base_instructions": "test instructions"
        }]
    })
}

fn hash_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn target_identity(version: &str, identity: &str) -> crate::model_catalog::VerifiedTargetIdentity {
    crate::model_catalog::VerifiedTargetIdentity {
        app_path: "/Applications/ChatGPT.app".to_string(),
        cli_path: "/Applications/ChatGPT.app/Contents/Resources/codex".to_string(),
        client_version: version.to_string(),
        publisher: "OpenAI Test Publisher".to_string(),
        identity_hash: identity.to_string(),
        trusted: true,
        capability_available: true,
        capability_message: "available".to_string(),
    }
}

fn official_auth_bytes(account: &str, workspace: &str) -> Vec<u8> {
    official_auth_bytes_with_exp(account, workspace, 4_102_444_800_u64)
}

fn official_auth_bytes_with_exp(account: &str, workspace: &str, exp: u64) -> Vec<u8> {
    let encode = |value: Value| {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&value).unwrap())
    };
    let access_token = format!("header.{}.signature", encode(json!({ "exp": exp })));
    let id_token = format!("header.{}.signature", encode(json!({})));
    serde_json::to_vec_pretty(&json!({
        "auth_mode": "chatgpt",
        "tokens": {
            "access_token": access_token,
            "id_token": id_token,
            "account_id": account,
            "workspace_id": workspace
        }
    }))
    .unwrap()
}

fn official_scope_hash(salt: &str, account: &str, workspace: &str) -> String {
    let scope_identity = hash_text(&format!("{salt}:{account}:{workspace}"));
    hash_text(&format!("{salt}:{scope_identity}"))
}

fn state_with_official() -> CatalogState {
    let scope_salt = "provider-commit-test-salt".to_string();
    CatalogState {
        scope_salt: scope_salt.clone(),
        official: Some(OfficialSnapshot {
            client_version: "0.147.0".to_string(),
            scope_hash: official_scope_hash(&scope_salt, "account-a", "workspace-a"),
            raw_catalog: official_catalog(),
            ..OfficialSnapshot::default()
        }),
        target: Some(target_identity("0.147.0", "target-a")),
        ..CatalogState::default()
    }
}

fn catalog_draft(profile_id: &str) -> ProfileCatalogDraft {
    ProfileCatalogDraft {
        profile_id: profile_id.to_string(),
        mode: CatalogMode::OfficialPlusCustom,
        mode_explicit: false,
        upstream_topology: UpstreamTopology::Direct,
        external_pointer: None,
        overlay: CatalogOverlay::default(),
    }
}

fn request(
    persisted: &BackendSettings,
    next: &BackendSettings,
    focused: &str,
    action: ProviderCommitAction,
    revision: u64,
) -> ProviderCommitRequest {
    ProviderCommitRequest {
        topology: ProviderOwnedTopologyDraft::from_settings(next),
        catalog_drafts: vec![catalog_draft(focused)],
        focused_profile_id: Some(focused.to_string()),
        action,
        previous_active_relay_id: persisted.active_relay_id.clone(),
        confirm_context_cleanup: false,
        draft_revision: revision,
        expected_provider_fingerprint: provider_owned_fingerprint(
            &ProviderOwnedTopologyDraft::from_settings(persisted),
        )
        .unwrap(),
    }
}

struct Fixture {
    _temp: tempfile::TempDir,
    paths: ProviderCommitPaths,
}

impl Fixture {
    fn new(settings: &BackendSettings, state: &CatalogState) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let app_state = temp.path().join("app-state");
        let codex_home = temp.path().join("codex-home");
        fs::create_dir_all(&app_state).unwrap();
        fs::create_dir_all(&codex_home).unwrap();
        let settings_path = app_state.join("settings.json");
        let catalog_state_path = app_state.join("model-catalog-state.json");
        fs::write(&settings_path, serde_json::to_vec_pretty(settings).unwrap()).unwrap();
        fs::write(
            &catalog_state_path,
            serde_json::to_vec_pretty(state).unwrap(),
        )
        .unwrap();
        fs::write(codex_home.join("config.toml"), "model = \"live-before\"\n").unwrap();
        fs::write(
            codex_home.join("auth.json"),
            official_auth_bytes("account-a", "workspace-a"),
        )
        .unwrap();
        Self {
            _temp: temp,
            paths: ProviderCommitPaths {
                app_state,
                codex_home,
                settings_path,
                catalog_state_path,
                current_target: Some(target_identity("0.147.0", "target-a")),
            },
        }
    }

    fn read_settings(&self) -> BackendSettings {
        let mut settings: BackendSettings =
            serde_json::from_slice(&fs::read(&self.paths.settings_path).unwrap()).unwrap();
        for profile in &mut settings.relay_profiles {
            codex_plus_core::relay_config::normalize_relay_profile_for_storage(profile).unwrap();
            if profile.relay_mode == RelayMode::PureApi {
                let mut document = profile
                    .config_contents
                    .parse::<toml_edit::DocumentMut>()
                    .unwrap();
                let provider_id = document["model_provider"].as_str().unwrap().to_string();
                document["model_providers"][&provider_id]["experimental_bearer_token"] =
                    toml_edit::value(profile.api_key.trim());
                document["model_providers"][&provider_id]["requires_openai_auth"] =
                    toml_edit::value(false);
                profile.config_contents = document.to_string();
            }
            profile.auth_contents.clear();
        }
        settings
    }

    fn read_state(&self) -> CatalogState {
        serde_json::from_slice(&fs::read(&self.paths.catalog_state_path).unwrap()).unwrap()
    }

    fn file_generation(&self) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::new();
        collect_files(&self.paths.app_state, "app-state", &mut files);
        collect_files(&self.paths.codex_home, "codex-home", &mut files);
        files
    }
}

fn collect_files(root: &Path, prefix: &str, files: &mut BTreeMap<String, Vec<u8>>) {
    let mut entries = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap().to_string_lossy();
        let key = format!("{prefix}/{relative}");
        if path.is_dir() {
            collect_files(&path, &key, files);
        } else {
            files.insert(key, fs::read(path).unwrap());
        }
    }
}

#[cfg(unix)]
fn assert_owner_only_dir(path: &Path) {
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o700,
        "directory is not owner-only: {}",
        path.display()
    );
}

#[cfg(unix)]
fn assert_owner_only_file(path: &Path) {
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600,
        "file is not owner-only: {}",
        path.display()
    );
}

fn assert_directory_has_no_entries(path: &Path) {
    if path.exists() {
        assert!(
            fs::read_dir(path).unwrap().next().is_none(),
            "private staging residue remains: {}",
            path.display()
        );
    }
}

fn rich_live_config() -> &'static str {
    r#"model = "live-before"
review_model = "live-review"
model_reasoning_effort = "xhigh"
disable_response_storage = true
network_access = "enabled"
windows_wsl_setup_acknowledged = true
sandbox_mode = "workspace-write"

[features]
goals = false
shell_snapshot = true

[mcp_servers.memory]
command = "memory-server"
args = ["--live"]

[skills.writer]
path = "/live/skills/writer"
enabled = true

[plugins.browser]
enabled = true
"#
}

fn semantic_context_tables(config: &str) -> BTreeMap<String, serde_json::Value> {
    let document: serde_json::Value = toml_edit::de::from_str(config).unwrap();
    ["mcp_servers", "skills", "plugins"]
        .into_iter()
        .map(|name| {
            (
                name.to_string(),
                document
                    .get(name)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )
        })
        .collect()
}

fn unrelated_live_semantics(config: &str) -> BTreeMap<String, serde_json::Value> {
    let document: serde_json::Value = toml_edit::de::from_str(config).unwrap();
    document
        .as_object()
        .unwrap()
        .iter()
        .filter(|(name, _)| {
            !matches!(
                name.as_str(),
                "model"
                    | "model_provider"
                    | "model_catalog_json"
                    | "base_url"
                    | "OPENAI_API_KEY"
                    | "model_context_window"
                    | "model_auto_compact_token_limit"
                    | "codex_plus_chat_base_url"
                    | "model_providers"
            )
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

fn selected_provider_field(config: &str, field: &str) -> String {
    let document = config.parse::<toml_edit::DocumentMut>().unwrap();
    let provider_id = document["model_provider"].as_str().unwrap();
    document["model_providers"][provider_id][field]
        .as_str()
        .unwrap()
        .to_string()
}

fn raw_stored_profile_config(settings_path: &Path, profile_id: &str) -> String {
    let settings: BackendSettings =
        serde_json::from_slice(&fs::read(settings_path).unwrap()).unwrap();
    settings
        .relay_profiles
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .unwrap()
        .config_contents
}

#[test]
fn successful_active_commit_preserves_context_semantics_and_auth_bytes() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let context_before = semantic_context_tables(rich_live_config());
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();
    let mut next = persisted.clone();
    next.relay_profiles[0] = canonical_profile(
        "sub2api",
        "official-a",
        "https://changed.example/v1",
        "changed-provider-key",
    );
    let mut context_verifications = 0;

    commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 56),
        |checkpoint| {
            if checkpoint == ProviderCommitCheckpoint::ContextVerification {
                context_verifications += 1;
            }
            Ok(())
        },
    )
    .unwrap();

    let live = fs::read_to_string(fixture.paths.codex_home.join("config.toml")).unwrap();
    assert_eq!(context_verifications, 1);
    assert_eq!(semantic_context_tables(&live), context_before);
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

#[test]
fn managed_context_cleanup_requires_confirmation_and_commits_settings_and_live_atomically() {
    let mut structured_only = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    structured_only.context_window = "272000".to_string();
    structured_only.auto_compact_limit = "240000".to_string();
    let mut raw_only = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    raw_only.config_contents = format!(
        "model_context_window = 272000\nmodel_auto_compact_token_limit = 240000\n{}",
        raw_only.config_contents
    );
    let mut raw_and_structured = structured_only.clone();
    raw_and_structured.config_contents = format!(
        "model_context_window = 272000\nmodel_auto_compact_token_limit = 240000\n{}",
        raw_and_structured.config_contents
    );
    for active in [
        structured_only.clone(),
        raw_only.clone(),
        raw_and_structured,
    ] {
        let rejected = Fixture::new(
            &settings_with(vec![active], "sub2api"),
            &state_with_official(),
        );
        let persisted = rejected.read_settings();
        let before = rejected.file_generation();
        let error = commit_provider_detail_from_paths(
            &rejected.paths,
            request(
                &persisted,
                &persisted,
                "sub2api",
                ProviderCommitAction::Save,
                57,
            ),
        )
        .unwrap_err();
        assert_eq!(error.code(), ProviderCommitErrorCode::InvalidDraft);
        assert_eq!(rejected.file_generation(), before);
    }

    let initial = settings_with(vec![structured_only], "sub2api");
    let confirmed = Fixture::new(&initial, &state_with_official());
    fs::write(
        confirmed.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let context_before = semantic_context_tables(rich_live_config());
    let auth_before = fs::read(confirmed.paths.codex_home.join("auth.json")).unwrap();
    let persisted = confirmed.read_settings();
    let mut confirmed_request = request(
        &persisted,
        &persisted,
        "sub2api",
        ProviderCommitAction::Save,
        58,
    );
    confirmed_request.confirm_context_cleanup = true;

    commit_provider_detail_from_paths(&confirmed.paths, confirmed_request).unwrap();

    let saved = confirmed.read_settings();
    let saved_profile = &saved.relay_profiles[0];
    assert!(saved_profile.context_window.is_empty());
    assert!(saved_profile.auto_compact_limit.is_empty());
    assert!(
        !saved_profile
            .config_contents
            .contains("model_context_window")
    );
    assert!(
        !saved_profile
            .config_contents
            .contains("model_auto_compact_token_limit")
    );
    let live = fs::read_to_string(confirmed.paths.codex_home.join("config.toml")).unwrap();
    assert!(!live.contains("model_context_window"));
    assert!(!live.contains("model_auto_compact_token_limit"));
    assert_eq!(semantic_context_tables(&live), context_before);
    assert_eq!(
        fs::read(confirmed.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );

    let confirmed_raw = Fixture::new(
        &settings_with(vec![raw_only], "sub2api"),
        &state_with_official(),
    );
    let persisted = confirmed_raw.read_settings();
    let mut confirmed_request = request(
        &persisted,
        &persisted,
        "sub2api",
        ProviderCommitAction::Save,
        61,
    );
    confirmed_request.confirm_context_cleanup = true;
    commit_provider_detail_from_paths(&confirmed_raw.paths, confirmed_request).unwrap();
    let saved = confirmed_raw.read_settings();
    assert!(
        !saved.relay_profiles[0]
            .config_contents
            .contains("model_context_window")
    );
    assert!(
        !saved.relay_profiles[0]
            .config_contents
            .contains("model_auto_compact_token_limit")
    );
    let live = fs::read_to_string(confirmed_raw.paths.codex_home.join("config.toml")).unwrap();
    assert!(!live.contains("model_context_window"));
    assert!(!live.contains("model_auto_compact_token_limit"));
}

#[test]
fn managed_context_cleanup_detects_live_only_conflicts_before_active_commit() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let live_with_limits = format!(
        "model_context_window = 272000\nmodel_auto_compact_token_limit = 240000\n{}",
        rich_live_config()
    );

    let rejected = Fixture::new(&initial, &state_with_official());
    fs::write(
        rejected.paths.codex_home.join("config.toml"),
        &live_with_limits,
    )
    .unwrap();
    let persisted = rejected.read_settings();
    let before = rejected.file_generation();
    let error = commit_provider_detail_from_paths(
        &rejected.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            59,
        ),
    )
    .unwrap_err();
    assert_eq!(error.code(), ProviderCommitErrorCode::InvalidDraft);
    assert_eq!(rejected.file_generation(), before);

    let confirmed = Fixture::new(&initial, &state_with_official());
    fs::write(
        confirmed.paths.codex_home.join("config.toml"),
        &live_with_limits,
    )
    .unwrap();
    let context_before = semantic_context_tables(&live_with_limits);
    let auth_before = fs::read(confirmed.paths.codex_home.join("auth.json")).unwrap();
    let persisted = confirmed.read_settings();
    let mut confirmed_request = request(
        &persisted,
        &persisted,
        "sub2api",
        ProviderCommitAction::Save,
        60,
    );
    confirmed_request.confirm_context_cleanup = true;

    commit_provider_detail_from_paths(&confirmed.paths, confirmed_request).unwrap();

    let live = fs::read_to_string(confirmed.paths.codex_home.join("config.toml")).unwrap();
    assert!(!live.contains("model_context_window"));
    assert!(!live.contains("model_auto_compact_token_limit"));
    assert_eq!(semantic_context_tables(&live), context_before);
    assert_eq!(
        fs::read(confirmed.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

#[test]
fn injected_normalization_failure_preserves_the_complete_prior_generation() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let before = fixture.file_generation();

    let error = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            50,
        ),
        |checkpoint| {
            if checkpoint == ProviderCommitCheckpoint::Normalization {
                anyhow::bail!("normalization-fault-sentinel");
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::InvalidDraft);
    assert!(!error.to_string().contains("normalization-fault-sentinel"));
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn injected_catalog_materialization_failure_preserves_the_complete_prior_generation() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let before = fixture.file_generation();

    let error = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            51,
        ),
        |checkpoint| {
            if checkpoint == ProviderCommitCheckpoint::CatalogMaterialization {
                anyhow::bail!("catalog-fault-sentinel");
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::CatalogUnavailable);
    assert!(!error.to_string().contains("catalog-fault-sentinel"));
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn injected_settings_persistence_failure_rolls_back_the_complete_prior_generation() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let before = fixture.file_generation();
    let mut next = persisted.clone();
    next.relay_profiles[0] = canonical_profile(
        "sub2api",
        "official-a",
        "https://changed.example/v1",
        "changed-provider-key",
    );

    let error = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 52),
        |checkpoint| {
            if checkpoint == ProviderCommitCheckpoint::SettingsPersistence {
                anyhow::bail!("settings-fault-sentinel");
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::TransactionFailed);
    assert!(!error.to_string().contains("settings-fault-sentinel"));
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn injected_live_config_commit_failure_rolls_back_the_complete_prior_generation() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let before = fixture.file_generation();
    let mut next = persisted.clone();
    next.relay_profiles[0] = canonical_profile(
        "sub2api",
        "official-a",
        "https://changed.example/v1",
        "changed-provider-key",
    );

    let error = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 53),
        |checkpoint| {
            if checkpoint == ProviderCommitCheckpoint::LiveConfigCommit {
                anyhow::bail!("live-config-fault-sentinel");
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::TransactionFailed);
    assert!(!error.to_string().contains("live-config-fault-sentinel"));
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn injected_context_verification_failure_rolls_back_the_complete_prior_generation() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let context_before = semantic_context_tables(rich_live_config());
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();
    let before = fixture.file_generation();
    let mut next = persisted.clone();
    next.relay_profiles[0] = canonical_profile(
        "sub2api",
        "official-a",
        "https://changed.example/v1",
        "changed-provider-key",
    );

    let error = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 54),
        |checkpoint| {
            if checkpoint == ProviderCommitCheckpoint::ContextVerification {
                anyhow::bail!("context-fault-sentinel");
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::TransactionFailed);
    assert!(!error.to_string().contains("context-fault-sentinel"));
    assert_eq!(fixture.file_generation(), before);
    let live = fs::read_to_string(fixture.paths.codex_home.join("config.toml")).unwrap();
    assert_eq!(semantic_context_tables(&live), context_before);
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

#[test]
fn injected_post_commit_verification_failure_rolls_back_the_complete_prior_generation() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let before = fixture.file_generation();
    let mut next = persisted.clone();
    next.relay_profiles[0] = canonical_profile(
        "sub2api",
        "official-a",
        "https://changed.example/v1",
        "changed-provider-key",
    );

    let error = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 55),
        |checkpoint| {
            if checkpoint == ProviderCommitCheckpoint::PostCommitVerification {
                anyhow::bail!("post-commit-fault-sentinel");
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::TransactionFailed);
    assert!(!error.to_string().contains("post-commit-fault-sentinel"));
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn concurrent_official_auth_update_is_preserved_while_manager_targets_roll_back() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let mut manager_before = fixture.file_generation();
    manager_before.remove("codex-home/auth.json").unwrap();
    let mut next = persisted.clone();
    next.relay_profiles[0] = canonical_profile(
        "sub2api",
        "official-a",
        "https://changed.example/v1",
        "changed-provider-key",
    );
    let auth_path = fixture.paths.codex_home.join("auth.json");
    let newer_auth = b"official-auth-newer".to_vec();
    let mut auth_updates = 0;

    let error = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 57),
        |checkpoint| {
            if checkpoint == ProviderCommitCheckpoint::AuthGenerationVerification {
                fs::write(&auth_path, &newer_auth)?;
                auth_updates += 1;
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::TransactionFailed);
    assert!(!error.to_string().contains("official-auth-newer"));
    assert_eq!(auth_updates, 1);
    assert_eq!(fs::read(&auth_path).unwrap(), newer_auth);
    let mut manager_after = fixture.file_generation();
    manager_after.remove("codex-home/auth.json").unwrap();
    assert_eq!(manager_after, manager_before);
}

#[test]
fn auth_update_after_scope_gate_cannot_become_the_commit_baseline() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let mut manager_before = fixture.file_generation();
    manager_before.remove("codex-home/auth.json").unwrap();
    let mut next = persisted.clone();
    next.relay_profiles[0] = canonical_profile(
        "sub2api",
        "official-a",
        "https://changed.example/v1",
        "changed-provider-key",
    );
    let auth_path = fixture.paths.codex_home.join("auth.json");
    let newer_auth = official_auth_bytes("account-b", "workspace-b");

    let error = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 68),
        |checkpoint| {
            if checkpoint == ProviderCommitCheckpoint::ActivationScopeVerification {
                fs::write(&auth_path, &newer_auth)?;
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::TransactionFailed);
    assert_eq!(fs::read(&auth_path).unwrap(), newer_auth);
    let mut manager_after = fixture.file_generation();
    manager_after.remove("codex-home/auth.json").unwrap();
    assert_eq!(manager_after, manager_before);
}

#[cfg(unix)]
#[test]
fn real_transaction_keeps_secret_stages_private_and_cleans_faulted_recovery_material() {
    const PRIOR_KEY: &str = "provider-key-before-sentinel";
    const NEXT_KEY: &str = "provider-key-after-sentinel";
    const OAUTH_SENTINEL: &str = "oauth-refresh-sentinel";

    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        PRIOR_KEY,
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let auth_path = fixture.paths.codex_home.join("auth.json");
    let mut auth: Value = serde_json::from_slice(&fs::read(&auth_path).unwrap()).unwrap();
    auth["tokens"]["refresh_token"] = Value::String(OAUTH_SENTINEL.to_string());
    fs::write(&auth_path, serde_json::to_vec_pretty(&auth).unwrap()).unwrap();

    let persisted = fixture.read_settings();
    let before = fixture.file_generation();
    let mut next = persisted.clone();
    next.relay_profiles[0] = canonical_profile(
        "sub2api",
        "official-a",
        "https://changed.example/v1",
        NEXT_KEY,
    );
    let app_state = fixture.paths.app_state.clone();
    let codex_home = fixture.paths.codex_home.clone();
    let mut inspected_real_journal = false;

    let error = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 69),
        |checkpoint| {
            if checkpoint != ProviderCommitCheckpoint::SettingsPersistence {
                return Ok(());
            }
            inspected_real_journal = true;
            let journal_path = app_state.join("live-state-transaction.json");
            let transaction_root = app_state.join("live-state-transactions");
            let journal = fs::read_to_string(&journal_path)?;
            assert!(!journal.contains(PRIOR_KEY));
            assert!(!journal.contains(NEXT_KEY));
            assert!(!journal.contains(OAUTH_SENTINEL));
            assert_owner_only_file(&journal_path);
            assert_owner_only_dir(&app_state);
            assert_owner_only_dir(&transaction_root);

            let transaction_dirs = fs::read_dir(&transaction_root)?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(transaction_dirs.len(), 1);
            let transaction_dir = &transaction_dirs[0];
            assert_owner_only_dir(transaction_dir);

            let mut key_bearing_stages = 0;
            for entry in fs::read_dir(transaction_dir)? {
                let stage_path = entry?.path();
                assert_owner_only_file(&stage_path);
                let bytes = fs::read(&stage_path)?;
                assert!(
                    !bytes
                        .windows(OAUTH_SENTINEL.len())
                        .any(|window| window == OAUTH_SENTINEL.as_bytes())
                );
                if bytes
                    .windows(PRIOR_KEY.len())
                    .any(|window| window == PRIOR_KEY.as_bytes())
                    || bytes
                        .windows(NEXT_KEY.len())
                        .any(|window| window == NEXT_KEY.as_bytes())
                {
                    key_bearing_stages += 1;
                }
            }
            assert!(key_bearing_stages >= 2);

            for root in [&app_state, &codex_home] {
                let mut artifacts = BTreeMap::new();
                collect_files(root, "artifact", &mut artifacts);
                for (name, bytes) in artifacts {
                    if name.ends_with("auth.json") {
                        continue;
                    }
                    assert!(
                        !bytes
                            .windows(OAUTH_SENTINEL.len())
                            .any(|window| window == OAUTH_SENTINEL.as_bytes()),
                        "OAuth payload escaped into {name}"
                    );
                }
            }
            anyhow::bail!("artifact-audit-fault")
        },
    )
    .unwrap_err();

    assert!(inspected_real_journal);
    assert_eq!(error.code(), ProviderCommitErrorCode::TransactionFailed);
    assert!(!error.to_string().contains(PRIOR_KEY));
    assert!(!error.to_string().contains(NEXT_KEY));
    assert!(!error.to_string().contains(OAUTH_SENTINEL));
    assert_eq!(fixture.file_generation(), before);
    assert!(
        !fixture
            .paths
            .app_state
            .join("live-state-transaction.json")
            .exists()
    );
    assert_directory_has_no_entries(&fixture.paths.app_state.join("live-state-transactions"));
    assert_directory_has_no_entries(&fixture.paths.app_state.join("private-staging"));
}

#[test]
fn generic_settings_save_allows_unrelated_changes_but_rejects_every_provider_owned_difference() {
    let first = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let second = canonical_profile(
        "backup",
        "official-a",
        "https://backup.example/v1",
        "backup-key",
    );
    let initial = settings_with(vec![first, second], "sub2api");

    for mutate in [
        |settings: &mut BackendSettings| settings.relay_profiles_enabled = false,
        |settings: &mut BackendSettings| settings.relay_profiles.reverse(),
        |settings: &mut BackendSettings| {
            let mut copy = settings.relay_profiles[0].clone();
            copy.id = "copy".to_string();
            settings.relay_profiles.push(copy);
        },
        |settings: &mut BackendSettings| {
            settings.relay_profiles.pop();
        },
        |settings: &mut BackendSettings| settings.active_relay_id = "backup".to_string(),
        |settings: &mut BackendSettings| {
            settings.relay_profiles[0].base_url = "https://bypass.example/v1".to_string()
        },
        |settings: &mut BackendSettings| {
            settings.relay_test_model = "bypass-test-model".to_string()
        },
    ] as [fn(&mut BackendSettings); 7]
    {
        let fixture = Fixture::new(&initial, &state_with_official());
        let persisted: BackendSettings =
            serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
        let before = fixture.file_generation();
        let mut bypass = persisted.clone();
        mutate(&mut bypass);

        let error = save_settings_with_provider_guard_at(&fixture.paths, bypass).unwrap_err();

        assert_eq!(error, GenericSettingsSaveError::ProviderOwnedDifference);
        assert!(error.to_string().contains("provider-owned"));
        assert_eq!(fixture.file_generation(), before);
    }

    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted: BackendSettings =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    let provider_before = ProviderOwnedTopologyDraft::from_settings(&persisted);
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
    let mut unrelated = persisted;
    unrelated.codex_goals_enabled = !unrelated.codex_goals_enabled;

    save_settings_with_provider_guard_at(&fixture.paths, unrelated.clone()).unwrap();

    let saved: BackendSettings =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(ProviderOwnedTopologyDraft::from_settings(&saved)).unwrap(),
        serde_json::to_value(provider_before).unwrap()
    );
    assert_eq!(saved.codex_goals_enabled, unrelated.codex_goals_enabled);
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        live_before
    );
}

#[test]
fn generic_settings_save_accepts_real_ui_derived_provider_shape_for_unrelated_changes() {
    let first = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let second = pure_oauth_profile("official");
    let initial = settings_with(vec![first, second], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted_before: BackendSettings =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    let provider_before = ProviderOwnedTopologyDraft::from_settings(&persisted_before);

    // Match the real load -> Tauri serialization -> TypeScript normalize path:
    // core load derives the structured provider fields from configContents, then
    // normalizeSettings initializes the context selection and synchronizes the
    // legacy active-provider fields before an unrelated setting is saved.
    let mut ui_round_trip = SettingsStore::new(fixture.paths.settings_path.clone())
        .load()
        .unwrap();
    for profile in &mut ui_round_trip.relay_profiles {
        if profile.relay_mode == RelayMode::Official && !profile.official_mix_api_key {
            profile.model.clear();
            profile.base_url.clear();
            profile.upstream_base_url.clear();
            profile.api_key.clear();
            profile.config_contents.clear();
        }
        if !profile.context_selection_initialized {
            profile.context_selection_initialized = true;
        }
    }
    let active = ui_round_trip
        .relay_profiles
        .iter()
        .find(|profile| profile.id == ui_round_trip.active_relay_id)
        .unwrap();
    ui_round_trip.relay_base_url = active.base_url.clone();
    ui_round_trip.relay_api_key = active.api_key.clone();
    ui_round_trip.codex_goals_enabled = !ui_round_trip.codex_goals_enabled;

    save_settings_with_provider_guard_at(&fixture.paths, ui_round_trip.clone()).unwrap();

    let saved: BackendSettings =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(ProviderOwnedTopologyDraft::from_settings(&saved)).unwrap(),
        serde_json::to_value(provider_before).unwrap()
    );
    assert_eq!(saved.codex_goals_enabled, ui_round_trip.codex_goals_enabled);
}

#[test]
fn generic_settings_save_uses_default_provider_baseline_when_settings_file_is_absent() {
    let fixture = Fixture::new(&BackendSettings::default(), &state_with_official());
    fs::remove_file(&fixture.paths.settings_path).unwrap();
    let mut first_save = BackendSettings::default();
    first_save.codex_goals_enabled = true;

    save_settings_with_provider_guard_at(&fixture.paths, first_save).unwrap();

    let saved: BackendSettings =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    assert!(saved.codex_goals_enabled);
    assert_eq!(
        serde_json::to_value(ProviderOwnedTopologyDraft::from_settings(&saved)).unwrap(),
        serde_json::to_value(ProviderOwnedTopologyDraft::from_settings(
            &BackendSettings::default()
        ))
        .unwrap()
    );
}

#[test]
fn generic_settings_save_accepts_first_run_ui_shape_and_active_aggregate_ui_projection() {
    let first_run = Fixture::new(&BackendSettings::default(), &state_with_official());
    fs::remove_file(&first_run.paths.settings_path).unwrap();
    let mut first_run_ui = BackendSettings::default();
    first_run_ui.relay_profiles[0].context_selection_initialized = true;
    first_run_ui.relay_base_url = first_run_ui.relay_profiles[0].base_url.clone();
    first_run_ui.relay_api_key = first_run_ui.relay_profiles[0].api_key.clone();
    first_run_ui.active_aggregate_relay_id.clear();
    first_run_ui.codex_goals_enabled = true;
    save_settings_with_provider_guard_at(&first_run.paths, first_run_ui).unwrap();

    let api = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let aggregate = RelayProfile {
        id: "aggregate".to_string(),
        name: "Aggregate".to_string(),
        relay_mode: RelayMode::Aggregate,
        protocol: RelayProtocol::ChatCompletions,
        official_mix_api_key: true,
        context_window: "123456".to_string(),
        auto_compact_limit: "100000".to_string(),
        model_list: "stale-model".to_string(),
        ..RelayProfile::default()
    };
    let mut initial = settings_with(vec![api, aggregate], "aggregate");
    initial.aggregate_relay_profiles = vec![AggregateRelayProfile {
        id: "aggregate".to_string(),
        name: "Aggregate".to_string(),
        strategy: AggregateRelayStrategy::Failover,
        members: vec![AggregateRelayMember {
            relay_id: "sub2api".to_string(),
            weight: 1,
        }],
    }];
    let fixture = Fixture::new(&initial, &state_with_official());
    let mut persisted_value: Value =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    persisted_value["relayProfiles"][1]["model"] = json!("aggregate-model");
    fs::write(
        &fixture.paths.settings_path,
        serde_json::to_vec_pretty(&persisted_value).unwrap(),
    )
    .unwrap();
    let raw_before = fs::read(&fixture.paths.settings_path).unwrap();
    let mut ui_round_trip = SettingsStore::new(fixture.paths.settings_path.clone())
        .load()
        .unwrap();
    for profile in &mut ui_round_trip.relay_profiles {
        if profile.relay_mode == RelayMode::Aggregate {
            profile.base_url.clear();
            profile.upstream_base_url.clear();
            profile.api_key.clear();
            profile.protocol = RelayProtocol::Responses;
            profile.official_mix_api_key = false;
            profile.config_contents.clear();
            profile.auth_contents.clear();
            profile.context_window.clear();
            profile.auto_compact_limit.clear();
            profile.model_list.clear();
            profile.model_windows.clear();
        }
        profile.context_selection_initialized = true;
    }
    ui_round_trip.relay_base_url = "http://127.0.0.1:57321/v1".to_string();
    ui_round_trip.relay_api_key.clear();
    ui_round_trip.active_aggregate_relay_id = "aggregate".to_string();
    ui_round_trip.codex_goals_enabled = true;

    save_settings_with_provider_guard_at(&fixture.paths, ui_round_trip).unwrap();

    let saved = fs::read(&fixture.paths.settings_path).unwrap();
    let raw_topology: BackendSettings = serde_json::from_slice(&raw_before).unwrap();
    let saved_settings: BackendSettings = serde_json::from_slice(&saved).unwrap();
    assert_eq!(
        serde_json::to_value(ProviderOwnedTopologyDraft::from_settings(&saved_settings)).unwrap(),
        serde_json::to_value(ProviderOwnedTopologyDraft::from_settings(&raw_topology)).unwrap()
    );
    assert!(saved_settings.codex_goals_enabled);
}

#[test]
fn generic_settings_save_rejects_a_concurrent_persisted_provider_generation_change() {
    let initial = settings_with(
        vec![canonical_profile(
            "sub2api",
            "official-a",
            "https://relay.example/v1",
            "provider-key",
        )],
        "sub2api",
    );
    let fixture = Fixture::new(&initial, &state_with_official());
    let mut incoming: BackendSettings =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    incoming.codex_goals_enabled = true;
    let mut concurrent = initial;
    concurrent.relay_profiles[0].config_contents = concurrent.relay_profiles[0]
        .config_contents
        .replace("https://relay.example/v1", "https://concurrent.example/v1");
    let concurrent_bytes = serde_json::to_vec_pretty(&concurrent).unwrap();
    let settings_path = fixture.paths.settings_path.clone();

    let error = save_settings_with_provider_guard_at_observed(&fixture.paths, incoming, || {
        fs::write(&settings_path, &concurrent_bytes)?;
        Ok(())
    })
    .unwrap_err();

    assert_eq!(error, GenericSettingsSaveError::PersistedSettingsChanged);
    assert_eq!(fs::read(settings_path).unwrap(), concurrent_bytes);
}

#[test]
fn direct_invoke_matrix_rejects_provider_bypasses_without_mutating_any_generation() {
    let first = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let second = canonical_profile(
        "backup",
        "official-a",
        "https://backup.example/v1",
        "backup-key",
    );
    let initial = settings_with(vec![first, second], "sub2api");

    for case in [
        "invalid-provider-toml",
        "actor-header",
        "provider-bearer",
        "auth-contents",
        "active-id",
        "topology-order",
        "common-config",
        "context-config",
        "legacy-base-url",
        "test-model",
    ] {
        let fixture = Fixture::new(&initial, &state_with_official());
        let mut incoming: BackendSettings =
            serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
        let before = fixture.file_generation();
        match case {
            "invalid-provider-toml" => {
                incoming.relay_profiles[0].config_contents =
                    "model_provider = [invalid-provider-toml".to_string();
            }
            "actor-header" => {
                incoming.relay_profiles[0].config_contents = incoming.relay_profiles[0]
                    .config_contents
                    .replace("local-image-extension", "actor-bypass-sentinel");
            }
            "provider-bearer" => {
                incoming.relay_profiles[0].api_key = "provider-bypass-sentinel".to_string();
                incoming.relay_profiles[0].config_contents = incoming.relay_profiles[0]
                    .config_contents
                    .replace("provider-key", "provider-bypass-sentinel");
            }
            "auth-contents" => {
                incoming.relay_profiles[0].auth_contents =
                    r#"{"tokens":{"access_token":"oauth-bypass-sentinel"}}"#.to_string();
            }
            "active-id" => incoming.active_relay_id = "backup".to_string(),
            "topology-order" => incoming.relay_profiles.reverse(),
            "common-config" => {
                incoming.relay_common_config_contents =
                    "sandbox_mode = \"danger-full-access\"\n".to_string();
            }
            "context-config" => {
                incoming.relay_context_config_contents =
                    "[mcp_servers.bypass]\ncommand = \"secret-command\"\n".to_string();
            }
            "legacy-base-url" => {
                incoming.relay_base_url = "https://legacy-bypass.example/v1".to_string();
            }
            "test-model" => incoming.relay_test_model = "bypass-model".to_string(),
            _ => unreachable!(),
        }

        let error = save_settings_with_provider_guard_at(&fixture.paths, incoming).unwrap_err();

        let expected = if case == "auth-contents" {
            GenericSettingsSaveError::ProviderAuthProhibited
        } else {
            GenericSettingsSaveError::ProviderOwnedDifference
        };
        assert_eq!(error, expected, "unexpected error for {case}");
        assert!(!error.to_string().contains("provider-bypass-sentinel"));
        assert!(!error.to_string().contains("oauth-bypass-sentinel"));
        assert!(!error.to_string().contains("legacy-bypass.example"));
        assert_eq!(
            fixture.file_generation(),
            before,
            "mutation leaked for {case}"
        );
    }

    for context in ["active-detail", "inactive-detail", "topology"] {
        let active = canonical_profile(
            "sub2api",
            "official-a",
            "https://stale-provider.example/v1",
            "stale-provider-key",
        );
        let inactive = canonical_profile(
            "backup",
            "official-a",
            "https://inactive.example/v1",
            "inactive-provider-key",
        );
        let persisted = settings_with(vec![active, inactive], "sub2api");
        let fixture = Fixture::new(&persisted, &state_with_official());
        let persisted = fixture.read_settings();
        let mut stale = request(
            &persisted,
            &persisted,
            if context == "inactive-detail" {
                "backup"
            } else {
                "sub2api"
            },
            ProviderCommitAction::Save,
            u64::MAX,
        );
        if context == "topology" {
            stale.focused_profile_id = None;
            stale.catalog_drafts.clear();
        }
        stale.expected_provider_fingerprint = "sha256:stale-direct-invoke".to_string();
        let before = fixture.file_generation();

        let error = commit_provider_detail_from_paths(&fixture.paths, stale).unwrap_err();

        assert_eq!(error.code(), ProviderCommitErrorCode::StaleState);
        assert_eq!(
            error.to_string(),
            "provider state changed; reload or merge before saving"
        );
        assert!(!error.to_string().contains("stale-provider-key"));
        assert!(!error.to_string().contains("stale-provider.example"));
        assert_eq!(
            fixture.file_generation(),
            before,
            "stale {context} mutated a generation"
        );
    }
}

#[test]
fn provider_commit_rejects_a_concurrent_raw_settings_generation_change() {
    let active = pure_oauth_profile("official");
    let inactive = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active, inactive], "official");
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();
    let commit = request(
        &persisted,
        &persisted,
        "sub2api",
        ProviderCommitAction::Save,
        999,
    );
    let mut concurrent: Value =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    concurrent["codexGoalsEnabled"] = json!(true);
    concurrent["concurrentRawGenerationSentinel"] = json!("must-survive");
    let concurrent_bytes = serde_json::to_vec_pretty(&concurrent).unwrap();
    let settings_path = fixture.paths.settings_path.clone();
    let mut expected = fixture.file_generation();
    expected.insert(
        "app-state/settings.json".to_string(),
        concurrent_bytes.clone(),
    );

    let error = commit_provider_detail_from_paths_observed(&fixture.paths, commit, |checkpoint| {
        if checkpoint == ProviderCommitCheckpoint::CatalogMaterialization {
            fs::write(&settings_path, &concurrent_bytes)?;
        }
        Ok(())
    })
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::StaleState);
    assert_eq!(
        error.to_string(),
        "provider settings changed during commit; reload or merge before saving"
    );
    assert_eq!(fixture.file_generation(), expected);
}

#[test]
fn topology_adapter_commits_list_mutations_through_the_shared_provider_transaction() {
    let a = canonical_profile(
        "relay-a",
        "official-a",
        "https://a.example/v1",
        "provider-key-a",
    );
    let b = canonical_profile(
        "relay-b",
        "official-a",
        "https://b.example/v1",
        "provider-key-b",
    );
    let mut shadow = b.clone();
    shadow.id = "relay-shadow".to_string();
    shadow.name = "Relay B shadow".to_string();
    let c = canonical_profile(
        "relay-c",
        "official-a",
        "https://c.example/v1",
        "provider-key-c",
    );
    let aggregate = RelayProfile {
        id: "aggregate".to_string(),
        name: "Aggregate".to_string(),
        relay_mode: RelayMode::Aggregate,
        ..RelayProfile::default()
    };
    let mut initial = settings_with(
        vec![a.clone(), shadow, b.clone(), c, aggregate.clone()],
        "relay-a",
    );
    initial.aggregate_relay_profiles = vec![AggregateRelayProfile {
        id: "aggregate".to_string(),
        name: "Aggregate".to_string(),
        strategy: AggregateRelayStrategy::Failover,
        members: vec![
            AggregateRelayMember {
                relay_id: "relay-b".to_string(),
                weight: 2,
            },
            AggregateRelayMember {
                relay_id: "relay-c".to_string(),
                weight: 1,
            },
        ],
    }];
    let mut catalog_state = state_with_official();
    for profile_id in ["relay-a", "relay-b", "relay-c"] {
        catalog_state
            .profiles
            .entry(profile_id.to_string())
            .or_default()
            .mode = CatalogMode::OfficialPlusCustom;
    }
    catalog_state
        .profiles
        .entry("relay-shadow".to_string())
        .or_default()
        .mode = CatalogMode::NativeOfficial;
    let fixture = Fixture::new(&initial, &catalog_state);
    let persisted = fixture.read_settings();
    let persisted_a = persisted
        .relay_profiles
        .iter()
        .find(|profile| profile.id == "relay-a")
        .unwrap()
        .clone();
    let persisted_b = persisted
        .relay_profiles
        .iter()
        .find(|profile| profile.id == "relay-b")
        .unwrap()
        .clone();
    let persisted_aggregate = persisted
        .relay_profiles
        .iter()
        .find(|profile| profile.id == "aggregate")
        .unwrap()
        .clone();
    let mut copy = persisted_b.clone();
    copy.id = "relay-copy".to_string();
    copy.name = "Relay B copy".to_string();
    let mut next = persisted.clone();
    next.relay_profiles = vec![persisted_b, persisted_a, copy, persisted_aggregate];
    next.relay_profiles_enabled = false;
    next.relay_test_model = "topology-test-model".to_string();
    next.aggregate_relay_profiles = vec![AggregateRelayProfile {
        id: "aggregate".to_string(),
        name: "Aggregate".to_string(),
        strategy: AggregateRelayStrategy::Failover,
        members: vec![AggregateRelayMember {
            relay_id: "relay-b".to_string(),
            weight: 2,
        }],
    }];
    let request = ProviderCommitRequest {
        topology: ProviderOwnedTopologyDraft::from_settings(&next),
        catalog_drafts: vec![catalog_draft("relay-copy")],
        focused_profile_id: None,
        action: ProviderCommitAction::Save,
        previous_active_relay_id: persisted.active_relay_id.clone(),
        confirm_context_cleanup: false,
        draft_revision: 41,
        expected_provider_fingerprint: provider_owned_fingerprint(
            &ProviderOwnedTopologyDraft::from_settings(&persisted),
        )
        .unwrap(),
    };
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();

    let payload = commit_provider_detail_from_paths(&fixture.paths, request).unwrap();

    let saved = fixture.read_settings();
    assert_eq!(payload.draft_revision, 41);
    assert!(!payload.restart_required);
    assert!(!saved.relay_profiles_enabled);
    assert_eq!(saved.relay_test_model, "topology-test-model");
    assert_eq!(
        saved
            .relay_profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<Vec<_>>(),
        vec!["relay-b", "relay-a", "relay-copy", "aggregate"]
    );
    assert_eq!(saved.aggregate_relay_profiles[0].members.len(), 1);
    let saved_catalog: CatalogState =
        serde_json::from_slice(&fs::read(&fixture.paths.catalog_state_path).unwrap()).unwrap();
    assert!(!saved_catalog.profiles.contains_key("relay-c"));
    assert!(!saved_catalog.profiles.contains_key("relay-shadow"));
    let copied_catalog = saved_catalog.profiles.get("relay-copy").unwrap();
    assert_eq!(copied_catalog.mode, CatalogMode::OfficialPlusCustom);
    let copied_path = copied_catalog.generated_path.as_deref().unwrap();
    assert!(fixture.paths.codex_home.join(copied_path).is_file());
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        live_before
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

#[test]
fn topology_copy_uses_trusted_catalog_scope_and_saves_stale_copy_action_required() {
    let official = pure_oauth_profile("official");
    let source = canonical_profile(
        "relay-source",
        "official-a",
        "https://source.example/v1",
        "provider-key-source",
    );
    let initial = settings_with(vec![official, source], "official");
    let mut catalog_state = stale_scope_state();
    catalog_state
        .profiles
        .entry("relay-source".to_string())
        .or_default()
        .mode = CatalogMode::OfficialPlusCustom;
    let fixture = Fixture::new(&initial, &catalog_state);
    let persisted = fixture.read_settings();
    let source_state_before = fixture.read_state().profiles["relay-source"].clone();
    let mut copy = persisted
        .relay_profiles
        .iter()
        .find(|profile| profile.id == "relay-source")
        .unwrap()
        .clone();
    copy.id = "relay-copy".to_string();
    copy.name = "Relay copy".to_string();
    let mut next = persisted.clone();
    next.relay_profiles.push(copy);
    let request = ProviderCommitRequest {
        topology: ProviderOwnedTopologyDraft::from_settings(&next),
        catalog_drafts: vec![catalog_draft("relay-copy")],
        focused_profile_id: None,
        action: ProviderCommitAction::Save,
        previous_active_relay_id: persisted.active_relay_id.clone(),
        confirm_context_cleanup: false,
        draft_revision: 72,
        expected_provider_fingerprint: provider_owned_fingerprint(
            &ProviderOwnedTopologyDraft::from_settings(&persisted),
        )
        .unwrap(),
    };
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();

    let payload = commit_provider_detail_from_paths(&fixture.paths, request).unwrap();

    let saved_state = fixture.read_state();
    let copied_state = &saved_state.profiles["relay-copy"];
    assert_eq!(payload.draft_revision, 72);
    assert_eq!(
        copied_state.action_required.as_deref(),
        Some("catalog-readiness-unavailable")
    );
    assert!(copied_state.generated_hash.is_none());
    assert!(copied_state.generated_path.is_none());
    assert!(!copied_state.restart_required);
    assert_eq!(
        serde_json::to_value(&saved_state.profiles["relay-source"]).unwrap(),
        serde_json::to_value(source_state_before).unwrap()
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        live_before
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

#[test]
fn topology_adapter_rejects_active_detail_and_catalog_bypasses_without_mutation() {
    let active = canonical_profile(
        "relay-a",
        "official-a",
        "https://a.example/v1",
        "provider-key-a",
    );
    let initial = settings_with(vec![active], "relay-a");
    let mut catalog_state = state_with_official();
    catalog_state
        .profiles
        .entry("relay-a".to_string())
        .or_default()
        .mode = CatalogMode::OfficialPlusCustom;

    for mutate in ["detail", "catalog"] {
        let fixture = Fixture::new(&initial, &catalog_state);
        let persisted = fixture.read_settings();
        let mut next = persisted.clone();
        let mut catalog_drafts = Vec::new();
        if mutate == "detail" {
            let profile = &mut next.relay_profiles[0];
            profile.model = "changed-model".to_string();
            profile.base_url = "https://changed.example/v1".to_string();
            profile.upstream_base_url = profile.base_url.clone();
            profile.api_key = "changed-provider-key".to_string();
            profile.config_contents = profile
                .config_contents
                .replace("official-a", "changed-model")
                .replace("https://a.example/v1", "https://changed.example/v1")
                .replace("provider-key-a", "changed-provider-key");
        } else {
            let mut draft = catalog_draft("relay-a");
            draft.mode_explicit = true;
            catalog_drafts.push(draft);
        }
        let request = ProviderCommitRequest {
            topology: ProviderOwnedTopologyDraft::from_settings(&next),
            catalog_drafts,
            focused_profile_id: None,
            action: ProviderCommitAction::Save,
            previous_active_relay_id: persisted.active_relay_id.clone(),
            confirm_context_cleanup: false,
            draft_revision: 42,
            expected_provider_fingerprint: provider_owned_fingerprint(
                &ProviderOwnedTopologyDraft::from_settings(&persisted),
            )
            .unwrap(),
        };
        let before = fixture.file_generation();

        let error = commit_provider_detail_from_paths(&fixture.paths, request).unwrap_err();

        assert_eq!(error.code(), ProviderCommitErrorCode::InvalidDraft);
        assert_eq!(fixture.file_generation(), before);
    }
}

#[test]
fn topology_adapter_accepts_one_complete_ui_canonical_generation() {
    let initial = settings_with(
        vec![
            canonical_profile(
                "relay-a",
                "official-a",
                "https://a.example/v1",
                "provider-key-a",
            ),
            canonical_profile(
                "relay-b",
                "official-a",
                "https://b.example/v1",
                "provider-key-b",
            ),
        ],
        "relay-a",
    );
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();
    assert!(!persisted.relay_profiles[0].context_selection_initialized);
    let mut ui_topology = ui_provider_topology_projection(
        settings_snapshot_for_ui_projection(persisted.clone()).unwrap(),
    )
    .unwrap();
    ui_topology.relay_profiles.reverse();
    ui_topology.relay_test_model = "ui-topology-test-model".to_string();
    let request = ProviderCommitRequest {
        topology: ui_topology,
        catalog_drafts: Vec::new(),
        focused_profile_id: None,
        action: ProviderCommitAction::Save,
        previous_active_relay_id: persisted.active_relay_id.clone(),
        confirm_context_cleanup: false,
        draft_revision: 43,
        expected_provider_fingerprint: provider_owned_fingerprint(
            &ProviderOwnedTopologyDraft::from_settings(&persisted),
        )
        .unwrap(),
    };

    let payload = commit_provider_detail_from_paths(&fixture.paths, request).unwrap();

    let saved = fixture.read_settings();
    assert_eq!(payload.draft_revision, 43);
    assert_eq!(saved.relay_test_model, "ui-topology-test-model");
    assert_eq!(
        saved
            .relay_profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<Vec<_>>(),
        vec!["relay-b", "relay-a"]
    );
}

#[test]
fn persisted_legacy_auth_migrates_only_api_key_only_payloads() {
    let active = pure_oauth_profile("official");
    let mut api_only =
        legacy_pure_api_profile("legacy", r#"{"OPENAI_API_KEY":"migrated-provider-key"}"#);
    api_only.api_key.clear();
    api_only.config_contents = api_only
        .config_contents
        .replace("experimental_bearer_token = \"legacy-provider-key\"\n", "");
    let initial = settings_with(vec![active.clone(), api_only], "official");
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();

    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "legacy",
            ProviderCommitAction::Save,
            59,
        ),
    )
    .unwrap();

    let raw_settings = fs::read_to_string(&fixture.paths.settings_path).unwrap();
    assert!(!raw_settings.contains("authContents"));
    assert!(!raw_settings.contains("OPENAI_API_KEY"));
    let migrated: BackendSettings = serde_json::from_str(&raw_settings).unwrap();
    let migrated_profile = migrated
        .relay_profiles
        .iter()
        .find(|profile| profile.id == "legacy")
        .unwrap();
    assert_eq!(
        selected_provider_field(
            &migrated_profile.config_contents,
            "experimental_bearer_token"
        ),
        "migrated-provider-key"
    );

    for forbidden_auth in [
        r#"{"auth_mode":"chatgpt","tokens":{"access_token":"oauth-access-sentinel","refresh_token":"oauth-refresh-sentinel"}}"#,
        r#"{"OPENAI_API_KEY":"provider-key-sentinel","auth_mode":"chatgpt","tokens":{"access_token":"oauth-access-sentinel"}}"#,
    ] {
        let forbidden = legacy_pure_api_profile("legacy", forbidden_auth);
        let initial = settings_with(vec![active.clone(), forbidden], "official");
        let fixture = Fixture::new(&initial, &state_with_official());
        let before = fixture.file_generation();
        let persisted = fixture.read_settings();

        let error = commit_provider_detail_from_paths(
            &fixture.paths,
            request(
                &persisted,
                &persisted,
                "legacy",
                ProviderCommitAction::Save,
                60,
            ),
        )
        .unwrap_err();

        assert_eq!(error.code(), ProviderCommitErrorCode::InputUnavailable);
        assert!(!error.to_string().contains("oauth-access-sentinel"));
        assert!(!error.to_string().contains("provider-key-sentinel"));
        assert_eq!(fixture.file_generation(), before);
    }
}

#[test]
fn active_native_commit_requires_current_official_auth_and_catalog_scope() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");

    let missing_auth = Fixture::new(&initial, &state_with_official());
    fs::remove_file(missing_auth.paths.codex_home.join("auth.json")).unwrap();
    let before = missing_auth.file_generation();
    let persisted = missing_auth.read_settings();
    let error = commit_provider_detail_from_paths(
        &missing_auth.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            61,
        ),
    )
    .unwrap_err();
    assert_eq!(error.code(), ProviderCommitErrorCode::OfficialAuthRequired);
    assert_eq!(missing_auth.file_generation(), before);

    for auth_bytes in [
        b"not-json".to_vec(),
        official_auth_bytes_with_exp("account-a", "workspace-a", 1),
    ] {
        let invalid_auth = Fixture::new(&initial, &state_with_official());
        fs::write(invalid_auth.paths.codex_home.join("auth.json"), auth_bytes).unwrap();
        let before = invalid_auth.file_generation();
        let persisted = invalid_auth.read_settings();
        let error = commit_provider_detail_from_paths(
            &invalid_auth.paths,
            request(
                &persisted,
                &persisted,
                "sub2api",
                ProviderCommitAction::Save,
                66,
            ),
        )
        .unwrap_err();
        assert_eq!(error.code(), ProviderCommitErrorCode::OfficialAuthRequired);
        assert_eq!(invalid_auth.file_generation(), before);
    }

    let missing_everything = Fixture::new(&initial, &CatalogState::default());
    fs::remove_file(missing_everything.paths.codex_home.join("auth.json")).unwrap();
    let before = missing_everything.file_generation();
    let persisted = missing_everything.read_settings();
    let error = commit_provider_detail_from_paths(
        &missing_everything.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            67,
        ),
    )
    .unwrap_err();
    assert_eq!(error.code(), ProviderCommitErrorCode::OfficialAuthRequired);
    assert_eq!(missing_everything.file_generation(), before);

    let set_current_initial = settings_with(
        vec![
            pure_oauth_profile("official"),
            canonical_profile(
                "sub2api",
                "official-a",
                "https://relay.example/v1",
                "provider-key",
            ),
        ],
        "official",
    );
    let missing_auth_set_current = Fixture::new(&set_current_initial, &state_with_official());
    fs::remove_file(missing_auth_set_current.paths.codex_home.join("auth.json")).unwrap();
    let before = missing_auth_set_current.file_generation();
    let persisted = missing_auth_set_current.read_settings();
    let mut next = persisted.clone();
    next.active_relay_id = "sub2api".to_string();
    let error = commit_provider_detail_from_paths(
        &missing_auth_set_current.paths,
        request(
            &persisted,
            &next,
            "sub2api",
            ProviderCommitAction::SetCurrent,
            65,
        ),
    )
    .unwrap_err();
    assert_eq!(error.code(), ProviderCommitErrorCode::OfficialAuthRequired);
    assert_eq!(missing_auth_set_current.file_generation(), before);

    for (account, workspace) in [("account-b", "workspace-a"), ("account-a", "workspace-b")] {
        let stale_scope = Fixture::new(&initial, &state_with_official());
        fs::write(
            stale_scope.paths.codex_home.join("auth.json"),
            official_auth_bytes(account, workspace),
        )
        .unwrap();
        let before = stale_scope.file_generation();
        let persisted = stale_scope.read_settings();
        let error = commit_provider_detail_from_paths(
            &stale_scope.paths,
            request(
                &persisted,
                &persisted,
                "sub2api",
                ProviderCommitAction::Save,
                62,
            ),
        )
        .unwrap_err();
        assert_eq!(error.code(), ProviderCommitErrorCode::CatalogScopeStale);
        assert_eq!(stale_scope.file_generation(), before);
    }

    let mut stale_target = Fixture::new(&initial, &state_with_official());
    stale_target.paths.current_target = Some(target_identity("0.148.0", "target-b"));
    let before = stale_target.file_generation();
    let persisted = stale_target.read_settings();
    let error = commit_provider_detail_from_paths(
        &stale_target.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            63,
        ),
    )
    .unwrap_err();
    assert_eq!(error.code(), ProviderCommitErrorCode::CatalogScopeStale);
    assert_eq!(stale_target.file_generation(), before);

    let inactive = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let inactive_initial =
        settings_with(vec![pure_oauth_profile("official"), inactive], "official");
    let inactive_fixture = Fixture::new(&inactive_initial, &CatalogState::default());
    fs::remove_file(inactive_fixture.paths.codex_home.join("auth.json")).unwrap();
    let persisted = inactive_fixture.read_settings();
    commit_provider_detail_from_paths(
        &inactive_fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            64,
        ),
    )
    .unwrap();
    assert!(
        inactive_fixture
            .read_state()
            .profiles
            .get("sub2api")
            .and_then(|state| state.action_required.as_ref())
            .is_some()
    );
}

#[test]
fn active_catalog_readiness_failures_preserve_the_complete_prior_generation() {
    for (label, catalog_state, model, expected_code) in [
        (
            "missing",
            CatalogState::default(),
            "official-a",
            ProviderCommitErrorCode::CatalogScopeStale,
        ),
        (
            "scope-stale",
            stale_scope_state(),
            "official-a",
            ProviderCommitErrorCode::CatalogScopeStale,
        ),
        (
            "invalid",
            invalid_official_catalog_state(),
            "official-a",
            ProviderCommitErrorCode::CatalogUnavailable,
        ),
        (
            "default-model-absent",
            state_with_official(),
            "missing-model",
            ProviderCommitErrorCode::CatalogUnavailable,
        ),
    ] {
        let active =
            canonical_profile("sub2api", model, "https://relay.example/v1", "provider-key");
        let fixture = Fixture::new(&settings_with(vec![active], "sub2api"), &catalog_state);
        let before = fixture.file_generation();
        let persisted = fixture.read_settings();

        let error = commit_provider_detail_from_paths(
            &fixture.paths,
            request(
                &persisted,
                &persisted,
                "sub2api",
                ProviderCommitAction::Save,
                69,
            ),
        )
        .unwrap_err();

        assert_eq!(error.code(), expected_code, "{label}");
        assert_eq!(fixture.file_generation(), before, "{label}");
    }
}

#[test]
fn active_commit_re_grafts_live_globals_and_rejects_profile_global_injection() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![active], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    fs::write(
        fixture.paths.codex_home.join("config.toml"),
        rich_live_config(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let live_globals_before = unrelated_live_semantics(rich_live_config());
    let mut injected = canonical_profile(
        "sub2api",
        "official-a",
        "https://changed.example/v1",
        "changed-provider-key",
    );
    let mut document = injected
        .config_contents
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    document["review_model"] = toml_edit::value("profile-review-must-not-leak");
    document["model_reasoning_effort"] = toml_edit::value("low");
    document["sandbox_mode"] = toml_edit::value("danger-full-access");
    document["network_access"] = toml_edit::value("disabled");
    document["windows_wsl_setup_acknowledged"] = toml_edit::value(false);
    document["features"]["goals"] = toml_edit::value(true);
    document["profile_only_table"]["leak"] = toml_edit::value("must-not-leak");
    document["mcp_servers"]["intruder"]["command"] = toml_edit::value("must-not-leak");
    injected.config_contents = document.to_string();
    let mut next = persisted.clone();
    next.relay_profiles[0] = injected;

    commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 58),
        |_| Ok(()),
    )
    .unwrap();

    let live = fs::read_to_string(fixture.paths.codex_home.join("config.toml")).unwrap();
    assert_eq!(unrelated_live_semantics(&live), live_globals_before);
    assert!(!live.contains("profile-review-must-not-leak"));
    assert!(!live.contains("must-not-leak"));
    assert_eq!(
        selected_provider_field(&live, "base_url"),
        "https://changed.example/v1"
    );
    assert_eq!(
        selected_provider_field(&live, "experimental_bearer_token"),
        "changed-provider-key"
    );
    let stored_config = raw_stored_profile_config(&fixture.paths.settings_path, "sub2api");
    assert!(unrelated_live_semantics(&stored_config).is_empty());
    assert_eq!(
        selected_provider_field(&stored_config, "base_url"),
        "https://changed.example/v1"
    );
    assert_eq!(
        selected_provider_field(&stored_config, "experimental_bearer_token"),
        "changed-provider-key"
    );
}

#[test]
fn first_and_later_inactive_save_commit_provider_and_catalog_without_live_side_effects() {
    let old = pure_oauth_profile("official");
    let persisted = settings_with(vec![old.clone()], "official");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();

    let first_profile = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key-one",
    );
    let mut first_settings = persisted.clone();
    first_settings.relay_profiles.push(first_profile);
    let first = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &first_settings,
            "sub2api",
            ProviderCommitAction::Save,
            1,
        ),
    )
    .unwrap();
    assert_eq!(first.draft_revision, 1);
    assert_eq!(fixture.read_settings().active_relay_id, "official");
    let first_state = fixture.read_state();
    let first_catalog = &first_state.profiles["sub2api"];
    assert!(first_catalog.action_required.is_none());
    assert!(!first_catalog.restart_required);
    let generated_path = fixture
        .paths
        .codex_home
        .join(first_catalog.generated_path.as_ref().unwrap());
    assert!(generated_path.is_file());

    let mut later_profile = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay-two.example/v1",
        "provider-key-two",
    );
    later_profile.name = "Sub2API updated".to_string();
    let first_persisted = fixture.read_settings();
    let mut later_settings = first_persisted.clone();
    later_settings.relay_profiles[1] = later_profile;
    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &first_persisted,
            &later_settings,
            "sub2api",
            ProviderCommitAction::Save,
            2,
        ),
    )
    .unwrap();
    let saved = fixture.read_settings();
    assert_eq!(saved.active_relay_id, "official");
    assert_eq!(saved.relay_profiles[1].name, "Sub2API updated");
    assert!(!fixture.read_state().profiles["sub2api"].restart_required);
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        live_before
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

#[test]
fn inactive_save_without_catalog_readiness_persists_one_action_required_state() {
    let old = pure_oauth_profile("official");
    let persisted = settings_with(vec![old.clone()], "official");
    let fixture = Fixture::new(&persisted, &CatalogState::default());
    let persisted = fixture.read_settings();
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();
    let mut next = persisted.clone();
    next.relay_profiles.push(canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    ));

    commit_provider_detail_from_paths(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 3),
    )
    .unwrap();

    assert!(
        fixture.read_state().profiles["sub2api"]
            .action_required
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(fixture.read_settings().relay_profiles.len(), 2);
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        live_before
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

fn stale_scope_state() -> CatalogState {
    let mut stale_state = state_with_official();
    let scope_salt = stale_state.scope_salt.clone();
    stale_state.official.as_mut().unwrap().scope_hash =
        official_scope_hash(&scope_salt, "different-account", "workspace-a");
    stale_state
}

fn invalid_official_catalog_state() -> CatalogState {
    let mut invalid_state = state_with_official();
    invalid_state.official.as_mut().unwrap().raw_catalog["models"][0]["visibility"] = json!("hide");
    invalid_state
}

#[test]
fn inactive_catalog_readiness_failures_persist_action_required_without_live_claims() {
    for (label, catalog_state, model) in [
        ("missing", CatalogState::default(), "official-a"),
        ("invalid", invalid_official_catalog_state(), "official-a"),
        (
            "default-model-absent",
            state_with_official(),
            "missing-model",
        ),
        ("scope-stale", stale_scope_state(), "official-a"),
    ] {
        let persisted = settings_with(vec![pure_oauth_profile("official")], "official");
        let fixture = Fixture::new(&persisted, &catalog_state);
        let persisted = fixture.read_settings();
        let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
        let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();
        let mut next = persisted.clone();
        next.relay_profiles.push(canonical_profile(
            "sub2api",
            model,
            "https://relay.example/v1",
            "provider-key",
        ));

        commit_provider_detail_from_paths(
            &fixture.paths,
            request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 68),
        )
        .unwrap_or_else(|error| panic!("{label}: {:?}", error.code()));

        let saved_state = fixture.read_state();
        let saved_profile = &saved_state.profiles["sub2api"];
        assert_eq!(
            saved_profile.action_required.as_deref(),
            Some("catalog-readiness-unavailable"),
            "{label}"
        );
        assert!(saved_profile.generated_hash.is_none(), "{label}");
        assert!(saved_profile.generated_path.is_none(), "{label}");
        assert!(!saved_profile.restart_required, "{label}");
        assert_eq!(
            fixture.read_settings().active_relay_id,
            "official",
            "{label}"
        );
        assert_eq!(
            fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
            live_before,
            "{label}"
        );
        assert_eq!(
            fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
            auth_before,
            "{label}"
        );
    }
}

#[test]
fn inactive_readiness_failure_preserves_the_last_valid_catalog_artifact() {
    let persisted = settings_with(vec![pure_oauth_profile("official")], "official");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();
    let mut first = persisted.clone();
    first.relay_profiles.push(canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    ));
    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &first,
            "sub2api",
            ProviderCommitAction::Save,
            73,
        ),
    )
    .unwrap();

    let mut stale_state = fixture.read_state();
    let prior_profile_state = stale_state.profiles["sub2api"].clone();
    let generated_path = prior_profile_state.generated_path.as_ref().unwrap().clone();
    let generated_before = fs::read(fixture.paths.codex_home.join(&generated_path)).unwrap();
    let scope_salt = stale_state.scope_salt.clone();
    stale_state.official.as_mut().unwrap().scope_hash =
        official_scope_hash(&scope_salt, "different-account", "workspace-a");
    fs::write(
        &fixture.paths.catalog_state_path,
        serde_json::to_vec_pretty(&stale_state).unwrap(),
    )
    .unwrap();
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();
    let persisted = fixture.read_settings();

    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            74,
        ),
    )
    .unwrap();

    let saved_state = fixture.read_state();
    let mut expected_profile_state = prior_profile_state;
    expected_profile_state.action_required = Some("catalog-readiness-unavailable".to_string());
    assert_eq!(
        serde_json::to_value(&saved_state.profiles["sub2api"]).unwrap(),
        serde_json::to_value(expected_profile_state).unwrap()
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join(generated_path)).unwrap(),
        generated_before
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        live_before
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

#[test]
fn active_readiness_failure_preserves_a_preexisting_valid_catalog_generation() {
    let official = pure_oauth_profile("official");
    let provider = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let persisted = settings_with(vec![official, provider], "official");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();
    let mut active = persisted.clone();
    active.active_relay_id = "sub2api".to_string();
    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &active,
            "sub2api",
            ProviderCommitAction::SetCurrent,
            75,
        ),
    )
    .unwrap();

    let mut stale_state = fixture.read_state();
    assert!(stale_state.profiles["sub2api"].generated_path.is_some());
    let scope_salt = stale_state.scope_salt.clone();
    stale_state.official.as_mut().unwrap().scope_hash =
        official_scope_hash(&scope_salt, "different-account", "workspace-a");
    fs::write(
        &fixture.paths.catalog_state_path,
        serde_json::to_vec_pretty(&stale_state).unwrap(),
    )
    .unwrap();
    let before = fixture.file_generation();
    let persisted = fixture.read_settings();

    let error = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            76,
        ),
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::CatalogScopeStale);
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn later_valid_detail_commit_clears_catalog_readiness_action_and_allows_activation_retry() {
    let persisted = settings_with(vec![pure_oauth_profile("official")], "official");
    let fixture = Fixture::new(&persisted, &stale_scope_state());
    let persisted = fixture.read_settings();
    let mut first = persisted.clone();
    first.relay_profiles.push(canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    ));
    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &first,
            "sub2api",
            ProviderCommitAction::Save,
            77,
        ),
    )
    .unwrap();
    assert_eq!(
        fixture.read_state().profiles["sub2api"]
            .action_required
            .as_deref(),
        Some("catalog-readiness-unavailable")
    );

    let mut recovered_state = fixture.read_state();
    let current = state_with_official();
    recovered_state.official = current.official;
    recovered_state.target = current.target;
    fs::write(
        &fixture.paths.catalog_state_path,
        serde_json::to_vec_pretty(&recovered_state).unwrap(),
    )
    .unwrap();
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
    let persisted = fixture.read_settings();
    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            78,
        ),
    )
    .unwrap();
    let recovered = fixture.read_state();
    let recovered_profile = &recovered.profiles["sub2api"];
    assert!(recovered_profile.action_required.is_none());
    assert!(recovered_profile.generated_path.is_some());
    assert!(!recovered_profile.restart_required);
    assert_eq!(fixture.read_settings().active_relay_id, "official");
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        live_before
    );

    let persisted = fixture.read_settings();
    let mut active = persisted.clone();
    active.active_relay_id = "sub2api".to_string();
    let activated = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &active,
            "sub2api",
            ProviderCommitAction::SetCurrent,
            79,
        ),
    )
    .unwrap();
    assert_eq!(activated.draft_revision, 79);
    assert_eq!(fixture.read_settings().active_relay_id, "sub2api");
}

#[test]
fn set_current_commits_settings_catalog_pointer_activation_and_restart_together() {
    let old = pure_oauth_profile("official");
    let next_profile = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let persisted = settings_with(vec![old.clone(), next_profile.clone()], "official");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();
    assert_eq!(
        persisted.relay_profiles[1].base_url,
        "https://relay.example/v1"
    );
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();
    let mut next = persisted.clone();
    next.active_relay_id = "sub2api".to_string();

    let result = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &next,
            "sub2api",
            ProviderCommitAction::SetCurrent,
            4,
        ),
    )
    .unwrap();

    assert_eq!(result.draft_revision, 4);
    assert!(result.restart_required);
    assert_eq!(fixture.read_settings().active_relay_id, "sub2api");
    let state = fixture.read_state();
    let profile_state = &state.profiles["sub2api"];
    assert!(profile_state.restart_required);
    let generated = fixture
        .paths
        .codex_home
        .join(profile_state.generated_path.as_ref().unwrap());
    assert!(generated.is_file());
    let live = fs::read_to_string(fixture.paths.codex_home.join("config.toml")).unwrap();
    assert!(live.contains("model_provider = \"RelayOne\""));
    assert!(live.contains("x-openai-actor-authorization"));
    assert!(live.contains(profile_state.generated_path.as_ref().unwrap()));
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );

    let first_active = fixture.read_settings();
    let mut updated_active = first_active.clone();
    updated_active.relay_profiles[1] = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay-updated.example/v1",
        "provider-key-updated",
    );
    let second = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &first_active,
            &updated_active,
            "sub2api",
            ProviderCommitAction::Save,
            5,
        ),
    )
    .unwrap();
    assert_eq!(second.draft_revision, 5);
    assert!(second.restart_required);
    let updated_live = fs::read_to_string(fixture.paths.codex_home.join("config.toml")).unwrap();
    assert!(updated_live.contains("https://relay-updated.example/v1"));
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

#[test]
fn fallible_normalizer_rejects_structured_raw_conflicts_before_any_mutation() {
    let canonical = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let mut legacy = canonical.clone();
    legacy.config_contents = legacy.config_contents.replace("RelayOne", "CodexPP");
    let mut pure_api = canonical.clone();
    pure_api.relay_mode = RelayMode::PureApi;

    for (index, (active, conflict_field)) in [
        (canonical, "base-url"),
        (legacy, "model"),
        (pure_api, "key"),
    ]
    .into_iter()
    .enumerate()
    {
        let persisted = settings_with(vec![active], "sub2api");
        let fixture = Fixture::new(&persisted, &state_with_official());
        let persisted = fixture.read_settings();
        let before = [
            fs::read(&fixture.paths.settings_path).unwrap(),
            fs::read(&fixture.paths.catalog_state_path).unwrap(),
            fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
            fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        ];
        let mut conflicted = persisted.relay_profiles[0].clone();
        match conflict_field {
            "base-url" => {
                conflicted.base_url = "https://structured-conflict.example/v1".to_string()
            }
            "model" => conflicted.model = "structured-conflict-model".to_string(),
            "key" => conflicted.api_key = "structured-conflict-key".to_string(),
            _ => unreachable!(),
        }
        let mut next = persisted.clone();
        next.relay_profiles[0] = conflicted;

        let error = commit_provider_detail_from_paths(
            &fixture.paths,
            request(
                &persisted,
                &next,
                "sub2api",
                ProviderCommitAction::Save,
                5 + index as u64,
            ),
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            ProviderCommitErrorCode::InvalidDraft,
            "profile variant {index}: {error}"
        );
        assert!(error.to_string().contains("conflict"));
        assert!(!error.to_string().contains("structured-conflict"));
        assert_eq!(fs::read(&fixture.paths.settings_path).unwrap(), before[0]);
        assert_eq!(
            fs::read(&fixture.paths.catalog_state_path).unwrap(),
            before[1]
        );
        assert_eq!(
            fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
            before[2]
        );
        assert_eq!(
            fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
            before[3]
        );
    }
}

#[test]
fn fallible_normalizer_rejects_byte_distinct_raw_values_before_any_mutation() {
    let canonical = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let mut legacy = canonical.clone();
    legacy.config_contents = legacy.config_contents.replace("RelayOne", "CodexPP");
    let mut pure_api = canonical.clone();
    pure_api.relay_mode = RelayMode::PureApi;

    for (index, (active, raw_field)) in [
        (canonical, "base-url"),
        (legacy, "model"),
        (pure_api, "key"),
    ]
    .into_iter()
    .enumerate()
    {
        let persisted = settings_with(vec![active], "sub2api");
        let fixture = Fixture::new(&persisted, &state_with_official());
        let persisted = fixture.read_settings();
        let before = [
            fs::read(&fixture.paths.settings_path).unwrap(),
            fs::read(&fixture.paths.catalog_state_path).unwrap(),
            fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
            fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        ];
        let mut conflicted = persisted.relay_profiles[0].clone();
        conflicted.config_contents = match raw_field {
            "base-url" => conflicted.config_contents.replace(
                "base_url = \"https://relay.example/v1\"",
                "base_url = \"https://relay.example/v1 \"",
            ),
            "model" => conflicted
                .config_contents
                .replace("model = \"official-a\"", "model = \"official-a \""),
            "key" => conflicted.config_contents.replace(
                "experimental_bearer_token = \"provider-key\"",
                "experimental_bearer_token = \" provider-key \"",
            ),
            _ => unreachable!(),
        };
        let mut next = persisted.clone();
        next.relay_profiles[0] = conflicted;

        let error = commit_provider_detail_from_paths(
            &fixture.paths,
            request(
                &persisted,
                &next,
                "sub2api",
                ProviderCommitAction::Save,
                10 + index as u64,
            ),
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            ProviderCommitErrorCode::InvalidDraft,
            "raw byte variant {index}: {error}"
        );
        assert!(error.to_string().contains("conflict"));
        assert!(!error.to_string().contains("provider-key"));
        assert_eq!(fs::read(&fixture.paths.settings_path).unwrap(), before[0]);
        assert_eq!(
            fs::read(&fixture.paths.catalog_state_path).unwrap(),
            before[1]
        );
        assert_eq!(
            fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
            before[2]
        );
        assert_eq!(
            fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
            before[3]
        );
    }
}

#[test]
fn provider_commit_failures_are_typed_and_failure_payloads_are_secret_free() {
    let active = canonical_profile(
        "sub2api",
        "official-a",
        "https://provider-commit-secret.example/v1",
        "provider-key-sentinel",
    );
    let persisted = settings_with(vec![active], "sub2api");

    let stale_fixture = Fixture::new(&persisted, &state_with_official());
    let stale_persisted = stale_fixture.read_settings();
    let mut stale = request(
        &stale_persisted,
        &stale_persisted,
        "sub2api",
        ProviderCommitAction::Save,
        40,
    );
    stale.expected_provider_fingerprint = "sha256:stale".to_string();
    let stale_error = commit_provider_detail_from_paths(&stale_fixture.paths, stale).unwrap_err();
    assert_eq!(stale_error.code(), ProviderCommitErrorCode::StaleState);

    let invalid_fixture = Fixture::new(&persisted, &state_with_official());
    let invalid_persisted = invalid_fixture.read_settings();
    let mut invalid_next = invalid_persisted.clone();
    invalid_next.relay_profiles[0].auth_contents = r#"{"OPENAI_API_KEY":"provider-key-sentinel","tokens":{"access_token":"oauth-output-sentinel","account_email":"provider-output@example.test"}}"#.to_string();
    let invalid_error = commit_provider_detail_from_paths(
        &invalid_fixture.paths,
        request(
            &invalid_persisted,
            &invalid_next,
            "sub2api",
            ProviderCommitAction::Save,
            41,
        ),
    )
    .unwrap_err();
    assert_eq!(invalid_error.code(), ProviderCommitErrorCode::InvalidDraft);

    let catalog_fixture = Fixture::new(&persisted, &state_with_official());
    let catalog_persisted = catalog_fixture.read_settings();
    let mut catalog_request = request(
        &catalog_persisted,
        &catalog_persisted,
        "sub2api",
        ProviderCommitAction::Save,
        42,
    );
    catalog_request.catalog_drafts[0]
        .overlay
        .custom
        .push(CustomModel::default());
    let catalog_error =
        commit_provider_detail_from_paths(&catalog_fixture.paths, catalog_request).unwrap_err();
    assert_eq!(
        catalog_error.code(),
        ProviderCommitErrorCode::CatalogUnavailable
    );

    let staging_fixture = Fixture::new(&persisted, &state_with_official());
    let staging_persisted = staging_fixture.read_settings();
    let mut staging_next = staging_persisted.clone();
    staging_next.relay_profiles_enabled = false;
    let staging_error = commit_provider_detail_from_paths(
        &staging_fixture.paths,
        request(
            &staging_persisted,
            &staging_next,
            "sub2api",
            ProviderCommitAction::Save,
            43,
        ),
    )
    .unwrap_err();
    assert_eq!(
        staging_error.code(),
        ProviderCommitErrorCode::StagingRejected
    );

    let transaction_fixture = Fixture::new(&persisted, &state_with_official());
    let transaction_persisted = transaction_fixture.read_settings();
    let mut broken_paths = transaction_fixture.paths.clone();
    broken_paths.app_state = transaction_fixture.paths.settings_path.clone();
    let transaction_error = commit_provider_detail_from_paths(
        &broken_paths,
        request(
            &transaction_persisted,
            &transaction_persisted,
            "sub2api",
            ProviderCommitAction::Save,
            44,
        ),
    )
    .unwrap_err();
    assert_eq!(
        transaction_error.code(),
        ProviderCommitErrorCode::TransactionFailed
    );

    for (revision, error) in [
        (40, stale_error),
        (41, invalid_error),
        (42, catalog_error),
        (43, staging_error),
        (44, transaction_error),
    ] {
        let serialized =
            serde_json::to_string(&ProviderCommitPayload::failure(revision, error.code())).unwrap();
        assert!(!serialized.contains("provider-key-sentinel"));
        assert!(!serialized.contains("oauth-output-sentinel"));
        assert!(!serialized.contains("provider-output@example.test"));
        assert!(!serialized.contains("https://provider-commit-secret.example/v1"));
        assert!(!serialized.contains("apiKey"));
        assert!(!serialized.contains("configContents"));
        assert!(!serialized.contains("settings"));
    }
}

#[test]
fn commit_boundary_rejects_missing_reserved_ambiguous_malformed_and_structural_catalog_inputs() {
    let base_profile = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    let mut invalid_profiles = Vec::new();

    let mut missing_model = base_profile.clone();
    missing_model.model.clear();
    missing_model.config_contents = missing_model
        .config_contents
        .replace("model = \"official-a\"", "model = \"\"");
    invalid_profiles.push(missing_model);

    let mut reserved = base_profile.clone();
    reserved.config_contents = reserved.config_contents.replace("RelayOne", "openai");
    invalid_profiles.push(reserved);

    let mut reserved_pure_api = base_profile.clone();
    reserved_pure_api.relay_mode = RelayMode::PureApi;
    reserved_pure_api.config_contents = reserved_pure_api
        .config_contents
        .replace("RelayOne", "openai");
    invalid_profiles.push(reserved_pure_api);

    let mut ambiguous_header = base_profile.clone();
    ambiguous_header.config_contents = ambiguous_header.config_contents.replace(
        "\"x-openai-actor-authorization\" = \"local-image-extension\",",
        "\"x-openai-actor-authorization\" = \"local-image-extension\", \"X-OpenAI-Actor-Authorization\" = \"other\",",
    );
    invalid_profiles.push(ambiguous_header);

    let mut malformed = base_profile.clone();
    malformed.config_contents = "[broken".to_string();
    invalid_profiles.push(malformed);

    for (index, invalid_profile) in invalid_profiles.into_iter().enumerate() {
        let initial = settings_with(vec![base_profile.clone()], "sub2api");
        let fixture = Fixture::new(&initial, &state_with_official());
        let persisted = fixture.read_settings();
        let before = [
            fs::read(&fixture.paths.settings_path).unwrap(),
            fs::read(&fixture.paths.catalog_state_path).unwrap(),
            fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
            fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        ];
        let mut next = persisted.clone();
        next.relay_profiles[0] = invalid_profile;
        let error = commit_provider_detail_from_paths(
            &fixture.paths,
            request(
                &persisted,
                &next,
                "sub2api",
                ProviderCommitAction::Save,
                20 + index as u64,
            ),
        )
        .unwrap_err()
        .to_string();
        assert!(!error.contains("provider-key"));
        assert_eq!(fs::read(&fixture.paths.settings_path).unwrap(), before[0]);
        assert_eq!(
            fs::read(&fixture.paths.catalog_state_path).unwrap(),
            before[1]
        );
        assert_eq!(
            fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
            before[2]
        );
        assert_eq!(
            fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
            before[3]
        );
    }

    let initial = settings_with(vec![base_profile], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();
    let before = [
        fs::read(&fixture.paths.settings_path).unwrap(),
        fs::read(&fixture.paths.catalog_state_path).unwrap(),
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
    ];
    let mut invalid_catalog = request(
        &persisted,
        &persisted,
        "sub2api",
        ProviderCommitAction::Save,
        30,
    );
    invalid_catalog.catalog_drafts[0]
        .overlay
        .custom
        .push(CustomModel::default());
    let error = commit_provider_detail_from_paths(&fixture.paths, invalid_catalog)
        .unwrap_err()
        .to_string();
    assert!(!error.contains("provider-key"));
    assert_eq!(fs::read(&fixture.paths.settings_path).unwrap(), before[0]);
    assert_eq!(
        fs::read(&fixture.paths.catalog_state_path).unwrap(),
        before[1]
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        before[2]
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        before[3]
    );
}

#[test]
fn staged_native_contract_assertion_rejects_core_drift() {
    let profile = canonical_profile(
        "sub2api",
        "official-a",
        "https://relay.example/v1",
        "provider-key",
    );
    assert_staged_native_provider_contract(
        &profile,
        &profile.config_contents,
        CatalogMode::OfficialPlusCustom,
    )
    .unwrap();

    let auth_drift = profile.config_contents.replace(
        "requires_openai_auth = false",
        "requires_openai_auth = true",
    );
    assert!(
        assert_staged_native_provider_contract(
            &profile,
            &auth_drift,
            CatalogMode::OfficialPlusCustom,
        )
        .is_err()
    );
    let header_drift = profile
        .config_contents
        .replace(
            "http_headers = { \"x-openai-actor-authorization\" = \"local-image-extension\", \"x-keep\" = \"yes\" }",
            "http_headers = { \"x-keep\" = \"yes\" }",
        );
    assert!(
        assert_staged_native_provider_contract(
            &profile,
            &header_drift,
            CatalogMode::OfficialPlusCustom,
        )
        .is_err()
    );
}
