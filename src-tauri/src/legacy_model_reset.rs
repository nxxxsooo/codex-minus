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

        let state_entry = next_state.profiles.entry(profile.id.clone()).or_default();
        if !ordinary_mixed_responses(profile)
            || state_entry.mode == CatalogMode::External
            || state_entry.external_pointer.is_some()
            || state_entry.mode_explicit
        {
            state_entry.legacy_model_reset_version = LEGACY_MODEL_RESET_VERSION;
            changed = true;
            continue;
        }

        let removed = exact_legacy_slugs(state_entry);
        let previous_model =
            top_level_model(&profile.config_contents).unwrap_or_else(|| profile.model.clone());
        let mut next_model = previous_model.clone();

        state_entry
            .overlay
            .custom
            .retain(|model| model.template_provenance != "legacy-model-list");
        state_entry.overlay.official.clear();
        state_entry.legacy_model_reset_version = LEGACY_MODEL_RESET_VERSION;
        profile.model_list.clear();
        profile.model_windows.clear();

        if removed.contains(&previous_model) {
            ensure!(
                official_visible_slugs.contains(CANONICAL_MIXED_DEFAULT_MODEL),
                "canonical mixed default model is not in the official visible catalog"
            );
            next_model = CANONICAL_MIXED_DEFAULT_MODEL.to_string();
            profile.model = next_model.clone();
            profile.config_contents = set_top_level_model(&profile.config_contents, &next_model)?;
        }

        changed = true;
        reset_profiles.push(ResetProfileSummary {
            profile_id: profile.id.clone(),
            removed_slugs: removed.into_iter().collect(),
            previous_model,
            next_model,
            active: profile.id == settings.active_relay_id,
        });
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
        || !profile.model_windows.trim().is_empty()
        || state
            .overlay
            .custom
            .iter()
            .any(|model| model.template_provenance == "legacy-model-list")
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
        CatalogMode, CatalogOverlay, CatalogState, CustomModel, ProfileCatalogState,
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
}
