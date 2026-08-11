use std::cell::Cell;

use codex_minus_lib::provider_native_capability::{
    CatalogMode, MANAGED_ACTOR_HEADER_NAME, MANAGED_ACTOR_HEADER_VALUE,
    NativeCapabilityDraftAction, NativeCapabilityDraftConfirmation, NativeCapabilityDraftStatus,
    NativeCapabilityReason, ProviderNativeCapabilityDraftReadOnlyBoundary,
    ProviderNativeCapabilityDraftRequest, draft_provider_native_capability,
    draft_provider_native_capability_with_boundary, inspect_profile,
    transform_provider_native_capability_draft,
    transform_provider_native_capability_draft_from_settings_path,
};
use codex_plus_core::settings::{BackendSettings, RelayMode, RelayProfile, RelayProtocol};
use toml_edit::{DocumentMut, Item, value};

fn mixed_profile(id: &str, api_key: &str, config_contents: &str) -> RelayProfile {
    RelayProfile {
        id: id.to_string(),
        name: id.to_string(),
        api_key: api_key.to_string(),
        protocol: RelayProtocol::Responses,
        relay_mode: RelayMode::Official,
        official_mix_api_key: true,
        config_contents: config_contents.to_string(),
        ..RelayProfile::default()
    }
}

fn request(
    profile: RelayProfile,
    catalog_mode: CatalogMode,
    action: NativeCapabilityDraftAction,
) -> ProviderNativeCapabilityDraftRequest {
    ProviderNativeCapabilityDraftRequest {
        draft_revision: 41,
        profile,
        catalog_mode,
        action,
        source_config_contents: None,
        confirmations: Vec::new(),
        replacement_provider_id: None,
    }
}

#[test]
fn typescript_transform_request_shape_deserializes_through_the_real_serde_boundary() {
    let profile = mixed_profile("serde", "same-secret", &canonical_source("inline"));
    let wire = serde_json::json!({
        "draftRevision": 73,
        "profile": profile,
        "catalogMode": "official-plus-custom",
        "action": "enableNativePriority",
        "confirmations": ["replaceActorHeader", "useStructuredKey"]
    });

    let request: ProviderNativeCapabilityDraftRequest = serde_json::from_value(wire).unwrap();
    assert_eq!(request.draft_revision, 73);
    assert_eq!(request.catalog_mode, CatalogMode::OfficialPlusCustom);
    assert_eq!(
        request.action,
        NativeCapabilityDraftAction::EnableNativePriority
    );
    assert_eq!(
        request.confirmations,
        vec![
            NativeCapabilityDraftConfirmation::ReplaceActorHeader,
            NativeCapabilityDraftConfirmation::UseStructuredKey,
        ]
    );

    let wrong_case = serde_json::json!({
        "draftRevision": 74,
        "profile": mixed_profile("serde-wrong", "same-secret", &canonical_source("inline")),
        "catalogMode": "officialPlusCustom",
        "action": "enableNativePriority",
        "confirmations": []
    });
    assert!(serde_json::from_value::<ProviderNativeCapabilityDraftRequest>(wrong_case).is_err());

    let raw_source = canonical_source("inline");
    let raw_wire = serde_json::json!({
        "draftRevision": 75,
        "profile": mixed_profile("serde-raw", "same-secret", &raw_source),
        "catalogMode": "official-plus-custom",
        "action": "validateRawEdit",
        "confirmations": [],
        "sourceConfigContents": raw_source,
    });
    let raw_request: ProviderNativeCapabilityDraftRequest =
        serde_json::from_value(raw_wire).unwrap();
    assert_eq!(
        raw_request.action,
        NativeCapabilityDraftAction::ValidateRawEdit
    );
    assert!(raw_request.source_config_contents.is_some());
}

fn parsed(
    payload: &codex_minus_lib::provider_native_capability::ProviderNativeCapabilityDraftPayload,
) -> DocumentMut {
    payload.draft.profile.config_contents.parse().unwrap()
}

fn provider<'a>(document: &'a DocumentMut, provider_id: &str) -> &'a dyn toml_edit::TableLike {
    document["model_providers"][provider_id]
        .as_table_like()
        .unwrap()
}

fn raw_edit_request(
    source_profile: &RelayProfile,
    candidate_config: String,
) -> ProviderNativeCapabilityDraftRequest {
    let mut profile = source_profile.clone();
    profile.config_contents = candidate_config;
    let mut request = request(
        profile,
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::ValidateRawEdit,
    );
    request.source_config_contents = Some(source_profile.config_contents.clone());
    request
}

fn canonical_source(header_shape: &str) -> String {
    let header = match header_shape {
        "inline" => r#"http_headers = { "x-unrelated-header" = "keep-me" }"#.to_string(),
        "table" => r#"

[model_providers.RelayOne.http_headers]
"x-unrelated-header" = "keep-me"
"#
        .to_string(),
        _ => unreachable!(),
    };
    format!(
        r#"# root-comment
model = "gpt-5"
model_provider = "RelayOne"
unrelated_root = "keep-root"

[model_providers.RelayOne] # provider-comment
name = "custom" # name-comment
base_url = "https://relay.example/v1" # base-comment
wire_api = "chat"
requires_openai_auth = true
experimental_bearer_token = "same-secret" # bearer-comment
arbitrary_provider_key = "keep-provider" # arbitrary-comment
{header}
"#
    )
}

