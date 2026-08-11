use std::collections::BTreeMap;
use std::path::Path;

use codex_plus_core::settings::{BackendSettings, RelayMode, RelayProfile, RelayProtocol};
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, InlineTable, Item, TableLike, Value, value};

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
    AuthContentsForbidden,
    ReplacementProviderIdRequired,
    ReplacementProviderIdInvalid,
    ReplacementProviderIdUnavailable,
    ConflictingKeySynchronization,
    DestructiveExitConfirmationRequired,
    CapabilityLossConfirmationRequired,
    RawProviderContractChangeRequiresExplicitAction,
    RawProviderSourceRequired,
    RawProviderSourceInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeCapabilityDraftAction {
    Inspect,
    ValidateRawEdit,
    EnableNativePriority,
    ExitPureApi,
    ExitLegacyCompatibility,
    ExitPureOAuth,
    ExitChatCompletions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeCapabilityDraftConfirmation {
    ReplaceActorHeader,
    UseStructuredKey,
    UseProviderBearer,
    ConfirmDestructivePureOAuth,
    ConfirmCapabilityLoss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeCapabilityDraftStatus {
    Ready,
    Blocked,
    ConfirmationRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderNativeCapabilityDraftRequest {
    pub draft_revision: u64,
    pub profile: RelayProfile,
    pub catalog_mode: CatalogMode,
    pub action: NativeCapabilityDraftAction,
    #[serde(default)]
    pub source_config_contents: Option<String>,
    #[serde(default)]
    pub confirmations: Vec<NativeCapabilityDraftConfirmation>,
    #[serde(default)]
    pub replacement_provider_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderNativeCapabilityDraftPreview {
    pub capability_loss: bool,
    pub removes_provider_table: bool,
    pub removed_provider_id: Option<String>,
    pub removed_provider_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderNativeCapabilityDraft {
    pub profile: RelayProfile,
    pub structured_api_key: String,
    pub catalog_mode: CatalogMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderNativeCapabilityDraftPayload {
    pub draft_revision: u64,
    pub status: NativeCapabilityDraftStatus,
    pub draft: ProviderNativeCapabilityDraft,
    pub inspection: ProviderNativeCapabilityInspection,
    pub blockers: Vec<NativeCapabilityReason>,
    pub preview: ProviderNativeCapabilityDraftPreview,
}

pub trait ProviderNativeCapabilityDraftReadOnlyBoundary {
    fn inspect(
        &self,
        profile: &RelayProfile,
        catalog_mode: CatalogMode,
    ) -> ProviderNativeCapabilityInspection;
}

struct EvaluatorDraftReadOnlyBoundary;

impl ProviderNativeCapabilityDraftReadOnlyBoundary for EvaluatorDraftReadOnlyBoundary {
    fn inspect(
        &self,
        profile: &RelayProfile,
        catalog_mode: CatalogMode,
    ) -> ProviderNativeCapabilityInspection {
        inspect_profile(profile, catalog_mode)
    }
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

pub fn draft_provider_native_capability(
    request: &ProviderNativeCapabilityDraftRequest,
) -> ProviderNativeCapabilityDraftPayload {
    draft_provider_native_capability_with_boundary(request, &EvaluatorDraftReadOnlyBoundary)
}

pub fn draft_provider_native_capability_with_boundary(
    request: &ProviderNativeCapabilityDraftRequest,
    boundary: &dyn ProviderNativeCapabilityDraftReadOnlyBoundary,
) -> ProviderNativeCapabilityDraftPayload {
    if !request.profile.auth_contents.is_empty() {
        return unchanged_draft_payload(
            request,
            boundary,
            NativeCapabilityDraftStatus::Blocked,
            vec![NativeCapabilityReason::AuthContentsForbidden],
            ProviderNativeCapabilityDraftPreview::default(),
        );
    }

    match request.action {
        NativeCapabilityDraftAction::Inspect => ready_draft_payload(
            request,
            boundary,
            request.profile.clone(),
            request.catalog_mode,
            ProviderNativeCapabilityDraftPreview::default(),
        ),
        NativeCapabilityDraftAction::ValidateRawEdit => {
            validate_raw_provider_config_edit(request, boundary)
        }
        NativeCapabilityDraftAction::EnableNativePriority => {
            enable_native_priority_draft(request, boundary)
        }
        NativeCapabilityDraftAction::ExitPureApi
        | NativeCapabilityDraftAction::ExitLegacyCompatibility
        | NativeCapabilityDraftAction::ExitChatCompletions => {
            compatibility_exit_draft(request, boundary)
        }
        NativeCapabilityDraftAction::ExitPureOAuth => pure_oauth_exit_draft(request, boundary),
    }
}

#[tauri::command]
pub fn transform_provider_native_capability_draft(
    request: ProviderNativeCapabilityDraftRequest,
) -> ProviderNativeCapabilityDraftPayload {
    draft_provider_native_capability(&request)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawProviderContractProjection {
    provider_id: String,
    provider_shape: String,
    provider_name: String,
    wire_api: String,
    requires_openai_auth: String,
    bearer: String,
    actor_headers: Vec<(String, String)>,
}

fn raw_string_projection(item: Option<&Item>) -> String {
    match item {
        None => "missing".to_string(),
        Some(item) => item
            .as_str()
            .map(|value| format!("string:{value}"))
            .unwrap_or_else(|| "malformed".to_string()),
    }
}

fn raw_bool_projection(item: Option<&Item>) -> String {
    match item {
        None => "missing".to_string(),
        Some(item) => item
            .as_bool()
            .map(|value| format!("bool:{value}"))
            .unwrap_or_else(|| "malformed".to_string()),
    }
}

fn raw_provider_contract_projection(document: &DocumentMut) -> RawProviderContractProjection {
    let provider_id = document.get("model_provider").and_then(Item::as_str);
    let provider_item = provider_id.and_then(|id| {
        document
            .get("model_providers")
            .and_then(Item::as_table_like)
            .and_then(|providers| providers.get(id))
    });
    let provider = provider_item.and_then(Item::as_table_like);
    let provider_shape = match provider_item {
        None => "missing",
        Some(_) if provider.is_none() => "malformed",
        Some(_) => "table",
    }
    .to_string();
    let mut actor_headers = match provider.and_then(|table| table.get("http_headers")) {
        None => vec![("missing".to_string(), "missing".to_string())],
        Some(headers) => match headers.as_table_like() {
            None => vec![("malformed".to_string(), "malformed".to_string())],
            Some(headers) => {
                let mut matches = headers
                    .iter()
                    .filter(|(name, _)| name.eq_ignore_ascii_case(MANAGED_ACTOR_HEADER_NAME))
                    .map(|(name, value)| (name.to_string(), raw_string_projection(Some(value))))
                    .collect::<Vec<_>>();
                if matches.is_empty() {
                    matches.push(("missing".to_string(), "missing".to_string()));
                }
                matches
            }
        },
    };
    actor_headers.sort();
    RawProviderContractProjection {
        provider_id: raw_string_projection(document.get("model_provider")),
        provider_shape,
        provider_name: raw_string_projection(provider.and_then(|table| table.get("name"))),
        wire_api: raw_string_projection(provider.and_then(|table| table.get("wire_api"))),
        requires_openai_auth: raw_bool_projection(
            provider.and_then(|table| table.get("requires_openai_auth")),
        ),
        bearer: raw_string_projection(
            provider.and_then(|table| table.get("experimental_bearer_token")),
        ),
        actor_headers,
    }
}

fn project_raw_provider_fields(profile: &mut RelayProfile, document: &DocumentMut) {
    if let Some(model) = document.get("model").and_then(Item::as_str) {
        profile.model = model.to_string();
    }
    if let Some(provider_id) = document.get("model_provider").and_then(Item::as_str)
        && let Some(provider) = document
            .get("model_providers")
            .and_then(Item::as_table_like)
            .and_then(|providers| providers.get(provider_id))
            .and_then(Item::as_table_like)
        && let Some(base_url) = provider.get("base_url").and_then(Item::as_str)
    {
        profile.base_url = base_url.to_string();
        profile.upstream_base_url = base_url.to_string();
    }
    profile.context_window = document
        .get("model_context_window")
        .and_then(Item::as_integer)
        .map(|value| value.to_string())
        .unwrap_or_default();
    profile.auto_compact_limit = document
        .get("model_auto_compact_token_limit")
        .and_then(Item::as_integer)
        .map(|value| value.to_string())
        .unwrap_or_default();
}

fn validate_raw_provider_config_edit(
    request: &ProviderNativeCapabilityDraftRequest,
    boundary: &dyn ProviderNativeCapabilityDraftReadOnlyBoundary,
) -> ProviderNativeCapabilityDraftPayload {
    let Some(source_config_contents) = request.source_config_contents.as_deref() else {
        return unchanged_draft_payload(
            request,
            boundary,
            NativeCapabilityDraftStatus::Blocked,
            vec![NativeCapabilityReason::RawProviderSourceRequired],
            ProviderNativeCapabilityDraftPreview::default(),
        );
    };
    let source_document = match source_config_contents.parse::<DocumentMut>() {
        Ok(document) => document,
        Err(_) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::RawProviderSourceInvalid],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
    };
    let candidate_document = match request.profile.config_contents.parse::<DocumentMut>() {
        Ok(document) => document,
        Err(_) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::MalformedToml],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
    };
    if raw_provider_contract_projection(&source_document)
        != raw_provider_contract_projection(&candidate_document)
    {
        return unchanged_draft_payload(
            request,
            boundary,
            NativeCapabilityDraftStatus::Blocked,
            vec![NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction],
            ProviderNativeCapabilityDraftPreview::default(),
        );
    }
    let mut profile = request.profile.clone();
    project_raw_provider_fields(&mut profile, &candidate_document);
    ready_draft_payload(
        request,
        boundary,
        profile,
        request.catalog_mode,
        ProviderNativeCapabilityDraftPreview::default(),
    )
}

fn enable_native_priority_draft(
    request: &ProviderNativeCapabilityDraftRequest,
    boundary: &dyn ProviderNativeCapabilityDraftReadOnlyBoundary,
) -> ProviderNativeCapabilityDraftPayload {
    if request.catalog_mode == CatalogMode::External {
        return unchanged_draft_payload(
            request,
            boundary,
            NativeCapabilityDraftStatus::Blocked,
            vec![NativeCapabilityReason::ExternalCatalog],
            ProviderNativeCapabilityDraftPreview::default(),
        );
    }

    let mut profile = request.profile.clone();
    let mut document = match profile.config_contents.parse::<DocumentMut>() {
        Ok(document) => document,
        Err(_) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::MalformedToml],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
    };
    let provider_id = match selected_provider_id(&document) {
        Ok(provider_id) => provider_id,
        Err(reason) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![reason],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
    };
    let provider_id = if LEGACY_PROVIDER_IDS.contains(&provider_id.as_str()) {
        match migrate_legacy_provider_id(
            &mut document,
            &provider_id,
            request.replacement_provider_id.as_deref(),
        ) {
            Ok(provider_id) => provider_id,
            Err(reason) => {
                return unchanged_draft_payload(
                    request,
                    boundary,
                    NativeCapabilityDraftStatus::Blocked,
                    vec![reason],
                    ProviderNativeCapabilityDraftPreview::default(),
                );
            }
        }
    } else {
        if provider_id == RESERVED_PROVIDER_ID {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::ReservedProviderId],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
        provider_id
    };

    let (raw_bearer, actor_header) = match provider_contract_inputs(&document, &provider_id) {
        Ok(inputs) => inputs,
        Err(reason) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![reason],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
    };
    let use_structured =
        has_confirmation(request, NativeCapabilityDraftConfirmation::UseStructuredKey);
    let use_bearer = has_confirmation(
        request,
        NativeCapabilityDraftConfirmation::UseProviderBearer,
    );
    if use_structured && use_bearer {
        return unchanged_draft_payload(
            request,
            boundary,
            NativeCapabilityDraftStatus::Blocked,
            vec![NativeCapabilityReason::ConflictingKeySynchronization],
            ProviderNativeCapabilityDraftPreview::default(),
        );
    }
    let structured_key = profile.api_key.clone();
    let structured_nonblank = !structured_key.trim().is_empty();
    let raw_nonblank = raw_bearer
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let bearer_to_write = match (structured_nonblank, raw_nonblank) {
        (false, false) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::MissingProviderBearer],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
        (false, true) => {
            profile.api_key = raw_bearer
                .as_ref()
                .expect("nonblank raw bearer must be present")
                .clone();
            None
        }
        (true, false) => Some(structured_key),
        (true, true) if raw_bearer.as_deref() == Some(structured_key.as_str()) => None,
        (true, true) if use_structured => Some(structured_key),
        (true, true) if use_bearer => {
            profile.api_key = raw_bearer
                .as_ref()
                .expect("nonblank raw bearer must be present")
                .clone();
            None
        }
        (true, true) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::ConfirmationRequired,
                vec![NativeCapabilityReason::StructuredKeyBearerConflict],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
    };

    match actor_header {
        ActorHeaderDraftState::Conflict(_)
            if !has_confirmation(
                request,
                NativeCapabilityDraftConfirmation::ReplaceActorHeader,
            ) =>
        {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::ConfirmationRequired,
                vec![NativeCapabilityReason::ActorHeaderValueConflict],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
        ActorHeaderDraftState::Duplicate => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::DuplicateActorHeader],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
        ActorHeaderDraftState::Malformed => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::MalformedHeaderStructure],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
        _ => {}
    }

    let provider = document
        .get_mut("model_providers")
        .and_then(Item::as_table_like_mut)
        .and_then(|providers| providers.get_mut(&provider_id))
        .and_then(Item::as_table_like_mut)
        .expect("provider shape was validated before mutation");
    set_string_preserving_decor(provider, "name", MANAGED_PROVIDER_NAME);
    set_string_preserving_decor(provider, "wire_api", MANAGED_WIRE_API);
    set_bool_preserving_decor(provider, "requires_openai_auth", false);
    if let Some(bearer) = bearer_to_write {
        set_string_preserving_decor(provider, "experimental_bearer_token", &bearer);
    }
    set_canonical_actor_header(provider, actor_header);

    profile.relay_mode = RelayMode::Official;
    profile.official_mix_api_key = true;
    profile.protocol = RelayProtocol::Responses;
    profile.config_contents = document.to_string();
    let inspection = boundary.inspect(&profile, CatalogMode::OfficialPlusCustom);
    if inspection.state != NativeCapabilityState::NativePriority {
        let blockers = inspection
            .fields
            .iter()
            .filter(|entry| entry.outcome != NativeCapabilityOutcome::Satisfied)
            .map(|entry| entry.reason)
            .collect();
        return unchanged_draft_payload(
            request,
            boundary,
            NativeCapabilityDraftStatus::Blocked,
            blockers,
            ProviderNativeCapabilityDraftPreview::default(),
        );
    }
    ready_draft_payload(
        request,
        boundary,
        profile,
        CatalogMode::OfficialPlusCustom,
        ProviderNativeCapabilityDraftPreview::default(),
    )
}

fn compatibility_exit_draft(
    request: &ProviderNativeCapabilityDraftRequest,
    boundary: &dyn ProviderNativeCapabilityDraftReadOnlyBoundary,
) -> ProviderNativeCapabilityDraftPayload {
    let preview = ProviderNativeCapabilityDraftPreview {
        capability_loss: true,
        ..ProviderNativeCapabilityDraftPreview::default()
    };
    if !has_confirmation(
        request,
        NativeCapabilityDraftConfirmation::ConfirmCapabilityLoss,
    ) {
        return unchanged_draft_payload(
            request,
            boundary,
            NativeCapabilityDraftStatus::ConfirmationRequired,
            vec![NativeCapabilityReason::CapabilityLossConfirmationRequired],
            preview,
        );
    }

    let mut profile = request.profile.clone();
    let mut document = match profile.config_contents.parse::<DocumentMut>() {
        Ok(document) => document,
        Err(_) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::MalformedToml],
                preview,
            );
        }
    };
    let provider_id = match selected_provider_id(&document) {
        Ok(provider_id) => provider_id,
        Err(reason) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![reason],
                preview,
            );
        }
    };
    let provider = match document
        .get_mut("model_providers")
        .and_then(Item::as_table_like_mut)
        .and_then(|providers| providers.get_mut(&provider_id))
        .and_then(Item::as_table_like_mut)
    {
        Some(provider) => provider,
        None => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::MalformedProviderTable],
                preview,
            );
        }
    };
    match provider_key_resolution(request, &profile, provider) {
        Ok(ProviderKeyResolution::Unchanged) => {}
        Ok(ProviderKeyResolution::UseStructured) => {
            set_string_preserving_decor(provider, "experimental_bearer_token", &profile.api_key)
        }
        Ok(ProviderKeyResolution::UseProvider(raw_bearer)) => {
            profile.api_key = raw_bearer;
        }
        Err((status, reason)) => {
            return unchanged_draft_payload(request, boundary, status, vec![reason], preview);
        }
    }
    if let Err(reason) = remove_manager_actor_header(provider) {
        return unchanged_draft_payload(
            request,
            boundary,
            NativeCapabilityDraftStatus::Blocked,
            vec![reason],
            preview,
        );
    }

    let catalog_mode = if request.catalog_mode == CatalogMode::External {
        CatalogMode::External
    } else {
        match request.action {
            NativeCapabilityDraftAction::ExitPureApi => CatalogMode::CustomOnly,
            _ => CatalogMode::OfficialPlusCustom,
        }
    };
    match request.action {
        NativeCapabilityDraftAction::ExitPureApi => {
            profile.relay_mode = RelayMode::PureApi;
            profile.official_mix_api_key = false;
            set_bool_preserving_decor(provider, "requires_openai_auth", false);
        }
        NativeCapabilityDraftAction::ExitLegacyCompatibility => {
            profile.relay_mode = RelayMode::Official;
            profile.official_mix_api_key = true;
            profile.protocol = RelayProtocol::Responses;
            set_string_preserving_decor(provider, "name", "custom");
            set_string_preserving_decor(provider, "wire_api", MANAGED_WIRE_API);
            set_bool_preserving_decor(provider, "requires_openai_auth", true);
        }
        NativeCapabilityDraftAction::ExitChatCompletions => {
            profile.protocol = RelayProtocol::ChatCompletions;
        }
        _ => unreachable!("only compatibility exit actions reach this helper"),
    }
    profile.config_contents = document.to_string();
    ready_draft_payload(request, boundary, profile, catalog_mode, preview)
}

