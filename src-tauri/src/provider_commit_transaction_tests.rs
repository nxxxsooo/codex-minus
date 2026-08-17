use std::fs;

use crate::commands::{
    GenericSettingsSaveError, ProviderCommitCheckpoint, ProviderCommitErrorCode,
    ProviderCommitPaths, ProviderCommitPayload, assert_staged_native_provider_contract,
    commit_provider_detail_from_paths, commit_provider_detail_from_paths_observed,
    commit_relay_profile_transaction_at, save_settings_value_with_provider_guard_at,
    save_settings_with_provider_guard_at, save_settings_with_provider_guard_at_observed,
    settings_snapshot_for_ui_projection, switch_relay_profile_blocking_at,
    ui_provider_topology_projection,
};
use crate::provider_commit::{
    CatalogMode, CatalogOverlay, CatalogState, CustomModel, OfficialSnapshot, ProfileCatalogDraft,
    ProviderCommitAction, ProviderCommitRequest, ProviderOwnedTopologyDraft, UpstreamTopology,
    provider_owned_fingerprint,
};
use base64::Engine;
use codex_plus_core::settings::{
    AggregateRelayProfile, AggregateRelayStrategy, BackendSettings, RelayMode, RelayProfile,
    RelayProtocol, SettingsStore,
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
        model: "gpt-5.6-sol".to_string(),
        relay_mode: RelayMode::Official,
        protocol: RelayProtocol::Responses,
        config_contents: "model = \"gpt-5.6-sol\"\n".to_string(),
        ..RelayProfile::default()
    }
}

fn legacy_pure_api_profile(id: &str, auth_contents: &str) -> RelayProfile {
    let mut profile = canonical_profile(
        id,
        "gpt-5.6-sol",
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
            "slug": "gpt-5.6-sol",
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
        cli_path: "/Applications/ChatGPT.app/Contents/Resources/codex".to_string(),
        client_version: version.to_string(),
        identity_hash: identity.to_string(),
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

fn persisted_settings_fixture_bytes(settings: &BackendSettings) -> Vec<u8> {
    let mut value = serde_json::to_value(settings).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("aggregateRelayProfiles");
    value
        .as_object_mut()
        .unwrap()
        .remove("activeAggregateRelayId");
    for profile in value["relayProfiles"].as_array_mut().unwrap() {
        profile.as_object_mut().unwrap().remove("protocol");
        profile.as_object_mut().unwrap().remove("upstreamBaseUrl");
    }
    serde_json::to_vec_pretty(&value).unwrap()
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
        fs::write(&settings_path, persisted_settings_fixture_bytes(settings)).unwrap();
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
fn responses_only_load_rejection_precedes_auth_migration() {
    let mut unsupported = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    unsupported.protocol = RelayProtocol::ChatCompletions;
    unsupported.auth_contents = r#"{"OPENAI_API_KEY":"legacy-provider-key"}"#.to_string();
    let initial = settings_with(vec![unsupported], "sub2api");
    let fixture = Fixture::new(&initial, &state_with_official());
    let mut unsupported_value: Value =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    unsupported_value["relayProfiles"][0]["protocol"] = json!("chatCompletions");
    fs::write(
        &fixture.paths.settings_path,
        serde_json::to_vec_pretty(&unsupported_value).unwrap(),
    )
    .unwrap();
    let persisted_bytes = fs::read(&fixture.paths.settings_path).unwrap();
    let before = fixture.file_generation();
    let mut valid_request_settings = initial.clone();
    valid_request_settings.relay_profiles[0].protocol = RelayProtocol::Responses;
    valid_request_settings.relay_profiles[0]
        .auth_contents
        .clear();
    let request = ProviderCommitRequest {
        topology: ProviderOwnedTopologyDraft::from_settings(&valid_request_settings),
        catalog_drafts: vec![catalog_draft("sub2api")],
        focused_profile_id: Some("sub2api".to_string()),
        action: ProviderCommitAction::Save,
        previous_active_relay_id: "sub2api".to_string(),
        confirm_context_cleanup: false,
        draft_revision: 101,
        expected_provider_fingerprint: provider_owned_fingerprint(
            &ProviderOwnedTopologyDraft::from_settings(&initial),
        )
        .unwrap(),
    };

    let error = commit_provider_detail_from_paths(&fixture.paths, request).unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::InputUnavailable);
    assert_eq!(
        fs::read(&fixture.paths.settings_path).unwrap(),
        persisted_bytes
    );
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn caller_raw_toml_rejection_precedes_persisted_auth_migration_and_any_checkpoint() {
    let initial = settings_with(
        vec![canonical_profile(
            "sub2api",
            "gpt-5.6-sol",
            "https://relay.example/v1",
            "provider-key",
        )],
        "sub2api",
    );
    let fixture = Fixture::new(&initial, &state_with_official());
    let mut persisted_value: Value =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    persisted_value["relayProfiles"][0]["authContents"] =
        json!(r#"{"OPENAI_API_KEY":"provider-key"}"#);
    fs::write(
        &fixture.paths.settings_path,
        serde_json::to_vec_pretty(&persisted_value).unwrap(),
    )
    .unwrap();
    let persisted = fixture.read_settings();
    let mut invalid = persisted.clone();
    invalid.relay_profiles[0].config_contents = invalid.relay_profiles[0]
        .config_contents
        .replace("wire_api = \"responses\"", "wire_api = \"chat\"");
    let before = fixture.file_generation();
    let mut checkpoints = Vec::new();

    let error = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(
            &persisted,
            &invalid,
            "sub2api",
            ProviderCommitAction::Save,
            102,
        ),
        |checkpoint| {
            checkpoints.push(checkpoint);
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::InvalidDraft);
    assert!(checkpoints.is_empty());
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn relay_transaction_rejection_precedes_migration_staging_and_context_mutation() {
    let initial = settings_with(
        vec![canonical_profile(
            "sub2api",
            "gpt-5.6-sol",
            "https://relay.example/v1",
            "provider-key",
        )],
        "sub2api",
    );
    for unsupported in [
        {
            let mut settings = initial.clone();
            settings.relay_profiles[0].protocol = RelayProtocol::ChatCompletions;
            settings
        },
        {
            let mut settings = initial.clone();
            settings.relay_profiles[0].base_url = "http://127.0.0.1:57321/v1".to_string();
            settings
        },
        {
            let mut settings = initial.clone();
            settings.relay_profiles[0].auth_contents =
                r#"{"OPENAI_API_KEY":"incoming-auth-must-not-migrate"}"#.to_string();
            settings
        },
    ] {
        let fixture = Fixture::new(&initial, &state_with_official());
        fs::write(
            fixture.paths.codex_home.join("config.toml"),
            rich_live_config(),
        )
        .unwrap();
        let before = fixture.file_generation();
        let context_before = semantic_context_tables(rich_live_config());

        assert!(
            commit_relay_profile_transaction_at(&fixture.paths, unsupported, "sub2api", false)
                .is_err()
        );

        assert_eq!(fixture.file_generation(), before);
        assert_eq!(
            semantic_context_tables(
                &fs::read_to_string(fixture.paths.codex_home.join("config.toml")).unwrap()
            ),
            context_before
        );
    }
}

#[test]
fn relay_transaction_catalog_plan_uses_the_injected_catalog_state_path() {
    let initial = settings_with(
        vec![canonical_profile(
            "sub2api",
            "gpt-5.6-sol",
            "https://relay.example/v1",
            "provider-key",
        )],
        "sub2api",
    );
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();
    let catalog_before = fs::read(&fixture.paths.catalog_state_path).unwrap();

    commit_relay_profile_transaction_at(&fixture.paths, persisted, "sub2api", false).unwrap();

    assert_ne!(
        fs::read(&fixture.paths.catalog_state_path).unwrap(),
        catalog_before,
        "the accepted relay transaction must persist its catalog generation beside its settings"
    );
}

#[test]
fn injected_switch_failure_payload_hides_every_rejected_persisted_topology() {
    let valid = settings_with(
        vec![canonical_profile(
            "sub2api",
            "gpt-5.6-sol",
            "https://relay.example/v1",
            "provider-key",
        )],
        "sub2api",
    );
    let invalid_states = [
        {
            let mut settings = valid.clone();
            settings.relay_profiles[0].protocol = RelayProtocol::ChatCompletions;
            settings
        },
        {
            let mut settings = valid.clone();
            settings.relay_profiles[0].base_url = "http://127.0.0.1:57321/v1".to_string();
            settings
        },
        {
            let mut settings = valid.clone();
            settings.relay_profiles[0].auth_contents = "{persisted-auth-must-not-leak".to_string();
            settings
        },
        {
            let mut settings = valid.clone();
            settings.relay_profiles[0].relay_mode = RelayMode::Aggregate;
            settings
        },
        {
            let mut settings = valid.clone();
            settings
                .aggregate_relay_profiles
                .push(AggregateRelayProfile {
                    id: "removed-aggregate".to_string(),
                    name: "Removed aggregate".to_string(),
                    strategy: AggregateRelayStrategy::Failover,
                    members: Vec::new(),
                });
            settings
        },
        {
            let mut settings = valid.clone();
            settings.active_aggregate_relay_id = "removed-aggregate".to_string();
            settings
        },
    ];
    for invalid in invalid_states {
        let fixture = Fixture::new(&invalid, &state_with_official());
        let mut forced_failure = valid.clone();
        forced_failure.relay_profiles_enabled = false;
        let result = switch_relay_profile_blocking_at(
            &fixture.paths,
            crate::commands::RelayProfileSwitchRequest {
                settings: forced_failure,
                previous_active_relay_id: "sub2api".to_string(),
                confirm_context_cleanup: false,
            },
        );

        assert_eq!(result.status, "failed");
        assert!(
            crate::provider_commit::validate_responses_only_settings(&result.payload.settings)
                .is_ok()
        );
        assert!(
            result
                .payload
                .settings
                .relay_profiles
                .iter()
                .all(|profile| profile.auth_contents.is_empty())
        );
    }
}

#[test]
fn managed_context_cleanup_requires_confirmation_and_commits_settings_and_live_atomically() {
    let mut structured_only = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    structured_only.context_window = "272000".to_string();
    structured_only.auto_compact_limit = "240000".to_string();
    let mut raw_only = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
        "https://changed.example/v1",
        "changed-provider-key",
    );
    let auth_path = fixture.paths.codex_home.join("auth.json");
    let newer_auth = b"gpt-5.6-soluth-newer".to_vec();
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
    assert!(!error.to_string().contains("gpt-5.6-soluth-newer"));
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
fn eva_residue_is_scrubbed_before_provider_commit_snapshots_or_recovery_artifacts() {
    const OAUTH_SENTINEL: &str = "eva-oauth-access-sentinel";
    let eva = RelayProfile {
        id: "eva".to_string(),
        name: "Eva|Codex".to_string(),
        model: "gpt-5.6-terra".to_string(),
        base_url: "https://example.test/v1".to_string(),
        upstream_base_url: "https://example.test/v1".to_string(),
        protocol: RelayProtocol::Responses,
        relay_mode: RelayMode::Official,
        official_mix_api_key: true,
        config_contents: r#"model = "gpt-5.6-terra"
model_provider = "OpenAI"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://example.test/v1"
wire_api = "responses"
requires_openai_auth = true
"#
        .to_string(),
        auth_contents: format!(
            r#"{{"OPENAI_API_KEY":"provider-key-sentinel","auth_mode":"chatgpt","tokens":{{"access_token":"{OAUTH_SENTINEL}"}}}}"#
        ),
        ..RelayProfile::default()
    };
    let fixture = Fixture::new(&settings_with(vec![eva], "eva"), &state_with_official());
    let auth_path = fixture.paths.codex_home.join("auth.json");
    let auth_before = fs::read(&auth_path).unwrap();
    // The editor observes this profile after startup's credential scrub: the profile copy is
    // gone and the provider key is in its selected TOML table. The raw on-disk fixture remains
    // intentionally legacy-shaped so this commit exercises the pre-snapshot boundary.
    let mut persisted = fixture.read_settings();
    persisted.relay_profiles[0].api_key = "provider-key-sentinel".to_string();
    let mut provider_config = persisted.relay_profiles[0]
        .config_contents
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    provider_config["model_providers"]["OpenAI"]["experimental_bearer_token"] =
        toml_edit::value("provider-key-sentinel");
    persisted.relay_profiles[0].config_contents = provider_config.to_string();
    let mut next = persisted.clone();
    next.relay_profiles[0].name = "Eva migrated".to_string();
    let app_state = fixture.paths.app_state.clone();
    let codex_home = fixture.paths.codex_home.clone();
    let mut inspected_recovery_artifacts = false;

    let result = commit_provider_detail_from_paths_observed(
        &fixture.paths,
        request(&persisted, &next, "eva", ProviderCommitAction::Save, 70),
        |checkpoint| {
            if checkpoint != ProviderCommitCheckpoint::SettingsPersistence {
                return Ok(());
            }
            inspected_recovery_artifacts = true;
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
            assert_eq!(fs::read(&auth_path)?, auth_before);
            Ok(())
        },
    );

    assert!(
        inspected_recovery_artifacts,
        "commit did not reach recovery-artifact inspection: {:?}",
        result.as_ref().err()
    );
    result.unwrap();
    assert_eq!(fs::read(&auth_path).unwrap(), auth_before);
    let final_settings = fs::read_to_string(&fixture.paths.settings_path).unwrap();
    assert!(!final_settings.contains("authContents"));
    assert!(!final_settings.contains(OAUTH_SENTINEL));
}

#[test]
fn provider_commit_pre_snapshot_scrub_preserves_invalid_settings_reason() {
    let initial = settings_with(
        vec![canonical_profile(
            "sub2api",
            "gpt-5.6-sol",
            "https://relay.example/v1",
            "provider-key",
        )],
        "sub2api",
    );
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();
    fs::write(&fixture.paths.settings_path, "{invalid-json").unwrap();

    let error = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            71,
        ),
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::InputUnavailable);
    assert_eq!(error.reason(), "provider settings are invalid JSON");
}

#[test]
fn provider_commit_pre_snapshot_scrub_classifies_profile_reconciliation_as_auth_migration() {
    let legacy = legacy_pure_api_profile("legacy", "{invalid-json");
    let fixture = Fixture::new(
        &settings_with(vec![legacy], "legacy"),
        &state_with_official(),
    );
    let persisted = fixture.read_settings();

    let error = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "legacy",
            ProviderCommitAction::Save,
            72,
        ),
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::InputUnavailable);
    assert_eq!(
        error.reason(),
        "a saved provider profile failed auth migration"
    );
    assert!(!error.to_string().contains("invalid-json"));
}

#[test]
fn provider_commit_pre_snapshot_scrub_classifies_owner_only_failure_as_transaction_failure() {
    let initial = settings_with(
        vec![canonical_profile(
            "sub2api",
            "gpt-5.6-sol",
            "https://relay.example/v1",
            "provider-key",
        )],
        "sub2api",
    );
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();
    fs::remove_file(&fixture.paths.settings_path).unwrap();
    fs::create_dir(&fixture.paths.settings_path).unwrap();

    let error = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            73,
        ),
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::TransactionFailed);
    assert_eq!(error.reason(), "provider transaction failed");
}

