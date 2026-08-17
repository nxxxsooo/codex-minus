use std::collections::BTreeMap;

use codex_minus_lib::provider_native_capability::{
    CatalogMode, NativeCapabilityField, NativeCapabilityOutcome, NativeCapabilityReason,
    NativeCapabilityState, ProviderNativeCapabilityInspectionError,
    ProviderNativeCapabilityInspectionRequest, inspect_profile, inspect_profiles,
    inspect_provider_native_capabilities_from_paths,
};
use codex_plus_core::settings::{BackendSettings, RelayMode, RelayProfile, RelayProtocol};

const CANONICAL_INLINE: &str =
    include_str!("fixtures/provider-native-capability/canonical-inline.toml");
const CANONICAL_HEADER_TABLE: &str =
    include_str!("fixtures/provider-native-capability/canonical-header-table.toml");
const LEGACY_MIXED: &str = include_str!("fixtures/provider-native-capability/legacy-mixed.toml");
const PARTIAL: &str = include_str!("fixtures/provider-native-capability/partial.toml");
const CONFLICTING_ACTOR_HEADER: &str =
    include_str!("fixtures/provider-native-capability/conflicting-actor-header.toml");
const MISSING_INPUT: &str = include_str!("fixtures/provider-native-capability/missing-input.toml");
const MALFORMED: &str = include_str!("fixtures/provider-native-capability/malformed.toml");
const RESERVED_OPENAI: &str =
    include_str!("fixtures/provider-native-capability/reserved-openai.toml");
const CUSTOM_OPENAI_CASE: &str =
    include_str!("fixtures/provider-native-capability/custom-openai-case.toml");
const LEGACY_CODEX_PLUS_PLUS: &str =
    include_str!("fixtures/provider-native-capability/legacy-codex-plus-plus.toml");
const LEGACY_CODEX_PP: &str =
    include_str!("fixtures/provider-native-capability/legacy-codex-pp.toml");
const DUPLICATE_SEMANTIC_HEADER: &str =
    include_str!("fixtures/provider-native-capability/duplicate-semantic-header.toml");
const MALFORMED_HEADER_SHAPE: &str =
    include_str!("fixtures/provider-native-capability/malformed-header-shape.toml");
const WRONG_CASE_HEADER: &str =
    include_str!("fixtures/provider-native-capability/wrong-case-header.toml");

fn mixed_profile(id: &str, config_contents: &str) -> RelayProfile {
    RelayProfile {
        id: id.to_string(),
        name: id.to_string(),
        protocol: RelayProtocol::Responses,
        relay_mode: RelayMode::Official,
        official_mix_api_key: true,
        config_contents: config_contents.to_string(),
        ..RelayProfile::default()
    }
}

fn reason(
    inspection: &codex_minus_lib::provider_native_capability::ProviderNativeCapabilityInspection,
    field: NativeCapabilityField,
) -> (NativeCapabilityOutcome, NativeCapabilityReason) {
    let entry = inspection
        .fields
        .iter()
        .find(|entry| entry.field == field)
        .unwrap_or_else(|| panic!("missing field result: {field:?}"));
    (entry.outcome, entry.reason)
}

