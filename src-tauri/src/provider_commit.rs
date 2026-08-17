use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, ensure};
use codex_plus_core::settings::{
    AggregateRelayMember, AggregateRelayProfile, AggregateRelayStrategy, BackendSettings,
    RelayContextSelection, RelayMode, RelayModelInsertMode, RelayProfile, RelayProtocol,
};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::model_catalog;

pub use crate::model_catalog::{
    CatalogMode, CatalogOverlay, CatalogState, CustomModel, OfficialSnapshot, ProfileCatalogState,
    UpstreamTopology,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ProviderCommitAction {
    Save,
    SetCurrent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderContextSelectionDraft {
    pub mcp_servers: Vec<String>,
    pub skills: Vec<String>,
    pub plugins: Vec<String>,
}

impl From<&RelayContextSelection> for ProviderContextSelectionDraft {
    fn from(selection: &RelayContextSelection) -> Self {
        Self {
            mcp_servers: selection.mcp_servers.clone(),
            skills: selection.skills.clone(),
            plugins: selection.plugins.clone(),
        }
    }
}

impl From<&ProviderContextSelectionDraft> for RelayContextSelection {
    fn from(selection: &ProviderContextSelectionDraft) -> Self {
        Self {
            mcp_servers: selection.mcp_servers.clone(),
            skills: selection.skills.clone(),
            plugins: selection.plugins.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderAggregateMemberDraft {
    pub relay_id: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderAggregateDraft {
    pub id: String,
    pub name: String,
    pub strategy: AggregateRelayStrategy,
    pub members: Vec<ProviderAggregateMemberDraft>,
}

impl From<&AggregateRelayProfile> for ProviderAggregateDraft {
    fn from(aggregate: &AggregateRelayProfile) -> Self {
        Self {
            id: aggregate.id.clone(),
            name: aggregate.name.clone(),
            strategy: aggregate.strategy,
            members: aggregate
                .members
                .iter()
                .map(|member| ProviderAggregateMemberDraft {
                    relay_id: member.relay_id.clone(),
                    weight: member.weight,
                })
                .collect(),
        }
    }
}

impl From<&ProviderAggregateDraft> for AggregateRelayProfile {
    fn from(aggregate: &ProviderAggregateDraft) -> Self {
        Self {
            id: aggregate.id.clone(),
            name: aggregate.name.clone(),
            strategy: aggregate.strategy,
            members: aggregate
                .members
                .iter()
                .map(|member| AggregateRelayMember {
                    relay_id: member.relay_id.clone(),
                    weight: member.weight,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
    pub context_selection: ProviderContextSelectionDraft,
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
            context_selection: ProviderContextSelectionDraft::from(&profile.context_selection),
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
            context_selection: RelayContextSelection::from(&profile.context_selection),
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderOwnedTopologyDraft {
    pub relay_profiles_enabled: bool,
    pub relay_profiles: Vec<ProviderRelayProfileDraft>,
    pub aggregate_relay_profiles: Vec<ProviderAggregateDraft>,
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
            aggregate_relay_profiles: settings
                .aggregate_relay_profiles
                .iter()
                .map(ProviderAggregateDraft::from)
                .collect(),
            active_relay_id: settings.active_relay_id.clone(),
            active_aggregate_relay_id: settings.active_aggregate_relay_id.clone(),
            relay_base_url: settings.relay_base_url.clone(),
            relay_api_key: settings.relay_api_key.clone(),
            relay_common_config_contents: settings.relay_common_config_contents.clone(),
            relay_context_config_contents: settings.relay_context_config_contents.clone(),
            relay_test_model: settings.relay_test_model.clone(),
        }
    }

    pub(crate) fn apply_to(&self, persisted: &BackendSettings) -> BackendSettings {
        let mut next = persisted.clone();
        next.relay_profiles_enabled = self.relay_profiles_enabled;
        next.relay_profiles = self.relay_profiles.iter().map(RelayProfile::from).collect();
        next.aggregate_relay_profiles = self
            .aggregate_relay_profiles
            .iter()
            .map(AggregateRelayProfile::from)
            .collect();
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileCatalogDraft {
    pub profile_id: String,
    pub mode: CatalogMode,
    pub mode_explicit: bool,
    pub upstream_topology: UpstreamTopology,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub external_pointer: Option<String>,
    #[serde(deserialize_with = "deserialize_catalog_overlay_strict")]
    pub overlay: CatalogOverlay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCommitRequest {
    pub topology: ProviderOwnedTopologyDraft,
    pub catalog_drafts: Vec<ProfileCatalogDraft>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub focused_profile_id: Option<String>,
    pub action: ProviderCommitAction,
    pub previous_active_relay_id: String,
    pub confirm_context_cleanup: bool,
    pub draft_revision: u64,
    pub expected_provider_fingerprint: String,
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn deserialize_catalog_overlay_strict<'de, D>(deserializer: D) -> Result<CatalogOverlay, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    validate_catalog_overlay_shape(&value).map_err(D::Error::custom)?;
    serde_json::from_value(value).map_err(D::Error::custom)
}

fn validate_catalog_overlay_shape(value: &Value) -> Result<(), String> {
    const OVERLAY_FIELDS: &[&str] = &["official", "custom"];
    const OFFICIAL_FIELDS: &[&str] = &[
        "displayName",
        "visible",
        "contextWindow",
        "effectiveContextWindowPercent",
        "order",
        "supportedReasoningLevels",
        "defaultReasoningLevel",
        "supportedTools",
        "toolCapabilities",
    ];
    const CUSTOM_FIELDS: &[&str] = &[
        "slug",
        "displayName",
        "contextWindow",
        "effectiveContextWindowPercent",
        "visible",
        "order",
        "supportedReasoningLevels",
        "defaultReasoningLevel",
        "supportedTools",
        "toolCapabilities",
        "templateProvenance",
    ];
    const REASONING_FIELDS: &[&str] = &["effort", "description"];

    let overlay = exact_object(value, "catalog overlay", OVERLAY_FIELDS)?;
    let official = overlay["official"]
        .as_object()
        .ok_or_else(|| "catalog overlay official must be an object".to_string())?;
    for (slug, override_value) in official {
        let item = exact_object(
            override_value,
            &format!("official override {slug}"),
            OFFICIAL_FIELDS,
        )?;
        validate_optional_reasoning_levels(
            &item["supportedReasoningLevels"],
            "official supportedReasoningLevels",
            REASONING_FIELDS,
        )?;
        validate_nullable_object(&item["toolCapabilities"], "official toolCapabilities")?;
    }
    let custom = overlay["custom"]
        .as_array()
        .ok_or_else(|| "catalog overlay custom must be an array".to_string())?;
    for (index, custom_value) in custom.iter().enumerate() {
        let item = exact_object(
            custom_value,
            &format!("custom model {index}"),
            CUSTOM_FIELDS,
        )?;
        let reasoning = item["supportedReasoningLevels"]
            .as_array()
            .ok_or_else(|| "custom supportedReasoningLevels must be an array".to_string())?;
        for level in reasoning {
            exact_object(level, "custom reasoning level", REASONING_FIELDS)?;
        }
        validate_nullable_object(&item["toolCapabilities"], "custom toolCapabilities")?;
    }
    Ok(())
}

fn exact_object<'a>(
    value: &'a Value,
    label: &str,
    fields: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))?;
    for field in fields {
        if !object.contains_key(*field) {
            return Err(format!("{label} is missing required field {field}"));
        }
    }
    if let Some(field) = object
        .keys()
        .find(|field| !fields.contains(&field.as_str()))
    {
        return Err(format!("{label} contains unknown field {field}"));
    }
    Ok(object)
}

fn validate_optional_reasoning_levels(
    value: &Value,
    label: &str,
    fields: &[&str],
) -> Result<(), String> {
    if value.is_null() {
        return Ok(());
    }
    let levels = value
        .as_array()
        .ok_or_else(|| format!("{label} must be an array or null"))?;
    for level in levels {
        exact_object(level, label, fields)?;
    }
    Ok(())
}

fn validate_nullable_object(value: &Value, label: &str) -> Result<(), String> {
    if value.is_null() || value.is_object() {
        Ok(())
    } else {
        Err(format!("{label} must be an object or null"))
    }
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderGenerationFingerprintMaterial<'a> {
    version: &'static str,
    topology: &'a ProviderOwnedTopologyDraft,
    catalogs: BTreeMap<String, EditableCatalogFingerprintMaterial>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EditableCatalogFingerprintMaterial {
    mode: CatalogMode,
    mode_explicit: bool,
    upstream_topology: UpstreamTopology,
    overlay: CatalogOverlay,
    external_pointer: Option<String>,
}

pub fn provider_generation_fingerprint(
    topology: &ProviderOwnedTopologyDraft,
    state: &CatalogState,
) -> anyhow::Result<String> {
    let catalogs = topology
        .relay_profiles
        .iter()
        .map(|profile| {
            let stored = state.profiles.get(&profile.id).cloned().unwrap_or_else(|| {
                let relay_profile = RelayProfile::from(profile);
                ProfileCatalogState {
                    mode: model_catalog::default_catalog_mode_for_profile(&relay_profile),
                    ..ProfileCatalogState::default()
                }
            });
            (
                profile.id.clone(),
                EditableCatalogFingerprintMaterial {
                    mode: stored.mode,
                    mode_explicit: stored.mode_explicit,
                    upstream_topology: stored.upstream_topology,
                    overlay: stored.overlay,
                    external_pointer: stored.external_pointer,
                },
            )
        })
        .collect();
    let material = ProviderGenerationFingerprintMaterial {
        version: "provider-generation-v1",
        topology,
        catalogs,
    };
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&material)?)
    ))
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
    let focused_profile = request
        .topology
        .relay_profiles
        .iter()
        .find(|profile| profile.id == focused_id)
        .context("focused provider profile is missing from the topology draft")?;
    if request.action == ProviderCommitAction::SetCurrent {
        ensure!(
            request.topology.active_relay_id == focused_id,
            "setCurrent must select the focused provider profile"
        );
    } else {
        ensure!(
            request.topology.active_relay_id == persisted_settings.active_relay_id,
            "save cannot change the active provider profile"
        );
    }
    let supplied = request
        .catalog_drafts
        .iter()
        .filter(|draft| draft.profile_id == focused_id)
        .count();
    let expected = usize::from(model_catalog::managed_catalog_capable(&RelayProfile::from(
        focused_profile,
    )));
    ensure!(
        supplied == expected,
        "focused provider profile must carry exactly the catalog drafts its capability supports"
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
    if request.action == ProviderCommitAction::Save {
        ensure!(
            request.topology.active_relay_id == persisted_settings.active_relay_id,
            "save cannot change the active provider profile"
        );
    }

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
            profile.auth_contents.is_empty(),
            "incoming authContents is prohibited"
        );
    }
    let aggregate_profile_ids = request
        .topology
        .relay_profiles
        .iter()
        .filter(|profile| profile.relay_mode == RelayMode::Aggregate)
        .map(|profile| profile.id.clone())
        .collect::<BTreeSet<_>>();
    let mut aggregate_ids = BTreeSet::new();
    for aggregate in &request.topology.aggregate_relay_profiles {
        ensure!(
            !aggregate.id.trim().is_empty(),
            "aggregate profile id is empty"
        );
        ensure!(
            aggregate_ids.insert(aggregate.id.clone()),
            "duplicate aggregate profile id"
        );
        ensure!(
            aggregate_profile_ids.contains(&aggregate.id),
            "aggregate profile metadata has no matching relay profile"
        );
        ensure!(
            !aggregate.members.is_empty(),
            "aggregate profile members are empty"
        );
        let mut member_ids = BTreeSet::new();
        for member in &aggregate.members {
            ensure!(
                member.weight > 0,
                "aggregate member weight must be positive"
            );
            ensure!(
                member_ids.insert(member.relay_id.clone()),
                "duplicate aggregate member"
            );
            let member_profile = request
                .topology
                .relay_profiles
                .iter()
                .find(|profile| profile.id == member.relay_id)
                .context("aggregate member references a missing provider profile")?;
            ensure!(
                member_profile.relay_mode != RelayMode::Aggregate,
                "aggregate member must reference an ordinary provider profile"
            );
        }
    }
    ensure!(
        aggregate_ids == aggregate_profile_ids,
        "aggregate relay profiles and aggregate metadata must be one-to-one"
    );
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
    let active_is_aggregate = aggregate_profile_ids.contains(&request.topology.active_relay_id);
    ensure!(
        (active_is_aggregate
            && request.topology.active_aggregate_relay_id == request.topology.active_relay_id)
            || (!active_is_aggregate && request.topology.active_aggregate_relay_id.is_empty()),
        "active provider and active aggregate ids are inconsistent"
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
            let prior = &persisted_state.profiles[&draft.profile_id];
            ensure!(
                draft.mode_explicit == prior.mode_explicit
                    && draft.upstream_topology == prior.upstream_topology
                    && draft.external_pointer == prior.external_pointer
                    && draft.overlay == prior.overlay,
                "ordinary save must preserve external catalog ownership identity"
            );
        }
    }
    for (profile_id, prior) in &persisted_state.profiles {
        if prior.mode != CatalogMode::External {
            continue;
        }
        let Some(profile) = profiles.get(profile_id.as_str()) else {
            continue;
        };
        let expected_pointer = prior
            .external_pointer
            .as_deref()
            .context("persisted external catalog ownership is missing its pointer")?;
        ensure!(
            parsed_catalog_pointer(&profile.config_contents)?.as_deref() == Some(expected_pointer),
            "ordinary save must preserve every external catalog pointer"
        );
    }
    for profile in &request.topology.relay_profiles {
        if persisted_settings
            .relay_profiles
            .iter()
            .any(|persisted| persisted.id == profile.id)
            || profile.relay_mode == RelayMode::Aggregate
            || !model_catalog::managed_catalog_capable(&RelayProfile::from(profile))
        {
            continue;
        }
        ensure!(
            request
                .catalog_drafts
                .iter()
                .filter(|draft| draft.profile_id == profile.id)
                .count()
                == 1,
            "new provider profile requires one complete catalog draft"
        );
    }
    Ok(())
}

fn validate_catalog_draft(
    profile: &RelayProfile,
    draft: &ProfileCatalogDraft,
) -> anyhow::Result<()> {
    ensure!(
        model_catalog::managed_catalog_capable(profile),
        "catalog-incapable provider profiles cannot carry catalog drafts"
    );
    model_catalog::validate_overlay(&draft.overlay)?;
    model_catalog::validate_upstream_topology(profile, draft.upstream_topology)?;
    match draft.mode {
        CatalogMode::External => {
            let pointer = draft
                .external_pointer
                .as_deref()
                .filter(|pointer| !pointer.trim().is_empty())
                .context("external catalog draft requires a pointer")?;
            ensure!(
                parsed_catalog_pointer(&profile.config_contents)?.as_deref() == Some(pointer),
                "external catalog pointer must exactly match profile configContents"
            );
        }
        _ => ensure!(
            draft.external_pointer.is_none(),
            "managed or native catalog draft cannot carry an external pointer"
        ),
    }
    Ok(())
}

#[cfg(test)]
fn plan_provider_detail_commit(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
) -> anyhow::Result<ProviderCommitPlan> {
    let readiness = CatalogReadinessInput {
        scope_current_by_profile: request
            .catalog_drafts
            .iter()
            .map(|draft| (draft.profile_id.clone(), true))
            .collect(),
    };
    plan_provider_detail_commit_with_readiness(
        persisted_settings,
        persisted_state,
        request,
        &readiness,
    )
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CatalogReadinessInput {
    pub(crate) scope_current_by_profile: BTreeMap<String, bool>,
}

pub(crate) fn plan_provider_detail_commit_with_readiness(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
    readiness: &CatalogReadinessInput,
) -> anyhow::Result<ProviderCommitPlan> {
    validate_provider_detail_request(persisted_settings, persisted_state, request)?;
    plan_validated_request(
        persisted_settings,
        persisted_state,
        request,
        request.catalog_drafts.clone(),
        readiness,
    )
}

pub(crate) fn validate_provider_topology_request(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
) -> anyhow::Result<()> {
    ensure!(
        request.focused_profile_id.is_none(),
        "topology request cannot select a focused provider profile"
    );
    ensure!(
        request.action == ProviderCommitAction::Save,
        "topology request supports save only"
    );
    validate_common_request(persisted_settings, persisted_state, request)
}

#[cfg(test)]
fn plan_provider_topology_commit(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
) -> anyhow::Result<ProviderCommitPlan> {
    let readiness = CatalogReadinessInput {
        scope_current_by_profile: request
            .catalog_drafts
            .iter()
            .map(|draft| (draft.profile_id.clone(), true))
            .collect(),
    };
    plan_provider_topology_commit_with_readiness(
        persisted_settings,
        persisted_state,
        request,
        &readiness,
    )
}

pub(crate) fn plan_provider_topology_commit_with_readiness(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
    readiness: &CatalogReadinessInput,
) -> anyhow::Result<ProviderCommitPlan> {
    validate_provider_topology_request(persisted_settings, persisted_state, request)?;
    plan_validated_request(
        persisted_settings,
        persisted_state,
        request,
        request.catalog_drafts.clone(),
        readiness,
    )
}

fn plan_validated_request(
    persisted_settings: &BackendSettings,
    persisted_state: &CatalogState,
    request: &ProviderCommitRequest,
    drafts: Vec<ProfileCatalogDraft>,
    readiness: &CatalogReadinessInput,
) -> anyhow::Result<ProviderCommitPlan> {
    let settings = request.topology.apply_to(persisted_settings);
    let mut catalog_state = persisted_state.clone();
    let profile_ids = settings
        .relay_profiles
        .iter()
        .map(|profile| profile.id.clone())
        .collect::<BTreeSet<_>>();
    catalog_state
        .profiles
        .retain(|profile_id, _| profile_ids.contains(profile_id));

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
        model_catalog::clear_catalog_readiness_action(state);
    }
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
        let scope_current = readiness
            .scope_current_by_profile
            .get(&draft.profile_id)
            .copied()
            .unwrap_or(false);
        let catalog_readiness = model_catalog::classify_managed_catalog_readiness(
            &catalog_state,
            profile,
            &profile_state,
            scope_current,
        );
        if catalog_readiness != model_catalog::ManagedCatalogReadiness::Ready {
            if settings.active_relay_id == draft.profile_id {
                // A bundled-baseline update can retire the model an active profile starts on.
                // That failure has its own repair — pick a replacement default — so it must not
                // hide inside the generic not-ready family the user can do nothing about.
                if catalog_readiness == model_catalog::ManagedCatalogReadiness::DefaultModelAbsent {
                    anyhow::bail!(
                        "active provider default model is absent from the bundled baseline"
                    );
                }
                anyhow::bail!("active provider catalog is not ready");
            }
            let profile_state = catalog_state.profiles.get_mut(&draft.profile_id).unwrap();
            profile_state.action_required =
                Some(model_catalog::CATALOG_READINESS_ACTION.to_string());
            continue;
        }
        match model_catalog::compose_profile_catalog(&catalog_state, profile, &profile_state) {
            Ok(catalog) => {
                let bytes = serde_json::to_vec_pretty(&catalog)?;
                let hash = format!("{:x}", Sha256::digest(&bytes));
                let profile_state = catalog_state.profiles.get_mut(&draft.profile_id).unwrap();
                let artifact_changed =
                    profile_state.generated_hash.as_deref() != Some(hash.as_str());
                if artifact_changed {
                    profile_state.generation = profile_state.generation.saturating_add(1);
                }
                profile_state.generated_hash = Some(hash);
                profile_state.generated_path =
                    Some(model_catalog::generated_relative_path(&draft.profile_id));
                model_catalog::clear_catalog_readiness_action(profile_state);
                if settings.active_relay_id == draft.profile_id {
                    active_catalog = Some(catalog.clone());
                }
                if artifact_changed {
                    generated_catalogs.insert(draft.profile_id.clone(), catalog);
                }
            }
            Err(_error) if settings.active_relay_id != draft.profile_id => {
                let profile_state = catalog_state.profiles.get_mut(&draft.profile_id).unwrap();
                let code = model_catalog::CATALOG_READINESS_ACTION.to_string();
                profile_state.action_required = Some(code);
            }
            Err(error) => return Err(error),
        }
    }
    let mut generation_neutral_state = catalog_state.clone();
    generation_neutral_state.operation_generation = persisted_state.operation_generation;
    let catalog_semantics_changed =
        serde_json::to_vec(&generation_neutral_state)? != serde_json::to_vec(persisted_state)?;
    if catalog_semantics_changed {
        catalog_state.operation_generation = catalog_state.operation_generation.saturating_add(1);
    }

    Ok(ProviderCommitPlan {
        settings,
        catalog_state,
        generated_catalogs,
        active_catalog,
        draft_revision: request.draft_revision,
    })
}

#[cfg(test)]
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

fn parsed_catalog_pointer(config_contents: &str) -> anyhow::Result<Option<String>> {
    let document = config_contents
        .parse::<toml_edit::DocumentMut>()
        .context("profile configContents is not valid TOML")?;
    match document.as_table().get("model_catalog_json") {
        Some(item) => Ok(Some(
            item.as_str()
                .context("model_catalog_json must be a string")?
                .to_string(),
        )),
        None => Ok(None),
    }
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
    fn generation_fingerprint_includes_editable_catalog_semantics_only() {
        let settings = settings_with(vec![mixed_profile("relay-a", "official-a")], "relay-a");
        let topology = ProviderOwnedTopologyDraft::from_settings(&settings);
        let mut state = CatalogState::default();
        state.profiles.insert(
            "relay-a".to_string(),
            ProfileCatalogState {
                mode: CatalogMode::OfficialPlusCustom,
                ..ProfileCatalogState::default()
            },
        );
        let baseline = provider_generation_fingerprint(&topology, &state).unwrap();

        let mut editable = state.clone();
        editable
            .profiles
            .get_mut("relay-a")
            .unwrap()
            .overlay
            .custom
            .push(CustomModel {
                slug: "custom-a".to_string(),
                display_name: "Custom A".to_string(),
                ..CustomModel::default()
            });
        assert_ne!(
            provider_generation_fingerprint(&topology, &editable).unwrap(),
            baseline
        );
        let mut editable = state.clone();
        let profile = editable.profiles.get_mut("relay-a").unwrap();
        profile.mode = CatalogMode::External;
        profile.mode_explicit = true;
        profile.external_pointer = Some("/private/catalog.json".to_string());
        profile.upstream_topology = UpstreamTopology::ServerSideComposite;
        assert_ne!(
            provider_generation_fingerprint(&topology, &editable).unwrap(),
            baseline
        );

        let mut bookkeeping = state.clone();
        bookkeeping.operation_generation = 99;
        let profile = bookkeeping.profiles.get_mut("relay-a").unwrap();
        profile.legacy_model_reset_version = 42;
        profile.generated_path = Some("model-catalogs/generated.json".to_string());
        profile.generated_hash = Some("generated-hash".to_string());
        profile.generation = 17;
        profile.restart_required = true;
        profile.action_required = Some("catalog-readiness-unavailable".to_string());
        profile.provider_evidence = Some(crate::model_catalog::ProviderEvidence {
            fetched_at_ms: 1,
            endpoint: "sha256:evidence".to_string(),
            reported_slugs: vec!["reported".to_string()],
            candidate_slugs: vec!["candidate".to_string()],
        });
        profile.applied_runtime_fingerprint = Some("runtime".to_string());
        assert_eq!(
            provider_generation_fingerprint(&topology, &bookkeeping).unwrap(),
            baseline
        );
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
    fn incoming_auth_contents_rejects_whitespace_bytes() {
        let persisted = settings_with(vec![mixed_profile("relay-a", "official-a")], "relay-a");
        let mut next = persisted.clone();
        next.relay_profiles[0].auth_contents = " \n\t".to_string();
        let request = request_for(
            &persisted,
            &next,
            Some("relay-a"),
            vec![catalog_draft(
                "relay-a",
                CatalogMode::OfficialPlusCustom,
                CatalogOverlay::default(),
            )],
            ProviderCommitAction::Save,
        );

        assert!(
            validate_provider_detail_request(&persisted, &state_with_official(), &request).is_err()
        );
    }

    #[test]
    fn planner_accepts_builder_supplied_implicit_first_save_catalog_state() {
        let old = mixed_profile("old", "official-a");
        let persisted = settings_with(vec![old.clone()], "old");
        let new = mixed_profile("new", "official-a");
        let next = settings_with(vec![old, new], "old");
        let request = request_for(
            &persisted,
            &next,
            Some("new"),
            vec![implicit_mixed_catalog_draft("new")],
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

    #[test]
    fn canonical_json_rejects_partial_drafts_and_response_only_or_unknown_fields() {
        let settings = settings_with(vec![mixed_profile("relay-a", "official-a")], "relay-a");
        let mut value = serde_json::to_value(request_for(
            &settings,
            &settings,
            Some("relay-a"),
            vec![catalog_draft(
                "relay-a",
                CatalogMode::OfficialPlusCustom,
                CatalogOverlay::default(),
            )],
            ProviderCommitAction::Save,
        ))
        .unwrap();

        value["catalogDrafts"][0]
            .as_object_mut()
            .unwrap()
            .remove("overlay");
        assert!(serde_json::from_value::<ProviderCommitRequest>(value).is_err());

        for required in ["catalogDrafts", "focusedProfileId", "confirmContextCleanup"] {
            let mut value = serde_json::to_value(request_for(
                &settings,
                &settings,
                Some("relay-a"),
                vec![catalog_draft(
                    "relay-a",
                    CatalogMode::OfficialPlusCustom,
                    CatalogOverlay::default(),
                )],
                ProviderCommitAction::Save,
            ))
            .unwrap();
            value.as_object_mut().unwrap().remove(required);
            assert!(
                serde_json::from_value::<ProviderCommitRequest>(value).is_err(),
                "missing required request field {required} must be rejected"
            );
        }

        let mut value = serde_json::to_value(request_for(
            &settings,
            &settings,
            Some("relay-a"),
            vec![catalog_draft(
                "relay-a",
                CatalogMode::OfficialPlusCustom,
                CatalogOverlay::default(),
            )],
            ProviderCommitAction::Save,
        ))
        .unwrap();
        value["catalogDrafts"][0]["generatedPath"] = json!("response-only.json");
        assert!(serde_json::from_value::<ProviderCommitRequest>(value).is_err());

        let mut value = serde_json::to_value(request_for(
            &settings,
            &settings,
            Some("relay-a"),
            vec![catalog_draft(
                "relay-a",
                CatalogMode::OfficialPlusCustom,
                CatalogOverlay::default(),
            )],
            ProviderCommitAction::Save,
        ))
        .unwrap();
        value["topology"]["relayProfiles"][0]["nativeCapabilityInspection"] =
            json!({ "state": "ready" });
        assert!(serde_json::from_value::<ProviderCommitRequest>(value).is_err());

        let base = serde_json::to_value(request_for(
            &settings,
            &settings,
            Some("relay-a"),
            vec![catalog_draft(
                "relay-a",
                CatalogMode::OfficialPlusCustom,
                CatalogOverlay::default(),
            )],
            ProviderCommitAction::Save,
        ))
        .unwrap();

        let mut value = base.clone();
        value["catalogDrafts"][0]
            .as_object_mut()
            .unwrap()
            .remove("externalPointer");
        assert!(serde_json::from_value::<ProviderCommitRequest>(value).is_err());

        let mut value = base.clone();
        value["catalogDrafts"][0]["overlay"]
            .as_object_mut()
            .unwrap()
            .remove("official");
        assert!(serde_json::from_value::<ProviderCommitRequest>(value).is_err());

        let mut value = base.clone();
        value["topology"]["relayProfiles"][0]["contextSelection"]["inspection"] =
            json!("response-only");
        assert!(serde_json::from_value::<ProviderCommitRequest>(value).is_err());

        let aggregate_member = mixed_profile("relay-a", "official-a");
        let mut aggregate_stub = mixed_profile("aggregate-a", "");
        aggregate_stub.relay_mode = RelayMode::Aggregate;
        let mut aggregate_settings =
            settings_with(vec![aggregate_member, aggregate_stub], "aggregate-a");
        aggregate_settings.active_aggregate_relay_id = "aggregate-a".to_string();
        aggregate_settings.aggregate_relay_profiles = vec![AggregateRelayProfile {
            id: "aggregate-a".to_string(),
            name: "Aggregate A".to_string(),
            strategy: Default::default(),
            members: vec![codex_plus_core::settings::AggregateRelayMember {
                relay_id: "relay-a".to_string(),
                weight: 1,
            }],
        }];
        let mut value = serde_json::to_value(request_for(
            &aggregate_settings,
            &aggregate_settings,
            None,
            vec![],
            ProviderCommitAction::Save,
        ))
        .unwrap();
        value["topology"]["aggregateRelayProfiles"][0]["inspection"] = json!("response-only");
        assert!(serde_json::from_value::<ProviderCommitRequest>(value).is_err());
    }

    #[test]
    fn ordinary_save_cannot_switch_or_delete_active_but_set_current_can_switch() {
        let a = mixed_profile("relay-a", "official-a");
        let b = mixed_profile("relay-b", "official-a");
        let persisted = settings_with(vec![a.clone(), b.clone()], "relay-a");
        let next_b = settings_with(vec![a.clone(), b.clone()], "relay-b");
        let b_draft = catalog_draft(
            "relay-b",
            CatalogMode::OfficialPlusCustom,
            CatalogOverlay::default(),
        );

        let save_switch = request_for(
            &persisted,
            &next_b,
            Some("relay-b"),
            vec![b_draft.clone()],
            ProviderCommitAction::Save,
        );
        assert!(
            plan_provider_detail_commit(&persisted, &state_with_official(), &save_switch).is_err()
        );

        let set_current = request_for(
            &persisted,
            &next_b,
            Some("relay-b"),
            vec![b_draft],
            ProviderCommitAction::SetCurrent,
        );
        assert!(
            plan_provider_detail_commit(&persisted, &state_with_official(), &set_current).is_ok()
        );

        let deleted = settings_with(vec![b], "relay-b");
        let topology_delete = request_for(
            &persisted,
            &deleted,
            None,
            vec![],
            ProviderCommitAction::Save,
        );
        assert!(
            plan_provider_topology_commit(&persisted, &state_with_official(), &topology_delete)
                .is_err()
        );
    }

    #[test]
    fn next_active_managed_draft_is_always_fail_closed_even_in_topology_requests() {
        let a = mixed_profile("relay-a", "official-a");
        let b = mixed_profile("relay-b", "official-a");
        let persisted = settings_with(vec![a.clone(), b.clone()], "relay-a");
        let next = settings_with(vec![a, b], "relay-b");
        let request = request_for(
            &persisted,
            &next,
            None,
            vec![catalog_draft(
                "relay-b",
                CatalogMode::OfficialPlusCustom,
                CatalogOverlay::default(),
            )],
            ProviderCommitAction::Save,
        );
        assert!(
            plan_provider_topology_commit(&persisted, &CatalogState::default(), &request).is_err()
        );
    }

    #[test]
    fn topology_requires_complete_catalog_drafts_for_every_new_ordinary_profile() {
        let a = mixed_profile("relay-a", "official-a");
        let persisted = settings_with(vec![a.clone()], "relay-a");
        let next = settings_with(
            vec![a, mixed_profile("relay-copy", "official-a")],
            "relay-a",
        );
        let missing = request_for(&persisted, &next, None, vec![], ProviderCommitAction::Save);
        assert!(
            plan_provider_topology_commit(&persisted, &state_with_official(), &missing).is_err()
        );

        let complete = request_for(
            &persisted,
            &next,
            None,
            vec![catalog_draft(
                "relay-copy",
                CatalogMode::OfficialPlusCustom,
                CatalogOverlay::default(),
            )],
            ProviderCommitAction::Save,
        );
        assert!(
            plan_provider_topology_commit(&persisted, &state_with_official(), &complete).is_ok()
        );
    }

    #[test]
    fn catalog_incapable_profiles_require_zero_catalog_drafts() {
        let active = mixed_profile("relay-a", "official-a");
        let persisted = settings_with(vec![active.clone()], "relay-a");
        let mut chat = mixed_profile("relay-chat", "chat-model");
        chat.protocol = RelayProtocol::ChatCompletions;
        let next = settings_with(vec![active, chat.clone()], "relay-a");
        let topology = request_for(&persisted, &next, None, vec![], ProviderCommitAction::Save);
        assert!(
            plan_provider_topology_commit(&persisted, &state_with_official(), &topology).is_ok()
        );

        let persisted_chat = settings_with(vec![chat], "relay-chat");
        let detail = request_for(
            &persisted_chat,
            &persisted_chat,
            Some("relay-chat"),
            vec![],
            ProviderCommitAction::Save,
        );
        assert!(
            plan_provider_detail_commit(&persisted_chat, &CatalogState::default(), &detail).is_ok()
        );
    }

    #[test]
    fn aggregate_projection_requires_one_to_one_nonempty_unique_eligible_members_and_active_linkage()
     {
        let member = mixed_profile("relay-a", "official-a");
        let mut aggregate_stub = mixed_profile("aggregate-a", "");
        aggregate_stub.relay_mode = RelayMode::Aggregate;
        let persisted = settings_with(vec![member.clone()], "relay-a");
        let mut next = settings_with(vec![member, aggregate_stub], "aggregate-a");
        next.aggregate_relay_profiles = vec![AggregateRelayProfile {
            id: "aggregate-a".to_string(),
            name: "Aggregate A".to_string(),
            strategy: Default::default(),
            members: vec![codex_plus_core::settings::AggregateRelayMember {
                relay_id: "relay-a".to_string(),
                weight: 1,
            }],
        }];
        next.active_aggregate_relay_id = "aggregate-a".to_string();
        let valid = request_for(&persisted, &next, None, vec![], ProviderCommitAction::Save);
        assert!(
            validate_common_request(&persisted, &state_with_official(), &valid).is_err(),
            "topology Save must not switch active"
        );

        let persisted_aggregate = next.clone();
        let valid = request_for(
            &persisted_aggregate,
            &persisted_aggregate,
            None,
            vec![],
            ProviderCommitAction::Save,
        );
        assert!(
            validate_common_request(&persisted_aggregate, &state_with_official(), &valid).is_ok()
        );

        let mut invalid = valid.clone();
        invalid.topology.aggregate_relay_profiles[0].members.clear();
        assert!(
            validate_common_request(&persisted_aggregate, &state_with_official(), &invalid)
                .is_err()
        );

        let mut invalid = valid.clone();
        let duplicate_member = invalid.topology.aggregate_relay_profiles[0].members[0].clone();
        invalid.topology.aggregate_relay_profiles[0]
            .members
            .push(duplicate_member);
        assert!(
            validate_common_request(&persisted_aggregate, &state_with_official(), &invalid)
                .is_err()
        );

        let mut invalid = valid.clone();
        invalid.topology.aggregate_relay_profiles[0].members[0].weight = 0;
        assert!(
            validate_common_request(&persisted_aggregate, &state_with_official(), &invalid)
                .is_err()
        );

        let mut invalid = valid.clone();
        invalid.topology.active_aggregate_relay_id.clear();
        assert!(
            validate_common_request(&persisted_aggregate, &state_with_official(), &invalid)
                .is_err()
        );
    }

    #[test]
    fn external_identity_is_preserved_and_new_external_pointer_must_come_from_profile_toml() {
        let mut profile = mixed_profile("relay-a", "official-a");
        profile.config_contents = "model_catalog_json = \"models/user-owned.json\"\n".to_string();
        let persisted = settings_with(vec![profile.clone()], "relay-a");
        let mut state = state_with_official();
        let external_overlay = CatalogOverlay {
            custom: vec![CustomModel {
                slug: "owned".to_string(),
                display_name: "Owned".to_string(),
                ..CustomModel::default()
            }],
            ..CatalogOverlay::default()
        };
        state.profiles.insert(
            "relay-a".to_string(),
            ProfileCatalogState {
                mode: CatalogMode::External,
                mode_explicit: true,
                upstream_topology: UpstreamTopology::Direct,
                overlay: external_overlay.clone(),
                external_pointer: Some("models/user-owned.json".to_string()),
                ..ProfileCatalogState::default()
            },
        );

        let mut changed =
            catalog_draft("relay-a", CatalogMode::External, CatalogOverlay::default());
        changed.external_pointer = Some("models/injected.json".to_string());
        let request = request_for(
            &persisted,
            &persisted,
            Some("relay-a"),
            vec![changed],
            ProviderCommitAction::Save,
        );
        assert!(validate_provider_detail_request(&persisted, &state, &request).is_err());

        let mut preserved = catalog_draft("relay-a", CatalogMode::External, external_overlay);
        preserved.external_pointer = Some("models/user-owned.json".to_string());
        let request = request_for(
            &persisted,
            &persisted,
            Some("relay-a"),
            vec![preserved],
            ProviderCommitAction::Save,
        );
        assert!(validate_provider_detail_request(&persisted, &state, &request).is_ok());

        let managed_state = state_with_official();
        let mut injected =
            catalog_draft("relay-a", CatalogMode::External, CatalogOverlay::default());
        injected.external_pointer = Some("models/injected.json".to_string());
        let request = request_for(
            &persisted,
            &persisted,
            Some("relay-a"),
            vec![injected],
            ProviderCommitAction::Save,
        );
        assert!(validate_provider_detail_request(&persisted, &managed_state, &request).is_err());
    }

    #[test]
    fn draftless_topology_save_preserves_every_external_profile_pointer() {
        let mut external = mixed_profile("relay-external", "owned-model");
        external.config_contents = "model_catalog_json = \"models/user-owned.json\"\n".to_string();
        let ordinary = mixed_profile("relay-b", "official-a");
        let persisted = settings_with(vec![external.clone(), ordinary.clone()], "relay-external");
        let mut state = state_with_official();
        state.profiles.insert(
            "relay-external".to_string(),
            ProfileCatalogState {
                mode: CatalogMode::External,
                mode_explicit: true,
                external_pointer: Some("models/user-owned.json".to_string()),
                ..ProfileCatalogState::default()
            },
        );

        let mut reordered = settings_with(vec![ordinary, external], "relay-external");
        reordered.relay_profiles_enabled = false;
        let valid = request_for(
            &persisted,
            &reordered,
            None,
            vec![],
            ProviderCommitAction::Save,
        );
        assert!(validate_common_request(&persisted, &state, &valid).is_ok());

        let mut changed = reordered.clone();
        changed
            .relay_profiles
            .iter_mut()
            .find(|profile| profile.id == "relay-external")
            .unwrap()
            .config_contents = "model_catalog_json = \"models/replaced.json\"\n".to_string();
        let changed = request_for(
            &persisted,
            &changed,
            None,
            vec![],
            ProviderCommitAction::Save,
        );
        assert!(validate_common_request(&persisted, &state, &changed).is_err());

        let mut removed = reordered;
        removed
            .relay_profiles
            .iter_mut()
            .find(|profile| profile.id == "relay-external")
            .unwrap()
            .config_contents = "model = \"owned-model\"\n".to_string();
        let removed = request_for(
            &persisted,
            &removed,
            None,
            vec![],
            ProviderCommitAction::Save,
        );
        assert!(validate_common_request(&persisted, &state, &removed).is_err());
    }

    #[test]
    fn external_pointer_identity_rejects_whitespace_wrapped_replacement() {
        let mut external = mixed_profile("relay-external", "owned-model");
        external.config_contents = "model_catalog_json = \"models/user-owned.json\"\n".to_string();
        let persisted = settings_with(vec![external], "relay-external");
        let mut state = state_with_official();
        state.profiles.insert(
            "relay-external".to_string(),
            ProfileCatalogState {
                mode: CatalogMode::External,
                mode_explicit: true,
                external_pointer: Some("models/user-owned.json".to_string()),
                ..ProfileCatalogState::default()
            },
        );
        let mut replacement = persisted.clone();
        replacement.relay_profiles[0].config_contents =
            "model_catalog_json = \" models/user-owned.json \"\n".to_string();
        let request = request_for(
            &persisted,
            &replacement,
            None,
            vec![],
            ProviderCommitAction::Save,
        );

        assert!(validate_common_request(&persisted, &state, &request).is_err());
    }

    #[test]
    fn external_pointer_identity_accepts_unchanged_pointer_with_spaces() {
        let pointer = " models/user-owned.json ";
        let mut external = mixed_profile("relay-external", "owned-model");
        external.config_contents = format!("model_catalog_json = \"{pointer}\"\n");
        let persisted = settings_with(vec![external], "relay-external");
        let mut state = state_with_official();
        state.profiles.insert(
            "relay-external".to_string(),
            ProfileCatalogState {
                mode: CatalogMode::External,
                mode_explicit: true,
                external_pointer: Some(pointer.to_string()),
                ..ProfileCatalogState::default()
            },
        );
        let request = request_for(
            &persisted,
            &persisted,
            None,
            vec![],
            ProviderCommitAction::Save,
        );

        assert!(validate_common_request(&persisted, &state, &request).is_ok());
    }

    #[test]
    fn semantic_noop_does_not_increment_operation_generation_or_schedule_unchanged_artifact() {
        let settings = settings_with(vec![mixed_profile("relay-a", "official-a")], "relay-a");
        let draft = catalog_draft(
            "relay-a",
            CatalogMode::OfficialPlusCustom,
            CatalogOverlay::default(),
        );
        let first = request_for(
            &settings,
            &settings,
            Some("relay-a"),
            vec![draft.clone()],
            ProviderCommitAction::Save,
        );
        let first_plan =
            plan_provider_detail_commit(&settings, &state_with_official(), &first).unwrap();
        let second = request_for(
            &settings,
            &settings,
            Some("relay-a"),
            vec![draft],
            ProviderCommitAction::Save,
        );
        let second_plan =
            plan_provider_detail_commit(&settings, &first_plan.catalog_state, &second).unwrap();

        assert_eq!(
            second_plan.catalog_state.operation_generation,
            first_plan.catalog_state.operation_generation
        );
        assert!(second_plan.generated_catalogs.is_empty());
        assert!(second_plan.active_catalog.is_some());
    }

    #[test]
    fn unchanged_action_required_plan_does_not_increment_operation_generation() {
        let active = mixed_profile("relay-a", "official-a");
        let inactive = mixed_profile("relay-b", "official-a");
        let settings = settings_with(vec![active, inactive], "relay-a");
        let draft = catalog_draft(
            "relay-b",
            CatalogMode::OfficialPlusCustom,
            CatalogOverlay::default(),
        );
        let mut state = CatalogState::default();
        state.operation_generation = 7;
        state.profiles.insert(
            "relay-b".to_string(),
            ProfileCatalogState {
                mode: CatalogMode::OfficialPlusCustom,
                mode_explicit: true,
                upstream_topology: UpstreamTopology::Direct,
                overlay: CatalogOverlay::default(),
                action_required: Some("catalog-readiness-unavailable".to_string()),
                ..ProfileCatalogState::default()
            },
        );
        let request = request_for(
            &settings,
            &settings,
            Some("relay-b"),
            vec![draft],
            ProviderCommitAction::Save,
        );

        let plan = plan_provider_detail_commit(&settings, &state, &request).unwrap();

        assert_eq!(plan.catalog_state.operation_generation, 7);
        assert_eq!(
            plan.catalog_state.profiles["relay-b"]
                .action_required
                .as_deref(),
            Some("catalog-readiness-unavailable")
        );
        assert!(plan.generated_catalogs.is_empty());
    }
}