fn pure_oauth_exit_draft(
    request: &ProviderNativeCapabilityDraftRequest,
    boundary: &dyn ProviderNativeCapabilityDraftReadOnlyBoundary,
) -> ProviderNativeCapabilityDraftPayload {
    let document = match request.profile.config_contents.parse::<DocumentMut>() {
        Ok(document) => document,
        Err(_) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::MalformedToml],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
    };
    let provider_id = match selected_provider_id(&document) {
        Ok(provider_id) => provider_id,
        Err(reason) => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![reason],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
    };
    let provider = match document
        .get("model_providers")
        .and_then(Item::as_table_like)
        .and_then(|providers| providers.get(&provider_id))
        .and_then(Item::as_table_like)
    {
        Some(provider) => provider,
        None => {
            return unchanged_draft_payload(
                request,
                boundary,
                NativeCapabilityDraftStatus::Blocked,
                vec![NativeCapabilityReason::MalformedProviderTable],
                ProviderNativeCapabilityDraftPreview::default(),
            );
        }
    };
    let preview = ProviderNativeCapabilityDraftPreview {
        capability_loss: true,
        removes_provider_table: true,
        removed_provider_id: Some(provider_id.clone()),
        removed_provider_fields: provider.iter().map(|(key, _)| key.to_string()).collect(),
    };
    if let Err((status, reason)) = provider_key_resolution(request, &request.profile, provider) {
        return unchanged_draft_payload(request, boundary, status, vec![reason], preview);
    }
    if !has_confirmation(
        request,
        NativeCapabilityDraftConfirmation::ConfirmDestructivePureOAuth,
    ) {
        return unchanged_draft_payload(
            request,
            boundary,
            NativeCapabilityDraftStatus::ConfirmationRequired,
            vec![NativeCapabilityReason::DestructiveExitConfirmationRequired],
            preview,
        );
    }

    let mut profile = request.profile.clone();
    let mut document = document;
    let remove_container = {
        let providers = document
            .get_mut("model_providers")
            .and_then(Item::as_table_like_mut)
            .expect("provider container was validated");
        providers.remove(&provider_id);
        providers.is_empty()
    };
    if remove_container {
        document.remove("model_providers");
    }
    document.remove("model_provider");
    profile.api_key.clear();
    profile.relay_mode = RelayMode::Official;
    profile.official_mix_api_key = false;
    profile.protocol = RelayProtocol::Responses;
    profile.config_contents = document.to_string();
    let catalog_mode = if request.catalog_mode == CatalogMode::External {
        CatalogMode::External
    } else {
        CatalogMode::NativeOfficial
    };
    ready_draft_payload(request, boundary, profile, catalog_mode, preview)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProviderKeyResolution {
    Unchanged,
    UseStructured,
    UseProvider(String),
}