#[test]
fn classifies_the_binding_fixture_matrix_from_profile_catalog_and_toml() {
    let mut pure_oauth = mixed_profile("pure-oauth", "");
    pure_oauth.official_mix_api_key = false;
    let mut pure_api = mixed_profile("pure-api", CANONICAL_INLINE);
    pure_api.relay_mode = RelayMode::PureApi;
    let mut chat_completions = mixed_profile("chat-completions", CANONICAL_INLINE);
    chat_completions.protocol = RelayProtocol::ChatCompletions;
    let cases = [
        (
            "canonical native-priority",
            mixed_profile("canonical", CANONICAL_INLINE),
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::NativePriority,
        ),
        (
            "eligible legacy mixed",
            mixed_profile("legacy", LEGACY_MIXED),
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::UpgradeAvailable,
        ),
        (
            "partial",
            mixed_profile("partial", PARTIAL),
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::UpgradeAvailable,
        ),
        (
            "conflicting actor header",
            mixed_profile("conflicting-header", CONFLICTING_ACTOR_HEADER),
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::Degraded,
        ),
        (
            "external mixed",
            mixed_profile("external-mixed", CANONICAL_INLINE),
            CatalogMode::External,
            NativeCapabilityState::NotApplicable,
        ),
        (
            "external pure OAuth",
            pure_oauth.clone(),
            CatalogMode::External,
            NativeCapabilityState::NotApplicable,
        ),
        (
            "ordinary pure OAuth",
            pure_oauth,
            CatalogMode::NativeOfficial,
            NativeCapabilityState::NotApplicable,
        ),
        (
            "pure API",
            pure_api,
            CatalogMode::CustomOnly,
            NativeCapabilityState::Compatibility,
        ),
        (
            "Chat Completions",
            chat_completions,
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::Compatibility,
        ),
        (
            "missing input",
            mixed_profile("missing", MISSING_INPUT),
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::Degraded,
        ),
        (
            "malformed TOML",
            mixed_profile("malformed", MALFORMED),
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::Degraded,
        ),
        (
            "reserved lowercase openai",
            mixed_profile("reserved", RESERVED_OPENAI),
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::Degraded,
        ),
        (
            "case-sensitive custom OpenAI ID",
            mixed_profile("custom-openai", CUSTOM_OPENAI_CASE),
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::NativePriority,
        ),
        (
            "CodexPlusPlus legacy alias",
            mixed_profile("alias-long", LEGACY_CODEX_PLUS_PLUS),
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::UpgradeAvailable,
        ),
        (
            "CodexPP legacy alias",
            mixed_profile("alias-short", LEGACY_CODEX_PP),
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityState::UpgradeAvailable,
        ),
    ];

    for (label, profile, catalog_mode, expected) in cases {
        assert_eq!(
            inspect_profile(&profile, catalog_mode).state,
            expected,
            "{label}"
        );
    }
}

#[test]
fn persisted_inspection_rejects_aggregate_settings_before_catalog_presentation() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let catalog_path = temp.path().join("model-catalog-state.json");
    let mut aggregate_profile = mixed_profile("aggregate", CANONICAL_INLINE);
    aggregate_profile.relay_mode = RelayMode::Aggregate;
    let aggregate_settings = BackendSettings {
        relay_profiles: vec![aggregate_profile],
        active_relay_id: "aggregate".to_string(),
        ..BackendSettings::default()
    };
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&aggregate_settings).unwrap(),
    )
    .unwrap();

    assert_eq!(
        inspect_provider_native_capabilities_from_paths(
            &settings_path,
            &catalog_path,
            ProviderNativeCapabilityInspectionRequest::default(),
        ),
        Err(ProviderNativeCapabilityInspectionError::InputUnavailable),
    );

    let mut aggregate_metadata = BackendSettings::default();
    aggregate_metadata.active_aggregate_relay_id = "removed-aggregate".to_string();
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&aggregate_metadata).unwrap(),
    )
    .unwrap();
    assert_eq!(
        inspect_provider_native_capabilities_from_paths(
            &settings_path,
            &catalog_path,
            ProviderNativeCapabilityInspectionRequest::default(),
        ),
        Err(ProviderNativeCapabilityInspectionError::InputUnavailable),
    );
}