#[test]
fn enabling_edits_only_owned_fields_and_preserves_nonlegacy_identity_evidence_and_comments() {
    for header_shape in ["inline", "table"] {
        let profile = mixed_profile("profile", "same-secret", &canonical_source(header_shape));
        let payload = draft_provider_native_capability(&request(
            profile,
            CatalogMode::NativeOfficial,
            NativeCapabilityDraftAction::EnableNativePriority,
        ));

        assert_eq!(
            payload.status,
            NativeCapabilityDraftStatus::Ready,
            "{header_shape}"
        );
        assert_eq!(payload.draft_revision, 41);
        assert_eq!(payload.draft.catalog_mode, CatalogMode::OfficialPlusCustom);
        assert_eq!(payload.draft.profile.relay_mode, RelayMode::Official);
        assert!(payload.draft.profile.official_mix_api_key);
        assert_eq!(payload.draft.profile.protocol, RelayProtocol::Responses);
        assert_eq!(payload.draft.structured_api_key, "same-secret");

        let document = parsed(&payload);
        assert_eq!(document["model_provider"].as_str(), Some("RelayOne"));
        assert_eq!(document["unrelated_root"].as_str(), Some("keep-root"));
        let selected = provider(&document, "RelayOne");
        assert_eq!(selected.get("name").and_then(Item::as_str), Some("OpenAI"));
        assert_eq!(
            selected.get("base_url").and_then(Item::as_str),
            Some("https://relay.example/v1")
        );
        assert_eq!(
            selected.get("wire_api").and_then(Item::as_str),
            Some("responses")
        );
        assert_eq!(
            selected.get("requires_openai_auth").and_then(Item::as_bool),
            Some(false)
        );
        assert_eq!(
            selected
                .get("experimental_bearer_token")
                .and_then(Item::as_str),
            Some("same-secret")
        );
        assert_eq!(
            selected
                .get("arbitrary_provider_key")
                .and_then(Item::as_str),
            Some("keep-provider")
        );
        let headers = selected
            .get("http_headers")
            .unwrap()
            .as_table_like()
            .unwrap();
        assert_eq!(
            headers
                .get("x-openai-actor-authorization")
                .and_then(Item::as_str),
            Some("local-image-extension")
        );
        assert_eq!(
            headers.get("x-unrelated-header").and_then(Item::as_str),
            Some("keep-me")
        );
        let rendered = document.to_string();
        for comment in [
            "# root-comment",
            "# provider-comment",
            "# base-comment",
            "# bearer-comment",
            "# arbitrary-comment",
        ] {
            assert!(
                rendered.contains(comment),
                "lost {comment} in {header_shape}\n{rendered}"
            );
        }
    }
}

#[test]
fn legacy_alias_moves_the_complete_table_only_to_an_available_or_identical_target() {
    let source = r#"model = "gpt-5"
model_provider = "CodexPlusPlus"

[model_providers.CodexPlusPlus]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "legacy-secret"
arbitrary = "preserve"
http_headers = { "x-openai-actor-authorization" = "local-image-extension", extra = "keep" }
"#;
    let payload = draft_provider_native_capability(&request(
        mixed_profile("legacy", "legacy-secret", source),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    ));
    assert_eq!(payload.status, NativeCapabilityDraftStatus::Ready);
    let document = parsed(&payload);
    assert_eq!(document["model_provider"].as_str(), Some("custom"));
    assert!(document["model_providers"].get("CodexPlusPlus").is_none());
    assert_eq!(
        provider(&document, "custom")
            .get("arbitrary")
            .and_then(Item::as_str),
        Some("preserve")
    );

    let (_, provider_body) = source
        .split_once("[model_providers.CodexPlusPlus]")
        .unwrap();
    let identical = format!("{source}\n[model_providers.custom]{provider_body}");
    let payload = draft_provider_native_capability(&request(
        mixed_profile("identical", "legacy-secret", &identical),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    ));
    assert_eq!(payload.status, NativeCapabilityDraftStatus::Ready);
    let document = parsed(&payload);
    assert_eq!(document["model_provider"].as_str(), Some("custom"));
    assert!(document["model_providers"].get("CodexPlusPlus").is_none());
    assert!(document["model_providers"].get("custom").is_some());
}

#[test]
fn different_custom_table_blocks_legacy_migration_until_an_unused_id_is_chosen() {
    let source = r#"model = "gpt-5"
model_provider = "CodexPP"

[model_providers.CodexPP]
name = "OpenAI"
base_url = "https://legacy.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "legacy-secret"
arbitrary = "legacy-content"
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }

