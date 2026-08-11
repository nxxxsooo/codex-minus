use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, ensure};
use codex_plus_core::settings::{
    AggregateRelayProfile, BackendSettings, RelayContextSelection, RelayMode, RelayModelInsertMode,
    RelayProfile, RelayProtocol,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::model_catalog::{self, CatalogMode, CatalogOverlay, CatalogState, UpstreamTopology};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderCommitAction {
    Save,
    SetCurrent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRelayProfileDraft {
    pub id: String,
    pub name: String,
    pub model: String,
    pub base_url: String,
    pub upstream_base_url: String,
    pub api_key: String,
    pub protocol: RelayProtocol,
    pub relay_mode: RelayMode,
    pub official_mix_api_key: bool,
    pub test_model: String,
    pub config_contents: String,
    pub auth_contents: String,
    pub use_common_config: bool,
    pub context_selection: RelayContextSelection,
    pub context_selection_initialized: bool,
    pub context_window: String,
    pub auto_compact_limit: String,
    pub model_insert_mode: RelayModelInsertMode,
    pub model_list: String,
    pub model_windows: String,
    pub user_agent: String,
}

impl From<&RelayProfile> for ProviderRelayProfileDraft {
    fn from(profile: &RelayProfile) -> Self {
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            model: profile.model.clone(),
            base_url: profile.base_url.clone(),
            upstream_base_url: profile.upstream_base_url.clone(),
            api_key: profile.api_key.clone(),
            protocol: profile.protocol,
            relay_mode: profile.relay_mode,
            official_mix_api_key: profile.official_mix_api_key,
            test_model: profile.test_model.clone(),
            config_contents: profile.config_contents.clone(),
            auth_contents: profile.auth_contents.clone(),
            use_common_config: profile.use_common_config,
            context_selection: profile.context_selection.clone(),
            context_selection_initialized: profile.context_selection_initialized,
            context_window: profile.context_window.clone(),
            auto_compact_limit: profile.auto_compact_limit.clone(),
            model_insert_mode: profile.model_insert_mode,
            model_list: profile.model_list.clone(),
            model_windows: profile.model_windows.clone(),
            user_agent: profile.user_agent.clone(),
        }
    }
}

impl From<&ProviderRelayProfileDraft> for RelayProfile {
    fn from(profile: &ProviderRelayProfileDraft) -> Self {
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            model: profile.model.clone(),
            base_url: profile.base_url.clone(),
            upstream_base_url: profile.upstream_base_url.clone(),
            api_key: profile.api_key.clone(),
            protocol: profile.protocol,
            relay_mode: profile.relay_mode,
            official_mix_api_key: profile.official_mix_api_key,
            test_model: profile.test_model.clone(),
            config_contents: profile.config_contents.clone(),
            auth_contents: profile.auth_contents.clone(),
            use_common_config: profile.use_common_config,
            context_selection: profile.context_selection.clone(),
            context_selection_initialized: profile.context_selection_initialized,
            context_window: profile.context_window.clone(),
            auto_compact_limit: profile.auto_compact_limit.clone(),
            model_insert_mode: profile.model_insert_mode,
            model_list: profile.model_list.clone(),
            model_windows: profile.model_windows.clone(),
            user_agent: profile.user_agent.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOwnedTopologyDraft {
    pub relay_profiles_enabled: bool,
    pub relay_profiles: Vec<ProviderRelayProfileDraft>,
    pub aggregate_relay_profiles: Vec<AggregateRelayProfile>,
    pub active_relay_id: String,
    pub active_aggregate_relay_id: String,
    pub relay_base_url: String,
    pub relay_api_key: String,
    pub relay_common_config_contents: String,
    pub relay_context_config_contents: String,
    pub relay_test_model: String,
}

impl ProviderOwnedTopologyDraft {
    pub fn from_settings(settings: &BackendSettings) -> Self {
        Self {
            relay_profiles_enabled: settings.relay_profiles_enabled,
            relay_profiles: settings
                .relay_profiles
                .iter()
                .map(ProviderRelayProfileDraft::from)
                .collect(),
            aggregate_relay_profiles: settings.aggregate_relay_profiles.clone(),
            active_relay_id: settings.active_relay_id.clone(),
            active_aggregate_relay_id: settings.active_aggregate_relay_id.clone(),
            relay_base_url: settings.relay_base_url.clone(),
            relay_api_key: settings.relay_api_key.clone(),
            relay_common_config_contents: settings.relay_common_config_contents.clone(),
            relay_context_config_contents: settings.relay_context_config_contents.clone(),
            relay_test_model: settings.relay_test_model.clone(),
        }
    }

    fn apply_to(&self, persisted: &BackendSettings) -> BackendSettings {
        let mut next = persisted.clone();
        next.relay_profiles_enabled = self.relay_profiles_enabled;
        next.relay_profiles = self.relay_profiles.iter().map(RelayProfile::from).collect();
        next.aggregate_relay_profiles = self.aggregate_relay_profiles.clone();
        next.active_relay_id = self.active_relay_id.clone();
        next.active_aggregate_relay_id = self.active_aggregate_relay_id.clone();
        next.relay_base_url = self.relay_base_url.clone();
        next.relay_api_key = self.relay_api_key.clone();
        next.relay_common_config_contents = self.relay_common_config_contents.clone();
        next.relay_context_config_contents = self.relay_context_config_contents.clone();
        next.relay_test_model = self.relay_test_model.clone();
        next
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCatalogDraft {
    pub profile_id: String,
    pub mode: CatalogMode,
    #[serde(default)]
    pub mode_explicit: bool,
    #[serde(default)]
    pub upstream_topology: UpstreamTopology,
    #[serde(default)]
    pub external_pointer: Option<String>,
    #[serde(default)]
    pub overlay: CatalogOverlay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCommitRequest {
    pub topology: ProviderOwnedTopologyDraft,
    #[serde(default)]
    pub catalog_drafts: Vec<ProfileCatalogDraft>,
    #[serde(default)]
    pub focused_profile_id: Option<String>,
    pub action: ProviderCommitAction,
    pub previous_active_relay_id: String,
    #[serde(default)]
    pub confirm_context_cleanup: bool,
    pub draft_revision: u64,
    pub expected_provider_fingerprint: String,
}

#[derive(Debug, Clone)]
pub struct ProviderCommitPlan {
    pub settings: BackendSettings,
    pub catalog_state: CatalogState,
    pub generated_catalogs: BTreeMap<String, Value>,
    pub active_catalog: Option<Value>,
    pub draft_revision: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderFingerprintStatus {
    pub provider_fingerprint: String,
}

pub fn provider_owned_fingerprint(topology: &ProviderOwnedTopologyDraft) -> anyhow::Result<String> {
    let canonical = serde_json::to_vec(topology)?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

pub fn provider_fingerprint_status(
    settings: &BackendSettings,
) -> anyhow::Result<ProviderFingerprintStatus> {
    Ok(ProviderFingerprintStatus {
        provider_fingerprint: provider_owned_fingerprint(
            &ProviderOwnedTopologyDraft::from_settings(settings),
        )?,
    })
}

pub fn validate_provider_detail_request(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
) -> anyhow::Result<()> {
    validate_common_request(persisted_settings, persisted_state, request)?;
    let focused_id = request
        .focused_profile_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .context("focused provider profile is required")?;
    ensure!(
        request
            .topology
            .relay_profiles
            .iter()
            .any(|profile| profile.id == focused_id),
        "focused provider profile is missing from the topology draft"
    );
    if request.action == ProviderCommitAction::SetCurrent {
        ensure!(
            request.topology.active_relay_id == focused_id,
            "setCurrent must select the focused provider profile"
        );
    }
    let is_new = !persisted_settings
        .relay_profiles
        .iter()
        .any(|profile| profile.id == focused_id);
    let supplied = request
        .catalog_drafts
        .iter()
        .filter(|draft| draft.profile_id == focused_id)
        .count();
    ensure!(
        supplied == 1
            || (supplied == 0 && is_new && implicit_mixed_catalog_eligible(request, focused_id)),
        "focused provider profile requires one complete catalog draft"
    );
    Ok(())
}

fn validate_common_request(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
) -> anyhow::Result<()> {
    ensure!(
        request.draft_revision > 0,
        "draft revision must be a positive correlation value"
    );
    let expected = provider_owned_fingerprint(&ProviderOwnedTopologyDraft::from_settings(
        persisted_settings,
    ))?;
    ensure!(
        request.expected_provider_fingerprint == expected,
        "provider state changed; reload or merge before saving"
    );
    ensure!(
        request.previous_active_relay_id == persisted_settings.active_relay_id,
        "previous active provider does not match the compare-and-swap snapshot"
    );

    let mut profile_ids = BTreeSet::new();
    for profile in &request.topology.relay_profiles {
        ensure!(
            !profile.id.trim().is_empty(),
            "provider profile id is empty"
        );
        ensure!(
            profile_ids.insert(profile.id.clone()),
            "duplicate provider profile id"
        );
        ensure!(
            profile.auth_contents.trim().is_empty(),
            "incoming authContents is prohibited"
        );
    }
    let mut aggregate_ids = BTreeSet::new();
    for aggregate in &request.topology.aggregate_relay_profiles {
        ensure!(
            aggregate_ids.insert(aggregate.id.clone()),
            "duplicate aggregate profile id"
        );
    }
    ensure!(
        request.topology.active_aggregate_relay_id.trim().is_empty()
            || aggregate_ids.contains(&request.topology.active_aggregate_relay_id),
        "active aggregate profile is missing from the topology draft"
    );
    ensure!(
        request.topology.active_relay_id.trim().is_empty()
            || profile_ids.contains(&request.topology.active_relay_id),
        "active provider profile is missing from the topology draft"
    );

    let profiles = request
        .topology
        .relay_profiles
        .iter()
        .map(|profile| (profile.id.as_str(), RelayProfile::from(profile)))
        .collect::<BTreeMap<_, _>>();
    let mut catalog_ids = BTreeSet::new();
    for draft in &request.catalog_drafts {
        ensure!(
            catalog_ids.insert(draft.profile_id.clone()),
            "duplicate catalog draft for provider profile"
        );
        let profile = profiles
            .get(draft.profile_id.as_str())
            .context("catalog draft references a missing provider profile")?;
        validate_catalog_draft(profile, draft)?;
        if persisted_state
            .profiles
            .get(&draft.profile_id)
            .is_some_and(|state| state.mode == CatalogMode::External)
        {
            ensure!(
                draft.mode == CatalogMode::External,
                "external catalog ownership requires the reviewed adoption command"
            );
        }
    }
    Ok(())
}

fn validate_catalog_draft(
    profile: &RelayProfile,
    draft: &ProfileCatalogDraft,
) -> anyhow::Result<()> {
    model_catalog::validate_overlay(&draft.overlay)?;
    model_catalog::validate_upstream_topology(profile, draft.upstream_topology)?;
    match draft.mode {
        CatalogMode::External => ensure!(
            draft
                .external_pointer
                .as_deref()
                .is_some_and(|pointer| !pointer.trim().is_empty()),
            "external catalog draft requires a pointer"
        ),
        _ => ensure!(
            draft.external_pointer.is_none(),
            "managed or native catalog draft cannot carry an external pointer"
        ),
    }
    Ok(())
}

pub fn plan_provider_detail_commit(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
) -> anyhow::Result<ProviderCommitPlan> {
    validate_provider_detail_request(persisted_settings, persisted_state, request)?;
    let focused_id = request.focused_profile_id.as_deref().unwrap();
    let mut drafts = request.catalog_drafts.clone();
    if !drafts.iter().any(|draft| draft.profile_id == focused_id) {
        drafts.push(implicit_mixed_catalog_draft(focused_id));
    }
    plan_validated_request(persisted_settings, persisted_state, request, drafts)
}

pub fn plan_provider_topology_commit(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
) -> anyhow::Result<ProviderCommitPlan> {
    ensure!(
        request.focused_profile_id.is_none(),
        "topology request cannot select a focused provider profile"
    );
    ensure!(
        request.action == ProviderCommitAction::Save,
        "topology request supports save only"
    );
    validate_common_request(persisted_settings, persisted_state, request)?;
    plan_validated_request(
        persisted_settings,
        persisted_state,
        request,
        request.catalog_drafts.clone(),
    )
}

fn plan_validated_request(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
    drafts: Vec<ProfileCatalogDraft>,
) -> anyhow::Result<ProviderCommitPlan> {
    let settings = request.topology.apply_to(persisted_settings);
    let mut catalog_state = persisted_state.clone();
    let catalog_profile_count_before = catalog_state.profiles.len();
    let profile_ids = settings
        .relay_profiles
        .iter()
        .map(|profile| profile.id.clone())
        .collect::<BTreeSet<_>>();
    catalog_state
        .profiles
        .retain(|profile_id, _| profile_ids.contains(profile_id));
    let catalog_topology_changed = catalog_state.profiles.len() != catalog_profile_count_before;

    for draft in &drafts {
        let state = catalog_state
            .profiles
            .entry(draft.profile_id.clone())
            .or_default();
        state.mode = draft.mode;
        state.mode_explicit = draft.mode_explicit;
        state.upstream_topology = draft.upstream_topology;
        state.overlay = draft.overlay.clone();
        state.external_pointer = draft.external_pointer.clone();
        state.action_required = None;
    }
    if catalog_topology_changed || !drafts.is_empty() {
        catalog_state.operation_generation = catalog_state.operation_generation.saturating_add(1);
    }

    let active_focus = request
        .focused_profile_id
        .as_deref()
        .is_some_and(|focused| {
            request.action == ProviderCommitAction::SetCurrent
                || request.previous_active_relay_id == focused
        });
    let mut generated_catalogs = BTreeMap::new();
    let mut active_catalog = None;
    for draft in &drafts {
        if !matches!(
            draft.mode,
            CatalogMode::OfficialPlusCustom | CatalogMode::CustomOnly
        ) {
            continue;
        }
        let profile = settings
            .relay_profiles
            .iter()
            .find(|profile| profile.id == draft.profile_id)
            .context("catalog planning profile is missing")?;
        let profile_state = catalog_state
            .profiles
            .get(&draft.profile_id)
            .cloned()
            .context("catalog planning state is missing")?;
        match model_catalog::compose_profile_catalog(&catalog_state, profile, &profile_state) {
            Ok(catalog) => {
                let bytes = serde_json::to_vec_pretty(&catalog)?;
                let hash = format!("{:x}", Sha256::digest(&bytes));
                let profile_state = catalog_state.profiles.get_mut(&draft.profile_id).unwrap();
                if profile_state.generated_hash.as_deref() != Some(hash.as_str()) {
                    profile_state.generation = profile_state.generation.saturating_add(1);
                }
                profile_state.generated_hash = Some(hash);
                profile_state.generated_path =
                    Some(model_catalog::generated_relative_path(&draft.profile_id));
                profile_state.action_required = None;
                if active_focus
                    && request.focused_profile_id.as_deref() == Some(draft.profile_id.as_str())
                {
                    active_catalog = Some(catalog.clone());
                }
                generated_catalogs.insert(draft.profile_id.clone(), catalog);
            }
            Err(error) if !active_focus => {
                catalog_state
                    .profiles
                    .get_mut(&draft.profile_id)
                    .unwrap()
                    .action_required = Some(error.to_string());
            }
            Err(error) => return Err(error),
        }
    }

    Ok(ProviderCommitPlan {
        settings,
        catalog_state,
        generated_catalogs,
        active_catalog,
        draft_revision: request.draft_revision,
    })
}

fn implicit_mixed_catalog_eligible(request: &ProviderCommitRequest, profile_id: &str) -> bool {
    request
        .topology
        .relay_profiles
        .iter()
        .find(|profile| profile.id == profile_id)
        .is_some_and(|profile| {
            profile.relay_mode == RelayMode::Official
                && profile.official_mix_api_key
                && profile.protocol == RelayProtocol::Responses
                && !has_catalog_pointer(&profile.config_contents)
        })
}

fn implicit_mixed_catalog_draft(profile_id: &str) -> ProfileCatalogDraft {
    ProfileCatalogDraft {
        profile_id: profile_id.to_string(),
        mode: CatalogMode::OfficialPlusCustom,
        mode_explicit: false,
        upstream_topology: UpstreamTopology::Direct,
        external_pointer: None,
        overlay: CatalogOverlay::default(),
    }
}

fn has_catalog_pointer(config_contents: &str) -> bool {
    config_contents
        .parse::<toml_edit::DocumentMut>()
        .ok()
        .and_then(|document| {
            document
                .as_table()
                .get("model_catalog_json")
                .and_then(toml_edit::Item::as_str)
                .map(str::to_string)
        })
        .is_some_and(|pointer| !pointer.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use codex_plus_core::settings::{BackendSettings, RelayMode, RelayProfile, RelayProtocol};
    use serde_json::{Value, json};

    use super::*;
    use crate::model_catalog::{
        CatalogMode, CatalogOverlay, CatalogState, CustomModel, OfficialSnapshot,
        ProfileCatalogState, UpstreamTopology,
    };

    fn mixed_profile(id: &str, model: &str) -> RelayProfile {
        RelayProfile {
            id: id.to_string(),
            name: format!("Provider {id}"),
            model: model.to_string(),
            base_url: format!("https://{id}.example/v1"),
            upstream_base_url: format!("https://{id}.example/v1"),
            api_key: format!("secret-{id}"),
            protocol: RelayProtocol::Responses,
            relay_mode: RelayMode::Official,
            official_mix_api_key: true,
            test_model: format!("{model}-test"),
            config_contents: format!("model = \"{model}\"\n"),
            auth_contents: String::new(),
            model_list: model.to_string(),
            model_windows: "{}".to_string(),
            user_agent: format!("agent-{id}"),
            ..RelayProfile::default()
        }
    }

    fn settings_with(profiles: Vec<RelayProfile>, active: &str) -> BackendSettings {
        BackendSettings {
            relay_profiles_enabled: true,
            relay_base_url: profiles
                .iter()
                .find(|profile| profile.id == active)
                .map(|profile| profile.base_url.clone())
                .unwrap_or_default(),
            relay_api_key: profiles
                .iter()
                .find(|profile| profile.id == active)
                .map(|profile| profile.api_key.clone())
                .unwrap_or_default(),
            relay_profiles: profiles,
            active_relay_id: active.to_string(),
            active_aggregate_relay_id: String::new(),
            relay_common_config_contents: "# common\n".to_string(),
            relay_context_config_contents: "# context\n".to_string(),
            relay_test_model: "global-test".to_string(),
            ..BackendSettings::default()
        }
    }

    fn official_catalog() -> Value {
        json!({
            "models": [{
                "slug": "official-a",
                "display_name": "Official A",
                "visibility": "list",
                "context_window": 272000,
                "effective_context_window_percent": 100,
                "supported_reasoning_levels": [],
                "supported_tools": [],
                "service_tiers": []
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

    fn catalog_draft(
        profile_id: &str,
        mode: CatalogMode,
        overlay: CatalogOverlay,
    ) -> ProfileCatalogDraft {
        ProfileCatalogDraft {
            profile_id: profile_id.to_string(),
            mode,
            mode_explicit: true,
            upstream_topology: UpstreamTopology::Direct,
            external_pointer: None,
            overlay,
        }
    }

    fn request_for(
        persisted: &BackendSettings,
        next: &BackendSettings,
        focused_profile_id: Option<&str>,
        catalog_drafts: Vec<ProfileCatalogDraft>,
        action: ProviderCommitAction,
    ) -> ProviderCommitRequest {
        ProviderCommitRequest {
            topology: ProviderOwnedTopologyDraft::from_settings(next),
            catalog_drafts,
            focused_profile_id: focused_profile_id.map(str::to_string),
            action,
            previous_active_relay_id: persisted.active_relay_id.clone(),
            confirm_context_cleanup: false,
            draft_revision: 11,
            expected_provider_fingerprint: provider_owned_fingerprint(
                &ProviderOwnedTopologyDraft::from_settings(persisted),
            )
            .unwrap(),
        }
    }

    #[test]
    fn camel_case_contract_deserializes_one_complete_envelope() {
        let request: ProviderCommitRequest = serde_json::from_value(json!({
            "topology": {
                "relayProfilesEnabled": true,
                "relayProfiles": [{
                    "id": "relay-new",
                    "name": "New",
                    "model": "official-a",
                    "baseUrl": "https://relay.example/v1",
                    "upstreamBaseUrl": "https://relay.example/v1",
                    "apiKey": "secret-provider-key",
                    "protocol": "responses",
                    "relayMode": "official",
                    "officialMixApiKey": true,
                    "testModel": "official-a-mini",
                    "configContents": "model = \"official-a\"\n",
                    "authContents": "",
                    "useCommonConfig": true,
                    "contextSelection": { "mcpServers": ["memory"], "skills": [], "plugins": [] },
                    "contextSelectionInitialized": true,
                    "contextWindow": "272000",
                    "autoCompactLimit": "240000",
                    "modelInsertMode": "patch",
                    "modelList": "official-a",
                    "modelWindows": "{}",
                    "userAgent": "contract-test"
                }],
                "aggregateRelayProfiles": [],
                "activeRelayId": "relay-new",
                "activeAggregateRelayId": "",
                "relayBaseUrl": "https://relay.example/v1",
                "relayApiKey": "secret-provider-key",
                "relayCommonConfigContents": "# common\n",
                "relayContextConfigContents": "# context\n",
                "relayTestModel": "global-test"
            },
            "catalogDrafts": [{
                "profileId": "relay-new",
                "mode": "official-plus-custom",
                "modeExplicit": false,
                "upstreamTopology": "direct",
                "externalPointer": null,
                "overlay": { "official": {}, "custom": [] }
            }],
            "focusedProfileId": "relay-new",
            "action": "setCurrent",
            "previousActiveRelayId": "relay-old",
            "confirmContextCleanup": true,
            "draftRevision": 42,
            "expectedProviderFingerprint": "sha256:before"
        }))
        .unwrap();

        assert_eq!(request.focused_profile_id.as_deref(), Some("relay-new"));
        assert_eq!(request.action, ProviderCommitAction::SetCurrent);
        assert_eq!(request.previous_active_relay_id, "relay-old");
        assert!(request.confirm_context_cleanup);
        assert_eq!(request.draft_revision, 42);
        assert_eq!(
            request.catalog_drafts[0].mode,
            CatalogMode::OfficialPlusCustom
        );
        assert_eq!(request.topology.relay_profiles[0].model, "official-a");
        assert_eq!(
            request.topology.relay_profiles[0].api_key,
            "secret-provider-key"
        );
    }

    #[test]
    fn fingerprint_hashes_structured_and_secret_fields_without_returning_them() {
        let base = settings_with(vec![mixed_profile("relay-a", "official-a")], "relay-a");
        let base_projection = ProviderOwnedTopologyDraft::from_settings(&base);
        let base_hash = provider_owned_fingerprint(&base_projection).unwrap();
        assert!(base_hash.starts_with("sha256:"));
        assert!(!base_hash.contains("secret-relay-a"));
        assert!(!base_hash.contains("official-a"));

        let mutations: [fn(&mut BackendSettings); 3] = [
            |settings: &mut BackendSettings| {
                settings.relay_profiles[0].model = "changed-model".to_string()
            },
            |settings: &mut BackendSettings| {
                settings.relay_profiles[0].base_url = "https://changed.example/v1".to_string()
            },
            |settings: &mut BackendSettings| {
                settings.relay_profiles[0].api_key = "changed-secret".to_string()
            },
        ];
        for mutate in mutations {
            let mut changed = base.clone();
            mutate(&mut changed);
            assert_ne!(
                provider_owned_fingerprint(&ProviderOwnedTopologyDraft::from_settings(&changed))
                    .unwrap(),
                base_hash,
            );
        }

        let status = provider_fingerprint_status(&base).unwrap();
        let status_json = serde_json::to_string(&status).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&status_json).unwrap(),
            json!({ "providerFingerprint": base_hash })
        );
        assert!(!status_json.contains("secret-relay-a"));
        assert!(!status_json.contains("official-a"));
    }

    #[test]
    fn detail_validation_rejects_missing_ambiguous_invalid_stale_auth_and_external_transitions() {
        let persisted = settings_with(vec![mixed_profile("relay-a", "official-a")], "relay-a");
        let state = state_with_official();

        let missing_focus = request_for(
            &persisted,
            &persisted,
            None,
            vec![],
            ProviderCommitAction::Save,
        );
        assert!(validate_provider_detail_request(&persisted, &state, &missing_focus).is_err());

        let mut absent_focus = request_for(
            &persisted,
            &persisted,
            Some("missing"),
            vec![],
            ProviderCommitAction::Save,
        );
        absent_focus.focused_profile_id = Some("missing".to_string());
        assert!(validate_provider_detail_request(&persisted, &state, &absent_focus).is_err());

        let mut duplicate = persisted.clone();
        duplicate
            .relay_profiles
            .push(duplicate.relay_profiles[0].clone());
        let duplicate_request = request_for(
            &persisted,
            &duplicate,
            Some("relay-a"),
            vec![],
            ProviderCommitAction::Save,
        );
        assert!(validate_provider_detail_request(&persisted, &state, &duplicate_request).is_err());

        let invalid_overlay = CatalogOverlay {
            custom: vec![
                CustomModel {
                    slug: "same".to_string(),
                    display_name: "Same".to_string(),
                    ..CustomModel::default()
                },
                CustomModel {
                    slug: "same".to_string(),
                    display_name: "Same again".to_string(),
                    ..CustomModel::default()
                },
            ],
            ..CatalogOverlay::default()
        };
        let invalid = request_for(
            &persisted,
            &persisted,
            Some("relay-a"),
            vec![catalog_draft(
                "relay-a",
                CatalogMode::CustomOnly,
                invalid_overlay,
            )],
            ProviderCommitAction::Save,
        );
        assert!(validate_provider_detail_request(&persisted, &state, &invalid).is_err());

        let mut stale = request_for(
            &persisted,
            &persisted,
            Some("relay-a"),
            vec![],
            ProviderCommitAction::Save,
        );
        stale.expected_provider_fingerprint = "sha256:stale".to_string();
        stale.draft_revision = u64::MAX;
        assert!(validate_provider_detail_request(&persisted, &state, &stale).is_err());

        let mut with_auth = persisted.clone();
        with_auth.relay_profiles[0].auth_contents = "oauth-must-not-enter".to_string();
        let with_auth_request = request_for(
            &persisted,
            &with_auth,
            Some("relay-a"),
            vec![],
            ProviderCommitAction::Save,
        );
        assert!(validate_provider_detail_request(&persisted, &state, &with_auth_request).is_err());

        let mut external_state = state;
        external_state.profiles.insert(
            "relay-a".to_string(),
            ProfileCatalogState {
                mode: CatalogMode::External,
                external_pointer: Some("models/user-owned.json".to_string()),
                ..ProfileCatalogState::default()
            },
        );
        let transition = request_for(
            &persisted,
            &persisted,
            Some("relay-a"),
            vec![catalog_draft(
                "relay-a",
                CatalogMode::OfficialPlusCustom,
                CatalogOverlay::default(),
            )],
            ProviderCommitAction::Save,
        );
        assert!(
            validate_provider_detail_request(&persisted, &external_state, &transition).is_err()
        );
    }

    #[test]
    fn planner_creates_implicit_first_save_catalog_state() {
        let old = mixed_profile("old", "official-a");
        let persisted = settings_with(vec![old.clone()], "old");
        let new = mixed_profile("new", "official-a");
        let next = settings_with(vec![old, new], "old");
        let request = request_for(
            &persisted,
            &next,
            Some("new"),
            vec![],
            ProviderCommitAction::Save,
        );

        let plan =
            plan_provider_detail_commit(&persisted, &state_with_official(), &request).unwrap();
        assert!(
            plan.settings
                .relay_profiles
                .iter()
                .any(|profile| profile.id == "new")
        );
        let profile_state = &plan.catalog_state.profiles["new"];
        assert_eq!(profile_state.mode, CatalogMode::OfficialPlusCustom);
        assert!(!profile_state.mode_explicit);
        assert!(profile_state.action_required.is_none());
        assert!(plan.generated_catalogs.contains_key("new"));
        assert!(plan.active_catalog.is_none());
        assert_eq!(plan.draft_revision, 11);
    }

    #[test]
    fn inactive_environment_failure_persists_complete_action_required_state() {
        let old = mixed_profile("old", "official-a");
        let persisted = settings_with(vec![old.clone()], "old");
        let new = mixed_profile("new", "official-a");
        let next = settings_with(vec![old, new], "old");
        let draft = catalog_draft(
            "new",
            CatalogMode::OfficialPlusCustom,
            CatalogOverlay::default(),
        );
        let request = request_for(
            &persisted,
            &next,
            Some("new"),
            vec![draft],
            ProviderCommitAction::Save,
        );

        let plan =
            plan_provider_detail_commit(&persisted, &CatalogState::default(), &request).unwrap();
        let profile_state = &plan.catalog_state.profiles["new"];
        assert_eq!(profile_state.mode, CatalogMode::OfficialPlusCustom);
        assert!(
            profile_state
                .action_required
                .as_deref()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(plan.generated_catalogs.is_empty());
        assert!(plan.active_catalog.is_none());
    }

    #[test]
    fn active_plan_uses_request_draft_and_fails_closed_without_readiness() {
        let profile = mixed_profile("relay-a", "persisted-model");
        let persisted = settings_with(vec![profile.clone()], "relay-a");
        let mut requested_profile = profile;
        requested_profile.model = "draft-model".to_string();
        requested_profile.config_contents = "model = \"draft-model\"\n".to_string();
        let requested = settings_with(vec![requested_profile], "relay-a");
        let mut persisted_state = state_with_official();
        persisted_state.profiles.insert(
            "relay-a".to_string(),
            ProfileCatalogState {
                mode: CatalogMode::NativeOfficial,
                overlay: CatalogOverlay::default(),
                ..ProfileCatalogState::default()
            },
        );
        let overlay = CatalogOverlay {
            custom: vec![CustomModel {
                slug: "draft-model".to_string(),
                display_name: "Draft Model".to_string(),
                ..CustomModel::default()
            }],
            ..CatalogOverlay::default()
        };
        let request = request_for(
            &persisted,
            &requested,
            Some("relay-a"),
            vec![catalog_draft("relay-a", CatalogMode::CustomOnly, overlay)],
            ProviderCommitAction::Save,
        );

        let plan = plan_provider_detail_commit(&persisted, &persisted_state, &request).unwrap();
        assert_eq!(plan.settings.relay_profiles[0].model, "draft-model");
        assert_eq!(
            plan.catalog_state.profiles["relay-a"].mode,
            CatalogMode::CustomOnly
        );
        assert_eq!(
            plan.active_catalog.as_ref().unwrap()["models"][0]["slug"],
            "draft-model"
        );

        let unavailable_request = request_for(
            &persisted,
            &requested,
            Some("relay-a"),
            vec![catalog_draft(
                "relay-a",
                CatalogMode::OfficialPlusCustom,
                CatalogOverlay::default(),
            )],
            ProviderCommitAction::Save,
        );
        assert!(
            plan_provider_detail_commit(&persisted, &CatalogState::default(), &unavailable_request)
                .is_err()
        );
    }

    #[test]
    fn previous_active_id_cannot_downgrade_active_save_to_inactive_planning() {
        let persisted = settings_with(vec![mixed_profile("relay-a", "official-a")], "relay-a");
        let mut request = request_for(
            &persisted,
            &persisted,
            Some("relay-a"),
            vec![catalog_draft(
                "relay-a",
                CatalogMode::OfficialPlusCustom,
                CatalogOverlay::default(),
            )],
            ProviderCommitAction::Save,
        );
        request.previous_active_relay_id = "not-the-cas-active-profile".to_string();

        assert!(
            plan_provider_detail_commit(&persisted, &CatalogState::default(), &request).is_err()
        );
    }

    #[test]
    fn duplicate_catalog_drafts_are_rejected_as_ambiguous() {
        let settings = settings_with(vec![mixed_profile("relay-a", "official-a")], "relay-a");
        let draft = catalog_draft(
            "relay-a",
            CatalogMode::OfficialPlusCustom,
            CatalogOverlay::default(),
        );
        let request = request_for(
            &settings,
            &settings,
            Some("relay-a"),
            vec![draft.clone(), draft],
            ProviderCommitAction::Save,
        );
        assert!(
            validate_provider_detail_request(&settings, &state_with_official(), &request).is_err()
        );
    }

    #[test]
    fn topology_projection_keeps_ordered_lists_and_legacy_fields_only() {
        let settings = settings_with(
            vec![
                mixed_profile("relay-b", "official-a"),
                mixed_profile("relay-a", "official-a"),
            ],
            "relay-b",
        );
        let projection = ProviderOwnedTopologyDraft::from_settings(&settings);
        let value = serde_json::to_value(&projection).unwrap();
        assert_eq!(value["relayProfiles"][0]["id"], "relay-b");
        assert_eq!(value["relayProfiles"][1]["id"], "relay-a");
        assert_eq!(value["relayBaseUrl"], settings.relay_base_url);
        assert_eq!(value["relayApiKey"], settings.relay_api_key);
        assert!(value.get("enhancementsEnabled").is_none());
        assert!(value.get("codexAppPath").is_none());
    }

    #[test]
    fn planned_settings_preserve_unrelated_backend_values() {
        let persisted = BackendSettings {
            enhancements_enabled: false,
            codex_app_path: "/Applications/Keep.app".to_string(),
            ..settings_with(vec![mixed_profile("relay-a", "official-a")], "relay-a")
        };
        let mut next = persisted.clone();
        next.relay_profiles_enabled = false;
        let request = request_for(&persisted, &next, None, vec![], ProviderCommitAction::Save);
        let plan =
            plan_provider_topology_commit(&persisted, &state_with_official(), &request).unwrap();
        assert!(!plan.settings.enhancements_enabled);
        assert_eq!(plan.settings.codex_app_path, "/Applications/Keep.app");
        assert!(!plan.settings.relay_profiles_enabled);
    }

    #[test]
    fn topology_validation_rejects_zero_revision_and_missing_active_aggregate() {
        let persisted = settings_with(vec![mixed_profile("relay-a", "official-a")], "relay-a");

        let mut zero_revision = request_for(
            &persisted,
            &persisted,
            None,
            vec![],
            ProviderCommitAction::Save,
        );
        zero_revision.draft_revision = 0;
        assert!(
            plan_provider_topology_commit(&persisted, &state_with_official(), &zero_revision)
                .is_err()
        );

        let mut missing_aggregate = request_for(
            &persisted,
            &persisted,
            None,
            vec![],
            ProviderCommitAction::Save,
        );
        missing_aggregate.topology.active_aggregate_relay_id = "missing-aggregate".to_string();
        assert!(
            plan_provider_topology_commit(&persisted, &state_with_official(), &missing_aggregate)
                .is_err()
        );
    }
}