#[test]
fn structured_key_is_optional_but_a_different_nonblank_key_conflicts() {
    let without_structured_key = mixed_profile("without-structured", CANONICAL_INLINE);
    assert_eq!(
        inspect_profile(&without_structured_key, CatalogMode::OfficialPlusCustom).state,
        NativeCapabilityState::NativePriority
    );

    let mut matching = mixed_profile("matching", CANONICAL_INLINE);
    matching.api_key = "secret-canonical-inline".to_string();
    assert_eq!(
        inspect_profile(&matching, CatalogMode::OfficialPlusCustom).state,
        NativeCapabilityState::NativePriority
    );

    let mut conflicting = mixed_profile("conflicting", CANONICAL_INLINE);
    conflicting.api_key = "different-structured-secret".to_string();
    let inspection = inspect_profile(&conflicting, CatalogMode::OfficialPlusCustom);
    assert_eq!(inspection.state, NativeCapabilityState::Degraded);
    assert_eq!(
        reason(&inspection, NativeCapabilityField::ProviderBearer),
        (
            NativeCapabilityOutcome::Conflict,
            NativeCapabilityReason::StructuredKeyBearerConflict,
        )
    );
}

#[test]
fn inline_and_header_table_forms_are_read_without_mutating_unrelated_fields() {
    for config in [CANONICAL_INLINE, CANONICAL_HEADER_TABLE] {
        let profile = mixed_profile("headers", config);
        let before = profile.config_contents.clone();
        let inspection = inspect_profile(&profile, CatalogMode::OfficialPlusCustom);
        assert_eq!(inspection.state, NativeCapabilityState::NativePriority);
        assert_eq!(profile.config_contents, before);
        assert!(profile.config_contents.contains("x-unrelated-header"));
    }
}

#[test]
fn actor_header_uses_http_lookup_but_requires_one_exact_canonical_entry() {
    let duplicate = inspect_profile(
        &mixed_profile("duplicate", DUPLICATE_SEMANTIC_HEADER),
        CatalogMode::OfficialPlusCustom,
    );
    assert_eq!(duplicate.state, NativeCapabilityState::Degraded);
    assert_eq!(
        reason(&duplicate, NativeCapabilityField::ActorHeader),
        (
            NativeCapabilityOutcome::Conflict,
            NativeCapabilityReason::DuplicateActorHeader,
        )
    );

    let malformed = inspect_profile(
        &mixed_profile("malformed-shape", MALFORMED_HEADER_SHAPE),
        CatalogMode::OfficialPlusCustom,
    );
    assert_eq!(malformed.state, NativeCapabilityState::Degraded);
    assert_eq!(
        reason(&malformed, NativeCapabilityField::ActorHeader),
        (
            NativeCapabilityOutcome::Malformed,
            NativeCapabilityReason::MalformedHeaderStructure,
        )
    );

    let wrong_case = inspect_profile(
        &mixed_profile("wrong-case", WRONG_CASE_HEADER),
        CatalogMode::OfficialPlusCustom,
    );
    // The transform rewrites the actor header, so the action stays reachable; the frontend
    // still routes an actor-header-only conflict to its explicit confirmation path first.
    assert_eq!(wrong_case.state, NativeCapabilityState::UpgradeAvailable);
    assert_eq!(
        reason(&wrong_case, NativeCapabilityField::ActorHeader),
        (
            NativeCapabilityOutcome::Mismatch,
            NativeCapabilityReason::ActorHeaderNameMismatch,
        )
    );
}

#[test]
fn a_managed_header_alone_never_proves_the_complete_contract() {
    let profile = mixed_profile(
        "header-alone",
        r#"model_provider = "custom"
[model_providers.custom]
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
"#,
    );
    let inspection = inspect_profile(&profile, CatalogMode::OfficialPlusCustom);
    assert_eq!(inspection.state, NativeCapabilityState::Degraded);
    assert_eq!(
        reason(&inspection, NativeCapabilityField::BaseUrl).0,
        NativeCapabilityOutcome::Missing
    );
    assert_eq!(
        reason(&inspection, NativeCapabilityField::Model).0,
        NativeCapabilityOutcome::Missing
    );
    assert_eq!(
        reason(&inspection, NativeCapabilityField::ProviderBearer).0,
        NativeCapabilityOutcome::Missing
    );
}