fn provider_key_resolution(
    request: &ProviderNativeCapabilityDraftRequest,
    profile: &RelayProfile,
    provider: &dyn TableLike,
) -> Result<ProviderKeyResolution, (NativeCapabilityDraftStatus, NativeCapabilityReason)> {
    let raw_bearer = match provider.get("experimental_bearer_token") {
        None => return Ok(ProviderKeyResolution::Unchanged),
        Some(item) => item.as_str().ok_or((
            NativeCapabilityDraftStatus::Blocked,
            NativeCapabilityReason::MalformedProviderBearer,
        ))?,
    };
    let structured_key = profile.api_key.as_str();
    if raw_bearer.trim().is_empty()
        || structured_key.trim().is_empty()
        || raw_bearer == structured_key
    {
        return Ok(ProviderKeyResolution::Unchanged);
    }

    let use_structured =
        has_confirmation(request, NativeCapabilityDraftConfirmation::UseStructuredKey);
    let use_provider = has_confirmation(
        request,
        NativeCapabilityDraftConfirmation::UseProviderBearer,
    );
    match (use_structured, use_provider) {
        (true, true) => Err((
            NativeCapabilityDraftStatus::Blocked,
            NativeCapabilityReason::ConflictingKeySynchronization,
        )),
        (true, false) => Ok(ProviderKeyResolution::UseStructured),
        (false, true) => Ok(ProviderKeyResolution::UseProvider(raw_bearer.to_string())),
        (false, false) => Err((
            NativeCapabilityDraftStatus::ConfirmationRequired,
            NativeCapabilityReason::StructuredKeyBearerConflict,
        )),
    }
}

