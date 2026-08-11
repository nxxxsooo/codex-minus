use std::path::Path;

use anyhow::{Context, ensure};
use codex_plus_core::settings::{BackendSettings, RelayMode};
use serde::{Deserialize, Serialize};

use crate::commands::CommandResult;
use crate::model_catalog::{CatalogState, ProfileCatalogState};
use crate::provider_native_capability::{
    NativeCapabilityField, NativeCapabilityOutcome, NativeCapabilityState,
    ProviderNativeCapabilityInspection, ProviderNativeCapabilityInspectionRequest,
    inspect_provider_native_capabilities_from_paths,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilityEvidenceRequest {
    pub profile_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderContractEvidence {
    Ready,
    UpgradeAvailable,
    Conflicting,
    Invalid,
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OAuthSessionEvidence {
    SignedIn,
    SignedOut,
    Expired,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalPlanEvidence {
    Free,
    Paid,
    Other,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActorMarkerEvidence {
    Eligible,
    Ineligible,
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogModelEvidence {
    Supported,
    MissingMetadata,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeEvidence {
    RestartRequired,
    NewTaskRequired,
    Adopted,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderRouteEvidence {
    NativePriorityMixed,
    KeyOnlyPureApi,
    LegacyCompatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TextResponsesEvidence {
    Reachable,
    FallbackReachable,
    Denied,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageGenerationEvidence {
    PermissionVerified,
    Denied,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ImagePlanEvidence {
    Unknown,
    VerifiedTargetPolicy {
        policy_source: String,
        target_version: String,
        capability_path: String,
        free_plan_rule: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilityEvidencePayload {
    pub profile_id: String,
    pub provider_contract: ProviderContractEvidence,
    pub oauth_session: OAuthSessionEvidence,
    pub local_plan: LocalPlanEvidence,
    pub actor_marker: ActorMarkerEvidence,
    pub catalog_model: CatalogModelEvidence,
    pub text_responses: TextResponsesEvidence,
    pub image_generation: ImageGenerationEvidence,
    pub runtime: RuntimeEvidence,
    pub route_kind: ProviderRouteEvidence,
    pub image_plan_evidence: ImagePlanEvidence,
}

impl Default for ProviderCapabilityEvidencePayload {
    fn default() -> Self {
        Self {
            profile_id: String::new(),
            provider_contract: ProviderContractEvidence::Unknown,
            oauth_session: OAuthSessionEvidence::Unknown,
            local_plan: LocalPlanEvidence::Unknown,
            actor_marker: ActorMarkerEvidence::Unknown,
            catalog_model: CatalogModelEvidence::Unknown,
            text_responses: TextResponsesEvidence::Unknown,
            image_generation: ImageGenerationEvidence::Unknown,
            runtime: RuntimeEvidence::Unknown,
            route_kind: ProviderRouteEvidence::LegacyCompatibility,
            image_plan_evidence: ImagePlanEvidence::Unknown,
        }
    }
}

pub fn inspect_provider_capability_evidence_from_paths(
    settings_path: &Path,
    catalog_state_path: &Path,
    codex_home: &Path,
    request: ProviderCapabilityEvidenceRequest,
) -> anyhow::Result<ProviderCapabilityEvidencePayload> {
    ensure!(
        !request.profile_id.trim().is_empty(),
        "profile ID is required"
    );
    let settings_bytes = std::fs::read(settings_path).context("settings unavailable")?;
    let settings =
        serde_json::from_slice::<BackendSettings>(&settings_bytes).context("settings invalid")?;
    let profile = settings
        .relay_profiles
        .iter()
        .find(|profile| profile.id == request.profile_id)
        .context("profile unavailable")?;
    let inspection_payload = inspect_provider_native_capabilities_from_paths(
        settings_path,
        catalog_state_path,
        ProviderNativeCapabilityInspectionRequest {
            profile_id: Some(request.profile_id.clone()),
        },
    )
    .map_err(|_| anyhow::anyhow!("provider inspection unavailable"))?;
    let inspection = inspection_payload
        .inspections
        .first()
        .context("provider inspection unavailable")?;
    let catalog_state = read_catalog_state(catalog_state_path)?;
    let profile_catalog = catalog_state.profiles.get(&request.profile_id);
    let auth = codex_plus_core::relay_config::chatgpt_auth_status_from_home(codex_home);

    Ok(ProviderCapabilityEvidencePayload {
        profile_id: request.profile_id,
        provider_contract: provider_contract_evidence(inspection),
        oauth_session: if auth.authenticated {
            OAuthSessionEvidence::SignedIn
        } else {
            OAuthSessionEvidence::SignedOut
        },
        local_plan: LocalPlanEvidence::Unknown,
        actor_marker: actor_marker_evidence(inspection),
        catalog_model: catalog_model_evidence(profile_catalog),
        text_responses: TextResponsesEvidence::Unknown,
        image_generation: ImageGenerationEvidence::Unknown,
        runtime: runtime_evidence(profile_catalog),
        route_kind: route_evidence(profile.relay_mode, inspection.state),
        // No verified target-version policy table exists yet. A trusted target identity alone is
        // insufficient evidence of a plan rule, so the producer must remain conservative.
        image_plan_evidence: ImagePlanEvidence::Unknown,
    })
}

#[tauri::command]
pub async fn inspect_provider_capability_evidence(
    request: ProviderCapabilityEvidenceRequest,
) -> CommandResult<ProviderCapabilityEvidencePayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = inspect_provider_capability_evidence_from_paths(
            &codex_plus_core::paths::default_settings_path(),
            &crate::model_catalog::catalog_state_path(),
            &codex_plus_core::relay_config::default_codex_home_dir(),
            request,
        );
        match result {
            Ok(payload) => CommandResult {
                status: "ok".to_string(),
                message: "供应商能力证据已读取。".to_string(),
                payload,
            },
            Err(_) => CommandResult {
                status: "failed".to_string(),
                message: "供应商能力证据读取失败。".to_string(),
                payload: ProviderCapabilityEvidencePayload::default(),
            },
        }
    })
    .await
    .expect("blocking command panicked")
}

fn read_catalog_state(path: &Path) -> anyhow::Result<CatalogState> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("catalog state invalid"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(CatalogState::default()),
        Err(error) => Err(error.into()),
    }
}

fn provider_contract_evidence(
    inspection: &ProviderNativeCapabilityInspection,
) -> ProviderContractEvidence {
    match inspection.state {
        NativeCapabilityState::NativePriority => ProviderContractEvidence::Ready,
        NativeCapabilityState::UpgradeAvailable => ProviderContractEvidence::UpgradeAvailable,
        NativeCapabilityState::Degraded
            if inspection.fields.iter().any(|field| {
                matches!(
                    field.outcome,
                    NativeCapabilityOutcome::Malformed | NativeCapabilityOutcome::Missing
                )
            }) =>
        {
            ProviderContractEvidence::Invalid
        }
        NativeCapabilityState::Degraded => ProviderContractEvidence::Conflicting,
        NativeCapabilityState::Compatibility => ProviderContractEvidence::Invalid,
        NativeCapabilityState::NotApplicable => ProviderContractEvidence::NotApplicable,
    }
}

fn actor_marker_evidence(
    inspection: &crate::provider_native_capability::ProviderNativeCapabilityInspection,
) -> ActorMarkerEvidence {
    match inspection
        .fields
        .iter()
        .find(|field| field.field == NativeCapabilityField::ActorHeader)
        .map(|field| field.outcome)
    {
        Some(NativeCapabilityOutcome::Satisfied) => ActorMarkerEvidence::Eligible,
        Some(NativeCapabilityOutcome::NotApplicable) => ActorMarkerEvidence::NotApplicable,
        Some(_) => ActorMarkerEvidence::Ineligible,
        None => ActorMarkerEvidence::Unknown,
    }
}

fn catalog_model_evidence(state: Option<&ProfileCatalogState>) -> CatalogModelEvidence {
    match state {
        Some(state) if state.action_required.is_some() => CatalogModelEvidence::Stale,
        // A materialized catalog hash proves only that an artifact exists. It does not prove
        // capability metadata for the selected model, so remain conservative until that exact
        // metadata is projected by a trusted catalog evaluator.
        Some(_) => CatalogModelEvidence::MissingMetadata,
        None => CatalogModelEvidence::Unknown,
    }
}

fn runtime_evidence(state: Option<&ProfileCatalogState>) -> RuntimeEvidence {
    match state {
        Some(state) if state.restart_required => RuntimeEvidence::RestartRequired,
        _ => RuntimeEvidence::Unknown,
    }
}

fn route_evidence(relay_mode: RelayMode, state: NativeCapabilityState) -> ProviderRouteEvidence {
    if state == NativeCapabilityState::NativePriority {
        ProviderRouteEvidence::NativePriorityMixed
    } else if relay_mode == RelayMode::PureApi {
        ProviderRouteEvidence::KeyOnlyPureApi
    } else {
        ProviderRouteEvidence::LegacyCompatibility
    }
}
