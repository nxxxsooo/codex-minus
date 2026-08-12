use codex_minus_lib::provider_capability_evidence::{
    ActorMarkerEvidence, CatalogModelEvidence, FreePlanRule, ImageCapabilityPath,
    ImageGenerationEvidence, ImagePlanEvidence, ImagePolicySource, LocalPlanEvidence,
    OAuthSessionEvidence, ProviderCapabilityEvidenceRequest, ProviderContractEvidence,
    ProviderRouteEvidence, RuntimeEvidence, TextResponsesEvidence,
    inspect_provider_capability_evidence_command_from_paths,
    inspect_provider_capability_evidence_from_paths,
};
use codex_plus_core::settings::{BackendSettings, RelayMode, RelayProfile, RelayProtocol};

const CONFIG: &str = r#"model = "gpt-5.5"
model_provider = "RelayOne"

[model_providers.RelayOne]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "provider-key-sentinel"
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
"#;

#[test]
fn trusted_read_only_evidence_is_redacted_and_never_invents_target_policy() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let catalog_path = temp.path().join("model-catalog-state.json");
    let codex_home = temp.path().join("codex-home");
    std::fs::create_dir(&codex_home).unwrap();
    let auth_path = codex_home.join("auth.json");
    std::fs::write(
        &auth_path,
        br#"{"auth_mode":"chatgpt","tokens":{"refresh_token":"oauth-refresh-sentinel","account_id":"account-sentinel","email":"private@example.test"}}"#,
    )
    .unwrap();
    let mut profile = RelayProfile {
        id: "relay-one".to_string(),
        name: "Relay One".to_string(),
        model: "gpt-5.5".to_string(),
        base_url: "https://relay.example/v1".to_string(),
        upstream_base_url: "https://relay.example/v1".to_string(),
        api_key: "provider-key-sentinel".to_string(),
        protocol: RelayProtocol::Responses,
        relay_mode: RelayMode::Official,
        official_mix_api_key: true,
        config_contents: CONFIG.to_string(),
        ..RelayProfile::default()
    };
    profile.auth_contents =
        r#"{"tokens":{"access_token":"oauth-token-sentinel"},"email":"private@example.test"}"#
            .to_string();
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
        br#"{
          "target":{"clientVersion":"0.147.0-alpha.6.5","trusted":true},
          "profiles":{"relay-one":{"mode":"official-plus-custom","modeExplicit":true,"generatedHash":"catalog-hash","restartRequired":true,"actionRequired":"materialization failed"}}
        }"#,
    )
    .unwrap();
    let settings_before = std::fs::read(&settings_path).unwrap();
    let catalog_before = std::fs::read(&catalog_path).unwrap();
    let auth_before = std::fs::read(&auth_path).unwrap();

    let payload = inspect_provider_capability_evidence_from_paths(
        &settings_path,
        &catalog_path,
        &codex_home,
        ProviderCapabilityEvidenceRequest {
            profile_id: "relay-one".to_string(),
        },
    )
    .unwrap();

    assert_eq!(payload.provider_contract, ProviderContractEvidence::Ready);
    assert_eq!(payload.oauth_session, OAuthSessionEvidence::SignedIn);
    assert_eq!(payload.local_plan, LocalPlanEvidence::Unknown);
    assert_eq!(payload.actor_marker, ActorMarkerEvidence::Eligible);
    assert_eq!(payload.catalog_model, CatalogModelEvidence::Unknown);
    assert_eq!(payload.text_responses, TextResponsesEvidence::Unknown);
    assert_eq!(payload.image_generation, ImageGenerationEvidence::Unknown);
    assert_eq!(payload.runtime, RuntimeEvidence::RestartRequired);
    assert_eq!(
        payload.route_kind,
        ProviderRouteEvidence::NativePriorityMixed
    );
    assert_eq!(payload.image_plan_evidence, ImagePlanEvidence::Unknown);
    let encoded = serde_json::to_string(&payload).unwrap();
    for forbidden in [
        "provider-key-sentinel",
        "oauth-token-sentinel",
        "oauth-refresh-sentinel",
        "account-sentinel",
        "private@example.test",
        "configContents",
        "authContents",
        "accountLabel",
        "token",
        "bearer",
        "apiKey",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "leaked {forbidden}: {encoded}"
        );
    }
    assert_eq!(std::fs::read(&settings_path).unwrap(), settings_before);
    assert_eq!(std::fs::read(&catalog_path).unwrap(), catalog_before);
    assert_eq!(std::fs::read(&auth_path).unwrap(), auth_before);

    std::fs::write(
        &catalog_path,
        br#"{
          "target":{"clientVersion":"0.147.0-alpha.6.5","trusted":true},
          "profiles":{"relay-one":{"mode":"official-plus-custom","modeExplicit":true,"generatedHash":"catalog-hash","restartRequired":true}}
        }"#,
    )
    .unwrap();
    let generated_without_metadata = inspect_provider_capability_evidence_from_paths(
        &settings_path,
        &catalog_path,
        &codex_home,
        ProviderCapabilityEvidenceRequest {
            profile_id: "relay-one".to_string(),
        },
    )
    .unwrap();
    assert_eq!(
        generated_without_metadata.catalog_model,
        CatalogModelEvidence::MissingMetadata
    );
    assert_eq!(
        generated_without_metadata.text_responses,
        TextResponsesEvidence::Unknown
    );
    assert_eq!(
        generated_without_metadata.image_generation,
        ImageGenerationEvidence::Unknown
    );

    std::fs::write(
        &settings_path,
        b"oauth-refresh-sentinel private@example.test",
    )
    .unwrap();
    let failed = inspect_provider_capability_evidence_command_from_paths(
        &settings_path,
        &catalog_path,
        &codex_home,
        ProviderCapabilityEvidenceRequest {
            profile_id: "relay-one".to_string(),
        },
    );
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.payload.route_kind, ProviderRouteEvidence::Unknown);
    let failed_json = serde_json::to_string(&failed).unwrap();
    assert!(!failed_json.contains("oauth-refresh-sentinel"));
    assert!(!failed_json.contains("private@example.test"));

    let serialized_policy = serde_json::to_value(ImagePlanEvidence::VerifiedTargetPolicy {
        policy_source: ImagePolicySource::TargetCliPolicy,
        target_version: "0.147.0-alpha.6.5".to_string(),
        capability_path: ImageCapabilityPath::ProviderRoutedImageActorMarker,
        free_plan_rule: FreePlanRule::Blocked,
    })
    .unwrap();
    assert_eq!(serialized_policy["kind"], "verifiedTargetPolicy");
    assert_eq!(serialized_policy["policySource"], "targetCliPolicy");
    assert_eq!(serialized_policy["targetVersion"], "0.147.0-alpha.6.5");
    assert_eq!(
        serialized_policy["capabilityPath"],
        "providerRoutedImageActorMarker"
    );
    assert_eq!(serialized_policy["freePlanRule"], "blocked");
}