fn selected_provider_id(document: &DocumentMut) -> Result<String, NativeCapabilityReason> {
    let Some(provider_id) = document.get("model_provider").and_then(Item::as_str) else {
        return Err(NativeCapabilityReason::MissingProviderSelection);
    };
    let provider_id = provider_id.trim();
    if provider_id.is_empty() {
        return Err(NativeCapabilityReason::MissingProviderSelection);
    }
    let Some(providers) = document
        .get("model_providers")
        .and_then(Item::as_table_like)
    else {
        return Err(NativeCapabilityReason::SelectedProviderTableMissing);
    };
    let Some(provider) = providers.get(provider_id) else {
        return Err(NativeCapabilityReason::SelectedProviderTableMissing);
    };
    if provider.as_table_like().is_none() {
        return Err(NativeCapabilityReason::MalformedProviderTable);
    }
    Ok(provider_id.to_string())
}

fn migrate_legacy_provider_id(
    document: &mut DocumentMut,
    provider_id: &str,
    replacement_provider_id: Option<&str>,
) -> Result<String, NativeCapabilityReason> {
    let legacy_item = document
        .get("model_providers")
        .and_then(Item::as_table_like)
        .and_then(|providers| providers.get(provider_id))
        .cloned()
        .ok_or(NativeCapabilityReason::SelectedProviderTableMissing)?;
    let custom_item = document
        .get("model_providers")
        .and_then(Item::as_table_like)
        .and_then(|providers| providers.get("custom"))
        .cloned();
    let target = match custom_item.as_ref() {
        None => "custom".to_string(),
        Some(custom) if items_semantically_equal(&legacy_item, custom) => "custom".to_string(),
        Some(_) => {
            let replacement = replacement_provider_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or(NativeCapabilityReason::ReplacementProviderIdRequired)?;
            if replacement == RESERVED_PROVIDER_ID || LEGACY_PROVIDER_IDS.contains(&replacement) {
                return Err(NativeCapabilityReason::ReplacementProviderIdInvalid);
            }
            if document
                .get("model_providers")
                .and_then(Item::as_table_like)
                .is_some_and(|providers| providers.get(replacement).is_some())
            {
                return Err(NativeCapabilityReason::ReplacementProviderIdUnavailable);
            }
            replacement.to_string()
        }
    };

    {
        let providers = document
            .get_mut("model_providers")
            .and_then(Item::as_table_like_mut)
            .expect("provider container was validated");
        let moved = providers
            .remove(provider_id)
            .expect("selected legacy provider was validated");
        if providers.get(&target).is_none() {
            providers.insert(&target, moved);
        }
    }
    set_string_preserving_decor(document.as_table_mut(), "model_provider", &target);
    Ok(target)
}