#[test]
fn generic_settings_save_allows_unrelated_changes_but_rejects_every_provider_owned_difference() {
    let first = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    let second = canonical_profile(
        "backup",
        "gpt-5.6-sol",
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
fn generic_settings_wire_rejects_explicit_retired_fields_in_incoming_and_persisted_json() {
    let profile = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    let initial = settings_with(vec![profile], "sub2api");
    let retired = [
        ("aggregateRelayProfiles", json!([]), false),
        ("activeAggregateRelayId", json!(""), false),
        ("protocol", json!("responses"), true),
        ("upstreamBaseUrl", json!(""), true),
    ];

    for (field, value, profile_field) in retired {
        let fixture = Fixture::new(&initial, &state_with_official());
        let mut incoming: Value =
            serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
        incoming["codexGoalsEnabled"] = json!(true);
        if profile_field {
            incoming["relayProfiles"][0][field] = value.clone();
        } else {
            incoming[field] = value.clone();
        }
        let before = fixture.file_generation();

        let error =
            save_settings_value_with_provider_guard_at(&fixture.paths, incoming).unwrap_err();

        assert_eq!(
            error,
            GenericSettingsSaveError::IncomingSettingsInvalid,
            "{field}"
        );
        assert_eq!(fixture.file_generation(), before, "incoming {field}");

        let fixture = Fixture::new(&initial, &state_with_official());
        let incoming: Value =
            serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
        let mut persisted = incoming.clone();
        if profile_field {
            persisted["relayProfiles"][0][field] = value;
        } else {
            persisted[field] = value;
        }
        let persisted_bytes = serde_json::to_vec_pretty(&persisted).unwrap();
        fs::write(&fixture.paths.settings_path, &persisted_bytes).unwrap();

        let error =
            save_settings_value_with_provider_guard_at(&fixture.paths, incoming).unwrap_err();

        assert_eq!(
            error,
            GenericSettingsSaveError::PersistedSettingsInvalid,
            "{field}"
        );
        assert_eq!(
            fs::read(&fixture.paths.settings_path).unwrap(),
            persisted_bytes,
            "persisted {field}"
        );
    }
}

#[test]
fn generic_settings_save_accepts_real_ui_derived_provider_shape_for_unrelated_changes() {
    let first = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
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
fn generic_settings_save_accepts_first_run_ui_shape() {
    let first_run = Fixture::new(&BackendSettings::default(), &state_with_official());
    fs::remove_file(&first_run.paths.settings_path).unwrap();
    let mut first_run_ui = BackendSettings::default();
    first_run_ui.relay_profiles[0].context_selection_initialized = true;
    first_run_ui.relay_base_url = first_run_ui.relay_profiles[0].base_url.clone();
    first_run_ui.relay_api_key = first_run_ui.relay_profiles[0].api_key.clone();
    first_run_ui.codex_goals_enabled = true;
    save_settings_with_provider_guard_at(&first_run.paths, first_run_ui).unwrap();
}

#[test]
fn generic_settings_save_rejects_a_concurrent_persisted_provider_generation_change() {
    let initial = settings_with(
        vec![canonical_profile(
            "sub2api",
            "gpt-5.6-sol",
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
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    let second = canonical_profile(
        "backup",
        "gpt-5.6-sol",
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

        let expected = match case {
            "auth-contents" => GenericSettingsSaveError::ProviderAuthProhibited,
            "invalid-provider-toml" => GenericSettingsSaveError::IncomingSettingsInvalid,
            _ => GenericSettingsSaveError::ProviderOwnedDifference,
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
            "gpt-5.6-sol",
            "https://stale-provider.example/v1",
            "stale-provider-key",
        );
        let inactive = canonical_profile(
            "backup",
            "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
        "https://a.example/v1",
        "provider-key-a",
    );
    let b = canonical_profile(
        "relay-b",
        "gpt-5.6-sol",
        "https://b.example/v1",
        "provider-key-b",
    );
    let mut shadow = b.clone();
    shadow.id = "relay-shadow".to_string();
    shadow.name = "Relay B shadow".to_string();
    let initial = settings_with(vec![a.clone(), shadow, b.clone()], "relay-a");
    let mut catalog_state = state_with_official();
    for profile_id in ["relay-a", "relay-b"] {
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
    let mut copy = persisted_b.clone();
    copy.id = "relay-copy".to_string();
    copy.name = "Relay B copy".to_string();
    let mut next = persisted.clone();
    next.relay_profiles = vec![persisted_b, persisted_a, copy];
    next.relay_profiles_enabled = false;
    next.relay_test_model = "topology-test-model".to_string();
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
        vec!["relay-b", "relay-a", "relay-copy"]
    );
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
        "gpt-5.6-sol",
        "https://source.example/v1",
        "provider-key-source",
    );
    let initial = settings_with(vec![official, source], "official");
    let mut catalog_state = state_with_official();
    catalog_state
        .profiles
        .entry("relay-source".to_string())
        .or_default()
        .mode = CatalogMode::OfficialPlusCustom;
    let fixture = Fixture::new(&initial, &catalog_state);
    fs::remove_file(fixture.paths.codex_home.join("auth.json")).unwrap();
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
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).ok();

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
        fs::read(fixture.paths.codex_home.join("auth.json")).ok(),
        auth_before
    );
}

#[test]
fn topology_adapter_rejects_active_detail_and_catalog_bypasses_without_mutation() {
    let active = canonical_profile(
        "relay-a",
        "gpt-5.6-sol",
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
                .replace("gpt-5.6-sol", "changed-model")
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
fn unified_provider_commit_rejects_unreviewed_external_to_managed_transition() {
    let mut external = canonical_profile(
        "external",
        "gpt-5.6-sol",
        "https://external.example/v1",
        "provider-key",
    );
    external.config_contents = format!(
        "model_catalog_json = \"external/user-owned.json\"\n{}",
        external.config_contents
    );
    let persisted = settings_with(vec![external], "external");
    let mut state = state_with_official();
    state.profiles.insert(
        "external".to_string(),
        crate::model_catalog::ProfileCatalogState {
            mode: CatalogMode::External,
            mode_explicit: true,
            external_pointer: Some("external/user-owned.json".to_string()),
            ..crate::model_catalog::ProfileCatalogState::default()
        },
    );
    let fixture = Fixture::new(&persisted, &state);
    let external_path = fixture.paths.codex_home.join("external/user-owned.json");
    fs::create_dir_all(external_path.parent().unwrap()).unwrap();
    fs::write(&external_path, b"{\"models\":[]}").unwrap();
    let before = fixture.file_generation();
    let persisted = fixture.read_settings();

    let error = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "external",
            ProviderCommitAction::Save,
            80,
        ),
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::InvalidDraft);
    // The rejection names the ownership rule, not the "draft validation" family fallback that
    // used to blame the form for a rule the form cannot see.
    assert_eq!(
        error.reason(),
        "external catalog ownership requires the reviewed adoption command"
    );
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn topology_adapter_accepts_one_complete_ui_canonical_generation() {
    let initial = settings_with(
        vec![
            canonical_profile(
                "relay-a",
                "gpt-5.6-sol",
                "https://a.example/v1",
                "provider-key-a",
            ),
            canonical_profile(
                "relay-b",
                "gpt-5.6-sol",
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
fn topology_reorder_preserves_a_surviving_legacy_profile_contract_byte_for_byte() {
    let active = canonical_profile(
        "relay-a",
        "gpt-5.6-sol",
        "https://a.example/v1",
        "provider-key-a",
    );
    let mut legacy = canonical_profile(
        "legacy",
        "gpt-5.6-sol",
        "https://legacy.example/v1",
        "legacy-key",
    );
    legacy.config_contents = GOLDEN_UNTOUCHED_BYSTANDER_ALIAS.to_string();
    let initial = settings_with(vec![active, legacy], "relay-a");
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted: BackendSettings =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    let mut topology = ui_provider_topology_projection(
        settings_snapshot_for_ui_projection(persisted.clone()).unwrap(),
    )
    .unwrap();
    topology.relay_profiles.reverse();
    let request = ProviderCommitRequest {
        topology,
        catalog_drafts: Vec::new(),
        focused_profile_id: None,
        action: ProviderCommitAction::Save,
        previous_active_relay_id: persisted.active_relay_id.clone(),
        confirm_context_cleanup: false,
        draft_revision: 44,
        expected_provider_fingerprint: provider_owned_fingerprint(
            &ProviderOwnedTopologyDraft::from_settings(&persisted),
        )
        .unwrap(),
    };

    commit_provider_detail_from_paths(&fixture.paths, request).unwrap();

    assert_eq!(
        raw_stored_profile_config(&fixture.paths.settings_path, "legacy"),
        GOLDEN_UNTOUCHED_BYSTANDER_ALIAS,
        "a topology-only reorder rewrote a surviving provider contract"
    );
}

#[test]
fn focused_commit_omitting_an_inactive_bystander_fails_without_mutation() {
    let active = canonical_profile(
        "relay-a",
        "gpt-5.6-sol",
        "https://a.example/v1",
        "provider-key-a",
    );
    let inactive = canonical_profile(
        "relay-b",
        "gpt-5.6-sol",
        "https://b.example/v1",
        "provider-key-b",
    );
    let initial = settings_with(vec![active, inactive], "relay-a");
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();
    let mut omitted = persisted.clone();
    omitted
        .relay_profiles
        .retain(|profile| profile.id == "relay-a");
    let before = fixture.file_generation();

    let error = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &omitted,
            "relay-a",
            ProviderCommitAction::Save,
            45,
        ),
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::InvalidDraft);
    assert_eq!(
        error.reason(),
        "focused provider detail cannot omit a persisted bystander profile"
    );
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn focused_commit_applies_only_the_focused_profile_contract() {
    let active = canonical_profile(
        "relay-a",
        "gpt-5.6-sol",
        "https://a.example/v1",
        "provider-key-a",
    );
    let inactive = canonical_profile(
        "relay-b",
        "gpt-5.6-sol",
        "https://b.example/v1",
        "provider-key-b",
    );
    let mut initial = settings_with(vec![active, inactive], "relay-a");
    initial.relay_test_model = "persisted-test-model".to_string();
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();
    let bystander_before = raw_stored_profile_config(&fixture.paths.settings_path, "relay-b");
    let mut forged = persisted.clone();
    forged.relay_profiles.reverse();
    forged.relay_profiles_enabled = false;
    forged.relay_test_model = "forged-topology-test-model".to_string();
    let focused = forged
        .relay_profiles
        .iter_mut()
        .find(|profile| profile.id == "relay-a")
        .unwrap();
    *focused = canonical_profile(
        "relay-a",
        "gpt-5.6-sol",
        "https://a-updated.example/v1",
        "provider-key-a-updated",
    );

    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &forged,
            "relay-a",
            ProviderCommitAction::Save,
            47,
        ),
    )
    .unwrap();

    let saved: BackendSettings =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    assert_eq!(
        saved
            .relay_profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<Vec<_>>(),
        vec!["relay-a", "relay-b"]
    );
    assert!(saved.relay_profiles_enabled);
    assert_eq!(saved.relay_test_model, "persisted-test-model");
    assert_eq!(
        fixture
            .read_settings()
            .relay_profiles
            .iter()
            .find(|profile| profile.id == "relay-a")
            .unwrap()
            .base_url,
        "https://a-updated.example/v1"
    );
    assert_eq!(
        raw_stored_profile_config(&fixture.paths.settings_path, "relay-b"),
        bystander_before
    );
}

#[test]
fn active_and_inactive_commits_reject_legacy_raw_provider_toml_without_mutation() {
    let a = canonical_profile(
        "relay-a",
        "gpt-5.6-sol",
        "https://a.example/v1",
        "provider-key-a",
    );
    let b = canonical_profile(
        "relay-b",
        "gpt-5.6-sol",
        "https://b.example/v1",
        "provider-key-b",
    );
    let initial = settings_with(vec![a, b], "relay-a");

    for focused in ["relay-a", "relay-b"] {
        for legacy in ["chat-wire", "missing-wire", "chat-marker", "proxy-url"] {
            let fixture = Fixture::new(&initial, &state_with_official());
            let persisted = fixture.read_settings();
            let mut next = persisted.clone();
            let profile = next
                .relay_profiles
                .iter_mut()
                .find(|profile| profile.id == focused)
                .unwrap();
            profile.config_contents = match legacy {
                "chat-wire" => profile
                    .config_contents
                    .replace("wire_api = \"responses\"", "wire_api = \"chat\""),
                "missing-wire" => profile
                    .config_contents
                    .replace("wire_api = \"responses\"\n", ""),
                "chat-marker" => format!(
                    "codex_plus_chat_base_url = \"https://legacy.example/v1\"\n{}",
                    profile.config_contents
                ),
                "proxy-url" => profile.config_contents.replace(
                    &format!("base_url = \"{}\"", profile.base_url),
                    "base_url = \"http://127.0.0.1:57321/v1\"",
                ),
                _ => unreachable!(),
            };
            let before = fixture.file_generation();

            let error = commit_provider_detail_from_paths(
                &fixture.paths,
                request(&persisted, &next, focused, ProviderCommitAction::Save, 46),
            )
            .unwrap_err();

            assert_eq!(
                error.code(),
                ProviderCommitErrorCode::InvalidDraft,
                "{focused} {legacy}"
            );
            assert_eq!(fixture.file_generation(), before, "{focused} {legacy}");
        }
    }
}

#[test]
fn persisted_legacy_api_key_auth_migrates_through_provider_commit() {
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
}

#[test]
fn active_native_commit_requires_current_official_auth_and_catalog_scope() {
    let active = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
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

    // An expired access token is deliberately absent from this list. It used to belong here,
    // because the commit projected that token into an isolated CODEX_HOME. Nothing is projected
    // now, so a token between refreshes is not a reason to refuse a save; what still fails closed
    // is auth that cannot be read or cannot name an account.
    for auth_bytes in [
        b"not-json".to_vec(),
        serde_json::to_vec(&serde_json::json!({
            "auth_mode": "chatgpt",
            "tokens": { "id_token": "x.x.x" },
        }))
        .unwrap(),
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
                "gpt-5.6-sol",
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

    // The baseline ships with the application, so it belongs to no account and no installed CLI.
    // Switching either used to strand every managed profile until a refresh that can no longer be
    // run; both must now commit normally as long as the official session is valid.
    for (account, workspace) in [("account-b", "workspace-a"), ("account-a", "workspace-b")] {
        let other_scope = Fixture::new(&initial, &state_with_official());
        fs::write(
            other_scope.paths.codex_home.join("auth.json"),
            official_auth_bytes(account, workspace),
        )
        .unwrap();
        let persisted = other_scope.read_settings();
        commit_provider_detail_from_paths(
            &other_scope.paths,
            request(
                &persisted,
                &persisted,
                "sub2api",
                ProviderCommitAction::Save,
                62,
            ),
        )
        .unwrap_or_else(|error| {
            panic!("a different official account must not strand the bundled baseline: {error:?}")
        });
    }

    let mut other_target = Fixture::new(&initial, &state_with_official());
    other_target.paths.current_target = Some(target_identity("0.148.0", "target-b"));
    let persisted = other_target.read_settings();
    commit_provider_detail_from_paths(
        &other_target.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            63,
        ),
    )
    .unwrap_or_else(|error| {
        panic!("a different installed CLI must not strand the bundled baseline: {error:?}")
    });

    let inactive = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
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
            .is_some(),
        "a signed-out save still records that the profile cannot be activated yet"
    );
}

#[test]
fn active_catalog_readiness_failures_preserve_the_complete_prior_generation() {
    for (label, catalog_state, model, signed_in, expected_code) in [
        (
            "signed-out",
            state_with_official(),
            "gpt-5.6-sol",
            false,
            ProviderCommitErrorCode::OfficialAuthRequired,
        ),
        (
            "default-model-absent",
            state_with_official(),
            "missing-model",
            true,
            ProviderCommitErrorCode::CatalogUnavailable,
        ),
    ] {
        let active =
            canonical_profile("sub2api", model, "https://relay.example/v1", "provider-key");
        let fixture = Fixture::new(&settings_with(vec![active], "sub2api"), &catalog_state);
        if !signed_in {
            fs::remove_file(fixture.paths.codex_home.join("auth.json")).unwrap();
        }
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
    let fixture = Fixture::new(&persisted, &state_with_official());
    fs::remove_file(fixture.paths.codex_home.join("auth.json")).unwrap();
    let persisted = fixture.read_settings();
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
    let mut next = persisted.clone();
    next.relay_profiles.push(canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
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
        fs::read(fixture.paths.codex_home.join("auth.json")).ok(),
        None
    );
}

#[test]
fn inactive_catalog_readiness_failures_persist_action_required_without_live_claims() {
    for (label, catalog_state, model, signed_in) in [
        (
            "default-model-absent",
            state_with_official(),
            "missing-model",
            true,
        ),
        ("signed-out", state_with_official(), "gpt-5.6-sol", false),
    ] {
        let persisted = settings_with(vec![pure_oauth_profile("official")], "official");
        let fixture = Fixture::new(&persisted, &catalog_state);
        if !signed_in {
            fs::remove_file(fixture.paths.codex_home.join("auth.json")).unwrap();
        }
        let persisted = fixture.read_settings();
        let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
        let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).ok();
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
            fs::read(fixture.paths.codex_home.join("auth.json")).ok(),
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
        "gpt-5.6-sol",
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

    let prior_profile_state = fixture.read_state().profiles["sub2api"].clone();
    let generated_path = prior_profile_state.generated_path.as_ref().unwrap().clone();
    let generated_before = fs::read(fixture.paths.codex_home.join(&generated_path)).unwrap();
    fs::remove_file(fixture.paths.codex_home.join("auth.json")).unwrap();
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();
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
        fs::read(fixture.paths.codex_home.join("auth.json")).ok(),
        None
    );
}

#[test]
fn active_readiness_failure_preserves_a_preexisting_valid_catalog_generation() {
    let official = pure_oauth_profile("official");
    let provider = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
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

    assert!(
        fixture.read_state().profiles["sub2api"]
            .generated_path
            .is_some()
    );
    fs::remove_file(fixture.paths.codex_home.join("auth.json")).unwrap();
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

    assert_eq!(error.code(), ProviderCommitErrorCode::OfficialAuthRequired);
    assert_eq!(fixture.file_generation(), before);
}

#[test]
fn later_valid_detail_commit_clears_catalog_readiness_action_and_allows_activation_retry() {
    let persisted = settings_with(vec![pure_oauth_profile("official")], "official");
    let fixture = Fixture::new(&persisted, &state_with_official());
    fs::remove_file(fixture.paths.codex_home.join("auth.json")).unwrap();
    let persisted = fixture.read_settings();
    let mut first = persisted.clone();
    first.relay_profiles.push(canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
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

    fs::write(
        fixture.paths.codex_home.join("auth.json"),
        official_auth_bytes("account-a", "workspace-a"),
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
        "gpt-5.6-sol",
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
                .replace("model = \"gpt-5.6-sol\"", "model = \"gpt-5.6-sol \""),
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
        "gpt-5.6-sol",
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
        (44, transaction_error),
    ] {
        let serialized = serde_json::to_string(&ProviderCommitPayload::failure(
            revision,
            error.code(),
            error.reason(),
        ))
        .unwrap();
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
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    let mut invalid_profiles = Vec::new();

    let mut missing_model = base_profile.clone();
    missing_model.model.clear();
    missing_model.config_contents = missing_model
        .config_contents
        .replace("model = \"gpt-5.6-sol\"", "model = \"\"");
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
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    assert_staged_native_provider_contract(
        &profile,
        &profile.config_contents,
        CatalogMode::OfficialPlusCustom,
    )
    .unwrap();

    // Both auth values are canonical runtime choices now, so a value flip is not drift —
    // but the field going missing entirely still is.
    let auth_flip = profile.config_contents.replace(
        "requires_openai_auth = false",
        "requires_openai_auth = true",
    );
    assert_staged_native_provider_contract(&profile, &auth_flip, CatalogMode::OfficialPlusCustom)
        .unwrap();
    let auth_drift = profile
        .config_contents
        .replace("requires_openai_auth = false\n", "");
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

#[test]
fn active_commit_binds_restart_to_the_runtime_fingerprint_without_a_catalog_generation() {
    let profile = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    let persisted = settings_with(vec![profile.clone()], "sub2api");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();

    let first = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            1,
        ),
    )
    .unwrap();

    assert!(first.restart_required);
    let after_first = fixture.read_state();
    let first_state = &after_first.profiles["sub2api"];
    assert!(first_state.restart_required);
    let first_fingerprint = first_state
        .applied_runtime_fingerprint
        .clone()
        .expect("an active commit records the applied runtime fingerprint");
    let first_generation = first_state.generation;
    assert_eq!(
        first_fingerprint,
        crate::model_catalog::applied_runtime_fingerprint(
            &fixture.read_settings().relay_profiles[0],
            first_state,
        )
        .unwrap()
    );

    // Acknowledge the first restart so a second transition is observable rather than sticky.
    let mut acknowledged = after_first.clone();
    acknowledged
        .profiles
        .get_mut("sub2api")
        .unwrap()
        .restart_required = false;
    fs::write(
        &fixture.paths.catalog_state_path,
        serde_json::to_vec_pretty(&acknowledged).unwrap(),
    )
    .unwrap();

    let persisted = fixture.read_settings();
    let second = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            2,
        ),
    )
    .unwrap();

    assert!(
        !second.restart_required,
        "an identical consecutive active save must not re-fire the restart transition"
    );
    let after_second = fixture.read_state();
    let second_state = &after_second.profiles["sub2api"];
    assert!(!second_state.restart_required);
    assert_eq!(
        second_state.applied_runtime_fingerprint.as_deref(),
        Some(first_fingerprint.as_str())
    );
    assert_eq!(
        second_state.generation, first_generation,
        "an identical consecutive active save must not open a new catalog generation"
    );
}

#[test]
fn active_commit_refires_restart_when_the_runtime_contract_changes_without_a_catalog_generation() {
    let profile = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    let persisted = settings_with(vec![profile], "sub2api");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();
    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            1,
        ),
    )
    .unwrap();

    let baseline = fixture.read_state();
    let baseline_state = &baseline.profiles["sub2api"];
    let baseline_fingerprint = baseline_state.applied_runtime_fingerprint.clone().unwrap();
    let baseline_generation = baseline_state.generation;
    let mut acknowledged = baseline.clone();
    acknowledged
        .profiles
        .get_mut("sub2api")
        .unwrap()
        .restart_required = false;
    fs::write(
        &fixture.paths.catalog_state_path,
        serde_json::to_vec_pretty(&acknowledged).unwrap(),
    )
    .unwrap();

    // Move the selected provider to another non-reserved identifier: a runtime identity change
    // that leaves the effective catalog artifact byte-identical, so only the fingerprint may
    // drive the restart signal.
    let persisted = fixture.read_settings();
    let mut renamed = persisted.clone();
    let profile = &mut renamed.relay_profiles[0];
    let mut document = profile
        .config_contents
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let provider_id = document["model_provider"].as_str().unwrap().to_string();
    let table = document["model_providers"][&provider_id].clone();
    document["model_providers"]
        .as_table_like_mut()
        .unwrap()
        .remove(&provider_id);
    document["model_providers"]["RelayTwo"] = table;
    document["model_provider"] = toml_edit::value("RelayTwo");
    profile.config_contents = document.to_string();

    let second = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &renamed,
            "sub2api",
            ProviderCommitAction::Save,
            2,
        ),
    )
    .unwrap();

    assert!(second.restart_required);
    let after = fixture.read_state();
    let after_state = &after.profiles["sub2api"];
    assert!(after_state.restart_required);
    assert_ne!(
        after_state.applied_runtime_fingerprint.as_deref(),
        Some(baseline_fingerprint.as_str()),
        "a changed runtime contract must update the applied fingerprint"
    );
    assert_eq!(
        after_state.generation, baseline_generation,
        "a runtime-only change must not open a new catalog generation"
    );
}

