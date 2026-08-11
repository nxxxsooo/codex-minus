use codex_minus_lib::provider_capability_evidence::{
    ActorMarkerEvidence, CatalogModelEvidence, ImagePlanEvidence, LocalPlanEvidence,
    OAuthSessionEvidence, ProviderCapabilityEvidenceRequest, ProviderContractEvidence,
    ProviderRouteEvidence, RuntimeEvidence, inspect_provider_capability_evidence_from_paths,
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
          "profiles":{"relay-one":{"mode":"official-plus-custom","modeExplicit":true,"generatedHash":"catalog-hash","restartRequired":true}}
        }"#,
    )
    .unwrap();
    let settings_before = std::fs::read(&settings_path).unwrap();
    let catalog_before = std::fs::read(&catalog_path).unwrap();

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
    assert_eq!(payload.oauth_session, OAuthSessionEvidence::SignedOut);
    assert_eq!(payload.local_plan, LocalPlanEvidence::Unknown);
    assert_eq!(payload.actor_marker, ActorMarkerEvidence::Eligible);
    assert_eq!(payload.catalog_model, CatalogModelEvidence::MissingMetadata);
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
}
