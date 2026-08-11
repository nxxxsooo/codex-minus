use std::fs;

use codex_minus_lib::commands::{
    ProviderCommitCheckpoint, ProviderCommitErrorCode, ProviderCommitPaths, ProviderCommitPayload,
    assert_staged_native_provider_contract, commit_provider_detail_from_paths,
    commit_provider_detail_from_paths_observed,
};
use codex_minus_lib::provider_commit::{
    CatalogMode, CatalogOverlay, CatalogState, CustomModel, OfficialSnapshot, ProfileCatalogDraft,
    ProviderCommitAction, ProviderCommitRequest, ProviderOwnedTopologyDraft, UpstreamTopology,
    provider_owned_fingerprint,
};
use codex_plus_core::settings::{BackendSettings, RelayMode, RelayProfile, RelayProtocol};
use serde_json::{Value, json};
use std::collections::BTreeMap;
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

fn state_with_official() -> CatalogState {
    CatalogState {
        official: Some(OfficialSnapshot {
            raw_catalog: official_catalog(),
            ..OfficialSnapshot::default()
        }),
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
        fs::write(codex_home.join("auth.json"), b"official-auth-before").unwrap();
        Self {
            _temp: temp,
            paths: ProviderCommitPaths {
                app_state,
                codex_home,
                settings_path,
                catalog_state_path,
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
        "https://relay.example/v1",
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
    invalid_next.relay_profiles[0].auth_contents =
        r#"{"OPENAI_API_KEY":"provider-key-sentinel"}"#.to_string();
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