[model_providers.custom]
name = "different"
base_url = "https://different.example/v1"
arbitrary = "custom-content"
"#;
    let original = mixed_profile("collision", "legacy-secret", source);
    let blocked = draft_provider_native_capability(&request(
        original.clone(),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    ));
    assert_eq!(blocked.status, NativeCapabilityDraftStatus::Blocked);
    assert_eq!(
        blocked.draft.profile.config_contents,
        original.config_contents
    );
    assert_eq!(
        blocked.blockers,
        vec![NativeCapabilityReason::ReplacementProviderIdRequired]
    );

    let mut confirmed_request = request(
        original,
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    );
    confirmed_request.replacement_provider_id = Some("RelayReplacement".to_string());
    let confirmed = draft_provider_native_capability(&confirmed_request);
    assert_eq!(confirmed.status, NativeCapabilityDraftStatus::Ready);
    let document = parsed(&confirmed);
    assert_eq!(
        document["model_provider"].as_str(),
        Some("RelayReplacement")
    );
    assert!(document["model_providers"].get("CodexPP").is_none());
    assert_eq!(
        provider(&document, "RelayReplacement")
            .get("arbitrary")
            .and_then(Item::as_str),
        Some("legacy-content")
    );
    assert_eq!(
        provider(&document, "custom")
            .get("arbitrary")
            .and_then(Item::as_str),
        Some("custom-content")
    );

    for invalid in ["openai", "CodexPlusPlus", "custom"] {
        let mut invalid_request = confirmed_request.clone();
        invalid_request.replacement_provider_id = Some(invalid.to_string());
        let rejected = draft_provider_native_capability(&invalid_request);
        assert_eq!(
            rejected.status,
            NativeCapabilityDraftStatus::Blocked,
            "{invalid}"
        );
        assert_eq!(rejected.draft.profile.config_contents, source, "{invalid}");
    }
}

#[test]
fn actor_header_conflict_and_semantic_duplicates_require_safe_explicit_resolution() {
    let custom = r#"model = "gpt-5"
model_provider = "RelayOne"
[model_providers.RelayOne]
name = "custom"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "same-secret"
http_headers = { "x-openai-actor-authorization" = "customer-owned", extra = "keep" }
"#;
    let original = mixed_profile("actor", "same-secret", custom);
    let blocked = draft_provider_native_capability(&request(
        original.clone(),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    ));
    assert_eq!(
        blocked.status,
        NativeCapabilityDraftStatus::ConfirmationRequired
    );
    assert_eq!(
        blocked.draft.profile.config_contents,
        original.config_contents
    );
    assert_eq!(
        blocked.blockers,
        vec![NativeCapabilityReason::ActorHeaderValueConflict]
    );
    let redacted = serde_json::to_string(&blocked.blockers).unwrap();
    assert!(!redacted.contains("customer-owned"));
    assert!(!redacted.contains("same-secret"));

    let mut confirmed_request = request(
        original,
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    );
    confirmed_request
        .confirmations
        .push(NativeCapabilityDraftConfirmation::ReplaceActorHeader);
    let confirmed = draft_provider_native_capability(&confirmed_request);
    assert_eq!(confirmed.status, NativeCapabilityDraftStatus::Ready);
    let document = parsed(&confirmed);
    let headers = provider(&document, "RelayOne")
        .get("http_headers")
        .unwrap()
        .as_table_like()
        .unwrap();
    assert_eq!(
        headers
            .get("x-openai-actor-authorization")
            .and_then(Item::as_str),
        Some("local-image-extension")
    );
    assert_eq!(headers.get("extra").and_then(Item::as_str), Some("keep"));

    let duplicate = custom.replace(
        "extra = \"keep\"",
        "\"X-OpenAI-Actor-Authorization\" = \"customer-owned\"",
    );
    let mut duplicate_request = request(
        mixed_profile("duplicate", "same-secret", &duplicate),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    );
    duplicate_request
        .confirmations
        .push(NativeCapabilityDraftConfirmation::ReplaceActorHeader);
    let rejected = draft_provider_native_capability(&duplicate_request);
    assert_eq!(rejected.status, NativeCapabilityDraftStatus::Blocked);
    assert_eq!(
        rejected.blockers,
        vec![NativeCapabilityReason::DuplicateActorHeader]
    );
    assert_eq!(rejected.draft.profile.config_contents, duplicate);
}

#[test]
fn structured_key_bearer_mismatch_blocks_until_one_explicit_sync_direction_is_chosen() {
    let source = canonical_source("inline").replace("same-secret", "raw-secret");
    let original = mixed_profile("key-conflict", "structured-secret", &source);
    let blocked = draft_provider_native_capability(&request(
        original.clone(),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    ));
    assert_eq!(
        blocked.status,
        NativeCapabilityDraftStatus::ConfirmationRequired
    );
    assert_eq!(
        blocked.blockers,
        vec![NativeCapabilityReason::StructuredKeyBearerConflict]
    );
    assert_eq!(
        blocked.draft.profile.config_contents,
        original.config_contents
    );
    let redacted = serde_json::to_string(&blocked.blockers).unwrap();
    assert!(!redacted.contains("raw-secret"));
    assert!(!redacted.contains("structured-secret"));

    let mut use_structured = request(
        original.clone(),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    );
    use_structured
        .confirmations
        .push(NativeCapabilityDraftConfirmation::UseStructuredKey);
    let synchronized = draft_provider_native_capability(&use_structured);
    assert_eq!(synchronized.status, NativeCapabilityDraftStatus::Ready);
    assert_eq!(
        provider(&parsed(&synchronized), "RelayOne")
            .get("experimental_bearer_token")
            .and_then(Item::as_str),
        Some("structured-secret")
    );

    let mut use_bearer = request(
        original,
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    );
    use_bearer
        .confirmations
        .push(NativeCapabilityDraftConfirmation::UseProviderBearer);
    let synchronized = draft_provider_native_capability(&use_bearer);
    assert_eq!(synchronized.status, NativeCapabilityDraftStatus::Ready);
    assert_eq!(synchronized.draft.structured_api_key, "raw-secret");
}