#[test]
fn provider_ids_are_exact_reserved_and_legacy_aware() {
    let case_mismatch = mixed_profile(
        "case-mismatch",
        r#"model = "gpt-5"
model_provider = "CustomProvider"
[model_providers.customprovider]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "case-sensitive-secret"
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
"#,
    );
    let inspection = inspect_profile(&case_mismatch, CatalogMode::OfficialPlusCustom);
    assert_eq!(inspection.state, NativeCapabilityState::Degraded);
    assert_eq!(
        reason(&inspection, NativeCapabilityField::ProviderSelection),
        (
            NativeCapabilityOutcome::Missing,
            NativeCapabilityReason::SelectedProviderTableMissing,
        )
    );

    for config in [LEGACY_CODEX_PLUS_PLUS, LEGACY_CODEX_PP] {
        let inspection = inspect_profile(
            &mixed_profile("legacy-alias", config),
            CatalogMode::OfficialPlusCustom,
        );
        assert_eq!(
            reason(&inspection, NativeCapabilityField::ProviderSelection),
            (
                NativeCapabilityOutcome::Mismatch,
                NativeCapabilityReason::LegacyProviderIdRequiresRename,
            )
        );
    }

    let mut chat_legacy = mixed_profile("chat-legacy", LEGACY_CODEX_PLUS_PLUS);
    chat_legacy.protocol = RelayProtocol::ChatCompletions;
    let inspection = inspect_profile(&chat_legacy, CatalogMode::OfficialPlusCustom);
    assert_eq!(inspection.state, NativeCapabilityState::Degraded);
    assert_eq!(
        reason(&inspection, NativeCapabilityField::ProviderSelection),
        (
            NativeCapabilityOutcome::Mismatch,
            NativeCapabilityReason::LegacyProviderIdRequiresRename,
        )
    );
}

#[test]
fn public_enums_are_sanitized_camel_case_and_field_order_is_stable() {
    let state_json = serde_json::to_value([
        NativeCapabilityState::NativePriority,
        NativeCapabilityState::UpgradeAvailable,
        NativeCapabilityState::Degraded,
        NativeCapabilityState::Compatibility,
        NativeCapabilityState::NotApplicable,
    ])
    .unwrap();
    assert_eq!(
        state_json,
        serde_json::json!([
            "nativePriority",
            "upgradeAvailable",
            "degraded",
            "compatibility",
            "notApplicable"
        ])
    );

    let inspection = inspect_profile(
        &mixed_profile("ordered", MISSING_INPUT),
        CatalogMode::OfficialPlusCustom,
    );
    assert_eq!(
        inspection
            .fields
            .iter()
            .map(|entry| entry.field)
            .collect::<Vec<_>>(),
        vec![
            NativeCapabilityField::RelayMode,
            NativeCapabilityField::Protocol,
            NativeCapabilityField::Catalog,
            NativeCapabilityField::ProviderSelection,
            NativeCapabilityField::BaseUrl,
            NativeCapabilityField::Model,
            NativeCapabilityField::ProviderName,
            NativeCapabilityField::WireApi,
            NativeCapabilityField::RequiresOpenAiAuth,
            NativeCapabilityField::ProviderBearer,
            NativeCapabilityField::ActorHeader,
        ]
    );
}

#[test]
fn classification_metadata_is_derived_and_never_serialized_into_settings() {
    let profile = mixed_profile("derived", CANONICAL_INLINE);
    let settings = BackendSettings {
        relay_profiles: vec![profile.clone()],
        ..BackendSettings::default()
    };
    let settings_json = serde_json::to_string(&settings).unwrap();
    assert!(!settings_json.contains("nativeCapability"));
    assert!(!settings_json.contains("upgradeAvailable"));

    assert_eq!(
        inspect_profile(&profile, CatalogMode::OfficialPlusCustom).state,
        NativeCapabilityState::NativePriority
    );
}