#[test]
fn inactive_save_records_no_restart_marker_and_no_applied_runtime_fingerprint() {
    let active = pure_oauth_profile("official");
    let inactive = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    let persisted = settings_with(vec![active, inactive], "official");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();

    let mut next = persisted.clone();
    next.relay_profiles[1] = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay-updated.example/v1",
        "provider-key-updated",
    );

    let result = commit_provider_detail_from_paths(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 1),
    )
    .unwrap();

    assert!(!result.restart_required);
    let state = fixture.read_state();
    let profile_state = &state.profiles["sub2api"];
    assert!(!profile_state.restart_required);
    assert!(
        profile_state.applied_runtime_fingerprint.is_none(),
        "an inactive save applies no runtime, so it records no applied fingerprint"
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        live_before
    );
}

#[test]
fn failure_payloads_carry_the_static_rejecting_reason_without_dynamic_content() {
    let profile = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://provider-reason-secret.example/v1",
        "provider-key-sentinel",
    );
    let persisted = settings_with(vec![profile], "sub2api");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();

    let mut stale = request(
        &persisted,
        &persisted,
        "sub2api",
        ProviderCommitAction::Save,
        7,
    );
    stale.expected_provider_fingerprint = "sha256:stale".to_string();
    let stale_error = commit_provider_detail_from_paths(&fixture.paths, stale).unwrap_err();
    let stale_payload = ProviderCommitPayload::failure(7, stale_error.code(), stale_error.reason());
    assert_eq!(
        stale_payload.error_code,
        Some(ProviderCommitErrorCode::StaleState)
    );
    assert_eq!(
        stale_payload.reason,
        Some("provider state changed; reload or merge before saving")
    );

    // Two different InvalidDraft rejections must stay distinguishable in the payload.
    let mut prohibited_auth = persisted.clone();
    prohibited_auth.relay_profiles[0].auth_contents =
        r#"{"tokens":{"access_token":"oauth-output-sentinel"}}"#.to_string();
    let auth_error = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &prohibited_auth,
            "sub2api",
            ProviderCommitAction::Save,
            8,
        ),
    )
    .unwrap_err();
    assert_eq!(auth_error.code(), ProviderCommitErrorCode::InvalidDraft);

    let mut base_url_conflict = persisted.clone();
    base_url_conflict.relay_profiles[0].base_url =
        "https://provider-reason-other.example/v1".to_string();
    base_url_conflict.relay_profiles[0].upstream_base_url =
        "https://provider-reason-other.example/v1".to_string();
    let conflict_error = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &base_url_conflict,
            "sub2api",
            ProviderCommitAction::Save,
            9,
        ),
    )
    .unwrap_err();
    assert_eq!(conflict_error.code(), ProviderCommitErrorCode::InvalidDraft);
    assert_ne!(
        conflict_error.reason(),
        auth_error.reason(),
        "distinct InvalidDraft rules must remain distinguishable"
    );

    let payload = ProviderCommitPayload::failure(9, conflict_error.code(), conflict_error.reason());
    let encoded = serde_json::to_string(&payload).unwrap();
    for forbidden in [
        "provider-key-sentinel",
        "oauth-output-sentinel",
        "provider-reason-secret.example",
        "provider-reason-other.example",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "leaked {forbidden}: {encoded}"
        );
    }
}

