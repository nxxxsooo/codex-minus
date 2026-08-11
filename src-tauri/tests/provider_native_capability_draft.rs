use codex_minus_lib::provider_native_capability::{
    CatalogMode, NativeCapabilityDraftAction, NativeCapabilityDraftConfirmation,
    NativeCapabilityDraftStatus, NativeCapabilityReason, ProviderNativeCapabilityDraftRequest,
    draft_provider_native_capability, transform_provider_native_capability_draft,
};
use codex_plus_core::settings::{RelayMode, RelayProfile, RelayProtocol};
use toml_edit::{DocumentMut, Item};

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
        confirmations: Vec::new(),
        replacement_provider_id: None,
    }
}

fn parsed(
    payload: &codex_minus_lib::provider_native_capability::ProviderNativeCapabilityDraftPayload,
) -> DocumentMut {
    payload.profile.config_contents.parse().unwrap()
}

fn provider<'a>(document: &'a DocumentMut, provider_id: &str) -> &'a dyn toml_edit::TableLike {
    document["model_providers"][provider_id]
        .as_table_like()
        .unwrap()
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
        assert_eq!(payload.catalog_mode, CatalogMode::OfficialPlusCustom);
        assert_eq!(payload.profile.relay_mode, RelayMode::Official);
        assert!(payload.profile.official_mix_api_key);
        assert_eq!(payload.profile.protocol, RelayProtocol::Responses);
        assert_eq!(payload.structured_api_key, "same-secret");

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
    assert_eq!(blocked.profile.config_contents, original.config_contents);
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
        assert_eq!(rejected.profile.config_contents, source, "{invalid}");
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
    assert_eq!(blocked.profile.config_contents, original.config_contents);
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
    assert_eq!(rejected.profile.config_contents, duplicate);
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
    assert_eq!(blocked.profile.config_contents, original.config_contents);
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
    assert_eq!(synchronized.structured_api_key, "raw-secret");
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
    assert_eq!(blocked.profile.config_contents, original.config_contents);

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
    assert!(synchronized.structured_api_key.is_empty());
    assert!(!synchronized.profile.config_contents.contains("raw-secret"));
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
        assert_eq!(payload.profile.relay_mode, expected_mode);
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
            unconfirmed.profile.config_contents,
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
        assert_eq!(confirmed.profile.relay_mode, RelayMode::Official);
        assert!(!confirmed.profile.official_mix_api_key);
        assert!(confirmed.structured_api_key.is_empty());
        assert_eq!(
            confirmed.catalog_mode,
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
    assert_eq!(unconfirmed.profile.protocol, RelayProtocol::Responses);
    assert_eq!(
        unconfirmed.profile.config_contents,
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
    assert_eq!(confirmed.profile.protocol, RelayProtocol::ChatCompletions);
    assert!(confirmed.preview.capability_loss);
    assert_eq!(
        provider(&parsed(&confirmed), "RelayOne")
            .get("arbitrary")
            .and_then(Item::as_str),
        Some("keep-provider")
    );
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
    assert_eq!(blocked.catalog_mode, CatalogMode::External);
    assert_eq!(blocked.profile.config_contents, original.config_contents);
}

#[test]
fn revisioned_command_echoes_revision_and_cannot_mutate_any_filesystem_store() {
    let temp = tempfile::tempdir().unwrap();
    let paths = [
        temp.path().join("settings.json"),
        temp.path().join("model-catalog-state.json"),
        temp.path().join("config.toml"),
        temp.path().join("auth.json"),
    ];
    for (index, path) in paths.iter().enumerate() {
        std::fs::write(path, format!("sentinel-{index}")).unwrap();
    }
    let before = paths
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect::<Vec<_>>();
    let entries_before = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();

    let request = request(
        mixed_profile("pure", "same-secret", &canonical_source("inline")),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::Inspect,
    );
    let payload = transform_provider_native_capability_draft(request);
    assert_eq!(payload.draft_revision, 41);
    assert_eq!(payload.status, NativeCapabilityDraftStatus::Ready);
    assert_eq!(
        serde_json::to_value(&payload).unwrap()["draftRevision"],
        serde_json::json!(41)
    );
    for (path, expected) in paths.iter().zip(before) {
        assert_eq!(
            std::fs::read(path).unwrap(),
            expected,
            "mutated {}",
            path.display()
        );
    }
    assert_eq!(
        std::fs::read_dir(temp.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>(),
        entries_before
    );
}

#[test]
fn zero_revision_is_rejected_without_transforming_the_draft() {
    let original = mixed_profile("zero", "same-secret", &canonical_source("inline"));
    let mut request = request(
        original.clone(),
        CatalogMode::OfficialPlusCustom,
        NativeCapabilityDraftAction::EnableNativePriority,
    );
    request.draft_revision = 0;
    let payload = draft_provider_native_capability(&request);
    assert_eq!(payload.status, NativeCapabilityDraftStatus::Blocked);
    assert_eq!(
        payload.blockers,
        vec![NativeCapabilityReason::InvalidDraftRevision]
    );
    assert_eq!(payload.profile.config_contents, original.config_contents);
}