#[test]
fn non_native_routes_are_not_collapsed_into_legacy_or_failure_defaults() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let catalog_path = temp.path().join("model-catalog-state.json");
    let codex_home = temp.path().join("codex-home");
    std::fs::create_dir(&codex_home).unwrap();
    let base = RelayProfile {
        id: "pure-oauth".to_string(),
        name: "Pure OAuth".to_string(),
        relay_mode: RelayMode::Official,
        official_mix_api_key: false,
        protocol: RelayProtocol::Responses,
        config_contents: String::new(),
        ..RelayProfile::default()
    };
    let aggregate = RelayProfile {
        id: "aggregate".to_string(),
        name: "Aggregate".to_string(),
        relay_mode: RelayMode::Aggregate,
        ..RelayProfile::default()
    };
    let external = RelayProfile {
        id: "external".to_string(),
        name: "External".to_string(),
        relay_mode: RelayMode::Official,
        official_mix_api_key: true,
        protocol: RelayProtocol::Responses,
        config_contents: CONFIG.to_string(),
        ..RelayProfile::default()
    };
    let settings = BackendSettings {
        relay_profiles: vec![base, aggregate, external],
        ..BackendSettings::default()
    };
    std::fs::write(&settings_path, serde_json::to_vec(&settings).unwrap()).unwrap();
    std::fs::write(
        &catalog_path,
        br#"{"profiles":{"external":{"mode":"external","modeExplicit":true}}}"#,
    )
    .unwrap();

    for profile_id in ["pure-oauth", "aggregate", "external"] {
        let payload = inspect_provider_capability_evidence_from_paths(
            &settings_path,
            &catalog_path,
            &codex_home,
            ProviderCapabilityEvidenceRequest {
                profile_id: profile_id.to_string(),
            },
        )
        .unwrap();
        assert_eq!(payload.route_kind, ProviderRouteEvidence::NotApplicable);
    }
}