#[test]
fn matching_bearer_keeps_its_exact_lexical_item_and_credential_bytes() {
    let source = r#"model = "gpt-5"
model_provider = "RelayOne"

[model_providers.RelayOne]
name = "custom"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = true
experimental_bearer_token = '  padded-secret  ' # preserve-bearer-lexical
http_headers = { extra = "keep" }
"#;
    let payload = draft_provider_native_capability(&request(
        mixed_profile("lexical", "  padded-secret  ", source),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    ));

    assert_eq!(payload.status, NativeCapabilityDraftStatus::Ready);
    assert_eq!(payload.draft.structured_api_key, "  padded-secret  ");
    assert!(
        payload
            .draft
            .profile
            .config_contents
            .contains("experimental_bearer_token = '  padded-secret  ' # preserve-bearer-lexical")
    );
}

#[test]
fn exits_cannot_bypass_a_structured_key_bearer_conflict() {
    let source = enabled_exit_source().replace("same-secret", "raw-secret");
    let original = mixed_profile("exit-key-conflict", "structured-secret", &source);

    let mut pure_api = request(
        original.clone(),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::ExitPureApi,
    );
    pure_api
        .confirmations
        .push(NativeCapabilityDraftConfirmation::ConfirmCapabilityLoss);
    let blocked = draft_provider_native_capability(&pure_api);
    assert_eq!(
        blocked.status,
        NativeCapabilityDraftStatus::ConfirmationRequired
    );
    assert_eq!(
        blocked.blockers,
        vec![NativeCapabilityReason::StructuredKeyBearerConflict]
    );
    assert_eq!(
        blocked.draft.profile.config_contents,
        original.config_contents
    );

    pure_api
        .confirmations
        .push(NativeCapabilityDraftConfirmation::UseStructuredKey);
    let synchronized = draft_provider_native_capability(&pure_api);
    assert_eq!(synchronized.status, NativeCapabilityDraftStatus::Ready);
    assert_eq!(
        provider(&parsed(&synchronized), "RelayOne")
            .get("experimental_bearer_token")
            .and_then(Item::as_str),
        Some("structured-secret")
    );

    let mut pure_oauth = request(
        original,
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::ExitPureOAuth,
    );
    pure_oauth
        .confirmations
        .push(NativeCapabilityDraftConfirmation::ConfirmDestructivePureOAuth);
    let blocked = draft_provider_native_capability(&pure_oauth);
    assert_eq!(
        blocked.status,
        NativeCapabilityDraftStatus::ConfirmationRequired
    );
    assert_eq!(
        blocked.blockers,
        vec![NativeCapabilityReason::StructuredKeyBearerConflict]
    );
    pure_oauth
        .confirmations
        .push(NativeCapabilityDraftConfirmation::UseProviderBearer);
    let synchronized = draft_provider_native_capability(&pure_oauth);
    assert_eq!(synchronized.status, NativeCapabilityDraftStatus::Ready);
    assert!(synchronized.draft.structured_api_key.is_empty());
    assert!(
        !synchronized
            .draft
            .profile
            .config_contents
            .contains("raw-secret")
    );
}

fn enabled_exit_source() -> &'static str {
    r#"model = "gpt-5"
model_provider = "RelayOne"
unrelated_root = "keep-root"

[model_providers.RelayOne]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "same-secret"
arbitrary = "keep-provider"
http_headers = { "x-openai-actor-authorization" = "local-image-extension", extra = "keep-header" }
"#
}

#[test]
fn pure_api_and_legacy_exits_preserve_unowned_provider_content() {
    for (action, expected_mode, expected_auth, expected_name) in [
        (
            NativeCapabilityDraftAction::ExitPureApi,
            RelayMode::PureApi,
            false,
            "OpenAI",
        ),
        (
            NativeCapabilityDraftAction::ExitLegacyCompatibility,
            RelayMode::Official,
            true,
            "custom",
        ),
    ] {
        let mut request = request(
            mixed_profile("exit", "same-secret", enabled_exit_source()),
            CatalogMode::OfficialPlusCustom,
            action,
        );
        request
            .confirmations
            .push(NativeCapabilityDraftConfirmation::ConfirmCapabilityLoss);
        let payload = draft_provider_native_capability(&request);
        assert_eq!(
            payload.status,
            NativeCapabilityDraftStatus::Ready,
            "{action:?}"
        );
        assert!(payload.preview.capability_loss);
        assert_eq!(payload.draft.profile.relay_mode, expected_mode);
        let document = parsed(&payload);
        assert_eq!(document["unrelated_root"].as_str(), Some("keep-root"));
        let selected = provider(&document, "RelayOne");
        assert_eq!(
            selected.get("name").and_then(Item::as_str),
            Some(expected_name)
        );
        assert_eq!(
            selected.get("requires_openai_auth").and_then(Item::as_bool),
            Some(expected_auth)
        );
        assert_eq!(
            selected.get("arbitrary").and_then(Item::as_str),
            Some("keep-provider")
        );
        let headers = selected
            .get("http_headers")
            .unwrap()
            .as_table_like()
            .unwrap();
        assert!(headers.get("x-openai-actor-authorization").is_none());
        assert_eq!(
            headers.get("extra").and_then(Item::as_str),
            Some("keep-header")
        );
    }
}

