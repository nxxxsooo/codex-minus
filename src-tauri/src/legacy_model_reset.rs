use std::collections::BTreeSet;

use anyhow::ensure;
use codex_plus_core::settings::{BackendSettings, RelayMode, RelayProfile, RelayProtocol};

use crate::model_catalog::{CatalogMode, CatalogState, ProfileCatalogState};

pub(crate) const LEGACY_MODEL_RESET_VERSION: u32 = 1;
pub(crate) const CANONICAL_MIXED_DEFAULT_MODEL: &str = "gpt-5.6-terra";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResetProfileSummary {
    pub profile_id: String,
    pub removed_slugs: Vec<String>,
    pub previous_model: String,
    pub next_model: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct LegacyModelResetPlan {
    pub settings: BackendSettings,
    pub state: CatalogState,
    pub reset_profiles: Vec<ResetProfileSummary>,
}

pub(crate) fn plan_legacy_model_reset(
    settings: &BackendSettings,
    state: &CatalogState,
    official_visible_slugs: &BTreeSet<String>,
) -> anyhow::Result<Option<LegacyModelResetPlan>> {
    let mut next_settings = settings.clone();
    let mut next_state = state.clone();
    let mut reset_profiles = Vec::new();
    let mut changed = false;

    for profile in &mut next_settings.relay_profiles {
        let existing_state = next_state
            .profiles
            .get(&profile.id)
            .cloned()
            .unwrap_or_default();
        if !has_legacy_signal(profile, &existing_state)
            || existing_state.legacy_model_reset_version >= LEGACY_MODEL_RESET_VERSION
        {
            continue;
        }

        if !ordinary_mixed_responses(profile)
            || existing_state.mode != CatalogMode::OfficialPlusCustom
            || existing_state.external_pointer.is_some()
            || existing_state.mode_explicit
        {
            let state_entry = next_state.profiles.entry(profile.id.clone()).or_default();
            let marker_changed =
                state_entry.legacy_model_reset_version < LEGACY_MODEL_RESET_VERSION;
            state_entry.legacy_model_reset_version = LEGACY_MODEL_RESET_VERSION;
            changed |= marker_changed;
            continue;
        }

        let legacy_overlay =
            crate::model_catalog::legacy_overlay_for_current_official(&next_state, profile)?;
        let state_entry = next_state.profiles.entry(profile.id.clone()).or_default();
        let marker_changed = state_entry.legacy_model_reset_version < LEGACY_MODEL_RESET_VERSION;
        state_entry.legacy_model_reset_version = LEGACY_MODEL_RESET_VERSION;
        let removed = exact_legacy_slugs(state_entry);
        let previous_model =
            top_level_model(&profile.config_contents).unwrap_or_else(|| profile.model.clone());
        let mut next_model = previous_model.clone();

        let overlay_changed = state_entry
            .overlay
            .custom
            .iter()
            .any(|model| model.template_provenance == "legacy-model-list");
        state_entry
            .overlay
            .custom
            .retain(|model| model.template_provenance != "legacy-model-list");
        let official_override_count = state_entry.overlay.official.len();
        state_entry
            .overlay
            .official
            .retain(|slug, value| legacy_overlay.official.get(slug) != Some(value));
        let official_overrides_removed =
            state_entry.overlay.official.len() != official_override_count;

        let mut default_changed = false;
        let selected_slug = previous_model.trim();
        let selected_survives = official_visible_slugs.contains(selected_slug)
            || state_entry
                .overlay
                .custom
                .iter()
                .any(|model| model.slug.trim() == selected_slug);
        if removed.contains(selected_slug) && !selected_survives {
            ensure!(
                official_visible_slugs.contains(CANONICAL_MIXED_DEFAULT_MODEL),
                "canonical mixed default model is not in the official visible catalog"
            );
            next_model = CANONICAL_MIXED_DEFAULT_MODEL.to_string();
            profile.model = next_model.clone();
            profile.config_contents = set_top_level_model(&profile.config_contents, &next_model)?;
            default_changed = true;
        }

        let state_changed = marker_changed || overlay_changed || official_overrides_removed;
        let profile_reset = !removed.is_empty() || official_overrides_removed || default_changed;
        changed |= state_changed;
        if profile_reset {
            profile.model_list.clear();
            profile.model_windows.clear();
            reset_profiles.push(ResetProfileSummary {
                profile_id: profile.id.clone(),
                removed_slugs: removed.into_iter().collect(),
                previous_model,
                next_model,
                active: profile.id == settings.active_relay_id,
            });
        }
    }

    if !changed {
        return Ok(None);
    }

    next_state.operation_generation += 1;
    Ok(Some(LegacyModelResetPlan {
        settings: next_settings,
        state: next_state,
        reset_profiles,
    }))
}

pub(crate) fn top_level_model(config: &str) -> Option<String> {
    let document = config.parse::<toml_edit::DocumentMut>().ok()?;
    document
        .get("model")
        .and_then(toml_edit::Item::as_str)
        .map(ToString::to_string)
}

pub(crate) fn set_top_level_model(config: &str, model: &str) -> anyhow::Result<String> {
    let mut document: toml_edit::DocumentMut = config.parse()?;
    document["model"] = toml_edit::value(model);
    Ok(document.to_string())
}

fn ordinary_mixed_responses(profile: &RelayProfile) -> bool {
    profile.relay_mode == RelayMode::Official
        && profile.official_mix_api_key
        && profile.protocol == RelayProtocol::Responses
}

fn has_legacy_signal(profile: &RelayProfile, state: &ProfileCatalogState) -> bool {
    !profile.model_list.trim().is_empty()
        || has_nonempty_legacy_model_windows(&profile.model_windows)
        || state
            .overlay
            .custom
            .iter()
            .any(|model| model.template_provenance == "legacy-model-list")
}

fn has_nonempty_legacy_model_windows(model_windows: &str) -> bool {
    let value = model_windows.trim();
    if value.is_empty() {
        return false;
    }
    serde_json::from_str::<serde_json::Value>(value)
        .map(|parsed| match parsed {
            serde_json::Value::Object(entries) => !entries.is_empty(),
            serde_json::Value::Array(entries) => !entries.is_empty(),
            serde_json::Value::Null => false,
            _ => true,
        })
        .unwrap_or(true)
}

fn exact_legacy_slugs(state: &ProfileCatalogState) -> BTreeSet<String> {
    state
        .overlay
        .custom
        .iter()
        .filter(|model| model.template_provenance == "legacy-model-list")
        .map(|model| model.slug.trim().to_string())
        .filter(|slug| !slug.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use codex_plus_core::settings::{BackendSettings, RelayMode, RelayProfile, RelayProtocol};

    use super::{plan_legacy_model_reset, top_level_model};
    use crate::model_catalog::{
        CatalogMode, CatalogOverlay, CatalogState, CustomModel, OfficialOverride,
        ProfileCatalogState, compose_profile_catalog,
    };

    fn custom(slug: &str, provenance: &str) -> CustomModel {
        CustomModel {
            slug: slug.to_string(),
            display_name: slug.to_string(),
            context_window: 272_000,
            effective_context_window_percent: 95,
            visible: true,
            template_provenance: provenance.to_string(),
            ..CustomModel::default()
        }
    }

    fn eva_settings(model: &str, model_list: &str, model_windows: &str) -> BackendSettings {
        let config_contents = format!(
            r#"model = "{model}"
model_provider = "OpenAI"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://example.test/v1"
wire_api = "responses"
requires_openai_auth = true
experimental_bearer_token = "provider-key-sentinel"
http_headers = {{ "x-openai-actor-authorization" = "local-image-extension" }}
fit_marker = "keep"
"#,
        );
        BackendSettings {
            relay_profiles_enabled: true,
            active_relay_id: "eva".to_string(),
            relay_profiles: vec![RelayProfile {
                id: "eva".to_string(),
                name: "Eva|Codex".to_string(),
                model: model.to_string(),
                relay_mode: RelayMode::Official,
                official_mix_api_key: true,
                protocol: RelayProtocol::Responses,
                config_contents,
                model_list: model_list.to_string(),
                model_windows: model_windows.to_string(),
                ..RelayProfile::default()
            }],
            ..BackendSettings::default()
        }
    }

    fn state_with_profile(
        id: &str,
        mode: CatalogMode,
        mode_explicit: bool,
        custom_models: Vec<CustomModel>,
    ) -> CatalogState {
        let mut state = CatalogState::default();
        state.official = Some(crate::model_catalog::bundled_official_snapshot().unwrap());
        state.profiles.insert(
            id.to_string(),
            ProfileCatalogState {
                mode,
                mode_explicit,
                overlay: CatalogOverlay {
                    official: BTreeMap::new(),
                    custom: custom_models,
                },
                ..ProfileCatalogState::default()
            },
        );
        state
    }

    fn eva_legacy_fixture() -> (BackendSettings, CatalogState, BTreeSet<String>) {
        let settings = eva_settings("gpt-5", "gpt-5.6-terra\ngpt-5", r#"{"gpt-5":"272000"}"#);
        let state = state_with_profile(
            "eva",
            CatalogMode::OfficialPlusCustom,
            false,
            vec![custom("gpt-5", "legacy-model-list")],
        );
        let official = BTreeSet::from([
            "gpt-5.6-terra".to_string(),
            "gpt-5.6-luna".to_string(),
            "gpt-5.6-sol".to_string(),
            "gpt-5.5".to_string(),
        ]);
        (settings, state, official)
    }

    fn matrix_fixture(
        id: &str,
        mode: CatalogMode,
        explicit: bool,
        pointer: Option<&str>,
        provenance: &str,
    ) -> (BackendSettings, CatalogState, BTreeSet<String>) {
        let mut settings = eva_settings("claude-or-legacy", "claude-or-legacy", "{}");
        settings.relay_profiles[0].id = id.to_string();
        settings.active_relay_id = id.to_string();
        let mut state = state_with_profile(
            id,
            mode,
            explicit,
            vec![custom("claude-or-legacy", provenance)],
        );
        state.profiles.get_mut(id).unwrap().external_pointer = pointer.map(ToString::to_string);
        let official = BTreeSet::from(["gpt-5.6-terra".to_string()]);
        (settings, state, official)
    }

    fn without_top_level_model(config: &str) -> String {
        let mut document: toml_edit::DocumentMut = config.parse().unwrap();
        document.remove("model");
        document.to_string()
    }

    #[test]
    fn eva_implicit_legacy_gpt5_resets_to_official_terra() {
        let (settings, state, official) = eva_legacy_fixture();

        let plan = plan_legacy_model_reset(&settings, &state, &official)
            .unwrap()
            .expect("Eva legacy state must produce a reset plan");

        let profile = &plan.settings.relay_profiles[0];
        assert_eq!(
            top_level_model(&profile.config_contents).as_deref(),
            Some("gpt-5.6-terra")
        );
        assert_eq!(profile.model, "gpt-5.6-terra");
        assert!(profile.model_list.is_empty());
        assert!(profile.model_windows.is_empty());
        assert!(plan.state.profiles["eva"].overlay.custom.is_empty());
        assert_eq!(
            plan.state.profiles["eva"].mode,
            CatalogMode::OfficialPlusCustom
        );
        assert_eq!(plan.reset_profiles[0].removed_slugs, vec!["gpt-5"]);
    }

    #[test]
    fn reset_matrix_preserves_every_explicit_or_ambiguous_owner() {
        let cases = [
            (
                "explicit",
                CatalogMode::OfficialPlusCustom,
                true,
                None,
                "legacy-model-list",
                false,
            ),
            (
                "external",
                CatalogMode::External,
                false,
                Some("C:/models.json"),
                "legacy-model-list",
                false,
            ),
            (
                "implicit-native",
                CatalogMode::NativeOfficial,
                false,
                None,
                "legacy-model-list",
                false,
            ),
            (
                "implicit-custom-only",
                CatalogMode::CustomOnly,
                false,
                None,
                "legacy-model-list",
                false,
            ),
            (
                "user-created",
                CatalogMode::OfficialPlusCustom,
                false,
                None,
                "user-created",
                false,
            ),
            (
                "unknown",
                CatalogMode::OfficialPlusCustom,
                false,
                None,
                "",
                false,
            ),
            (
                "legacy",
                CatalogMode::OfficialPlusCustom,
                false,
                None,
                "legacy-model-list",
                true,
            ),
        ];

        for (id, mode, explicit, pointer, provenance, reset_expected) in cases {
            let (settings, state, official) =
                matrix_fixture(id, mode, explicit, pointer, provenance);
            let plan = plan_legacy_model_reset(&settings, &state, &official).unwrap();
            assert_eq!(
                plan.as_ref()
                    .is_some_and(|plan| !plan.reset_profiles.is_empty()),
                reset_expected,
                "{id}"
            );
            if !reset_expected {
                let (next_settings, next_state) = plan
                    .map(|plan| (plan.settings, plan.state))
                    .unwrap_or((settings.clone(), state));
                assert_eq!(
                    next_state.profiles[id].overlay.custom[0].slug,
                    "claude-or-legacy"
                );
                assert_eq!(
                    next_settings.relay_profiles[0].model_list,
                    settings.relay_profiles[0].model_list
                );
                assert_eq!(
                    next_settings.relay_profiles[0].model_windows,
                    settings.relay_profiles[0].model_windows
                );
                assert_eq!(
                    next_settings.relay_profiles[0].model,
                    settings.relay_profiles[0].model
                );
            }
        }
    }

    #[test]
    fn ineligible_profiles_marker_preserve_invalid_dormant_legacy_fields() {
        let invalid_model_list = "x".repeat(161);
        let invalid_model_windows = "{not-json";
        let cases = [
            (
                "explicit",
                CatalogMode::OfficialPlusCustom,
                true,
                None,
                true,
                RelayProtocol::Responses,
                RelayMode::Official,
            ),
            (
                "external",
                CatalogMode::External,
                false,
                Some("C:/models.json"),
                true,
                RelayProtocol::Responses,
                RelayMode::Official,
            ),
            (
                "external-pointer",
                CatalogMode::OfficialPlusCustom,
                false,
                Some("C:/models.json"),
                true,
                RelayProtocol::Responses,
                RelayMode::Official,
            ),
            (
                "implicit-native",
                CatalogMode::NativeOfficial,
                false,
                None,
                true,
                RelayProtocol::Responses,
                RelayMode::Official,
            ),
            (
                "implicit-custom-only",
                CatalogMode::CustomOnly,
                false,
                None,
                true,
                RelayProtocol::Responses,
                RelayMode::Official,
            ),
            (
                "pure-oauth",
                CatalogMode::OfficialPlusCustom,
                false,
                None,
                false,
                RelayProtocol::Responses,
                RelayMode::Official,
            ),
            (
                "chat-completions",
                CatalogMode::OfficialPlusCustom,
                false,
                None,
                true,
                RelayProtocol::ChatCompletions,
                RelayMode::Official,
            ),
            (
                "pure-api",
                CatalogMode::OfficialPlusCustom,
                false,
                None,
                true,
                RelayProtocol::Responses,
                RelayMode::PureApi,
            ),
        ];

        for (id, mode, explicit, pointer, mixed, protocol, relay_mode) in cases {
            let mut settings =
                eva_settings("legacy-row", &invalid_model_list, invalid_model_windows);
            settings.relay_profiles[0].id = id.to_string();
            settings.relay_profiles[0].official_mix_api_key = mixed;
            settings.relay_profiles[0].protocol = protocol;
            settings.relay_profiles[0].relay_mode = relay_mode;
            settings.active_relay_id = id.to_string();
            let mut state = state_with_profile(
                id,
                mode,
                explicit,
                vec![custom("legacy-row", "legacy-model-list")],
            );
            state.profiles.get_mut(id).unwrap().external_pointer = pointer.map(ToString::to_string);
            let official = BTreeSet::from(["gpt-5.6-terra".to_string()]);

            let plan = plan_legacy_model_reset(&settings, &state, &official)
                .unwrap_or_else(|error| panic!("{id} parsed dormant legacy data: {error}"))
                .expect("the preserved profile must record its one-time marker");
            let next_profile = &plan.settings.relay_profiles[0];
            let next_state = &plan.state.profiles[id];

            assert!(plan.reset_profiles.is_empty(), "{id}");
            assert_eq!(next_profile.model_list, invalid_model_list, "{id}");
            assert_eq!(next_profile.model_windows, invalid_model_windows, "{id}");
            assert_eq!(next_profile.model, "legacy-row", "{id}");
            assert_eq!(
                serde_json::to_value(next_profile).unwrap(),
                serde_json::to_value(&settings.relay_profiles[0]).unwrap(),
                "{id}",
            );
            let mut expected_state = state.profiles[id].clone();
            expected_state.legacy_model_reset_version = super::LEGACY_MODEL_RESET_VERSION;
            assert_eq!(
                serde_json::to_value(next_state).unwrap(),
                serde_json::to_value(expected_state).unwrap(),
                "{id}",
            );
            assert_eq!(
                next_state.legacy_model_reset_version,
                super::LEGACY_MODEL_RESET_VERSION,
                "{id}",
            );
        }
    }

    #[test]
    fn eligible_profile_fails_closed_for_invalid_legacy_reconstruction() {
        let invalid_model_list = "x".repeat(161);
        let settings = eva_settings("legacy-row", &invalid_model_list, "{}");
        let state = state_with_profile(
            "eva",
            CatalogMode::OfficialPlusCustom,
            false,
            vec![custom("legacy-row", "legacy-model-list")],
        );
        let official = BTreeSet::from(["gpt-5.6-terra".to_string()]);

        let error = match plan_legacy_model_reset(&settings, &state, &official) {
            Err(error) => error,
            Ok(_) => panic!("eligible invalid legacy data was accepted"),
        };

        assert!(error.to_string().contains("model slug is too long"));
        assert_eq!(state.profiles["eva"].legacy_model_reset_version, 0);
        assert_eq!(settings.relay_profiles[0].model_list, invalid_model_list);
        assert_eq!(settings.relay_profiles[0].model_windows, "{}");
    }

    #[test]
    fn eligible_profile_rejects_untrustworthy_model_windows_without_mutation() {
        let invalid = [
            ("malformed-json", "{not-json"),
            ("null", "null"),
            ("array", "[]"),
            ("number-scalar", "272000"),
            ("boolean-scalar", "true"),
            ("string-scalar", r#""272000""#),
            ("non-string-number", r#"{"gpt-5":272000}"#),
            ("non-string-boolean", r#"{"gpt-5":true}"#),
            ("non-string-null", r#"{"gpt-5":null}"#),
            ("non-string-array", r#"{"gpt-5":[]}"#),
            ("non-string-object", r#"{"gpt-5":{}}"#),
            ("empty", r#"{"gpt-5":""}"#),
            ("whitespace", r#"{"gpt-5":"   "}"#),
            ("non-numeric", r#"{"gpt-5":"window-secret-sentinel"}"#),
            ("legacy-unit-token", r#"{"gpt-5":"1M"}"#),
            ("zero", r#"{"gpt-5":"0"}"#),
            ("negative", r#"{"gpt-5":"-1"}"#),
            ("invalid-extra-entry", r#"{"gpt-5":"272000","unused":"0"}"#),
            ("overflow", r#"{"gpt-5":"18446744073709551616"}"#),
        ];
        let official = BTreeSet::from(["gpt-5.6-terra".to_string()]);

        for (label, model_windows) in invalid {
            let settings = eva_settings("gpt-5", "gpt-5", model_windows);
            let state = state_with_profile(
                "eva",
                CatalogMode::OfficialPlusCustom,
                false,
                vec![custom("gpt-5", "legacy-model-list")],
            );
            let settings_before = serde_json::to_vec(&settings).unwrap();
            let state_before = serde_json::to_vec(&state).unwrap();

            let error = match plan_legacy_model_reset(&settings, &state, &official) {
                Err(error) => error,
                Ok(_) => panic!("{label}: eligible invalid model windows were accepted"),
            };

            assert!(
                error.to_string().contains("legacy model windows"),
                "{label}: {error}"
            );
            assert!(
                !error.to_string().contains("window-secret-sentinel"),
                "{label}: {error}"
            );
            assert_eq!(
                serde_json::to_vec(&settings).unwrap(),
                settings_before,
                "{label}"
            );
            assert_eq!(serde_json::to_vec(&state).unwrap(), state_before, "{label}");
        }
    }

    #[test]
    fn eligible_profile_accepts_blank_empty_and_positive_model_windows() {
        let valid = [
            ("blank", ""),
            ("whitespace-blank", "   "),
            ("empty-map", "{}"),
            ("positive-map", r#"{"gpt-5":" 272000 "}"#),
        ];
        let official = BTreeSet::from(["gpt-5.6-terra".to_string()]);

        for (label, model_windows) in valid {
            let settings = eva_settings("gpt-5", "gpt-5", model_windows);
            let state = state_with_profile(
                "eva",
                CatalogMode::OfficialPlusCustom,
                false,
                vec![custom("gpt-5", "legacy-model-list")],
            );

            let plan = plan_legacy_model_reset(&settings, &state, &official)
                .unwrap_or_else(|error| panic!("{label}: {error}"))
                .expect("eligible legacy state must reset");

            assert_eq!(plan.reset_profiles.len(), 1, "{label}");
            assert_eq!(plan.reset_profiles[0].removed_slugs, ["gpt-5"], "{label}");
            assert!(
                plan.settings.relay_profiles[0].model_windows.is_empty(),
                "{label}"
            );
        }
    }

    #[test]
    fn reset_removes_only_exact_legacy_derived_official_overrides() {
        let settings = eva_settings(
            "gpt-5",
            "gpt-5.4\ngpt-5.6-luna\ngpt-5",
            r#"{"gpt-5.4":"300000","gpt-5.6-luna":"310000"}"#,
        );
        let mut state = state_with_profile(
            "eva",
            CatalogMode::OfficialPlusCustom,
            false,
            vec![custom("gpt-5", "legacy-model-list")],
        );
        let exact_hidden_legacy = OfficialOverride {
            context_window: Some(300_000),
            order: Some(0),
            ..OfficialOverride::default()
        };
        let modified_legacy = OfficialOverride {
            context_window: Some(310_000),
            order: Some(1),
            visible: Some(false),
            ..OfficialOverride::default()
        };
        let user_added = OfficialOverride {
            context_window: Some(512_000),
            visible: Some(true),
            ..OfficialOverride::default()
        };
        let official_overlay = &mut state.profiles.get_mut("eva").unwrap().overlay.official;
        official_overlay.insert("gpt-5.4".to_string(), exact_hidden_legacy);
        official_overlay.insert("gpt-5.6-luna".to_string(), modified_legacy.clone());
        official_overlay.insert("gpt-5.6-sol".to_string(), user_added.clone());
        let visible_official = BTreeSet::from([
            "gpt-5.6-terra".to_string(),
            "gpt-5.6-luna".to_string(),
            "gpt-5.6-sol".to_string(),
        ]);

        let plan = plan_legacy_model_reset(&settings, &state, &visible_official)
            .unwrap()
            .unwrap();
        let official_overlay = &plan.state.profiles["eva"].overlay.official;

        assert!(!official_overlay.contains_key("gpt-5.4"));
        assert_eq!(official_overlay.get("gpt-5.6-luna"), Some(&modified_legacy));
        assert_eq!(official_overlay.get("gpt-5.6-sol"), Some(&user_added));
    }

    #[test]
    fn adjacent_user_custom_survives_legacy_row_removal() {
        let settings = eva_settings("gpt-5", "gpt-5\nclaude-opus-5", "{}");
        let mut state = state_with_profile(
            "eva",
            CatalogMode::OfficialPlusCustom,
            false,
            vec![custom("gpt-5", "legacy-model-list")],
        );
        state
            .profiles
            .get_mut("eva")
            .unwrap()
            .overlay
            .custom
            .push(custom("claude-opus-5", "user-created"));
        let official = BTreeSet::from(["gpt-5.6-terra".to_string()]);

        let plan = plan_legacy_model_reset(&settings, &state, &official)
            .unwrap()
            .unwrap();

        let custom = &plan.state.profiles["eva"].overlay.custom;
        assert_eq!(custom.len(), 1);
        assert_eq!(custom[0].slug, "claude-opus-5");
        assert_eq!(custom[0].template_provenance, "user-created");
    }

    #[test]
    fn valid_official_or_retained_custom_default_never_changes() {
        let official = BTreeSet::from(["gpt-5.6-terra".to_string(), "gpt-5.6-luna".to_string()]);
        for default_model in ["gpt-5.6-luna", "claude-opus-5"] {
            let settings = eva_settings(default_model, "gpt-5\nclaude-opus-5", "{}");
            let mut state = state_with_profile(
                "eva",
                CatalogMode::OfficialPlusCustom,
                false,
                vec![custom("gpt-5", "legacy-model-list")],
            );
            state
                .profiles
                .get_mut("eva")
                .unwrap()
                .overlay
                .custom
                .push(custom("claude-opus-5", "user-created"));

            let plan = plan_legacy_model_reset(&settings, &state, &official)
                .unwrap()
                .unwrap();

            assert_eq!(
                top_level_model(&plan.settings.relay_profiles[0].config_contents).as_deref(),
                Some(default_model),
            );
        }
    }

    #[test]
    fn removed_legacy_shadow_of_visible_official_default_keeps_that_default() {
        let settings = eva_settings("gpt-5.6-sol", "gpt-5.6-sol", "{}");
        let state = state_with_profile(
            "eva",
            CatalogMode::OfficialPlusCustom,
            false,
            vec![custom("gpt-5.6-sol", "legacy-model-list")],
        );
        let official = BTreeSet::from(["gpt-5.6-sol".to_string(), "gpt-5.6-terra".to_string()]);

        let plan = plan_legacy_model_reset(&settings, &state, &official)
            .unwrap()
            .unwrap();
        let profile = &plan.settings.relay_profiles[0];
        assert_eq!(profile.model, "gpt-5.6-sol");
        assert_eq!(
            top_level_model(&profile.config_contents).as_deref(),
            Some("gpt-5.6-sol")
        );
        assert!(plan.state.profiles["eva"].overlay.custom.is_empty());
        let catalog =
            compose_profile_catalog(&plan.state, profile, &plan.state.profiles["eva"]).unwrap();
        assert!(
            catalog["models"]
                .as_array()
                .unwrap()
                .iter()
                .any(|model| { model["slug"].as_str() == Some("gpt-5.6-sol") })
        );
    }

    #[test]
    fn removed_legacy_duplicate_keeps_a_retained_custom_default_with_the_same_slug() {
        let settings = eva_settings("claude-opus-5", "claude-opus-5", "{}");
        let state = state_with_profile(
            "eva",
            CatalogMode::OfficialPlusCustom,
            false,
            vec![
                custom("claude-opus-5", "legacy-model-list"),
                custom("claude-opus-5", "user-created"),
            ],
        );
        let official = BTreeSet::from(["gpt-5.6-terra".to_string()]);

        let plan = plan_legacy_model_reset(&settings, &state, &official)
            .unwrap()
            .unwrap();
        let profile = &plan.settings.relay_profiles[0];
        assert_eq!(profile.model, "claude-opus-5");
        assert_eq!(
            top_level_model(&profile.config_contents).as_deref(),
            Some("claude-opus-5")
        );
        assert_eq!(plan.state.profiles["eva"].overlay.custom.len(), 1);
        assert_eq!(
            plan.state.profiles["eva"].overlay.custom[0].template_provenance,
            "user-created"
        );
    }

    #[test]
    fn second_reset_is_a_byte_identical_noop() {
        let (settings, state, official) = eva_legacy_fixture();
        let first = plan_legacy_model_reset(&settings, &state, &official)
            .unwrap()
            .unwrap();
        let settings_bytes = serde_json::to_vec(&first.settings).unwrap();
        let state_bytes = serde_json::to_vec(&first.state).unwrap();

        let second = plan_legacy_model_reset(&first.settings, &first.state, &official).unwrap();

        assert!(second.is_none());
        assert_eq!(serde_json::to_vec(&first.settings).unwrap(), settings_bytes);
        assert_eq!(serde_json::to_vec(&first.state).unwrap(), state_bytes);
    }

    #[test]
    fn profile_without_legacy_signals_is_a_byte_identical_noop() {
        let settings = eva_settings("gpt-5.6-terra", "", "{}");
        let state = state_with_profile(
            "eva",
            CatalogMode::OfficialPlusCustom,
            false,
            vec![custom("claude-opus-5", "user-created")],
        );
        let official = BTreeSet::from(["gpt-5.6-terra".to_string()]);

        assert!(
            plan_legacy_model_reset(&settings, &state, &official)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn eva_reset_catalog_contains_terra_and_not_gpt5() {
        let (settings, state, official) = eva_legacy_fixture();
        let original_config = settings.relay_profiles[0].config_contents.clone();
        let plan = plan_legacy_model_reset(&settings, &state, &official)
            .unwrap()
            .unwrap();
        let catalog = compose_profile_catalog(
            &plan.state,
            &plan.settings.relay_profiles[0],
            &plan.state.profiles["eva"],
        )
        .unwrap();
        let slugs = catalog["models"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|model| model["slug"].as_str())
            .collect::<Vec<_>>();
        assert!(slugs.contains(&"gpt-5.6-terra"));
        assert!(!slugs.contains(&"gpt-5"));
        assert_eq!(
            without_top_level_model(&plan.settings.relay_profiles[0].config_contents),
            without_top_level_model(&original_config),
        );
    }
}