fn items_semantically_equal(left: &Item, right: &Item) -> bool {
    fn normalized(item: &Item) -> Option<serde_json::Value> {
        let mut document = DocumentMut::new();
        document.as_table_mut().insert("candidate", item.clone());
        toml_edit::de::from_str(&document.to_string()).ok()
    }
    normalized(left) == normalized(right)
}

#[derive(Debug, Clone)]
enum ActorHeaderDraftState {
    Missing,
    Canonical(String),
    Conflict(String),
    Duplicate,
    Malformed,
}

fn provider_contract_inputs(
    document: &DocumentMut,
    provider_id: &str,
) -> Result<(Option<String>, ActorHeaderDraftState), NativeCapabilityReason> {
    let provider = document
        .get("model_providers")
        .and_then(Item::as_table_like)
        .and_then(|providers| providers.get(provider_id))
        .and_then(Item::as_table_like)
        .ok_or(NativeCapabilityReason::MalformedProviderTable)?;
    let raw_bearer = match provider.get("experimental_bearer_token") {
        None => None,
        Some(item) => Some(
            item.as_str()
                .ok_or(NativeCapabilityReason::MalformedProviderBearer)?
                .to_string(),
        ),
    };
    let actor_header = match provider.get("http_headers") {
        None => ActorHeaderDraftState::Missing,
        Some(item) => {
            let Some(headers) = item.as_table_like() else {
                return Ok((raw_bearer, ActorHeaderDraftState::Malformed));
            };
            let matches = headers
                .iter()
                .filter(|(name, _)| name.eq_ignore_ascii_case(MANAGED_ACTOR_HEADER_NAME))
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [] => ActorHeaderDraftState::Missing,
                [(_, _), _, ..] => ActorHeaderDraftState::Duplicate,
                [(name, item)] => match item.as_str() {
                    Some(MANAGED_ACTOR_HEADER_VALUE) => {
                        ActorHeaderDraftState::Canonical((*name).to_string())
                    }
                    Some(_) => ActorHeaderDraftState::Conflict((*name).to_string()),
                    None => ActorHeaderDraftState::Malformed,
                },
            }
        }
    };
    Ok((raw_bearer, actor_header))
}