#[test]
fn pure_oauth_requires_destructive_confirmation_then_removes_the_complete_selected_table() {
    for starting_mode in [CatalogMode::OfficialPlusCustom, CatalogMode::External] {
        let original = mixed_profile("oauth-exit", "same-secret", enabled_exit_source());
        let unconfirmed = draft_provider_native_capability(&request(
            original.clone(),
            starting_mode,
            NativeCapabilityDraftAction::ExitPureOAuth,
        ));
        assert_eq!(
            unconfirmed.status,
            NativeCapabilityDraftStatus::ConfirmationRequired
        );
        assert_eq!(
            unconfirmed.draft.profile.config_contents,
            original.config_contents
        );
        assert!(unconfirmed.preview.removes_provider_table);
        assert_eq!(
            unconfirmed.preview.removed_provider_id.as_deref(),
            Some("RelayOne")
        );
        for field in ["experimental_bearer_token", "arbitrary", "http_headers"] {
            assert!(
                unconfirmed
                    .preview
                    .removed_provider_fields
                    .iter()
                    .any(|item| item == field),
                "missing destructive preview field {field}"
            );
        }

        let mut confirmed_request = request(
            original,
            starting_mode,
            NativeCapabilityDraftAction::ExitPureOAuth,
        );
        confirmed_request
            .confirmations
            .push(NativeCapabilityDraftConfirmation::ConfirmDestructivePureOAuth);
        let confirmed = draft_provider_native_capability(&confirmed_request);
        assert_eq!(confirmed.status, NativeCapabilityDraftStatus::Ready);
        assert_eq!(confirmed.draft.profile.relay_mode, RelayMode::Official);
        assert!(!confirmed.draft.profile.official_mix_api_key);
        assert!(confirmed.draft.structured_api_key.is_empty());
        assert_eq!(
            confirmed.draft.catalog_mode,
            if starting_mode == CatalogMode::External {
                CatalogMode::External
            } else {
                CatalogMode::NativeOfficial
            }
        );
        let document = parsed(&confirmed);
        assert!(document.get("model_provider").is_none());
        assert!(
            document
                .get("model_providers")
                .and_then(Item::as_table_like)
                .is_none_or(|providers| providers.get("RelayOne").is_none())
        );
        let rendered = document.to_string();
        assert!(!rendered.contains("same-secret"));
        assert!(!rendered.contains("keep-provider"));
        assert!(rendered.contains("unrelated_root = \"keep-root\""));
    }
}

#[test]
fn protocol_change_to_chat_completions_is_an_explicit_capability_loss_exit() {
    let original = mixed_profile("chat-exit", "same-secret", enabled_exit_source());
    let unconfirmed = draft_provider_native_capability(&request(
        original.clone(),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::ExitChatCompletions,
    ));
    assert_eq!(
        unconfirmed.status,
        NativeCapabilityDraftStatus::ConfirmationRequired
    );
    assert_eq!(unconfirmed.draft.profile.protocol, RelayProtocol::Responses);
    assert_eq!(
        unconfirmed.draft.profile.config_contents,
        original.config_contents
    );

    let mut confirmed_request = request(
        original,
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::ExitChatCompletions,
    );
    confirmed_request
        .confirmations
        .push(NativeCapabilityDraftConfirmation::ConfirmCapabilityLoss);
    let confirmed = draft_provider_native_capability(&confirmed_request);
    assert_eq!(confirmed.status, NativeCapabilityDraftStatus::Ready);
    assert_eq!(
        confirmed.draft.profile.protocol,
        RelayProtocol::ChatCompletions
    );
    assert!(confirmed.preview.capability_loss);
    assert_eq!(
        provider(&parsed(&confirmed), "RelayOne")
            .get("arbitrary")
            .and_then(Item::as_str),
        Some("keep-provider")
    );
}