#[test]
fn bulk_and_per_profile_inspection_are_exact_sanitized_and_secret_free() {
    let mut canonical = mixed_profile("CaseSensitive", CANONICAL_INLINE);
    canonical.auth_contents =
        r#"{"tokens":{"access_token":"oauth-secret"},"account_id":"account-secret"}"#.to_string();
    canonical.api_key = "secret-canonical-inline".to_string();
    let other = mixed_profile("other", LEGACY_MIXED);
    let settings = BackendSettings {
        relay_profiles: vec![canonical, other],
        ..BackendSettings::default()
    };
    let modes = BTreeMap::from([
        ("CaseSensitive".to_string(), CatalogMode::OfficialPlusCustom),
        ("other".to_string(), CatalogMode::OfficialPlusCustom),
    ]);

    let bulk = inspect_profiles(&settings, &modes, None).unwrap();
    assert_eq!(
        bulk.iter()
            .map(|item| item.profile_id.as_str())
            .collect::<Vec<_>>(),
        vec!["CaseSensitive", "other"]
    );
    let one = inspect_profiles(&settings, &modes, Some("CaseSensitive")).unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].profile_id, "CaseSensitive");
    assert!(inspect_profiles(&settings, &modes, Some("casesensitive")).is_err());

    let serialized = serde_json::to_string(&bulk).unwrap();
    for forbidden in [
        "secret-canonical-inline",
        "secret-legacy",
        "oauth-secret",
        "account-secret",
        "authContents",
        "account_id",
        "access_token",
        "must-never-escape",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }

    assert_eq!(
        serde_json::to_value(ProviderNativeCapabilityInspectionRequest::default()).unwrap(),
        serde_json::json!({"profileId": null})
    );
}

#[test]
fn external_ownership_short_circuits_before_toml_parsing() {
    let inspection = inspect_profile(
        &mixed_profile("external-malformed", MALFORMED),
        CatalogMode::External,
    );
    assert_eq!(inspection.state, NativeCapabilityState::NotApplicable);
    assert_eq!(
        inspection.fields,
        vec![
            codex_minus_lib::provider_native_capability::NativeCapabilityFieldResult {
                field: NativeCapabilityField::Catalog,
                outcome: NativeCapabilityOutcome::NotApplicable,
                reason: NativeCapabilityReason::ExternalCatalog,
            }
        ]
    );
}

#[test]
fn malformed_toml_reports_only_a_typed_reason_without_source_text() {
    let inspection = inspect_profile(
        &mixed_profile("malformed-secret", MALFORMED),
        CatalogMode::OfficialPlusCustom,
    );
    assert_eq!(inspection.state, NativeCapabilityState::Degraded);
    let json = serde_json::to_string(&inspection).unwrap();
    assert!(json.contains("malformedToml"));
    assert!(!json.contains("secret_parser_line"));
    assert!(!json.contains("must-never-escape"));
    assert!(!json.contains("secret-malformed"));
}

#[test]
fn command_loader_reads_bulk_or_one_profile_without_modifying_either_store() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let catalog_path = temp.path().join("model-catalog-state.json");
    let mut profile = mixed_profile("CaseSensitive", CANONICAL_INLINE);
    profile.auth_contents = "oauth-sentinel".to_string();
    let settings = BackendSettings {
        relay_profiles: vec![profile],
        ..BackendSettings::default()
    };
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&settings).unwrap(),
    )
    .unwrap();
    std::fs::write(
        &catalog_path,
        br#"{"profiles":{"CaseSensitive":{"mode":"official-plus-custom"}}}"#,
    )
    .unwrap();
    let settings_before = std::fs::read(&settings_path).unwrap();
    let catalog_before = std::fs::read(&catalog_path).unwrap();
    let entries_before = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();

    let bulk = inspect_provider_native_capabilities_from_paths(
        &settings_path,
        &catalog_path,
        ProviderNativeCapabilityInspectionRequest::default(),
    )
    .unwrap();
    assert_eq!(bulk.inspections.len(), 1);
    let one = inspect_provider_native_capabilities_from_paths(
        &settings_path,
        &catalog_path,
        ProviderNativeCapabilityInspectionRequest {
            profile_id: Some("CaseSensitive".to_string()),
        },
    )
    .unwrap();
    assert_eq!(
        one.inspections[0].state,
        NativeCapabilityState::NativePriority
    );
    assert!(
        inspect_provider_native_capabilities_from_paths(
            &settings_path,
            &catalog_path,
            ProviderNativeCapabilityInspectionRequest {
                profile_id: Some("casesensitive".to_string()),
            },
        )
        .is_err()
    );

    assert_eq!(std::fs::read(&settings_path).unwrap(), settings_before);
    assert_eq!(std::fs::read(&catalog_path).unwrap(), catalog_before);
    assert_eq!(
        std::fs::read_dir(temp.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>(),
        entries_before
    );
    assert!(
        !serde_json::to_string(&one)
            .unwrap()
            .contains("oauth-sentinel")
    );
}