fn set_canonical_actor_header(provider: &mut dyn TableLike, state: ActorHeaderDraftState) {
    if provider.get("http_headers").is_none() {
        let mut headers = InlineTable::new();
        headers.insert(
            MANAGED_ACTOR_HEADER_NAME,
            Value::from(MANAGED_ACTOR_HEADER_VALUE),
        );
        provider.insert("http_headers", Item::Value(Value::InlineTable(headers)));
        return;
    }
    let headers = provider
        .get_mut("http_headers")
        .and_then(Item::as_table_like_mut)
        .expect("header shape was validated before mutation");
    match state {
        ActorHeaderDraftState::Canonical(name) | ActorHeaderDraftState::Conflict(name) => {
            if name == MANAGED_ACTOR_HEADER_NAME {
                set_string_preserving_decor(
                    headers,
                    MANAGED_ACTOR_HEADER_NAME,
                    MANAGED_ACTOR_HEADER_VALUE,
                );
            } else {
                headers.remove(&name);
                headers.insert(MANAGED_ACTOR_HEADER_NAME, value(MANAGED_ACTOR_HEADER_VALUE));
            }
        }
        ActorHeaderDraftState::Missing => {
            headers.insert(MANAGED_ACTOR_HEADER_NAME, value(MANAGED_ACTOR_HEADER_VALUE));
        }
        ActorHeaderDraftState::Duplicate | ActorHeaderDraftState::Malformed => {
            unreachable!("ambiguous headers are rejected before mutation")
        }
    }
}

