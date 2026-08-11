use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
#[cfg(any(target_os = "macos", test))]
use std::process::Command;
use std::process::Stdio;
use std::sync::{Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, ensure};
use base64::Engine;
use codex_plus_core::settings::{BackendSettings, RelayMode, RelayProfile, SettingsStore};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::commands::{CommandResult, discover_target_codex_cli};
use crate::live_state::{self, FileMutation};

const STATE_VERSION: u32 = 2;
const STATE_FILE: &str = "model-catalog-state.json";
const GENERATED_DIR: &str = "model-catalogs";
const GENERATED_PREFIX: &str = "codex-minus-";
const REFRESH_TIMEOUT: Duration = Duration::from_secs(45);
const CAPABILITY_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_COMMAND_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
const MIN_SUPPORTED_CLI: &str = "0.147.0-alpha.1";
#[cfg(any(target_os = "macos", test))]
const OPENAI_MAC_TEAM_IDS: &[&str] = &["2DC432GLL2"];

#[derive(Debug, Clone, PartialEq, Eq)]
struct TargetVerificationCacheKey {
    app_path: PathBuf,
    app_len: u64,
    app_modified: Option<SystemTime>,
    cli_path: PathBuf,
    cli_len: u64,
    cli_modified: Option<SystemTime>,
}

static TARGET_VERIFICATION_CACHE: OnceLock<
    StdMutex<Option<(TargetVerificationCacheKey, TargetVerificationCacheValue)>>,
> = OnceLock::new();