#[test]
fn command_loader_preserves_raw_persisted_evidence_before_evaluation() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let catalog_path = temp.path().join("missing-catalog-state.json");
    let raw_settings = serde_json::json!({
        "relayProfiles": [
            {
                "id": "reserved",
                "name": "reserved",
                "protocol": "responses",
                "relayMode": "official",
                "officialMixApiKey": true,
                "configContents": RESERVED_OPENAI,
                "authContents": "oauth-reserved-sentinel"
            },
            {
                "id": "CodexPlusPlus-profile",
                "name": "alias long",
                "protocol": "responses",
                "relayMode": "official",
                "officialMixApiKey": true,
                "configContents": LEGACY_CODEX_PLUS_PLUS
            },
            {
                "id": "CodexPP-profile",
                "name": "alias short",
                "protocol": "responses",
                "relayMode": "official",
                "officialMixApiKey": true,
                "configContents": LEGACY_CODEX_PP
            },
            {
                "id": "key-conflict",
                "name": "key conflict",
                "apiKey": "different-structured-secret",
                "protocol": "responses",
                "relayMode": "official",
                "officialMixApiKey": true,
                "configContents": CANONICAL_INLINE
            },
            {
                "id": "missing-field",
                "name": "missing field",
                "protocol": "responses",
                "relayMode": "official",
                "officialMixApiKey": true,
                "configContents": PARTIAL
            }
        ]
    });
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&raw_settings).unwrap(),
    )
    .unwrap();
    let before = std::fs::read(&settings_path).unwrap();

    let payload = inspect_provider_native_capabilities_from_paths(
        &settings_path,
        &catalog_path,
        ProviderNativeCapabilityInspectionRequest::default(),
    )
    .unwrap();
    assert_eq!(
        payload
            .inspections
            .iter()
            .map(|item| (item.profile_id.as_str(), item.state))
            .collect::<Vec<_>>(),
        vec![
            ("reserved", NativeCapabilityState::Degraded),
            (
                "CodexPlusPlus-profile",
                NativeCapabilityState::UpgradeAvailable,
            ),
            ("CodexPP-profile", NativeCapabilityState::UpgradeAvailable),
            ("key-conflict", NativeCapabilityState::Degraded),
            ("missing-field", NativeCapabilityState::UpgradeAvailable),
        ]
    );
    assert_eq!(
        reason(
            &payload.inspections[0],
            NativeCapabilityField::ProviderSelection
        ),
        (
            NativeCapabilityOutcome::Conflict,
            NativeCapabilityReason::ReservedProviderId,
        )
    );
    for inspection in &payload.inspections[1..=2] {
        assert_eq!(
            reason(inspection, NativeCapabilityField::ProviderSelection),
            (
                NativeCapabilityOutcome::Mismatch,
                NativeCapabilityReason::LegacyProviderIdRequiresRename,
            )
        );
    }
    assert_eq!(
        reason(
            &payload.inspections[3],
            NativeCapabilityField::ProviderBearer,
        ),
        (
            NativeCapabilityOutcome::Conflict,
            NativeCapabilityReason::StructuredKeyBearerConflict,
        )
    );
    assert_eq!(
        reason(&payload.inspections[4], NativeCapabilityField::ActorHeader),
        (
            NativeCapabilityOutcome::Missing,
            NativeCapabilityReason::MissingActorHeader,
        )
    );

    let serialized = serde_json::to_string(&payload).unwrap();
    for forbidden in [
        "secret-reserved",
        "secret-alias-long",
        "secret-alias-short",
        "secret-canonical-inline",
        "different-structured-secret",
        "secret-partial",
        "oauth-reserved-sentinel",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }
    assert_eq!(std::fs::read(&settings_path).unwrap(), before);
    assert!(!catalog_path.exists());
}