#[test]
fn an_incomplete_native_contract_reports_its_own_reason_rather_than_the_generic_fallback() {
    let mut profile = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    // Drop the required top-level model: a native-priority contract gap that previously
    // collapsed into the catch-all normalization message.
    profile.config_contents = profile
        .config_contents
        .replace("model = \"gpt-5.6-sol\"\n", "");
    profile.model = String::new();
    let persisted = settings_with(vec![profile], "sub2api");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();

    let error = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            3,
        ),
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::InvalidDraft);
    assert_eq!(error.reason(), "provider model is required");
}

#[test]
fn a_contract_gap_names_the_field_it_is_missing() {
    let build = |mutate: &dyn Fn(&mut RelayProfile)| {
        let mut profile = canonical_profile(
            "sub2api",
            "gpt-5.6-sol",
            "https://relay.example/v1",
            "provider-key",
        );
        mutate(&mut profile);
        let persisted = settings_with(vec![profile], "sub2api");
        let fixture = Fixture::new(&persisted, &state_with_official());
        let persisted = fixture.read_settings();
        commit_provider_detail_from_paths(
            &fixture.paths,
            request(
                &persisted,
                &persisted,
                "sub2api",
                ProviderCommitAction::Save,
                3,
            ),
        )
        .unwrap_err()
        .reason()
    };

    assert_eq!(
        build(&|profile| {
            profile.config_contents = profile
                .config_contents
                .replace("model = \"gpt-5.6-sol\"\n", "");
            profile.model = String::new();
        }),
        "provider model is required"
    );
    assert_eq!(
        build(&|profile| {
            profile.config_contents = profile
                .config_contents
                .replace("base_url = \"https://relay.example/v1\"\n", "");
            profile.base_url = String::new();
            profile.upstream_base_url = String::new();
        }),
        "provider base URL is required"
    );
}

#[test]
fn a_degraded_contract_saves_instead_of_locking_the_profile_out_of_its_own_repair() {
    // A legacy `custom` provider that is one field short of the recognized upgradeable shape.
    // Rejecting it made the profile unrepairable: the missing field can only be persisted by a
    // save, and the save was what the contract check refused.
    let mut profile = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    profile.config_contents = r#"model_provider = "custom"

[model_providers.custom]
name = "custom"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
    .to_string();
    profile.model = String::new();
    let persisted = settings_with(vec![profile], "sub2api");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();

    let mut next = persisted.clone();
    next.relay_profiles[0].model = "gpt-5.6-sol".to_string();
    next.relay_profiles[0].config_contents = format!(
        "model = \"gpt-5.6-sol\"\n{}",
        next.relay_profiles[0].config_contents
    );

    let result = commit_provider_detail_from_paths(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 5),
    )
    .expect("a degraded contract must still be savable");

    assert_eq!(result.draft_revision, 5);
    let saved = fixture.read_settings();
    assert!(
        saved.relay_profiles[0]
            .config_contents
            .contains("gpt-5.6-sol")
    );
}

#[test]
fn a_disabled_routing_switch_saves_the_draft_without_writing_live_config() {
    let profile = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    let mut persisted = settings_with(vec![profile], "sub2api");
    persisted.relay_profiles_enabled = false;
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();
    let live_before = fs::read(fixture.paths.codex_home.join("config.toml")).unwrap();

    let mut next = persisted.clone();
    next.relay_profiles[0] = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay-updated.example/v1",
        "provider-key-updated",
    );

    let result = commit_provider_detail_from_paths(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 6),
    )
    .expect("the switch governs live writes, not whether a draft can be saved");

    assert!(!result.restart_required);
    assert_eq!(
        fixture.read_settings().relay_profiles[0].base_url,
        "https://relay-updated.example/v1"
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("config.toml")).unwrap(),
        live_before,
        "a disabled switch must leave live configuration untouched"
    );
}