#[test]
fn every_compatibility_exit_rejects_a_non_string_semantic_actor_header() {
    let malformed = enabled_exit_source().replace(
        "\"x-openai-actor-authorization\" = \"local-image-extension\"",
        "\"x-openai-actor-authorization\" = 123",
    );
    for action in [
        NativeCapabilityDraftAction::ExitPureApi,
        NativeCapabilityDraftAction::ExitLegacyCompatibility,
        NativeCapabilityDraftAction::ExitChatCompletions,
    ] {
        let mut request = request(
            mixed_profile("malformed-exit", "same-secret", &malformed),
            CatalogMode::OfficialPlusCustom,
            action,
        );
        request
            .confirmations
            .push(NativeCapabilityDraftConfirmation::ConfirmCapabilityLoss);
        let blocked = draft_provider_native_capability(&request);
        assert_eq!(
            blocked.status,
            NativeCapabilityDraftStatus::Blocked,
            "{action:?}"
        );
        assert_eq!(
            blocked.blockers,
            vec![NativeCapabilityReason::MalformedHeaderStructure],
            "{action:?}"
        );
        assert_eq!(
            blocked.draft.profile.config_contents, malformed,
            "{action:?}"
        );
    }
}

#[test]
fn external_ownership_blocks_enablement_and_is_never_silently_adopted() {
    let original = mixed_profile("external", "same-secret", &canonical_source("inline"));
    let blocked = draft_provider_native_capability(&request(
        original.clone(),
        CatalogMode::External,
        NativeCapabilityDraftAction::EnableNativePriority,
    ));
    assert_eq!(blocked.status, NativeCapabilityDraftStatus::Blocked);
    assert_eq!(
        blocked.blockers,
        vec![NativeCapabilityReason::ExternalCatalog]
    );
    assert_eq!(blocked.draft.catalog_mode, CatalogMode::External);
    assert_eq!(
        blocked.draft.profile.config_contents,
        original.config_contents
    );
}

#[test]
fn revisioned_command_uses_only_the_injected_read_only_inspection_boundary() {
    #[derive(Default)]
    struct AuditBoundary {
        inspection_calls: Cell<usize>,
    }
    impl ProviderNativeCapabilityDraftReadOnlyBoundary for AuditBoundary {
        fn inspect(
            &self,
            profile: &RelayProfile,
            catalog_mode: CatalogMode,
        ) -> codex_minus_lib::provider_native_capability::ProviderNativeCapabilityInspection
        {
            self.inspection_calls
                .set(self.inspection_calls.get().saturating_add(1));
            inspect_profile(profile, catalog_mode)
        }
    }

    let request = request(
        mixed_profile("pure", "same-secret", &canonical_source("inline")),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    );
    let audit = AuditBoundary::default();
    let audited = draft_provider_native_capability_with_boundary(&request, &audit);
    assert_eq!(audit.inspection_calls.get(), 2);

    let command_payload =
        tauri::async_runtime::block_on(transform_provider_native_capability_draft(request));
    assert_eq!(command_payload.draft_revision, 41);
    assert_eq!(command_payload.status, NativeCapabilityDraftStatus::Ready);
    assert_eq!(
        serde_json::to_value(&command_payload).unwrap(),
        serde_json::to_value(&audited).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&command_payload).unwrap()["draftRevision"],
        serde_json::json!(41)
    );
}

#[test]
fn zero_revision_is_echoed_without_business_semantics() {
    let mut request = request(
        mixed_profile("zero", "same-secret", &canonical_source("inline")),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    );
    request.draft_revision = 0;
    let payload = draft_provider_native_capability(&request);
    assert_eq!(payload.draft_revision, 0);
    assert_eq!(payload.status, NativeCapabilityDraftStatus::Ready);
    assert!(payload.blockers.is_empty());
}