#[test]
fn command_loader_rejects_malformed_settings_with_one_sanitized_typed_error() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let catalog_path = temp.path().join("missing-catalog-state.json");
    let malformed = br#"{"relayProfiles":[{"configContents":"secret-settings-parser"}"#;
    std::fs::write(&settings_path, malformed).unwrap();
    let before = std::fs::read(&settings_path).unwrap();

    let error = inspect_provider_native_capabilities_from_paths(
        &settings_path,
        &catalog_path,
        ProviderNativeCapabilityInspectionRequest::default(),
    )
    .unwrap_err();
    assert_eq!(
        error,
        ProviderNativeCapabilityInspectionError::InputUnavailable
    );
    let serialized = serde_json::to_string(&error).unwrap();
    assert_eq!(serialized, r#""inputUnavailable""#);
    assert!(!serialized.contains("secret-settings-parser"));
    assert_eq!(std::fs::read(&settings_path).unwrap(), before);
    assert!(!catalog_path.exists());
}

const NON_LEGACY_REPAIRABLE: &str =
    include_str!("fixtures/provider-native-capability/non-legacy-repairable.toml");
const REPAIRABLE_WITHOUT_MODEL: &str =
    include_str!("fixtures/provider-native-capability/repairable-without-model.toml");

#[test]
fn upgrade_is_offered_whenever_the_transform_writes_every_remaining_gap() {
    // Not the classic `custom` shape: a different identifier and provider name, no bearer, no
    // actor header, and official auth still required. Every one of those is a field the upgrade
    // transform writes, so the profile must be able to reach its own upgrade.
    let profile = mixed_profile("relay-one", NON_LEGACY_REPAIRABLE);
    let inspection = inspect_profile(&profile, CatalogMode::OfficialPlusCustom);

    assert_eq!(inspection.state, NativeCapabilityState::UpgradeAvailable);
    assert!(
        inspection
            .fields
            .iter()
            .any(|field| field.outcome != NativeCapabilityOutcome::Satisfied),
        "the contract is still incomplete; only its reachability changed"
    );
}

#[test]
fn upgrade_is_withheld_when_a_gap_the_transform_cannot_fill_remains() {
    // Same profile without a default model. The transform never invents a model, so the action
    // stays withheld and the missing input is reported instead.
    let profile = mixed_profile("relay-one", REPAIRABLE_WITHOUT_MODEL);
    let inspection = inspect_profile(&profile, CatalogMode::OfficialPlusCustom);

    assert_eq!(inspection.state, NativeCapabilityState::Degraded);
    assert!(inspection.fields.iter().any(|field| {
        field.field == NativeCapabilityField::Model
            && field.reason == NativeCapabilityReason::MissingModel
    }));
}

#[test]
fn the_classic_legacy_contract_still_reaches_its_upgrade() {
    let profile = mixed_profile("relay-one", LEGACY_MIXED);

    assert_eq!(
        inspect_profile(&profile, CatalogMode::OfficialPlusCustom).state,
        NativeCapabilityState::UpgradeAvailable
    );
}