/// Golden: the exact effective contract an active native-priority commit stages.
///
/// Pins the provider table byte-for-byte so a future normalization, core upgrade, or staging
/// change cannot quietly alter provider identity, name, wire API, auth requirement, bearer, or
/// the actor-authorization marker. Profile-scoped global keys must not reach live root.
const GOLDEN_STAGED_PROVIDER: &str = r#"[model_providers.RelayOne]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "provider-key"
http_headers = { "x-openai-actor-authorization" = "local-image-extension", "x-keep" = "yes" }
"#;

#[test]
fn golden_active_commit_stages_the_exact_actor_authorized_contract() {
    let mut profile = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    // Profile-scoped globals, one unrelated and one invalid for a provider draft. Neither may
    // reach live root, and neither may disturb the staged provider table.
    profile.config_contents = format!(
        "unrelated_profile_key = \"kept-out-of-live\"\nhide_agent_reasoning = true\n{}",
        profile.config_contents
    );
    let persisted = settings_with(vec![profile], "sub2api");
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted = fixture.read_settings();

    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            12,
        ),
    )
    .expect("the supplied custom OpenAI configuration commits");

    let live = fs::read_to_string(fixture.paths.codex_home.join("config.toml")).unwrap();
    let staged: toml_edit::DocumentMut = live.parse().unwrap();

    let mut rendered = toml_edit::DocumentMut::new();
    rendered["model_providers"] = staged["model_providers"].clone();
    // Leading decor travels with a grafted table and is not part of the contract.
    assert_eq!(rendered.to_string().trim_start(), GOLDEN_STAGED_PROVIDER);

    assert_eq!(
        staged
            .get("model_provider")
            .and_then(toml_edit::Item::as_str),
        Some("RelayOne")
    );
    for leaked in ["unrelated_profile_key", "hide_agent_reasoning"] {
        assert!(
            !staged.as_table().contains_key(leaked),
            "profile-scoped global {leaked} reached live root"
        );
    }
}

/// Golden: a legacy provider-ID alias belonging to another profile, exactly as authored.
///
/// The core storage normalizer rewrites this shape on sight — it renames the alias to its own
/// `custom` identity, drops the table it replaces along with the actor header, and restores
/// `requires_openai_auth = true`. A commit focused on a different profile must never apply that
/// to a bystander: this profile's upgrade is the user's to authorize, one profile at a time.
const GOLDEN_UNTOUCHED_BYSTANDER_ALIAS: &str = r#"model = "gpt-5.6-sol"
model_provider = "CodexPlusPlus"

[model_providers.CodexPlusPlus]
name = "OpenAI"
base_url = "https://bystander.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "bystander-key"
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
"#;

#[test]
fn golden_commit_never_migrates_a_bystander_profile_contract() {
    let mut bystander = canonical_profile(
        "bystander",
        "gpt-5.6-sol",
        "https://bystander.example/v1",
        "bystander-key",
    );
    bystander.config_contents = GOLDEN_UNTOUCHED_BYSTANDER_ALIAS.to_string();
    let persisted = settings_with(
        vec![
            canonical_profile(
                "sub2api",
                "gpt-5.6-sol",
                "https://relay.example/v1",
                "provider-key",
            ),
            bystander,
        ],
        "sub2api",
    );
    let fixture = Fixture::new(&persisted, &state_with_official());
    let persisted: BackendSettings =
        serde_json::from_slice(&fs::read(&fixture.paths.settings_path).unwrap()).unwrap();
    assert_eq!(
        raw_stored_profile_config(&fixture.paths.settings_path, "bystander"),
        GOLDEN_UNTOUCHED_BYSTANDER_ALIAS
    );

    let mut next = persisted.clone();
    next.relay_profiles[0] = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay-updated.example/v1",
        "provider-key",
    );

    commit_provider_detail_from_paths(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 21),
    )
    .expect("the focused profile commits");

    assert_eq!(
        raw_stored_profile_config(&fixture.paths.settings_path, "bystander"),
        GOLDEN_UNTOUCHED_BYSTANDER_ALIAS,
        "a commit migrated a profile the user never opened"
    );
}

#[test]
fn explicit_pure_oauth_exit_deletes_the_provider_without_leaving_a_dormant_copy() {
    use crate::provider_native_capability::{
        NativeCapabilityDraftAction, NativeCapabilityDraftConfirmation,
        NativeCapabilityDraftStatus, ProviderNativeCapabilityDraftRequest,
        transform_provider_native_capability_draft_from_paths,
    };

    let persisted = settings_with(
        vec![canonical_profile(
            "sub2api",
            "gpt-5.6-sol",
            "https://relay.example/v1",
            "provider-key",
        )],
        "sub2api",
    );
    let mut state = state_with_official();
    state.profiles.insert(
        "sub2api".to_string(),
        crate::model_catalog::ProfileCatalogState {
            mode: CatalogMode::OfficialPlusCustom,
            mode_explicit: true,
            ..crate::model_catalog::ProfileCatalogState::default()
        },
    );
    let fixture = Fixture::new(&persisted, &state);
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();

    // Stage the provider for real first, so live configuration carries the contract this exit is
    // supposed to remove. Asserting removal against a live file that never held it proves nothing.
    let seed = fixture.read_settings();
    commit_provider_detail_from_paths(
        &fixture.paths,
        request(&seed, &seed, "sub2api", ProviderCommitAction::Save, 29),
    )
    .expect("the provider stages before it is exited");
    let live_before = fs::read_to_string(fixture.paths.codex_home.join("config.toml")).unwrap();
    assert!(
        live_before.contains("RelayOne") && live_before.contains("provider-key"),
        "the exit must be measured against live configuration that holds the contract"
    );

    let persisted = fixture.read_settings();
    let config_before = raw_stored_profile_config(&fixture.paths.settings_path, "sub2api");

    let draft_request = |confirmations: Vec<NativeCapabilityDraftConfirmation>| {
        ProviderNativeCapabilityDraftRequest {
            draft_revision: 30,
            profile: persisted.relay_profiles[0].clone(),
            catalog_mode: CatalogMode::OfficialPlusCustom,
            action: NativeCapabilityDraftAction::ExitPureOAuth,
            source_config_contents: None,
            confirmations,
            replacement_provider_id: None,
        }
    };

    // The preview discloses the deletion, and discloses it without performing any of it.
    let preview = transform_provider_native_capability_draft_from_paths(
        &fixture.paths.settings_path,
        &fixture.paths.catalog_state_path,
        draft_request(vec![]),
    );
    assert_eq!(
        preview.status,
        NativeCapabilityDraftStatus::ConfirmationRequired
    );
    assert!(preview.preview.removes_provider_table);
    assert_eq!(
        preview.preview.removed_provider_id.as_deref(),
        Some("RelayOne")
    );
    assert!(
        preview
            .preview
            .removed_provider_fields
            .iter()
            .any(|field| field == "experimental_bearer_token"),
        "the preview must name the credential it deletes"
    );
    assert_eq!(
        raw_stored_profile_config(&fixture.paths.settings_path, "sub2api"),
        config_before,
        "a preview persisted a change"
    );

    let confirmed = transform_provider_native_capability_draft_from_paths(
        &fixture.paths.settings_path,
        &fixture.paths.catalog_state_path,
        draft_request(vec![
            NativeCapabilityDraftConfirmation::ConfirmDestructivePureOAuth,
        ]),
    );
    assert_eq!(confirmed.status, NativeCapabilityDraftStatus::Ready);
    assert_eq!(confirmed.draft.catalog_mode, CatalogMode::NativeOfficial);
    assert!(confirmed.draft.profile.base_url.is_empty());
    assert!(confirmed.draft.profile.upstream_base_url.is_empty());

    let mut next = persisted.clone();
    next.relay_profiles[0] = confirmed.draft.profile.clone();
    let mut commit = request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 30);
    commit.catalog_drafts = vec![ProfileCatalogDraft {
        mode: CatalogMode::NativeOfficial,
        mode_explicit: true,
        ..catalog_draft("sub2api")
    }];

    commit_provider_detail_from_paths(&fixture.paths, commit)
        .expect("an explicitly confirmed pure OAuth exit commits");

    // No dormant copy: not in the profile contract, not in any other persisted field, not live.
    let persisted_bytes = fs::read(&fixture.paths.settings_path).unwrap();
    let persisted_text = String::from_utf8(persisted_bytes).unwrap();
    for dormant in ["provider-key", "RelayOne", "x-openai-actor-authorization"] {
        assert!(
            !persisted_text.contains(dormant),
            "persisted settings retained {dormant} after the exit"
        );
    }
    let live = fs::read_to_string(fixture.paths.codex_home.join("config.toml")).unwrap();
    assert_ne!(
        live, live_before,
        "the active commit never reached live configuration, so the checks below prove nothing"
    );
    for dormant in ["provider-key", "RelayOne", "experimental_bearer_token"] {
        assert!(
            !live.contains(dormant),
            "live configuration retained {dormant} after the exit"
        );
    }

    // A non-external profile returns to the native official catalog.
    assert_eq!(
        fixture.read_state().profiles["sub2api"].mode,
        CatalogMode::NativeOfficial
    );

    // Official auth is the official client's to write.
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

#[test]
fn target_switching_changes_the_contract_only_on_commit_and_keeps_unowned_content() {
    use crate::provider_native_capability::{
        NativeCapabilityDraftAction, NativeCapabilityDraftConfirmation,
        NativeCapabilityDraftStatus, ProviderNativeCapabilityDraftRequest,
        transform_provider_native_capability_draft_from_paths,
    };

    let persisted = settings_with(
        vec![canonical_profile(
            "sub2api",
            // A slug the official baseline does not carry, so a custom-only target can represent
            // this profile's default model at all: an official slug is filtered out of the custom
            // entries, leaving a custom-only catalog with nothing in it.
            "relay-model",
            "https://relay.example/v1",
            "provider-key",
        )],
        "sub2api",
    );
    let mut state = state_with_official();
    state.profiles.insert(
        "sub2api".to_string(),
        crate::model_catalog::ProfileCatalogState {
            mode: CatalogMode::OfficialPlusCustom,
            mode_explicit: true,
            ..crate::model_catalog::ProfileCatalogState::default()
        },
    );
    let fixture = Fixture::new(&persisted, &state);

    let mut revision = 40;
    for (action, expected_name, expected_auth) in [
        (NativeCapabilityDraftAction::ExitPureApi, "OpenAI", false),
        (
            NativeCapabilityDraftAction::ExitLegacyCompatibility,
            "custom",
            true,
        ),
        (
            NativeCapabilityDraftAction::EnableNativePriority,
            "OpenAI",
            true,
        ),
    ] {
        revision += 1;
        let persisted = fixture.read_settings();
        let before = raw_stored_profile_config(&fixture.paths.settings_path, "sub2api");

        let payload = transform_provider_native_capability_draft_from_paths(
            &fixture.paths.settings_path,
            &fixture.paths.catalog_state_path,
            ProviderNativeCapabilityDraftRequest {
                draft_revision: revision,
                profile: persisted.relay_profiles[0].clone(),
                catalog_mode: CatalogMode::OfficialPlusCustom,
                action,
                source_config_contents: None,
                confirmations: vec![
                    NativeCapabilityDraftConfirmation::ConfirmCapabilityLoss,
                    NativeCapabilityDraftConfirmation::UseStructuredKey,
                ],
                replacement_provider_id: None,
            },
        );
        assert_eq!(
            payload.status,
            NativeCapabilityDraftStatus::Ready,
            "{action:?} was not ready: {:?}",
            payload.blockers
        );
        assert_eq!(
            raw_stored_profile_config(&fixture.paths.settings_path, "sub2api"),
            before,
            "{action:?} changed the persisted contract before any commit"
        );

        let mut next = persisted.clone();
        next.relay_profiles[0] = payload.draft.profile.clone();
        let mut commit = request(
            &persisted,
            &next,
            "sub2api",
            ProviderCommitAction::Save,
            revision,
        );
        commit.catalog_drafts = vec![ProfileCatalogDraft {
            mode: payload.draft.catalog_mode,
            mode_explicit: true,
            // A target without official catalog access carries its own model, so planning can
            // represent the profile's default instead of failing as catalog-unavailable.
            overlay: CatalogOverlay {
                custom: vec![CustomModel {
                    slug: "relay-model".to_string(),
                    display_name: "Relay Model".to_string(),
                    ..CustomModel::default()
                }],
                ..CatalogOverlay::default()
            },
            ..catalog_draft("sub2api")
        }];
        commit_provider_detail_from_paths(&fixture.paths, commit)
            .unwrap_or_else(|error| panic!("{action:?} must commit: {error:?}"));

        let stored = raw_stored_profile_config(&fixture.paths.settings_path, "sub2api");
        assert_ne!(stored, before, "{action:?} committed no change");
        let document: toml_edit::DocumentMut = stored.parse().unwrap();
        let provider = document["model_providers"]["RelayOne"]
            .as_table_like()
            .unwrap();
        assert_eq!(
            provider.get("name").and_then(toml_edit::Item::as_str),
            Some(expected_name),
            "{action:?}"
        );
        assert_eq!(
            provider
                .get("requires_openai_auth")
                .and_then(toml_edit::Item::as_bool),
            Some(expected_auth),
            "{action:?}"
        );
        // The header the manager does not own survives every target it is carried through.
        assert!(
            stored.contains("\"x-keep\" = \"yes\""),
            "{action:?} dropped an unowned provider header"
        );
    }

    let final_config = raw_stored_profile_config(&fixture.paths.settings_path, "sub2api");
    assert!(
        final_config.contains("x-openai-actor-authorization"),
        "returning to native priority must restore the actor marker"
    );
}