#[test]
fn an_unobserved_runtime_stays_unknown_instead_of_being_reported_as_adopted() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let catalog_path = temp.path().join("model-catalog-state.json");
    let codex_home = temp.path().join("codex-home");
    std::fs::create_dir(&codex_home).unwrap();
    std::fs::write(
        codex_home.join("auth.json"),
        br#"{"auth_mode":"chatgpt","tokens":{"account_id":"account-sentinel"}}"#,
    )
    .unwrap();
    let settings = BackendSettings {
        relay_profiles: vec![RelayProfile {
            id: "relay-one".to_string(),
            name: "Relay One".to_string(),
            model: "gpt-5.5".to_string(),
            base_url: "https://relay.example/v1".to_string(),
            upstream_base_url: "https://relay.example/v1".to_string(),
            api_key: "provider-key-sentinel".to_string(),
            protocol: RelayProtocol::Responses,
            relay_mode: RelayMode::Official,
            official_mix_api_key: true,
            config_contents: CONFIG.to_string(),
            ..RelayProfile::default()
        }],
        ..BackendSettings::default()
    };
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&settings).unwrap(),
    )
    .unwrap();

    // A committed generation whose restart marker was already acknowledged. There is no
    // trustworthy runtime observer, so adoption must stay unproven rather than assumed.
    std::fs::write(
        &catalog_path,
        br#"{
          "target":{"clientVersion":"0.147.0-alpha.6.6","trusted":true},
          "profiles":{"relay-one":{"mode":"official-plus-custom","modeExplicit":true,"generatedHash":"catalog-hash","restartRequired":false,"appliedRuntimeFingerprint":"sha256:applied"}}
        }"#,
    )
    .unwrap();

    let payload = inspect_provider_capability_evidence_from_paths(
        &settings_path,
        &catalog_path,
        &codex_home,
        ProviderCapabilityEvidenceRequest {
            profile_id: "relay-one".to_string(),
        },
    )
    .unwrap();

    assert_eq!(payload.runtime, RuntimeEvidence::Unknown);
    assert_ne!(payload.runtime, RuntimeEvidence::Adopted);
}

/// The provider-routable capability matrix is row-scoped.
///
/// Rows: text Responses, model discovery, image generation, image editing, remote compaction,
/// web search. A complete native-priority contract is evidence about the contract, not about any
/// row. Each row's outcome — success, denial, fallback, or unknown — belongs to that row alone
/// and must be observed independently. Rows this build models explicitly are asserted here; rows
/// it does not model at all cannot be claimed, which is the same guarantee by construction.
#[test]
fn capability_rows_are_scoped_and_never_inferred_from_a_complete_contract() {
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    let catalog_path = temp.path().join("model-catalog-state.json");
    let codex_home = temp.path().join("codex-home");
    std::fs::create_dir(&codex_home).unwrap();
    std::fs::write(
        codex_home.join("auth.json"),
        br#"{"auth_mode":"chatgpt","tokens":{"account_id":"account-sentinel"}}"#,
    )
    .unwrap();
    let settings = BackendSettings {
        relay_profiles: vec![RelayProfile {
            id: "relay-one".to_string(),
            name: "Relay One".to_string(),
            model: "gpt-5.5".to_string(),
            base_url: "https://relay.example/v1".to_string(),
            upstream_base_url: "https://relay.example/v1".to_string(),
            api_key: "provider-key-sentinel".to_string(),
            protocol: RelayProtocol::Responses,
            relay_mode: RelayMode::Official,
            official_mix_api_key: true,
            config_contents: CONFIG.to_string(),
            ..RelayProfile::default()
        }],
        ..BackendSettings::default()
    };
    std::fs::write(
        &settings_path,
        serde_json::to_vec_pretty(&settings).unwrap(),
    )
    .unwrap();
    std::fs::write(
        &catalog_path,
        br#"{
          "target":{"clientVersion":"0.147.0-alpha.6.6","trusted":true},
          "profiles":{"relay-one":{"mode":"official-plus-custom","modeExplicit":true,"generatedHash":"catalog-hash"}}
        }"#,
    )
    .unwrap();

    let payload = inspect_provider_capability_evidence_from_paths(
        &settings_path,
        &catalog_path,
        &codex_home,
        ProviderCapabilityEvidenceRequest {
            profile_id: "relay-one".to_string(),
        },
    )
    .unwrap();

    // The contract itself is complete and the client is eligible to mark itself.
    assert_eq!(payload.provider_contract, ProviderContractEvidence::Ready);
    assert_eq!(payload.actor_marker, ActorMarkerEvidence::Eligible);
    assert_eq!(payload.route_kind, ProviderRouteEvidence::NativePriorityMixed);

    // None of that is evidence for any capability row.
    assert_eq!(payload.text_responses, TextResponsesEvidence::Unknown);
    assert_eq!(payload.image_generation, ImageGenerationEvidence::Unknown);
    assert_eq!(payload.image_plan_evidence, ImagePlanEvidence::Unknown);
    // Model discovery: an existing artifact is not metadata for the selected model.
    assert_eq!(payload.catalog_model, CatalogModelEvidence::MissingMetadata);
    // Remote compaction and web search have no observation at all, so the payload carries no
    // field that could report them as available.
    let serialized = serde_json::to_string(&payload).unwrap();
    for absent in ["compaction", "webSearch", "imageEdit"] {
        assert!(
            !serialized.contains(absent),
            "an unobserved capability row appeared in the payload as {absent}"
        );
    }
}