fn remove_manager_actor_header(provider: &mut dyn TableLike) -> Result<(), NativeCapabilityReason> {
    let Some(headers_item) = provider.get_mut("http_headers") else {
        return Ok(());
    };
    let Some(headers) = headers_item.as_table_like_mut() else {
        return Err(NativeCapabilityReason::MalformedHeaderStructure);
    };
    let matches = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case(MANAGED_ACTOR_HEADER_NAME))
        .map(|(name, value)| (name.to_string(), value.as_str().map(str::to_string)))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(NativeCapabilityReason::DuplicateActorHeader);
    }
    if matches
        .first()
        .is_some_and(|(_, header_value)| header_value.is_none())
    {
        return Err(NativeCapabilityReason::MalformedHeaderStructure);
    }
    if let Some((name, Some(header_value))) = matches.first()
        && name == MANAGED_ACTOR_HEADER_NAME
        && header_value == MANAGED_ACTOR_HEADER_VALUE
    {
        headers.remove(name);
    }
    let remove_container = headers.is_empty();
    if remove_container {
        provider.remove("http_headers");
    }
    Ok(())
}

fn set_string_preserving_decor(table: &mut dyn TableLike, key: &str, new_value: &str) {
    set_value_preserving_decor(table, key, Value::from(new_value));
}