/// Pinned-core compatibility: the dependency semantics this contract is built on.
///
/// The manager stages a provider table and then trusts the pinned core to read it, preserve it,
/// and keep its own rewriting rules stable. Each assertion below is a semantic the native
/// capability contract depends on. A future core upgrade that changes one of these must fail
/// here and be evaluated, rather than silently changing what a saved provider means.
#[test]
fn pinned_core_semantics_for_the_staged_contract_are_unchanged() {
    let provider_id = |config: &str| {
        config.parse::<toml_edit::DocumentMut>().unwrap()["model_provider"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let provider_flag = |config: &str, field: &str| {
        let document = config.parse::<toml_edit::DocumentMut>().unwrap();
        let id = document["model_provider"].as_str().unwrap().to_string();
        document["model_providers"][&id][field].as_bool()
    };
    let staged = format!(
        "model = \"gpt-5.6-sol\"\nmodel_provider = \"RelayOne\"\n\n{GOLDEN_STAGED_PROVIDER}"
    );
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join("config.toml"), &staged).unwrap();

    // 1. The staged contract still reads as a configured provider that does not require official
    //    auth and does carry a bearer.
    let status = codex_plus_core::relay_config::relay_config_status_from_home(home.path());
    assert!(
        !status.configured,
        "core's `configured` now accepts the native-priority contract; the manager decides \
         readiness itself precisely because `configured` still means the legacy shape, where \
         requires_openai_auth is true"
    );
    assert!(
        !status.requires_openai_auth,
        "core stopped reading requires_openai_auth = false"
    );
    assert!(
        status.has_bearer_token,
        "core stopped reading experimental_bearer_token"
    );

    // 2. Storage normalization preserves the canonical contract byte-for-byte, including the
    //    provider name, the Responses wire API, the actor marker, and unowned headers.
    let mut canonical = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    let authored = canonical.config_contents.clone();
    codex_plus_core::relay_config::normalize_relay_profile_for_storage(&mut canonical).unwrap();
    assert_eq!(
        canonical.config_contents, authored,
        "core normalization now rewrites the canonical contract"
    );

    // 3. A reserved provider identifier is still rewritten away, which is why the contract
    //    refuses one instead of trying to keep it.
    let mut reserved = canonical_profile(
        "reserved",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    reserved.config_contents = reserved.config_contents.replace("RelayOne", "openai");
    codex_plus_core::relay_config::normalize_relay_profile_for_storage(&mut reserved).unwrap();
    assert_eq!(
        provider_id(&reserved.config_contents),
        "custom",
        "core stopped rewriting a reserved provider identifier"
    );

    // 4. A legacy alias is still rewritten away, and still loses the table it replaces. This is
    //    why an alias must be renamed by an explicit action instead of being carried forward.
    let mut alias = canonical_profile(
        "alias",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    alias.config_contents = alias.config_contents.replace("RelayOne", "CodexPlusPlus");
    codex_plus_core::relay_config::normalize_relay_profile_for_storage(&mut alias).unwrap();
    assert_eq!(
        provider_id(&alias.config_contents),
        "custom",
        "core stopped rewriting a legacy provider alias"
    );
    assert!(
        !alias
            .config_contents
            .contains("x-openai-actor-authorization"),
        "core now preserves the actor header across an alias rewrite; the rename rule can relax"
    );

    // 5. `OpenAI` is a normal profile-scoped identifier: core's reserved list holds the built-in
    //    lowercase `openai`, and the match is case-sensitive. New drafts default to `OpenAI` and
    //    an upgrade preserves it, so a rewrite here would silently rename every shipped profile.
    let mut mixed_case = canonical_profile(
        "mixed-case",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    mixed_case.config_contents = mixed_case.config_contents.replace("RelayOne", "OpenAI");
    let authored_mixed_case = mixed_case.config_contents.clone();
    codex_plus_core::relay_config::normalize_relay_profile_for_storage(&mut mixed_case).unwrap();
    assert_eq!(
        provider_id(&mixed_case.config_contents),
        "OpenAI",
        "core now treats `OpenAI` as reserved or legacy; the default identifier must change"
    );
    assert_eq!(
        mixed_case.config_contents, authored_mixed_case,
        "core now rewrites a profile whose identifier is `OpenAI`"
    );

    // 5. An absent gpt-5.6-soluth requirement is still defaulted to true, which is why the
    //    startup credential migration must not decide that field.
    let mut absent = canonical_profile(
        "absent",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    absent.config_contents = absent
        .config_contents
        .replace("requires_openai_auth = false\n", "");
    codex_plus_core::relay_config::normalize_relay_profile_for_storage(&mut absent).unwrap();
    assert_eq!(
        provider_flag(&absent.config_contents, "requires_openai_auth"),
        Some(true),
        "core stopped defaulting requires_openai_auth to true"
    );
}

/// Sentinel audit: nothing outside the two files that are supposed to hold a credential may
/// contain one, and no payload, error, or log detail may carry either kind of secret.
#[test]
fn sentinel_credentials_never_reach_artifacts_payloads_errors_or_logs() {
    const PROVIDER_SENTINEL: &str = "SENTINEL-PROVIDER-KEY-a1b2";
    const OAUTH_ACCOUNT: &str = "SENTINEL-OAUTH-ACCOUNT";
    const OAUTH_WORKSPACE: &str = "SENTINEL-OAUTH-WORKSPACE";

    let persisted = settings_with(
        vec![canonical_profile(
            "sub2api",
            "gpt-5.6-sol",
            "https://relay.example/v1",
            PROVIDER_SENTINEL,
        )],
        "sub2api",
    );
    let scope_salt = "provider-commit-test-salt".to_string();
    let state = CatalogState {
        scope_salt: scope_salt.clone(),
        official: Some(OfficialSnapshot {
            client_version: "0.147.0".to_string(),
            scope_hash: official_scope_hash(&scope_salt, OAUTH_ACCOUNT, OAUTH_WORKSPACE),
            raw_catalog: official_catalog(),
            ..OfficialSnapshot::default()
        }),
        target: Some(target_identity("0.147.0", "target-a")),
        ..CatalogState::default()
    };
    let fixture = Fixture::new(&persisted, &state);
    fs::write(
        fixture.paths.codex_home.join("auth.json"),
        official_auth_bytes(OAUTH_ACCOUNT, OAUTH_WORKSPACE),
    )
    .unwrap();
    let persisted = fixture.read_settings();

    let payload = commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &persisted,
            "sub2api",
            ProviderCommitAction::Save,
            50,
        ),
    )
    .expect("the sentinel provider commits");

    // Artifacts. Exactly two files are entitled to a credential: the persisted settings the
    // manager owns, and the live configuration Codex reads. Everything the transaction leaves
    // behind — catalogs, catalog state, staging remnants, recovery material, backups — must not
    // carry one, and nothing at all may carry the official account identity except auth.json.
    let entitled_to_provider_key = ["app-state/settings.json", "codex-home/config.toml"];
    for (path, bytes) in fixture.file_generation() {
        let text = String::from_utf8_lossy(&bytes).to_string();
        if !entitled_to_provider_key.contains(&path.as_str()) {
            assert!(
                !text.contains(PROVIDER_SENTINEL),
                "provider key leaked into {path}"
            );
        }
        if path != "codex-home/auth.json" {
            for identity in [OAUTH_ACCOUNT, OAUTH_WORKSPACE] {
                assert!(!text.contains(identity), "{identity} leaked into {path}");
            }
        }
    }

    // Payloads: the commit result and the native-capability inspection.
    let inspection =
        crate::provider_native_capability::inspect_provider_native_capabilities_from_paths(
            &fixture.paths.settings_path,
            &fixture.paths.catalog_state_path,
            crate::provider_native_capability::ProviderNativeCapabilityInspectionRequest::default(),
        )
        .unwrap();
    let commit_surface = serde_json::to_string(&payload).unwrap();
    let status_surfaces = [("inspection", serde_json::to_string(&inspection).unwrap())];
    // The commit reply is the local provider-detail IPC the editor round-trips, so it is allowed
    // to carry the bearer back. It may still never carry official identity.
    for identity in [OAUTH_ACCOUNT, OAUTH_WORKSPACE] {
        assert!(
            !commit_surface.contains(identity),
            "{identity} reached the commit reply"
        );
    }
    for (name, surface) in &status_surfaces {
        for secret in [PROVIDER_SENTINEL, OAUTH_ACCOUNT, OAUTH_WORKSPACE] {
            assert!(
                !surface.contains(secret),
                "{secret} reached the {name} surface"
            );
        }
    }

    // Errors: a refused commit must not describe the credential it refused.
    let mut stale = persisted.clone();
    stale.relay_profiles[0].api_key = format!("{PROVIDER_SENTINEL}-changed");
    let failure = commit_provider_detail_from_paths(
        &fixture.paths,
        request(&stale, &stale, "sub2api", ProviderCommitAction::Save, 51),
    )
    .expect_err("a mismatched compare-and-swap baseline is refused");
    let rendered = format!("{} {} {failure:?}", failure.reason(), failure);
    for secret in [PROVIDER_SENTINEL, OAUTH_ACCOUNT, OAUTH_WORKSPACE] {
        assert!(
            !rendered.contains(secret),
            "{secret} reached a failure payload"
        );
    }

    // Doctor: an upstream answer that echoes the credential is redacted before it is shown.
    let doctor_profile = persisted.relay_profiles[0].clone();
    let doctor = crate::commands::sanitize_provider_doctor_result(
        &doctor_profile,
        crate::commands::CommandResult {
            status: "ok".to_string(),
            message: format!("upstream echoed {PROVIDER_SENTINEL}"),
            payload: crate::commands::ProviderDoctorPayload {
                profile_name: PROVIDER_SENTINEL.to_string(),
                model: format!("model {PROVIDER_SENTINEL}"),
                summary: format!("summary {PROVIDER_SENTINEL}"),
                recommendation: format!("recommendation {PROVIDER_SENTINEL}"),
                checks: vec![crate::commands::ProviderDoctorCheck {
                    id: "auth".to_string(),
                    title: format!("title {PROVIDER_SENTINEL}"),
                    status: "ok".to_string(),
                    detail: format!("detail {PROVIDER_SENTINEL}"),
                }],
                compatibility_fallback_used: false,
                initial_http_status: None,
                request_http_status: None,
            },
        },
    );
    let rendered = format!(
        "{} {}",
        doctor.message,
        serde_json::to_string(&doctor.payload).unwrap()
    );
    assert!(
        !rendered.contains(PROVIDER_SENTINEL),
        "provider key reached the Doctor payload"
    );

    // Logs: diagnostic details are redacted before they are ever appended.
    for event in [
        "provider.commit",
        "manager.normalize_relay_profile_for_storage.failed",
        "manager.start",
    ] {
        let sanitized = crate::commands::sanitize_diagnostic_detail_for_event(
            event,
            json!({
                "apiKey": PROVIDER_SENTINEL,
                "configContents": format!("experimental_bearer_token = \"{PROVIDER_SENTINEL}\"\n"),
                "authContents": format!("{{\"account_id\":\"{OAUTH_ACCOUNT}\"}}"),
                "nested": { "workspace": OAUTH_WORKSPACE }
            }),
        );
        let rendered = serde_json::to_string(&sanitized).unwrap();
        for secret in [PROVIDER_SENTINEL, OAUTH_ACCOUNT, OAUTH_WORKSPACE] {
            assert!(
                !rendered.contains(secret),
                "{secret} survived diagnostic sanitization of {event}"
            );
        }
    }
}

#[test]
fn injected_staging_failure_is_typed_as_staging_rejected_and_mutates_nothing() {
    let active = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
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
            60,
        ),
        |checkpoint| {
            if checkpoint == ProviderCommitCheckpoint::Staging {
                anyhow::bail!("staging-fault-sentinel");
            }
            Ok(())
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), ProviderCommitErrorCode::StagingRejected);
    assert_eq!(error.reason(), "provider staging failed");
    assert!(!error.to_string().contains("staging-fault-sentinel"));
    assert_eq!(
        fixture.file_generation(),
        before,
        "a rejected staging must leave the complete prior generation"
    );
}

#[test]
fn a_legacy_custom_profile_reaches_the_canonical_contract_at_the_real_entry_points() {
    use crate::provider_native_capability::{
        NativeCapabilityDraftAction, NativeCapabilityDraftStatus, NativeCapabilityState,
        ProviderNativeCapabilityDraftRequest, inspect_profile,
        transform_provider_native_capability_draft_from_paths,
    };

    // The shape a user actually arrives with: provider name `custom`, official auth still
    // required, no actor marker, and no default model. The missing model is the blocking gap, so
    // the upgrade is withheld until it is supplied.
    let mut legacy = canonical_profile(
        "sub2api",
        "gpt-5.6-sol",
        "https://relay.example/v1",
        "provider-key",
    );
    legacy.config_contents = r#"model_provider = "custom"

[model_providers.custom]
name = "custom"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = true
experimental_bearer_token = "provider-key"
"#
    .to_string();
    legacy.model = String::new();

    let mut bystander = canonical_profile(
        "bystander",
        "gpt-5.6-sol",
        "https://bystander.example/v1",
        "bystander-key",
    );
    bystander.config_contents = GOLDEN_UNTOUCHED_BYSTANDER_ALIAS.to_string();

    let mut state = state_with_official();
    for id in ["sub2api", "bystander"] {
        state.profiles.insert(
            id.to_string(),
            crate::model_catalog::ProfileCatalogState {
                mode: CatalogMode::OfficialPlusCustom,
                mode_explicit: true,
                ..crate::model_catalog::ProfileCatalogState::default()
            },
        );
    }
    let fixture = Fixture::new(&settings_with(vec![legacy, bystander], "sub2api"), &state);
    let auth_before = fs::read(fixture.paths.codex_home.join("auth.json")).unwrap();
    let bystander_before = raw_stored_profile_config(&fixture.paths.settings_path, "bystander");

    let persisted = fixture.read_settings();
    assert_eq!(
        inspect_profile(
            &persisted.relay_profiles[0],
            CatalogMode::OfficialPlusCustom
        )
        .state,
        NativeCapabilityState::Degraded,
        "a profile missing its default model cannot reach its own upgrade yet"
    );

    // Step 1: supply the missing input and save. This is the step the old gate refused, which is
    // what stranded the profile: the input can only be persisted by a save.
    let mut next = persisted.clone();
    next.relay_profiles[0].model = "gpt-5.6-sol".to_string();
    next.relay_profiles[0].config_contents = format!(
        "model = \"gpt-5.6-sol\"\n{}",
        next.relay_profiles[0].config_contents
    );
    commit_provider_detail_from_paths(
        &fixture.paths,
        request(&persisted, &next, "sub2api", ProviderCommitAction::Save, 70),
    )
    .expect("a repairable draft must save so the contract can be completed in steps");

    let persisted = fixture.read_settings();
    assert_eq!(
        inspect_profile(
            &persisted.relay_profiles[0],
            CatalogMode::OfficialPlusCustom
        )
        .state,
        NativeCapabilityState::UpgradeAvailable,
        "with its input supplied, the profile must now be able to reach its upgrade"
    );

    // Step 2: one explicit revisioned transform at the real command boundary.
    let payload = transform_provider_native_capability_draft_from_paths(
        &fixture.paths.settings_path,
        &fixture.paths.catalog_state_path,
        ProviderNativeCapabilityDraftRequest {
            draft_revision: 71,
            profile: persisted.relay_profiles[0].clone(),
            catalog_mode: CatalogMode::OfficialPlusCustom,
            action: NativeCapabilityDraftAction::EnableNativePriority,
            source_config_contents: None,
            confirmations: vec![],
            replacement_provider_id: None,
        },
    );
    assert_eq!(payload.status, NativeCapabilityDraftStatus::Ready);
    assert_eq!(payload.draft_revision, 71);
    assert_eq!(
        raw_stored_profile_config(&fixture.paths.settings_path, "sub2api"),
        persisted.relay_profiles[0].config_contents,
        "the transform must not persist anything on its own"
    );

    // Step 3: commit the upgraded draft.
    let mut upgraded = persisted.clone();
    upgraded.relay_profiles[0] = payload.draft.profile.clone();
    commit_provider_detail_from_paths(
        &fixture.paths,
        request(
            &persisted,
            &upgraded,
            "sub2api",
            ProviderCommitAction::Save,
            71,
        ),
    )
    .expect("the upgraded contract commits");

    let committed = fixture.read_settings();
    assert_eq!(
        inspect_profile(
            &committed.relay_profiles[0],
            CatalogMode::OfficialPlusCustom
        )
        .state,
        NativeCapabilityState::NativePriority
    );
    let stored = raw_stored_profile_config(&fixture.paths.settings_path, "sub2api");
    let document: toml_edit::DocumentMut = stored.parse().unwrap();
    let provider = document["model_providers"]["custom"]
        .as_table_like()
        .unwrap();
    assert_eq!(
        provider.get("name").and_then(toml_edit::Item::as_str),
        Some("OpenAI")
    );
    assert_eq!(
        provider
            .get("requires_openai_auth")
            .and_then(toml_edit::Item::as_bool),
        Some(true)
    );
    assert!(stored.contains("x-openai-actor-authorization"));

    // Nothing else moved: no other profile was migrated, and auth.json was never written.
    assert_eq!(
        raw_stored_profile_config(&fixture.paths.settings_path, "bystander"),
        bystander_before,
        "another profile was migrated by this upgrade"
    );
    assert_eq!(
        fs::read(fixture.paths.codex_home.join("auth.json")).unwrap(),
        auth_before
    );
}

#[test]
fn a_second_save_in_one_session_is_accepted_without_reloading_the_editor() {
    // The editor's compare-and-swap baseline is whatever the previous save answered with. If a
    // commit answers with the draft it planned rather than the generation the next read will show,
    // every later save in that session reports stale state and only restarting the app clears it.
    let profile = canonical_profile(
        "relay-a",
        "gpt-5.6-sol",
        "https://a.example/v1",
        "provider-key-a",
    );
    let initial = settings_with(vec![profile], "relay-a");
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();

    let context_draft = |window: u64| {
        let mut draft = catalog_draft("relay-a");
        draft.mode = CatalogMode::NativeOfficial;
        draft.mode_explicit = true;
        draft.overlay.official.insert(
            "gpt-5.6-sol".to_string(),
            crate::model_catalog::OfficialOverride {
                context_window: Some(window),
                ..Default::default()
            },
        );
        draft
    };

    let first = ProviderCommitRequest {
        topology: ProviderOwnedTopologyDraft::from_settings(&persisted),
        catalog_drafts: vec![context_draft(372_000)],
        focused_profile_id: Some("relay-a".to_string()),
        action: ProviderCommitAction::Save,
        previous_active_relay_id: persisted.active_relay_id.clone(),
        confirm_context_cleanup: false,
        draft_revision: 1,
        expected_provider_fingerprint: provider_owned_fingerprint(
            &ProviderOwnedTopologyDraft::from_settings(&persisted),
        )
        .unwrap(),
    };
    let first_payload = commit_provider_detail_from_paths(&fixture.paths, first).unwrap();
    let editor_baseline = first_payload
        .settings
        .clone()
        .expect("a successful save answers with the settings the editor keeps");

    // No reload happens here: the editor edits the generation the first save handed back.
    let second = ProviderCommitRequest {
        topology: ProviderOwnedTopologyDraft::from_settings(&editor_baseline),
        catalog_drafts: vec![context_draft(400_000)],
        focused_profile_id: Some("relay-a".to_string()),
        action: ProviderCommitAction::Save,
        previous_active_relay_id: editor_baseline.active_relay_id.clone(),
        confirm_context_cleanup: false,
        draft_revision: 2,
        expected_provider_fingerprint: first_payload.provider_fingerprint.clone(),
    };

    commit_provider_detail_from_paths(&fixture.paths, second)
        .expect("a second save must not require an application restart");

    assert_eq!(
        fixture.read_state().profiles["relay-a"].overlay.official["gpt-5.6-sol"].context_window,
        Some(400_000)
    );
}

#[test]
fn a_context_override_on_a_native_profile_generates_the_catalog_it_needs() {
    // Native mode generates no catalog and points at none, so an override saved there would be
    // stored and then ignored. The editor promotes the mode when a value is typed; this is the
    // backend half — the promoted save has to actually produce the generation.
    let profile = canonical_profile(
        "relay-a",
        "gpt-5.6-sol",
        "https://a.example/v1",
        "provider-key-a",
    );
    let initial = settings_with(vec![profile], "relay-a");
    let mut catalog_state = state_with_official();
    catalog_state
        .profiles
        .entry("relay-a".to_string())
        .or_default()
        .mode = CatalogMode::NativeOfficial;
    let fixture = Fixture::new(&initial, &catalog_state);
    let persisted = fixture.read_settings();
    assert_eq!(
        fixture.read_state().profiles["relay-a"].generated_path,
        None,
        "native mode starts with nothing generated"
    );

    let mut promoted = catalog_draft("relay-a");
    promoted.mode = CatalogMode::OfficialPlusCustom;
    promoted.mode_explicit = true;
    promoted.overlay.official.insert(
        "gpt-5.6-sol".to_string(),
        crate::model_catalog::OfficialOverride {
            context_window: Some(372_000),
            ..Default::default()
        },
    );
    let request = ProviderCommitRequest {
        topology: ProviderOwnedTopologyDraft::from_settings(&persisted),
        catalog_drafts: vec![promoted],
        focused_profile_id: Some("relay-a".to_string()),
        action: ProviderCommitAction::Save,
        previous_active_relay_id: persisted.active_relay_id.clone(),
        confirm_context_cleanup: false,
        draft_revision: 1,
        expected_provider_fingerprint: provider_owned_fingerprint(
            &ProviderOwnedTopologyDraft::from_settings(&persisted),
        )
        .unwrap(),
    };

    let payload = commit_provider_detail_from_paths(&fixture.paths, request).unwrap();

    let saved = fixture.read_state().profiles["relay-a"].clone();
    assert_eq!(saved.mode, CatalogMode::OfficialPlusCustom);
    assert_eq!(
        saved.overlay.official["gpt-5.6-sol"].context_window,
        Some(372_000)
    );
    let generated = saved
        .generated_path
        .as_ref()
        .expect("the promoted save generated a catalog");
    assert!(
        fixture.paths.codex_home.join(generated).is_file(),
        "the generated catalog exists on disk"
    );
    // The profile has to point at it, or Codex would keep reading its own built-in list.
    // Codex reads live config.toml, so that is where the pointer has to land for the active
    // profile; without it Codex keeps reading its own built-in list and the number changes nothing.
    let live = fs::read_to_string(fixture.paths.codex_home.join("config.toml")).unwrap();
    assert!(
        live.contains(&format!("model_catalog_json = \"{generated}\"")),
        "live config points at the generated catalog: {live}"
    );
    assert!(
        payload.restart_required,
        "a new generation only reaches Codex after a restart"
    );
}

/// The fingerprint a save answers with must be the fingerprint the next save is judged against.
///
/// The editor keeps whatever the last save returned as its compare-and-swap baseline. If that value
/// described the draft the transaction planned rather than the generation now on disk, every later
/// save in the session would report stale state and only restarting the application — which reads
/// the settings fresh — would clear it. That is the reported failure, so this pins the invariant
/// even though it currently holds: the accounting has three chances to disagree (the plan, the
/// sanitized payload, and the file) and only one of them is what a later save is compared against.
#[test]
fn the_fingerprint_a_save_returns_is_the_one_the_next_save_is_judged_against() {
    let context_draft = || {
        let mut draft = catalog_draft("relay-a");
        draft.mode = CatalogMode::NativeOfficial;
        draft.mode_explicit = true;
        draft.overlay.official.insert(
            "gpt-5.6-sol".to_string(),
            crate::model_catalog::OfficialOverride {
                context_window: Some(372_000),
                ..Default::default()
            },
        );
        draft
    };

    let shapes: Vec<(&str, RelayProfile)> = vec![
        (
            "official mixed with an API key",
            canonical_profile(
                "relay-a",
                "gpt-5.6-sol",
                "https://a.example/v1",
                "provider-key-a",
            ),
        ),
        ("official OAuth only", pure_oauth_profile("relay-a")),
    ];

    for (shape, profile) in shapes {
        let initial = settings_with(vec![profile], "relay-a");
        let fixture = Fixture::new(&initial, &state_with_official());
        let persisted = fixture.read_settings();

        let payload = commit_provider_detail_from_paths(
            &fixture.paths,
            ProviderCommitRequest {
                topology: ProviderOwnedTopologyDraft::from_settings(&persisted),
                catalog_drafts: vec![context_draft()],
                focused_profile_id: Some("relay-a".to_string()),
                action: ProviderCommitAction::Save,
                previous_active_relay_id: persisted.active_relay_id.clone(),
                confirm_context_cleanup: false,
                draft_revision: 1,
                expected_provider_fingerprint: provider_owned_fingerprint(
                    &ProviderOwnedTopologyDraft::from_settings(&persisted),
                )
                .unwrap(),
            },
        )
        .unwrap_or_else(|error| panic!("{shape}: first save failed: {error:?}"));

        // The settings handed back must describe the same generation as the fingerprint handed
        // back, or the editor's next request carries a topology that fingerprint does not cover.
        let returned = payload
            .settings
            .clone()
            .expect("a successful save answers with settings");
        assert_eq!(
            provider_owned_fingerprint(&ProviderOwnedTopologyDraft::from_settings(&returned))
                .unwrap(),
            payload.provider_fingerprint,
            "{shape}: the returned settings and fingerprint describe different generations",
        );

        // And it must match the generation the next save is actually compared against: the settings
        // as they are read back from disk.
        let reread = fixture.read_settings();
        assert_eq!(
            payload.provider_fingerprint,
            provider_owned_fingerprint(&ProviderOwnedTopologyDraft::from_settings(&reread))
                .unwrap(),
            "{shape}: the editor was handed a fingerprint no later save can match",
        );
    }
}

/// A leftover catalog pointer must not make a profile unsaveable.
///
/// An older version wrote `model_catalog_json` into a profile's stored config. On the next load,
/// migration read any pointer as external ownership, and an ordinary save must preserve every
/// external pointer — but the editor sends a native or managed draft that carries none, so the save
/// was rejected as `InvalidDraft`. The profile could not be repaired either, because correcting the
/// mode is itself a save. Only a pointer at the path this manager generates for this profile is
/// treated as our own leftover; a pointer the user chose still means what it says.
#[test]
fn a_profile_carrying_a_stale_catalog_pointer_can_still_be_saved() {
    let mut profile = canonical_profile(
        "relay-a",
        "gpt-5.6-sol",
        "https://a.example/v1",
        "provider-key-a",
    );
    // The shape an older version actually left behind: our own generated path, in stored config.
    profile.config_contents = format!(
        "model_catalog_json = \"{}\"\n{}",
        crate::model_catalog::generated_relative_path("relay-a"),
        profile.config_contents
    );
    let initial = settings_with(vec![profile], "relay-a");
    let fixture = Fixture::new(&initial, &state_with_official());
    let persisted = fixture.read_settings();

    let mut draft = catalog_draft("relay-a");
    draft.mode = CatalogMode::NativeOfficial;
    draft.mode_explicit = true;
    draft.overlay.official.insert(
        "gpt-5.6-sol".to_string(),
        crate::model_catalog::OfficialOverride {
            context_window: Some(372_000),
            ..Default::default()
        },
    );

    commit_provider_detail_from_paths(
        &fixture.paths,
        ProviderCommitRequest {
            topology: ProviderOwnedTopologyDraft::from_settings(&persisted),
            catalog_drafts: vec![draft],
            focused_profile_id: Some("relay-a".to_string()),
            action: ProviderCommitAction::Save,
            previous_active_relay_id: persisted.active_relay_id.clone(),
            confirm_context_cleanup: false,
            draft_revision: 1,
            expected_provider_fingerprint: provider_owned_fingerprint(
                &ProviderOwnedTopologyDraft::from_settings(&persisted),
            )
            .unwrap(),
        },
    )
    .expect("a leftover catalog pointer must not make a profile unsaveable");
}

/// Builds a managed profile whose default model an application update's bundled baseline no
/// longer carries at all, with a generation already on disk — the state an app update leaves
/// behind. `gpt-5.2` is the real instance: one shipped asset carried it listed, the corrected
/// asset does not carry it, and any profile saved against the old asset wakes up exactly here.
fn stranded_default_fixture(active: &str) -> (Fixture, Vec<u8>) {
    let stranded = canonical_profile(
        "stranded",
        "gpt-5.2",
        "https://stranded.example/v1",
        "provider-key",
    );
    let other = pure_oauth_profile("main");
    let initial = settings_with(vec![stranded, other], active);
    let mut state = state_with_official();
    let generated_path = crate::model_catalog::generated_relative_path("stranded");
    state.profiles.insert(
        "stranded".to_string(),
        crate::model_catalog::ProfileCatalogState {
            mode: CatalogMode::OfficialPlusCustom,
            mode_explicit: true,
            generated_path: Some(generated_path.clone()),
            generated_hash: Some("hash-from-the-previous-app-version".to_string()),
            generation: 3,
            ..crate::model_catalog::ProfileCatalogState::default()
        },
    );
    let fixture = Fixture::new(&initial, &state);
    let artifact = fixture.paths.codex_home.join(&generated_path);
    fs::create_dir_all(artifact.parent().unwrap()).unwrap();
    let effective_catalog = b"{\"models\":[{\"slug\":\"gpt-5.2\"}]}".to_vec();
    fs::write(&artifact, &effective_catalog).unwrap();
    (fixture, effective_catalog)
}

fn stranded_default_request(persisted: &BackendSettings, revision: u64) -> ProviderCommitRequest {
    let mut draft = catalog_draft("stranded");
    draft.mode_explicit = true;
    ProviderCommitRequest {
        topology: ProviderOwnedTopologyDraft::from_settings(persisted),
        catalog_drafts: vec![draft],
        focused_profile_id: Some("stranded".to_string()),
        action: ProviderCommitAction::Save,
        previous_active_relay_id: persisted.active_relay_id.clone(),
        confirm_context_cleanup: false,
        draft_revision: revision,
        expected_provider_fingerprint: provider_owned_fingerprint(
            &ProviderOwnedTopologyDraft::from_settings(persisted),
        )
        .unwrap(),
    }
}

#[test]
fn a_bundled_update_that_retires_the_default_model_keeps_the_catalog_for_continuity() {
    let (fixture, effective_catalog) = stranded_default_fixture("main");
    let persisted = fixture.read_settings();
    let generated_path = crate::model_catalog::generated_relative_path("stranded");

    commit_provider_detail_from_paths(&fixture.paths, stranded_default_request(&persisted, 7))
        .expect("a retired default on an inactive profile must not block the save");

    // The profile is reported as needing a replacement default, not silently invalidated: the
    // last valid generation — file, hash, path, and counter — survives untouched.
    let state = fixture.read_state();
    let profile_state = state.profiles.get("stranded").unwrap();
    assert_eq!(
        profile_state.action_required.as_deref(),
        Some("catalog-readiness-unavailable")
    );
    assert_eq!(profile_state.mode, CatalogMode::OfficialPlusCustom);
    assert_eq!(
        profile_state.generated_path.as_deref(),
        Some(generated_path.as_str())
    );
    assert_eq!(
        profile_state.generated_hash.as_deref(),
        Some("hash-from-the-previous-app-version")
    );
    assert_eq!(profile_state.generation, 3);
    assert_eq!(
        fs::read(fixture.paths.codex_home.join(&generated_path)).unwrap(),
        effective_catalog,
        "the effective catalog the user is running on must survive the update"
    );
}

#[test]
fn a_bundled_update_that_retires_the_active_default_model_names_the_repair() {
    let (fixture, effective_catalog) = stranded_default_fixture("stranded");
    let persisted = fixture.read_settings();
    let before = fixture.file_generation();

    let error =
        commit_provider_detail_from_paths(&fixture.paths, stranded_default_request(&persisted, 8))
            .unwrap_err();

    // The active profile cannot commit against a catalog that lost its default, but the failure
    // names the repair — pick a replacement default — instead of the generic not-ready family.
    assert_eq!(error.code(), ProviderCommitErrorCode::CatalogUnavailable);
    assert_eq!(
        error.reason(),
        "active provider default model is absent from the bundled baseline"
    );
    assert_eq!(fixture.file_generation(), before, "nothing on disk moved");
    assert_eq!(
        fs::read(
            fixture
                .paths
                .codex_home
                .join(crate::model_catalog::generated_relative_path("stranded"))
        )
        .unwrap(),
        effective_catalog,
        "the catalog Codex is running on stays in place for continuity"
    );
}