#[derive(Debug, Clone)]
enum TargetVerificationCacheValue {
    Verified(VerifiedTargetIdentity),
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CatalogState {
    pub version: u32,
    pub scope_salt: String,
    pub official: Option<OfficialSnapshot>,
    pub target: Option<VerifiedTargetIdentity>,
    pub profiles: BTreeMap<String, ProfileCatalogState>,
    pub last_diff: CatalogDiff,
    pub operation_generation: u64,
}

impl Default for CatalogState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            scope_salt: new_scope_salt(),
            official: None,
            target: None,
            profiles: BTreeMap::new(),
            last_diff: CatalogDiff::default(),
            operation_generation: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OfficialSnapshot {
    pub source: String,
    pub fetched_at_ms: i64,
    pub etag: Option<String>,
    pub client_version: String,
    pub content_hash: String,
    pub scope_hash: String,
    pub raw_catalog: Value,
    pub visible_count: usize,
    pub total_count: usize,
}

impl Default for OfficialSnapshot {
    fn default() -> Self {
        Self {
            source: "target-cli".to_string(),
            fetched_at_ms: 0,
            etag: None,
            client_version: String::new(),
            content_hash: String::new(),
            scope_hash: String::new(),
            raw_catalog: json!({ "models": [] }),
            visible_count: 0,
            total_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct VerifiedTargetIdentity {
    pub app_path: String,
    pub cli_path: String,
    pub client_version: String,
    pub publisher: String,
    pub identity_hash: String,
    pub trusted: bool,
    pub capability_available: bool,
    pub capability_message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogMode {
    #[default]
    NativeOfficial,
    OfficialPlusCustom,
    CustomOnly,
    External,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum UpstreamTopology {
    #[default]
    Direct,
    ServerSideComposite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileCatalogState {
    pub mode: CatalogMode,
    pub mode_explicit: bool,
    pub upstream_topology: UpstreamTopology,
    pub overlay: CatalogOverlay,
    pub external_pointer: Option<String>,
    pub generated_path: Option<String>,
    pub generated_hash: Option<String>,
    pub generation: u64,
    pub restart_required: bool,
    pub action_required: Option<String>,
    pub provider_evidence: Option<ProviderEvidence>,
}

impl Default for ProfileCatalogState {
    fn default() -> Self {
        Self {
            mode: CatalogMode::NativeOfficial,
            mode_explicit: false,
            upstream_topology: UpstreamTopology::Direct,
            overlay: CatalogOverlay::default(),
            external_pointer: None,
            generated_path: None,
            generated_hash: None,
            generation: 0,
            restart_required: false,
            action_required: None,
            provider_evidence: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CatalogOverlay {
    pub official: BTreeMap<String, OfficialOverride>,
    pub custom: Vec<CustomModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct OfficialOverride {
    pub display_name: Option<String>,
    pub visible: Option<bool>,
    pub context_window: Option<u64>,
    pub effective_context_window_percent: Option<u8>,
    pub order: Option<i64>,
    pub supported_reasoning_levels: Option<Vec<ReasoningLevel>>,
    pub default_reasoning_level: Option<String>,
    pub supported_tools: Option<Vec<String>>,
    pub tool_capabilities: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct ReasoningLevel {
    pub effort: String,
    pub description: String,
}

impl Default for ReasoningLevel {
    fn default() -> Self {
        Self {
            effort: String::new(),
            description: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CustomModel {
    pub slug: String,
    pub display_name: String,
    pub context_window: u64,
    pub effective_context_window_percent: u8,
    pub visible: bool,
    pub order: i64,
    pub supported_reasoning_levels: Vec<ReasoningLevel>,
    pub default_reasoning_level: Option<String>,
    pub supported_tools: Vec<String>,
    pub tool_capabilities: Option<Value>,
    pub template_provenance: String,
}

impl Default for CustomModel {
    fn default() -> Self {
        Self {
            slug: String::new(),
            display_name: String::new(),
            context_window: 272_000,
            effective_context_window_percent: 100,
            visible: true,
            order: 0,
            supported_reasoning_levels: Vec::new(),
            default_reasoning_level: None,
            supported_tools: Vec::new(),
            tool_capabilities: None,
            template_provenance: "pinned-upstream-bundled-template".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderEvidence {
    pub fetched_at_ms: i64,
    pub endpoint: String,
    pub reported_slugs: Vec<String>,
    pub candidate_slugs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CatalogDiff {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub removed: Vec<String>,
    pub collisions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogStatusPayload {
    pub state_path: String,
    pub source: String,
    pub target_client_version: Option<String>,
    pub target_cli_path: Option<String>,
    pub target_trusted: bool,
    pub refresh_available: bool,
    pub last_successful_refresh_at_ms: Option<i64>,
    pub visible_count: usize,
    pub total_count: usize,
    pub freshness: String,
    pub credential_action: Option<String>,
    pub diff: CatalogDiff,
    pub official_models: Vec<OfficialModelSummary>,
    pub profiles: Vec<ProfileCatalogSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialModelSummary {
    pub slug: String,
    pub display_name: String,
    pub visible: bool,
    pub context_window: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCatalogSummary {
    pub profile_id: String,
    pub mode: CatalogMode,
    pub mode_explicit: bool,
    pub upstream_topology: UpstreamTopology,
    pub managed_available: bool,
    pub context_conflicts: Vec<String>,
    pub external_pointer: Option<String>,
    pub generated_path: Option<String>,
    pub effective_hash: Option<String>,
    pub restart_required: bool,
    pub action_required: Option<String>,
    pub official_override_count: usize,
    pub custom_count: usize,
    pub provider_evidence_at_ms: Option<i64>,
    pub provider_reported_count: usize,
    pub custom_candidates: Vec<String>,
    pub provider_reported_slugs: Vec<String>,
    pub overlay: CatalogOverlay,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProfileCatalogRequest {
    pub profile_id: String,
    pub mode: CatalogMode,
    #[serde(default)]
    pub mode_explicit: bool,
    #[serde(default)]
    pub upstream_topology: UpstreamTopology,
    #[serde(default)]
    pub confirm_context_cleanup: bool,
    #[serde(default)]
    pub overlay: CatalogOverlay,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptCatalogRequest {
    pub profile_id: String,
    #[serde(default)]
    pub commit: bool,
    #[serde(default)]
    pub expected_source_hash: Option<String>,
    #[serde(default)]
    pub expected_target_client_version: Option<String>,
    #[serde(default)]
    pub expected_version_status: Option<String>,
    #[serde(default)]
    pub accept_version_mismatch: bool,
    #[serde(default)]
    pub confirm_context_cleanup: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionPreviewPayload {
    pub profile_id: String,
    pub source_path: String,
    pub official_override_count: usize,
    pub custom_models: Vec<CustomModel>,
    pub collisions: Vec<String>,
    pub source_hash: String,
    pub catalog_client_version: Option<String>,
    pub target_client_version: String,
    pub version_status: String,
    pub committed: bool,
}

#[derive(Debug, Clone)]
pub struct ActiveCatalogPlan {
    pub config_contents: String,
    pub mutations: Vec<FileMutation>,
}

#[derive(Debug, Clone)]
struct AuthSnapshot {
    generation_hash: String,
    scope_identity: String,
    projection: Value,
}

fn auth_snapshot_matches(expected: &AuthSnapshot, current: &AuthSnapshot) -> bool {
    expected.generation_hash == current.generation_hash
        && expected.scope_identity == current.scope_identity
}

#[tauri::command]
pub async fn model_catalog_status() -> CommandResult<CatalogStatusPayload> {
    tauri::async_runtime::spawn_blocking(model_catalog_status_blocking)
        .await
        .expect("blocking command panicked")
}

#[tauri::command]
pub async fn refresh_official_model_catalog() -> CommandResult<CatalogStatusPayload> {
    tauri::async_runtime::spawn_blocking(refresh_official_model_catalog_blocking)
        .await
        .expect("blocking command panicked")
}

#[tauri::command]
pub async fn save_profile_catalog(
    request: SaveProfileCatalogRequest,
) -> CommandResult<CatalogStatusPayload> {
    tauri::async_runtime::spawn_blocking(move || save_profile_catalog_blocking(request))
        .await
        .expect("blocking command panicked")
}

#[tauri::command]
pub async fn adopt_external_model_catalog(
    request: AdoptCatalogRequest,
) -> CommandResult<AdoptionPreviewPayload> {
    tauri::async_runtime::spawn_blocking(move || adopt_external_model_catalog_blocking(request))
        .await
        .expect("blocking command panicked")
}

fn model_catalog_status_blocking() -> CommandResult<CatalogStatusPayload> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let result = (|| -> anyhow::Result<CatalogStatusPayload> {
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        live_state::recover_locked()?;
        let settings = sanitized_settings()?;
        let state = load_and_migrate_state(&settings, &home)?;
        status_payload(&state, &settings, &home)
    })();
    command_result(result, "模型目录状态已加载。", "模型目录状态读取失败")
}

fn refresh_official_model_catalog_blocking() -> CommandResult<CatalogStatusPayload> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let result = (|| -> anyhow::Result<CatalogStatusPayload> {
        let network = crate::network_policy::resolve_current_policy()?;
        network.ensure_supported()?;
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        live_state::recover_locked()?;
        let settings = sanitized_settings()?;
        let mut state = load_and_migrate_state(&settings, &home)?;
        let target = verify_target_cli_fresh()?;
        ensure!(target.trusted, "目标 Codex CLI 未通过平台信任校验");
        ensure!(target.capability_available, "{}", target.capability_message);
        let auth_path = home.join("auth.json");
        let auth = snapshot_live_auth(&auth_path, &state.scope_salt)?;
        let raw = run_isolated_refresh(&target, &auth.projection, &network)?;
        let cache = raw.cache;
        let output = raw.output;
        validate_catalog(&output, &target.client_version)?;
        validate_refresh_cache(&cache, &target.client_version)?;
        validate_refresh_cache_matches_output(&output, &cache)?;
        let current_auth = snapshot_live_auth(&auth_path, &state.scope_salt)?;
        ensure!(
            auth_snapshot_matches(&auth, &current_auth),
            "官方客户端认证在刷新期间发生变化，已丢弃结果"
        );

        let scope_hash = hash_text(&format!("{}:{}", state.scope_salt, auth.scope_identity));
        let content_hash = canonical_json_hash(&output)?;
        let mut diff = diff_catalogs(
            state.official.as_ref().map(|item| &item.raw_catalog),
            &output,
        )?;
        let refreshed_slugs = catalog_slugs(&output)?;
        diff.collisions = normalize_slugs(state.profiles.values().flat_map(|profile| {
            profile
                .overlay
                .custom
                .iter()
                .filter(|custom| refreshed_slugs.contains(&custom.slug))
                .map(|custom| custom.slug.clone())
                .collect::<Vec<_>>()
        }));
        let (visible_count, total_count) = catalog_counts(&output)?;
        state.operation_generation = state.operation_generation.saturating_add(1);
        state.target = Some(target.clone());
        state.last_diff = diff;
        state.official = Some(OfficialSnapshot {
            source: "verified-target-cli".to_string(),
            fetched_at_ms: now_ms(),
            etag: cache
                .get("etag")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            client_version: target.client_version.clone(),
            content_hash,
            scope_hash,
            raw_catalog: output,
            visible_count,
            total_count,
        });

        let mut mutations = materialize_inactive_profiles(&mut state, &settings, &home)?;
        let live_config = fs::read_to_string(home.join("config.toml")).unwrap_or_default();
        match plan_active_profile_with_state(&home, &settings, &live_config, &mut state, false) {
            Ok(plan) => {
                mutations.extend(plan.mutations);
                if plan.config_contents != live_config {
                    mutations.push(FileMutation::text(
                        home.join("config.toml"),
                        plan.config_contents,
                    ));
                }
            }
            Err(error) => {
                if let Some(active) = state.profiles.get_mut(&settings.active_relay_id) {
                    active.action_required = Some(error.to_string());
                }
            }
        }
        mutations.push(state_mutation(&state)?);
        live_state::commit_locked(&mutations)?;
        status_payload(&state, &settings, &home)
    })();
    command_result(result, "官方模型目录已刷新。", "官方模型目录刷新失败")
}

fn save_profile_catalog_blocking(
    request: SaveProfileCatalogRequest,
) -> CommandResult<CatalogStatusPayload> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let result = (|| -> anyhow::Result<CatalogStatusPayload> {
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        live_state::recover_locked()?;
        let mut settings = sanitized_settings()?;
        let profile_index = settings
            .relay_profiles
            .iter()
            .position(|profile| profile.id == request.profile_id)
            .context("供应商不存在")?;
        let profile = &settings.relay_profiles[profile_index];
        ensure!(
            managed_catalog_capable(profile),
            "该供应商不支持托管模型目录"
        );
        validate_upstream_topology(profile, request.upstream_topology)?;
        validate_overlay(&request.overlay)?;
        let mut state = load_and_migrate_state(&settings, &home)?;
        let profile_state = state.profiles.entry(profile.id.clone()).or_default();
        profile_state.upstream_topology = request.upstream_topology;
        profile_state.mode = request.mode;
        profile_state.mode_explicit = request.mode_explicit;
        profile_state.overlay = request.overlay;
        if request.mode != CatalogMode::External {
            profile_state.external_pointer = None;
        }
        profile_state.action_required = None;
        state.operation_generation = state.operation_generation.saturating_add(1);

        let mut mutations = Vec::new();
        let saved_conflicts = if managed_mode(request.mode) {
            global_context_conflicts(&profile.config_contents)
        } else {
            Vec::new()
        };
        if !saved_conflicts.is_empty() {
            ensure!(
                request.confirm_context_cleanup,
                "托管目录需要移除全局上下文设置：{}",
                saved_conflicts.join(", ")
            );
            settings.relay_profiles[profile_index].config_contents = remove_global_context_keys(
                &settings.relay_profiles[profile_index].config_contents,
            )?;
            mutations.push(FileMutation::bytes(
                codex_plus_core::paths::default_settings_path(),
                serde_json::to_vec_pretty(&settings)?,
            ));
        }
        let profile = &settings.relay_profiles[profile_index];
        if profile.id == settings.active_relay_id {
            let live_config = fs::read_to_string(home.join("config.toml"))?;
            let plan = plan_active_profile_with_state(
                &home,
                &settings,
                &live_config,
                &mut state,
                request.confirm_context_cleanup,
            )?;
            mutations.extend(plan.mutations);
            mutations.push(FileMutation::text(
                home.join("config.toml"),
                plan.config_contents,
            ));
        } else {
            if let Some(mutation) = materialize_profile(&mut state, profile, &home)? {
                mutations.push(mutation);
            }
        }
        mutations.push(state_mutation(&state)?);
        live_state::commit_locked(&mutations)?;
        status_payload(&state, &settings, &home)
    })();
    command_result(
        result,
        "供应商模型目录设置已保存。",
        "供应商模型目录保存失败",
    )
}

fn adopt_external_model_catalog_blocking(
    request: AdoptCatalogRequest,
) -> CommandResult<AdoptionPreviewPayload> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let result = (|| -> anyhow::Result<AdoptionPreviewPayload> {
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        live_state::recover_locked()?;
        let mut settings = sanitized_settings()?;
        let profile_index = settings
            .relay_profiles
            .iter()
            .position(|profile| profile.id == request.profile_id)
            .context("供应商不存在")?;
        let profile = settings.relay_profiles[profile_index].clone();
        let mut state = load_and_migrate_state(&settings, &home)?;
        let profile_state = state.profiles.get(&profile.id).context("缺少目录状态")?;
        ensure!(
            profile_state.mode == CatalogMode::External,
            "该目录不属于外部模式"
        );
        let pointer = profile_state
            .external_pointer
            .clone()
            .context("外部目录指针为空")?;
        let source = resolve_catalog_pointer(&home, &pointer)?;
        let source_bytes = fs::read(&source)?;
        let source_hash = content_hash(&source_bytes);
        let raw: Value = serde_json::from_slice(&source_bytes)?;
        validate_catalog_structure(&raw)?;
        validate_effective_catalog_offline(&raw)?;
        let target = verify_target_cli_fresh()?;
        let catalog_client_version = catalog_declared_version(&raw);
        let version_status =
            external_version_status(catalog_client_version.as_deref(), &target.client_version);
        let (overlay, collisions) = overlay_from_catalog(state.official.as_ref(), &raw)?;
        let payload = AdoptionPreviewPayload {
            profile_id: profile.id.clone(),
            source_path: source.to_string_lossy().to_string(),
            official_override_count: overlay.official.len(),
            custom_models: overlay.custom.clone(),
            collisions,
            source_hash: source_hash.clone(),
            catalog_client_version,
            target_client_version: target.client_version,
            version_status: version_status.clone(),
            committed: request.commit,
        };
        if request.commit {
            ensure!(payload.collisions.is_empty(), "外部目录含冲突 slug");
            ensure!(
                request.expected_source_hash.as_deref() == Some(source_hash.as_str()),
                "外部目录已在预览后变化，请重新预览"
            );
            ensure!(
                request.expected_target_client_version.as_deref()
                    == Some(payload.target_client_version.as_str())
                    && request.expected_version_status.as_deref()
                        == Some(payload.version_status.as_str()),
                "目标 CLI 或版本兼容状态已在预览后变化，请重新预览"
            );
            if version_status == "mismatch" {
                ensure!(
                    request.accept_version_mismatch,
                    "需要明确接受外部目录版本不匹配警告"
                );
            }
            let profile_state = state.profiles.get_mut(&profile.id).unwrap();
            profile_state.mode = if state.official.is_some() {
                CatalogMode::OfficialPlusCustom
            } else {
                CatalogMode::CustomOnly
            };
            profile_state.mode_explicit = true;
            profile_state.overlay = overlay;
            profile_state.external_pointer = None;
            state.operation_generation = state.operation_generation.saturating_add(1);
            let saved_conflicts = global_context_conflicts(&profile.config_contents);
            let mut settings_mutation = None;
            if !saved_conflicts.is_empty() {
                ensure!(
                    request.confirm_context_cleanup,
                    "采用托管目录需要移除全局上下文设置：{}",
                    saved_conflicts.join(", ")
                );
                settings.relay_profiles[profile_index].config_contents =
                    remove_global_context_keys(&profile.config_contents)?;
                settings_mutation = Some(FileMutation::bytes(
                    codex_plus_core::paths::default_settings_path(),
                    serde_json::to_vec_pretty(&settings)?,
                ));
            }
            let profile = &settings.relay_profiles[profile_index];
            let source_config = if profile.id == settings.active_relay_id {
                fs::read_to_string(home.join("config.toml"))?
            } else {
                profile.config_contents.clone()
            };
            let mut mutations = vec![FileMutation::text(
                adoption_backup_path(&profile.id),
                sanitize_nonsecret_config_backup(&source_config)?,
            )];
            if let Some(mutation) = settings_mutation {
                mutations.push(mutation);
            }
            if profile.id == settings.active_relay_id {
                let plan = plan_active_profile_with_state(
                    &home,
                    &settings,
                    &source_config,
                    &mut state,
                    request.confirm_context_cleanup,
                )?;
                mutations.extend(plan.mutations);
                mutations.push(FileMutation::text(
                    home.join("config.toml"),
                    plan.config_contents,
                ));
            } else if let Some(mutation) = materialize_profile(&mut state, profile, &home)? {
                mutations.push(mutation);
            }
            mutations.push(state_mutation(&state)?);
            live_state::commit_locked(&mutations)?;
        }
        Ok(payload)
    })();
    command_result(result, "外部模型目录预览已生成。", "外部模型目录采用失败")
}

pub fn plan_active_profile(
    home: &Path,
    settings: &BackendSettings,
    provider_config: &str,
    confirm_context_cleanup: bool,
) -> anyhow::Result<ActiveCatalogPlan> {
    let mut state = load_and_migrate_state(settings, home)?;
    let mut plan = plan_active_profile_with_state(
        home,
        settings,
        provider_config,
        &mut state,
        confirm_context_cleanup,
    )?;
    state.operation_generation = state.operation_generation.saturating_add(1);
    plan.mutations.push(state_mutation(&state)?);
    Ok(plan)
}

fn plan_active_profile_with_state(
    home: &Path,
    settings: &BackendSettings,
    provider_config: &str,
    state: &mut CatalogState,
    confirm_context_cleanup: bool,
) -> anyhow::Result<ActiveCatalogPlan> {
    let profile = settings.active_relay_profile();
    let profile_state = state
        .profiles
        .entry(profile.id.clone())
        .or_default()
        .clone();
    let mut config = provider_config.to_string();
    let mut mutations = Vec::new();

    match profile_state.mode {
        CatalogMode::NativeOfficial => {
            if manager_owned_pointer(
                &state,
                &profile.id,
                root_catalog_pointer(&config).as_deref(),
            ) {
                config = set_root_catalog_pointer(&config, None)?;
            }
        }
        CatalogMode::External => {
            config = set_root_catalog_pointer(&config, profile_state.external_pointer.as_deref())?;
        }
        CatalogMode::OfficialPlusCustom | CatalogMode::CustomOnly => {
            ensure!(
                managed_catalog_capable(&profile),
                "该供应商不支持托管模型目录"
            );
            validate_upstream_topology(&profile, profile_state.upstream_topology)?;
            let conflicts = global_context_conflicts(&config);
            if !conflicts.is_empty() {
                ensure!(
                    confirm_context_cleanup,
                    "托管目录需要移除全局上下文设置：{}",
                    conflicts.join(", ")
                );
                config = remove_global_context_keys(&config)?;
            }
            let catalog = compose_profile_catalog(&state, &profile, &profile_state)?;
            validate_catalog_structure(&catalog)?;
            validate_effective_catalog_offline(&catalog)?;
            let bytes = serde_json::to_vec_pretty(&catalog)?;
            let hash = content_hash(&bytes);
            let relative = generated_relative_path(&profile.id);
            let path = home.join(&relative);
            if !catalog_file_matches(&path, &hash)? {
                mutations.push(FileMutation::bytes(path, bytes));
            }
            config = set_root_catalog_pointer(&config, Some(&relative))?;
            let profile_state = state.profiles.get_mut(&profile.id).unwrap();
            if profile_state.generated_hash.as_deref() != Some(&hash) {
                profile_state.restart_required = true;
                profile_state.generation = profile_state.generation.saturating_add(1);
            }
            profile_state.generated_path = Some(relative);
            profile_state.generated_hash = Some(hash);
            profile_state.action_required = None;
        }
    }
    Ok(ActiveCatalogPlan {
        config_contents: config,
        mutations,
    })
}

pub fn record_provider_evidence(
    profile_id: &str,
    endpoint: &str,
    models: &[String],
) -> anyhow::Result<()> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let _guard = live_state::lock()?;
    live_state::prepare_secret_paths(&home)?;
    let settings = sanitized_settings()?;
    let mut state = load_and_migrate_state(&settings, &home)?;
    let official_slugs = state
        .official
        .as_ref()
        .map(|snapshot| catalog_slugs(&snapshot.raw_catalog))
        .transpose()?
        .unwrap_or_default();
    let reported_slugs = normalize_slugs(models.iter().cloned());
    let candidate_slugs = reported_slugs
        .iter()
        .filter(|slug| !official_slugs.contains(*slug))
        .cloned()
        .collect();
    let profile_state = state.profiles.entry(profile_id.to_string()).or_default();
    profile_state.provider_evidence = Some(ProviderEvidence {
        fetched_at_ms: now_ms(),
        endpoint: format!("sha256:{}", hash_text(endpoint)),
        reported_slugs,
        candidate_slugs,
    });
    live_state::commit_locked(&[state_mutation(&state)?])
}

fn sanitized_settings() -> anyhow::Result<BackendSettings> {
    let mut settings = SettingsStore::default().load()?;
    for profile in &mut settings.relay_profiles {
        profile.auth_contents.clear();
    }
    Ok(settings)
}

fn load_and_migrate_state(settings: &BackendSettings, home: &Path) -> anyhow::Result<CatalogState> {
    load_and_migrate_state_from_path(settings, home, &state_path())
}

fn load_and_migrate_state_from_path(
    settings: &BackendSettings,
    home: &Path,
    path: &Path,
) -> anyhow::Result<CatalogState> {
    let mut state = match fs::read(&path) {
        Ok(bytes) => {
            live_state::ensure_owner_only_file(&path)?;
            serde_json::from_slice::<CatalogState>(&bytes)
                .context("model catalog state is invalid")?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => CatalogState::default(),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        state.version <= STATE_VERSION,
        "model catalog state comes from a newer manager version"
    );
    state.version = STATE_VERSION;
    if state.scope_salt.trim().is_empty() {
        state.scope_salt = new_scope_salt();
    }
    let official_slugs = state
        .official
        .as_ref()
        .map(|snapshot| catalog_slugs(&snapshot.raw_catalog))
        .transpose()?
        .unwrap_or_default();
    let profile_ids = settings
        .relay_profiles
        .iter()
        .map(|profile| profile.id.clone())
        .collect::<BTreeSet<_>>();
    state.profiles.retain(|id, _| profile_ids.contains(id));
    for profile in &settings.relay_profiles {
        let existing_pointer = root_catalog_pointer(&profile.config_contents);
        let state_has_profile = state.profiles.contains_key(&profile.id);
        let entry = state.profiles.entry(profile.id.clone()).or_default();
        if !state_has_profile {
            entry.mode = default_mode(
                profile,
                existing_pointer.as_deref(),
                entry.upstream_topology,
            );
            if entry.mode == CatalogMode::External {
                entry.external_pointer = existing_pointer.clone();
            }
            entry.overlay = migrate_legacy_overlay(profile, &official_slugs)?;
        } else if !entry.mode_explicit {
            if let Some(pointer) = existing_pointer.as_deref() {
                if !manager_owned_pointer_path(&profile.id, pointer, entry) {
                    entry.mode = CatalogMode::External;
                    entry.external_pointer = Some(pointer.to_string());
                }
            }
        }
        if profile.relay_mode == RelayMode::Aggregate
            || profile.protocol == codex_plus_core::settings::RelayProtocol::ChatCompletions
        {
            entry.action_required = Some("该供应商依赖未提供的代理能力。".to_string());
        }
    }
    if settings.active_relay_id.trim().is_empty() {
        let _ = home;
    }
    Ok(state)
}

fn migrate_legacy_overlay(
    profile: &RelayProfile,
    official_slugs: &BTreeSet<String>,
) -> anyhow::Result<CatalogOverlay> {
    let windows = serde_json::from_str::<BTreeMap<String, String>>(&profile.model_windows)
        .unwrap_or_default();
    let mut overlay = CatalogOverlay::default();
    for (order, slug) in split_model_list(&profile.model_list)
        .into_iter()
        .enumerate()
    {
        let window = windows
            .get(&slug)
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|value| *value > 0);
        if official_slugs.contains(&slug) {
            if window.is_some() {
                overlay.official.insert(
                    slug,
                    OfficialOverride {
                        context_window: window,
                        order: Some(order as i64),
                        visible: None,
                        ..OfficialOverride::default()
                    },
                );
            }
        } else {
            overlay.custom.push(CustomModel {
                slug: slug.clone(),
                display_name: slug,
                context_window: window.unwrap_or(272_000),
                visible: true,
                order: order as i64,
                template_provenance: "legacy-model-list".to_string(),
                ..CustomModel::default()
            });
        }
    }
    validate_overlay(&overlay)?;
    Ok(overlay)
}

fn default_mode(
    profile: &RelayProfile,
    pointer: Option<&str>,
    topology: UpstreamTopology,
) -> CatalogMode {
    if pointer.is_some() {
        return CatalogMode::External;
    }
    match profile.relay_mode {
        RelayMode::Official if !profile.official_mix_api_key => CatalogMode::NativeOfficial,
        RelayMode::Official | RelayMode::MixedApi => CatalogMode::OfficialPlusCustom,
        RelayMode::PureApi if topology == UpstreamTopology::ServerSideComposite => {
            CatalogMode::OfficialPlusCustom
        }
        RelayMode::PureApi => CatalogMode::CustomOnly,
        RelayMode::Aggregate => CatalogMode::NativeOfficial,
    }
}

pub(crate) fn default_catalog_mode_for_profile(profile: &RelayProfile) -> CatalogMode {
    default_mode(
        profile,
        root_catalog_pointer(&profile.config_contents).as_deref(),
        UpstreamTopology::Direct,
    )
}

pub(crate) fn catalog_state_path() -> PathBuf {
    state_path()
}

pub(crate) fn read_only_catalog_modes_from_path(
    settings: &BackendSettings,
    path: &Path,
) -> anyhow::Result<BTreeMap<String, CatalogMode>> {
    let state = match fs::read(path) {
        Ok(bytes) => Some(
            serde_json::from_slice::<CatalogState>(&bytes)
                .context("model catalog state is invalid")?,
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let mut modes = BTreeMap::new();
    for profile in &settings.relay_profiles {
        let pointer = root_catalog_pointer(&profile.config_contents);
        let mode = match state
            .as_ref()
            .and_then(|state| state.profiles.get(&profile.id))
        {
            Some(profile_state) if profile_state.mode == CatalogMode::External => {
                CatalogMode::External
            }
            Some(profile_state)
                if !profile_state.mode_explicit
                    && pointer.as_deref().is_some_and(|pointer| {
                        !manager_owned_pointer_path(&profile.id, pointer, profile_state)
                    }) =>
            {
                CatalogMode::External
            }
            Some(profile_state) => profile_state.mode,
            None => default_mode(profile, pointer.as_deref(), UpstreamTopology::Direct),
        };
        modes.insert(profile.id.clone(), mode);
    }
    Ok(modes)
}

fn managed_mode(mode: CatalogMode) -> bool {
    matches!(
        mode,
        CatalogMode::OfficialPlusCustom | CatalogMode::CustomOnly
    )
}

pub(crate) fn validate_upstream_topology(
    profile: &RelayProfile,
    topology: UpstreamTopology,
) -> anyhow::Result<()> {
    if topology == UpstreamTopology::Direct {
        return Ok(());
    }
    ensure!(
        profile.relay_mode == RelayMode::PureApi,
        "服务端复合供应商必须使用纯 API 模式"
    );
    ensure!(
        profile.protocol == codex_plus_core::settings::RelayProtocol::Responses,
        "服务端复合供应商必须使用 Responses API"
    );
    ensure!(
        !codex_plus_core::relay_config::relay_profile_base_url(profile)
            .trim()
            .is_empty(),
        "服务端复合供应商缺少 Base URL"
    );
    ensure!(
        !codex_plus_core::relay_config::relay_profile_api_key(profile)
            .trim()
            .is_empty(),
        "服务端复合供应商缺少 provider bearer token"
    );
    Ok(())
}

const GLOBAL_CONTEXT_KEYS: [&str; 2] = ["model_context_window", "model_auto_compact_token_limit"];

fn global_context_conflicts(config: &str) -> Vec<String> {
    let Ok(doc) = config.parse::<toml_edit::DocumentMut>() else {
        return Vec::new();
    };
    GLOBAL_CONTEXT_KEYS
        .iter()
        .filter(|key| doc.as_table().contains_key(**key))
        .map(|key| (*key).to_string())
        .collect()
}

fn remove_global_context_keys(config: &str) -> anyhow::Result<String> {
    let mut doc: toml_edit::DocumentMut = config.parse()?;
    for key in GLOBAL_CONTEXT_KEYS {
        doc.as_table_mut().remove(key);
    }
    Ok(doc.to_string())
}

pub fn ensure_active_config_context_compatible(config: &str) -> anyhow::Result<()> {
    let settings = sanitized_settings()?;
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let state = load_and_migrate_state(&settings, &home)?;
    let profile = settings.active_relay_profile();
    let mode = state
        .profiles
        .get(&profile.id)
        .map(|item| item.mode)
        .unwrap_or(CatalogMode::NativeOfficial);
    if managed_mode(mode) {
        let conflicts = global_context_conflicts(config);
        ensure!(
            conflicts.is_empty(),
            "托管目录不能写入全局上下文设置：{}",
            conflicts.join(", ")
        );
    }
    Ok(())
}

pub fn prepare_active_profile_context_settings(
    settings: &mut BackendSettings,
    confirm_context_cleanup: bool,
) -> anyhow::Result<()> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let state = load_and_migrate_state(settings, &home)?;
    let active_id = settings.active_relay_id.clone();
    let mode = state
        .profiles
        .get(&active_id)
        .map(|item| item.mode)
        .unwrap_or(CatalogMode::NativeOfficial);
    if !managed_mode(mode) {
        return Ok(());
    }
    let profile = settings
        .relay_profiles
        .iter_mut()
        .find(|profile| profile.id == active_id)
        .context("当前供应商不存在")?;
    let conflicts = global_context_conflicts(&profile.config_contents);
    if conflicts.is_empty() {
        return Ok(());
    }
    ensure!(
        confirm_context_cleanup,
        "托管目录需要移除全局上下文设置：{}",
        conflicts.join(", ")
    );
    profile.config_contents = remove_global_context_keys(&profile.config_contents)?;
    Ok(())
}

fn state_mutation(state: &CatalogState) -> anyhow::Result<FileMutation> {
    let value = serde_json::to_value(state)?;
    ensure!(
        !contains_forbidden_credential_field(&value),
        "catalog state contains a credential field"
    );
    let bytes = serde_json::to_vec_pretty(&value)?;
    Ok(FileMutation::bytes(state_path(), bytes))
}

fn contains_forbidden_credential_field(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            matches!(
                key.as_str(),
                "access_token"
                    | "accessToken"
                    | "refresh_token"
                    | "refreshToken"
                    | "id_token"
                    | "idToken"
                    | "OPENAI_API_KEY"
                    | "experimental_bearer_token"
                    | "authContents"
                    | "apiKey"
            ) || contains_forbidden_credential_field(value)
        }),
        Value::Array(values) => values.iter().any(contains_forbidden_credential_field),
        _ => false,
    }
}

fn state_path() -> PathBuf {
    codex_plus_core::paths::default_app_state_dir().join(STATE_FILE)
}

fn status_payload(
    state: &CatalogState,
    settings: &BackendSettings,
    home: &Path,
) -> anyhow::Result<CatalogStatusPayload> {
    let target = verify_target_cli().ok().or_else(|| state.target.clone());
    let auth_snapshot = snapshot_live_auth(&home.join("auth.json"), &state.scope_salt);
    let auth_action = auth_snapshot
        .as_ref()
        .err()
        .map(|_| "请在官方 Codex/ChatGPT 客户端中登录或刷新认证后重试。".to_string());
    let target_stale = match (&state.official, &target) {
        (Some(official), Some(target)) => official.client_version != target.client_version,
        (Some(_), None) => true,
        (None, _) => true,
    };
    let scope_stale = state.official.as_ref().is_some_and(|official| {
        auth_snapshot.as_ref().map_or(true, |auth| {
            hash_text(&format!("{}:{}", state.scope_salt, auth.scope_identity))
                != official.scope_hash
        })
    });
    let age_stale = state.official.as_ref().is_some_and(|official| {
        now_ms().saturating_sub(official.fetched_at_ms) > 7 * 24 * 60 * 60 * 1000
    });
    let profiles = settings
        .relay_profiles
        .iter()
        .map(|profile| {
            let item = state.profiles.get(&profile.id).cloned().unwrap_or_default();
            let evidence = item.provider_evidence.as_ref();
            let mut context_conflicts = if managed_mode(item.mode) {
                global_context_conflicts(&profile.config_contents)
            } else {
                Vec::new()
            };
            if managed_mode(item.mode) && profile.id == settings.active_relay_id {
                if let Ok(live_config) = fs::read_to_string(home.join("config.toml")) {
                    context_conflicts.extend(global_context_conflicts(&live_config));
                    context_conflicts.sort();
                    context_conflicts.dedup();
                }
            }
            let action_required = item.action_required.clone().or_else(|| {
                (!context_conflicts.is_empty()).then(|| {
                    format!(
                        "全局上下文设置会覆盖托管目录：{}；确认后移除。",
                        context_conflicts.join(", ")
                    )
                })
            });
            ProfileCatalogSummary {
                profile_id: profile.id.clone(),
                mode: item.mode,
                mode_explicit: item.mode_explicit,
                upstream_topology: item.upstream_topology,
                managed_available: managed_catalog_capable(profile),
                context_conflicts,
                external_pointer: item.external_pointer,
                generated_path: item.generated_path,
                effective_hash: item.generated_hash,
                restart_required: item.restart_required,
                action_required,
                official_override_count: item.overlay.official.len(),
                custom_count: item.overlay.custom.len(),
                provider_evidence_at_ms: evidence.map(|value| value.fetched_at_ms),
                provider_reported_count: evidence
                    .map(|value| value.reported_slugs.len())
                    .unwrap_or(0),
                custom_candidates: evidence
                    .map(|value| value.candidate_slugs.clone())
                    .unwrap_or_default(),
                provider_reported_slugs: evidence
                    .map(|value| value.reported_slugs.clone())
                    .unwrap_or_default(),
                overlay: item.overlay,
            }
        })
        .collect();
    let official = state.official.as_ref();
    let official_models = official
        .and_then(|snapshot| catalog_models(&snapshot.raw_catalog).ok())
        .map(|models| {
            models
                .iter()
                .filter_map(|model| {
                    Some(OfficialModelSummary {
                        slug: model.get("slug")?.as_str()?.to_string(),
                        display_name: model
                            .get("display_name")
                            .and_then(Value::as_str)
                            .unwrap_or_else(|| {
                                model.get("slug").and_then(Value::as_str).unwrap_or("")
                            })
                            .to_string(),
                        visible: model.get("visibility").and_then(Value::as_str) != Some("hide"),
                        context_window: model.get("context_window").and_then(Value::as_u64),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(CatalogStatusPayload {
        state_path: state_path().to_string_lossy().to_string(),
        source: official
            .map(|item| item.source.clone())
            .unwrap_or_else(|| "none".to_string()),
        target_client_version: target.as_ref().map(|item| item.client_version.clone()),
        target_cli_path: target.as_ref().map(|item| item.cli_path.clone()),
        target_trusted: target.as_ref().is_some_and(|item| item.trusted),
        refresh_available: target
            .as_ref()
            .is_some_and(|item| item.trusted && item.capability_available),
        last_successful_refresh_at_ms: official.map(|item| item.fetched_at_ms),
        visible_count: official.map(|item| item.visible_count).unwrap_or(0),
        total_count: official.map(|item| item.total_count).unwrap_or(0),
        freshness: if official.is_none() {
            "missing"
        } else if target_stale || scope_stale {
            "scope-stale"
        } else if age_stale {
            "stale"
        } else {
            "current"
        }
        .to_string(),
        credential_action: auth_action,
        diff: state.last_diff.clone(),
        official_models,
        profiles,
    })
}

fn verify_target_cli() -> anyhow::Result<VerifiedTargetIdentity> {
    verify_target_cli_with_cache(true)
}

fn verify_target_cli_fresh() -> anyhow::Result<VerifiedTargetIdentity> {
    verify_target_cli_with_cache(false)
}

fn verify_target_cli_with_cache(
    reuse_cached_failure: bool,
) -> anyhow::Result<VerifiedTargetIdentity> {
    let cli = discover_target_codex_cli()?;
    let canonical_cli = fs::canonicalize(&cli)?;
    let app = application_bundle_for_cli(&canonical_cli)?;
    let canonical_app = fs::canonicalize(&app)?;
    verify_bundle_relationship(&canonical_app, &canonical_cli)?;
    let cache_key = target_verification_cache_key(&canonical_app, &canonical_cli)?;
    if let Some(result) = cached_target_verification(&cache_key, reuse_cached_failure) {
        return result;
    }

    let result = (|| -> anyhow::Result<VerifiedTargetIdentity> {
        let version_output = run_bounded_command(
            &canonical_cli,
            &["--version"],
            None,
            CAPABILITY_TIMEOUT,
            false,
            None,
        )?;
        let version = parse_cli_version(&version_output.stdout)?;
        let supported = semver::Version::parse(&version)
            .ok()
            .zip(semver::Version::parse(MIN_SUPPORTED_CLI).ok())
            .is_some_and(|(current, minimum)| current >= minimum);

        let (trusted, publisher) = verify_platform_publisher(&canonical_app, &canonical_cli)?;
        let capability = run_bounded_command(
            &canonical_cli,
            &["debug", "models", "--bundled"],
            None,
            CAPABILITY_TIMEOUT,
            true,
            None,
        );
        let capability_available = supported
            && capability
                .as_ref()
                .ok()
                .and_then(|result| serde_json::from_slice::<Value>(&result.stdout).ok())
                .and_then(|value| value.get("models").and_then(Value::as_array).map(Vec::len))
                .is_some_and(|count| count > 0);
        let identity_hash = hash_text(&format!(
            "{}:{}:{}",
            canonical_cli.display(),
            version,
            publisher
        ));
        Ok(VerifiedTargetIdentity {
            app_path: canonical_app.to_string_lossy().to_string(),
            cli_path: canonical_cli.to_string_lossy().to_string(),
            client_version: version,
            publisher,
            identity_hash,
            trusted,
            capability_available,
            capability_message: if !supported {
                format!("目标 CLI 版本低于支持下限 {MIN_SUPPORTED_CLI}")
            } else if capability_available {
                "目标 CLI 支持 debug models。".to_string()
            } else {
                "目标 CLI 的 bundled models 能力探测失败。".to_string()
            },
        })
    })();
    remember_target_verification(cache_key, &result);
    result
}

fn target_verification_cache_key(
    app: &Path,
    cli: &Path,
) -> anyhow::Result<TargetVerificationCacheKey> {
    let app_metadata = fs::metadata(app)?;
    let cli_metadata = fs::metadata(cli)?;
    Ok(TargetVerificationCacheKey {
        app_path: app.to_path_buf(),
        app_len: app_metadata.len(),
        app_modified: app_metadata.modified().ok(),
        cli_path: cli.to_path_buf(),
        cli_len: cli_metadata.len(),
        cli_modified: cli_metadata.modified().ok(),
    })
}

fn cached_target_verification(
    key: &TargetVerificationCacheKey,
    reuse_cached_failure: bool,
) -> Option<anyhow::Result<VerifiedTargetIdentity>> {
    TARGET_VERIFICATION_CACHE
        .get_or_init(|| StdMutex::new(None))
        .lock()
        .ok()
        .and_then(|cached| {
            let (_, value) = cached
                .as_ref()
                .filter(|(cached_key, _)| cached_key == key)?;
            match value {
                TargetVerificationCacheValue::Verified(target) => Some(Ok(target.clone())),
                TargetVerificationCacheValue::Failed(message) if reuse_cached_failure => {
                    Some(Err(anyhow::anyhow!(message.clone())))
                }
                TargetVerificationCacheValue::Failed(_) => None,
            }
        })
}

fn remember_target_verification(
    key: TargetVerificationCacheKey,
    result: &anyhow::Result<VerifiedTargetIdentity>,
) {
    if let Ok(mut cached) = TARGET_VERIFICATION_CACHE
        .get_or_init(|| StdMutex::new(None))
        .lock()
    {
        let value = match result {
            Ok(target) => TargetVerificationCacheValue::Verified(target.clone()),
            Err(error) => TargetVerificationCacheValue::Failed(error.to_string()),
        };
        *cached = Some((key, value));
    }
}

fn application_bundle_for_cli(cli: &Path) -> anyhow::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        cli.ancestors()
            .find(|path| path.extension().and_then(|value| value.to_str()) == Some("app"))
            .map(Path::to_path_buf)
            .context("目标 CLI 不在 .app bundle 内")
    }
    #[cfg(windows)]
    {
        cli.parent()
            .map(Path::to_path_buf)
            .context("目标 CLI 缺少应用目录")
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = cli;
        anyhow::bail!("当前平台未实现 credential-bearing target trust verifier")
    }
}

#[cfg(target_os = "macos")]
fn verify_bundle_relationship(app: &Path, cli: &Path) -> anyhow::Result<()> {
    let expected_root = app.join("Contents").join("Resources");
    ensure!(cli.starts_with(&expected_root), "目标 CLI 逃逸应用 bundle");
    ensure!(
        cli == expected_root.join("codex"),
        "目标 CLI 不是 bundle 内固定 codex 路径"
    );
    Ok(())
}

#[cfg(windows)]
fn verify_bundle_relationship(app: &Path, cli: &Path) -> anyhow::Result<()> {
    ensure!(cli.parent() == Some(app), "目标 CLI 逃逸应用目录");
    ensure!(
        cli.file_name().and_then(|value| value.to_str()) == Some("codex.exe"),
        "目标 CLI 不是应用目录内固定 codex.exe 路径"
    );
    Ok(())
}

#[cfg(not(any(target_os = "macos", windows)))]
fn verify_bundle_relationship(_app: &Path, _cli: &Path) -> anyhow::Result<()> {
    anyhow::bail!("当前平台未实现 target bundle relationship verifier")
}

#[cfg(target_os = "macos")]
fn verify_platform_publisher(app: &Path, cli: &Path) -> anyhow::Result<(bool, String)> {
    let app_status = Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(app)
        .status()?;
    ensure!(
        app_status.success(),
        "codesign verification failed for {}",
        app.display()
    );
    let cli_status = Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict"])
        .arg(cli)
        .status()?;
    ensure!(
        cli_status.success(),
        "codesign verification failed for {}",
        cli.display()
    );
    let output = Command::new("/usr/bin/codesign")
        .args(["-dv", "--verbose=4"])
        .arg(cli)
        .output()?;
    ensure!(output.status.success(), "cannot inspect CLI signature");
    let detail = String::from_utf8_lossy(&output.stderr);
    let team_id = detail
        .lines()
        .find_map(|line| line.strip_prefix("TeamIdentifier="))
        .map(str::trim)
        .context("CLI signature has no TeamIdentifier")?;
    ensure!(supported_mac_team_id(team_id), "unsupported CLI publisher");
    Ok((true, format!("OpenAI Team {team_id}")))
}

#[cfg(any(target_os = "macos", test))]
fn supported_mac_team_id(team_id: &str) -> bool {
    OPENAI_MAC_TEAM_IDS.contains(&team_id)
}

#[cfg(windows)]
fn verify_platform_publisher(app: &Path, cli: &Path) -> anyhow::Result<(bool, String)> {
    let _ = app;
    let script = format!(
        "$s=Get-AuthenticodeSignature -LiteralPath '{}'; Write-Output $s.Status; Write-Output $s.SignerCertificate.Subject",
        cli.to_string_lossy().replace('\'', "''")
    );
    let output = crate::platform_command::background_command("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .output()?;
    ensure!(output.status.success(), "Authenticode verification failed");
    let text = String::from_utf8_lossy(&output.stdout);
    ensure!(
        text.lines().next() == Some("Valid"),
        "CLI signature is not valid"
    );
    ensure!(
        text.to_ascii_lowercase().contains("openai"),
        "unsupported CLI publisher"
    );
    Ok((true, "OpenAI Authenticode publisher".to_string()))
}

#[cfg(not(any(target_os = "macos", windows)))]
fn verify_platform_publisher(_app: &Path, _cli: &Path) -> anyhow::Result<(bool, String)> {
    Ok((false, "unsupported platform".to_string()))
}

fn parse_cli_version(output: &[u8]) -> anyhow::Result<String> {
    let text = String::from_utf8(output.to_vec())?;
    text.split_whitespace()
        .find(|part| {
            part.chars()
                .next()
                .is_some_and(|value| value.is_ascii_digit())
        })
        .map(ToString::to_string)
        .context("目标 CLI 未返回完整版本号")
}

fn snapshot_live_auth(path: &Path, scope_salt: &str) -> anyhow::Result<AuthSnapshot> {
    live_state::ensure_owner_only_file(path).context("live auth 不可读；请在官方客户端登录")?;
    let bytes = fs::read(path)?;
    let value: Value = serde_json::from_slice(&bytes).context("live auth JSON 无效")?;
    ensure!(
        value.get("auth_mode").and_then(Value::as_str) == Some("chatgpt"),
        "live auth 不是文件型 ChatGPT 登录"
    );
    ensure!(
        value.get("OPENAI_API_KEY").is_none_or(|api_key| {
            api_key.is_null()
                || api_key
                    .as_str()
                    .is_some_and(|value| value.trim().is_empty())
        }),
        "live auth 混有 API key"
    );
    let tokens = value
        .get("tokens")
        .or_else(|| value.get("chatgptAuthTokens"))
        .and_then(Value::as_object)
        .context("live auth 不含 chatgpt token 数据")?;
    let id_token = token_string(tokens, &["id_token", "idToken"])?;
    let access_token = token_string(tokens, &["access_token", "accessToken"])?;
    validate_access_token_expiry(&access_token)?;
    let claims = jwt_claims(&id_token).unwrap_or_default();
    let account_id = token_optional_string(tokens, &["account_id", "accountId"])
        .or_else(|| {
            json_path_string(
                &claims,
                &["https://api.openai.com/auth", "chatgpt_account_id"],
            )
        })
        .or_else(|| json_path_string(&claims, &["chatgpt_account_id"]))
        .context("live auth 缺少 ChatGPT account identity")?;
    let workspace_id = token_optional_string(tokens, &["workspace_id", "workspaceId"])
        .or_else(|| json_path_string(&claims, &["workspace_id"]))
        .unwrap_or_default();
    let scope_identity = hash_text(&format!("{scope_salt}:{account_id}:{workspace_id}"));
    let mut projected_tokens = Map::new();
    projected_tokens.insert("id_token".to_string(), Value::String(id_token));
    projected_tokens.insert("access_token".to_string(), Value::String(access_token));
    projected_tokens.insert("refresh_token".to_string(), Value::String(String::new()));
    projected_tokens.insert("account_id".to_string(), Value::String(account_id));
    if !workspace_id.is_empty() {
        projected_tokens.insert("workspace_id".to_string(), Value::String(workspace_id));
    }
    Ok(AuthSnapshot {
        generation_hash: content_hash(&bytes),
        scope_identity,
        projection: json!({
            "auth_mode": "chatgpt",
            "tokens": projected_tokens,
            "last_refresh": chrono::Utc::now().to_rfc3339(),
        }),
    })
}

fn token_string(tokens: &Map<String, Value>, names: &[&str]) -> anyhow::Result<String> {
    token_optional_string(tokens, names).context("required token field is missing")
}

fn token_optional_string(tokens: &Map<String, Value>, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| tokens.get(*name).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn jwt_claims(token: &str) -> anyhow::Result<Value> {
    let payload = token.split('.').nth(1).context("token is not a JWT")?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn validate_access_token_expiry(token: &str) -> anyhow::Result<()> {
    let claims = jwt_claims(token)?;
    let exp = claims
        .get("exp")
        .and_then(Value::as_i64)
        .context("access token 缺少 exp")?;
    ensure!(exp > now_ms() / 1000 + 60, "access token 已过期或即将过期");
    Ok(())
}

fn json_path_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

#[derive(Debug)]
struct IsolatedRefreshResult {
    output: Value,
    cache: Value,
}

fn run_isolated_refresh(
    target: &VerifiedTargetIdentity,
    auth_projection: &Value,
    network: &crate::network_policy::ResolvedNetworkPolicy,
) -> anyhow::Result<IsolatedRefreshResult> {
    let root = codex_plus_core::paths::default_app_state_dir().join("catalog-refresh");
    live_state::ensure_owner_only_dir(&root)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let home = root.join(format!("refresh-{}-{nonce}", std::process::id()));
    live_state::ensure_owner_only_dir(&home)?;
    let result = (|| -> anyhow::Result<IsolatedRefreshResult> {
        live_state::atomic_write_owner_only(
            &home.join("config.toml"),
            b"forced_login_method = \"chatgpt\"\n",
        )?;
        live_state::atomic_write_owner_only(
            &home.join("auth.json"),
            &serde_json::to_vec_pretty(auth_projection)?,
        )?;
        ensure!(
            !home.join("models_cache.json").exists(),
            "temporary cache was not empty"
        );
        let output = run_bounded_command(
            Path::new(&target.cli_path),
            &["debug", "models"],
            Some(&home),
            REFRESH_TIMEOUT,
            true,
            Some(network),
        )
        .with_context(|| format!("Manager 网络请求失败（来源：{}）", network.source))?;
        let projected_after: Value = serde_json::from_slice(&fs::read(home.join("auth.json"))?)?;
        let projected_tokens = projected_after
            .get("tokens")
            .and_then(Value::as_object)
            .context("isolated auth projection was replaced")?;
        ensure!(
            token_optional_string(projected_tokens, &["refresh_token", "refreshToken"]).is_none(),
            "target CLI persisted a usable refresh credential"
        );
        ensure!(
            projected_after.get("OPENAI_API_KEY").is_none(),
            "target CLI persisted an API key"
        );
        let cache_path = home.join("models_cache.json");
        ensure!(
            cache_path.is_file(),
            "目标 CLI 未创建远端模型缓存，已回退 bundled 模型；Manager 网络来源：{}。请先在 Manager 网络中测试连接",
            network.source
        );
        let command_json: Value = serde_json::from_slice(&output.stdout)
            .context("target CLI model output is malformed")?;
        let cache_json: Value = serde_json::from_slice(&fs::read(cache_path)?)
            .context("isolated model cache is malformed")?;
        Ok(IsolatedRefreshResult {
            output: command_json,
            cache: cache_json,
        })
    })();
    let cleanup = fs::remove_dir_all(&home);
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) => Err(error).context("failed to clean isolated refresh home"),
        (Err(error), _) => Err(error),
    }
}

fn validate_effective_catalog_offline(catalog: &Value) -> anyhow::Result<()> {
    let target = verify_target_cli_fresh()?;
    ensure!(target.capability_available, "目标 CLI 不支持静态目录验证");
    let root = codex_plus_core::paths::default_app_state_dir().join("catalog-validation");
    live_state::ensure_owner_only_dir(&root)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let home = root.join(format!("validate-{}-{nonce}", std::process::id()));
    live_state::ensure_owner_only_dir(&home)?;
    let result = (|| -> anyhow::Result<()> {
        live_state::atomic_write_owner_only(
            &home.join("effective.json"),
            &serde_json::to_vec_pretty(catalog)?,
        )?;
        live_state::atomic_write_owner_only(
            &home.join("config.toml"),
            b"model_catalog_json = \"effective.json\"\n",
        )?;
        let output = run_bounded_command(
            Path::new(&target.cli_path),
            &["debug", "models"],
            Some(&home),
            CAPABILITY_TIMEOUT,
            true,
            None,
        )?;
        ensure!(
            !home.join("auth.json").exists(),
            "offline validation read or created auth"
        );
        let output: Value = serde_json::from_slice(&output.stdout)?;
        validate_catalog_structure(&output)?;
        ensure!(
            catalog_compatibility_projection(&output)?
                == catalog_compatibility_projection(catalog)?,
            "target CLI rejected or changed effective catalog picker semantics"
        );
        Ok(())
    })();
    let cleanup = fs::remove_dir_all(&home);
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(error)) => Err(error).context("failed to clean catalog validation home"),
        (Err(error), _) => Err(error),
    }
}

struct BoundedOutput {
    stdout: Vec<u8>,
}

fn run_bounded_command(
    cli: &Path,
    args: &[&str],
    codex_home: Option<&Path>,
    timeout: Duration,
    clear_environment: bool,
    network: Option<&crate::network_policy::ResolvedNetworkPolicy>,
) -> anyhow::Result<BoundedOutput> {
    let output_root = codex_plus_core::paths::default_app_state_dir().join("command-output");
    live_state::ensure_owner_only_dir(&output_root)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let stdout_path = output_root.join(format!("stdout-{}-{nonce}", std::process::id()));
    let stderr_path = output_root.join(format!("stderr-{}-{nonce}", std::process::id()));
    create_private_empty_file(&stdout_path)?;
    create_private_empty_file(&stderr_path)?;
    let stdout_file = OpenOptions::new().append(true).open(&stdout_path)?;
    let stderr_file = OpenOptions::new().append(true).open(&stderr_path)?;
    let mut command = crate::platform_command::background_command(cli);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout_file)
        .stderr(stderr_file);
    if clear_environment {
        command.env_clear();
        for (name, value) in isolated_child_environment(std::env::vars_os(), network) {
            command.env(name, value);
        }
        command.env("LANG", "C.UTF-8");
        command.env("LC_ALL", "C.UTF-8");
        #[cfg(unix)]
        command.env("PATH", "/usr/bin:/bin");
    }
    if let Some(home) = codex_home {
        command.env("CODEX_HOME", home);
    }
    let result = (|| -> anyhow::Result<BoundedOutput> {
        let mut child = command.spawn()?;
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if started.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("target CLI timed out");
            }
            if fs::metadata(&stdout_path)?.len() > MAX_COMMAND_OUTPUT_BYTES
                || fs::metadata(&stderr_path)?.len() > MAX_COMMAND_OUTPUT_BYTES
            {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("target CLI output exceeded limit");
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        ensure!(status.success(), "target CLI failed with status {status}");
        Ok(BoundedOutput {
            stdout: fs::read(&stdout_path)?,
        })
    })();
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    result
}

fn isolated_child_environment(
    source: impl IntoIterator<Item = (OsString, OsString)>,
    network: Option<&crate::network_policy::ResolvedNetworkPolicy>,
) -> Vec<(OsString, OsString)> {
    let mut environment = safe_non_network_child_environment(source);
    if let Some(network) = network {
        environment.extend(network.environment.iter().cloned());
    }
    environment
}

fn safe_non_network_child_environment(
    source: impl IntoIterator<Item = (OsString, OsString)>,
) -> Vec<(OsString, OsString)> {
    const COMMON: &[&str] = &["SSL_CERT_FILE", "SSL_CERT_DIR", "TMPDIR"];
    #[cfg(windows)]
    const PLATFORM: &[&str] = &["SystemRoot", "WINDIR", "TEMP", "TMP", "PATH"];
    #[cfg(not(windows))]
    const PLATFORM: &[&str] = &[];
    source
        .into_iter()
        .filter(|(name, _)| {
            let name = name.to_string_lossy();
            COMMON
                .iter()
                .chain(PLATFORM.iter())
                .any(|allowed| name.eq_ignore_ascii_case(allowed))
        })
        .collect()
}

fn create_private_empty_file(path: &Path) -> anyhow::Result<()> {
    live_state::atomic_write_owner_only(path, b"")
}

fn validate_catalog(value: &Value, client_version: &str) -> anyhow::Result<()> {
    validate_catalog_structure(value)?;
    validate_catalog_client_version(value, client_version)?;
    let models = catalog_models(value)?;
    for model in models {
        validate_rich_model(model)?;
        ensure!(
            model
                .get("base_instructions")
                .and_then(Value::as_str)
                .is_some(),
            "model has no instructions"
        );
    }
    Ok(())
}

fn validate_refresh_cache(value: &Value, client_version: &str) -> anyhow::Result<()> {
    validate_catalog_structure(value)?;
    validate_catalog_client_version(value, client_version)?;
    for model in catalog_models(value)? {
        validate_rich_model(model)?;
        ensure!(
            model
                .get("base_instructions")
                .is_none_or(|instructions| instructions.is_null() || instructions.is_string()),
            "cached model has invalid instructions"
        );
    }
    Ok(())
}

fn validate_catalog_client_version(value: &Value, client_version: &str) -> anyhow::Result<()> {
    if let Some(version) = value
        .get("client_version")
        .or_else(|| value.get("clientVersion"))
        .and_then(Value::as_str)
    {
        ensure!(
            catalog_client_version_compatible(version, client_version),
            "catalog client version mismatch: catalog={version}, target={client_version}"
        );
    }
    Ok(())
}

fn validate_rich_model(model: &Value) -> anyhow::Result<()> {
    ensure!(
        model.get("display_name").and_then(Value::as_str).is_some(),
        "model is not rich"
    );
    ensure!(
        model
            .get("context_window")
            .and_then(Value::as_u64)
            .is_some(),
        "model has no context window"
    );
    Ok(())
}

fn catalog_client_version_compatible(catalog: &str, target: &str) -> bool {
    let (Ok(catalog), Ok(target)) = (Version::parse(catalog), Version::parse(target)) else {
        return false;
    };
    catalog.major == target.major && catalog.minor == target.minor && catalog.patch == target.patch
}

fn validate_catalog_structure(value: &Value) -> anyhow::Result<()> {
    let models = catalog_models(value)?;
    ensure!(!models.is_empty(), "model catalog is empty");
    let mut slugs = HashSet::new();
    let mut visible = 0usize;
    for model in models {
        let slug = model
            .get("slug")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .context("model slug is empty")?;
        ensure!(
            slugs.insert(slug.to_string()),
            "duplicate model slug: {slug}"
        );
        if model.get("visibility").and_then(Value::as_str) != Some("hide") {
            visible += 1;
        }
    }
    ensure!(visible > 0, "model catalog has no visible models");
    Ok(())
}

fn catalog_models(value: &Value) -> anyhow::Result<&Vec<Value>> {
    value
        .get("models")
        .and_then(Value::as_array)
        .context("catalog has no models array")
}

fn catalog_counts(value: &Value) -> anyhow::Result<(usize, usize)> {
    let models = catalog_models(value)?;
    Ok((
        models
            .iter()
            .filter(|model| model.get("visibility").and_then(Value::as_str) != Some("hide"))
            .count(),
        models.len(),
    ))
}

fn catalog_slugs(value: &Value) -> anyhow::Result<BTreeSet<String>> {
    Ok(catalog_models(value)?
        .iter()
        .filter_map(|model| model.get("slug").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect())
}

fn validate_refresh_cache_matches_output(output: &Value, cache: &Value) -> anyhow::Result<()> {
    let output = model_value_map(output)?;
    let cache = model_value_map(cache)?;
    ensure!(
        output.keys().eq(cache.keys()),
        "目标 CLI 输出与隔离缓存的模型集合不一致"
    );
    for (slug, cached_model) in cache {
        let mut output_model = output
            .get(&slug)
            .cloned()
            .context("目标 CLI 输出缺少缓存模型")?;
        let mut cached_model = cached_model;
        // The CLI may hydrate instructions omitted by its raw cache. A supplied cache value
        // remains authoritative for this comparison and must still match exactly.
        if cached_model
            .get("base_instructions")
            .is_none_or(Value::is_null)
        {
            output_model
                .as_object_mut()
                .context("目标 CLI 输出模型不是对象")?
                .remove("base_instructions");
            cached_model
                .as_object_mut()
                .context("隔离缓存模型不是对象")?
                .remove("base_instructions");
        }
        ensure!(
            output_model == cached_model,
            "目标 CLI 输出与隔离缓存不一致：{slug}"
        );
    }
    Ok(())
}

fn catalog_compatibility_projection(
    value: &Value,
) -> anyhow::Result<BTreeMap<String, (String, String, Option<u64>, Option<u64>)>> {
    Ok(catalog_models(value)?
        .iter()
        .filter_map(|model| {
            let slug = model.get("slug")?.as_str()?.to_string();
            let display_name = model
                .get("display_name")
                .and_then(Value::as_str)
                .unwrap_or(&slug)
                .to_string();
            let visibility = model
                .get("visibility")
                .and_then(Value::as_str)
                .unwrap_or("list")
                .to_string();
            Some((
                slug,
                (
                    display_name,
                    visibility,
                    model.get("context_window").and_then(Value::as_u64),
                    model.get("max_context_window").and_then(Value::as_u64),
                ),
            ))
        })
        .collect())
}

fn diff_catalogs(previous: Option<&Value>, next: &Value) -> anyhow::Result<CatalogDiff> {
    let previous = previous
        .map(model_value_map)
        .transpose()?
        .unwrap_or_default();
    let next = model_value_map(next)?;
    let mut diff = CatalogDiff::default();
    for (slug, value) in &next {
        match previous.get(slug) {
            None => diff.added.push(slug.clone()),
            Some(previous) if canonical_json_hash(previous)? != canonical_json_hash(value)? => {
                diff.updated.push(slug.clone())
            }
            _ => {}
        }
    }
    for slug in previous.keys() {
        if !next.contains_key(slug) {
            diff.removed.push(slug.clone());
        }
    }
    Ok(diff)
}

fn model_value_map(value: &Value) -> anyhow::Result<BTreeMap<String, Value>> {
    Ok(catalog_models(value)?
        .iter()
        .filter_map(|model| {
            model
                .get("slug")
                .and_then(Value::as_str)
                .map(|slug| (slug.to_string(), model.clone()))
        })
        .collect())
}

pub(crate) fn compose_profile_catalog(
    state: &CatalogState,
    profile: &RelayProfile,
    profile_state: &ProfileCatalogState,
) -> anyhow::Result<Value> {
    validate_overlay(&profile_state.overlay)?;
    let mut root = match profile_state.mode {
        CatalogMode::OfficialPlusCustom => state
            .official
            .as_ref()
            .context("尚无当前目标与账号范围内的官方基线")?
            .raw_catalog
            .clone(),
        CatalogMode::CustomOnly => json!({ "models": [] }),
        _ => anyhow::bail!("profile mode is not composable"),
    };
    let official_slugs = state
        .official
        .as_ref()
        .map(|snapshot| catalog_slugs(&snapshot.raw_catalog))
        .transpose()?
        .unwrap_or_default();
    let mut models = if profile_state.mode == CatalogMode::OfficialPlusCustom {
        catalog_models(&root)?.clone()
    } else {
        Vec::new()
    };

    for model in &mut models {
        let Some(slug) = model
            .get("slug")
            .and_then(Value::as_str)
            .map(ToString::to_string)
        else {
            continue;
        };
        if let Some(custom) = profile_state
            .overlay
            .custom
            .iter()
            .find(|custom| custom.slug == slug)
        {
            apply_official_override(
                model,
                &OfficialOverride {
                    display_name: Some(custom.display_name.clone()),
                    visible: Some(custom.visible),
                    context_window: Some(custom.context_window),
                    effective_context_window_percent: Some(custom.effective_context_window_percent),
                    order: Some(custom.order),
                    supported_reasoning_levels: Some(custom.supported_reasoning_levels.clone()),
                    default_reasoning_level: custom.default_reasoning_level.clone(),
                    supported_tools: Some(custom.supported_tools.clone()),
                    tool_capabilities: custom.tool_capabilities.clone(),
                },
            )?;
        }
        if let Some(override_value) = profile_state.overlay.official.get(&slug) {
            apply_official_override(model, override_value)?;
        }
    }
    models.sort_by_key(|model| {
        let slug = model
            .get("slug")
            .and_then(Value::as_str)
            .unwrap_or_default();
        profile_state
            .overlay
            .official
            .get(slug)
            .and_then(|value| value.order)
            .unwrap_or_else(|| {
                model
                    .get("priority")
                    .and_then(Value::as_i64)
                    .unwrap_or(i64::MAX)
            })
    });

    let custom_entries = profile_state
        .overlay
        .custom
        .iter()
        .filter(|custom| !official_slugs.contains(&custom.slug))
        .map(|custom| codex_plus_core::model_suffix::ModelCatalogEntry {
            slug: custom.slug.clone(),
            display_name: if custom.display_name.trim().is_empty() {
                custom.slug.clone()
            } else {
                custom.display_name.clone()
            },
            suffix_window: Some(custom.context_window),
        })
        .collect::<Vec<_>>();
    if !custom_entries.is_empty() {
        let generated =
            codex_plus_core::model_suffix::build_model_catalog_json(&custom_entries, None);
        let generated: Value = serde_json::from_str(&generated)?;
        let by_slug = profile_state
            .overlay
            .custom
            .iter()
            .map(|custom| (custom.slug.as_str(), custom))
            .collect::<BTreeMap<_, _>>();
        for mut model in catalog_models(&generated)?.clone() {
            let slug = model
                .get("slug")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if let Some(custom) = by_slug.get(slug) {
                model["visibility"] = json!(if custom.visible { "list" } else { "hide" });
                model["priority"] = json!(custom.order);
                strip_official_only_capabilities(&mut model);
                model["effective_context_window_percent"] =
                    json!(custom.effective_context_window_percent);
                if !custom.supported_reasoning_levels.is_empty() {
                    model["supported_reasoning_levels"] =
                        serde_json::to_value(&custom.supported_reasoning_levels)?;
                }
                if let Some(default) = custom.default_reasoning_level.as_deref() {
                    model["default_reasoning_level"] = json!(default);
                }
                if !custom.supported_tools.is_empty() {
                    model["supported_tools"] = json!(custom.supported_tools);
                }
                if let Some(capabilities) = custom.tool_capabilities.as_ref() {
                    model["tool_capabilities"] = capabilities.clone();
                }
            }
            models.push(model);
        }
    }
    let output_slugs = models
        .iter()
        .filter_map(|model| model.get("slug").and_then(Value::as_str))
        .collect::<Vec<_>>();
    ensure!(
        output_slugs.len() == output_slugs.iter().copied().collect::<HashSet<_>>().len(),
        "effective catalog has duplicate slugs"
    );
    if let Some(default_model) = profile_default_model(profile) {
        ensure!(
            output_slugs.contains(&default_model.as_str()),
            "默认模型 {default_model} 不在有效目录中"
        );
    }
    root["models"] = Value::Array(models);
    validate_catalog_structure(&root)?;
    Ok(root)
}

fn apply_official_override(model: &mut Value, overlay: &OfficialOverride) -> anyhow::Result<()> {
    if let Some(display_name) = overlay.display_name.as_deref() {
        ensure!(
            !display_name.trim().is_empty(),
            "display name must not be empty"
        );
        model["display_name"] = json!(display_name.trim());
    }
    if let Some(visible) = overlay.visible {
        model["visibility"] = json!(if visible { "list" } else { "hide" });
    }
    if let Some(window) = overlay.context_window {
        ensure!(window > 0, "official context window must be positive");
        model["context_window"] = json!(window);
        model["max_context_window"] = json!(window);
    }
    if let Some(percent) = overlay.effective_context_window_percent {
        ensure!(
            (1..=100).contains(&percent),
            "effective context percent must be 1-100"
        );
        model["effective_context_window_percent"] = json!(percent);
    }
    if let Some(order) = overlay.order {
        model["priority"] = json!(order);
    }
    if let Some(levels) = overlay.supported_reasoning_levels.as_ref() {
        validate_reasoning(levels, overlay.default_reasoning_level.as_deref())?;
        model["supported_reasoning_levels"] = serde_json::to_value(levels)?;
    }
    if let Some(default) = overlay.default_reasoning_level.as_deref() {
        model["default_reasoning_level"] = json!(default);
    }
    if let Some(tools) = overlay.supported_tools.as_ref() {
        validate_supported_tools(tools)?;
        model["supported_tools"] = json!(tools);
    }
    if let Some(capabilities) = overlay.tool_capabilities.as_ref() {
        ensure!(
            capabilities.is_object(),
            "tool capabilities must be an object"
        );
        model["tool_capabilities"] = capabilities.clone();
    }
    Ok(())
}

fn strip_official_only_capabilities(model: &mut Value) {
    let Some(object) = model.as_object_mut() else {
        return;
    };
    for key in [
        "availability_nux",
        "upgrade",
        "service_tiers",
        "additional_speed_tiers",
        "supported_tools",
        "tool_capabilities",
    ] {
        object.remove(key);
    }
    object.insert("service_tiers".to_string(), Value::Array(Vec::new()));
    object.insert(
        "additional_speed_tiers".to_string(),
        Value::Array(Vec::new()),
    );
}

pub(crate) fn validate_overlay(overlay: &CatalogOverlay) -> anyhow::Result<()> {
    let mut slugs = HashSet::new();
    for (slug, value) in &overlay.official {
        validate_slug(slug)?;
        if let Some(window) = value.context_window {
            ensure!(window > 0, "context window must be positive");
        }
        if let Some(display_name) = value.display_name.as_deref() {
            ensure!(
                !display_name.trim().is_empty(),
                "display name must not be empty"
            );
        }
        if let Some(percent) = value.effective_context_window_percent {
            ensure!(
                (1..=100).contains(&percent),
                "effective context percent must be 1-100"
            );
        }
        if let Some(levels) = value.supported_reasoning_levels.as_ref() {
            validate_reasoning(levels, value.default_reasoning_level.as_deref())?;
        }
        if let Some(tools) = value.supported_tools.as_ref() {
            validate_supported_tools(tools)?;
        }
        if let Some(capabilities) = value.tool_capabilities.as_ref() {
            ensure!(
                capabilities.is_object(),
                "tool capabilities must be an object"
            );
        }
    }
    for custom in &overlay.custom {
        validate_slug(&custom.slug)?;
        ensure!(
            !custom.display_name.trim().is_empty(),
            "display name must not be empty"
        );
        ensure!(custom.context_window > 0, "context window must be positive");
        ensure!(
            (1..=100).contains(&custom.effective_context_window_percent),
            "effective context percent must be 1-100"
        );
        validate_reasoning(
            &custom.supported_reasoning_levels,
            custom.default_reasoning_level.as_deref(),
        )?;
        validate_supported_tools(&custom.supported_tools)?;
        if let Some(capabilities) = custom.tool_capabilities.as_ref() {
            ensure!(
                capabilities.is_object(),
                "tool capabilities must be an object"
            );
        }
        ensure!(slugs.insert(custom.slug.clone()), "duplicate custom slug");
    }
    Ok(())
}

fn validate_reasoning(levels: &[ReasoningLevel], default: Option<&str>) -> anyhow::Result<()> {
    let mut efforts = HashSet::new();
    for level in levels {
        let effort = level.effort.trim();
        ensure!(
            !effort.is_empty() && effort.len() <= 32,
            "reasoning effort is invalid"
        );
        ensure!(efforts.insert(effort), "duplicate reasoning effort");
    }
    if let Some(default) = default {
        ensure!(
            efforts.contains(default),
            "default reasoning effort is not supported"
        );
    }
    Ok(())
}

fn validate_supported_tools(tools: &[String]) -> anyhow::Result<()> {
    let mut seen = HashSet::new();
    for tool in tools {
        let tool = tool.trim();
        ensure!(
            !tool.is_empty() && tool.len() <= 80,
            "supported tool is invalid"
        );
        ensure!(seen.insert(tool), "duplicate supported tool");
    }
    Ok(())
}

fn validate_slug(slug: &str) -> anyhow::Result<()> {
    let slug = slug.trim();
    ensure!(!slug.is_empty(), "model slug is empty");
    ensure!(slug.len() <= 160, "model slug is too long");
    ensure!(
        !slug.chars().any(char::is_control),
        "model slug contains control characters"
    );
    Ok(())
}

fn materialize_inactive_profiles(
    state: &mut CatalogState,
    settings: &BackendSettings,
    home: &Path,
) -> anyhow::Result<Vec<FileMutation>> {
    let mut mutations = Vec::new();
    for profile in &settings.relay_profiles {
        if profile.id == settings.active_relay_id {
            continue;
        }
        match materialize_profile(state, profile, home) {
            Ok(Some(mutation)) => mutations.push(mutation),
            Ok(None) => {}
            Err(error) => {
                if let Some(item) = state.profiles.get_mut(&profile.id) {
                    item.action_required = Some(error.to_string());
                }
            }
        }
    }
    Ok(mutations)
}

fn materialize_profile(
    state: &mut CatalogState,
    profile: &RelayProfile,
    home: &Path,
) -> anyhow::Result<Option<FileMutation>> {
    let profile_state = state.profiles.get(&profile.id).cloned().unwrap_or_default();
    if !matches!(
        profile_state.mode,
        CatalogMode::OfficialPlusCustom | CatalogMode::CustomOnly
    ) {
        return Ok(None);
    }
    let catalog = compose_profile_catalog(state, profile, &profile_state)?;
    validate_effective_catalog_offline(&catalog)?;
    let bytes = serde_json::to_vec_pretty(&catalog)?;
    let hash = content_hash(&bytes);
    let relative = generated_relative_path(&profile.id);
    let path = home.join(&relative);
    let profile_state = state.profiles.get_mut(&profile.id).unwrap();
    if profile_state.generated_hash.as_deref() == Some(&hash) && catalog_file_matches(&path, &hash)?
    {
        return Ok(None);
    }
    profile_state.generated_hash = Some(hash);
    profile_state.generated_path = Some(relative);
    profile_state.generation = profile_state.generation.saturating_add(1);
    profile_state.action_required = None;
    Ok(Some(FileMutation::bytes(path, bytes)))
}

fn catalog_file_matches(path: &Path, expected_hash: &str) -> anyhow::Result<bool> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if content_hash(&bytes) != expected_hash {
        return Ok(false);
    }
    let value: Value = serde_json::from_slice(&bytes)?;
    validate_catalog_structure(&value)?;
    live_state::ensure_owner_only_file(path)?;
    Ok(true)
}

pub(crate) fn managed_catalog_capable(profile: &RelayProfile) -> bool {
    profile.relay_mode != RelayMode::Aggregate
        && profile.protocol != codex_plus_core::settings::RelayProtocol::ChatCompletions
}

pub(crate) fn generated_relative_path(profile_id: &str) -> String {
    let identity = hash_text(profile_id);
    format!(
        "{GENERATED_DIR}/{GENERATED_PREFIX}{}-{}.json",
        sanitize_profile_id(profile_id),
        &identity[..12]
    )
}

fn sanitize_profile_id(profile_id: &str) -> String {
    let value = profile_id
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || matches!(value, '-' | '_') {
                value
            } else {
                '-'
            }
        })
        .collect::<String>();
    let value = value.trim_matches('-');
    if value.is_empty() {
        "profile".to_string()
    } else {
        value.chars().take(96).collect()
    }
}

fn manager_owned_pointer(state: &CatalogState, profile_id: &str, pointer: Option<&str>) -> bool {
    let Some(pointer) = pointer else {
        return false;
    };
    state
        .profiles
        .get(profile_id)
        .is_some_and(|profile| manager_owned_pointer_path(profile_id, pointer, profile))
}

fn manager_owned_pointer_path(
    profile_id: &str,
    pointer: &str,
    profile: &ProfileCatalogState,
) -> bool {
    pointer == generated_relative_path(profile_id)
        && profile.generated_path.as_deref() == Some(pointer)
        && profile.generated_hash.is_some()
}

fn root_catalog_pointer(config: &str) -> Option<String> {
    let doc: toml_edit::DocumentMut = config.parse().ok()?;
    doc.get("model_catalog_json")
        .and_then(toml_edit::Item::as_str)
        .map(ToString::to_string)
}

fn set_root_catalog_pointer(config: &str, pointer: Option<&str>) -> anyhow::Result<String> {
    let mut doc: toml_edit::DocumentMut = config.parse()?;
    match pointer {
        Some(pointer) => doc["model_catalog_json"] = toml_edit::value(pointer),
        None => {
            doc.as_table_mut().remove("model_catalog_json");
        }
    }
    let rendered = doc.to_string();
    let _: toml_edit::DocumentMut = rendered.parse()?;
    Ok(rendered)
}

fn resolve_catalog_pointer(home: &Path, pointer: &str) -> anyhow::Result<PathBuf> {
    let path = Path::new(pointer);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        home.join(path)
    };
    let canonical = fs::canonicalize(path)?;
    ensure!(canonical.is_file(), "catalog pointer is not a file");
    Ok(canonical)
}

fn adoption_backup_path(profile_id: &str) -> PathBuf {
    codex_plus_core::paths::default_app_state_dir()
        .join("backups")
        .join(format!(
            "catalog-adoption-{}-{}.toml",
            sanitize_profile_id(profile_id),
            now_ms()
        ))
}

fn sanitize_nonsecret_config_backup(config: &str) -> anyhow::Result<String> {
    let mut doc: toml_edit::DocumentMut = config.parse()?;
    doc.as_table_mut().remove("OPENAI_API_KEY");
    if let Some(providers) = doc
        .get_mut("model_providers")
        .and_then(toml_edit::Item::as_table_mut)
    {
        for (_, provider) in providers.iter_mut() {
            if let Some(provider) = provider.as_table_mut() {
                provider.remove("experimental_bearer_token");
                provider.remove("bearer_token");
                provider.remove("api_key");
            }
        }
    }
    Ok(doc.to_string())
}

fn overlay_from_catalog(
    official: Option<&OfficialSnapshot>,
    catalog: &Value,
) -> anyhow::Result<(CatalogOverlay, Vec<String>)> {
    let official_map = official
        .map(|snapshot| model_value_map(&snapshot.raw_catalog))
        .transpose()?
        .unwrap_or_default();
    let mut overlay = CatalogOverlay::default();
    let mut collisions = Vec::new();
    let mut seen = HashSet::new();
    for (order, model) in catalog_models(catalog)?.iter().enumerate() {
        let slug = model
            .get("slug")
            .and_then(Value::as_str)
            .context("external model slug is missing")?
            .to_string();
        if !seen.insert(slug.clone()) {
            collisions.push(slug);
            continue;
        }
        let context_window = model
            .get("context_window")
            .and_then(Value::as_u64)
            .unwrap_or(272_000);
        let visible = model.get("visibility").and_then(Value::as_str) != Some("hide");
        let display_name = model
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or(&slug)
            .to_string();
        let effective_percent = model
            .get("effective_context_window_percent")
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(100);
        let reasoning_levels = model
            .get("supported_reasoning_levels")
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<ReasoningLevel>>(value).ok())
            .unwrap_or_default();
        let default_reasoning = model
            .get("default_reasoning_level")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let supported_tools = model
            .get("supported_tools")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let tool_capabilities = model.get("tool_capabilities").cloned();
        if let Some(base) = official_map.get(&slug) {
            let base_window = base.get("context_window").and_then(Value::as_u64);
            let base_visible = base.get("visibility").and_then(Value::as_str) != Some("hide");
            let candidate = OfficialOverride {
                display_name: (base.get("display_name").and_then(Value::as_str)
                    != Some(display_name.as_str()))
                .then_some(display_name),
                visible: (base_visible != visible).then_some(visible),
                context_window: (base_window != Some(context_window)).then_some(context_window),
                effective_context_window_percent: (base
                    .get("effective_context_window_percent")
                    .and_then(Value::as_u64)
                    != Some(u64::from(effective_percent)))
                .then_some(effective_percent),
                order: Some(order as i64),
                supported_reasoning_levels: (base.get("supported_reasoning_levels")
                    != model.get("supported_reasoning_levels"))
                .then_some(reasoning_levels),
                default_reasoning_level: (base.get("default_reasoning_level")
                    != model.get("default_reasoning_level"))
                .then_some(default_reasoning)
                .flatten(),
                supported_tools: (base.get("supported_tools") != model.get("supported_tools"))
                    .then_some(supported_tools),
                tool_capabilities: (base.get("tool_capabilities")
                    != model.get("tool_capabilities"))
                .then_some(tool_capabilities)
                .flatten(),
            };
            if official_override_has_values(&candidate) {
                overlay.official.insert(slug, candidate);
            }
        } else {
            overlay.custom.push(CustomModel {
                slug: slug.clone(),
                display_name,
                context_window,
                effective_context_window_percent: effective_percent,
                visible,
                order: order as i64,
                supported_reasoning_levels: reasoning_levels,
                default_reasoning_level: default_reasoning,
                supported_tools,
                tool_capabilities,
                template_provenance: "adopted-external-catalog".to_string(),
            });
        }
    }
    Ok((overlay, collisions))
}

fn official_override_has_values(value: &OfficialOverride) -> bool {
    value.display_name.is_some()
        || value.visible.is_some()
        || value.context_window.is_some()
        || value.effective_context_window_percent.is_some()
        || value.order.is_some()
        || value.supported_reasoning_levels.is_some()
        || value.default_reasoning_level.is_some()
        || value.supported_tools.is_some()
        || value.tool_capabilities.is_some()
}

fn catalog_declared_version(catalog: &Value) -> Option<String> {
    catalog
        .get("client_version")
        .or_else(|| catalog.get("clientVersion"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn external_version_status(catalog: Option<&str>, target: &str) -> String {
    match catalog {
        None => "unknown",
        Some(catalog) if catalog == target => "match",
        Some(_) => "mismatch",
    }
    .to_string()
}

fn profile_default_model(profile: &RelayProfile) -> Option<String> {
    let doc: toml_edit::DocumentMut = profile.config_contents.parse().ok()?;
    doc.get("model")
        .and_then(toml_edit::Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn split_model_list(value: &str) -> Vec<String> {
    normalize_slugs(value.split(['\r', '\n', ',']).map(ToString::to_string))
}

fn normalize_slugs(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && !value.chars().any(char::is_control))
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn command_result<T: Serialize + Default>(
    result: anyhow::Result<T>,
    success: &str,
    failure: &str,
) -> CommandResult<T> {
    match result {
        Ok(payload) => CommandResult {
            status: "ok".to_string(),
            message: success.to_string(),
            payload,
        },
        Err(error) => CommandResult {
            status: "failed".to_string(),
            message: format!("{failure}：{error}"),
            payload: T::default(),
        },
    }
}

impl Default for CatalogStatusPayload {
    fn default() -> Self {
        Self {
            state_path: state_path().to_string_lossy().to_string(),
            source: "none".to_string(),
            target_client_version: None,
            target_cli_path: None,
            target_trusted: false,
            refresh_available: false,
            last_successful_refresh_at_ms: None,
            visible_count: 0,
            total_count: 0,
            freshness: "missing".to_string(),
            credential_action: None,
            diff: CatalogDiff::default(),
            official_models: Vec::new(),
            profiles: Vec::new(),
        }
    }
}

impl Default for AdoptionPreviewPayload {
    fn default() -> Self {
        Self {
            profile_id: String::new(),
            source_path: String::new(),
            official_override_count: 0,
            custom_models: Vec::new(),
            collisions: Vec::new(),
            source_hash: String::new(),
            catalog_client_version: None,
            target_client_version: String::new(),
            version_status: "unknown".to_string(),
            committed: false,
        }
    }
}

fn content_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn hash_text(text: &str) -> String {
    content_hash(text.as_bytes())
}

fn canonical_json_hash(value: &Value) -> anyhow::Result<String> {
    Ok(content_hash(&serde_json::to_vec(value)?))
}

fn new_scope_salt() -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    hash_text(&format!("{}:{nonce}", std::process::id()))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static FAKE_REFRESH_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn direct_network() -> crate::network_policy::ResolvedNetworkPolicy {
        crate::network_policy::ResolvedNetworkPolicy {
            mode: crate::network_policy::NetworkPolicyMode::Direct,
            source: "direct".to_string(),
            environment: Vec::new(),
            endpoint: None,
            bypass_count: 0,
            supported: true,
            action_required: None,
        }
    }

    fn official_catalog() -> Value {
        json!({
            "models": [
                {
                    "slug": "official-a",
                    "display_name": "Official A",
                    "description": "A",
                    "visibility": "list",
                    "priority": 1,
                    "context_window": 100000,
                    "max_context_window": 100000,
                    "effective_context_window_percent": 95,
                    "base_instructions": "keep exactly",
                    "unknown_future_field": { "kept": true }
                },
                {
                    "slug": "hidden-b",
                    "display_name": "Hidden B",
                    "description": "B",
                    "visibility": "hide",
                    "priority": 2,
                    "context_window": 200000,
                    "max_context_window": 200000,
                    "base_instructions": "hidden"
                }
            ]
        })
    }

    #[test]
    fn official_fields_and_hidden_entries_survive_composition() {
        let mut state = CatalogState::default();
        state.official = Some(OfficialSnapshot {
            raw_catalog: official_catalog(),
            ..OfficialSnapshot::default()
        });
        let profile = RelayProfile {
            id: "p".to_string(),
            config_contents: "model = \"official-a\"\n".to_string(),
            ..RelayProfile::default()
        };
        let profile_state = ProfileCatalogState {
            mode: CatalogMode::OfficialPlusCustom,
            overlay: CatalogOverlay {
                official: BTreeMap::from([(
                    "official-a".to_string(),
                    OfficialOverride {
                        context_window: Some(300000),
                        ..OfficialOverride::default()
                    },
                )]),
                custom: vec![CustomModel {
                    slug: "custom-c".to_string(),
                    display_name: "Custom C".to_string(),
                    ..CustomModel::default()
                }],
            },
            ..ProfileCatalogState::default()
        };
        let output = compose_profile_catalog(&state, &profile, &profile_state).unwrap();
        let models = catalog_models(&output).unwrap();
        assert_eq!(models.len(), 3);
        assert_eq!(models[0]["base_instructions"], "keep exactly");
        assert_eq!(models[0]["unknown_future_field"]["kept"], true);
        assert!(models.iter().any(|model| model["slug"] == "hidden-b"));
        assert_eq!(models[0]["context_window"], 300000);
        assert_eq!(models[0]["effective_context_window_percent"], 95);
    }

    #[test]
    fn server_side_composite_is_explicit_and_keeps_proxy_modes_blocked() {
        let profile = RelayProfile {
            relay_mode: RelayMode::PureApi,
            protocol: codex_plus_core::settings::RelayProtocol::Responses,
            base_url: "https://relay.example/v1".to_string(),
            api_key: "provider-key".to_string(),
            ..RelayProfile::default()
        };
        validate_upstream_topology(&profile, UpstreamTopology::ServerSideComposite).unwrap();
        assert_eq!(
            default_mode(&profile, None, UpstreamTopology::Direct),
            CatalogMode::CustomOnly
        );
        assert_eq!(
            default_mode(&profile, None, UpstreamTopology::ServerSideComposite),
            CatalogMode::OfficialPlusCustom
        );

        let mut aggregate = profile.clone();
        aggregate.relay_mode = RelayMode::Aggregate;
        assert!(
            validate_upstream_topology(&aggregate, UpstreamTopology::ServerSideComposite).is_err()
        );
        assert!(!managed_catalog_capable(&aggregate));

        let mut chat = profile;
        chat.protocol = codex_plus_core::settings::RelayProtocol::ChatCompletions;
        assert!(validate_upstream_topology(&chat, UpstreamTopology::ServerSideComposite).is_err());
        assert!(!managed_catalog_capable(&chat));
    }

    #[test]
    fn rich_custom_metadata_round_trips_and_rejects_invalid_defaults() {
        let levels = vec![
            ReasoningLevel {
                effort: "low".to_string(),
                description: "Fast".to_string(),
            },
            ReasoningLevel {
                effort: "high".to_string(),
                description: "Deep".to_string(),
            },
        ];
        let overlay = CatalogOverlay {
            custom: vec![CustomModel {
                slug: "claude-haiku".to_string(),
                display_name: "Haiku".to_string(),
                context_window: 1_000_000,
                effective_context_window_percent: 95,
                supported_reasoning_levels: levels.clone(),
                default_reasoning_level: Some("low".to_string()),
                supported_tools: vec!["web_search".to_string()],
                tool_capabilities: Some(json!({"web_search": true})),
                ..CustomModel::default()
            }],
            ..CatalogOverlay::default()
        };
        validate_overlay(&overlay).unwrap();

        let profile = RelayProfile {
            config_contents: "model = \"claude-haiku\"\n".to_string(),
            ..RelayProfile::default()
        };
        let state = CatalogState::default();
        let profile_state = ProfileCatalogState {
            mode: CatalogMode::CustomOnly,
            overlay,
            ..ProfileCatalogState::default()
        };
        let output = compose_profile_catalog(&state, &profile, &profile_state).unwrap();
        let model = &catalog_models(&output).unwrap()[0];
        assert_eq!(model["display_name"], "Haiku");
        assert_eq!(model["effective_context_window_percent"], 95);
        assert_eq!(model["default_reasoning_level"], "low");
        assert_eq!(model["supported_reasoning_levels"][1]["effort"], "high");
        assert_eq!(model["supported_tools"][0], "web_search");
        assert_eq!(model["tool_capabilities"]["web_search"], true);
        assert_eq!(model["service_tiers"], json!([]));

        let mut invalid = profile_state.overlay.clone();
        invalid.custom[0].default_reasoning_level = Some("ultra".to_string());
        assert!(validate_overlay(&invalid).is_err());
    }

    #[test]
    fn managed_context_cleanup_is_scoped_and_deterministic() {
        let config = "model = \"m\"\nmodel_context_window = 372000\nmodel_auto_compact_token_limit = 330000\n[mcp_servers.memory]\ncommand = \"memory\"\n";
        assert_eq!(
            global_context_conflicts(config),
            GLOBAL_CONTEXT_KEYS.map(str::to_string)
        );
        let cleaned = remove_global_context_keys(config).unwrap();
        assert!(global_context_conflicts(&cleaned).is_empty());
        assert!(cleaned.contains("[mcp_servers.memory]"));
        assert!(cleaned.contains("command = \"memory\""));
    }

    #[test]
    fn external_version_status_is_evidence_not_official_compatibility() {
        assert_eq!(external_version_status(None, "0.147.0"), "unknown");
        assert_eq!(external_version_status(Some("0.147.0"), "0.147.0"), "match");
        assert_eq!(
            external_version_status(Some("0.145.0"), "0.147.0"),
            "mismatch"
        );
        assert!(!catalog_client_version_compatible("0.145.0", "0.147.0"));
    }

    #[test]
    fn composite_plan_stages_one_provider_pointer_and_no_auth_mutation() {
        let profile = RelayProfile {
            id: "composite".to_string(),
            relay_mode: RelayMode::PureApi,
            protocol: codex_plus_core::settings::RelayProtocol::Responses,
            base_url: "https://relay.example/v1".to_string(),
            api_key: "provider-key".to_string(),
            config_contents: "model = \"codex-minus-test-model\"\n".to_string(),
            ..RelayProfile::default()
        };
        let settings = BackendSettings {
            active_relay_id: profile.id.clone(),
            relay_profiles: vec![profile],
            ..BackendSettings::default()
        };
        let mut state = CatalogState::default();
        state.profiles.insert(
            "composite".to_string(),
            ProfileCatalogState {
                mode: CatalogMode::CustomOnly,
                upstream_topology: UpstreamTopology::ServerSideComposite,
                overlay: CatalogOverlay {
                    custom: vec![CustomModel {
                        slug: "codex-minus-test-model".to_string(),
                        display_name: "Codex Minus Test Model".to_string(),
                        supported_reasoning_levels: vec![ReasoningLevel {
                            effort: "low".to_string(),
                            description: "Fast".to_string(),
                        }],
                        default_reasoning_level: Some("low".to_string()),
                        ..CustomModel::default()
                    }],
                    ..CatalogOverlay::default()
                },
                ..ProfileCatalogState::default()
            },
        );
        let provider_config = "model = \"codex-minus-test-model\"\nmodel_provider = \"relay\"\n[model_providers.relay]\nbase_url = \"https://relay.example/v1\"\nexperimental_bearer_token = \"provider-key\"\n";
        let home = tempfile::tempdir().unwrap();
        let plan = plan_active_profile_with_state(
            home.path(),
            &settings,
            provider_config,
            &mut state,
            false,
        )
        .unwrap();
        assert_eq!(plan.config_contents.matches("[model_providers.").count(), 1);
        assert_eq!(
            plan.config_contents.matches("model_catalog_json").count(),
            1
        );
        assert!(plan.mutations.iter().all(|mutation| {
            mutation.path.file_name().and_then(|name| name.to_str()) != Some("auth.json")
        }));
    }

    #[test]
    fn provider_omission_does_not_remove_official_models() {
        let official = official_catalog();
        let official_slugs = catalog_slugs(&official).unwrap();
        let reported = normalize_slugs(vec!["official-a".to_string()]);
        assert!(official_slugs.contains("hidden-b"));
        assert!(!reported.contains(&"hidden-b".to_string()));
    }

    #[test]
    fn legacy_migration_classifies_modes_without_copying_provider_secrets() {
        let profile = RelayProfile {
            id: "mixed".to_string(),
            relay_mode: RelayMode::Official,
            official_mix_api_key: true,
            model_list: "official-a\ncustom-x".to_string(),
            model_windows: r#"{"official-a":"300000","custom-x":"120000"}"#.to_string(),
            api_key: "provider-secret".to_string(),
            auth_contents: "oauth-secret".to_string(),
            config_contents: "approval_policy = \"never\"\n".to_string(),
            ..RelayProfile::default()
        };
        let official = BTreeSet::from(["official-a".to_string()]);
        let overlay = migrate_legacy_overlay(&profile, &official).unwrap();
        assert_eq!(
            default_mode(&profile, None, UpstreamTopology::Direct),
            CatalogMode::OfficialPlusCustom
        );
        assert_eq!(overlay.official["official-a"].context_window, Some(300_000));
        assert_eq!(overlay.custom.len(), 1);
        assert_eq!(overlay.custom[0].slug, "custom-x");
        assert_eq!(overlay.custom[0].context_window, 120_000);

        let migrated = serde_json::to_string(&overlay).unwrap();
        assert!(!migrated.contains("provider-secret"));
        assert!(!migrated.contains("oauth-secret"));
        assert!(!migrated.contains("approval_policy"));
        assert_eq!(profile.model_list, "official-a\ncustom-x");
        assert_eq!(profile.api_key, "provider-secret");

        let external = default_mode(
            &profile,
            Some("/tmp/user-owned.json"),
            UpstreamTopology::Direct,
        );
        assert_eq!(external, CatalogMode::External);
        assert_eq!(
            default_mode(
                &RelayProfile {
                    relay_mode: RelayMode::PureApi,
                    ..RelayProfile::default()
                },
                None,
                UpstreamTopology::Direct,
            ),
            CatalogMode::CustomOnly
        );
    }

    #[test]
    fn manager_ownership_requires_state_metadata() {
        let pointer = generated_relative_path("profile");
        let state = CatalogState::default();
        assert!(!manager_owned_pointer(&state, "profile", Some(&pointer)));
        let mut state = state;
        state.profiles.insert(
            "profile".to_string(),
            ProfileCatalogState {
                generated_path: Some(pointer.clone()),
                generated_hash: Some("hash".to_string()),
                ..ProfileCatalogState::default()
            },
        );
        assert!(manager_owned_pointer(&state, "profile", Some(&pointer)));
    }

    #[test]
    fn auth_projection_has_no_usable_refresh_or_api_key() {
        let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&json!({ "exp": now_ms() / 1000 + 3600 })).unwrap());
        let id_claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&json!({ "chatgpt_account_id": "acct" })).unwrap());
        let value = json!({
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": null,
            "tokens": {
                "id_token": format!("x.{id_claims}.x"),
                "access_token": format!("x.{claims}.x"),
                "refresh_token": "live-refresh"
            }
        });
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        live_state::atomic_write_owner_only(&path, &serde_json::to_vec(&value).unwrap()).unwrap();
        let snapshot = snapshot_live_auth(&path, "salt").unwrap();
        assert_eq!(snapshot.projection["tokens"]["refresh_token"], "");
        assert!(snapshot.projection.get("OPENAI_API_KEY").is_none());

        let mut with_api_key = value;
        with_api_key["OPENAI_API_KEY"] = json!("sk-live-key");
        live_state::atomic_write_owner_only(&path, &serde_json::to_vec(&with_api_key).unwrap())
            .unwrap();
        assert!(snapshot_live_auth(&path, "salt").is_err());
    }

    #[test]
    fn custom_slug_promotes_to_official_once_and_keeps_user_overrides() {
        let mut state = CatalogState::default();
        state.official = Some(OfficialSnapshot {
            raw_catalog: official_catalog(),
            ..OfficialSnapshot::default()
        });
        state.official.as_mut().unwrap().raw_catalog["models"][1]["visibility"] = json!("list");
        let profile = RelayProfile {
            id: "p".to_string(),
            config_contents: "model = \"official-a\"\n".to_string(),
            ..RelayProfile::default()
        };
        let profile_state = ProfileCatalogState {
            mode: CatalogMode::OfficialPlusCustom,
            overlay: CatalogOverlay {
                custom: vec![CustomModel {
                    slug: "official-a".to_string(),
                    display_name: "Old custom".to_string(),
                    context_window: 444000,
                    visible: false,
                    order: 99,
                    template_provenance: "legacy".to_string(),
                    ..CustomModel::default()
                }],
                ..CatalogOverlay::default()
            },
            ..ProfileCatalogState::default()
        };
        let output = compose_profile_catalog(&state, &profile, &profile_state).unwrap();
        let promoted = catalog_models(&output)
            .unwrap()
            .iter()
            .filter(|model| model["slug"] == "official-a")
            .collect::<Vec<_>>();
        assert_eq!(promoted.len(), 1);
        assert_eq!(promoted[0]["display_name"], "Old custom");
        assert_eq!(promoted[0]["context_window"], 444000);
        assert_eq!(promoted[0]["visibility"], "hide");
        assert_eq!(promoted[0]["unknown_future_field"]["kept"], true);
    }

    #[test]
    fn duplicate_custom_and_removed_default_fail_before_materialization() {
        let duplicate = CatalogOverlay {
            custom: vec![
                CustomModel {
                    slug: "same".to_string(),
                    ..CustomModel::default()
                },
                CustomModel {
                    slug: "same".to_string(),
                    ..CustomModel::default()
                },
            ],
            ..CatalogOverlay::default()
        };
        assert!(validate_overlay(&duplicate).is_err());

        let mut state = CatalogState::default();
        state.official = Some(OfficialSnapshot {
            raw_catalog: official_catalog(),
            ..OfficialSnapshot::default()
        });
        let profile = RelayProfile {
            config_contents: "model = \"removed-model\"\n".to_string(),
            ..RelayProfile::default()
        };
        let profile_state = ProfileCatalogState {
            mode: CatalogMode::OfficialPlusCustom,
            ..ProfileCatalogState::default()
        };
        assert!(compose_profile_catalog(&state, &profile, &profile_state).is_err());
    }

    #[test]
    fn inactive_materialization_failure_keeps_last_valid_generation() {
        let profile = RelayProfile {
            id: "inactive".to_string(),
            config_contents: "model = \"same\"\n".to_string(),
            ..RelayProfile::default()
        };
        let mut state = CatalogState::default();
        state.profiles.insert(
            profile.id.clone(),
            ProfileCatalogState {
                mode: CatalogMode::CustomOnly,
                overlay: CatalogOverlay {
                    custom: vec![
                        CustomModel {
                            slug: "same".to_string(),
                            ..CustomModel::default()
                        },
                        CustomModel {
                            slug: "same".to_string(),
                            ..CustomModel::default()
                        },
                    ],
                    ..CatalogOverlay::default()
                },
                generated_path: Some("model-catalogs/last-valid.json".to_string()),
                generated_hash: Some("last-valid-hash".to_string()),
                generation: 7,
                ..ProfileCatalogState::default()
            },
        );
        let settings = BackendSettings {
            active_relay_id: "other".to_string(),
            relay_profiles: vec![profile],
            ..BackendSettings::default()
        };
        let home = tempfile::tempdir().unwrap();
        let mutations = materialize_inactive_profiles(&mut state, &settings, home.path()).unwrap();
        assert!(mutations.is_empty());
        let retained = &state.profiles["inactive"];
        assert_eq!(retained.generated_hash.as_deref(), Some("last-valid-hash"));
        assert_eq!(retained.generation, 7);
        assert!(retained.action_required.is_some());
    }

    #[test]
    fn status_loading_is_read_only_for_missing_state_and_migrates_existing_state_in_memory() {
        let temp = tempfile::tempdir().unwrap();
        let missing_path = temp.path().join("missing-state.json");
        let settings = BackendSettings {
            relay_profiles: vec![RelayProfile {
                id: "new-profile".to_string(),
                ..RelayProfile::default()
            }],
            ..BackendSettings::default()
        };

        let loaded =
            load_and_migrate_state_from_path(&settings, temp.path(), &missing_path).unwrap();
        assert!(loaded.profiles.contains_key("new-profile"));
        assert!(!missing_path.exists());

        let existing_path = temp.path().join("existing-state.json");
        let old = CatalogState {
            version: 1,
            ..CatalogState::default()
        };
        std::fs::write(&existing_path, serde_json::to_vec(&old).unwrap()).unwrap();
        let migrated =
            load_and_migrate_state_from_path(&settings, temp.path(), &existing_path).unwrap();
        assert_eq!(migrated.version, STATE_VERSION);
        let unchanged: CatalogState =
            serde_json::from_slice(&std::fs::read(&existing_path).unwrap()).unwrap();
        assert_eq!(unchanged.version, 1);
    }

    #[test]
    fn state_rejects_credential_fields_but_keeps_model_metadata() {
        let mut state = CatalogState::default();
        state.official = Some(OfficialSnapshot {
            raw_catalog: official_catalog(),
            ..OfficialSnapshot::default()
        });
        assert!(state_mutation(&state).is_ok());
        state.official.as_mut().unwrap().raw_catalog["access_token"] = json!("secret");
        assert!(state_mutation(&state).is_err());
    }

    #[test]
    fn whole_cli_version_and_publisher_allowlist_are_strict() {
        assert_eq!(
            parse_cli_version(b"codex-cli 0.147.0-alpha.1.2\n").unwrap(),
            "0.147.0-alpha.1.2"
        );
        assert!(catalog_client_version_compatible(
            "0.147.0",
            "0.147.0-alpha.1.2"
        ));
        assert!(catalog_client_version_compatible(
            "0.147.0-alpha.1.2",
            "0.147.0-alpha.1.2"
        ));
        assert!(!catalog_client_version_compatible(
            "0.147.1",
            "0.147.0-alpha.1.2"
        ));
        assert!(!catalog_client_version_compatible(
            "invalid",
            "0.147.0-alpha.1.2"
        ));
        assert!(supported_mac_team_id("2DC432GLL2"));
        assert!(!supported_mac_team_id("UNTRUSTED"));
    }

    #[test]
    fn refresh_cache_may_omit_only_cli_hydrated_instructions() {
        let output = official_catalog();
        let mut cache = output.clone();
        cache["models"][0]
            .as_object_mut()
            .unwrap()
            .remove("base_instructions");
        cache["models"][1]["base_instructions"] = Value::Null;

        validate_catalog(&output, "0.147.0-alpha.6.5").unwrap();
        validate_refresh_cache(&cache, "0.147.0-alpha.6.5").unwrap();
        validate_refresh_cache_matches_output(&output, &cache).unwrap();

        let mut conflicting_instructions = cache.clone();
        conflicting_instructions["models"][0]["base_instructions"] = json!("different");
        assert!(validate_refresh_cache_matches_output(&output, &conflicting_instructions).is_err());

        let mut conflicting_metadata = cache.clone();
        conflicting_metadata["models"][0]["context_window"] = json!(999);
        assert!(validate_refresh_cache_matches_output(&output, &conflicting_metadata).is_err());

        let mut missing_model = cache.clone();
        missing_model["models"].as_array_mut().unwrap().pop();
        assert!(validate_refresh_cache_matches_output(&output, &missing_model).is_err());

        let mut invalid_instructions = cache.clone();
        invalid_instructions["models"][0]["base_instructions"] = json!(false);
        assert!(validate_refresh_cache(&invalid_instructions, "0.147.0-alpha.6.5").is_err());

        let mut incomplete_output = output.clone();
        incomplete_output["models"][0]
            .as_object_mut()
            .unwrap()
            .remove("base_instructions");
        assert!(validate_catalog(&incomplete_output, "0.147.0-alpha.6.5").is_err());
    }

    #[test]
    fn isolated_child_environment_uses_only_the_resolved_proxy_snapshot() {
        let source = vec![
            (
                OsString::from("HTTPS_PROXY"),
                OsString::from("http://ambient:7890"),
            ),
            (OsString::from("NO_PROXY"), OsString::from("localhost")),
            (OsString::from("SSL_CERT_FILE"), OsString::from("/cert.pem")),
            (OsString::from("OPENAI_API_KEY"), OsString::from("secret")),
            (
                OsString::from("OPENAI_BASE_URL"),
                OsString::from("https://wrong"),
            ),
            (
                OsString::from("CODEX_AUTHAPI_BASE_URL"),
                OsString::from("https://wrong-auth"),
            ),
            (
                OsString::from("AWS_ACCESS_KEY_ID"),
                OsString::from("aws-secret"),
            ),
        ];
        let direct = direct_network();
        let filtered = isolated_child_environment(source.clone(), Some(&direct))
            .into_iter()
            .map(|(name, _)| name.to_string_lossy().to_string())
            .collect::<BTreeSet<_>>();
        assert!(filtered.contains("SSL_CERT_FILE"));
        assert!(!filtered.contains("HTTPS_PROXY"));
        assert!(!filtered.contains("NO_PROXY"));
        assert!(!filtered.contains("OPENAI_API_KEY"));
        assert!(!filtered.contains("OPENAI_BASE_URL"));
        assert!(!filtered.contains("CODEX_AUTHAPI_BASE_URL"));
        assert!(!filtered.contains("AWS_ACCESS_KEY_ID"));

        let custom = crate::network_policy::ResolvedNetworkPolicy {
            mode: crate::network_policy::NetworkPolicyMode::Custom,
            source: "custom".to_string(),
            environment: vec![
                (
                    OsString::from("HTTPS_PROXY"),
                    OsString::from("http://resolved:7890"),
                ),
                (
                    OsString::from("https_proxy"),
                    OsString::from("http://resolved:7890"),
                ),
            ],
            endpoint: Some("http://resolved:7890".to_string()),
            bypass_count: 0,
            supported: true,
            action_required: None,
        };
        let proxied = isolated_child_environment(source, Some(&custom))
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            proxied.get(&OsString::from("HTTPS_PROXY")),
            Some(&OsString::from("http://resolved:7890"))
        );
        assert_ne!(
            proxied.get(&OsString::from("HTTPS_PROXY")),
            Some(&OsString::from("http://ambient:7890"))
        );
    }

    #[cfg(unix)]
    fn fake_cli(contents: &str) -> (tempfile::TempDir, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let cli = temp.path().join("codex");
        fs::write(&cli, contents).unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
        (temp, cli)
    }

    #[cfg(unix)]
    #[test]
    fn isolated_refresh_uses_empty_refresh_projection_and_cleans_up() {
        let _guard = FAKE_REFRESH_TEST_LOCK.lock().unwrap();
        let payload = r#"{"models":[{"slug":"m","display_name":"M","visibility":"list","context_window":1000,"base_instructions":"x"}]}"#;
        let script = format!(
            "#!/bin/sh\nprintf '%s' '{payload}' > \"$CODEX_HOME/models_cache.json\"\nprintf '%s' '{payload}'\n"
        );
        let (_temp, cli) = fake_cli(&script);
        let target = VerifiedTargetIdentity {
            cli_path: cli.to_string_lossy().to_string(),
            ..VerifiedTargetIdentity::default()
        };
        let projection = json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "id_token": "id",
                "access_token": "access",
                "refresh_token": ""
            }
        });
        let root = codex_plus_core::paths::default_app_state_dir().join("catalog-refresh");
        let before = fs::read_dir(&root)
            .ok()
            .map(|items| {
                items
                    .filter_map(Result::ok)
                    .map(|item| item.path())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let result = run_isolated_refresh(&target, &projection, &direct_network()).unwrap();
        assert_eq!(
            catalog_slugs(&result.output).unwrap(),
            BTreeSet::from(["m".to_string()])
        );
        let after = fs::read_dir(&root)
            .ok()
            .map(|items| {
                items
                    .filter_map(Result::ok)
                    .map(|item| item.path())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        assert_eq!(before, after);
    }

    #[cfg(unix)]
    #[test]
    fn isolated_refresh_rejects_missing_cache_malformed_output_and_timeout() {
        let _guard = FAKE_REFRESH_TEST_LOCK.lock().unwrap();
        let (_temp, missing_cache) = fake_cli("#!/bin/sh\nprintf '{\"models\":[]}'\n");
        let target = VerifiedTargetIdentity {
            cli_path: missing_cache.to_string_lossy().to_string(),
            ..VerifiedTargetIdentity::default()
        };
        let projection = json!({"auth_mode":"chatgpt","tokens":{"id_token":"id","access_token":"access","refresh_token":""}});
        let error = run_isolated_refresh(&target, &projection, &direct_network()).unwrap_err();
        assert!(error.to_string().contains("bundled"));

        let (_temp, malformed) =
            fake_cli("#!/bin/sh\nprintf '{'\nprintf '{' > \"$CODEX_HOME/models_cache.json\"\n");
        let target = VerifiedTargetIdentity {
            cli_path: malformed.to_string_lossy().to_string(),
            ..VerifiedTargetIdentity::default()
        };
        assert!(run_isolated_refresh(&target, &projection, &direct_network()).is_err());

        let (_temp, slow) = fake_cli("#!/bin/sh\nsleep 2\n");
        assert!(
            run_bounded_command(&slow, &[], None, Duration::from_millis(30), true, None).is_err()
        );

        let (_temp, rejected) = fake_cli("#!/bin/sh\nexit 23\n");
        let projection_before = projection.clone();
        let target = VerifiedTargetIdentity {
            cli_path: rejected.to_string_lossy().to_string(),
            ..VerifiedTargetIdentity::default()
        };
        assert!(run_isolated_refresh(&target, &projection, &direct_network()).is_err());
        assert_eq!(projection, projection_before);
    }

    #[cfg(unix)]
    #[test]
    fn isolated_refresh_does_not_stop_an_active_target_process() {
        let _guard = FAKE_REFRESH_TEST_LOCK.lock().unwrap();
        let payload = r#"{"models":[{"slug":"m","display_name":"M","visibility":"list","context_window":1000,"base_instructions":"x"}]}"#;
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"--hold\" ]; then sleep 5; exit 0; fi\nprintf '%s' '{payload}' > \"$CODEX_HOME/models_cache.json\"\nprintf '%s' '{payload}'\n"
        );
        let (_temp, cli) = fake_cli(&script);
        let mut active = Command::new(&cli)
            .arg("--hold")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let target = VerifiedTargetIdentity {
            cli_path: cli.to_string_lossy().to_string(),
            ..VerifiedTargetIdentity::default()
        };
        let projection = json!({"auth_mode":"chatgpt","tokens":{"id_token":"id","access_token":"access","refresh_token":""}});
        let refreshed = run_isolated_refresh(&target, &projection, &direct_network()).unwrap();
        assert_eq!(
            catalog_slugs(&refreshed.output).unwrap(),
            BTreeSet::from(["m".to_string()])
        );
        assert!(active.try_wait().unwrap().is_none());
        active.kill().unwrap();
        active.wait().unwrap();
    }

    #[test]
    fn expired_or_missing_file_auth_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        assert!(snapshot_live_auth(&temp.path().join("missing.json"), "salt").is_err());
        let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&json!({ "exp": now_ms() / 1000 - 1 })).unwrap());
        let id_claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&json!({ "chatgpt_account_id": "acct" })).unwrap());
        let auth = json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "id_token": format!("x.{id_claims}.x"),
                "access_token": format!("x.{claims}.x"),
                "refresh_token": "refresh"
            }
        });
        let path = temp.path().join("auth.json");
        live_state::atomic_write_owner_only(&path, &serde_json::to_vec(&auth).unwrap()).unwrap();
        assert!(snapshot_live_auth(&path, "salt").is_err());
    }

    #[test]
    fn concurrent_auth_generation_or_account_change_is_rejected() {
        let original = AuthSnapshot {
            generation_hash: "generation-a".to_string(),
            scope_identity: "account-a".to_string(),
            projection: Value::Null,
        };
        let same = original.clone();
        let rotated = AuthSnapshot {
            generation_hash: "generation-b".to_string(),
            ..original.clone()
        };
        let account_changed = AuthSnapshot {
            scope_identity: "account-b".to_string(),
            ..original.clone()
        };
        assert!(auth_snapshot_matches(&original, &same));
        assert!(!auth_snapshot_matches(&original, &rotated));
        assert!(!auth_snapshot_matches(&original, &account_changed));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn bundle_relationship_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let app = temp.path().join("Fake.app");
        let resources = app.join("Contents/Resources");
        fs::create_dir_all(&resources).unwrap();
        let outside = temp.path().join("outside-codex");
        fs::write(&outside, "binary").unwrap();
        symlink(&outside, resources.join("codex")).unwrap();
        let canonical_app = fs::canonicalize(app).unwrap();
        let canonical_cli = fs::canonicalize(resources.join("codex")).unwrap();
        assert!(verify_bundle_relationship(&canonical_app, &canonical_cli).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn installed_target_is_trusted_and_accepts_static_catalog_offline() {
        if discover_target_codex_cli().is_err() {
            return;
        }
        let target = verify_target_cli().unwrap();
        assert!(target.trusted);
        assert!(target.capability_available);
        assert!(semver::Version::parse(&target.client_version).is_ok());
        let catalog = codex_plus_core::model_suffix::build_model_catalog_json(
            &[codex_plus_core::model_suffix::ModelCatalogEntry {
                slug: "codex-minus-test-model".to_string(),
                display_name: "Codex Minus Test Model".to_string(),
                suffix_window: Some(123_000),
            }],
            None,
        );
        let catalog: Value = serde_json::from_str(&catalog).unwrap();
        validate_effective_catalog_offline(&catalog).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires explicit approval for a live OAuth model-catalog request"]
    fn live_refresh_preserves_auth_and_is_idempotent() {
        assert_eq!(
            std::env::var("CODEX_MINUS_LIVE_REFRESH").as_deref(),
            Ok("1"),
            "set CODEX_MINUS_LIVE_REFRESH=1 only after explicit live-request approval"
        );
        let home = codex_plus_core::relay_config::default_codex_home_dir();
        let auth_path = home.join("auth.json");
        let auth_before = fs::read(&auth_path).unwrap();

        let first = refresh_official_model_catalog_blocking();
        assert_eq!(first.status, "ok", "{}", first.message);
        assert!(first.payload.target_trusted);
        assert!(first.payload.total_count > 0);
        assert_eq!(fs::read(&auth_path).unwrap(), auth_before);
        for profile in first.payload.profiles.iter().filter(|profile| {
            matches!(
                profile.mode,
                CatalogMode::OfficialPlusCustom | CatalogMode::CustomOnly
            )
        }) {
            assert!(profile.action_required.is_none(), "{profile:?}");
            assert!(profile.generated_path.is_some(), "{profile:?}");
            assert!(profile.effective_hash.is_some(), "{profile:?}");
        }

        let second = refresh_official_model_catalog_blocking();
        assert_eq!(second.status, "ok", "{}", second.message);
        assert!(second.payload.diff.added.is_empty());
        assert!(second.payload.diff.updated.is_empty());
        assert!(second.payload.diff.removed.is_empty());
        assert_eq!(fs::read(&auth_path).unwrap(), auth_before);
    }
}