fn string_paths_containing(value: &serde_json::Value, needle: &str) -> Vec<String> {
    fn visit(value: &serde_json::Value, needle: &str, path: &str, found: &mut Vec<String>) {
        match value {
            serde_json::Value::String(text) if text.contains(needle) => {
                found.push(path.to_string());
            }
            serde_json::Value::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    visit(value, needle, &format!("{path}/{index}"), found);
                }
            }
            serde_json::Value::Object(values) => {
                for (key, value) in values {
                    visit(value, needle, &format!("{path}/{key}"), found);
                }
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    visit(value, needle, "", &mut found);
    found.sort();
    found
}

#[test]
fn direct_command_rejects_auth_contents_and_never_echoes_them() {
    for auth_contents in ["oauth-auth-sentinel", " \t "] {
        let mut profile = mixed_profile(
            "auth-rejected",
            "provider-secret-sentinel",
            &canonical_source("inline").replace("same-secret", "provider-secret-sentinel"),
        );
        profile.auth_contents = auth_contents.to_string();
        let payload =
            tauri::async_runtime::block_on(transform_provider_native_capability_draft(request(
                profile,
                CatalogMode::OfficialPlusCustom,
                NativeCapabilityDraftAction::Inspect,
            )));
        assert_eq!(payload.status, NativeCapabilityDraftStatus::Blocked);
        assert_eq!(
            payload.blockers,
            vec![NativeCapabilityReason::AuthContentsForbidden]
        );
        assert!(payload.draft.profile.auth_contents.is_empty());
        let serialized = serde_json::to_string(&payload).unwrap();
        assert!(!serialized.contains(auth_contents));
    }
}

#[test]
fn raw_edit_backend_adopts_only_parsed_non_contract_changes() {
    let upgraded = draft_provider_native_capability(&request(
        mixed_profile("raw-edit", "same-secret", &canonical_source("inline")),
        CatalogMode::NativeOfficial,
        NativeCapabilityDraftAction::EnableNativePriority,
    ));
    assert_eq!(upgraded.status, NativeCapabilityDraftStatus::Ready);
    let source = upgraded.draft.profile;
    let candidate = source
        .config_contents
        .replace("gpt-5", "gpt-5.6")
        .replace("https://relay.example/v1", "https://new.example/v1")
        .replace("keep-root", "changed-root")
        .replace("keep-me", "changed-header");
    let payload = draft_provider_native_capability(&raw_edit_request(&source, candidate.clone()));

    assert_eq!(payload.status, NativeCapabilityDraftStatus::Ready);
    assert!(payload.blockers.is_empty());
    assert_eq!(payload.draft.profile.config_contents, candidate);
    assert_eq!(payload.draft.profile.model, "gpt-5.6");
    assert_eq!(payload.draft.profile.base_url, "https://new.example/v1");
    assert_eq!(
        payload.draft.profile.upstream_base_url,
        "https://new.example/v1"
    );
    assert!(
        payload
            .draft
            .profile
            .config_contents
            .contains("local-image-extension")
    );

    let without_required_values = candidate
        .lines()
        .filter(|line| !line.starts_with("model = ") && !line.starts_with("base_url = "))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let missing = draft_provider_native_capability(&raw_edit_request(
        &payload.draft.profile,
        without_required_values,
    ));
    assert_eq!(missing.status, NativeCapabilityDraftStatus::Ready);
    assert!(missing.draft.profile.model.is_empty());
    assert!(missing.draft.profile.base_url.is_empty());
    assert!(missing.draft.profile.upstream_base_url.is_empty());
}

#[test]
fn raw_edit_backend_blocks_malformed_and_owned_contract_changes_without_secret_errors() {
    let upgraded = draft_provider_native_capability(&request(
        mixed_profile("raw-blocked", "same-secret", &canonical_source("inline")),
        CatalogMode::NativeOfficial,
        NativeCapabilityDraftAction::EnableNativePriority,
    ));
    let source = upgraded.draft.profile;
    let mut actor_removed = source.config_contents.parse::<DocumentMut>().unwrap();
    actor_removed["model_providers"]["RelayOne"]["http_headers"]
        .as_table_like_mut()
        .unwrap()
        .remove(MANAGED_ACTOR_HEADER_NAME);
    let mut actor_non_string = source.config_contents.parse::<DocumentMut>().unwrap();
    actor_non_string["model_providers"]["RelayOne"]["http_headers"]
        .as_table_like_mut()
        .unwrap()
        .insert(MANAGED_ACTOR_HEADER_NAME, value(7));
    let mut actor_duplicate = source.config_contents.parse::<DocumentMut>().unwrap();
    actor_duplicate["model_providers"]["RelayOne"]["http_headers"]
        .as_table_like_mut()
        .unwrap()
        .insert(
            "X-OpenAI-Actor-Authorization",
            value(MANAGED_ACTOR_HEADER_VALUE),
        );
    let cases = [
        (
            "malformed",
            "broken = \"provider-secret-sentinel".to_string(),
            NativeCapabilityReason::MalformedToml,
        ),
        (
            "actor",
            source
                .config_contents
                .replace("local-image-extension", "provider-secret-sentinel"),
            NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction,
        ),
        (
            "actor removed",
            actor_removed.to_string(),
            NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction,
        ),
        (
            "actor non-string",
            actor_non_string.to_string(),
            NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction,
        ),
        (
            "actor duplicate",
            actor_duplicate.to_string(),
            NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction,
        ),
        (
            "requires auth",
            source.config_contents.replace(
                "requires_openai_auth = false",
                "requires_openai_auth = true",
            ),
            NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction,
        ),
        (
            "bearer",
            source
                .config_contents
                .replace("same-secret", "provider-secret-sentinel"),
            NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction,
        ),
        (
            "provider identity",
            source.config_contents.replace(
                "model_provider = \"RelayOne\"",
                "model_provider = \"Other\"",
            ),
            NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction,
        ),
        (
            "provider name",
            source
                .config_contents
                .replace("name = \"OpenAI\"", "name = \"custom\""),
            NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction,
        ),
        (
            "wire api",
            source
                .config_contents
                .replace("wire_api = \"responses\"", "wire_api = \"chat\""),
            NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction,
        ),
        (
            "malformed model",
            source
                .config_contents
                .replace("model = \"gpt-5\"", "model = 7"),
            NativeCapabilityReason::MalformedModel,
        ),
        (
            "malformed base URL",
            source
                .config_contents
                .replace("base_url = \"https://relay.example/v1\"", "base_url = 7"),
            NativeCapabilityReason::MalformedBaseUrl,
        ),
        (
            "malformed context limit",
            format!("model_context_window = \"bad\"\n{}", source.config_contents),
            NativeCapabilityReason::MalformedContextLimit,
        ),
    ];

    for (label, candidate, reason) in cases {
        let payload = draft_provider_native_capability(&raw_edit_request(&source, candidate));
        assert_eq!(
            payload.status,
            NativeCapabilityDraftStatus::Blocked,
            "{label}"
        );
        assert_eq!(payload.blockers, vec![reason], "{label}");
        let operational = serde_json::to_string(&(&payload.blockers, &payload.preview)).unwrap();
        assert!(!operational.contains("provider-secret-sentinel"), "{label}");
    }

    let legacy_source = mixed_profile("raw-add-actor", "same-secret", &canonical_source("inline"));
    let mut actor_added = legacy_source
        .config_contents
        .parse::<DocumentMut>()
        .unwrap();
    actor_added["model_providers"]["RelayOne"]["http_headers"]
        .as_table_like_mut()
        .unwrap()
        .insert(MANAGED_ACTOR_HEADER_NAME, value(MANAGED_ACTOR_HEADER_VALUE));
    let added = draft_provider_native_capability(&raw_edit_request(
        &legacy_source,
        actor_added.to_string(),
    ));
    assert_eq!(added.status, NativeCapabilityDraftStatus::Blocked);
    assert_eq!(
        added.blockers,
        vec![NativeCapabilityReason::RawProviderContractChangeRequiresExplicitAction]
    );

    for invalid_source in [
        source
            .config_contents
            .replace("model_provider = \"RelayOne\"\n", ""),
        source
            .config_contents
            .replace("model_provider = \"RelayOne\"", "model_provider = 7"),
        source
            .config_contents
            .replace("[model_providers.RelayOne]", "[model_providers.Other]"),
    ] {
        let invalid_profile = mixed_profile("invalid-source", "same-secret", &invalid_source);
        let payload = draft_provider_native_capability(&raw_edit_request(
            &invalid_profile,
            invalid_source.clone(),
        ));
        assert_eq!(payload.status, NativeCapabilityDraftStatus::Blocked);
        assert_eq!(
            payload.blockers,
            vec![NativeCapabilityReason::RawProviderSourceInvalid]
        );
    }
}

#[test]
fn raw_edit_command_binds_the_source_contract_to_persisted_settings() {
    let upgraded = draft_provider_native_capability(&request(
        mixed_profile("persisted-raw", "same-secret", &canonical_source("inline")),
        CatalogMode::NativeOfficial,
        NativeCapabilityDraftAction::EnableNativePriority,
    ));
    let persisted = upgraded.draft.profile;
    let mut settings = BackendSettings::default();
    settings.relay_profiles.push(persisted.clone());
    let temp = tempfile::tempdir().unwrap();
    let settings_path = temp.path().join("settings.json");
    std::fs::write(&settings_path, serde_json::to_vec(&settings).unwrap()).unwrap();

    let ordinary_candidate = persisted
        .config_contents
        .replace("keep-root", "changed-root");
    let accepted = transform_provider_native_capability_draft_from_settings_path(
        &settings_path,
        raw_edit_request(&persisted, ordinary_candidate),
    );
    assert_eq!(accepted.status, NativeCapabilityDraftStatus::Ready);

    let actor_candidate = persisted
        .config_contents
        .replace("local-image-extension", "forged-actor");
    let forged_source = mixed_profile("persisted-raw", "same-secret", &actor_candidate);
    let rejected = transform_provider_native_capability_draft_from_settings_path(
        &settings_path,
        raw_edit_request(&forged_source, actor_candidate),
    );
    assert_eq!(rejected.status, NativeCapabilityDraftStatus::Blocked);
    assert_eq!(
        rejected.blockers,
        vec![NativeCapabilityReason::RawProviderSourceInvalid]
    );

    let missing_profile = mixed_profile("not-persisted", "same-secret", &persisted.config_contents);
    let missing = transform_provider_native_capability_draft_from_settings_path(
        &settings_path,
        raw_edit_request(&missing_profile, missing_profile.config_contents.clone()),
    );
    assert_eq!(missing.status, NativeCapabilityDraftStatus::Blocked);
    assert_eq!(
        missing.blockers,
        vec![NativeCapabilityReason::RawProviderSourceInvalid]
    );
}

#[test]
fn complete_response_keeps_provider_secret_only_in_declared_local_draft_fields() {
    let secret = "provider-secret-sentinel";
    let profile = mixed_profile(
        "secret-placement",
        secret,
        &canonical_source("inline").replace("same-secret", secret),
    );
    let payload =
        tauri::async_runtime::block_on(transform_provider_native_capability_draft(request(
            profile,
            CatalogMode::OfficialPlusCustom,
            NativeCapabilityDraftAction::Inspect,
        )));
    let serialized = serde_json::to_value(&payload).unwrap();

    assert_eq!(
        string_paths_containing(&serialized, secret),
        vec![
            "/draft/profile/configContents".to_string(),
            "/draft/structuredApiKey".to_string(),
        ]
    );
    assert_eq!(serialized["draft"]["profile"]["authContents"], "");
    assert!(
        !serde_json::to_string(&serialized["blockers"])
            .unwrap()
            .contains(secret)
    );
    assert!(
        !serde_json::to_string(&serialized["preview"])
            .unwrap()
            .contains(secret)
    );
    assert!(
        !serde_json::to_string(&serialized["inspection"])
            .unwrap()
            .contains(secret)
    );
    assert!(serialized.get("error").is_none());
}