fn set_bool_preserving_decor(table: &mut dyn TableLike, key: &str, new_value: bool) {
    set_value_preserving_decor(table, key, Value::from(new_value));
}

fn set_value_preserving_decor(table: &mut dyn TableLike, key: &str, mut new_value: Value) {
    if let Some(decor) = table
        .get(key)
        .and_then(Item::as_value)
        .map(|value| value.decor().clone())
    {
        *new_value.decor_mut() = decor;
    }
    table.insert(key, Item::Value(new_value));
}

fn has_confirmation(
    request: &ProviderNativeCapabilityDraftRequest,
    confirmation: NativeCapabilityDraftConfirmation,
) -> bool {
    request.confirmations.contains(&confirmation)
}

fn unchanged_draft_payload(
    request: &ProviderNativeCapabilityDraftRequest,
    boundary: &dyn ProviderNativeCapabilityDraftReadOnlyBoundary,
    status: NativeCapabilityDraftStatus,
    blockers: Vec<NativeCapabilityReason>,
    preview: ProviderNativeCapabilityDraftPreview,
) -> ProviderNativeCapabilityDraftPayload {
    let mut profile = request.profile.clone();
    profile.auth_contents.clear();
    ProviderNativeCapabilityDraftPayload {
        draft_revision: request.draft_revision,
        status,
        draft: ProviderNativeCapabilityDraft {
            structured_api_key: profile.api_key.clone(),
            profile: profile.clone(),
            catalog_mode: request.catalog_mode,
        },
        inspection: boundary.inspect(&profile, request.catalog_mode),
        blockers,
        preview,
    }
}

fn ready_draft_payload(
    request: &ProviderNativeCapabilityDraftRequest,
    boundary: &dyn ProviderNativeCapabilityDraftReadOnlyBoundary,
    mut profile: RelayProfile,
    catalog_mode: CatalogMode,
    preview: ProviderNativeCapabilityDraftPreview,
) -> ProviderNativeCapabilityDraftPayload {
    profile.auth_contents.clear();
    ProviderNativeCapabilityDraftPayload {
        draft_revision: request.draft_revision,
        status: NativeCapabilityDraftStatus::Ready,
        draft: ProviderNativeCapabilityDraft {
            structured_api_key: profile.api_key.clone(),
            profile: profile.clone(),
            catalog_mode,
        },
        inspection: boundary.inspect(&profile, catalog_mode),
        blockers: Vec::new(),
        preview,
    }
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
