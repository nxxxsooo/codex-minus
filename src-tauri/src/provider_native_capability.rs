use std::collections::BTreeMap;
use std::path::Path;

use codex_plus_core::settings::{BackendSettings, RelayMode, RelayProfile, RelayProtocol};
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, TableLike};

use crate::commands::CommandResult;

pub use crate::model_catalog::CatalogMode;

pub const MANAGED_ACTOR_HEADER_NAME: &str = "x-openai-actor-authorization";
pub const MANAGED_ACTOR_HEADER_VALUE: &str = "local-image-extension";
pub const MANAGED_PROVIDER_NAME: &str = "OpenAI";
pub const MANAGED_WIRE_API: &str = "responses";

const RESERVED_PROVIDER_ID: &str = "openai";
const LEGACY_PROVIDER_IDS: [&str; 2] = ["CodexPlusPlus", "CodexPP"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeCapabilityState {
    NativePriority,
    UpgradeAvailable,
    Degraded,
    Compatibility,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeCapabilityField {
    RelayMode,
    Protocol,
    Catalog,
    Configuration,
    ProviderSelection,
    BaseUrl,
    Model,
    ProviderName,
    WireApi,
    RequiresOpenAiAuth,
    ProviderBearer,
    ActorHeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeCapabilityOutcome {
    Satisfied,
    Missing,
    Mismatch,
    Conflict,
    Malformed,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeCapabilityReason {
    Canonical,
    ExternalCatalog,
    PureOAuth,
    PureApi,
    Aggregate,
    UnsupportedRelayMode,
    ChatCompletions,
    CatalogModeMismatch,
    MalformedToml,
    MissingProviderSelection,
    ReservedProviderId,
    LegacyProviderIdRequiresRename,
    SelectedProviderTableMissing,
    MalformedProviderTable,
    MissingBaseUrl,
    MalformedBaseUrl,
    MissingModel,
    MalformedModel,
    MissingProviderName,
    ProviderNameMismatch,
    MalformedProviderName,
    MissingWireApi,
    WireApiMismatch,
    MalformedWireApi,
    MissingOpenAiAuthRequirement,
    OpenAiAuthRequired,
    MalformedOpenAiAuthRequirement,
    MissingProviderBearer,
    MalformedProviderBearer,
    StructuredKeyBearerConflict,
    MissingActorHeader,
    ActorHeaderNameMismatch,
    ActorHeaderValueConflict,
    DuplicateActorHeader,
    MalformedHeaderStructure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCapabilityFieldResult {
    pub field: NativeCapabilityField,
    pub outcome: NativeCapabilityOutcome,
    pub reason: NativeCapabilityReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderNativeCapabilityInspection {
    pub profile_id: String,
    pub state: NativeCapabilityState,
    pub fields: Vec<NativeCapabilityFieldResult>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderNativeCapabilityInspectionRequest {
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderNativeCapabilityInspectionPayload {
    pub inspections: Vec<ProviderNativeCapabilityInspection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderNativeCapabilityInspectionError {
    InputUnavailable,
    ProfileNotFound,
}

fn field(
    field: NativeCapabilityField,
    outcome: NativeCapabilityOutcome,
    reason: NativeCapabilityReason,
) -> NativeCapabilityFieldResult {
    NativeCapabilityFieldResult {
        field,
        outcome,
        reason,
    }
}

fn canonical(field_name: NativeCapabilityField) -> NativeCapabilityFieldResult {
    field(
        field_name,
        NativeCapabilityOutcome::Satisfied,
        NativeCapabilityReason::Canonical,
    )
}

pub fn inspect_profile(
    profile: &RelayProfile,
    catalog_mode: CatalogMode,
) -> ProviderNativeCapabilityInspection {
    if catalog_mode == CatalogMode::External {
        return inspection(
            profile,
            NativeCapabilityState::NotApplicable,
            vec![field(
                NativeCapabilityField::Catalog,
                NativeCapabilityOutcome::NotApplicable,
                NativeCapabilityReason::ExternalCatalog,
            )],
        );
    }

    match profile.relay_mode {
        RelayMode::Aggregate => {
            return inspection(
                profile,
                NativeCapabilityState::NotApplicable,
                vec![field(
                    NativeCapabilityField::RelayMode,
                    NativeCapabilityOutcome::NotApplicable,
                    NativeCapabilityReason::Aggregate,
                )],
            );
        }
        RelayMode::PureApi => {
            return inspection(
                profile,
                NativeCapabilityState::Compatibility,
                vec![field(
                    NativeCapabilityField::RelayMode,
                    NativeCapabilityOutcome::Mismatch,
                    NativeCapabilityReason::PureApi,
                )],
            );
        }
        RelayMode::Official if !profile.official_mix_api_key => {
            return inspection(
                profile,
                NativeCapabilityState::NotApplicable,
                vec![field(
                    NativeCapabilityField::RelayMode,
                    NativeCapabilityOutcome::NotApplicable,
                    NativeCapabilityReason::PureOAuth,
                )],
            );
        }
        RelayMode::Official => {}
        RelayMode::MixedApi => {
            return inspection(
                profile,
                NativeCapabilityState::Compatibility,
                vec![field(
                    NativeCapabilityField::RelayMode,
                    NativeCapabilityOutcome::Mismatch,
                    NativeCapabilityReason::UnsupportedRelayMode,
                )],
            );
        }
    }

    if profile.protocol != RelayProtocol::Responses {
        return inspection(
            profile,
            NativeCapabilityState::Compatibility,
            vec![
                canonical(NativeCapabilityField::RelayMode),
                field(
                    NativeCapabilityField::Protocol,
                    NativeCapabilityOutcome::Mismatch,
                    NativeCapabilityReason::ChatCompletions,
                ),
            ],
        );
    }

    let mut fields = vec![
        canonical(NativeCapabilityField::RelayMode),
        canonical(NativeCapabilityField::Protocol),
    ];
    if catalog_mode != CatalogMode::OfficialPlusCustom {
        fields.push(field(
            NativeCapabilityField::Catalog,
            NativeCapabilityOutcome::Mismatch,
            NativeCapabilityReason::CatalogModeMismatch,
        ));
        return inspection(profile, NativeCapabilityState::Degraded, fields);
    }
    fields.push(canonical(NativeCapabilityField::Catalog));

    let document = match profile.config_contents.parse::<DocumentMut>() {
        Ok(document) => document,
        Err(_) => {
            fields.push(field(
                NativeCapabilityField::Configuration,
                NativeCapabilityOutcome::Malformed,
                NativeCapabilityReason::MalformedToml,
            ));
            return inspection(profile, NativeCapabilityState::Degraded, fields);
        }
    };

    evaluate_document(profile, &document, fields)
}

fn evaluate_document(
    profile: &RelayProfile,
    document: &DocumentMut,
    mut fields: Vec<NativeCapabilityFieldResult>,
) -> ProviderNativeCapabilityInspection {
    let provider_id = document
        .get("model_provider")
        .and_then(Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let provider_item = provider_id.and_then(|id| {
        document
            .get("model_providers")
            .and_then(Item::as_table_like)
            .and_then(|providers| providers.get(id))
    });

    let provider_selection = match provider_id {
        None => field(
            NativeCapabilityField::ProviderSelection,
            NativeCapabilityOutcome::Missing,
            NativeCapabilityReason::MissingProviderSelection,
        ),
        Some(RESERVED_PROVIDER_ID) => field(
            NativeCapabilityField::ProviderSelection,
            NativeCapabilityOutcome::Conflict,
            NativeCapabilityReason::ReservedProviderId,
        ),
        Some(id) if LEGACY_PROVIDER_IDS.contains(&id) => field(
            NativeCapabilityField::ProviderSelection,
            NativeCapabilityOutcome::Mismatch,
            NativeCapabilityReason::LegacyProviderIdRequiresRename,
        ),
        Some(_) if provider_item.is_none() => field(
            NativeCapabilityField::ProviderSelection,
            NativeCapabilityOutcome::Missing,
            NativeCapabilityReason::SelectedProviderTableMissing,
        ),
        Some(_) if provider_item.and_then(Item::as_table_like).is_none() => field(
            NativeCapabilityField::ProviderSelection,
            NativeCapabilityOutcome::Malformed,
            NativeCapabilityReason::MalformedProviderTable,
        ),
        Some(_) => canonical(NativeCapabilityField::ProviderSelection),
    };
    fields.push(provider_selection);

    let provider = provider_item.and_then(Item::as_table_like);
    let base_url = string_contract_field(
        provider.and_then(|table| table.get("base_url")),
        NativeCapabilityField::BaseUrl,
        NativeCapabilityReason::MissingBaseUrl,
        NativeCapabilityReason::MalformedBaseUrl,
        None,
        None,
    );
    fields.push(base_url);

    let model = string_contract_field(
        document.get("model"),
        NativeCapabilityField::Model,
        NativeCapabilityReason::MissingModel,
        NativeCapabilityReason::MalformedModel,
        None,
        None,
    );
    fields.push(model);

    let provider_name = string_contract_field(
        provider.and_then(|table| table.get("name")),
        NativeCapabilityField::ProviderName,
        NativeCapabilityReason::MissingProviderName,
        NativeCapabilityReason::MalformedProviderName,
        Some(MANAGED_PROVIDER_NAME),
        Some(NativeCapabilityReason::ProviderNameMismatch),
    );
    fields.push(provider_name);

    let wire_api = string_contract_field(
        provider.and_then(|table| table.get("wire_api")),
        NativeCapabilityField::WireApi,
        NativeCapabilityReason::MissingWireApi,
        NativeCapabilityReason::MalformedWireApi,
        Some(MANAGED_WIRE_API),
        Some(NativeCapabilityReason::WireApiMismatch),
    );
    fields.push(wire_api);

    let requires_openai_auth = match provider.and_then(|table| table.get("requires_openai_auth")) {
        None => field(
            NativeCapabilityField::RequiresOpenAiAuth,
            NativeCapabilityOutcome::Missing,
            NativeCapabilityReason::MissingOpenAiAuthRequirement,
        ),
        Some(item) => match item.as_bool() {
            Some(false) => canonical(NativeCapabilityField::RequiresOpenAiAuth),
            Some(true) => field(
                NativeCapabilityField::RequiresOpenAiAuth,
                NativeCapabilityOutcome::Mismatch,
                NativeCapabilityReason::OpenAiAuthRequired,
            ),
            None => field(
                NativeCapabilityField::RequiresOpenAiAuth,
                NativeCapabilityOutcome::Malformed,
                NativeCapabilityReason::MalformedOpenAiAuthRequirement,
            ),
        },
    };
    fields.push(requires_openai_auth);

    let bearer = provider.and_then(|table| table.get("experimental_bearer_token"));
    let bearer_field = match bearer {
        None => field(
            NativeCapabilityField::ProviderBearer,
            NativeCapabilityOutcome::Missing,
            NativeCapabilityReason::MissingProviderBearer,
        ),
        Some(item) => match item.as_str() {
            Some(value) if value.trim().is_empty() => field(
                NativeCapabilityField::ProviderBearer,
                NativeCapabilityOutcome::Missing,
                NativeCapabilityReason::MissingProviderBearer,
            ),
            Some(value)
                if !profile.api_key.trim().is_empty() && profile.api_key.trim() != value.trim() =>
            {
                field(
                    NativeCapabilityField::ProviderBearer,
                    NativeCapabilityOutcome::Conflict,
                    NativeCapabilityReason::StructuredKeyBearerConflict,
                )
            }
            Some(_) => canonical(NativeCapabilityField::ProviderBearer),
            None => field(
                NativeCapabilityField::ProviderBearer,
                NativeCapabilityOutcome::Malformed,
                NativeCapabilityReason::MalformedProviderBearer,
            ),
        },
    };
    fields.push(bearer_field);

    let actor_header = evaluate_actor_header(provider);
    fields.push(actor_header);

    let alias_requires_rename = fields.iter().any(|entry| {
        entry.field == NativeCapabilityField::ProviderSelection
            && entry.reason == NativeCapabilityReason::LegacyProviderIdRequiresRename
    });
    let legacy_compatible = legacy_compatible_contract(provider_id, provider, &fields);
    let all_canonical = fields
        .iter()
        .all(|entry| entry.outcome == NativeCapabilityOutcome::Satisfied);
    let state = if all_canonical {
        NativeCapabilityState::NativePriority
    } else if alias_requires_rename && only_alias_mismatch(&fields) || legacy_compatible {
        NativeCapabilityState::UpgradeAvailable
    } else {
        NativeCapabilityState::Degraded
    };
    inspection(profile, state, fields)
}

fn string_contract_field(
    item: Option<&Item>,
    field_name: NativeCapabilityField,
    missing_reason: NativeCapabilityReason,
    malformed_reason: NativeCapabilityReason,
    expected: Option<&str>,
    mismatch_reason: Option<NativeCapabilityReason>,
) -> NativeCapabilityFieldResult {
    let Some(item) = item else {
        return field(field_name, NativeCapabilityOutcome::Missing, missing_reason);
    };
    let Some(value) = item.as_str() else {
        return field(
            field_name,
            NativeCapabilityOutcome::Malformed,
            malformed_reason,
        );
    };
    if value.trim().is_empty() {
        return field(field_name, NativeCapabilityOutcome::Missing, missing_reason);
    }
    if expected.is_some_and(|expected| value != expected) {
        return field(
            field_name,
            NativeCapabilityOutcome::Mismatch,
            mismatch_reason.expect("mismatch reason is required with an expected value"),
        );
    }
    canonical(field_name)
}

fn evaluate_actor_header(provider: Option<&dyn TableLike>) -> NativeCapabilityFieldResult {
    let Some(headers) = provider.and_then(|table| table.get("http_headers")) else {
        return field(
            NativeCapabilityField::ActorHeader,
            NativeCapabilityOutcome::Missing,
            NativeCapabilityReason::MissingActorHeader,
        );
    };
    let Some(headers) = headers.as_table_like() else {
        return field(
            NativeCapabilityField::ActorHeader,
            NativeCapabilityOutcome::Malformed,
            NativeCapabilityReason::MalformedHeaderStructure,
        );
    };
    let matches = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case(MANAGED_ACTOR_HEADER_NAME))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => field(
            NativeCapabilityField::ActorHeader,
            NativeCapabilityOutcome::Missing,
            NativeCapabilityReason::MissingActorHeader,
        ),
        [(_, _), _, ..] => field(
            NativeCapabilityField::ActorHeader,
            NativeCapabilityOutcome::Conflict,
            NativeCapabilityReason::DuplicateActorHeader,
        ),
        [(name, value)] if *name != MANAGED_ACTOR_HEADER_NAME => field(
            NativeCapabilityField::ActorHeader,
            NativeCapabilityOutcome::Mismatch,
            NativeCapabilityReason::ActorHeaderNameMismatch,
        ),
        [(_, value)] => match value.as_str() {
            Some(MANAGED_ACTOR_HEADER_VALUE) => canonical(NativeCapabilityField::ActorHeader),
            Some(_) => field(
                NativeCapabilityField::ActorHeader,
                NativeCapabilityOutcome::Conflict,
                NativeCapabilityReason::ActorHeaderValueConflict,
            ),
            None => field(
                NativeCapabilityField::ActorHeader,
                NativeCapabilityOutcome::Malformed,
                NativeCapabilityReason::MalformedHeaderStructure,
            ),
        },
    }
}

fn legacy_compatible_contract(
    provider_id: Option<&str>,
    provider: Option<&dyn TableLike>,
    fields: &[NativeCapabilityFieldResult],
) -> bool {
    if provider_id != Some("custom") {
        return false;
    }
    let legacy_values = provider.is_some_and(|provider| {
        provider.get("name").and_then(Item::as_str) == Some("custom")
            && provider.get("wire_api").and_then(Item::as_str) == Some(MANAGED_WIRE_API)
            && provider.get("requires_openai_auth").and_then(Item::as_bool) == Some(true)
            && provider.get("http_headers").is_none()
    });
    legacy_values
        && fields.iter().all(|entry| {
            entry.outcome == NativeCapabilityOutcome::Satisfied
                || matches!(
                    entry.reason,
                    NativeCapabilityReason::ProviderNameMismatch
                        | NativeCapabilityReason::OpenAiAuthRequired
                        | NativeCapabilityReason::MissingActorHeader
                )
        })
}

fn only_alias_mismatch(fields: &[NativeCapabilityFieldResult]) -> bool {
    fields.iter().all(|entry| {
        entry.outcome == NativeCapabilityOutcome::Satisfied
            || entry.reason == NativeCapabilityReason::LegacyProviderIdRequiresRename
    })
}

fn inspection(
    profile: &RelayProfile,
    state: NativeCapabilityState,
    fields: Vec<NativeCapabilityFieldResult>,
) -> ProviderNativeCapabilityInspection {
    ProviderNativeCapabilityInspection {
        profile_id: profile.id.clone(),
        state,
        fields,
    }
}

pub fn inspect_profiles(
    settings: &BackendSettings,
    catalog_modes: &BTreeMap<String, CatalogMode>,
    profile_id: Option<&str>,
) -> Result<Vec<ProviderNativeCapabilityInspection>, ProviderNativeCapabilityInspectionError> {
    let profiles = match profile_id {
        Some(profile_id) => vec![
            settings
                .relay_profiles
                .iter()
                .find(|profile| profile.id == profile_id)
                .ok_or(ProviderNativeCapabilityInspectionError::ProfileNotFound)?,
        ],
        None => settings.relay_profiles.iter().collect(),
    };
    Ok(profiles
        .into_iter()
        .map(|profile| {
            let mode = catalog_modes
                .get(&profile.id)
                .copied()
                .unwrap_or_else(|| crate::model_catalog::default_catalog_mode_for_profile(profile));
            inspect_profile(profile, mode)
        })
        .collect())
}

pub fn inspect_provider_native_capabilities_from_paths(
    settings_path: &Path,
    catalog_state_path: &Path,
    request: ProviderNativeCapabilityInspectionRequest,
) -> Result<ProviderNativeCapabilityInspectionPayload, ProviderNativeCapabilityInspectionError> {
    let settings_bytes = std::fs::read(settings_path)
        .map_err(|_| ProviderNativeCapabilityInspectionError::InputUnavailable)?;
    let mut settings = serde_json::from_slice::<BackendSettings>(&settings_bytes)
        .map_err(|_| ProviderNativeCapabilityInspectionError::InputUnavailable)?;
    for profile in &mut settings.relay_profiles {
        profile.auth_contents.clear();
    }
    let catalog_modes =
        crate::model_catalog::read_only_catalog_modes_from_path(&settings, catalog_state_path)
            .map_err(|_| ProviderNativeCapabilityInspectionError::InputUnavailable)?;
    let inspections = inspect_profiles(&settings, &catalog_modes, request.profile_id.as_deref())?;
    Ok(ProviderNativeCapabilityInspectionPayload { inspections })
}

#[tauri::command]
pub async fn inspect_provider_native_capabilities(
    request: Option<ProviderNativeCapabilityInspectionRequest>,
) -> CommandResult<ProviderNativeCapabilityInspectionPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        inspect_command_blocking(request.unwrap_or_default())
    })
    .await
    .expect("blocking command panicked")
}

fn inspect_command_blocking(
    request: ProviderNativeCapabilityInspectionRequest,
) -> CommandResult<ProviderNativeCapabilityInspectionPayload> {
    let result = inspect_provider_native_capabilities_from_paths(
        &codex_plus_core::paths::default_settings_path(),
        &crate::model_catalog::catalog_state_path(),
        request,
    );
    match result {
        Ok(payload) => CommandResult {
            status: "ok".to_string(),
            message: "供应商原生能力状态已读取。".to_string(),
            payload,
        },
        Err(_) => CommandResult {
            status: "failed".to_string(),
            message: "供应商原生能力状态读取失败。".to_string(),
            payload: ProviderNativeCapabilityInspectionPayload::default(),
        },
    }
}
