use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use codex_plus_core::models::{DeleteResult, SessionRef};
use codex_plus_core::settings::{
    BackendSettings, RelayContextSelection, RelayProfile, SettingsStore,
};
use codex_plus_core::status::LaunchStatus;
use codex_plus_core::zed_remote::{ZedOpenStrategy, ZedRemoteProject};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::live_state::{self, FileMutation};

#[derive(Debug, Clone, Serialize)]
pub struct CommandResult<T>
where
    T: Serialize,
{
    pub status: String,
    pub message: String,
    #[serde(flatten)]
    pub payload: T,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionPayload {
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathState {
    pub status: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OverviewPayload {
    pub codex_app: PathState,
    pub codex_version: Option<String>,
    pub silent_shortcut: PathState,
    pub management_shortcut: PathState,
    pub latest_launch: Option<LaunchStatus>,
    pub current_version: String,
    pub update_status: String,
    pub settings_path: String,
    pub logs_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsPayload {
    pub settings: BackendSettings,
    pub settings_path: String,
    pub user_scripts: Value,
    pub provider_fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketplaceRepairPayload {
    pub codex_home: String,
    pub marketplace_root: Option<String>,
    pub initialized: bool,
    pub configured: bool,
    pub needs_repair: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketplaceStatusPayload {
    pub codex_home: String,
    pub marketplace_root: Option<String>,
    pub config_registered: bool,
    pub needs_repair: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePluginMarketplacePayload {
    pub codex_home: String,
    pub marketplace_root: Option<String>,
    pub config_registered: bool,
    pub needs_repair: bool,
    pub plugin_count: usize,
    pub skill_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcsProvidersPayload {
    pub db_path: String,
    pub providers: Vec<codex_plus_core::ccs_import::CcsProviderImport>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingProviderImportPayload {
    pub pending: Option<codex_plus_core::provider_import::ProviderImportRequest>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSessionsPayload {
    pub db_path: String,
    pub db_paths: Vec<String>,
    pub sessions: Vec<codex_plus_data::LocalSession>,
    pub active_count: usize,
    pub archived_count: usize,
    pub archived: bool,
    pub next_cursor: Option<String>,
    pub page_size: usize,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalSessionsRequest {
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionLifecycleSettings {
    pub archive_enabled: bool,
    pub first_run_reviewed: bool,
    pub retention_days: u32,
    pub last_completed_at_ms: Option<i64>,
    // Runs the active-only session adaptation automatically after a provider switch. Defaults
    // on — the write is backed up and active-only — and serde-default keeps old files loading.
    pub auto_adapt_provider_on_switch: bool,
}

impl Default for SessionLifecycleSettings {
    fn default() -> Self {
        Self {
            archive_enabled: false,
            first_run_reviewed: false,
            retention_days: 30,
            last_completed_at_ms: None,
            auto_adapt_provider_on_switch: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArchivePreviewRequest {
    pub retention_days: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLifecycleOperationRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveCapability {
    pub available: bool,
    pub cli_path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivePreviewPayload {
    pub retention_days: u32,
    pub cutoff_at_ms: i64,
    pub candidate_count: usize,
    pub missing_timestamp_count: usize,
    pub destination: String,
    pub capability: ArchiveCapability,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLifecycleOperationPayload {
    pub session_id: String,
    pub archived: bool,
    pub current_provider: String,
    pub session_provider: String,
    pub provider_mismatch: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveMaintenancePayload {
    pub due: bool,
    pub deferred: bool,
    pub cutoff_at_ms: i64,
    pub candidate_count: usize,
    pub archived_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub elapsed_ms: u128,
    pub last_completed_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCompatibilityPayload {
    pub current_provider: String,
    pub active_count: usize,
    pub mismatch_count: usize,
    pub missing_provider_count: usize,
    pub scan_generation: String,
    pub encrypted_content_warning: Option<String>,
    pub adaptation_available: bool,
    pub adaptation_message: String,
    pub scan_elapsed_ms: u128,
    pub archived_rollouts_traversed: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZedRemoteProjectsPayload {
    pub projects: Vec<ZedRemoteProject>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZedRemoteOpenPayload {
    pub url: String,
    pub strategy: ZedOpenStrategy,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteLocalSessionRequest {
    pub session_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub db_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayPayload {
    pub authenticated: bool,
    pub auth_source: String,
    pub account_label: Option<String>,
    pub config_path: String,
    pub configured: bool,
    pub requires_openai_auth: bool,
    pub has_bearer_token: bool,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayFilesPayload {
    pub config_path: String,
    pub auth_path: String,
    pub config_contents: String,
    pub auth_status: LiveAuthStatusPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveAuthStatusPayload {
    pub authenticated: bool,
    pub source: String,
    pub account_label: Option<String>,
    pub action_required: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelaySwitchPayload {
    pub settings: BackendSettings,
    pub relay: RelayPayload,
    pub settings_path: String,
    pub user_scripts: Value,
    pub previous_provider: String,
    pub current_provider: String,
    pub provider_changed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsBackfillPayload {
    pub settings: BackendSettings,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEntriesPayload {
    pub settings: BackendSettings,
    pub entries: codex_plus_core::relay_config::CodexContextEntries,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveContextEntriesPayload {
    pub entries: codex_plus_core::relay_config::CodexContextEntries,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractRelayCommonConfigPayload {
    pub common_config_contents: String,
    pub profile_config_contents: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayProfileTestPayload {
    pub http_status: u16,
    pub endpoint: String,
    pub response_preview: String,
    pub compatibility_fallback_used: bool,
    pub initial_http_status: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepwiseTestPayload {
    pub item_count: usize,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayProfileModelsPayload {
    pub models: Vec<String>,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDoctorCheck {
    pub id: String,
    pub title: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDoctorPayload {
    pub profile_name: String,
    pub model: String,
    pub summary: String,
    pub recommendation: String,
    pub checks: Vec<ProviderDoctorCheck>,
    pub compatibility_fallback_used: bool,
    pub initial_http_status: Option<u16>,
    pub request_http_status: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvConflictsPayload {
    pub conflicts: Vec<codex_plus_core::env_conflicts::EnvConflict>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveEnvConflictsRequest {
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveEnvConflictsPayload {
    pub removed: Vec<codex_plus_core::env_conflicts::EnvConflictRemoval>,
    pub backup_path: Option<String>,
    pub remaining: Vec<codex_plus_core::env_conflicts::EnvConflict>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRelayFileRequest {
    pub kind: String,
    pub contents: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackfillRelayProfileRequest {
    pub settings: BackendSettings,
    pub profile_id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSettingsRequest {
    pub settings: BackendSettings,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEntryRequest {
    pub settings: BackendSettings,
    pub kind: String,
    pub id: String,
    pub toml_body: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDeleteRequest {
    pub settings: BackendSettings,
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractRelayCommonConfigRequest {
    pub config_contents: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    #[serde(default)]
    pub app_path: String,
    #[serde(default = "default_debug_port")]
    pub debug_port: u16,
    #[serde(default = "default_helper_port")]
    pub helper_port: u16,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRequest {
    #[serde(default = "default_log_lines")]
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogsPayload {
    pub path: String,
    pub text: String,
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticsPayload {
    pub report: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WatcherPayload {
    pub enabled: bool,
    pub disabled_flag: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdsPayload {
    pub version: u64,
    pub ads: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScriptMarketPayload {
    pub market: Value,
    pub user_scripts: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupPayload {
    pub show_update: bool,
}

#[tauri::command]
pub async fn load_settings() -> CommandResult<SettingsPayload> {
    // A panic here used to reject the whole invoke, which left the UI with no settings baseline
    // and no reason for it. Answering with the fallback payload keeps the failure legible.
    settle_blocking(
        tauri::async_runtime::spawn_blocking(|| load_settings_blocking()),
        "设置读取中断；请重新加载设置。",
        fallback_settings_payload,
    )
    .await
}

fn load_settings_blocking() -> CommandResult<SettingsPayload> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let result = (|| -> anyhow::Result<()> {
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        live_state::recover_locked()?;
        migrate_legacy_profile_auth_locked()?;
        Ok(())
    })();
    match result {
        Ok(()) => settings_payload("设置已加载。", "设置读取失败"),
        Err(error) => failed(
            &format!("设置安全检查失败：{error}"),
            fallback_settings_payload(),
        ),
    }
}

#[tauri::command]
pub async fn save_settings(settings: BackendSettings) -> CommandResult<SettingsPayload> {
    settle_blocking(
        tauri::async_runtime::spawn_blocking(move || save_settings_blocking(settings)),
        "设置保存中断；请重新加载设置后再试。",
        fallback_settings_payload,
    )
    .await
}

fn save_settings_blocking(settings: BackendSettings) -> CommandResult<SettingsPayload> {
    let result = save_settings_with_provider_guard_at(&ProviderCommitPaths::defaults(), settings);
    match result {
        Ok(()) => settings_payload("设置已保存。", "设置保存后重新读取失败"),
        Err(error) => match settings_payload_value() {
            Ok(payload) => failed(error.user_message(), payload),
            Err((_, payload)) => failed(error.user_message(), payload),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenericSettingsSaveError {
    ProviderOwnedDifference,
    ProviderAuthProhibited,
    PersistedSettingsInvalid,
    PersistedSettingsChanged,
    SecureStorageFailed,
}

impl GenericSettingsSaveError {
    fn user_message(self) -> &'static str {
        match self {
            Self::ProviderOwnedDifference => "保存设置失败；供应商相关改动必须使用统一供应商保存。",
            Self::ProviderAuthProhibited => "保存设置失败；供应商认证内容不能通过通用设置保存。",
            Self::PersistedSettingsInvalid => "保存设置失败；本地设置文件无效，请先修复设置文件。",
            Self::PersistedSettingsChanged => "保存设置失败；本地设置已更新，请重新加载后再试。",
            Self::SecureStorageFailed => "保存设置失败；安全存储事务未能完成。",
        }
    }
}

impl std::fmt::Display for GenericSettingsSaveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::ProviderOwnedDifference => "provider-owned settings differ from persisted state",
            Self::ProviderAuthProhibited => "provider-owned authContents is prohibited",
            Self::PersistedSettingsInvalid => "persisted settings are invalid",
            Self::PersistedSettingsChanged => "persisted settings generation changed",
            Self::SecureStorageFailed => "secure settings transaction failed",
        })
    }
}

impl std::error::Error for GenericSettingsSaveError {}

pub(crate) fn settings_snapshot_for_ui_projection(
    mut settings: BackendSettings,
) -> Result<BackendSettings, GenericSettingsSaveError> {
    crate::provider_commit::validate_responses_only_settings(&settings)
        .map_err(|_| GenericSettingsSaveError::PersistedSettingsInvalid)?;
    let (common_without_context, extracted_context) =
        split_relay_context_config_sections(&settings.relay_common_config_contents);
    settings.relay_common_config_contents =
        codex_plus_core::relay_config::sanitize_common_config_contents(&common_without_context);
    settings.relay_context_config_contents =
        relay_join_config_sections(&[&settings.relay_context_config_contents, &extracted_context]);
    settings.relay_context_config_contents =
        codex_plus_core::relay_config::sanitize_common_config_contents(
            &settings.relay_context_config_contents,
        );
    for profile in &mut settings.relay_profiles {
        codex_plus_core::relay_config::normalize_relay_profile_for_storage(profile)
            .map_err(|_| GenericSettingsSaveError::PersistedSettingsInvalid)?;
    }
    Ok(settings)
}

fn serialize_settings_with_raw_provider_snapshot(
    settings: &BackendSettings,
    persisted_value: Option<&Value>,
) -> Result<Vec<u8>, GenericSettingsSaveError> {
    let bytes = serialize_settings_without_profile_auth(settings)
        .map_err(|_| GenericSettingsSaveError::SecureStorageFailed)?;
    let Some(persisted) = persisted_value.and_then(Value::as_object) else {
        return Ok(bytes);
    };
    let mut next: Value = serde_json::from_slice(&bytes)
        .map_err(|_| GenericSettingsSaveError::SecureStorageFailed)?;
    let next = next
        .as_object_mut()
        .ok_or(GenericSettingsSaveError::SecureStorageFailed)?;
    for key in [
        "relayProfilesEnabled",
        "relayProfiles",
        "aggregateRelayProfiles",
        "activeRelayId",
        "activeAggregateRelayId",
        "relayBaseUrl",
        "relayApiKey",
        "relayCommonConfigContents",
        "relayContextConfigContents",
        "relayTestModel",
    ] {
        match persisted.get(key) {
            Some(value) => {
                next.insert(key.to_string(), value.clone());
            }
            None => {
                next.remove(key);
            }
        }
    }
    serde_json::to_vec_pretty(&next).map_err(|_| GenericSettingsSaveError::SecureStorageFailed)
}

pub(crate) fn ui_provider_topology_projection(
    mut settings: BackendSettings,
) -> Result<crate::provider_commit::ProviderOwnedTopologyDraft, GenericSettingsSaveError> {
    let all_context = codex_plus_core::relay_config::list_context_entries_from_common_config(
        &settings.relay_context_config_contents,
    )
    .map_err(|_| GenericSettingsSaveError::PersistedSettingsInvalid)?;
    let all_context = RelayContextSelection {
        mcp_servers: all_context
            .mcp_servers
            .into_iter()
            .map(|entry| entry.id)
            .collect(),
        skills: all_context
            .skills
            .into_iter()
            .map(|entry| entry.id)
            .collect(),
        plugins: all_context
            .plugins
            .into_iter()
            .map(|entry| entry.id)
            .collect(),
    };
    for profile in &mut settings.relay_profiles {
        sanitize_profile_after_core_normalize(profile);
        if profile.relay_mode == codex_plus_core::settings::RelayMode::MixedApi {
            profile.relay_mode = codex_plus_core::settings::RelayMode::Official;
            profile.official_mix_api_key = true;
        }
        if profile.relay_mode == codex_plus_core::settings::RelayMode::Official
            && !profile.official_mix_api_key
        {
            profile.model.clear();
            profile.base_url.clear();
            profile.upstream_base_url.clear();
            profile.api_key.clear();
            profile.config_contents.clear();
        }
        if profile.relay_mode == codex_plus_core::settings::RelayMode::Aggregate {
            profile.base_url.clear();
            profile.upstream_base_url.clear();
            profile.api_key.clear();
            profile.protocol = codex_plus_core::settings::RelayProtocol::Responses;
            profile.official_mix_api_key = false;
            profile.config_contents.clear();
            profile.context_window.clear();
            profile.auto_compact_limit.clear();
            profile.model_list.clear();
            profile.model_windows.clear();
        }
        if !profile.context_selection_initialized {
            profile.context_selection = all_context.clone();
            profile.context_selection_initialized = true;
        }
    }
    if !settings
        .relay_profiles
        .iter()
        .any(|profile| profile.id == settings.active_relay_id)
    {
        settings.active_relay_id = settings
            .relay_profiles
            .first()
            .map(|profile| profile.id.clone())
            .unwrap_or_else(|| "default".to_string());
    }
    let api_profile_ids = settings
        .relay_profiles
        .iter()
        .filter(|profile| {
            profile.relay_mode != codex_plus_core::settings::RelayMode::Aggregate
                && !profile.base_url.trim().is_empty()
                && !profile.api_key.trim().is_empty()
        })
        .map(|profile| profile.id.clone())
        .collect::<HashSet<_>>();
    settings.aggregate_relay_profiles = settings
        .relay_profiles
        .iter()
        .filter(|profile| profile.relay_mode == codex_plus_core::settings::RelayMode::Aggregate)
        .map(|profile| {
            let mut aggregate = settings
                .aggregate_relay_profiles
                .iter()
                .find(|aggregate| aggregate.id == profile.id)
                .cloned()
                .unwrap_or_else(|| codex_plus_core::settings::AggregateRelayProfile {
                    id: profile.id.clone(),
                    name: profile.name.clone(),
                    strategy: Default::default(),
                    members: Vec::new(),
                });
            aggregate.name = if profile.name.is_empty() {
                aggregate.name
            } else {
                profile.name.clone()
            };
            let mut member_ids = HashSet::new();
            aggregate.members.retain(|member| {
                !member.relay_id.is_empty()
                    && member_ids.insert(member.relay_id.clone())
                    && (api_profile_ids.is_empty() || api_profile_ids.contains(&member.relay_id))
            });
            for member in &mut aggregate.members {
                member.weight = member.weight.clamp(1, 999);
            }
            aggregate
        })
        .collect();
    if let Some(active) = settings
        .relay_profiles
        .iter()
        .find(|profile| profile.id == settings.active_relay_id)
    {
        let is_aggregate = active.relay_mode == codex_plus_core::settings::RelayMode::Aggregate;
        settings.relay_base_url = if is_aggregate {
            "http://127.0.0.1:57321/v1".to_string()
        } else {
            active.base_url.clone()
        };
        settings.relay_api_key = active.api_key.clone();
        settings.active_aggregate_relay_id = if is_aggregate {
            active.id.clone()
        } else {
            String::new()
        };
    }
    Ok(crate::provider_commit::ProviderOwnedTopologyDraft::from_settings(&settings))
}

pub(crate) fn save_settings_with_provider_guard_at(
    paths: &ProviderCommitPaths,
    incoming: BackendSettings,
) -> Result<(), GenericSettingsSaveError> {
    save_settings_with_provider_guard_at_observed(paths, incoming, || Ok(()))
}

pub(crate) fn save_settings_with_provider_guard_at_observed(
    paths: &ProviderCommitPaths,
    incoming: BackendSettings,
    after_snapshot: impl FnOnce() -> anyhow::Result<()>,
) -> Result<(), GenericSettingsSaveError> {
    use crate::provider_commit::ProviderOwnedTopologyDraft;

    crate::provider_commit::validate_responses_only_settings(&incoming)
        .map_err(|_| GenericSettingsSaveError::PersistedSettingsInvalid)?;
    let _guard = live_state::lock().map_err(|_| GenericSettingsSaveError::SecureStorageFailed)?;
    live_state::prepare_secret_paths_at(&paths.app_state, &paths.settings_path, &paths.codex_home)
        .map_err(|_| GenericSettingsSaveError::SecureStorageFailed)?;
    live_state::recover_locked_at(&paths.app_state)
        .map_err(|_| GenericSettingsSaveError::SecureStorageFailed)?;
    let persisted_bytes = match std::fs::read(&paths.settings_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(GenericSettingsSaveError::SecureStorageFailed),
    };
    after_snapshot().map_err(|_| GenericSettingsSaveError::SecureStorageFailed)?;
    let (persisted, persisted_value) = match persisted_bytes.as_ref() {
        Some(persisted_bytes) => {
            let value: Value = serde_json::from_slice(persisted_bytes)
                .map_err(|_| GenericSettingsSaveError::PersistedSettingsInvalid)?;
            let settings = serde_json::from_value(value.clone())
                .map_err(|_| GenericSettingsSaveError::PersistedSettingsInvalid)?;
            (settings, Some(value))
        }
        None => (BackendSettings::default(), None),
    };
    crate::provider_commit::validate_responses_only_settings(&persisted)
        .map_err(|_| GenericSettingsSaveError::PersistedSettingsInvalid)?;
    if persisted
        .relay_profiles
        .iter()
        .any(|profile| !profile.auth_contents.is_empty())
        || incoming
            .relay_profiles
            .iter()
            .any(|profile| !profile.auth_contents.is_empty())
    {
        return Err(GenericSettingsSaveError::ProviderAuthProhibited);
    }
    let persisted_topology = ProviderOwnedTopologyDraft::from_settings(&persisted);
    let incoming_topology = ProviderOwnedTopologyDraft::from_settings(&incoming);
    let persisted_ui_topology =
        ui_provider_topology_projection(settings_snapshot_for_ui_projection(persisted.clone())?)?;
    let incoming_bytes = serde_json::to_vec(&incoming_topology)
        .map_err(|_| GenericSettingsSaveError::SecureStorageFailed)?;
    let matches_raw = serde_json::to_vec(&persisted_topology)
        .map_err(|_| GenericSettingsSaveError::SecureStorageFailed)?
        == incoming_bytes;
    let matches_ui = serde_json::to_vec(&persisted_ui_topology)
        .map_err(|_| GenericSettingsSaveError::SecureStorageFailed)?
        == incoming_bytes;
    if !matches_raw && !matches_ui {
        return Err(GenericSettingsSaveError::ProviderOwnedDifference);
    }

    let normalized_unrelated = normalize_settings_before_save(incoming);
    let merged = persisted_topology.apply_to(&normalized_unrelated);
    let bytes = serialize_settings_with_raw_provider_snapshot(&merged, persisted_value.as_ref())?;
    let generation_matches = match persisted_bytes.as_ref() {
        Some(expected) => std::fs::read(&paths.settings_path)
            .map(|current| current == *expected)
            .unwrap_or(false),
        None => !paths.settings_path.exists(),
    };
    if !generation_matches {
        return Err(GenericSettingsSaveError::PersistedSettingsChanged);
    }
    live_state::commit_locked_verified_at(
        &paths.app_state,
        &[FileMutation::bytes(paths.settings_path.clone(), bytes)],
        || Ok(()),
    )
    .map_err(|_| GenericSettingsSaveError::SecureStorageFailed)
}

#[tauri::command]
pub async fn list_local_sessions(
    request: Option<LocalSessionsRequest>,
) -> CommandResult<LocalSessionsPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        list_local_sessions_blocking(request.unwrap_or_default())
    })
    .await
    .expect("blocking command panicked")
}

fn load_local_session_inventory() -> (
    Vec<PathBuf>,
    Vec<codex_plus_data::LocalSession>,
    Vec<String>,
) {
    let home = codex_plus_core::codex_sqlite::default_codex_home_dir();
    let db_paths = codex_plus_core::codex_sqlite::codex_session_db_paths_from_home(&home);
    let mut sessions = Vec::new();
    let mut errors = Vec::new();
    for db_path in &db_paths {
        let adapter = local_session_adapter(db_path);
        match adapter.list_local_sessions() {
            Ok(mut items) => sessions.append(&mut items),
            Err(error) if db_path.exists() => {
                errors.push(format!("{}: {error}", db_path.to_string_lossy()));
            }
            Err(_) => {}
        }
    }
    sessions.sort_by(|left, right| {
        right
            .updated_at_ms
            .cmp(&left.updated_at_ms)
            .then_with(|| right.id.cmp(&left.id))
    });
    let mut seen_session_ids = std::collections::HashSet::new();
    sessions.retain(|session| seen_session_ids.insert(session.id.clone()));
    (db_paths, sessions, errors)
}

fn session_cursor(session: &codex_plus_data::LocalSession) -> String {
    format!(
        "{}|{}",
        session.updated_at_ms.unwrap_or(i64::MIN),
        session.id
    )
}

fn paginate_local_sessions(
    sessions: Vec<codex_plus_data::LocalSession>,
    archived: bool,
    cursor: Option<&str>,
    page_size: usize,
) -> (Vec<codex_plus_data::LocalSession>, Option<String>, bool) {
    let filtered = sessions
        .into_iter()
        .filter(|session| session.archived == archived)
        .collect::<Vec<_>>();
    let start = match cursor {
        Some(cursor) => match filtered
            .iter()
            .position(|session| session_cursor(session) == cursor)
        {
            Some(position) => position + 1,
            None => return (Vec::new(), None, false),
        },
        None => 0,
    };
    let mut page = filtered
        .into_iter()
        .skip(start)
        .take(page_size + 1)
        .collect::<Vec<_>>();
    let has_more = page.len() > page_size;
    page.truncate(page_size);
    let next_cursor = has_more.then(|| page.last().map(session_cursor)).flatten();
    (page, next_cursor, true)
}

fn list_local_sessions_blocking(
    request: LocalSessionsRequest,
) -> CommandResult<LocalSessionsPayload> {
    let started = Instant::now();
    let (db_paths, sessions, errors) = load_local_session_inventory();
    let active_count = sessions.iter().filter(|session| !session.archived).count();
    let archived_count = sessions.len().saturating_sub(active_count);
    let page_size = request.page_size.unwrap_or(100).clamp(1, 200);
    let (page, next_cursor, cursor_valid) = paginate_local_sessions(
        sessions,
        request.archived,
        request.cursor.as_deref(),
        page_size,
    );
    let payload = LocalSessionsPayload {
        db_path: db_paths
            .first()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        db_paths: db_paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
        sessions: page,
        active_count,
        archived_count,
        archived: request.archived,
        next_cursor,
        page_size,
        elapsed_ms: started.elapsed().as_millis(),
    };
    if !cursor_valid {
        failed("会话列表游标已过期，请刷新当前列表。", payload)
    } else if errors.is_empty() {
        ok(
            &format!("已读取 {} 个本地会话。", payload.sessions.len()),
            payload,
        )
    } else {
        failed(
            &format!("读取部分本地会话失败：{}", errors.join("; ")),
            payload,
        )
    }
}

const ARCHIVE_CHECK_INTERVAL_MS: i64 = 24 * 60 * 60 * 1_000;
/// Upper bound for one provider-facing probe.
///
/// The pinned core builds these HTTP clients without a timeout of their own, so an upstream that
/// accepts a connection and then stalls would otherwise keep the request — and, for the model
/// probe, the coordinator lock its result needs — outstanding for the life of the process.
const PROVIDER_PROBE_TIMEOUT: Duration = Duration::from_secs(30);

const ARCHIVE_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const ARCHIVE_CAPABILITY_TIMEOUT: Duration = Duration::from_secs(5);

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

fn session_lifecycle_settings_path() -> PathBuf {
    codex_plus_core::paths::default_app_state_dir().join("session-lifecycle.json")
}

fn read_session_lifecycle_settings_from(path: &Path) -> anyhow::Result<SessionLifecycleSettings> {
    if !path.exists() {
        return Ok(SessionLifecycleSettings::default());
    }
    let settings: SessionLifecycleSettings = serde_json::from_slice(&std::fs::read(path)?)?;
    validate_session_lifecycle_settings(&settings)?;
    Ok(settings)
}

fn write_session_lifecycle_settings_to(
    path: &Path,
    settings: &SessionLifecycleSettings,
) -> anyhow::Result<()> {
    validate_session_lifecycle_settings(settings)?;
    if settings.archive_enabled && !settings.first_run_reviewed {
        anyhow::bail!("启用自动归档前必须先确认候选预览");
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("生命周期设置路径缺少父目录"))?;
    std::fs::create_dir_all(parent)?;
    let temp_path = path.with_extension("json.tmp");
    let contents = serde_json::to_vec_pretty(settings)?;
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&temp_path)?;
        file.write_all(&contents)?;
        file.sync_all()?;
        std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    std::fs::write(&temp_path, contents)?;
    std::fs::rename(temp_path, path)?;
    Ok(())
}

fn validate_session_lifecycle_settings(settings: &SessionLifecycleSettings) -> anyhow::Result<()> {
    if !(1..=3_650).contains(&settings.retention_days) {
        anyhow::bail!("保留天数必须在 1 到 3650 之间");
    }
    Ok(())
}

fn archive_cutoff_at_ms(retention_days: u32, current_time_ms: i64) -> i64 {
    current_time_ms.saturating_sub(i64::from(retention_days) * ARCHIVE_CHECK_INTERVAL_MS)
}

fn archive_candidates(
    sessions: &[codex_plus_data::LocalSession],
    cutoff_at_ms: i64,
) -> (Vec<String>, usize) {
    let mut missing_timestamp_count = 0;
    let candidates = sessions
        .iter()
        .filter(|session| !session.archived)
        .filter_map(|session| match session.updated_at_ms {
            Some(updated_at_ms) if updated_at_ms < cutoff_at_ms => Some(session.id.clone()),
            Some(_) => None,
            None => {
                missing_timestamp_count += 1;
                None
            }
        })
        .collect();
    (candidates, missing_timestamp_count)
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

fn codex_cli_from_app_dir(app_dir: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Some(app_bundle) = app_dir
        .ancestors()
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("app"))
    {
        return app_bundle.join("Contents").join("Resources").join("codex");
    }
    #[cfg(windows)]
    {
        return windows_codex_cli_from_app_dir(app_dir);
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        app_dir.join("codex")
    }
    #[cfg(all(target_os = "macos", not(windows)))]
    {
        app_dir.join("codex")
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn windows_codex_cli_from_app_dir(app_dir: &Path) -> PathBuf {
    let direct = app_dir.join("codex.exe");
    if direct.is_file() {
        return direct;
    }

    std::fs::read_dir(app_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("codex.exe"))
        .filter(|candidate| candidate.is_file())
        .max_by_key(|candidate| {
            candidate
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
        })
        .unwrap_or(direct)
}

/// Standalone Codex installs keep the CLI in a content-addressed subdirectory, e.g.
/// `%LOCALAPPDATA%\OpenAI\Codex\bin\<hash>\codex.exe`. Shared app-directory resolution only
/// probes the fixed roots, and on a machine that also carries the MS Store package it resolves
/// to `WindowsApps\...\app`, whose binaries cannot be spawned by path. The standalone CLI is
/// therefore the only usable target whenever it exists.
#[cfg_attr(not(windows), allow(dead_code))]
fn windows_standalone_codex_cli_in(local_app_data: &Path) -> Option<PathBuf> {
    [
        local_app_data.join("OpenAI").join("Codex").join("bin"),
        local_app_data.join("OpenAI").join("Codex"),
        local_app_data.join("Programs").join("OpenAI").join("Codex"),
    ]
    .into_iter()
    .map(|root| windows_codex_cli_from_app_dir(&root))
    .find(|candidate| candidate.is_file())
}

pub(crate) fn discover_target_codex_cli() -> anyhow::Result<PathBuf> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    let saved = settings.codex_app_path.trim();
    #[cfg(windows)]
    if saved.is_empty()
        && let Some(cli) = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .as_deref()
            .and_then(windows_standalone_codex_cli_in)
    {
        return Ok(cli);
    }
    let app_dir = codex_plus_core::app_paths::resolve_codex_app_dir_with_saved(
        None,
        (!saved.is_empty()).then_some(saved),
    )
    .ok_or_else(|| anyhow::anyhow!("未找到目标 Codex 或 ChatGPT 应用"))?;
    let cli = codex_cli_from_app_dir(&app_dir);
    if !cli.is_file() {
        anyhow::bail!("目标应用未包含原生 Codex CLI：{}", cli.to_string_lossy());
    }
    Ok(cli)
}

fn command_succeeds(cli: &Path, home: &Path, args: &[&str]) -> bool {
    let Ok(mut child) = crate::platform_command::background_command(cli)
        .args(args)
        .env("CODEX_HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if started.elapsed() < ARCHIVE_CAPABILITY_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

fn archive_capability_for(home: &Path) -> ArchiveCapability {
    match discover_target_codex_cli() {
        Ok(cli)
            if command_succeeds(&cli, home, &["archive", "--help"])
                && command_succeeds(&cli, home, &["unarchive", "--help"]) =>
        {
            ArchiveCapability {
                available: true,
                cli_path: Some(cli.to_string_lossy().to_string()),
                message: "目标 Codex 支持原生归档与恢复。".to_string(),
            }
        }
        Ok(cli) => ArchiveCapability {
            available: false,
            cli_path: Some(cli.to_string_lossy().to_string()),
            message: "目标 Codex CLI 不支持 archive/unarchive，请先更新目标客户端。".to_string(),
        },
        Err(error) => ArchiveCapability {
            available: false,
            cli_path: None,
            message: error.to_string(),
        },
    }
}

fn run_native_session_operation(
    cli: &Path,
    home: &Path,
    operation: &str,
    session_id: &str,
) -> anyhow::Result<()> {
    if !matches!(operation, "archive" | "unarchive") || !is_uuid(session_id) {
        anyhow::bail!("原生会话操作仅接受 UUID 和 archive/unarchive");
    }
    let mut child = crate::platform_command::background_command(cli)
        .arg(operation)
        .arg(session_id)
        .env("CODEX_HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            anyhow::bail!("目标 Codex CLI 返回失败状态：{status}");
        }
        if started.elapsed() >= ARCHIVE_OPERATION_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("目标 Codex CLI 会话操作超时");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn find_local_session(session_id: &str) -> Option<codex_plus_data::LocalSession> {
    let (_, sessions, _) = load_local_session_inventory();
    sessions
        .into_iter()
        .find(|session| session.id == session_id)
}

fn validate_native_session_postcondition(
    home: &Path,
    session_id: &str,
    archived: bool,
) -> anyhow::Result<codex_plus_data::LocalSession> {
    let session = find_local_session(session_id)
        .ok_or_else(|| anyhow::anyhow!("原生操作后未找到会话状态记录"))?;
    let expected_root = if archived {
        home.join("archived_sessions")
    } else {
        home.join("sessions")
    };
    let rollout_path = std::fs::canonicalize(&session.rollout_path)
        .map_err(|error| anyhow::anyhow!("原生操作后的 rollout 不可读取：{error}"))?;
    let expected_root = std::fs::canonicalize(expected_root)
        .map_err(|error| anyhow::anyhow!("原生归档目录不可读取：{error}"))?;
    if session.archived != archived || !rollout_path.starts_with(&expected_root) {
        anyhow::bail!("原生操作后 rollout 位置与归档状态不一致");
    }
    Ok(session)
}

fn native_session_operation(
    home: &Path,
    session_id: &str,
    archived: bool,
) -> anyhow::Result<codex_plus_data::LocalSession> {
    let capability = archive_capability_for(home);
    if !capability.available {
        anyhow::bail!(capability.message);
    }
    let cli = capability
        .cli_path
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("未找到目标 Codex CLI"))?;
    run_native_session_operation(
        &cli,
        home,
        if archived { "archive" } else { "unarchive" },
        session_id,
    )?;
    validate_native_session_postcondition(home, session_id, archived)
}

fn session_operation_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lifecycle_settings_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn persist_last_completed_at_ms(completed_at_ms: i64) -> anyhow::Result<SessionLifecycleSettings> {
    let _guard = lifecycle_settings_mutex()
        .lock()
        .map_err(|_| anyhow::anyhow!("生命周期设置锁已损坏"))?;
    let mut latest = read_session_lifecycle_settings_from(&session_lifecycle_settings_path())?;
    latest.last_completed_at_ms = Some(completed_at_ms);
    write_session_lifecycle_settings_to(&session_lifecycle_settings_path(), &latest)?;
    Ok(latest)
}

#[tauri::command]
pub async fn load_session_lifecycle_settings() -> CommandResult<SessionLifecycleSettings> {
    tauri::async_runtime::spawn_blocking(|| {
        match read_session_lifecycle_settings_from(&session_lifecycle_settings_path()) {
            Ok(settings) => ok("会话归档设置已加载。", settings),
            Err(error) => failed(
                &format!("读取会话归档设置失败：{error}"),
                SessionLifecycleSettings::default(),
            ),
        }
    })
    .await
    .expect("blocking command panicked")
}

#[tauri::command]
pub async fn save_session_lifecycle_settings(
    settings: SessionLifecycleSettings,
) -> CommandResult<SessionLifecycleSettings> {
    tauri::async_runtime::spawn_blocking(move || {
        let Ok(_guard) = lifecycle_settings_mutex().lock() else {
            return failed("生命周期设置锁已损坏，请重启管理器后再试。", settings);
        };
        match write_session_lifecycle_settings_to(&session_lifecycle_settings_path(), &settings) {
            Ok(()) => ok("会话归档设置已保存。", settings),
            Err(error) => failed(&format!("保存会话归档设置失败：{error}"), settings),
        }
    })
    .await
    .expect("blocking command panicked")
}

#[tauri::command]
pub async fn preview_session_archive(
    request: Option<ArchivePreviewRequest>,
) -> CommandResult<ArchivePreviewPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let retention_days = request
            .and_then(|request| request.retention_days)
            .unwrap_or(30);
        if !(1..=3_650).contains(&retention_days) {
            return failed(
                "保留天数必须在 1 到 3650 之间。",
                ArchivePreviewPayload {
                    retention_days,
                    cutoff_at_ms: 0,
                    candidate_count: 0,
                    missing_timestamp_count: 0,
                    destination: String::new(),
                    capability: ArchiveCapability {
                        available: false,
                        cli_path: None,
                        message: "保留天数无效。".to_string(),
                    },
                },
            );
        }
        let home = codex_plus_core::codex_sqlite::default_codex_home_dir();
        let cutoff_at_ms = archive_cutoff_at_ms(retention_days, now_ms());
        let (_, sessions, errors) = load_local_session_inventory();
        let (candidate_ids, missing_timestamp_count) = archive_candidates(&sessions, cutoff_at_ms);
        let payload = ArchivePreviewPayload {
            retention_days,
            cutoff_at_ms,
            candidate_count: candidate_ids.len(),
            missing_timestamp_count,
            destination: home.join("archived_sessions").to_string_lossy().to_string(),
            capability: archive_capability_for(&home),
        };
        if errors.is_empty() {
            ok("归档候选预览已生成，尚未修改任何会话。", payload)
        } else {
            failed(
                &format!("部分会话库读取失败：{}", errors.join("; ")),
                payload,
            )
        }
    })
    .await
    .expect("blocking command panicked")
}

fn current_effective_provider_from_home(home: &Path) -> String {
    std::fs::read_to_string(home.join("config.toml"))
        .ok()
        .and_then(|contents| contents.parse::<toml_edit::DocumentMut>().ok())
        .and_then(|document| {
            document
                .get("model_provider")
                .and_then(toml_edit::Item::as_str)
                .map(str::to_string)
        })
        .filter(|provider| !provider.trim().is_empty())
        .unwrap_or_else(|| "openai".to_string())
}

#[tauri::command]
pub async fn archive_local_session(
    request: SessionLifecycleOperationRequest,
) -> CommandResult<SessionLifecycleOperationPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        session_lifecycle_operation_blocking(request, true)
    })
    .await
    .expect("blocking command panicked")
}

#[tauri::command]
pub async fn restore_local_session(
    request: SessionLifecycleOperationRequest,
) -> CommandResult<SessionLifecycleOperationPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        session_lifecycle_operation_blocking(request, false)
    })
    .await
    .expect("blocking command panicked")
}

fn session_lifecycle_operation_blocking(
    request: SessionLifecycleOperationRequest,
    archived: bool,
) -> CommandResult<SessionLifecycleOperationPayload> {
    let session_id = request.session_id.trim().to_string();
    let home = codex_plus_core::codex_sqlite::default_codex_home_dir();
    let current_provider = current_effective_provider_from_home(&home);
    let fallback = SessionLifecycleOperationPayload {
        session_id: session_id.clone(),
        archived,
        current_provider: current_provider.clone(),
        session_provider: String::new(),
        provider_mismatch: false,
    };
    if !is_uuid(&session_id) {
        return failed("会话 ID 不是有效 UUID。", fallback);
    }
    let Ok(_guard) = session_operation_mutex().lock() else {
        return failed("会话操作锁已损坏，请重启管理器后再试。", fallback);
    };
    let Some(before) = find_local_session(&session_id) else {
        return failed("未找到会话。", fallback);
    };
    if before.archived == archived {
        return ok(
            if archived {
                "会话已经归档。"
            } else {
                "会话已经恢复。"
            },
            SessionLifecycleOperationPayload {
                session_provider: before.model_provider.clone(),
                provider_mismatch: before.model_provider != current_provider,
                ..fallback
            },
        );
    }
    if archived && target_client_running() != Some(false) {
        return failed(
            "目标 Codex/ChatGPT 正在运行或占用状态不可确认；请关闭目标客户端后再归档。",
            fallback,
        );
    }
    match native_session_operation(&home, &session_id, archived) {
        Ok(after) => ok(
            if archived {
                "会话已原生归档。"
            } else {
                "会话已原生恢复。"
            },
            SessionLifecycleOperationPayload {
                session_provider: after.model_provider.clone(),
                provider_mismatch: after.model_provider != current_provider,
                ..fallback
            },
        ),
        Err(error) => failed(
            &format!(
                "{}失败：{error}",
                if archived {
                    "原生归档"
                } else {
                    "原生恢复"
                }
            ),
            fallback,
        ),
    }
}

fn target_client_running() -> Option<bool> {
    #[cfg(target_os = "macos")]
    {
        for name in ["ChatGPT", "Codex", "codex"] {
            match crate::platform_command::status_bounded(
                std::process::Command::new("/usr/bin/pgrep").args(["-x", name]),
                crate::platform_command::HELPER_TIMEOUT,
                "the process probe",
            ) {
                Ok(status) if status.success() => return Some(true),
                Ok(_) => {}
                Err(_) => return None,
            }
        }
        Some(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[tauri::command]
pub async fn run_session_archive_maintenance(
    force: Option<bool>,
) -> CommandResult<ArchiveMaintenancePayload> {
    tauri::async_runtime::spawn_blocking(move || {
        run_session_archive_maintenance_blocking(force.unwrap_or(false))
    })
    .await
    .expect("blocking command panicked")
}

// The daily interval only guards the automatic pass; a user-initiated check runs regardless,
// because clicking a button and silently being told "not due yet" reads as a broken button.
fn archive_maintenance_due(
    settings: &SessionLifecycleSettings,
    force: bool,
    current_time_ms: i64,
) -> bool {
    settings.archive_enabled
        && settings.first_run_reviewed
        && (force
            || settings.last_completed_at_ms.is_none_or(|last| {
                current_time_ms.saturating_sub(last) >= ARCHIVE_CHECK_INTERVAL_MS
            }))
}

fn run_session_archive_maintenance_blocking(
    force: bool,
) -> CommandResult<ArchiveMaintenancePayload> {
    let started = Instant::now();
    let current_time_ms = now_ms();
    let settings = match read_session_lifecycle_settings_from(&session_lifecycle_settings_path()) {
        Ok(settings) => settings,
        Err(error) => {
            return failed(
                &format!("读取会话归档设置失败：{error}"),
                ArchiveMaintenancePayload {
                    due: false,
                    deferred: false,
                    cutoff_at_ms: 0,
                    candidate_count: 0,
                    archived_count: 0,
                    skipped_count: 0,
                    failed_count: 0,
                    elapsed_ms: started.elapsed().as_millis(),
                    last_completed_at_ms: None,
                },
            );
        }
    };
    let due = archive_maintenance_due(&settings, force, current_time_ms);
    let cutoff_at_ms = archive_cutoff_at_ms(settings.retention_days, current_time_ms);
    let mut payload = ArchiveMaintenancePayload {
        due,
        deferred: false,
        cutoff_at_ms,
        candidate_count: 0,
        archived_count: 0,
        skipped_count: 0,
        failed_count: 0,
        elapsed_ms: 0,
        last_completed_at_ms: settings.last_completed_at_ms,
    };
    if !due {
        payload.elapsed_ms = started.elapsed().as_millis();
        return ok("自动归档尚未到检查时间。", payload);
    }
    let home = codex_plus_core::codex_sqlite::default_codex_home_dir();
    let (_, sessions, errors) = load_local_session_inventory();
    let (candidate_ids, _) = archive_candidates(&sessions, cutoff_at_ms);
    payload.candidate_count = candidate_ids.len();
    if !errors.is_empty() {
        payload.failed_count = errors.len();
    }
    if candidate_ids.is_empty() {
        payload.elapsed_ms = started.elapsed().as_millis();
        if !errors.is_empty() {
            return failed("自动归档无法完整读取会话库。", payload);
        }
        return match persist_last_completed_at_ms(now_ms()) {
            Ok(latest) => {
                payload.last_completed_at_ms = latest.last_completed_at_ms;
                ok("自动归档检查已完成，没有符合条件的会话。", payload)
            }
            Err(error) => failed(&format!("保存自动归档检查时间失败：{error}"), payload),
        };
    }
    if target_client_running() != Some(false) {
        payload.deferred = true;
        payload.elapsed_ms = started.elapsed().as_millis();
        return CommandResult {
            status: "not_checked".to_string(),
            message: "目标 Codex/ChatGPT 正在运行或占用状态不可确认，自动归档已延后。".to_string(),
            payload,
        };
    }
    let Ok(_guard) = session_operation_mutex().try_lock() else {
        payload.deferred = true;
        payload.elapsed_ms = started.elapsed().as_millis();
        return CommandResult {
            status: "not_checked".to_string(),
            message: "已有会话操作在执行，自动归档已合并到后续检查。".to_string(),
            payload,
        };
    };
    let capability = archive_capability_for(&home);
    let Some(cli) = capability.cli_path.as_deref().map(Path::new) else {
        payload.failed_count += payload.candidate_count.max(1);
        payload.elapsed_ms = started.elapsed().as_millis();
        return failed(&capability.message, payload);
    };
    if !capability.available {
        payload.failed_count += payload.candidate_count.max(1);
        payload.elapsed_ms = started.elapsed().as_millis();
        return failed(&capability.message, payload);
    }
    for session_id in candidate_ids {
        let still_eligible = find_local_session(&session_id).is_some_and(|session| {
            !session.archived
                && session
                    .updated_at_ms
                    .is_some_and(|updated_at_ms| updated_at_ms < cutoff_at_ms)
        });
        if !still_eligible {
            payload.skipped_count += 1;
            continue;
        }
        if target_client_running() != Some(false) {
            payload.skipped_count += 1;
            continue;
        }
        match run_native_session_operation(cli, &home, "archive", &session_id)
            .and_then(|_| validate_native_session_postcondition(&home, &session_id, true))
        {
            Ok(_) => payload.archived_count += 1,
            Err(_) => payload.failed_count += 1,
        }
    }
    match persist_last_completed_at_ms(now_ms()) {
        Ok(latest) => payload.last_completed_at_ms = latest.last_completed_at_ms,
        Err(_) => payload.failed_count += 1,
    }
    payload.elapsed_ms = started.elapsed().as_millis();
    log_manager_event(
        "manager.session_archive_maintenance.finish",
        json!({
            "candidateCount": payload.candidate_count,
            "archivedCount": payload.archived_count,
            "skippedCount": payload.skipped_count,
            "failedCount": payload.failed_count,
            "elapsedMs": payload.elapsed_ms,
            "cutoffAtMs": payload.cutoff_at_ms
        }),
    );
    if payload.failed_count == 0 {
        ok("自动归档检查已完成。", payload)
    } else {
        failed("自动归档部分完成，请查看失败计数。", payload)
    }
}

#[tauri::command]
pub async fn delete_local_session(
    request: DeleteLocalSessionRequest,
) -> CommandResult<DeleteResult> {
    tauri::async_runtime::spawn_blocking(move || delete_local_session_blocking(request))
        .await
        .expect("blocking command panicked")
}

fn delete_local_session_blocking(
    request: DeleteLocalSessionRequest,
) -> CommandResult<DeleteResult> {
    let session_id = request.session_id.trim();
    if session_id.is_empty() {
        return failed(
            "会话 ID 不能为空。",
            DeleteResult {
                status: codex_plus_core::models::DeleteStatus::Failed,
                session_id: String::new(),
                message: "会话 ID 不能为空。".to_string(),
                undo_token: None,
                backup_path: None,
            },
        );
    }
    let Ok(_guard) = session_operation_mutex().lock() else {
        return failed(
            "会话操作锁已损坏，请重启管理器后再试。",
            DeleteResult {
                status: codex_plus_core::models::DeleteStatus::Failed,
                session_id: session_id.to_string(),
                message: "会话操作锁已损坏。".to_string(),
                undo_token: None,
                backup_path: None,
            },
        );
    };
    let session = SessionRef {
        session_id: session_id.to_string(),
        title: request.title,
    };
    let mut candidate_paths = Vec::new();
    if let Some(path) = request.db_path.as_deref() {
        let path = PathBuf::from(path);
        if !candidate_paths.iter().any(|candidate| candidate == &path) {
            candidate_paths.push(path);
        }
    }
    for path in codex_plus_core::codex_sqlite::codex_session_db_paths_from_home(
        &codex_plus_core::codex_sqlite::default_codex_home_dir(),
    ) {
        if !candidate_paths.iter().any(|candidate| candidate == &path) {
            candidate_paths.push(path);
        }
    }
    log_manager_event(
        "manager.delete_local_session.start",
        json!({
            "session_id": session_id,
            "title": session.title,
            "requested_db_path": request.db_path,
            "candidate_paths": candidate_paths
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
        }),
    );
    let result = codex_plus_data::delete_local_from_paths(
        candidate_paths.clone(),
        codex_plus_data::BackupStore::new(
            codex_plus_core::paths::default_app_state_dir().join("backups"),
        ),
        &session,
    );
    log_manager_event(
        "manager.delete_local_session.finish",
        json!({
            "session_id": session_id,
            "final_status": format!("{:?}", result.status),
            "final_message": result.message,
            "candidate_paths": candidate_paths
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
        }),
    );
    let status = if matches!(
        result.status,
        codex_plus_core::models::DeleteStatus::LocalDeleted
    ) {
        "ok"
    } else {
        "failed"
    };
    CommandResult {
        status: status.to_string(),
        message: result.message.clone(),
        payload: result,
    }
}

fn local_session_adapter(db_path: &Path) -> codex_plus_data::SQLiteStorageAdapter {
    codex_plus_data::SQLiteStorageAdapter::new(
        db_path,
        codex_plus_data::BackupStore::new(
            codex_plus_core::paths::default_app_state_dir().join("backups"),
        ),
    )
}

fn normalize_settings_before_save(mut settings: BackendSettings) -> BackendSettings {
    if let Some(path) =
        codex_plus_core::app_paths::normalize_codex_app_path(Path::new(&settings.codex_app_path))
    {
        settings.codex_app_path = path.to_string_lossy().to_string();
    }
    settings.relay_common_config_contents =
        codex_plus_core::relay_config::sanitize_common_config_contents(
            &settings.relay_common_config_contents,
        );
    let (common_without_context, extracted_context) =
        split_relay_context_config_sections(&settings.relay_common_config_contents);
    settings.relay_common_config_contents = common_without_context;
    settings.relay_context_config_contents =
        relay_join_config_sections(&[&settings.relay_context_config_contents, &extracted_context]);
    settings.relay_context_config_contents =
        codex_plus_core::relay_config::sanitize_common_config_contents(
            &settings.relay_context_config_contents,
        );
    for profile in &mut settings.relay_profiles {
        migrate_profile_auth_to_config(profile);
        if let Err(error) =
            codex_plus_core::relay_config::normalize_relay_profile_for_storage(profile)
        {
            log_manager_event(
                "manager.normalize_relay_profile_for_storage.failed",
                json!({
                    "profileId": profile.id,
                    "profileName": profile.name,
                    "error": error.to_string()
                }),
            );
        }
        sanitize_profile_after_core_normalize(profile);
        match retain_provider_owned_profile_config(&profile.config_contents) {
            Ok(config) => profile.config_contents = config,
            Err(error) => log_manager_event(
                "manager.retain_provider_owned_profile_config.failed",
                json!({
                    "profileId": profile.id,
                    "profileName": profile.name,
                    "error": error.to_string()
                }),
            ),
        }
    }
    let common_config = relay_combined_common_config(&settings);
    if !common_config.trim().is_empty() {
        for profile in &mut settings.relay_profiles {
            if !profile.use_common_config || profile.config_contents.trim().is_empty() {
                continue;
            }
            match codex_plus_core::relay_config::strip_common_config_from_config(
                &profile.config_contents,
                &common_config,
            ) {
                Ok(stripped) => {
                    profile.config_contents =
                        strip_common_config_text_fallback(&stripped, &common_config);
                }
                Err(_) => {
                    profile.config_contents =
                        strip_common_config_text_fallback(&profile.config_contents, &common_config);
                }
            }
        }
    }
    settings.provider_sync_saved_providers =
        normalize_provider_sync_provider_list(settings.provider_sync_saved_providers);
    settings.provider_sync_manual_providers =
        normalize_provider_sync_provider_list(settings.provider_sync_manual_providers);
    settings.provider_sync_last_selected_provider = settings
        .provider_sync_last_selected_provider
        .trim()
        .to_string();
    scrub_legacy_managed_config_state(&mut settings);
    settings
}

fn migrate_profile_auth_to_config(profile: &mut RelayProfile) {
    let auth_api_key = profile_api_key_from_auth(&profile.auth_contents);
    if profile.api_key.trim().is_empty() {
        if let Some(api_key) = auth_api_key {
            profile.api_key = api_key;
        }
    }
    profile.auth_contents.clear();
}

fn sanitize_profile_after_core_normalize(profile: &mut RelayProfile) {
    if profile.relay_mode == codex_plus_core::settings::RelayMode::PureApi {
        let api_key = profile_api_key_from_auth(&profile.auth_contents)
            .or_else(|| provider_bearer_token_from_config(&profile.config_contents))
            .or_else(|| (!profile.api_key.trim().is_empty()).then(|| profile.api_key.clone()));
        if let Some(api_key) = api_key {
            if let Ok(config) =
                set_provider_config_bearer(&profile.config_contents, &api_key, Some(false))
            {
                profile.config_contents = config;
            }
            profile.api_key = api_key;
        }
    }
    profile.auth_contents.clear();
}

fn profile_api_key_from_auth(auth_contents: &str) -> Option<String> {
    serde_json::from_str::<Value>(auth_contents)
        .ok()?
        .get("OPENAI_API_KEY")?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn provider_bearer_token_from_config(config_contents: &str) -> Option<String> {
    provider_bearer_token_from_config_exact(config_contents)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn provider_bearer_token_from_config_exact(config_contents: &str) -> Option<String> {
    let doc: toml_edit::DocumentMut = config_contents.parse().ok()?;
    let provider_id = doc.get("model_provider")?.as_str()?.trim();
    doc.get("model_providers")?
        .as_table_like()?
        .get(provider_id)?
        .as_table_like()?
        .get("experimental_bearer_token")?
        .as_str()
        .map(ToString::to_string)
}

/// Writes the provider bearer, and `requires_openai_auth` only when the caller owns that field.
///
/// `None` leaves the authored value exactly as it is, present or absent. The startup credential
/// migration relocates a legacy key and is not the upgrade transform: deciding the official-auth
/// requirement there would change what the client sends, automatically and without an explicit
/// revisioned commit.
fn set_provider_config_bearer(
    config_contents: &str,
    api_key: &str,
    requires_openai_auth: Option<bool>,
) -> anyhow::Result<String> {
    let mut doc: toml_edit::DocumentMut = if config_contents.trim().is_empty() {
        toml_edit::DocumentMut::new()
    } else {
        config_contents.parse()?
    };
    let provider_id = doc
        .get("model_provider")
        .and_then(toml_edit::Item::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("codex-plus-relay")
        .to_string();
    doc["model_provider"] = toml_edit::value(provider_id.as_str());
    doc["model_providers"][provider_id.as_str()]["experimental_bearer_token"] =
        toml_edit::value(api_key.trim());
    if let Some(requires_openai_auth) = requires_openai_auth {
        doc["model_providers"][provider_id.as_str()]["requires_openai_auth"] =
            toml_edit::value(requires_openai_auth);
    }
    let mut result = doc.to_string();
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

fn normalize_provider_sync_provider_list(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            result.push(trimmed.to_string());
        }
    }
    result.sort();
    result
}

fn relay_combined_common_config(settings: &BackendSettings) -> String {
    relay_join_config_sections(&[
        &settings.relay_common_config_contents,
        &settings.relay_context_config_contents,
    ])
}

fn relay_join_config_sections(sections: &[&str]) -> String {
    let sections = sections
        .iter()
        .map(|section| section.trim())
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>();
    if sections.is_empty() {
        String::new()
    } else {
        codex_plus_core::relay_config::normalize_config_text(&format!(
            "{}\n",
            sections.join("\n\n")
        ))
    }
}

fn split_relay_context_config_sections(config: &str) -> (String, String) {
    let mut common = Vec::new();
    let mut context = Vec::new();
    let mut in_context_table = false;

    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_context_table = trimmed.starts_with("[mcp_servers.")
                || trimmed.starts_with("[skills.")
                || trimmed.starts_with("[plugins.");
        }
        if in_context_table {
            context.push(line);
        } else {
            common.push(line);
        }
    }

    (
        relay_join_config_sections(&[&common.join("\n")]),
        relay_join_config_sections(&[&context.join("\n")]),
    )
}

fn strip_common_config_text_fallback(config_contents: &str, common_config: &str) -> String {
    let common = common_config_anchors(common_config);
    if common.root_keys.is_empty() && common.table_headers.is_empty() {
        return ensure_text_newline(config_contents.trim_end());
    }

    let mut kept = Vec::new();
    let mut skipping_table = false;
    let mut in_root_section = true;
    let mut removed_root_keys = std::collections::HashSet::new();
    let source_root_keys = toml_root_keys_before_first_table(config_contents);

    for line in config_contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_root_section = false;
            let header = trimmed.to_string();
            skipping_table = common.table_headers.contains(&header);
            if skipping_table {
                continue;
            }
        }

        if skipping_table {
            continue;
        }

        if in_root_section && let Some(key) = toml_key_from_line(trimmed) {
            if common.root_keys.contains(key) {
                let is_duplicate_common_key = removed_root_keys.contains(key)
                    || source_root_keys.contains(key)
                    || common.table_headers.contains("[features]")
                    || common
                        .table_headers
                        .contains("[marketplaces.openai-bundled]")
                    || common
                        .table_headers
                        .contains("[plugins.\"superpowers@openai-curated\"]");
                if is_duplicate_common_key {
                    removed_root_keys.insert(key.to_string());
                    continue;
                }
            }
        }

        kept.push(line);
    }

    ensure_text_newline(kept.join("\n").trim_end())
}

fn toml_root_keys_before_first_table(config_contents: &str) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();
    for line in config_contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            break;
        }
        if let Some(key) = toml_key_from_line(trimmed) {
            keys.insert(key.to_string());
        }
    }
    keys
}

struct CommonConfigAnchors {
    root_keys: std::collections::HashSet<String>,
    table_headers: std::collections::HashSet<String>,
}

fn common_config_anchors(common_config: &str) -> CommonConfigAnchors {
    let mut root_keys = std::collections::HashSet::new();
    let mut table_headers = std::collections::HashSet::new();
    let mut in_table = false;

    for line in common_config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_table = true;
            table_headers.insert(trimmed.to_string());
            continue;
        }
        if !in_table {
            if let Some(key) = toml_key_from_line(trimmed) {
                root_keys.insert(key.to_string());
            }
        }
    }

    CommonConfigAnchors {
        root_keys,
        table_headers,
    }
}

fn toml_key_from_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (key, _) = trimmed.split_once('=')?;
    let key = key.trim();
    if key.is_empty() { None } else { Some(key) }
}

fn ensure_text_newline(value: &str) -> String {
    if value.trim().is_empty() {
        String::new()
    } else {
        format!("{}\n", value.trim_end())
    }
}

#[tauri::command]
pub fn open_external_url(url: String) -> CommandResult<Value> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return failed("只允许打开 http 或 https 链接。", json!({}));
    }
    match open_url(trimmed) {
        Ok(()) => ok("已在系统浏览器打开链接。", json!({ "url": trimmed })),
        Err(error) => failed(&format!("打开链接失败：{error}"), json!({ "url": trimmed })),
    }
}

/// Explicitly restart the target Codex/ChatGPT host so a changed contract or static
/// catalog takes effect. Quit is always graceful (AppleScript `quit`, no signal): a host
/// mid-write is never force-killed — if it does not exit in time the command reports
/// failure and the user decides. Nothing calls this automatically.
#[tauri::command]
pub async fn restart_codex_host() -> CommandResult<Value> {
    tauri::async_runtime::spawn_blocking(restart_codex_host_blocking)
        .await
        .unwrap_or_else(|_| failed("重启 Codex 中断；请手动确认 Codex 状态。", json!({})))
}

#[cfg(target_os = "macos")]
fn macos_codex_host_running(app_dir: &Path) -> bool {
    let needle = app_dir.join("Contents/MacOS");
    std::process::Command::new("pgrep")
        .args(["-f", &needle.to_string_lossy()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn restart_codex_host_blocking() -> CommandResult<Value> {
    #[cfg(target_os = "macos")]
    {
        let settings = SettingsStore::default().load().unwrap_or_default();
        let saved = settings.codex_app_path.trim().to_string();
        let Some(app_dir) = codex_plus_core::app_paths::resolve_codex_app_dir_with_saved(
            None,
            (!saved.is_empty()).then_some(saved.as_str()),
        ) else {
            return failed("未找到目标 Codex 或 ChatGPT 应用。", json!({}));
        };
        let app_name = app_dir
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("ChatGPT")
            .to_string();
        let was_running = macos_codex_host_running(&app_dir);
        if was_running {
            let quit = std::process::Command::new("osascript")
                .args(["-e", &format!("tell application \"{app_name}\" to quit")])
                .status();
            if !quit.map(|status| status.success()).unwrap_or(false) {
                return failed("请求退出 Codex 失败，请手动重启。", json!({}));
            }
            for _ in 0..30 {
                if !macos_codex_host_running(&app_dir) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            if macos_codex_host_running(&app_dir) {
                return failed("Codex 未在限时内退出；未强制结束，请手动重启。", json!({}));
            }
        }
        let launched = std::process::Command::new("open").arg(&app_dir).status();
        match launched {
            Ok(status) if status.success() => ok(
                if was_running {
                    "已重启 Codex。"
                } else {
                    "Codex 未在运行，已直接启动。"
                },
                json!({ "wasRunning": was_running }),
            ),
            _ => failed("启动 Codex 失败，请手动打开。", json!({})),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        failed("当前平台暂不支持一键重启 Codex，请手动重启。", json!({}))
    }
}

#[tauri::command]
pub async fn relay_status() -> CommandResult<RelayPayload> {
    tauri::async_runtime::spawn_blocking(|| relay_status_blocking())
        .await
        .expect("blocking command panicked")
}

fn relay_status_blocking() -> CommandResult<RelayPayload> {
    let status = codex_plus_core::relay_config::default_relay_status();
    let message = if status.authenticated {
        "已检测到 ChatGPT 登录状态。"
    } else {
        "未检测到 ChatGPT 登录状态，请先在 Codex/ChatGPT 中正常登录。"
    };
    ok(message, relay_payload(status, None))
}

#[tauri::command]
pub async fn read_relay_files() -> CommandResult<RelayFilesPayload> {
    tauri::async_runtime::spawn_blocking(|| read_relay_files_blocking())
        .await
        .expect("blocking command panicked")
}

fn read_relay_files_blocking() -> CommandResult<RelayFilesPayload> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    match relay_files_payload_from_home(&home) {
        Ok(payload) => ok("配置文件内容已读取。", payload),
        Err(_) => failed(
            "读取配置文件失败；未返回底层文件或解析错误。",
            RelayFilesPayload {
                config_path: home.join("config.toml").to_string_lossy().to_string(),
                auth_path: home.join("auth.json").to_string_lossy().to_string(),
                config_contents: String::new(),
                auth_status: live_auth_status_payload(&home),
            },
        ),
    }
}

#[tauri::command]
pub async fn check_env_conflicts() -> CommandResult<EnvConflictsPayload> {
    tauri::async_runtime::spawn_blocking(|| check_env_conflicts_blocking())
        .await
        .expect("blocking command panicked")
}

fn check_env_conflicts_blocking() -> CommandResult<EnvConflictsPayload> {
    let conflicts = codex_plus_core::env_conflicts::detect_env_conflicts();
    let message = if conflicts.is_empty() {
        "未检测到会覆盖 Codex 供应商配置的 OPENAI 环境变量。"
    } else {
        "检测到可能覆盖 Codex 供应商配置的 OPENAI 环境变量。"
    };
    ok(message, EnvConflictsPayload { conflicts })
}

#[tauri::command]
pub fn remove_env_conflicts(
    request: RemoveEnvConflictsRequest,
) -> CommandResult<RemoveEnvConflictsPayload> {
    let backup_dir = codex_plus_core::paths::default_app_state_dir().join("backups");
    match codex_plus_core::env_conflicts::remove_env_conflicts(&request.names, backup_dir) {
        Ok(result) => {
            let remaining = codex_plus_core::env_conflicts::detect_env_conflicts();
            ok(
                "环境变量已按确认项删除；重新启动 Codex 后生效。",
                RemoveEnvConflictsPayload {
                    removed: result.removed,
                    backup_path: result.backup_path,
                    remaining,
                },
            )
        }
        Err(error) => failed(
            &format!("删除环境变量失败：{error}"),
            RemoveEnvConflictsPayload {
                removed: Vec::new(),
                backup_path: None,
                remaining: codex_plus_core::env_conflicts::detect_env_conflicts(),
            },
        ),
    }
}

#[tauri::command]
pub async fn save_relay_file(request: SaveRelayFileRequest) -> CommandResult<RelayFilesPayload> {
    tauri::async_runtime::spawn_blocking(move || save_relay_file_blocking(request))
        .await
        .expect("blocking command panicked")
}

fn save_relay_file_blocking(request: SaveRelayFileRequest) -> CommandResult<RelayFilesPayload> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let result = (|| -> anyhow::Result<RelayFilesPayload> {
        validate_relay_file_save_kind(&request.kind)?;
        crate::model_catalog::ensure_active_config_context_compatible(&request.contents)?;
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        let (protected, context_snapshot) = context_protected_config(&home, &request.contents)?;
        let config_path = home.join("config.toml");
        live_state::commit_locked_verified(&[FileMutation::text(config_path, protected)], || {
            verify_context_tables(&home, &context_snapshot)
        })?;
        relay_files_payload_from_home(&home)
    })();
    match result {
        Ok(payload) => ok("配置文件已保存。", payload),
        Err(_) => failed(
            "保存配置文件失败；配置无效或事务未完成。",
            relay_files_payload_from_home(&home).unwrap_or_else(|_| RelayFilesPayload {
                config_path: home.join("config.toml").to_string_lossy().to_string(),
                auth_path: home.join("auth.json").to_string_lossy().to_string(),
                config_contents: String::new(),
                auth_status: live_auth_status_payload(&home),
            }),
        ),
    }
}

fn validate_relay_file_save_kind(kind: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        kind == "config",
        "auth.json 由官方客户端管理，Codex Minus 不接受认证文件写入"
    );
    Ok(())
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayProfileSwitchRequest {
    pub settings: BackendSettings,
    #[serde(default)]
    pub previous_active_relay_id: String,
    #[serde(default)]
    pub confirm_context_cleanup: bool,
}

#[derive(Debug, Clone)]
pub struct ProviderCommitPaths {
    pub app_state: PathBuf,
    pub codex_home: PathBuf,
    pub settings_path: PathBuf,
    pub catalog_state_path: PathBuf,
    #[cfg(test)]
    pub(crate) current_target: Option<crate::model_catalog::VerifiedTargetIdentity>,
}

impl ProviderCommitPaths {
    fn defaults() -> Self {
        Self {
            app_state: codex_plus_core::paths::default_app_state_dir(),
            codex_home: codex_plus_core::relay_config::default_codex_home_dir(),
            settings_path: codex_plus_core::paths::default_settings_path(),
            catalog_state_path: crate::model_catalog::catalog_state_path(),
            #[cfg(test)]
            current_target: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCommitPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<BackendSettings>,
    pub draft_revision: u64,
    pub provider_fingerprint: String,
    pub restart_required: bool,
    pub error_code: Option<ProviderCommitErrorCode>,
    /// Static rejecting rule. Every value is a literal chosen at the failing call site, so the
    /// discriminator can reach the user without a redaction pass over dynamic content.
    pub reason: Option<&'static str>,
}

impl ProviderCommitPayload {
    pub fn failure(
        draft_revision: u64,
        error_code: ProviderCommitErrorCode,
        reason: &'static str,
    ) -> Self {
        Self {
            settings: None,
            draft_revision,
            provider_fingerprint: String::new(),
            restart_required: false,
            error_code: Some(error_code),
            reason: Some(reason),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderCommitErrorCode {
    InputUnavailable,
    OfficialAuthRequired,
    CatalogScopeStale,
    StaleState,
    InvalidDraft,
    CatalogUnavailable,
    StagingRejected,
    TransactionFailed,
}

#[derive(Debug)]
pub struct ProviderCommitFailure {
    code: ProviderCommitErrorCode,
    message: &'static str,
}

/// Transaction checkpoints exposed only so isolated regression fixtures can inject failures
/// without replacing the production journal or storage implementation.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderCommitCheckpoint {
    Normalization,
    ActivationScopeVerification,
    CatalogMaterialization,
    /// Immediately before the active profile is staged into a private home.
    ///
    /// Staging failure is the only source of `StagingRejected`, and no input reachable through
    /// the public commit API produces one: common configuration is sanitized before it gets here.
    /// The checkpoint exists so that code can be exercised the same way catalog materialization
    /// already is, rather than being asserted only in theory.
    Staging,
    SettingsPersistence,
    LiveConfigCommit,
    ContextVerification,
    AuthGenerationVerification,
    PostCommitVerification,
}

impl ProviderCommitFailure {
    fn new(code: ProviderCommitErrorCode, message: &'static str) -> Self {
        Self { code, message }
    }

    pub fn code(&self) -> ProviderCommitErrorCode {
        self.code
    }

    pub fn reason(&self) -> &'static str {
        self.message
    }
}

impl std::fmt::Display for ProviderCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for ProviderCommitFailure {}

fn provider_commit_failure(
    code: ProviderCommitErrorCode,
    message: &'static str,
) -> ProviderCommitFailure {
    ProviderCommitFailure::new(code, message)
}

fn provider_commit_failure_for_legacy_auth_migration(
    error: LegacyProfileAuthMigrationError,
) -> ProviderCommitFailure {
    // Keep the loader's input taxonomy intact; only profile-level reconciliation is an auth
    // migration failure. Security and atomic-write work belongs to the transaction category.
    match error {
        LegacyProfileAuthMigrationError::SettingsUnreadable(_) => provider_commit_failure(
            ProviderCommitErrorCode::InputUnavailable,
            "provider settings file is unreadable",
        ),
        LegacyProfileAuthMigrationError::SettingsInvalidJson(_) => provider_commit_failure(
            ProviderCommitErrorCode::InputUnavailable,
            "provider settings are invalid JSON",
        ),
        LegacyProfileAuthMigrationError::ProfileReconciliation(_) => provider_commit_failure(
            ProviderCommitErrorCode::InputUnavailable,
            "a saved provider profile failed auth migration",
        ),
        LegacyProfileAuthMigrationError::SecureStorage(_) => provider_commit_failure(
            ProviderCommitErrorCode::TransactionFailed,
            "provider transaction failed",
        ),
    }
}

/// Awaits a blocking command, reporting a panic instead of dropping the reply.
///
/// Re-panicking inside a Tauri command drops its IPC responder without answering, so the caller's
/// promise never settles and the editor keeps a pending state with nothing to show.
pub(crate) async fn settle_blocking<T, F>(
    task: tauri::async_runtime::JoinHandle<CommandResult<T>>,
    message: &str,
    on_panic: F,
) -> CommandResult<T>
where
    T: Serialize,
    F: FnOnce() -> T,
{
    match task.await {
        Ok(result) => result,
        Err(_) => failed(message, on_panic()),
    }
}

#[tauri::command]
pub async fn commit_provider_detail(
    request: crate::provider_commit::ProviderCommitRequest,
) -> CommandResult<ProviderCommitPayload> {
    let draft_revision = request.draft_revision;
    let task =
        tauri::async_runtime::spawn_blocking(move || {
            match commit_provider_detail_from_paths(&ProviderCommitPaths::defaults(), request) {
                Ok(payload) => ok("供应商与模型目录已作为同一代提交。", payload),
                Err(error) => failed(
                    "供应商提交失败；已保留原有设置与 live 配置。",
                    ProviderCommitPayload::failure(draft_revision, error.code(), error.reason()),
                ),
            }
        });
    settle_blocking(
        task,
        "供应商提交中断；已保留原有设置与 live 配置。",
        move || {
            ProviderCommitPayload::failure(
                draft_revision,
                ProviderCommitErrorCode::TransactionFailed,
                "提交过程意外中断。",
            )
        },
    )
    .await
}

pub fn commit_provider_detail_from_paths(
    paths: &ProviderCommitPaths,
    request: crate::provider_commit::ProviderCommitRequest,
) -> Result<ProviderCommitPayload, ProviderCommitFailure> {
    commit_provider_detail_from_paths_observed(paths, request, |_| Ok(()))
}

#[doc(hidden)]
pub fn commit_provider_detail_from_paths_observed(
    paths: &ProviderCommitPaths,
    request: crate::provider_commit::ProviderCommitRequest,
    mut observe: impl FnMut(ProviderCommitCheckpoint) -> anyhow::Result<()>,
) -> Result<ProviderCommitPayload, ProviderCommitFailure> {
    use crate::model_catalog::CatalogMode;
    use crate::provider_commit::{
        CatalogReadinessInput, ProviderOwnedTopologyDraft,
        plan_provider_detail_commit_with_readiness, plan_provider_topology_commit_with_readiness,
        validate_provider_topology_request,
    };

    let transaction_failure = |_| {
        provider_commit_failure(
            ProviderCommitErrorCode::TransactionFailed,
            "provider transaction failed",
        )
    };
    let _guard = live_state::lock().map_err(transaction_failure)?;
    live_state::prepare_secret_paths_at(&paths.app_state, &paths.settings_path, &paths.codex_home)
        .map_err(transaction_failure)?;
    live_state::recover_locked_at(&paths.app_state).map_err(transaction_failure)?;
    // Recovery artifacts snapshot every transaction target before applying it. Scrub legacy
    // profile auth through the startup's owner-only atomic no-backup path before this commit can
    // prepare a settings prior stage, so copied OAuth can never enter recovery material.
    validate_persisted_responses_only_settings_at(&paths.settings_path).map_err(|error| {
        let reason = if error
            .downcast_ref::<crate::provider_commit::ResponsesOnlyProviderError>()
            .is_some()
        {
            "provider settings contain an unsupported provider topology"
        } else {
            "provider settings are invalid JSON"
        };
        provider_commit_failure(ProviderCommitErrorCode::InputUnavailable, reason)
    })?;
    migrate_legacy_profile_auth_locked_at(&paths.settings_path)
        .map_err(provider_commit_failure_for_legacy_auth_migration)?;

    let (persisted_settings_bytes, persisted_settings) =
        load_provider_commit_settings(&paths.settings_path).map_err(|reason| {
            provider_commit_failure(ProviderCommitErrorCode::InputUnavailable, reason)
        })?;
    let persisted_state = crate::model_catalog::load_and_migrate_state_from_path(
        &persisted_settings,
        &paths.codex_home,
        &paths.catalog_state_path,
    )
    .map_err(|_| {
        provider_commit_failure(
            ProviderCommitErrorCode::CatalogUnavailable,
            "provider catalog state is unavailable",
        )
    })?;

    // Validate the unmodified request first so compare-and-swap, structural catalog rules,
    // and authContents ownership are decided before normalization can change evidence.
    let persisted_as_shown = serde_json::from_slice(&persisted_settings_bytes)
        .map(sanitize_settings_for_output)
        .map_err(|_| {
            provider_commit_failure(
                ProviderCommitErrorCode::InputUnavailable,
                "provider settings are unavailable",
            )
        })?;
    let mut request = request;
    // Compare-and-swap is decided here and only here. Restating the accepted baseline keeps the
    // later validators comparing against one agreed generation instead of re-deciding staleness
    // against a form the editor was never shown.
    request.expected_provider_fingerprint =
        validate_provider_commit_cas(&persisted_settings, &persisted_as_shown, &request)?;
    validate_provider_commit_catalog_structure(&request)?;
    let focused_id = request.focused_profile_id.clone();
    if focused_id.is_some() {
        crate::provider_commit::validate_provider_detail_request(
            &persisted_settings,
            &persisted_state,
            &request,
        )
        .map_err(|error| {
            provider_commit_failure(
                ProviderCommitErrorCode::InvalidDraft,
                sanitized_provider_validation_error(&error, "provider draft validation failed"),
            )
        })?;
    } else {
        validate_provider_topology_mutation_scope(&persisted_settings, &persisted_state, &request)?;
        validate_provider_topology_request(&persisted_settings, &persisted_state, &request)
            .map_err(|error| {
                provider_commit_failure(
                    ProviderCommitErrorCode::InvalidDraft,
                    sanitized_provider_validation_error(
                        &error,
                        "provider topology validation failed",
                    ),
                )
            })?;
    }
    let focused_mode = focused_id
        .as_deref()
        .and_then(|focused_id| {
            request
                .catalog_drafts
                .iter()
                .find(|draft| draft.profile_id == focused_id)
                .map(|draft| draft.mode)
        })
        .unwrap_or(CatalogMode::NativeOfficial);
    if let Some(focused_id) = focused_id.as_deref()
        && crate::model_catalog::managed_mode(focused_mode)
    {
        let focused = request
            .topology
            .relay_profiles
            .iter()
            .find(|profile| profile.id == focused_id)
            .ok_or_else(|| {
                provider_commit_failure(
                    ProviderCommitErrorCode::InvalidDraft,
                    "focused provider profile is missing",
                )
            })?;
        let saved_conflict = !focused.context_window.trim().is_empty()
            || !focused.auto_compact_limit.trim().is_empty()
            || !crate::model_catalog::global_context_conflicts(&focused.config_contents).is_empty();
        let live_conflict = if request.topology.active_relay_id == focused_id {
            let live = read_optional_bytes(&paths.codex_home.join("config.toml"))
                .map_err(transaction_failure)?
                .map(|bytes| String::from_utf8(bytes).map_err(|_| ()))
                .transpose()
                .map_err(|_| {
                    provider_commit_failure(
                        ProviderCommitErrorCode::InvalidDraft,
                        "live provider config is invalid",
                    )
                })?
                .unwrap_or_default();
            !crate::model_catalog::global_context_conflicts(&live).is_empty()
        } else {
            false
        };
        if (saved_conflict || live_conflict) && !request.confirm_context_cleanup {
            return Err(provider_commit_failure(
                ProviderCommitErrorCode::InvalidDraft,
                "managed catalog context cleanup confirmation is required",
            ));
        }
    }
    observe(ProviderCommitCheckpoint::Normalization).map_err(|_| {
        provider_commit_failure(
            ProviderCommitErrorCode::InvalidDraft,
            "provider draft normalization failed",
        )
    })?;
    let normalized_settings = if let Some(focused_id) = focused_id.as_deref() {
        normalize_provider_detail_settings_fallible(
            request.topology.apply_to(&persisted_settings),
            &persisted_as_shown,
            focused_id,
            focused_mode,
            request.confirm_context_cleanup,
        )
    } else {
        normalize_provider_topology_settings_fallible(
            request.topology.apply_to(&persisted_settings),
            &request,
            &persisted_state,
        )
    }
    .map_err(|error| {
        provider_commit_failure(
            ProviderCommitErrorCode::InvalidDraft,
            sanitized_provider_normalization_error(&error),
        )
    })?;

    let auth_path = paths.codex_home.join("auth.json");
    let auth_before = read_optional_bytes(&auth_path).map_err(transaction_failure)?;
    let catalog_scope_current = match focused_id.as_deref() {
        Some(focused_id) => managed_provider_catalog_scope_current(
            paths,
            &persisted_state,
            &normalized_settings,
            focused_id,
            focused_mode,
        )?,
        None => None,
    };
    let activation_scope_verified = focused_id.as_deref().is_some_and(|focused_id| {
        normalized_settings.active_relay_id == focused_id && catalog_scope_current == Some(true)
    });
    if activation_scope_verified {
        observe(ProviderCommitCheckpoint::ActivationScopeVerification)
            .map_err(transaction_failure)?;
    }

    // Re-plan from the normalized projection. The CAS still binds to persisted state; only
    // provider-owned normalized fields are allowed to differ from the submitted draft.
    let mut normalized_request = request.clone();
    normalized_request.topology = ProviderOwnedTopologyDraft::from_settings(&normalized_settings);
    let mut catalog_readiness = CatalogReadinessInput::default();
    if let (Some(focused_id), Some(scope_current)) = (focused_id.as_deref(), catalog_scope_current)
    {
        catalog_readiness
            .scope_current_by_profile
            .insert(focused_id.to_string(), scope_current);
    }
    if focused_id.is_none() {
        let managed_profile_ids = normalized_request
            .catalog_drafts
            .iter()
            .filter(|draft| draft.mode == CatalogMode::OfficialPlusCustom)
            .map(|draft| draft.profile_id.clone())
            .collect::<Vec<_>>();
        if !managed_profile_ids.is_empty() {
            let scope_current = current_official_catalog_scope(paths, &persisted_state, false)?;
            for profile_id in managed_profile_ids {
                catalog_readiness
                    .scope_current_by_profile
                    .insert(profile_id, scope_current);
            }
        }
    }
    let mut plan = if focused_id.is_some() {
        crate::provider_commit::validate_provider_detail_request(
            &persisted_settings,
            &persisted_state,
            &normalized_request,
        )
        .map_err(|error| {
            provider_commit_failure(
                ProviderCommitErrorCode::InvalidDraft,
                sanitized_provider_validation_error(&error, "normalized provider draft is invalid"),
            )
        })?;
        plan_provider_detail_commit_with_readiness(
            &persisted_settings,
            &persisted_state,
            &normalized_request,
            &catalog_readiness,
        )
        .map_err(|error| {
            provider_commit_failure(
                ProviderCommitErrorCode::CatalogUnavailable,
                sanitized_provider_validation_error(
                    &error,
                    "normalized provider catalog planning failed",
                ),
            )
        })?
    } else {
        plan_provider_topology_commit_with_readiness(
            &persisted_settings,
            &persisted_state,
            &normalized_request,
            &catalog_readiness,
        )
        .map_err(|error| {
            provider_commit_failure(
                ProviderCommitErrorCode::CatalogUnavailable,
                sanitized_provider_validation_error(
                    &error,
                    "normalized provider topology planning failed",
                ),
            )
        })?
    };

    // The master switch governs live writes, not whether a draft can be saved: the page states
    // that a disabled switch still saves configuration and simply does not write Codex's live
    // file, so a disabled switch commits as an inactive draft rather than refusing the save.
    let active_commit = plan.settings.relay_profiles_enabled
        && focused_id
            .as_deref()
            .is_some_and(|focused_id| plan.settings.active_relay_id == focused_id);
    let mut mutations = materialize_provider_commit_catalogs(paths, &normalized_request, &plan)
        .map_err(|_| {
            provider_commit_failure(
                ProviderCommitErrorCode::CatalogUnavailable,
                "provider catalog materialization failed",
            )
        })?;
    observe(ProviderCommitCheckpoint::CatalogMaterialization).map_err(|_| {
        provider_commit_failure(
            ProviderCommitErrorCode::CatalogUnavailable,
            "provider catalog materialization failed",
        )
    })?;
    let mut context_snapshot = None;

    if active_commit {
        let staged = observe(ProviderCommitCheckpoint::Staging)
            .and_then(|()| {
                stage_active_relay_config_at(&paths.codex_home, &paths.app_state, &plan.settings)
            })
            .map_err(|_| {
                provider_commit_failure(
                    ProviderCommitErrorCode::StagingRejected,
                    "provider staging failed",
                )
            })?;
        let active_profile = plan.settings.active_relay_profile();
        let active_state = plan
            .catalog_state
            .profiles
            .get(&active_profile.id)
            .cloned()
            .unwrap_or_default();
        let inspection =
            crate::provider_native_capability::inspect_profile(&active_profile, active_state.mode);
        if inspection.state
            == crate::provider_native_capability::NativeCapabilityState::NativePriority
        {
            assert_staged_native_provider_contract(&active_profile, &staged, active_state.mode)
                .map_err(|_| {
                    provider_commit_failure(
                        ProviderCommitErrorCode::StagingRejected,
                        "staged provider contract was rejected",
                    )
                })?;
        }

        let staged = match active_state.mode {
            CatalogMode::NativeOfficial => {
                crate::model_catalog::set_root_catalog_pointer(&staged, None).map_err(|_| {
                    provider_commit_failure(
                        ProviderCommitErrorCode::CatalogUnavailable,
                        "provider catalog pointer is invalid",
                    )
                })?
            }
            CatalogMode::External => crate::model_catalog::set_root_catalog_pointer(
                &staged,
                active_state.external_pointer.as_deref(),
            )
            .map_err(|_| {
                provider_commit_failure(
                    ProviderCommitErrorCode::CatalogUnavailable,
                    "external catalog pointer is invalid",
                )
            })?,
            CatalogMode::OfficialPlusCustom | CatalogMode::CustomOnly => {
                if active_state.action_required.is_some() || plan.active_catalog.is_none() {
                    return Err(provider_commit_failure(
                        ProviderCommitErrorCode::CatalogUnavailable,
                        "active provider catalog is not ready",
                    ));
                }
                let pointer = active_state.generated_path.as_deref().ok_or_else(|| {
                    provider_commit_failure(
                        ProviderCommitErrorCode::CatalogUnavailable,
                        "active provider catalog pointer is missing",
                    )
                })?;
                crate::model_catalog::set_root_catalog_pointer(&staged, Some(pointer)).map_err(
                    |_| {
                        provider_commit_failure(
                            ProviderCommitErrorCode::CatalogUnavailable,
                            "active provider catalog pointer is invalid",
                        )
                    },
                )?
            }
        };
        let (protected, snapshot) =
            context_protected_config(&paths.codex_home, &staged).map_err(transaction_failure)?;
        let active_state = plan
            .catalog_state
            .profiles
            .entry(active_profile.id.clone())
            .or_default();
        // The runtime fingerprint is computed after catalog planning so it carries the final
        // catalog artifact identity, and it never reads the catalog generation counter that its
        // own update would otherwise perturb. Only a changed fingerprint marks a restart, so two
        // identical consecutive active saves stay idempotent.
        let runtime_fingerprint =
            crate::model_catalog::applied_runtime_fingerprint(&active_profile, active_state)
                .map_err(|_| {
                    provider_commit_failure(
                        ProviderCommitErrorCode::StagingRejected,
                        "provider runtime identity is incomplete",
                    )
                })?;
        if active_state.applied_runtime_fingerprint.as_deref() != Some(runtime_fingerprint.as_str())
        {
            active_state.applied_runtime_fingerprint = Some(runtime_fingerprint);
            active_state.restart_required = true;
        }
        context_snapshot = Some(snapshot);
        mutations.push(FileMutation::text(
            paths.codex_home.join("config.toml"),
            protected,
        ));
    }

    mutations.push(FileMutation::bytes(
        paths.settings_path.clone(),
        serialize_settings_without_profile_auth(&plan.settings).map_err(transaction_failure)?,
    ));
    mutations.push(
        crate::model_catalog::state_mutation_at(&plan.catalog_state, &paths.catalog_state_path)
            .map_err(transaction_failure)?,
    );

    let settings_generation_matches = read_optional_bytes(&paths.settings_path)
        .map_err(transaction_failure)?
        .is_some_and(|current| current == persisted_settings_bytes);
    if !settings_generation_matches {
        return Err(provider_commit_failure(
            ProviderCommitErrorCode::StaleState,
            "provider settings changed during commit; reload or merge before saving",
        ));
    }

    let live_config_path = paths.codex_home.join("config.toml");
    let observe = std::cell::RefCell::new(&mut observe);
    live_state::commit_locked_verified_at_observed(
        &paths.app_state,
        &mutations,
        |path| {
            if path == paths.settings_path {
                observe.borrow_mut()(ProviderCommitCheckpoint::SettingsPersistence)?;
            } else if path == live_config_path {
                observe.borrow_mut()(ProviderCommitCheckpoint::LiveConfigCommit)?;
            }
            Ok(())
        },
        || {
            if let Some(snapshot) = context_snapshot.as_ref() {
                verify_context_tables(&paths.codex_home, snapshot)?;
                observe.borrow_mut()(ProviderCommitCheckpoint::ContextVerification)?;
            }
            observe.borrow_mut()(ProviderCommitCheckpoint::AuthGenerationVerification)?;
            anyhow::ensure!(
                read_optional_bytes(&auth_path)? == auth_before,
                "live auth changed concurrently"
            );
            Ok(())
        },
        || {
            observe.borrow_mut()(ProviderCommitCheckpoint::PostCommitVerification)?;
            Ok(())
        },
    )
    .map_err(transaction_failure)?;

    let restart_required = focused_id.as_deref().is_some_and(|focused_id| {
        plan.catalog_state
            .profiles
            .get(focused_id)
            .is_some_and(|state| state.restart_required)
    });
    let provider_fingerprint = crate::provider_commit::provider_owned_fingerprint(
        &ProviderOwnedTopologyDraft::from_settings(&plan.settings),
    )
    .map_err(|_| {
        provider_commit_failure(
            ProviderCommitErrorCode::InvalidDraft,
            "provider fingerprint generation failed",
        )
    })?;
    Ok(ProviderCommitPayload {
        settings: Some(sanitize_settings_for_output(plan.settings)),
        draft_revision: plan.draft_revision,
        provider_fingerprint,
        restart_required,
        error_code: None,
        reason: None,
    })
}

fn managed_provider_catalog_scope_current(
    paths: &ProviderCommitPaths,
    persisted_state: &crate::model_catalog::CatalogState,
    normalized_settings: &BackendSettings,
    focused_id: &str,
    focused_mode: crate::model_catalog::CatalogMode,
) -> Result<Option<bool>, ProviderCommitFailure> {
    let active = normalized_settings.active_relay_id == focused_id;
    if focused_mode != crate::model_catalog::CatalogMode::OfficialPlusCustom {
        return Ok(None);
    }

    current_official_catalog_scope(paths, persisted_state, active).map(Some)
}

fn current_official_catalog_scope(
    paths: &ProviderCommitPaths,
    persisted_state: &crate::model_catalog::CatalogState,
    active: bool,
) -> Result<bool, ProviderCommitFailure> {
    // The baseline ships with the application, so it is neither account-scoped nor tied to an
    // installed CLI. What remains is the requirement the mixed contract actually has: the official
    // client must be signed in, because inference still travels under that session.
    let auth_path = paths.codex_home.join("auth.json");
    match crate::model_catalog::current_activation_scope_hash_at(persisted_state, &auth_path) {
        Ok(_) => Ok(true),
        Err(_) if active => Err(provider_commit_failure(
            ProviderCommitErrorCode::OfficialAuthRequired,
            "official ChatGPT authentication is required",
        )),
        Err(_) => Ok(false),
    }
}

/// Reasons are static so the UI can only ever surface whitelisted text, and distinct so the
/// user can tell a missing file from corrupt JSON from one profile failing migration — the
/// previous single "provider settings are unavailable" hid which of the three it was.
fn load_provider_commit_settings(path: &Path) -> Result<(Vec<u8>, BackendSettings), &'static str> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        // A machine that has never saved settings has no file yet. That is not broken
        // input: bootstrap the defaults on disk so the very first provider save can
        // proceed instead of refusing with "provider settings are unavailable".
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let defaults = serde_json::to_vec(&BackendSettings::default())
                .map_err(|_| "provider settings bootstrap failed")?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|_| "provider settings bootstrap failed")?;
            }
            std::fs::write(path, &defaults).map_err(|_| "provider settings bootstrap failed")?;
            defaults
        }
        Err(_) => return Err("provider settings file is unreadable"),
    };
    let mut settings: BackendSettings =
        serde_json::from_slice(&bytes).map_err(|_| "provider settings are invalid JSON")?;
    crate::provider_commit::validate_responses_only_settings(&settings)
        .map_err(|_| "provider settings contain an unsupported provider topology")?;
    for profile in &mut settings.relay_profiles {
        migrate_persisted_legacy_api_key_auth(profile)
            .map_err(|_| "a saved provider profile failed auth migration")?;
        codex_plus_core::relay_config::normalize_relay_profile_for_storage(profile)
            .map_err(|_| "a saved provider profile failed normalization")?;
        sanitize_profile_after_core_normalize_fallible(profile)
            .map_err(|_| "a saved provider profile failed sanitization")?;
    }
    Ok((bytes, settings))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PersistedProviderConfigError {
    Invalid,
}

impl std::fmt::Display for PersistedProviderConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => formatter.write_str("persisted provider config is invalid"),
        }
    }
}

impl std::error::Error for PersistedProviderConfigError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PersistedProfileAuthMigrationError {
    AuthCopyInvalid,
    ProviderApiKeyMissing,
    ProviderKeyConflict,
    ProviderConfig(PersistedProviderConfigError),
}

impl std::fmt::Display for PersistedProfileAuthMigrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthCopyInvalid => formatter.write_str("persisted provider auth copy is invalid"),
            Self::ProviderApiKeyMissing => {
                formatter.write_str("persisted provider API key is missing")
            }
            Self::ProviderKeyConflict => formatter.write_str("persisted provider key conflict"),
            Self::ProviderConfig(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PersistedProfileAuthMigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ProviderConfig(error) => Some(error),
            Self::AuthCopyInvalid | Self::ProviderApiKeyMissing | Self::ProviderKeyConflict => None,
        }
    }
}

impl From<PersistedProviderConfigError> for PersistedProfileAuthMigrationError {
    fn from(error: PersistedProviderConfigError) -> Self {
        Self::ProviderConfig(error)
    }
}

fn migrate_persisted_legacy_api_key_auth(
    profile: &mut RelayProfile,
) -> Result<(), PersistedProfileAuthMigrationError> {
    if profile.auth_contents.is_empty() {
        return Ok(());
    }
    let value: serde_json::Value = serde_json::from_str(&profile.auth_contents)
        .map_err(|_| PersistedProfileAuthMigrationError::AuthCopyInvalid)?;
    let object = value
        .as_object()
        .ok_or(PersistedProfileAuthMigrationError::AuthCopyInvalid)?;
    // Parsing is intentionally complete before deciding whether this profile owns a provider
    // key: OAuth-only profiles must reject malformed/non-object copies, then discard valid
    // residue without adopting a legacy API key.
    if !profile_owns_provider_key(profile) {
        profile.auth_contents.clear();
        return Ok(());
    }

    let mut candidates = Vec::new();
    if let Some(legacy_key) = non_empty_provider_key(
        object
            .get("OPENAI_API_KEY")
            .and_then(serde_json::Value::as_str),
    ) {
        candidates.push(legacy_key);
    }
    if let Some(structured_key) = non_empty_provider_key(Some(&profile.api_key)) {
        candidates.push(structured_key);
    }
    if let Some(bearer_key) = provider_bearer_token_from_config_exact(&profile.config_contents)
        .and_then(|value| non_empty_provider_key(Some(&value)))
    {
        candidates.push(bearer_key);
    }
    let mut candidates = candidates.into_iter();
    let agreed_key = candidates
        .next()
        .ok_or(PersistedProfileAuthMigrationError::ProviderApiKeyMissing)?;
    if !candidates.all(|candidate| candidate.as_bytes() == agreed_key.as_bytes()) {
        return Err(PersistedProfileAuthMigrationError::ProviderKeyConflict);
    }
    profile.api_key = agreed_key.clone();
    profile.config_contents =
        set_provider_config_bearer(&profile.config_contents, &agreed_key, None)
            .map_err(|_| PersistedProviderConfigError::Invalid)?;
    profile.auth_contents.clear();
    Ok(())
}

fn profile_owns_provider_key(profile: &RelayProfile) -> bool {
    profile.relay_mode == codex_plus_core::settings::RelayMode::PureApi
        || (profile.relay_mode == codex_plus_core::settings::RelayMode::Official
            && profile.official_mix_api_key)
}

fn non_empty_provider_key(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn sanitized_provider_normalization_error(error: &anyhow::Error) -> &'static str {
    let message = error.to_string();
    for (needle, safe) in [
        // The bail carries the contract reason variant name, so the specific gap is matched
        // before the family fallback and the user learns which field to fix.
        ("MissingModel", "provider model is required"),
        ("MalformedModel", "provider model is malformed"),
        ("MissingBaseUrl", "provider base URL is required"),
        ("MalformedBaseUrl", "provider base URL is malformed"),
        ("MissingProviderName", "provider name is required"),
        ("ProviderNameMismatch", "provider name mismatch"),
        ("MalformedProviderName", "provider name is malformed"),
        ("MissingProviderBearer", "provider key is required"),
        ("MalformedProviderBearer", "provider key is malformed"),
        ("StructuredKeyBearerConflict", "provider key conflict"),
        ("MissingWireApi", "provider wire API is required"),
        ("WireApiMismatch", "provider wire API mismatch"),
        (
            "OpenAiAuthRequired",
            "provider must not require official auth",
        ),
        ("MissingActorHeader", "provider actor header is required"),
        (
            "ActorHeaderNameMismatch",
            "provider actor header name mismatch",
        ),
        (
            "ActorHeaderValueConflict",
            "provider actor header value conflict",
        ),
        (
            "DuplicateActorHeader",
            "provider actor header is duplicated",
        ),
        ("CatalogModeMismatch", "provider catalog mode mismatch"),
        ("ReservedProviderId", "provider id is reserved"),
        (
            "LegacyProviderIdRequiresRename",
            "legacy provider id requires rename",
        ),
        (
            "SelectedProviderTableMissing",
            "selected provider table is missing",
        ),
        (
            "MalformedProviderTable",
            "selected provider table is malformed",
        ),
        ("MissingProviderSelection", "provider selection is required"),
        (
            "native provider contract is invalid",
            "provider native contract is incomplete",
        ),
        ("base URL conflict", "provider base URL conflict"),
        ("model conflict", "provider model conflict"),
        ("key conflict", "provider key conflict"),
        ("TOML", "provider config TOML is invalid"),
        ("authContents", "incoming authContents is prohibited"),
    ] {
        if message.contains(needle) {
            return safe;
        }
    }
    "provider draft normalization failed"
}

/// Interns a request-validation error back to the static rule that rejected it.
///
/// `validate_provider_detail_request` and its helpers reject with literal rule messages, but the
/// payload's `reason` must stay `&'static str` so no dynamic content reaches the user without a
/// redaction pass. Matching the message against the known rule set keeps that property while
/// naming the actual gate instead of the family fallback — the fallback previously blamed "draft
/// validation" for rules like external-pointer preservation that have nothing to do with the form.
/// A message outside the set (a dynamic TOML parse error bubbled through a pointer read) stays
/// behind the fallback.
fn sanitized_provider_validation_error(
    error: &anyhow::Error,
    fallback: &'static str,
) -> &'static str {
    const KNOWN_RULES: &[&str] = &[
        "focused provider profile is required",
        "focused provider profile is missing from the topology draft",
        "setCurrent must select the focused provider profile",
        "save cannot change the active provider profile",
        "focused provider profile must carry exactly the catalog drafts its capability supports",
        "topology request cannot select a focused provider profile",
        "topology request supports save only",
        "draft revision must be a positive correlation value",
        "provider state changed; reload or merge before saving",
        "previous active provider does not match the compare-and-swap snapshot",
        "provider profile id is empty",
        "duplicate provider profile id",
        "incoming authContents is prohibited",
        "only Responses API provider profiles are supported",
        "local aggregate provider profiles are unsupported",
        "the removed local proxy base URL is unsupported",
        "aggregate profile id is empty",
        "duplicate aggregate profile id",
        "aggregate profile metadata has no matching relay profile",
        "aggregate profile members are empty",
        "aggregate member weight must be positive",
        "duplicate aggregate member",
        "aggregate member references a missing provider profile",
        "aggregate member must reference an ordinary provider profile",
        "aggregate relay profiles and aggregate metadata must be one-to-one",
        "active aggregate profile is missing from the topology draft",
        "active provider profile is missing from the topology draft",
        "active provider and active aggregate ids are inconsistent",
        "duplicate catalog draft for provider profile",
        "catalog draft references a missing provider profile",
        "catalog-incapable provider profiles cannot carry catalog drafts",
        "external catalog draft requires a pointer",
        "external catalog pointer must exactly match profile configContents",
        "managed or native catalog draft cannot carry an external pointer",
        "external catalog ownership requires the reviewed adoption command",
        "ordinary save must preserve external catalog ownership identity",
        "persisted external catalog ownership is missing its pointer",
        "ordinary save must preserve every external catalog pointer",
        "new provider profile requires one complete catalog draft",
        "active provider catalog is not ready",
        "active provider default model is absent from the bundled baseline",
    ];
    let message = error.to_string();
    KNOWN_RULES
        .iter()
        .copied()
        .find(|rule| *rule == message)
        .unwrap_or(fallback)
}

/// Decides compare-and-swap once, at the boundary, and returns the canonical baseline fingerprint.
///
/// Two persisted forms are equally legitimate evidence of "nothing changed underneath": the
/// normalized baseline this transaction plans against, and the form the settings payload actually
/// handed the editor, which is read without core storage normalization. They differ whenever a
/// persisted profile is not already in core-canonical form — a legacy provider-ID alias, a table
/// missing a default the normalizer supplies, a hand-edited file. Accepting only the normalized
/// form strands such a profile: every save reports stale state, and reloading reads the same file
/// and reports it again.
fn validate_provider_commit_cas(
    persisted_settings: &BackendSettings,
    persisted_as_shown: &BackendSettings,
    request: &crate::provider_commit::ProviderCommitRequest,
) -> Result<String, ProviderCommitFailure> {
    let fingerprint = |settings: &BackendSettings| {
        crate::provider_commit::provider_owned_fingerprint(
            &crate::provider_commit::ProviderOwnedTopologyDraft::from_settings(settings),
        )
        .map_err(|_| {
            provider_commit_failure(
                ProviderCommitErrorCode::InvalidDraft,
                "provider fingerprint validation failed",
            )
        })
    };
    let expected = fingerprint(persisted_settings)?;
    let accepted = request.expected_provider_fingerprint == expected
        || request.expected_provider_fingerprint == fingerprint(persisted_as_shown)?;
    if !accepted || request.previous_active_relay_id != persisted_settings.active_relay_id {
        return Err(provider_commit_failure(
            ProviderCommitErrorCode::StaleState,
            "provider state changed; reload or merge before saving",
        ));
    }
    Ok(expected)
}

fn validate_provider_commit_catalog_structure(
    request: &crate::provider_commit::ProviderCommitRequest,
) -> Result<(), ProviderCommitFailure> {
    for draft in &request.catalog_drafts {
        crate::model_catalog::validate_overlay(&draft.overlay).map_err(|_| {
            provider_commit_failure(
                ProviderCommitErrorCode::CatalogUnavailable,
                "provider catalog structure is invalid",
            )
        })?;
    }
    Ok(())
}

fn validate_provider_topology_mutation_scope(
    persisted_settings: &BackendSettings,
    persisted_state: &crate::model_catalog::CatalogState,
    request: &crate::provider_commit::ProviderCommitRequest,
) -> Result<(), ProviderCommitFailure> {
    use crate::provider_commit::ProviderOwnedTopologyDraft;

    let invalid = || {
        provider_commit_failure(
            ProviderCommitErrorCode::InvalidDraft,
            "provider topology contains detail-owned changes",
        )
    };
    let persisted_raw = ProviderOwnedTopologyDraft::from_settings(persisted_settings);
    let persisted_ui = ui_provider_topology_projection(
        settings_snapshot_for_ui_projection(persisted_settings.clone()).map_err(|_| invalid())?,
    )
    .map_err(|_| invalid())?;
    let profile_copy_signature = |profile: &crate::provider_commit::ProviderRelayProfileDraft| {
        let mut profile = profile.clone();
        profile.id.clear();
        profile.name.clear();
        serde_json::to_vec(&profile).map_err(|_| invalid())
    };
    let catalog_draft_matches_source = |draft: &crate::provider_commit::ProfileCatalogDraft,
                                        source_id: &str| {
        let prior = persisted_state
            .profiles
            .get(source_id)
            .cloned()
            .unwrap_or_default();
        draft.mode == prior.mode
            && draft.mode_explicit == prior.mode_explicit
            && draft.upstream_topology == prior.upstream_topology
            && draft.external_pointer == prior.external_pointer
            && draft.overlay == prior.overlay
    };
    let validate_against = |persisted: &ProviderOwnedTopologyDraft| {
        if request.topology.active_relay_id != persisted.active_relay_id
            || request.topology.active_aggregate_relay_id != persisted.active_aggregate_relay_id
            || request.topology.relay_base_url != persisted.relay_base_url
            || request.topology.relay_api_key != persisted.relay_api_key
            || request.topology.relay_common_config_contents
                != persisted.relay_common_config_contents
            || request.topology.relay_context_config_contents
                != persisted.relay_context_config_contents
        {
            return Err(invalid());
        }

        let mut copied_from = std::collections::HashMap::<String, String>::new();
        for profile in &request.topology.relay_profiles {
            let prior = persisted
                .relay_profiles
                .iter()
                .find(|prior| prior.id == profile.id);
            match prior {
                Some(prior) => {
                    let prior = serde_json::to_vec(prior).map_err(|_| invalid())?;
                    let incoming = serde_json::to_vec(profile).map_err(|_| invalid())?;
                    if incoming != prior {
                        return Err(invalid());
                    }
                }
                None => {
                    let signature = profile_copy_signature(profile)?;
                    let copy_draft = request
                        .catalog_drafts
                        .iter()
                        .find(|draft| draft.profile_id == profile.id);
                    let source = persisted
                        .relay_profiles
                        .iter()
                        .find(|prior| {
                            profile_copy_signature(prior)
                                .map(|prior| prior == signature)
                                .unwrap_or(false)
                                && copy_draft.is_none_or(|draft| {
                                    catalog_draft_matches_source(draft, &prior.id)
                                })
                        })
                        .ok_or_else(invalid)?;
                    copied_from.insert(profile.id.clone(), source.id.clone());
                }
            }
        }

        let retained_profile_ids = request
            .topology
            .relay_profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<HashSet<_>>();
        for aggregate in &request.topology.aggregate_relay_profiles {
            let prior = persisted
                .aggregate_relay_profiles
                .iter()
                .find(|prior| prior.id == aggregate.id);
            match prior {
                Some(prior) => {
                    let expected_members = prior
                        .members
                        .iter()
                        .filter(|member| retained_profile_ids.contains(member.relay_id.as_str()))
                        .collect::<Vec<_>>();
                    if aggregate.name != prior.name
                        || aggregate.strategy != prior.strategy
                        || serde_json::to_vec(&aggregate.members).map_err(|_| invalid())?
                            != serde_json::to_vec(&expected_members).map_err(|_| invalid())?
                    {
                        return Err(invalid());
                    }
                }
                None => {
                    let source_id = copied_from.get(&aggregate.id).ok_or_else(invalid)?;
                    let source = persisted
                        .aggregate_relay_profiles
                        .iter()
                        .find(|prior| prior.id == *source_id)
                        .ok_or_else(invalid)?;
                    if aggregate.strategy != source.strategy
                        || serde_json::to_vec(&aggregate.members).map_err(|_| invalid())?
                            != serde_json::to_vec(&source.members).map_err(|_| invalid())?
                    {
                        return Err(invalid());
                    }
                }
            }
        }

        for draft in &request.catalog_drafts {
            if !persisted
                .relay_profiles
                .iter()
                .any(|profile| profile.id == draft.profile_id)
            {
                let source_id = copied_from.get(&draft.profile_id).ok_or_else(invalid)?;
                if !catalog_draft_matches_source(draft, source_id) {
                    return Err(invalid());
                }
                continue;
            }
            let prior = persisted_state
                .profiles
                .get(&draft.profile_id)
                .cloned()
                .unwrap_or_default();
            if draft.mode != prior.mode
                || draft.mode_explicit != prior.mode_explicit
                || draft.upstream_topology != prior.upstream_topology
                || draft.external_pointer != prior.external_pointer
                || draft.overlay != prior.overlay
            {
                return Err(invalid());
            }
        }
        Ok(())
    };

    validate_against(&persisted_raw).or_else(|_| validate_against(&persisted_ui))
}

fn normalize_provider_topology_settings_fallible(
    mut settings: BackendSettings,
    request: &crate::provider_commit::ProviderCommitRequest,
    persisted_state: &crate::model_catalog::CatalogState,
) -> anyhow::Result<BackendSettings> {
    use crate::model_catalog::CatalogMode;

    settings.relay_common_config_contents.clear();
    settings.relay_context_config_contents.clear();
    for profile in &mut settings.relay_profiles {
        anyhow::ensure!(
            profile.auth_contents.is_empty(),
            "incoming authContents is prohibited"
        );
        let mode = request
            .catalog_drafts
            .iter()
            .find(|draft| draft.profile_id == profile.id)
            .map(|draft| draft.mode)
            .or_else(|| {
                persisted_state
                    .profiles
                    .get(&profile.id)
                    .map(|state| state.mode)
            })
            .unwrap_or(CatalogMode::NativeOfficial);
        if profile.relay_mode != codex_plus_core::settings::RelayMode::Aggregate {
            reconcile_provider_detail_raw_fields(profile)?;
            validate_provider_detail_contract(profile, mode)?;
        }
        codex_plus_core::relay_config::normalize_relay_profile_for_storage(profile)
            .map_err(|_| anyhow::anyhow!("provider profile normalization failed"))?;
        sanitize_profile_after_core_normalize_fallible(profile)?;
        profile.config_contents = retain_provider_owned_profile_config(&profile.config_contents)
            .map_err(|_| anyhow::anyhow!("provider config ownership normalization failed"))?;
        if profile.relay_mode != codex_plus_core::settings::RelayMode::Aggregate {
            validate_provider_detail_contract(profile, mode)?;
        }
    }
    Ok(settings)
}

fn normalize_provider_detail_settings_fallible(
    mut settings: BackendSettings,
    persisted_as_shown: &BackendSettings,
    focused_id: &str,
    focused_mode: crate::model_catalog::CatalogMode,
    confirm_context_cleanup: bool,
) -> anyhow::Result<BackendSettings> {
    let focused = settings
        .relay_profiles
        .iter_mut()
        .find(|profile| profile.id == focused_id)
        .ok_or_else(|| anyhow::anyhow!("focused provider profile is missing"))?;
    reconcile_provider_detail_raw_fields(focused)?;
    validate_provider_detail_contract(focused, focused_mode)?;
    if crate::model_catalog::managed_mode(focused_mode) {
        let conflicts = crate::model_catalog::global_context_conflicts(&focused.config_contents);
        let structured_conflict = !focused.context_window.trim().is_empty()
            || !focused.auto_compact_limit.trim().is_empty();
        if structured_conflict || !conflicts.is_empty() {
            anyhow::ensure!(
                confirm_context_cleanup,
                "managed catalog context cleanup confirmation is required"
            );
            focused.config_contents =
                crate::model_catalog::remove_global_context_keys(&focused.config_contents)?;
            focused.context_window.clear();
            focused.auto_compact_limit.clear();
        }
    }

    // Provider-detail commits do not own a stored copy of live common/context configuration.
    settings.relay_common_config_contents.clear();
    settings.relay_context_config_contents.clear();
    for profile in &mut settings.relay_profiles {
        anyhow::ensure!(
            profile.auth_contents.is_empty(),
            "incoming authContents is prohibited"
        );
        if profile.id != focused_id {
            // A detail commit owns exactly one profile. Every other profile keeps the contract
            // it was persisted with: core storage normalization rewrites a legacy provider-ID
            // alias to its own identity, drops the actor header along with the table it
            // replaces, and restores `requires_openai_auth = true`. Running it here would
            // migrate profiles the user never opened, as a side effect of saving another one.
            if let Some(prior) = persisted_as_shown
                .relay_profiles
                .iter()
                .find(|prior| prior.id == profile.id)
            {
                *profile = prior.clone();
                profile.auth_contents.clear();
            }
            continue;
        }
        codex_plus_core::relay_config::normalize_relay_profile_for_storage(profile)
            .map_err(|_| anyhow::anyhow!("provider profile normalization failed"))?;
        sanitize_profile_after_core_normalize_fallible(profile)?;
        profile.config_contents = retain_provider_owned_profile_config(&profile.config_contents)
            .map_err(|_| anyhow::anyhow!("provider config ownership normalization failed"))?;
    }
    let focused = settings
        .relay_profiles
        .iter()
        .find(|profile| profile.id == focused_id)
        .ok_or_else(|| anyhow::anyhow!("focused provider profile is missing"))?;
    validate_provider_detail_contract(focused, focused_mode)?;
    Ok(settings)
}

fn reconcile_provider_detail_raw_fields(profile: &mut RelayProfile) -> anyhow::Result<()> {
    let document = profile
        .config_contents
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| anyhow::anyhow!("provider config TOML is invalid"))?;

    if let Some(raw_model) = document.get("model") {
        let raw_model = raw_model
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("provider model is malformed"))?;
        reconcile_provider_string(&mut profile.model, raw_model, "provider model conflict")?;
    }

    let Some(provider_id_item) = document.get("model_provider") else {
        return Ok(());
    };
    let provider_id = provider_id_item
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("provider selection is invalid"))?;
    anyhow::ensure!(
        provider_id != "openai",
        "reserved provider selection is invalid"
    );
    let provider = document
        .get("model_providers")
        .and_then(toml_edit::Item::as_table_like)
        .and_then(|providers| providers.get(provider_id))
        .and_then(toml_edit::Item::as_table_like)
        .ok_or_else(|| anyhow::anyhow!("selected provider table is invalid"))?;

    if let Some(raw_base_url) = provider.get("base_url") {
        let raw_base_url = raw_base_url
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("provider base URL is malformed"))?;
        reconcile_provider_string(
            &mut profile.base_url,
            raw_base_url,
            "provider base URL conflict",
        )?;
        if profile.upstream_base_url.trim().is_empty() {
            profile.upstream_base_url = profile.base_url.clone();
        }
    }
    if let Some(raw_key) = provider.get("experimental_bearer_token") {
        let raw_key = raw_key
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("provider bearer is malformed"))?;
        reconcile_provider_string(&mut profile.api_key, raw_key, "provider key conflict")?;
    }
    Ok(())
}

fn reconcile_provider_string(
    structured: &mut String,
    raw: &str,
    conflict: &'static str,
) -> anyhow::Result<()> {
    if structured.trim().is_empty() {
        *structured = raw.to_string();
    } else {
        anyhow::ensure!(structured.as_bytes() == raw.as_bytes(), conflict);
    }
    Ok(())
}

fn sanitize_profile_after_core_normalize_fallible(
    profile: &mut RelayProfile,
) -> anyhow::Result<()> {
    if profile.relay_mode == codex_plus_core::settings::RelayMode::PureApi {
        let api_key = profile_api_key_from_auth(&profile.auth_contents)
            .or_else(|| provider_bearer_token_from_config(&profile.config_contents))
            .or_else(|| (!profile.api_key.trim().is_empty()).then(|| profile.api_key.clone()))
            .ok_or_else(|| anyhow::anyhow!("pure API provider key is missing"))?;
        profile.config_contents =
            set_provider_config_bearer(&profile.config_contents, &api_key, Some(false))
                .map_err(|_| anyhow::anyhow!("pure API provider projection failed"))?;
        profile.api_key = api_key;
    }
    profile.auth_contents.clear();
    Ok(())
}

fn validate_provider_detail_contract(
    profile: &RelayProfile,
    catalog_mode: crate::model_catalog::CatalogMode,
) -> anyhow::Result<()> {
    use crate::provider_native_capability::NativeCapabilityState;

    let inspection = crate::provider_native_capability::inspect_profile(profile, catalog_mode);
    // A degraded contract is refused only for gaps that make the draft unusable. A provider is
    // edited toward the target contract in steps, so a half-repaired draft must stay savable;
    // refusing it strands the profile, because the fields that complete the contract can only be
    // persisted by a save.
    if let Some(blocking) = inspection
        .fields
        .iter()
        .filter(|field| {
            field.outcome != crate::provider_native_capability::NativeCapabilityOutcome::Satisfied
        })
        .map(|field| field.reason)
        .find(|reason| crate::provider_native_capability::reason_blocks_save(*reason))
    {
        anyhow::bail!("native provider contract is invalid: {blocking:?}");
    }
    if inspection.state != NativeCapabilityState::NativePriority {
        return Ok(());
    }
    let document = profile
        .config_contents
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| anyhow::anyhow!("provider config TOML is invalid"))?;
    let provider_id = document
        .get("model_provider")
        .and_then(toml_edit::Item::as_str)
        .ok_or_else(|| anyhow::anyhow!("provider selection is invalid"))?;
    let provider = document
        .get("model_providers")
        .and_then(toml_edit::Item::as_table_like)
        .and_then(|providers| providers.get(provider_id))
        .and_then(toml_edit::Item::as_table_like)
        .ok_or_else(|| anyhow::anyhow!("selected provider table is invalid"))?;
    let raw_base_url = provider
        .get("base_url")
        .and_then(toml_edit::Item::as_str)
        .ok_or_else(|| anyhow::anyhow!("provider base URL is missing"))?;
    anyhow::ensure!(
        profile.base_url.trim() == raw_base_url.trim(),
        "provider base URL conflict"
    );
    let raw_model = document
        .get("model")
        .and_then(toml_edit::Item::as_str)
        .ok_or_else(|| anyhow::anyhow!("provider model is missing"))?;
    anyhow::ensure!(
        profile.model.trim() == raw_model.trim(),
        "provider model conflict"
    );
    let raw_key = provider
        .get("experimental_bearer_token")
        .and_then(toml_edit::Item::as_str)
        .ok_or_else(|| anyhow::anyhow!("provider bearer is missing"))?;
    anyhow::ensure!(
        profile.api_key.trim() == raw_key.trim(),
        "provider key conflict"
    );
    Ok(())
}

pub fn assert_staged_native_provider_contract(
    profile: &RelayProfile,
    staged_config: &str,
    catalog_mode: crate::model_catalog::CatalogMode,
) -> anyhow::Result<()> {
    let mut staged_profile = profile.clone();
    staged_profile.config_contents = staged_config.to_string();
    let inspection =
        crate::provider_native_capability::inspect_profile(&staged_profile, catalog_mode);
    anyhow::ensure!(
        inspection.state
            == crate::provider_native_capability::NativeCapabilityState::NativePriority,
        "staged native provider contract failed canonical assertion"
    );
    validate_provider_detail_contract(&staged_profile, catalog_mode)
        .context("staged provider validation failed")
}

fn materialize_provider_commit_catalogs(
    paths: &ProviderCommitPaths,
    request: &crate::provider_commit::ProviderCommitRequest,
    plan: &crate::provider_commit::ProviderCommitPlan,
) -> anyhow::Result<Vec<FileMutation>> {
    use crate::model_catalog::CatalogMode;

    let mut mutations = Vec::new();
    for draft in &request.catalog_drafts {
        if !matches!(
            draft.mode,
            CatalogMode::OfficialPlusCustom | CatalogMode::CustomOnly
        ) {
            continue;
        }
        let profile_state = plan
            .catalog_state
            .profiles
            .get(&draft.profile_id)
            .ok_or_else(|| anyhow::anyhow!("planned catalog state is missing"))?;
        if profile_state.action_required.is_some() {
            continue;
        }
        let profile = plan
            .settings
            .relay_profiles
            .iter()
            .find(|profile| profile.id == draft.profile_id)
            .ok_or_else(|| anyhow::anyhow!("planned catalog profile is missing"))?;
        let catalog = crate::model_catalog::compose_profile_catalog(
            &plan.catalog_state,
            profile,
            profile_state,
        )
        .map_err(|_| anyhow::anyhow!("provider catalog composition failed"))?;
        crate::model_catalog::validate_catalog_structure(&catalog)
            .map_err(|_| anyhow::anyhow!("provider catalog structure is invalid"))?;
        let bytes = serde_json::to_vec_pretty(&catalog)?;
        let expected_hash = format!("{:x}", Sha256::digest(&bytes));
        anyhow::ensure!(
            profile_state.generated_hash.as_deref() == Some(expected_hash.as_str()),
            "planned provider catalog hash is inconsistent"
        );
        let relative = profile_state
            .generated_path
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("planned provider catalog path is missing"))?;
        let path = paths.codex_home.join(relative);
        if read_optional_bytes(&path)?.as_deref() != Some(bytes.as_slice()) {
            mutations.push(FileMutation::bytes(path, bytes));
        }
    }
    Ok(mutations)
}

#[tauri::command]
pub async fn switch_relay_profile(
    request: RelayProfileSwitchRequest,
) -> CommandResult<RelaySwitchPayload> {
    tauri::async_runtime::spawn_blocking(move || switch_relay_profile_blocking(request))
        .await
        .expect("blocking command panicked")
}

#[tauri::command]
pub async fn save_active_relay_profile(
    request: RelayProfileSwitchRequest,
) -> CommandResult<RelaySwitchPayload> {
    tauri::async_runtime::spawn_blocking(move || switch_relay_profile_blocking(request))
        .await
        .expect("blocking command panicked")
}

fn switch_relay_profile_blocking(
    request: RelayProfileSwitchRequest,
) -> CommandResult<RelaySwitchPayload> {
    if validate_relay_profile_transaction_input(&request.settings).is_err() {
        return switch_relay_profile_blocking_at(&ProviderCommitPaths::defaults(), request);
    }
    let previous_active_relay_id = request.previous_active_relay_id.clone();
    let target_relay_id = request.settings.active_relay_id.clone();
    log_manager_event(
        "manager.switch_relay_profile.start",
        json!({
            "previousActiveRelayId": previous_active_relay_id,
            "targetRelayId": target_relay_id,
        }),
    );
    let result = switch_relay_profile_blocking_at(&ProviderCommitPaths::defaults(), request);
    let event = if result.status == "ok" {
        "manager.switch_relay_profile.ok"
    } else {
        "manager.switch_relay_profile.failed"
    };
    log_manager_event(
        event,
        json!({
            "activeRelayId": result.payload.settings.active_relay_id,
            "error": (result.status == "failed").then_some(result.message.as_str()),
        }),
    );
    result
}

pub(crate) fn switch_relay_profile_blocking_at(
    paths: &ProviderCommitPaths,
    request: RelayProfileSwitchRequest,
) -> CommandResult<RelaySwitchPayload> {
    let home = &paths.codex_home;
    let previous_provider = current_effective_provider_from_home(&home);
    let previous_active_relay_id = request.previous_active_relay_id;
    let confirm_context_cleanup = request.confirm_context_cleanup;
    if let Err(error) = validate_relay_profile_transaction_input(&request.settings) {
        return failed(
            &format!("供应商切换失败：{error}"),
            relay_switch_payload_at(
                BackendSettings::default(),
                codex_plus_core::relay_config::relay_status_from_home(&home),
                None,
                previous_provider.clone(),
                previous_provider,
                &paths.settings_path,
            ),
        );
    }
    match commit_relay_profile_transaction_at(
        paths,
        request.settings,
        &previous_active_relay_id,
        confirm_context_cleanup,
    ) {
        Ok(settings) => {
            let status = codex_plus_core::relay_config::relay_status_from_home(&home);
            let current_provider = current_effective_provider_from_home(&home);
            ok(
                "供应商已切换。",
                relay_switch_payload_at(
                    settings,
                    status,
                    None,
                    previous_provider,
                    current_provider,
                    &paths.settings_path,
                ),
            )
        }
        Err(error) => {
            let status = codex_plus_core::relay_config::relay_status_from_home(&home);
            let current_provider = current_effective_provider_from_home(&home);
            let settings = SettingsStore::new(paths.settings_path.clone())
                .load()
                .map(safe_relay_switch_failure_settings)
                .unwrap_or_default();
            failed(
                &format!("供应商切换失败：{error}"),
                relay_switch_payload_at(
                    settings,
                    status,
                    None,
                    previous_provider,
                    current_provider,
                    &paths.settings_path,
                ),
            )
        }
    }
}

pub(crate) fn commit_relay_profile_transaction(
    settings: BackendSettings,
    previous_active_relay_id: &str,
    confirm_context_cleanup: bool,
) -> anyhow::Result<BackendSettings> {
    commit_relay_profile_transaction_at(
        &ProviderCommitPaths::defaults(),
        settings,
        previous_active_relay_id,
        confirm_context_cleanup,
    )
}

pub(crate) fn commit_relay_profile_transaction_at(
    paths: &ProviderCommitPaths,
    mut settings: BackendSettings,
    previous_active_relay_id: &str,
    confirm_context_cleanup: bool,
) -> anyhow::Result<BackendSettings> {
    validate_relay_profile_transaction_input(&settings)?;
    let home = &paths.codex_home;
    let _guard = live_state::lock()?;
    live_state::prepare_secret_paths_at(&paths.app_state, &paths.settings_path, home)?;
    live_state::recover_locked_at(&paths.app_state)?;
    validate_persisted_responses_only_settings_at(&paths.settings_path)?;
    let migrated = migrate_legacy_profile_auth_locked_at(&paths.settings_path)?;
    if migrated > 0 {
        log_manager_event(
            "manager.profile_auth_migration.completed",
            json!({ "profileCount": migrated }),
        );
    }

    let auth_path = home.join("auth.json");
    let auth_before = read_optional_bytes(&auth_path)?;
    if !previous_active_relay_id.trim().is_empty()
        && previous_active_relay_id != settings.active_relay_id
    {
        backfill_profile_config_only(&home, &mut settings, previous_active_relay_id)?;
    }
    settings = normalize_settings_before_save(settings);
    crate::model_catalog::prepare_active_profile_context_settings(
        &mut settings,
        confirm_context_cleanup,
    )?;
    anyhow::ensure!(
        settings.relay_profiles_enabled,
        "供应商配置总开关已关闭，未写入 live 配置"
    );

    let candidate = stage_active_relay_config_at(home, &paths.app_state, &settings)?;
    let catalog_plan = crate::model_catalog::plan_active_profile_at(
        home,
        &settings,
        &candidate,
        confirm_context_cleanup,
        &paths.catalog_state_path,
    )?;
    let (protected_config, context_snapshot) =
        context_protected_config(&home, &catalog_plan.config_contents)?;
    let settings_bytes = serialize_settings_without_profile_auth(&settings)?;
    let config_path = home.join("config.toml");
    let mut mutations = catalog_plan.mutations;
    mutations.push(FileMutation::bytes(
        paths.settings_path.clone(),
        settings_bytes,
    ));
    mutations.push(FileMutation::text(config_path, protected_config));
    live_state::commit_locked_verified_at(&paths.app_state, &mutations, || {
        verify_context_tables(&home, &context_snapshot)?;
        anyhow::ensure!(
            read_optional_bytes(&auth_path)? == auth_before,
            "live auth changed concurrently; provider transaction was rolled back"
        );
        Ok(())
    })?;
    Ok(sanitize_settings_for_output(settings))
}

fn validate_relay_profile_transaction_input(settings: &BackendSettings) -> anyhow::Result<()> {
    crate::provider_commit::validate_responses_only_settings(settings)?;
    anyhow::ensure!(
        settings
            .relay_profiles
            .iter()
            .all(|profile| profile.auth_contents.is_empty()),
        "incoming authContents is prohibited"
    );
    Ok(())
}

fn safe_relay_switch_failure_settings(settings: BackendSettings) -> BackendSettings {
    validate_relay_profile_transaction_input(&settings)
        .map(|()| sanitize_settings_for_output(settings))
        .unwrap_or_default()
}

fn backfill_profile_config_only(
    home: &Path,
    settings: &mut BackendSettings,
    profile_id: &str,
) -> anyhow::Result<()> {
    let profile = settings
        .relay_profiles
        .iter_mut()
        .find(|profile| profile.id == profile_id)
        .with_context(|| "当前供应商已不在配置列表中，已停止切换以避免覆盖用户改动。")?;
    with_private_staging_home("backfill", |stage_home| {
        seed_staging_config(home, stage_home)?;
        codex_plus_core::relay_config::backfill_relay_profile_from_home_with_common(
            stage_home,
            profile,
            &mut settings.relay_context_config_contents,
        )?;
        profile.auth_contents.clear();
        Ok(())
    })
}

#[cfg(test)]
fn stage_active_relay_config(home: &Path, settings: &BackendSettings) -> anyhow::Result<String> {
    stage_active_relay_config_at(
        home,
        &codex_plus_core::paths::default_app_state_dir(),
        settings,
    )
}

fn stage_active_relay_config_at(
    home: &Path,
    app_state: &Path,
    settings: &BackendSettings,
) -> anyhow::Result<String> {
    let profile = settings.active_relay_profile();
    anyhow::ensure!(
        profile.relay_mode != codex_plus_core::settings::RelayMode::Aggregate,
        "聚合供应商依赖已移除的本地代理，不能写入 live 配置"
    );
    // The provider Codex is currently pointed at. Exiting to pure OAuth deletes that provider, and
    // the core clear only knows to delete its own relay identifiers — a provider staged under any
    // other id would keep its whole table, `experimental_bearer_token` included, in the live file
    // the user was told it had been removed from.
    let live_provider_id = read_optional_bytes(&home.join("config.toml"))?
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|config| config.parse::<toml_edit::DocumentMut>().ok())
        .and_then(|document| {
            document
                .get("model_provider")
                .and_then(toml_edit::Item::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        });
    with_private_staging_home_at(app_state, "provider", |stage_home| {
        seed_staging_config(home, stage_home)?;
        if profile.relay_mode == codex_plus_core::settings::RelayMode::Official
            && !profile.official_mix_api_key
        {
            codex_plus_core::relay_config::clear_relay_config_to_home_with_auth_and_computer_use_guard(
                stage_home,
                None,
                settings.computer_use_guard_enabled,
            )?;
        } else {
            let mut projection = profile.clone();
            projection.auth_contents.clear();
            if profile.relay_mode == codex_plus_core::settings::RelayMode::PureApi {
                let api_key = provider_bearer_token_from_config(&profile.config_contents)
                    .or_else(|| {
                        (!profile.api_key.trim().is_empty()).then(|| profile.api_key.clone())
                    })
                    .context("纯 API 供应商缺少 provider bearer token")?;
                projection.relay_mode = codex_plus_core::settings::RelayMode::Official;
                projection.official_mix_api_key = true;
                projection.api_key = api_key.clone();
                projection.config_contents =
                    set_provider_config_bearer(&projection.config_contents, &api_key, Some(false))?;
            }
            codex_plus_core::relay_config::apply_relay_profile_config_to_home_with_context(
                stage_home,
                &projection,
                &relay_combined_common_config(settings),
            )?;
        }
        anyhow::ensure!(
            !stage_home.join("auth.json").exists(),
            "上游配置生成器意外写入了 staged auth.json"
        );
        let mut config = std::fs::read_to_string(stage_home.join("config.toml"))?;
        if profile.relay_mode == codex_plus_core::settings::RelayMode::PureApi {
            let api_key = provider_bearer_token_from_config(&config)
                .context("纯 API staged 配置缺少 provider bearer token")?;
            config = set_provider_config_bearer(&config, &api_key, Some(false))?;
        }
        if profile.relay_mode == codex_plus_core::settings::RelayMode::Official
            && !profile.official_mix_api_key
            && let Some(removed) = live_provider_id.as_deref()
        {
            config = remove_live_provider_table(&config, removed)?;
        }
        Ok(config)
    })
}

/// Removes one provider table from a staged live configuration, leaving everything else intact.
///
/// Used only where the user explicitly confirmed deleting that provider. An empty
/// `model_providers` container is removed with it so the exit does not leave a husk behind.
fn remove_live_provider_table(config: &str, provider_id: &str) -> anyhow::Result<String> {
    let mut document: toml_edit::DocumentMut = config.parse()?;
    let Some(providers) = document
        .get_mut("model_providers")
        .and_then(toml_edit::Item::as_table_like_mut)
    else {
        return Ok(config.to_string());
    };
    providers.remove(provider_id);
    if providers.is_empty() {
        document.as_table_mut().remove("model_providers");
    }
    Ok(document.to_string())
}

fn seed_staging_config(live_home: &Path, stage_home: &Path) -> anyhow::Result<()> {
    let config = read_optional_bytes(&live_home.join("config.toml"))?;
    if let Some(config) = config {
        live_state::atomic_write_owner_only(&stage_home.join("config.toml"), &config)?;
    }
    Ok(())
}

fn with_private_staging_home<T>(
    label: &str,
    run: impl FnOnce(&Path) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    with_private_staging_home_at(&codex_plus_core::paths::default_app_state_dir(), label, run)
}

fn with_private_staging_home_at<T>(
    app_state: &Path,
    label: &str,
    run: impl FnOnce(&Path) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = app_state.join("private-staging");
    live_state::ensure_owner_only_dir(&root)?;
    let stage_home = root.join(format!("{label}-{}-{nonce}", std::process::id()));
    live_state::ensure_owner_only_dir(&stage_home)?;
    let result = run(&stage_home);
    let cleanup = std::fs::remove_dir_all(&stage_home);
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) => Err(error).context("failed to clean private staging home"),
        (Err(error), _) => Err(error),
    }
}

fn read_optional_bytes(path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

#[tauri::command]
pub fn write_diagnostic_event(event: String, detail: Value) -> CommandResult<Value> {
    let event = sanitize_ui_manager_event(&event);
    let detail = if event == "manager.ui.event" {
        json!({})
    } else {
        detail
    };
    match append_manager_diagnostic(&event, detail) {
        Ok(()) => ok("诊断日志已写入。", json!({})),
        Err(error) => failed(&format!("写入诊断日志失败：{error}"), json!({})),
    }
}

#[tauri::command]
pub fn backfill_relay_profile_from_live(
    request: BackfillRelayProfileRequest,
) -> CommandResult<SettingsBackfillPayload> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let mut settings = request.settings;
    let requested_profile_id = request.profile_id.clone();
    if let Err(error) = validate_relay_profile_transaction_input(&settings) {
        return failed(
            &format!("回填当前供应商配置失败：{error}"),
            SettingsBackfillPayload {
                settings: BackendSettings::default(),
            },
        );
    }
    log_manager_event(
        "manager.backfill_relay_profile_from_live.start",
        json!({
            "profileId": requested_profile_id,
            "activeRelayId": settings.active_relay_id
        }),
    );
    let result = (|| -> anyhow::Result<()> {
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        backfill_profile_config_only(&home, &mut settings, &request.profile_id)?;
        settings = normalize_settings_before_save(settings.clone());
        Ok(())
    })();
    match result {
        Ok(()) => {
            log_manager_event(
                "manager.backfill_relay_profile_from_live.ok",
                json!({
                    "profileId": requested_profile_id
                }),
            );
            ok(
                "当前供应商配置已从 live 文件回填。",
                SettingsBackfillPayload {
                    settings: sanitize_settings_for_output(settings),
                },
            )
        }
        Err(error) => {
            log_manager_event(
                "manager.backfill_relay_profile_from_live.failed",
                json!({
                    "profileId": requested_profile_id,
                    "error": error.to_string()
                }),
            );
            failed(
                &format!("回填当前供应商配置失败：{error}"),
                SettingsBackfillPayload {
                    settings: sanitize_settings_for_output(settings),
                },
            )
        }
    }
}

#[tauri::command]
pub fn extract_relay_common_config(
    request: ExtractRelayCommonConfigRequest,
) -> CommandResult<ExtractRelayCommonConfigPayload> {
    match codex_plus_core::relay_config::extract_common_config_from_config(&request.config_contents)
        .and_then(|common_config_contents| {
            let profile_config_contents =
                codex_plus_core::relay_config::strip_common_config_from_config(
                    &request.config_contents,
                    &common_config_contents,
                )?;
            Ok(ExtractRelayCommonConfigPayload {
                common_config_contents,
                profile_config_contents,
            })
        }) {
        Ok(payload) => ok("通用配置已按兼容切换规则提取。", payload),
        Err(error) => failed(
            &format!("提取通用配置失败：{error}"),
            ExtractRelayCommonConfigPayload {
                common_config_contents: String::new(),
                profile_config_contents: request.config_contents,
            },
        ),
    }
}

#[derive(Debug, Clone)]
struct RelayProfileCompatibilityTestResult {
    http_status: u16,
    endpoint: String,
    response_preview: String,
    compatibility_fallback_used: bool,
    initial_http_status: Option<u16>,
}

/// Awaits a provider-facing request, failing instead of waiting on a stalled upstream forever.
async fn bounded_probe<T>(
    task: impl std::future::Future<Output = anyhow::Result<T>>,
    what: &str,
) -> anyhow::Result<T> {
    match tokio::time::timeout(PROVIDER_PROBE_TIMEOUT, task).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!(
            "{what}超时（{} 秒内没有响应）",
            PROVIDER_PROBE_TIMEOUT.as_secs()
        ),
    }
}

async fn test_relay_profile_with_compatibility(
    profile: &RelayProfile,
    model: &str,
) -> anyhow::Result<RelayProfileCompatibilityTestResult> {
    let initial = bounded_probe(
        codex_plus_core::relay_config::test_relay_profile(profile, model),
        "供应商测试请求",
    )
    .await?;
    if !responses_output_limit_fallback_allowed(
        profile.protocol,
        initial.http_status,
        &initial.response_preview,
    ) {
        return Ok(RelayProfileCompatibilityTestResult {
            http_status: initial.http_status,
            endpoint: initial.endpoint,
            response_preview: initial.response_preview,
            compatibility_fallback_used: false,
            initial_http_status: None,
        });
    }

    let api_key = codex_plus_core::relay_config::relay_profile_api_key(profile);
    anyhow::ensure!(!api_key.trim().is_empty(), "API Key 不能为空");
    let client = codex_plus_core::http_client::proxied_client("CodexMinus/RelayTestFallback")?;
    let payload = json!({
        "model": model.trim(),
        "input": "hi"
    });
    let response = client
        .post(&initial.endpoint)
        .bearer_auth(api_key.trim())
        .header("content-type", "application/json")
        .body(payload.to_string())
        .timeout(PROVIDER_PROBE_TIMEOUT)
        .send()
        .await?;
    let http_status = response.status().as_u16();
    let response_text = response.text().await.unwrap_or_default();
    let response_preview = response_text
        .replace(api_key.trim(), "[REDACTED]")
        .chars()
        .take(320)
        .collect();
    Ok(RelayProfileCompatibilityTestResult {
        http_status,
        endpoint: initial.endpoint,
        response_preview,
        compatibility_fallback_used: true,
        initial_http_status: Some(initial.http_status),
    })
}

fn responses_output_limit_fallback_allowed(
    protocol: codex_plus_core::settings::RelayProtocol,
    http_status: u16,
    response_text: &str,
) -> bool {
    if protocol != codex_plus_core::settings::RelayProtocol::Responses || http_status != 400 {
        return false;
    }

    let normalized = response_text.to_ascii_lowercase();
    if normalized.contains("max_output_tokens")
        && [
            "unknown parameter",
            "unknown field",
            "unrecognized parameter",
            "unsupported parameter",
            "unsupported field",
            "invalid parameter",
            "invalid field",
            "not supported",
        ]
        .iter()
        .any(|phrase| normalized.contains(phrase))
    {
        return true;
    }

    serde_json::from_str::<Value>(response_text)
        .ok()
        .is_some_and(|value| {
            let Some(error) = value.get("error") else {
                return false;
            };
            error.get("type").and_then(Value::as_str) == Some("upstream_error")
                && error.get("message").and_then(Value::as_str) == Some("Upstream request failed")
        })
}

fn compatibility_fallback_note(used: bool) -> &'static str {
    if used {
        " 已通过省略 max_output_tokens 的兼容重试。"
    } else {
        ""
    }
}

fn collect_sensitive_json_strings(value: &Value, values: &mut Vec<String>) {
    match value {
        Value::String(value) if !value.trim().is_empty() => values.push(value.clone()),
        Value::Array(items) => {
            for item in items {
                collect_sensitive_json_strings(item, values);
            }
        }
        Value::Object(object) => {
            for value in object.values() {
                collect_sensitive_json_strings(value, values);
            }
        }
        _ => {}
    }
}

fn provider_sensitive_values(profile: &RelayProfile) -> Vec<String> {
    let mut values = [
        codex_plus_core::relay_config::relay_profile_api_key(profile),
        codex_plus_core::relay_config::relay_profile_base_url(profile),
        profile.upstream_base_url.clone(),
        provider_bearer_token_from_config(&profile.config_contents).unwrap_or_default(),
        profile.auth_contents.clone(),
    ]
    .into_iter()
    .filter(|value| !value.trim().is_empty())
    .collect::<Vec<_>>();
    if let Ok(auth) = serde_json::from_str::<Value>(&profile.auth_contents) {
        collect_sensitive_json_strings(&auth, &mut values);
    }
    values.sort_by_key(|value| std::cmp::Reverse(value.len()));
    values.dedup();
    values
}

fn redact_provider_surface_text(profile: &RelayProfile, text: &str) -> String {
    provider_sensitive_values(profile)
        .into_iter()
        .fold(text.to_string(), |redacted, secret| {
            redacted.replace(&secret, "[REDACTED]")
        })
}

fn sanitize_provider_test_result(
    profile: &RelayProfile,
    mut result: CommandResult<RelayProfileTestPayload>,
) -> CommandResult<RelayProfileTestPayload> {
    result.message = format!(
        "供应商测试完成，HTTP {}。{}",
        result.payload.http_status,
        compatibility_fallback_note(result.payload.compatibility_fallback_used)
    );
    result.payload.endpoint = redact_provider_surface_text(profile, &result.payload.endpoint);
    result.payload.response_preview.clear();
    result
}

fn sanitize_provider_models_result(
    profile: &RelayProfile,
    mut result: CommandResult<RelayProfileModelsPayload>,
) -> CommandResult<RelayProfileModelsPayload> {
    result.message = redact_provider_surface_text(profile, &result.message);
    result.payload.endpoint = redact_provider_surface_text(profile, &result.payload.endpoint);
    result.payload.models = sanitize_provider_model_ids(profile, result.payload.models);
    result
}

fn sanitize_provider_model_ids(profile: &RelayProfile, models: Vec<String>) -> Vec<String> {
    let sensitive = provider_sensitive_values(profile);
    models
        .into_iter()
        .filter(|model| {
            let model = model.trim();
            !model.is_empty()
                && model.len() <= 200
                && model.is_ascii()
                && !model.contains('@')
                && !model.contains('\\')
                && !model.to_ascii_lowercase().starts_with("http://")
                && !model.to_ascii_lowercase().starts_with("https://")
                && model.chars().all(|ch| {
                    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':')
                })
                && !sensitive
                    .iter()
                    .any(|secret| !secret.trim().is_empty() && model.contains(secret))
        })
        .collect()
}

pub(crate) fn sanitize_provider_doctor_result(
    profile: &RelayProfile,
    mut result: CommandResult<ProviderDoctorPayload>,
) -> CommandResult<ProviderDoctorPayload> {
    result.message = redact_provider_surface_text(profile, &result.message);
    result.payload.profile_name =
        redact_provider_surface_text(profile, &result.payload.profile_name);
    result.payload.model = redact_provider_surface_text(profile, &result.payload.model);
    result.payload.summary = redact_provider_surface_text(profile, &result.payload.summary);
    result.payload.recommendation =
        redact_provider_surface_text(profile, &result.payload.recommendation);
    for check in &mut result.payload.checks {
        check.title = redact_provider_surface_text(profile, &check.title);
        check.detail = if check.id == "request" {
            format!(
                "上游请求状态：{}。{}",
                check.status,
                compatibility_fallback_note(result.payload.compatibility_fallback_used)
            )
        } else {
            redact_provider_surface_text(profile, &check.detail)
        };
    }
    result
}

#[tauri::command]
pub async fn test_relay_profile(profile: RelayProfile) -> CommandResult<RelayProfileTestPayload> {
    let profile_name = if profile.name.trim().is_empty() {
        "未命名供应商".to_string()
    } else {
        profile.name.trim().to_string()
    };
    let settings = SettingsStore::default().load().unwrap_or_default();
    let test_model: String = if !profile.test_model.trim().is_empty() {
        // 1. 使用者在該供應商明確填的測試模型
        profile.test_model.trim().to_string()
    } else {
        // 2. 該供應商自己 config.toml 裡的 model（避免串味）
        let from_profile = codex_plus_core::relay_config::relay_profile_model(&profile);
        if from_profile.trim().is_empty() {
            // 3. 最後才用全域預設
            settings.relay_test_model.trim().to_string()
        } else {
            from_profile
        }
    };
    let result = match test_relay_profile_with_compatibility(&profile, &test_model).await {
        Ok(result) => {
            let status = if result.http_status < 400 {
                "ok"
            } else {
                "failed"
            };
            let preview = result.response_preview.trim();
            let detail = if preview.is_empty() {
                "响应内容为空".to_string()
            } else {
                format!("响应：{preview}")
            };
            CommandResult {
                status: status.to_string(),
                message: format!(
                    "已向「{profile_name}」用模型「{test_model}」发送 hi，HTTP {}。{detail}{}",
                    result.http_status,
                    compatibility_fallback_note(result.compatibility_fallback_used)
                ),
                payload: RelayProfileTestPayload {
                    http_status: result.http_status,
                    endpoint: result.endpoint,
                    response_preview: result.response_preview,
                    compatibility_fallback_used: result.compatibility_fallback_used,
                    initial_http_status: result.initial_http_status,
                },
            }
        }
        Err(error) => failed(
            &format!("测试「{profile_name}」失败：{error}"),
            RelayProfileTestPayload {
                http_status: 0,
                endpoint: String::new(),
                response_preview: String::new(),
                compatibility_fallback_used: false,
                initial_http_status: None,
            },
        ),
    };
    sanitize_provider_test_result(&profile, result)
}

#[tauri::command]
pub async fn fetch_relay_profile_models(
    profile: RelayProfile,
) -> CommandResult<RelayProfileModelsPayload> {
    let profile_name = if profile.name.trim().is_empty() {
        "未命名供应商".to_string()
    } else {
        profile.name.trim().to_string()
    };
    let result = match bounded_probe(
        codex_plus_core::model_catalog::fetch_relay_profile_model_ids(&profile),
        "获取模型列表",
    )
    .await
    {
        Ok((models, endpoint)) => {
            let models = sanitize_provider_model_ids(&profile, models);
            // Recording evidence takes the coordinator lock and writes owner-only files. Running
            // that on an async worker blocks the whole runtime thread, so a later save waits on a
            // lock held by a task that cannot yield.
            let recorded = {
                let profile_id = profile.id.clone();
                let endpoint = endpoint.clone();
                let models = models.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    crate::model_catalog::record_provider_evidence(&profile_id, &endpoint, &models)
                })
                .await
            };
            match recorded {
                Ok(Ok(())) => ok(
                    &format!("已从「{profile_name}」获取 {} 个模型。", models.len()),
                    RelayProfileModelsPayload { models, endpoint },
                ),
                Ok(Err(_)) | Err(_) => failed(
                    "模型已获取，但供应商证据保存失败。",
                    RelayProfileModelsPayload { models, endpoint },
                ),
            }
        }
        Err(error) => failed(
            &format!("从「{profile_name}」获取模型失败：{error}"),
            RelayProfileModelsPayload {
                models: Vec::new(),
                endpoint: String::new(),
            },
        ),
    };
    sanitize_provider_models_result(&profile, result)
}

#[tauri::command]
pub async fn diagnose_relay_profile(profile: RelayProfile) -> CommandResult<ProviderDoctorPayload> {
    let profile_name = if profile.name.trim().is_empty() {
        "未命名供应商".to_string()
    } else {
        profile.name.trim().to_string()
    };
    let settings = SettingsStore::default().load().unwrap_or_default();
    let test_model = if !profile.test_model.trim().is_empty() {
        profile.test_model.trim().to_string()
    } else {
        let from_profile = codex_plus_core::relay_config::relay_profile_model(&profile);
        if from_profile.trim().is_empty() {
            settings.relay_test_model.trim().to_string()
        } else {
            from_profile
        }
    };
    let mut checks = Vec::new();

    if profile.relay_mode == codex_plus_core::settings::RelayMode::Official
        && !profile.official_mix_api_key
    {
        checks.push(ProviderDoctorCheck {
            id: "config".to_string(),
            title: "配置完整性".to_string(),
            status: "ok".to_string(),
            detail: "官方登录供应商不需要 Base URL / API Key。".to_string(),
        });
        let payload = ProviderDoctorPayload {
            profile_name,
            model: test_model,
            summary: "官方登录供应商无需 API 诊断。".to_string(),
            recommendation: "如果 Codex 官方账号可用，直接使用官方登录模式即可。".to_string(),
            checks,
            compatibility_fallback_used: false,
            initial_http_status: None,
            request_http_status: None,
        };
        return sanitize_provider_doctor_result(
            &profile,
            ok("Provider Doctor：官方登录供应商无需 API 诊断。", payload),
        );
    }

    if codex_plus_core::relay_config::relay_profile_base_url(&profile)
        .trim()
        .is_empty()
        || codex_plus_core::relay_config::relay_profile_api_key(&profile)
            .trim()
            .is_empty()
    {
        checks.push(ProviderDoctorCheck {
            id: "config".to_string(),
            title: "配置完整性".to_string(),
            status: "failed".to_string(),
            detail: "Base URL 或 API Key 为空。".to_string(),
        });
        let payload = ProviderDoctorPayload {
            profile_name,
            model: test_model,
            summary: "配置不完整，无法发起上游诊断。".to_string(),
            recommendation: "先填写 Base URL 和 API Key；如果是官方账号，请切换到官方登录模式。"
                .to_string(),
            checks,
            compatibility_fallback_used: false,
            initial_http_status: None,
            request_http_status: None,
        };
        return sanitize_provider_doctor_result(
            &profile,
            failed("Provider Doctor：配置不完整。", payload),
        );
    }

    checks.push(ProviderDoctorCheck {
        id: "config".to_string(),
        title: "配置完整性".to_string(),
        status: "ok".to_string(),
        detail: format!(
            "{} / {}",
            codex_plus_core::relay_config::relay_profile_base_url(&profile),
            match profile.protocol {
                codex_plus_core::settings::RelayProtocol::Responses => "Responses API",
                codex_plus_core::settings::RelayProtocol::ChatCompletions => "Chat Completions",
            }
        ),
    });

    match bounded_probe(
        codex_plus_core::model_catalog::fetch_relay_profile_model_ids(&profile),
        "获取模型列表",
    )
    .await
    {
        Ok((models, endpoint)) => {
            let contains_model = !test_model.trim().is_empty()
                && models.iter().any(|model| model == test_model.trim());
            let status = if models.is_empty() {
                "failed"
            } else if contains_model || test_model.trim().is_empty() {
                "ok"
            } else {
                "warning"
            };
            let detail = if models.is_empty() {
                format!("{endpoint} 返回 0 个模型。")
            } else if contains_model || test_model.trim().is_empty() {
                format!("{endpoint} 返回 {} 个模型。", models.len())
            } else {
                format!(
                    "{endpoint} 返回 {} 个模型，但未看到测试模型「{}」。",
                    models.len(),
                    test_model
                )
            };
            checks.push(ProviderDoctorCheck {
                id: "models".to_string(),
                title: "模型列表".to_string(),
                status: status.to_string(),
                detail,
            });
        }
        Err(error) => checks.push(ProviderDoctorCheck {
            id: "models".to_string(),
            title: "模型列表".to_string(),
            status: "failed".to_string(),
            detail: error.to_string(),
        }),
    }

    let mut compatibility_fallback_used = false;
    let mut initial_http_status = None;
    let mut request_http_status = None;
    match test_relay_profile_with_compatibility(&profile, &test_model).await {
        Ok(result) => {
            compatibility_fallback_used = result.compatibility_fallback_used;
            initial_http_status = result.initial_http_status;
            request_http_status = Some(result.http_status);
            let status = if result.http_status < 400 {
                "ok"
            } else {
                "failed"
            };
            let preview = result.response_preview.trim();
            checks.push(ProviderDoctorCheck {
                id: "request".to_string(),
                title: "真实请求".to_string(),
                status: status.to_string(),
                detail: if preview.is_empty() {
                    format!(
                        "{} 返回 HTTP {}，响应内容为空。{}",
                        result.endpoint,
                        result.http_status,
                        compatibility_fallback_note(result.compatibility_fallback_used)
                    )
                } else {
                    format!(
                        "{} 返回 HTTP {}：{}{}",
                        result.endpoint,
                        result.http_status,
                        preview,
                        compatibility_fallback_note(result.compatibility_fallback_used)
                    )
                },
            });
        }
        Err(error) => checks.push(ProviderDoctorCheck {
            id: "request".to_string(),
            title: "真实请求".to_string(),
            status: "failed".to_string(),
            detail: error.to_string(),
        }),
    }

    let failed_count = checks
        .iter()
        .filter(|check| check.status == "failed")
        .count();
    let warning_count = checks
        .iter()
        .filter(|check| check.status == "warning")
        .count();
    let status = if failed_count > 0 {
        "failed"
    } else if warning_count > 0 {
        "ok"
    } else {
        "ok"
    };
    let summary = if failed_count > 0 {
        format!("发现 {failed_count} 项失败，Codex 可能无法使用该供应商。")
    } else if warning_count > 0 {
        format!("基础连接可用，但有 {warning_count} 项需要确认。")
    } else {
        "供应商基础诊断通过。".to_string()
    };
    let recommendation = provider_doctor_recommendation(&checks);
    let message = format!("Provider Doctor：{summary}");
    sanitize_provider_doctor_result(
        &profile,
        CommandResult {
            status: status.to_string(),
            message,
            payload: ProviderDoctorPayload {
                profile_name,
                model: test_model,
                summary,
                recommendation,
                checks,
                compatibility_fallback_used,
                initial_http_status,
                request_http_status,
            },
        },
    )
}

fn provider_doctor_recommendation(checks: &[ProviderDoctorCheck]) -> String {
    if checks
        .iter()
        .any(|check| check.id == "config" && check.status == "failed")
    {
        return "先补齐 Base URL 和 API Key；如果使用官方账号，请切换到官方登录模式。".to_string();
    }
    if checks
        .iter()
        .any(|check| check.id == "models" && check.status == "failed")
    {
        return "优先检查 Base URL 是否包含正确的 /v1 前缀，以及供应商是否支持 /v1/models。"
            .to_string();
    }
    if checks
        .iter()
        .any(|check| check.id == "request" && check.status == "failed")
    {
        return "优先检查测试模型名称、上游协议选择和 Key 权限；如果 Chat Completions 可用，请切到对应协议。".to_string();
    }
    if checks.iter().any(|check| check.status == "warning") {
        return "连接可用，但测试模型没有出现在模型列表里；建议改用上游返回的模型名。".to_string();
    }
    "可以作为 Codex 供应商使用；如果真实对话仍失败，请查看协议代理日志里的上游响应。".to_string()
}

#[tauri::command]
pub async fn apply_relay_injection() -> CommandResult<RelayPayload> {
    tauri::async_runtime::spawn_blocking(|| apply_active_relay_profile_blocking("供应商配置"))
        .await
        .expect("blocking command panicked")
}

#[tauri::command]
pub async fn apply_pure_api_injection() -> CommandResult<RelayPayload> {
    tauri::async_runtime::spawn_blocking(|| apply_active_relay_profile_blocking("纯 API 配置"))
        .await
        .expect("blocking command panicked")
}

fn apply_active_relay_profile_blocking(label: &str) -> CommandResult<RelayPayload> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let settings = SettingsStore::default()
        .load()
        .map(sanitize_settings_for_output)
        .unwrap_or_default();
    let active_id = settings.active_relay_id.clone();
    match commit_relay_profile_transaction(settings, &active_id, false) {
        Ok(_) => {
            let status = codex_plus_core::relay_config::relay_status_from_home(&home);
            ok(
                &format!("{label}已通过统一事务写入，live auth 保持不变。"),
                relay_payload(status, None),
            )
        }
        Err(error) => failed(
            &format!("{label}写入失败：{error}"),
            relay_payload(
                codex_plus_core::relay_config::relay_status_from_home(&home),
                None,
            ),
        ),
    }
}

#[tauri::command]
pub async fn clear_relay_injection() -> CommandResult<RelayPayload> {
    tauri::async_runtime::spawn_blocking(clear_relay_injection_blocking)
        .await
        .expect("blocking command panicked")
}

fn clear_relay_injection_blocking() -> CommandResult<RelayPayload> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    log_manager_event("manager.clear_relay_injection.start", json!({}));
    let result = (|| -> anyhow::Result<()> {
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        live_state::recover_locked()?;
        migrate_legacy_profile_auth_locked()?;
        let auth_path = home.join("auth.json");
        let auth_before = read_optional_bytes(&auth_path)?;
        let staged = with_private_staging_home("clear", |stage_home| {
            seed_staging_config(&home, stage_home)?;
            codex_plus_core::relay_config::clear_relay_config_to_home_with_auth(stage_home, None)?;
            anyhow::ensure!(
                !stage_home.join("auth.json").exists(),
                "clear staged auth.json"
            );
            Ok(std::fs::read_to_string(stage_home.join("config.toml"))?)
        })?;
        let (protected, context_snapshot) = context_protected_config(&home, &staged)?;
        live_state::commit_locked_verified(
            &[FileMutation::text(home.join("config.toml"), protected)],
            || {
                verify_context_tables(&home, &context_snapshot)?;
                anyhow::ensure!(
                    read_optional_bytes(&auth_path)? == auth_before,
                    "live auth changed"
                );
                Ok(())
            },
        )
    })();
    match result {
        Ok(()) => {
            let status = codex_plus_core::relay_config::relay_status_from_home(&home);
            log_manager_event(
                "manager.clear_relay_injection.ok",
                json!({
                    "configured": status.configured
                }),
            );
            ok(
                "已清除 custom 中转 API 模式，并切换到官方 ChatGPT 登录模式。",
                relay_payload(status, None),
            )
        }
        Err(error) => {
            let status = codex_plus_core::relay_config::relay_status_from_home(&home);
            log_manager_event(
                "manager.clear_relay_injection.failed",
                json!({
                    "configured": status.configured,
                    "error": error.to_string()
                }),
            );
            failed(
                &format!("清除中转配置失败：{error}"),
                relay_payload(status, None),
            )
        }
    }
}

fn log_manager_event(event: &str, detail: Value) {
    let _ = append_manager_diagnostic(event, detail);
}

fn sanitize_diagnostic_detail(detail: Value) -> Value {
    fn sanitize(value: Value, key: Option<&str>) -> Option<Value> {
        const ALLOWED_KEYS: &[&str] = &[
            "version",
            "had_visible_windows",
            "requested_guard_port",
            "guard_port",
            "configured",
            "currentRelayId",
            "targetRelayId",
            "targetRelayName",
            "targetRelayMode",
            "activeRelayId",
            "previousActiveRelayId",
            "command",
            "status",
            "launchMode",
            "previousProvider",
            "currentProvider",
            "providerChanged",
            "attempt",
            "retry",
            "activeCount",
            "mismatchCount",
            "archivedCount",
            "archivedRolloutsTraversed",
            "candidateCount",
            "elapsedMs",
            "failedCount",
            "profileCount",
            "sessionScansScheduled",
            "skippedCount",
            "profileId",
            "profileName",
            "error",
            "message",
            "payload",
            "location",
            "file",
            "line",
            "column",
            "socket_path",
            "fallback_lock_path",
        ];
        match value {
            Value::Object(object) => Some(Value::Object(
                object
                    .into_iter()
                    .filter(|(key, _)| ALLOWED_KEYS.contains(&key.as_str()))
                    .filter_map(|(key, value)| {
                        sanitize(value, Some(&key)).map(|value| (key, value))
                    })
                    .collect(),
            )),
            Value::Array(values) => Some(Value::Array(
                values
                    .into_iter()
                    .filter_map(|value| sanitize(value, key))
                    .collect(),
            )),
            Value::String(value) => {
                let key = key.unwrap_or_default();
                let safe_enum = match key {
                    "version" => semver::Version::parse(&value).is_ok(),
                    "status" => matches!(
                        value.as_str(),
                        "ok" | "failed" | "not_implemented" | "stale" | "partial"
                    ),
                    "targetRelayMode" => matches!(
                        value.as_str(),
                        "official" | "mixedApi" | "pureApi" | "aggregate"
                    ),
                    "launchMode" => matches!(value.as_str(), "patch" | "relay"),
                    "command" => matches!(
                        value.as_str(),
                        "clear_relay_injection"
                            | "apply_relay_injection"
                            | "apply_pure_api_injection"
                    ),
                    _ => false,
                };
                if safe_enum {
                    Some(Value::String(value))
                } else if matches!(
                    key,
                    "currentRelayId"
                        | "targetRelayId"
                        | "activeRelayId"
                        | "previousActiveRelayId"
                        | "previousProvider"
                        | "currentProvider"
                        | "profileId"
                ) {
                    let identity_hash = format!("{:x}", Sha256::digest(value.as_bytes()));
                    Some(Value::String(format!("sha256:{}", &identity_hash[..16])))
                } else {
                    Some(Value::String("[REDACTED]".to_string()))
                }
            }
            value @ (Value::Bool(_) | Value::Number(_) | Value::Null) => Some(value),
        }
    }
    sanitize(detail, None).unwrap_or_else(|| json!({}))
}

pub(crate) fn sanitize_diagnostic_detail_for_event(event: &str, detail: Value) -> Value {
    let allowed_ui_fields: Option<&[&str]> = match event {
        "manager.ui.switchRelayProfile.start" => Some(&[
            "currentRelayId",
            "targetRelayId",
            "targetRelayName",
            "targetRelayMode",
        ]),
        "manager.ui.switchRelayProfile.validation_failed" => {
            Some(&["targetRelayId", "targetRelayName", "error"])
        }
        "manager.ui.switchRelayProfile.apply_start" => Some(&[
            "targetRelayId",
            "targetRelayName",
            "previousActiveRelayId",
            "command",
        ]),
        "manager.ui.switchRelayProfile.apply_no_result" => Some(&["targetRelayId"]),
        "manager.ui.switchRelayProfile.apply_failed" => {
            Some(&["targetRelayId", "status", "message", "activeRelayId"])
        }
        "manager.ui.switchRelayProfile.ok" => Some(&[
            "targetRelayId",
            "launchMode",
            "status",
            "previousProvider",
            "currentProvider",
            "providerChanged",
        ]),
        "manager.ui.event" => Some(&[]),
        _ => None,
    };
    let detail = match (allowed_ui_fields, detail) {
        (Some(allowed), Value::Object(object)) => Value::Object(
            object
                .into_iter()
                .filter(|(key, _)| allowed.contains(&key.as_str()))
                .collect(),
        ),
        (Some(_), _) => json!({}),
        (None, detail) => detail,
    };
    sanitize_diagnostic_detail(detail)
}

pub(crate) fn append_manager_diagnostic(event: &str, detail: Value) -> anyhow::Result<()> {
    let event = sanitize_manager_event(event);
    codex_plus_core::diagnostic_log::append_diagnostic_log(
        &event,
        sanitize_diagnostic_detail_for_event(&event, detail),
    )?;
    Ok(())
}

fn sanitize_ui_manager_event(event: &str) -> String {
    match event.trim() {
        "switchRelayProfile.start"
        | "switchRelayProfile.validation_failed"
        | "switchRelayProfile.apply_start"
        | "switchRelayProfile.apply_no_result"
        | "switchRelayProfile.apply_failed"
        | "switchRelayProfile.ok" => sanitize_manager_event(event),
        _ => "manager.ui.event".to_string(),
    }
}

fn sanitize_manager_event(event: &str) -> String {
    let suffix = event
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let suffix = suffix.trim_matches(['.', '_', '-']).trim();
    if suffix.is_empty() {
        "manager.ui.event".to_string()
    } else if suffix.starts_with("manager.") {
        suffix.to_string()
    } else {
        format!("manager.ui.{suffix}")
    }
}

fn relay_payload(
    status: codex_plus_core::relay_config::RelayStatus,
    backup_path: Option<String>,
) -> RelayPayload {
    let account_label = redacted_account_label(status.authenticated, status.account_label);
    RelayPayload {
        authenticated: status.authenticated,
        auth_source: status.auth_source,
        account_label,
        config_path: status.config_path,
        configured: status.configured,
        requires_openai_auth: status.requires_openai_auth,
        has_bearer_token: status.has_bearer_token,
        backup_path,
    }
}

fn normalized_secret_key(key: &str) -> String {
    key.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn redact_toml_diagnostic_value(value: &mut Value, inside_headers: bool) {
    match value {
        Value::Array(items) => {
            for item in items {
                redact_toml_diagnostic_value(item, inside_headers);
            }
        }
        Value::Object(object) => {
            for (key, value) in object {
                let normalized = normalized_secret_key(key);
                let actor_marker = normalized == "xopenaiactorauthorization"
                    && value.as_str() == Some("local-image-extension");
                let header_container = matches!(normalized.as_str(), "headers" | "httpheaders");
                let credential_key = matches!(
                    normalized.as_str(),
                    "baseurl"
                        | "upstreambaseurl"
                        | "token"
                        | "bearertoken"
                        | "clientsecret"
                        | "secret"
                        | "password"
                        | "openaiapikey"
                        | "apikey"
                        | "experimentalbearertoken"
                        | "authorization"
                        | "proxyauthorization"
                        | "xapikey"
                        | "xopenaiapikey"
                        | "cookie"
                        | "setcookie"
                        | "accesstoken"
                        | "refreshtoken"
                        | "idtoken"
                        | "authcontents"
                );
                if credential_key || (inside_headers && !actor_marker) {
                    *value = Value::String("[REDACTED]".to_string());
                } else {
                    redact_toml_diagnostic_value(value, inside_headers || header_container);
                }
            }
        }
        _ => {}
    }
}

fn redact_live_config_for_output(config: &str) -> String {
    let Ok(mut value) = toml_edit::de::from_str::<Value>(config) else {
        return "# 配置包含无法安全解析的内容；已停止在诊断界面显示。\n".to_string();
    };
    redact_toml_diagnostic_value(&mut value, false);
    toml_edit::ser::to_string_pretty(&value)
        .unwrap_or_else(|_| "# 配置无法安全序列化；已停止在诊断界面显示。\n".to_string())
}

fn relay_switch_payload_at(
    settings: BackendSettings,
    status: codex_plus_core::relay_config::RelayStatus,
    backup_path: Option<String>,
    previous_provider: String,
    current_provider: String,
    settings_path: &Path,
) -> RelaySwitchPayload {
    let provider_changed = previous_provider != current_provider;
    RelaySwitchPayload {
        settings,
        relay: relay_payload(status, backup_path),
        settings_path: settings_path.to_string_lossy().to_string(),
        user_scripts: user_script_inventory(),
        previous_provider,
        current_provider,
        provider_changed,
    }
}

/// Codex Minus 核心保证：供应商切换/注入永远不改动 config.toml 里不属于供应商的
/// mcp_servers / skills / plugins 三张表。上游 core 的写入流程会用 settings 里的
/// managed 副本对这些表做合并与选择过滤（正是历史上吞掉 `[mcp_servers.memory]`
/// 的根源），所以这里在写入前快照、写入后原样回植。
const PROTECTED_CONTEXT_TABLES: &[&str] = &["mcp_servers", "skills", "plugins"];

#[derive(Clone)]
struct ContextTablesSnapshot {
    tables: Vec<(&'static str, Option<toml_edit::Item>)>,
}

fn snapshot_context_tables(home: &Path) -> anyhow::Result<ContextTablesSnapshot> {
    let contents = read_optional_text_file(&home.join("config.toml"))?;
    let doc: toml_edit::DocumentMut = contents.parse()?;
    Ok(ContextTablesSnapshot {
        tables: PROTECTED_CONTEXT_TABLES
            .iter()
            .map(|name| (*name, doc.get(name).cloned()))
            .collect(),
    })
}

/// 隐式表（只含子表）单独 to_string 会渲染成空串，必须挂进临时 Document
/// 再整体渲染才能得到可比较的文本。
fn render_context_table(name: &str, item: Option<&toml_edit::Item>) -> String {
    match item {
        Some(item) => {
            let mut doc = toml_edit::DocumentMut::new();
            doc[name] = item.clone();
            doc.to_string()
        }
        None => String::new(),
    }
}

#[cfg(test)]
fn restore_context_tables(home: &Path, snapshot: &ContextTablesSnapshot) -> anyhow::Result<()> {
    let config_path = home.join("config.toml");
    let contents = read_optional_text_file(&config_path)?;
    let mut doc: toml_edit::DocumentMut = contents.parse()?;
    let mut changed = false;
    for (name, item) in &snapshot.tables {
        let live_rendered = render_context_table(name, doc.get(name));
        let snapshot_rendered = render_context_table(name, item.as_ref());
        if live_rendered == snapshot_rendered {
            continue;
        }
        match item {
            Some(item) => {
                doc[*name] = item.clone();
            }
            None => {
                doc.as_table_mut().remove(name);
            }
        }
        changed = true;
    }
    if changed {
        live_state::atomic_write_owner_only(&config_path, doc.to_string().as_bytes())?;
        log_manager_event(
            "manager.context_guard.restored",
            json!({ "tables": PROTECTED_CONTEXT_TABLES }),
        );
    }
    verify_context_tables(home, snapshot)
}

fn verify_context_tables(home: &Path, snapshot: &ContextTablesSnapshot) -> anyhow::Result<()> {
    let current = snapshot_context_tables(home)?;
    for ((name, expected), (current_name, actual)) in
        snapshot.tables.iter().zip(current.tables.iter())
    {
        anyhow::ensure!(name == current_name, "Context snapshot table order changed");
        anyhow::ensure!(
            render_context_table(name, expected.as_ref())
                == render_context_table(current_name, actual.as_ref()),
            "受保护 Context 表 {name} 写入后校验失败"
        );
    }
    Ok(())
}

fn context_protected_config(
    home: &Path,
    candidate: &str,
) -> anyhow::Result<(String, ContextTablesSnapshot)> {
    let live_contents = read_optional_text_file(&home.join("config.toml"))?;
    let live_doc: toml_edit::DocumentMut = live_contents.parse()?;
    let snapshot = snapshot_context_tables(home)?;
    let mut candidate_doc: toml_edit::DocumentMut = candidate.parse()?;

    let live_keys = live_doc
        .as_table()
        .iter()
        .map(|(name, _)| name.to_string())
        .collect::<std::collections::HashSet<_>>();
    let candidate_keys = candidate_doc
        .as_table()
        .iter()
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    for name in candidate_keys {
        if !is_provider_owned_root_item(&name) && !live_keys.contains(&name) {
            candidate_doc.as_table_mut().remove(&name);
        }
    }
    for (name, item) in live_doc.as_table().iter() {
        if !is_provider_owned_root_item(name) {
            candidate_doc[name] = item.clone();
        }
    }
    for (name, item) in &snapshot.tables {
        match item {
            Some(item) => candidate_doc[*name] = item.clone(),
            None => {
                candidate_doc.as_table_mut().remove(name);
            }
        }
    }

    let rendered = candidate_doc.to_string();
    let parsed_back: toml_edit::DocumentMut = rendered.parse()?;
    for (name, item) in &snapshot.tables {
        anyhow::ensure!(
            render_context_table(name, parsed_back.get(name))
                == render_context_table(name, item.as_ref()),
            "受保护 Context 表 {name} 回植校验失败"
        );
    }
    for (name, item) in live_doc.as_table().iter() {
        if is_provider_owned_root_item(name) {
            continue;
        }
        anyhow::ensure!(
            render_toml_item(name, parsed_back.get(name)) == render_toml_item(name, Some(item)),
            "无关根配置 {name} 未能保持原样"
        );
    }
    Ok((rendered, snapshot))
}

fn is_provider_owned_root_item(name: &str) -> bool {
    matches!(
        name,
        "model"
            | "model_provider"
            | "model_catalog_json"
            | "base_url"
            | "OPENAI_API_KEY"
            | "model_context_window"
            | "model_auto_compact_token_limit"
            | "codex_plus_chat_base_url"
            | "model_providers"
    )
}

fn retain_provider_owned_profile_config(
    config: &str,
) -> Result<String, PersistedProviderConfigError> {
    if config.trim().is_empty() {
        return Ok(String::new());
    }
    let mut doc: toml_edit::DocumentMut = config
        .parse()
        .map_err(|_| PersistedProviderConfigError::Invalid)?;
    let keys = doc
        .as_table()
        .iter()
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    for name in keys {
        if !is_provider_owned_root_item(&name) {
            doc.as_table_mut().remove(&name);
        }
    }
    Ok(doc.to_string())
}

fn render_toml_item(name: &str, item: Option<&toml_edit::Item>) -> String {
    match item {
        Some(item) => {
            let mut doc = toml_edit::DocumentMut::new();
            doc[name] = item.clone();
            doc.to_string()
        }
        None => String::new(),
    }
}

/// 销毁 settings 中旧版 Manager 保存的全局 config 副本。
/// live config.toml 是唯一事实源；供应商档案只保留供应商拥有的字段。
fn scrub_legacy_managed_config_state(settings: &mut BackendSettings) -> bool {
    let mut dirty = false;
    if !settings.relay_common_config_contents.is_empty() {
        settings.relay_common_config_contents = String::new();
        dirty = true;
    }
    if !settings.relay_context_config_contents.is_empty() {
        settings.relay_context_config_contents = String::new();
        dirty = true;
    }
    for profile in &mut settings.relay_profiles {
        match retain_provider_owned_profile_config(&profile.config_contents) {
            Ok(config) if config != profile.config_contents => {
                profile.config_contents = config;
                dirty = true;
            }
            Ok(_) => {}
            Err(error) => log_manager_event(
                "manager.retain_provider_owned_profile_config.failed",
                json!({
                    "profileId": profile.id,
                    "profileName": profile.name,
                    "error": error.to_string()
                }),
            ),
        }
        if profile.context_selection_initialized
            || profile.context_selection != RelayContextSelection::default()
        {
            profile.context_selection = RelayContextSelection::default();
            profile.context_selection_initialized = false;
            dirty = true;
        }
    }
    dirty
}

pub fn scrub_legacy_managed_config_store() {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let result = (|| -> anyhow::Result<bool> {
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        live_state::recover_locked()?;
        migrate_legacy_profile_auth_locked()?;
        let mut settings = SettingsStore::default().load()?;
        let dirty = scrub_legacy_managed_config_state(&mut settings);
        settings = normalize_settings_before_save(settings);
        if dirty {
            live_state::commit_locked(&[FileMutation::bytes(
                codex_plus_core::paths::default_settings_path(),
                serialize_settings_without_profile_auth(&settings)?,
            )])?;
        }
        Ok(dirty)
    })();
    match result {
        Ok(true) => log_manager_event("manager.live_config.store_scrubbed", json!({})),
        Ok(false) => {}
        Err(error) => log_manager_event(
            "manager.live_config.store_scrub_failed",
            json!({ "error": error.to_string() }),
        ),
    }
}

fn relay_files_payload_from_home(home: &std::path::Path) -> anyhow::Result<RelayFilesPayload> {
    let config_path = home.join("config.toml");
    let auth_path = home.join("auth.json");
    Ok(RelayFilesPayload {
        config_path: config_path.to_string_lossy().to_string(),
        auth_path: auth_path.to_string_lossy().to_string(),
        config_contents: redact_live_config_for_output(&read_optional_text_file(&config_path)?),
        auth_status: live_auth_status_payload(home),
    })
}

fn live_auth_status_payload(home: &Path) -> LiveAuthStatusPayload {
    let status = codex_plus_core::relay_config::chatgpt_auth_status_from_home(home);
    LiveAuthStatusPayload {
        authenticated: status.authenticated,
        source: status.source,
        account_label: redacted_account_label(status.authenticated, status.account_label),
        action_required: (!status.authenticated)
            .then(|| "请在官方 Codex/ChatGPT 客户端中登录。".to_string()),
    }
}

fn redacted_account_label(authenticated: bool, account_label: Option<String>) -> Option<String> {
    (authenticated && account_label.is_some()).then(|| "ChatGPT account".to_string())
}

fn read_optional_text_file(path: &std::path::Path) -> anyhow::Result<String> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

fn open_url(url: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        codex_plus_core::windows_open_url(url)
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| anyhow::anyhow!("启动系统浏览器失败：{error}"))
    }
}

fn settings_payload(message: &str, failure_context: &str) -> CommandResult<SettingsPayload> {
    match settings_payload_value() {
        Ok(payload) => ok(message, payload),
        Err((error, payload)) => failed(&format!("{failure_context}：{error}"), payload),
    }
}

fn settings_payload_value() -> Result<SettingsPayload, (anyhow::Error, SettingsPayload)> {
    let store = SettingsStore::default();
    let settings_path = codex_plus_core::paths::default_settings_path()
        .to_string_lossy()
        .to_string();
    match store.load() {
        Ok(settings) => {
            crate::provider_commit::validate_responses_only_settings(&settings).map_err(
                |error| {
                    (
                        anyhow::Error::new(error),
                        SettingsPayload {
                            settings: BackendSettings::default(),
                            settings_path: settings_path.clone(),
                            user_scripts: user_script_inventory(),
                            provider_fingerprint: String::new(),
                        },
                    )
                },
            )?;
            let settings = sanitize_settings_for_output(settings);
            let provider_fingerprint = crate::provider_commit::provider_owned_fingerprint(
                &crate::provider_commit::ProviderOwnedTopologyDraft::from_settings(&settings),
            )
            .unwrap_or_default();
            Ok(SettingsPayload {
                settings,
                settings_path,
                user_scripts: user_script_inventory(),
                provider_fingerprint,
            })
        }
        Err(error) => Err((
            error,
            SettingsPayload {
                settings: BackendSettings::default(),
                settings_path,
                user_scripts: user_script_inventory(),
                provider_fingerprint: String::new(),
            },
        )),
    }
}

fn fallback_settings_payload() -> SettingsPayload {
    SettingsPayload {
        settings: BackendSettings::default(),
        settings_path: codex_plus_core::paths::default_settings_path()
            .to_string_lossy()
            .to_string(),
        user_scripts: user_script_inventory(),
        provider_fingerprint: String::new(),
    }
}

fn sanitize_settings_for_output(mut settings: BackendSettings) -> BackendSettings {
    for profile in &mut settings.relay_profiles {
        sanitize_profile_after_core_normalize(profile);
    }
    settings
}

fn serialize_settings_without_profile_auth(settings: &BackendSettings) -> anyhow::Result<Vec<u8>> {
    let mut value = serde_json::to_value(settings)?;
    if let Some(profiles) = value.get_mut("relayProfiles").and_then(Value::as_array_mut) {
        for profile in profiles {
            if let Some(profile) = profile.as_object_mut() {
                profile.remove("authContents");
            }
        }
    }
    Ok(serde_json::to_vec_pretty(&value)?)
}

fn migrate_legacy_profile_auth_locked() -> anyhow::Result<()> {
    let settings_path = codex_plus_core::paths::default_settings_path();
    validate_persisted_responses_only_settings_at(&settings_path)?;
    let migrated = migrate_legacy_profile_auth_locked_at(&settings_path)?;
    if migrated > 0 {
        log_manager_event(
            "manager.profile_auth_migration.completed",
            json!({ "profileCount": migrated }),
        );
    }
    Ok(())
}

pub(crate) fn validate_persisted_responses_only_settings_at(path: &Path) -> anyhow::Result<()> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).context("persisted provider settings are unreadable"),
    };
    let settings: BackendSettings =
        serde_json::from_slice(&bytes).context("persisted provider settings are invalid")?;
    crate::provider_commit::validate_responses_only_settings(&settings)
        .context("persisted provider settings contain an unsupported provider topology")
}

#[derive(Debug)]
enum LegacyProfileAuthMigrationError {
    SettingsUnreadable(anyhow::Error),
    SettingsInvalidJson(anyhow::Error),
    ProfileReconciliation(anyhow::Error),
    SecureStorage(anyhow::Error),
}

impl std::fmt::Display for LegacyProfileAuthMigrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileReconciliation(error) => write!(formatter, "{error:#}"),
            Self::SettingsUnreadable(error)
            | Self::SettingsInvalidJson(error)
            | Self::SecureStorage(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LegacyProfileAuthMigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SettingsUnreadable(error)
            | Self::SettingsInvalidJson(error)
            | Self::ProfileReconciliation(error)
            | Self::SecureStorage(error) => Some(error.as_ref()),
        }
    }
}

fn migrate_legacy_profile_auth_locked_at(
    settings_path: &Path,
) -> Result<usize, LegacyProfileAuthMigrationError> {
    if !settings_path.exists() {
        return Ok(0);
    }
    live_state::ensure_owner_only_file(settings_path)
        .map_err(LegacyProfileAuthMigrationError::SecureStorage)?;
    let raw = std::fs::read(settings_path).map_err(|error| {
        LegacyProfileAuthMigrationError::SettingsUnreadable(
            anyhow::Error::new(error).context("persisted provider settings are unreadable"),
        )
    })?;
    let mut settings: BackendSettings = serde_json::from_slice(&raw).map_err(|error| {
        LegacyProfileAuthMigrationError::SettingsInvalidJson(
            anyhow::Error::new(error).context("persisted provider settings are invalid"),
        )
    })?;
    let mut migrated = 0;
    for profile in &mut settings.relay_profiles {
        if profile.auth_contents.is_empty() {
            continue;
        }
        let profile_label = if profile.name.trim().is_empty() {
            profile.id.trim().to_string()
        } else {
            profile.name.trim().to_string()
        };
        migrate_persisted_legacy_api_key_auth(profile)
            .with_context(|| format!("provider profile {profile_label:?} failed auth migration"))
            .map_err(LegacyProfileAuthMigrationError::ProfileReconciliation)?;
        profile.config_contents = retain_provider_owned_profile_config(&profile.config_contents)
            .map_err(PersistedProfileAuthMigrationError::from)
            .with_context(|| format!("provider profile {profile_label:?} failed auth migration"))
            .map_err(LegacyProfileAuthMigrationError::ProfileReconciliation)?;
        migrated += 1;
    }
    if migrated == 0 {
        return Ok(0);
    }
    let bytes = serialize_settings_without_profile_auth(&settings)
        .map_err(LegacyProfileAuthMigrationError::SecureStorage)?;
    // Credential migration intentionally has no prior-file backup. The old file is
    // secured first and then atomically replaced so OAuth copies cannot survive in
    // a recovery artifact.
    live_state::atomic_write_owner_only(settings_path, &bytes)
        .map_err(LegacyProfileAuthMigrationError::SecureStorage)?;
    Ok(migrated)
}

fn user_script_inventory() -> Value {
    // 用户脚本功能已随注入一并移除；返回空清单，不再触碰 Codex++ 的配置目录。
    json!({ "enabled": false, "scripts": [] })
}

fn ok<T: Serialize>(message: &str, payload: T) -> CommandResult<T> {
    CommandResult {
        status: "ok".to_string(),
        message: message.to_string(),
        payload,
    }
}

fn failed<T: Serialize>(message: &str, payload: T) -> CommandResult<T> {
    CommandResult {
        status: "failed".to_string(),
        message: message.to_string(),
        payload,
    }
}

fn default_debug_port() -> u16 {
    9229
}

fn default_helper_port() -> u16 {
    57321
}

fn default_log_lines() -> usize {
    200
}

fn provider_compatibility_from_sessions(
    current_provider: String,
    sessions: &[codex_plus_data::LocalSession],
) -> ProviderCompatibilityPayload {
    let active = sessions
        .iter()
        .filter(|session| !session.archived)
        .collect::<Vec<_>>();
    let mismatch_count = active
        .iter()
        .filter(|session| session.model_provider != current_provider)
        .count();
    let missing_provider_count = active
        .iter()
        .filter(|session| session.model_provider.trim().is_empty())
        .count();
    let mut generation = DefaultHasher::new();
    current_provider.hash(&mut generation);
    active.len().hash(&mut generation);
    for session in &active {
        session.id.hash(&mut generation);
        session.model_provider.hash(&mut generation);
        session.updated_at_ms.hash(&mut generation);
    }
    ProviderCompatibilityPayload {
        current_provider,
        active_count: active.len(),
        mismatch_count,
        missing_provider_count,
        scan_generation: format!("{:016x}", generation.finish()),
        encrypted_content_warning: None,
        adaptation_available: mismatch_count > 0,
        adaptation_message: "适配会逐个改写这些活动会话自己的记录文件与数据库行，写前自动备份；归档历史不会被读取或写入。".to_string(),
        scan_elapsed_ms: 0,
        archived_rollouts_traversed: 0,
    }
}

fn provider_compatibility_payload() -> ProviderCompatibilityPayload {
    let home = codex_plus_core::codex_sqlite::default_codex_home_dir();
    let current_provider = current_effective_provider_from_home(&home);
    let (_, sessions, _) = load_local_session_inventory();
    provider_compatibility_from_sessions(current_provider, &sessions)
}

#[tauri::command]
pub async fn scan_provider_compatibility() -> CommandResult<ProviderCompatibilityPayload> {
    tauri::async_runtime::spawn_blocking(|| {
        let started = Instant::now();
        let mut payload = provider_compatibility_payload();
        payload.scan_elapsed_ms = started.elapsed().as_millis();
        log_manager_event(
            "manager.provider_compatibility.scan",
            json!({
                "currentProvider": payload.current_provider,
                "activeCount": payload.active_count,
                "mismatchCount": payload.mismatch_count,
                "elapsedMs": payload.scan_elapsed_ms,
                "archivedRolloutsTraversed": payload.archived_rollouts_traversed
            }),
        );
        ok("活动会话供应商兼容性已检查。", payload)
    })
    .await
    .expect("blocking command panicked")
}

#[tauri::command]
pub async fn adapt_active_sessions_to_current_provider(
    scan_generation: String,
) -> CommandResult<ProviderCompatibilityPayload> {
    tauri::async_runtime::spawn_blocking(move || {
        let before = provider_compatibility_payload();
        if scan_generation != before.scan_generation {
            return failed("兼容性检查结果已过期，请重新检查。", before);
        }
        if before.mismatch_count == 0 {
            return ok("活动会话的 provider 已兼容当前配置。", before);
        }
        let home = codex_plus_core::codex_sqlite::default_codex_home_dir();
        let (_, sessions, _) = load_local_session_inventory();
        match crate::session_adaptation::adapt_active_sessions_to_provider(
            &home,
            &before.current_provider,
            &sessions,
        ) {
            Ok(outcome) => {
                let mut payload = provider_compatibility_payload();
                if outcome.encrypted_sessions > 0 {
                    payload.encrypted_content_warning = Some(format!(
                        "{} 个会话带有上一供应商的 encrypted_content：标记已适配，但继续或压缩这些对话可能失败；需要可靠续聊时请切回原供应商或开新会话。",
                        outcome.encrypted_sessions
                    ));
                }
                log_manager_event(
                    "manager.provider_compatibility.adapt",
                    json!({
                        "targetProvider": payload.current_provider,
                        "adapted": outcome.adapted,
                        "skippedLocked": outcome.skipped_locked,
                        "failed": outcome.failed,
                        "encryptedSessions": outcome.encrypted_sessions,
                        "backupDir": outcome
                            .backup_dir
                            .as_ref()
                            .map(|path| path.to_string_lossy().to_string()),
                    }),
                );
                let mut message = format!("已适配 {} 个活动会话，写前已备份。", outcome.adapted);
                if outcome.skipped_locked > 0 {
                    message.push_str(&format!(
                        "另有 {} 个被运行中的 Codex 占用，已跳过；关闭 Codex 后可重试。",
                        outcome.skipped_locked
                    ));
                }
                if let Some(warning) = &payload.encrypted_content_warning {
                    message.push_str(warning);
                }
                if outcome.failed > 0 {
                    return failed(
                        &format!(
                            "{} 个会话适配失败，其改动已回退。{message}",
                            outcome.failed
                        ),
                        payload,
                    );
                }
                ok(&message, payload)
            }
            Err(error) => {
                let payload = provider_compatibility_payload();
                failed(&format!("适配未执行：{error}"), payload)
            }
        }
    })
    .await
    .expect("blocking command panicked")
}

#[cfg(test)]
mod session_lifecycle_tests {
    use super::*;

    fn responses_only_settings_for_route_tests() -> BackendSettings {
        let mut settings = BackendSettings::default();
        settings.relay_profiles_enabled = true;
        settings
    }

    fn unsupported_route_settings(kind: &str) -> BackendSettings {
        let mut settings = responses_only_settings_for_route_tests();
        match kind {
            "chat" => {
                settings.relay_profiles[0].protocol =
                    codex_plus_core::settings::RelayProtocol::ChatCompletions
            }
            "proxy" => {
                settings.relay_profiles[0].base_url = "http://127.0.0.1:57321/v1".to_string()
            }
            "auth" => {
                settings.relay_profiles[0].auth_contents =
                    r#"{"OPENAI_API_KEY":"incoming-auth-must-not-migrate"}"#.to_string()
            }
            "aggregate-profile" => {
                settings.relay_profiles[0].relay_mode =
                    codex_plus_core::settings::RelayMode::Aggregate
            }
            "aggregate-metadata" => settings.aggregate_relay_profiles.push(
                codex_plus_core::settings::AggregateRelayProfile {
                    id: "removed-aggregate".to_string(),
                    name: "Removed aggregate".to_string(),
                    strategy: Default::default(),
                    members: Vec::new(),
                },
            ),
            "active-aggregate" => {
                settings.active_aggregate_relay_id = "removed-aggregate".to_string()
            }
            _ => panic!("unknown unsupported route test fixture"),
        }
        settings
    }

    fn assert_safe_route_settings(settings: &BackendSettings) {
        assert!(crate::provider_commit::validate_responses_only_settings(settings).is_ok());
        assert!(
            settings
                .relay_profiles
                .iter()
                .all(|profile| profile.auth_contents.is_empty())
        );
    }

    #[test]
    fn switch_and_save_routes_reject_removed_topologies_before_the_transaction() {
        for kind in [
            "chat",
            "proxy",
            "auth",
            "aggregate-profile",
            "aggregate-metadata",
            "active-aggregate",
        ] {
            let settings = unsupported_route_settings(kind);
            assert!(validate_relay_profile_transaction_input(&settings).is_err());

            let switch =
                tauri::async_runtime::block_on(switch_relay_profile(RelayProfileSwitchRequest {
                    settings: settings.clone(),
                    previous_active_relay_id: String::new(),
                    confirm_context_cleanup: false,
                }));
            assert_eq!(switch.status, "failed");
            assert_safe_route_settings(&switch.payload.settings);

            let save = tauri::async_runtime::block_on(save_active_relay_profile(
                RelayProfileSwitchRequest {
                    settings,
                    previous_active_relay_id: String::new(),
                    confirm_context_cleanup: false,
                },
            ));
            assert_eq!(save.status, "failed");
            assert_safe_route_settings(&save.payload.settings);
        }
    }

    #[test]
    fn backfill_route_rejects_removed_topologies_with_a_safe_fallback() {
        for kind in [
            "chat",
            "proxy",
            "auth",
            "aggregate-profile",
            "aggregate-metadata",
            "active-aggregate",
        ] {
            let result = backfill_relay_profile_from_live(BackfillRelayProfileRequest {
                settings: unsupported_route_settings(kind),
                profile_id: "default".to_string(),
            });

            assert_eq!(result.status, "failed");
            assert_safe_route_settings(&result.payload.settings);
        }
    }

    #[test]
    fn switch_failure_fallback_never_reflects_rejected_persisted_topology() {
        for kind in [
            "chat",
            "proxy",
            "auth",
            "aggregate-profile",
            "aggregate-metadata",
            "active-aggregate",
        ] {
            let fallback = safe_relay_switch_failure_settings(unsupported_route_settings(kind));

            assert_safe_route_settings(&fallback);
        }
    }

    fn session(
        id: &str,
        archived: bool,
        updated_at_ms: Option<i64>,
        provider: &str,
    ) -> codex_plus_data::LocalSession {
        codex_plus_data::LocalSession {
            id: id.to_string(),
            title: String::new(),
            cwd: String::new(),
            model_provider: provider.to_string(),
            archived,
            updated_at_ms,
            rollout_path: String::new(),
            db_path: String::new(),
        }
    }

    #[test]
    fn archive_candidates_require_old_active_timestamp() {
        let cutoff = 1_000_000;
        let sessions = vec![
            session("old", false, Some(cutoff - 1), "openai"),
            session("boundary", false, Some(cutoff), "openai"),
            session("missing", false, None, "openai"),
            session("archived", true, Some(cutoff - 10), "openai"),
        ];

        let (candidates, missing) = archive_candidates(&sessions, cutoff);

        assert_eq!(candidates, vec!["old"]);
        assert_eq!(missing, 1);
    }

    #[test]
    fn pagination_is_archive_filtered_and_cursor_stable() {
        let sessions = vec![
            session("a", false, Some(5), "openai"),
            session("b", false, Some(4), "openai"),
            session("c", false, Some(3), "openai"),
            session("z", true, Some(9), "openai"),
        ];

        let (first, next, cursor_valid) = paginate_local_sessions(sessions.clone(), false, None, 2);
        assert!(cursor_valid);
        assert_eq!(
            first
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        let (second, final_cursor, cursor_valid) =
            paginate_local_sessions(sessions, false, next.as_deref(), 2);
        assert!(cursor_valid);
        assert_eq!(
            second
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["c"]
        );
        assert!(final_cursor.is_none());
    }

    #[test]
    fn pagination_rejects_stale_cursor() {
        let sessions = vec![session("a", false, Some(5), "openai")];
        let (page, next, cursor_valid) =
            paginate_local_sessions(sessions, false, Some("missing-cursor"), 100);
        assert!(page.is_empty());
        assert!(next.is_none());
        assert!(!cursor_valid);
    }

    #[test]
    fn lifecycle_settings_require_consent_and_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("lifecycle.json");
        let settings = SessionLifecycleSettings {
            retention_days: 45,
            ..SessionLifecycleSettings::default()
        };
        write_session_lifecycle_settings_to(&path, &settings).unwrap();
        assert_eq!(
            read_session_lifecycle_settings_from(&path)
                .unwrap()
                .retention_days,
            45
        );

        let invalid = SessionLifecycleSettings {
            archive_enabled: true,
            first_run_reviewed: false,
            ..SessionLifecycleSettings::default()
        };
        assert!(write_session_lifecycle_settings_to(&path, &invalid).is_err());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn provider_compatibility_ignores_archived_sessions() {
        let sessions = vec![
            session("matching", false, Some(3), "OpenAI"),
            session("mismatch", false, Some(2), "custom"),
            session("missing", false, Some(1), ""),
            session("archived", true, Some(4), "legacy"),
        ];

        let result = provider_compatibility_from_sessions("OpenAI".to_string(), &sessions);

        assert_eq!(result.active_count, 3);
        assert_eq!(result.mismatch_count, 2);
        assert_eq!(result.missing_provider_count, 1);
        assert_eq!(result.archived_rollouts_traversed, 0);
        assert!(result.adaptation_available);

        let matching = vec![session("matching", false, Some(3), "OpenAI")];
        let clean = provider_compatibility_from_sessions("OpenAI".to_string(), &matching);
        assert!(!clean.adaptation_available);
    }

    #[test]
    fn lifecycle_settings_predating_the_adapt_toggle_load_with_it_on() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("session-lifecycle.json");
        std::fs::write(
            &path,
            r#"{"archiveEnabled":true,"firstRunReviewed":true,"retentionDays":20,"lastCompletedAtMs":null}"#,
        )
        .unwrap();
        let settings = read_session_lifecycle_settings_from(&path).unwrap();
        assert!(settings.auto_adapt_provider_on_switch);
        assert!(settings.archive_enabled);
        assert_eq!(settings.retention_days, 20);
    }

    #[test]
    fn forced_manual_check_bypasses_the_daily_interval_but_not_the_policy() {
        let recently_checked = SessionLifecycleSettings {
            archive_enabled: true,
            first_run_reviewed: true,
            retention_days: 30,
            last_completed_at_ms: Some(1_000),
            auto_adapt_provider_on_switch: true,
        };
        assert!(!archive_maintenance_due(&recently_checked, false, 2_000));
        assert!(archive_maintenance_due(&recently_checked, true, 2_000));

        let disabled = SessionLifecycleSettings {
            archive_enabled: false,
            ..recently_checked
        };
        assert!(!archive_maintenance_due(&disabled, true, 2_000));
    }

    #[test]
    fn provider_identity_preserves_case_and_defaults_to_openai() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(current_effective_provider_from_home(temp.path()), "openai");
        std::fs::write(
            temp.path().join("config.toml"),
            "model_provider = \"OpenAI\"\n",
        )
        .unwrap();
        assert_eq!(current_effective_provider_from_home(temp.path()), "OpenAI");
    }

    #[test]
    fn native_operation_accepts_uuid_only() {
        assert!(is_uuid("123e4567-e89b-12d3-a456-426614174000"));
        assert!(!is_uuid("session-name"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let temp = tempfile::tempdir().unwrap();
            let cli = temp.path().join("codex");
            std::fs::write(&cli, "#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(
                run_native_session_operation(
                    &cli,
                    temp.path(),
                    "archive",
                    "123e4567-e89b-12d3-a456-426614174000",
                )
                .is_ok()
            );
            assert!(
                run_native_session_operation(&cli, temp.path(), "archive", "session-name",)
                    .is_err()
            );
        }
    }

    #[test]
    fn macos_cli_path_is_target_app_resource() {
        let app = Path::new("/Applications/ChatGPT.app");
        let expected = if cfg!(target_os = "macos") {
            app.join("Contents/Resources/codex")
        } else if cfg!(windows) {
            app.join("codex.exe")
        } else {
            app.join("codex")
        };
        assert_eq!(codex_cli_from_app_dir(app), expected);
        #[cfg(target_os = "macos")]
        assert_eq!(
            codex_cli_from_app_dir(&app.join("Contents/MacOS")),
            app.join("Contents/Resources/codex")
        );
    }

    #[test]
    fn windows_cli_path_accepts_versioned_standalone_bin_layout() {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("bin");
        let versioned = bin.join("a61afac3bb4ee395");
        std::fs::create_dir_all(&versioned).unwrap();
        std::fs::write(versioned.join("codex.exe"), b"test cli").unwrap();

        assert_eq!(
            windows_codex_cli_from_app_dir(&bin),
            versioned.join("codex.exe")
        );
    }

    #[test]
    fn windows_standalone_cli_is_found_under_versioned_local_appdata_bin() {
        let temp = tempfile::tempdir().unwrap();
        let versioned = temp
            .path()
            .join("OpenAI")
            .join("Codex")
            .join("bin")
            .join("a61afac3bb4ee395");
        std::fs::create_dir_all(&versioned).unwrap();
        std::fs::write(versioned.join("codex.exe"), b"test cli").unwrap();

        assert_eq!(
            windows_standalone_codex_cli_in(temp.path()),
            Some(versioned.join("codex.exe"))
        );
    }

    #[test]
    fn windows_standalone_cli_is_absent_without_a_codex_binary() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("OpenAI").join("Codex").join("bin")).unwrap();

        assert_eq!(windows_standalone_codex_cli_in(temp.path()), None);
    }
}

#[cfg(test)]
mod context_guard_tests {
    use super::*;

    const LIVE_CONFIG: &str = r#"model_provider = "OpenAI"
model = "gpt-5.6-sol"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://example.test/"

[mcp_servers.memory]
enabled = true
type = "stdio"
command = "/Users/x/.local/bin/memory"
args = ["server", "--storage-backend", "sqlite_vec"]

[mcp_servers.memory.env]
HOME = "/Users/x"
MCP_EMBEDDING_MODEL = "/Users/x/Models/Qwen/Qwen3-Embedding-0.6B"

[mcp_servers.filesystem]
command = "/opt/homebrew/bin/mcp-server-filesystem"
"#;

    #[test]
    fn restore_recovers_clobbered_context_tables() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("config.toml"), LIVE_CONFIG).unwrap();
        let snapshot = snapshot_context_tables(home.path()).unwrap();

        // 模拟上游切换流程吞掉 memory 的 transport 字段并整表重排
        let clobbered = r#"model_provider = "Other"
model = "gpt-5.5"

[model_providers.Other]
name = "Other"
base_url = "https://other.test/"

[mcp_servers.memory]
enabled = true
"#;
        std::fs::write(home.path().join("config.toml"), clobbered).unwrap();
        restore_context_tables(home.path(), &snapshot).unwrap();

        let restored = std::fs::read_to_string(home.path().join("config.toml")).unwrap();
        // 供应商字段保持切换后的新值
        assert!(restored.contains(r#"model_provider = "Other""#));
        assert!(restored.contains("[model_providers.Other]"));
        // 三张受保护表恢复原样
        assert!(restored.contains(r#"command = "/Users/x/.local/bin/memory""#));
        assert!(restored.contains(r#"args = ["server", "--storage-backend", "sqlite_vec"]"#));
        assert!(restored.contains("MCP_EMBEDDING_MODEL"));
        assert!(restored.contains("[mcp_servers.filesystem]"));
    }

    #[test]
    fn restore_removes_tables_injected_from_managed_copy() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("config.toml"), "model = \"gpt-5.6-sol\"\n").unwrap();
        let snapshot = snapshot_context_tables(home.path()).unwrap();

        // 切换流程从 managed 副本注入了本不存在的 mcp_servers
        let injected = "model = \"gpt-5.5\"\n\n[mcp_servers.ghost]\nenabled = true\n";
        std::fs::write(home.path().join("config.toml"), injected).unwrap();
        restore_context_tables(home.path(), &snapshot).unwrap();

        let restored = std::fs::read_to_string(home.path().join("config.toml")).unwrap();
        assert!(restored.contains(r#"model = "gpt-5.5""#));
        assert!(!restored.contains("mcp_servers"));
    }

    #[test]
    fn restore_is_noop_when_tables_untouched() {
        let home = tempfile::tempdir().unwrap();
        let config_path = home.path().join("config.toml");
        std::fs::write(&config_path, LIVE_CONFIG).unwrap();
        let snapshot = snapshot_context_tables(home.path()).unwrap();
        let mtime_before = std::fs::metadata(&config_path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        restore_context_tables(home.path(), &snapshot).unwrap();
        let mtime_after = std::fs::metadata(&config_path).unwrap().modified().unwrap();
        assert_eq!(mtime_before, mtime_after, "未被改动时不应重写 config.toml");
    }

    #[test]
    fn scrub_clears_legacy_global_copies_and_selections() {
        let mut settings = BackendSettings::default();
        settings.relay_common_config_contents = "[agents]\nmax_threads = 1000\n".to_string();
        settings.relay_context_config_contents =
            "[mcp_servers.memory]\nenabled = true\n".to_string();
        let mut profile = RelayProfile::default();
        profile.context_selection.mcp_servers = vec!["memory".to_string()];
        profile.context_selection_initialized = true;
        settings.relay_profiles.push(profile);

        assert!(scrub_legacy_managed_config_state(&mut settings));
        assert!(settings.relay_common_config_contents.is_empty());
        assert!(settings.relay_context_config_contents.is_empty());
        assert!(!settings.relay_profiles[0].context_selection_initialized);
        assert!(
            settings.relay_profiles[0]
                .context_selection
                .mcp_servers
                .is_empty()
        );
        // 二次执行应为 no-op
        assert!(!scrub_legacy_managed_config_state(&mut settings));
    }

    #[test]
    fn scrub_removes_global_profile_config_without_a_common_copy() {
        let mut settings = BackendSettings::default();
        settings.relay_profiles = vec![RelayProfile {
            id: "api".to_string(),
            config_contents: r#"model_provider = "custom"

[model_providers.custom]
base_url = "https://example.test/v1"

[agents]
max_threads = 1000
"#
            .to_string(),
            ..RelayProfile::default()
        }];

        assert!(scrub_legacy_managed_config_state(&mut settings));
        let config = &settings.relay_profiles[0].config_contents;
        assert!(config.contains("[model_providers.custom]"));
        assert!(!config.contains("[agents]"));
        assert!(!config.contains("max_threads"));
        assert!(!scrub_legacy_managed_config_state(&mut settings));
    }

    #[test]
    fn normalization_keeps_only_provider_owned_profile_config() {
        let mut settings = BackendSettings::default();
        settings.relay_profiles = vec![RelayProfile {
            id: "mixed".to_string(),
            relay_mode: codex_plus_core::settings::RelayMode::Official,
            official_mix_api_key: true,
            config_contents: r#"model = "gpt-5.5"
model_provider = "custom"
model_catalog_json = "model-catalogs/mixed.json"
approval_policy = "never"

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://example.test/v1"
experimental_bearer_token = "sk-test"

[agents]
max_threads = 1000
"#
            .to_string(),
            ..RelayProfile::default()
        }];

        let normalized = normalize_settings_before_save(settings);
        let config = &normalized.relay_profiles[0].config_contents;
        assert!(config.contains(r#"model = "gpt-5.5""#));
        assert!(config.contains(r#"model_provider = "custom""#));
        assert!(config.contains(r#"model_catalog_json = "model-catalogs/mixed.json""#));
        assert!(config.contains("[model_providers.custom]"));
        assert!(!config.contains("approval_policy"));
        assert!(!config.contains("[agents]"));
        assert!(!config.contains("max_threads"));
    }

    #[test]
    fn raw_auth_save_is_rejected() {
        assert!(validate_relay_file_save_kind("auth").is_err());
        assert!(validate_relay_file_save_kind("config").is_ok());
    }

    #[test]
    fn normalization_projects_api_key_only_legacy_copy_to_provider_config() {
        let mut settings = BackendSettings::default();
        settings.relay_profiles = vec![RelayProfile {
            id: "pure".to_string(),
            relay_mode: codex_plus_core::settings::RelayMode::PureApi,
            base_url: "https://example.test/v1".to_string(),
            upstream_base_url: "https://example.test/v1".to_string(),
            auth_contents: r#"{"OPENAI_API_KEY":"sk-test"}"#.to_string(),
            ..RelayProfile::default()
        }];
        let settings = normalize_settings_before_save(settings);
        let profile = &settings.relay_profiles[0];
        assert!(profile.auth_contents.is_empty());
        assert_eq!(
            provider_bearer_token_from_config(&profile.config_contents).as_deref(),
            Some("sk-test")
        );
        let doc: toml_edit::DocumentMut = profile.config_contents.parse().unwrap();
        let provider = doc["model_provider"].as_str().unwrap();
        assert_eq!(
            doc["model_providers"][provider]["requires_openai_auth"].as_bool(),
            Some(false)
        );
        let persisted =
            String::from_utf8(serialize_settings_without_profile_auth(&settings).unwrap()).unwrap();
        assert!(!persisted.contains("authContents"));
    }

    fn write_legacy_auth_fixture(
        profile: RelayProfile,
    ) -> (tempfile::TempDir, std::path::PathBuf, Vec<u8>) {
        let temp = tempfile::tempdir().unwrap();
        let settings_path = temp.path().join("settings.json");
        let structured_api_key = profile.api_key.clone();
        let settings = BackendSettings {
            relay_profiles: vec![profile],
            ..BackendSettings::default()
        };
        let mut serialized = serde_json::to_value(&settings).unwrap();
        if !structured_api_key.trim().is_empty() {
            serialized["relayProfiles"][0]["apiKey"] = json!(structured_api_key);
        }
        let before = serde_json::to_vec_pretty(&serialized).unwrap();
        std::fs::write(&settings_path, &before).unwrap();
        (temp, settings_path, before)
    }

    fn error_chain_messages(error: &(dyn std::error::Error + 'static)) -> Vec<String> {
        let mut messages = Vec::new();
        let mut current = Some(error);
        while let Some(error) = current {
            messages.push(error.to_string());
            current = error.source();
        }
        messages
    }

    fn eva_legacy_settings_bytes() -> Vec<u8> {
        let mut settings = BackendSettings::default();
        settings.relay_profiles = vec![RelayProfile {
            id: "eva".to_string(),
            name: "Eva|Codex".to_string(),
            model: "gpt-5.6-terra".to_string(),
            relay_mode: codex_plus_core::settings::RelayMode::Official,
            official_mix_api_key: true,
            protocol: codex_plus_core::settings::RelayProtocol::Responses,
            base_url: "https://example.test/v1".to_string(),
            upstream_base_url: "https://example.test/v1".to_string(),
            config_contents: r#"model = "gpt-5.6-terra"
model_provider = "OpenAI"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://example.test/v1"
wire_api = "responses"
requires_openai_auth = true
"#
            .to_string(),
            auth_contents: r#"{
                "OPENAI_API_KEY": "provider-key-sentinel",
                "auth_mode": "chatgpt",
                "tokens": {"access_token": "oauth-access-sentinel"}
            }"#
            .to_string(),
            ..RelayProfile::default()
        }];

        serde_json::to_vec_pretty(&settings).unwrap()
    }

    #[test]
    fn oauth_only_residue_uses_an_existing_provider_bearer() {
        let mut profile = RelayProfile {
            id: "mixed".to_string(),
            relay_mode: codex_plus_core::settings::RelayMode::Official,
            official_mix_api_key: true,
            config_contents: set_provider_config_bearer("", "existing-key", Some(true)).unwrap(),
            auth_contents: r#"{"auth_mode":"chatgpt","tokens":{"access_token":"oauth-sentinel"}}"#
                .to_string(),
            ..RelayProfile::default()
        };

        migrate_persisted_legacy_api_key_auth(&mut profile).unwrap();

        assert_eq!(profile.api_key, "existing-key");
        assert!(profile.auth_contents.is_empty());
    }

    #[test]
    fn pure_oauth_discards_the_complete_legacy_copy_without_adopting_its_key() {
        let mut profile = RelayProfile {
            id: "official".to_string(),
            relay_mode: codex_plus_core::settings::RelayMode::Official,
            official_mix_api_key: false,
            auth_contents: r#"{"OPENAI_API_KEY":"orphan-key","auth_mode":"chatgpt"}"#.to_string(),
            ..RelayProfile::default()
        };

        migrate_persisted_legacy_api_key_auth(&mut profile).unwrap();

        assert!(profile.api_key.is_empty());
        assert!(provider_bearer_token_from_config_exact(&profile.config_contents).is_none());
        assert!(profile.auth_contents.is_empty());
    }

    #[test]
    fn provider_key_mode_ignores_empty_or_non_string_legacy_keys_when_structured_key_exists() {
        for auth_contents in [
            r#"{"OPENAI_API_KEY":"","auth_mode":"chatgpt"}"#,
            r#"{"OPENAI_API_KEY":null,"auth_mode":"chatgpt"}"#,
        ] {
            let mut profile = RelayProfile {
                id: "mixed".to_string(),
                relay_mode: codex_plus_core::settings::RelayMode::Official,
                official_mix_api_key: true,
                api_key: "existing-key".to_string(),
                auth_contents: auth_contents.to_string(),
                ..RelayProfile::default()
            };

            migrate_persisted_legacy_api_key_auth(&mut profile).unwrap();

            assert_eq!(profile.api_key, "existing-key");
            assert!(profile.auth_contents.is_empty());
            assert_eq!(
                provider_bearer_token_from_config_exact(&profile.config_contents).as_deref(),
                Some("existing-key")
            );
        }
    }

    #[test]
    fn legacy_profile_auth_missing_key_identifies_the_profile_without_writing() {
        let profile = RelayProfile {
            id: "eva".to_string(),
            name: "Eva|Codex".to_string(),
            relay_mode: codex_plus_core::settings::RelayMode::Official,
            official_mix_api_key: true,
            auth_contents:
                r#"{"auth_mode":"chatgpt","tokens":{"access_token":"oauth-access-sentinel"}}"#
                    .to_string(),
            ..RelayProfile::default()
        };
        let (_temp, settings_path, before) = write_legacy_auth_fixture(profile);
        let _guard = live_state::lock().unwrap();

        let error = migrate_legacy_profile_auth_locked_at(&settings_path).unwrap_err();

        assert!(error.to_string().contains("Eva|Codex"));
        assert!(!error.to_string().contains("oauth-access-sentinel"));
        assert_eq!(std::fs::read(&settings_path).unwrap(), before);
    }

    #[test]
    fn legacy_profile_auth_rejects_disagreeing_existing_destinations_without_writing() {
        for (api_key, config_contents, auth_contents) in [
            (
                "structured-key",
                String::new(),
                r#"{"OPENAI_API_KEY":"legacy-key","tokens":{"access_token":"oauth-access-sentinel"}}"#,
            ),
            (
                "",
                set_provider_config_bearer("", "bearer-key", Some(true)).unwrap(),
                r#"{"OPENAI_API_KEY":"legacy-key","tokens":{"access_token":"oauth-access-sentinel"}}"#,
            ),
            (
                "legacy-key",
                set_provider_config_bearer("", "bearer-key", Some(true)).unwrap(),
                r#"{"OPENAI_API_KEY":"legacy-key","tokens":{"access_token":"oauth-access-sentinel"}}"#,
            ),
        ] {
            let profile = RelayProfile {
                id: "mixed".to_string(),
                name: "Eva|Codex".to_string(),
                relay_mode: codex_plus_core::settings::RelayMode::Official,
                official_mix_api_key: true,
                api_key: api_key.to_string(),
                config_contents,
                auth_contents: auth_contents.to_string(),
                ..RelayProfile::default()
            };
            let (_temp, settings_path, before) = write_legacy_auth_fixture(profile);
            let _guard = live_state::lock().unwrap();

            let error = migrate_legacy_profile_auth_locked_at(&settings_path).unwrap_err();
            let messages = error_chain_messages(&error);

            assert!(
                messages[0].contains("persisted provider key conflict"),
                "top-level error lacks the conflict category: {messages:?}"
            );
            for message in messages {
                for sentinel in [
                    "legacy-key",
                    "structured-key",
                    "bearer-key",
                    "oauth-access-sentinel",
                ] {
                    assert!(
                        !message.contains(sentinel),
                        "conflict error source exposed {sentinel}: {message}"
                    );
                }
            }
            assert_eq!(std::fs::read(&settings_path).unwrap(), before);
        }
    }

    #[test]
    fn malformed_provider_toml_is_opaque_through_every_startup_error_source() {
        let malformed_config = r#"model_provider = "OpenAI"

[model_providers.OpenAI]
experimental_bearer_token = "provider-key-config-sentinel" secret_header = "secret-header-config-sentinel"
"#;
        let profiles = [
            RelayProfile {
                id: "mixed".to_string(),
                name: "Eva|Projection".to_string(),
                relay_mode: codex_plus_core::settings::RelayMode::Official,
                official_mix_api_key: true,
                config_contents: malformed_config.to_string(),
                auth_contents: r#"{"OPENAI_API_KEY":"legacy-provider-key-sentinel"}"#.to_string(),
                ..RelayProfile::default()
            },
            RelayProfile {
                id: "official".to_string(),
                name: "Eva|Ownership".to_string(),
                relay_mode: codex_plus_core::settings::RelayMode::Official,
                official_mix_api_key: false,
                config_contents: malformed_config.to_string(),
                auth_contents: r#"{"auth_mode":"chatgpt"}"#.to_string(),
                ..RelayProfile::default()
            },
        ];
        let _guard = live_state::lock().unwrap();

        for profile in profiles {
            let expected_label = profile.name.clone();
            let (_temp, settings_path, before) = write_legacy_auth_fixture(profile);

            let error = migrate_legacy_profile_auth_locked_at(&settings_path).unwrap_err();
            let messages = error_chain_messages(&error);

            assert!(
                messages[0].contains(&expected_label),
                "top-level error lacks safe profile context: {messages:?}"
            );
            assert!(
                messages[0].contains("persisted provider config is invalid"),
                "top-level error lacks the opaque config category: {messages:?}"
            );
            assert!(
                messages
                    .iter()
                    .any(|message| message == "persisted provider config is invalid"),
                "source chain lacks the typed config category: {messages:?}"
            );
            for message in messages {
                for sentinel in [
                    "provider-key-config-sentinel",
                    "secret-header-config-sentinel",
                    "legacy-provider-key-sentinel",
                ] {
                    assert!(
                        !message.contains(sentinel),
                        "startup error source exposed {sentinel}: {message}"
                    );
                }
            }
            assert_eq!(std::fs::read(&settings_path).unwrap(), before);
        }
    }

    #[test]
    fn legacy_profile_auth_rejects_invalid_or_non_object_copies_without_writing() {
        for auth_contents in ["{invalid-json", "[\"not-an-object\"]"] {
            let profile = RelayProfile {
                id: "mixed".to_string(),
                name: "Eva|Codex".to_string(),
                relay_mode: codex_plus_core::settings::RelayMode::Official,
                official_mix_api_key: true,
                auth_contents: auth_contents.to_string(),
                ..RelayProfile::default()
            };
            let (_temp, settings_path, before) = write_legacy_auth_fixture(profile);
            let _guard = live_state::lock().unwrap();

            let error = migrate_legacy_profile_auth_locked_at(&settings_path).unwrap_err();

            assert!(
                error
                    .to_string()
                    .contains("persisted provider auth copy is invalid")
            );
            assert!(!error.to_string().contains("oauth-access-sentinel"));
            assert_eq!(std::fs::read(&settings_path).unwrap(), before);
        }
    }

    #[test]
    fn load_time_legacy_migration_repairs_api_key_plus_oauth_residue() {
        let temp = tempfile::tempdir().unwrap();
        let settings_path = temp.path().join("settings.json");
        std::fs::write(&settings_path, eva_legacy_settings_bytes()).unwrap();
        let _guard = live_state::lock().unwrap();

        assert_eq!(
            migrate_legacy_profile_auth_locked_at(&settings_path).unwrap(),
            1
        );

        let bytes = std::fs::read(&settings_path).unwrap();
        let raw = String::from_utf8(bytes.clone()).unwrap();
        let migrated: BackendSettings = serde_json::from_slice(&bytes).unwrap();
        let profile = &migrated.relay_profiles[0];
        let provider_key = provider_bearer_token_from_config_exact(&profile.config_contents);
        assert_eq!(provider_key.as_deref(), Some("provider-key-sentinel"));
        assert!(!raw.contains("authContents"));
        assert!(!raw.contains("oauth-access-sentinel"));
        assert!(!profile.config_contents.contains("oauth-access-sentinel"));
    }

    #[test]
    fn provider_commit_load_accepts_eva_residue_without_mutating_its_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let settings_path = temp.path().join("settings.json");
        let original = eva_legacy_settings_bytes();
        std::fs::write(&settings_path, &original).unwrap();

        let (snapshot, loaded) = load_provider_commit_settings(&settings_path).unwrap();

        assert_eq!(snapshot, original);
        assert_eq!(std::fs::read(&settings_path).unwrap(), original);
        let profile = &loaded.relay_profiles[0];
        assert!(profile.auth_contents.is_empty());
        assert_eq!(profile.api_key, "provider-key-sentinel");
        assert_eq!(
            provider_bearer_token_from_config_exact(&profile.config_contents).as_deref(),
            Some("provider-key-sentinel")
        );
    }

    #[test]
    fn pre_snapshot_auth_migration_maps_each_typed_failure_to_a_static_commit_reason() {
        let cases = [
            (
                LegacyProfileAuthMigrationError::SettingsUnreadable(anyhow::anyhow!(
                    "read error sentinel"
                )),
                ProviderCommitErrorCode::InputUnavailable,
                "provider settings file is unreadable",
            ),
            (
                LegacyProfileAuthMigrationError::SettingsInvalidJson(anyhow::anyhow!(
                    "JSON error sentinel"
                )),
                ProviderCommitErrorCode::InputUnavailable,
                "provider settings are invalid JSON",
            ),
            (
                LegacyProfileAuthMigrationError::ProfileReconciliation(anyhow::anyhow!(
                    "profile error sentinel"
                )),
                ProviderCommitErrorCode::InputUnavailable,
                "a saved provider profile failed auth migration",
            ),
            (
                LegacyProfileAuthMigrationError::SecureStorage(anyhow::anyhow!(
                    "storage error sentinel"
                )),
                ProviderCommitErrorCode::TransactionFailed,
                "provider transaction failed",
            ),
        ];

        for (error, expected_code, expected_reason) in cases {
            let failure = provider_commit_failure_for_legacy_auth_migration(error);

            assert_eq!(failure.code(), expected_code);
            assert_eq!(failure.reason(), expected_reason);
            assert!(!failure.to_string().contains("sentinel"));
        }
    }

    #[test]
    fn pre_snapshot_auth_migration_preserves_each_inner_source_without_leaking_it_to_commit_ipc() {
        let cases = [
            (
                LegacyProfileAuthMigrationError::SettingsUnreadable(anyhow::anyhow!(
                    "unreadable source sentinel"
                )),
                "provider settings file is unreadable",
            ),
            (
                LegacyProfileAuthMigrationError::SettingsInvalidJson(anyhow::anyhow!(
                    "invalid JSON source sentinel"
                )),
                "provider settings are invalid JSON",
            ),
            (
                LegacyProfileAuthMigrationError::ProfileReconciliation(anyhow::anyhow!(
                    "reconciliation source sentinel"
                )),
                "a saved provider profile failed auth migration",
            ),
            (
                LegacyProfileAuthMigrationError::SecureStorage(anyhow::anyhow!(
                    "storage source sentinel"
                )),
                "provider transaction failed",
            ),
        ];

        for (error, expected_reason) in cases {
            let error_as_dyn: &dyn std::error::Error = &error;
            let source = error_as_dyn
                .source()
                .expect("typed error must retain its source");
            assert!(source.to_string().contains("source sentinel"));

            let failure = provider_commit_failure_for_legacy_auth_migration(error);
            assert_eq!(failure.reason(), expected_reason);
            assert!(!failure.to_string().contains("source sentinel"));
        }
    }

    #[test]
    fn load_time_legacy_migration_preserves_pure_api_contract_field_semantics() {
        let temp = tempfile::tempdir().unwrap();
        let settings_path = temp.path().join("settings.json");
        let mut settings = BackendSettings::default();
        let config = r#"model = "gpt-5.6-terra"
model_provider = "PureAPI"

[model_providers.PureAPI]
name = "OpenAI"
base_url = "https://example.test/v1"
wire_api = "responses"
requires_openai_auth = true
custom_field = "preserve-me"

[model_providers.PureAPI.http_headers]
x-openai-actor-authorization = "pure-header"
"#;

        settings.relay_profiles = vec![RelayProfile {
            id: "pure".to_string(),
            model: "gpt-5.6-terra".to_string(),
            relay_mode: codex_plus_core::settings::RelayMode::PureApi,
            base_url: "https://example.test/v1".to_string(),
            upstream_base_url: "https://example.test/v1".to_string(),
            config_contents: config.to_string(),
            auth_contents: r#"{
                "OPENAI_API_KEY": "provider-key-sentinel",
                "auth_mode": "chatgpt",
                "tokens": {"access_token": "oauth-access-sentinel"}
            }"#
            .to_string(),
            ..RelayProfile::default()
        }];

        std::fs::write(
            &settings_path,
            serde_json::to_vec_pretty(&settings).unwrap(),
        )
        .unwrap();
        let _guard = live_state::lock().unwrap();

        assert_eq!(
            migrate_legacy_profile_auth_locked_at(&settings_path).unwrap(),
            1
        );

        let bytes = std::fs::read(&settings_path).unwrap();
        let raw = String::from_utf8(bytes).unwrap();
        let migrated: BackendSettings = serde_json::from_slice(raw.as_bytes()).unwrap();
        let profile = &migrated.relay_profiles[0];
        let document: toml_edit::DocumentMut = profile.config_contents.parse().unwrap();
        let provider = document["model_providers"]["PureAPI"].clone();

        assert_eq!(
            provider_bearer_token_from_config_exact(&profile.config_contents).as_deref(),
            Some("provider-key-sentinel")
        );
        assert_eq!(document["model_provider"].as_str(), Some("PureAPI"));
        assert_eq!(document["model"].as_str(), Some("gpt-5.6-terra"));
        assert_eq!(provider["name"].as_str(), Some("OpenAI"));
        assert_eq!(
            provider["base_url"].as_str(),
            Some("https://example.test/v1")
        );
        assert_eq!(provider["wire_api"].as_str(), Some("responses"));
        assert_eq!(provider["requires_openai_auth"].as_bool(), Some(true));
        assert_eq!(provider["custom_field"].as_str(), Some("preserve-me"));
        assert_eq!(
            provider["http_headers"]["x-openai-actor-authorization"].as_str(),
            Some("pure-header")
        );
        assert_eq!(
            provider["experimental_bearer_token"].as_str(),
            Some("provider-key-sentinel")
        );
        assert!(!raw.contains("authContents"));
        assert!(!raw.contains("oauth-access-sentinel"));
    }

    /// Golden: a legacy mixed contract, exactly as authored.
    ///
    /// `name = "custom"`, official auth still required, no actor header, no provider bearer
    /// marker. Every one of those is a field the upgrade transform writes, which is precisely
    /// why startup must not write them.
    const GOLDEN_UNTOUCHED_LEGACY_MIXED: &str = r#"model = "gpt-5.5"
model_provider = "custom"

[model_providers.custom]
name = "custom"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = true
experimental_bearer_token = "legacy-mixed-key"
"#;

    /// Golden: a legacy provider-ID alias, exactly as authored.
    ///
    /// The identifier requires an explicit rename that only the user can authorize; startup may
    /// not rename it, and may not "finish" the surrounding contract on its behalf.
    const GOLDEN_UNTOUCHED_LEGACY_ALIAS: &str = r#"model = "gpt-5.5"
model_provider = "CodexPlusPlus"

[model_providers.CodexPlusPlus]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "legacy-alias-key"
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
"#;

    /// Golden: a complete contract carrying unowned provider and header keys, exactly as authored.
    ///
    /// `custom_field` and `x-unrelated-header` belong to the user. A header-table form must also
    /// survive as a table instead of being folded into the inline form.
    const GOLDEN_UNTOUCHED_CUSTOM_HEADER: &str = r#"model = "gpt-5.5"
model_provider = "CustomProvider"

[model_providers.CustomProvider]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "custom-header-key"
custom_field = "preserve-me"

[model_providers.CustomProvider.http_headers]
"x-openai-actor-authorization" = "local-image-extension"
"x-unrelated-header" = "keep-me"
"#;

    #[test]
    fn golden_startup_and_inspection_never_rewrite_an_existing_contract() {
        let temp = tempfile::tempdir().unwrap();
        let settings_path = temp.path().join("settings.json");
        let catalog_state_path = temp.path().join("model-catalog-state.json");
        let home = temp.path().join("codex-home");
        std::fs::create_dir_all(&home).unwrap();

        let goldens = [
            (
                "legacy-mixed",
                GOLDEN_UNTOUCHED_LEGACY_MIXED,
                "legacy-mixed-key",
                crate::provider_native_capability::NativeCapabilityState::UpgradeAvailable,
            ),
            (
                "CodexPlusPlus-profile",
                GOLDEN_UNTOUCHED_LEGACY_ALIAS,
                "legacy-alias-key",
                crate::provider_native_capability::NativeCapabilityState::UpgradeAvailable,
            ),
            (
                "custom-header",
                GOLDEN_UNTOUCHED_CUSTOM_HEADER,
                "custom-header-key",
                crate::provider_native_capability::NativeCapabilityState::NativePriority,
            ),
        ];
        let settings = BackendSettings {
            relay_profiles: goldens
                .iter()
                .map(|(id, config, key, _)| RelayProfile {
                    id: (*id).to_string(),
                    name: (*id).to_string(),
                    model: "gpt-5.5".to_string(),
                    base_url: "https://relay.example/v1".to_string(),
                    upstream_base_url: "https://relay.example/v1".to_string(),
                    api_key: (*key).to_string(),
                    protocol: codex_plus_core::settings::RelayProtocol::Responses,
                    relay_mode: codex_plus_core::settings::RelayMode::Official,
                    official_mix_api_key: true,
                    config_contents: (*config).to_string(),
                    // A legacy API-key auth copy, so the startup migration engages its rewrite
                    // path instead of returning early and proving nothing.
                    auth_contents: json!({ "OPENAI_API_KEY": key }).to_string(),
                    ..RelayProfile::default()
                })
                .collect(),
            ..BackendSettings::default()
        };
        std::fs::write(
            &settings_path,
            serde_json::to_vec_pretty(&settings).unwrap(),
        )
        .unwrap();
        let _guard = live_state::lock().unwrap();

        assert_eq!(
            migrate_legacy_profile_auth_locked_at(&settings_path).unwrap(),
            goldens.len(),
            "every profile must take the rewriting migration path"
        );

        let migrated: BackendSettings =
            serde_json::from_slice(&std::fs::read(&settings_path).unwrap()).unwrap();
        for (index, (id, config, _, _)) in goldens.iter().enumerate() {
            let profile = &migrated.relay_profiles[index];
            assert_eq!(
                profile.id.as_str(),
                *id,
                "startup renamed or reordered a profile"
            );
            assert_eq!(
                profile.config_contents.as_str(),
                *config,
                "startup rewrote the {id} contract"
            );
            assert!(profile.auth_contents.is_empty());
        }

        // The catalog startup path reads the same settings and may derive a mode; it may not
        // write back into any profile contract.
        let after_migration = std::fs::read(&settings_path).unwrap();
        let state = crate::model_catalog::load_and_migrate_state_from_path(
            &migrated,
            &home,
            &catalog_state_path,
        )
        .unwrap();
        let modes =
            crate::model_catalog::read_only_catalog_modes_from_state(&migrated, Some(&state));
        assert_eq!(std::fs::read(&settings_path).unwrap(), after_migration);

        for (id, config, _, expected_state) in goldens {
            let profile = migrated
                .relay_profiles
                .iter()
                .find(|profile| profile.id == id)
                .unwrap();
            let inspection = crate::provider_native_capability::inspect_profile(profile, modes[id]);
            assert_eq!(inspection.state, expected_state, "{id}");
            assert_eq!(
                profile.config_contents.as_str(),
                config,
                "inspection rewrote the {id} contract"
            );
        }
        assert_eq!(std::fs::read(&settings_path).unwrap(), after_migration);
    }

    #[test]
    fn context_transaction_preserves_unrelated_root_settings() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("config.toml"),
            format!("{LIVE_CONFIG}\napproval_policy = \"never\"\n[profiles.work]\nmodel = \"x\"\n\n[agents]\nmax_concurrent_threads_per_session = 8\n"),
        )
        .unwrap();
        let candidate = r#"model_provider = "Other"
model = "other"

[model_providers.Other]
name = "Other"
base_url = "https://other.test"

[agents]
max_threads = 1000
"#;
        let (protected, snapshot) = context_protected_config(home.path(), candidate).unwrap();
        assert!(protected.contains("approval_policy = \"never\""));
        assert!(protected.contains("[profiles.work]"));
        assert!(protected.contains("[mcp_servers.memory]"));
        assert!(protected.contains("max_concurrent_threads_per_session = 8"));
        assert!(!protected.contains("max_threads = 1000"));
        live_state::atomic_write_owner_only(&home.path().join("config.toml"), protected.as_bytes())
            .unwrap();
        verify_context_tables(home.path(), &snapshot).unwrap();
    }

    #[test]
    fn invalid_live_config_fails_before_context_mutation() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("config.toml"), "[broken").unwrap();
        assert!(context_protected_config(home.path(), "model = \"safe\"\n").is_err());
        assert_eq!(
            std::fs::read_to_string(home.path().join("config.toml")).unwrap(),
            "[broken"
        );
    }

    #[test]
    fn pure_api_staging_uses_config_bearer_without_touching_live_auth() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("config.toml"), LIVE_CONFIG).unwrap();
        std::fs::write(home.path().join("auth.json"), "official-auth").unwrap();
        let auth_before = std::fs::read(home.path().join("auth.json")).unwrap();
        let mut settings = BackendSettings::default();
        settings.relay_profiles_enabled = true;
        settings.active_relay_id = "pure".to_string();
        settings.relay_profiles = vec![RelayProfile {
            id: "pure".to_string(),
            name: "Pure".to_string(),
            relay_mode: codex_plus_core::settings::RelayMode::PureApi,
            base_url: "https://example.test/v1".to_string(),
            upstream_base_url: "https://example.test/v1".to_string(),
            api_key: "sk-stage".to_string(),
            config_contents: set_provider_config_bearer("", "sk-stage", Some(false)).unwrap(),
            ..RelayProfile::default()
        }];
        let staged = stage_active_relay_config(home.path(), &settings).unwrap();
        assert_eq!(
            provider_bearer_token_from_config(&staged).as_deref(),
            Some("sk-stage")
        );
        let doc: toml_edit::DocumentMut = staged.parse().unwrap();
        let provider = doc["model_provider"].as_str().unwrap();
        assert_eq!(
            doc["model_providers"][provider]["requires_openai_auth"].as_bool(),
            Some(false)
        );
        assert_eq!(
            std::fs::read(home.path().join("auth.json")).unwrap(),
            auth_before
        );
    }
}

#[cfg(test)]
mod provider_test_compatibility_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::JoinHandle;

    fn spawn_provider_test_server(
        responses: Vec<(u16, String)>,
    ) -> (String, JoinHandle<Vec<Value>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            responses
                .into_iter()
                .map(|(status, response_body)| {
                    let (mut stream, _) = listener.accept().unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 4096];
                    let (header_end, content_length) = loop {
                        let read = stream.read(&mut buffer).unwrap();
                        assert!(read > 0, "request closed before headers completed");
                        request.extend_from_slice(&buffer[..read]);
                        if let Some(header_end) =
                            request.windows(4).position(|part| part == b"\r\n\r\n")
                        {
                            let headers = String::from_utf8_lossy(&request[..header_end]);
                            let content_length = headers
                                .lines()
                                .find_map(|line| {
                                    let (name, value) = line.split_once(':')?;
                                    name.eq_ignore_ascii_case("content-length")
                                        .then(|| value.trim().parse::<usize>().unwrap())
                                })
                                .unwrap_or_default();
                            break (header_end + 4, content_length);
                        }
                    };
                    while request.len() < header_end + content_length {
                        let read = stream.read(&mut buffer).unwrap();
                        assert!(read > 0, "request closed before body completed");
                        request.extend_from_slice(&buffer[..read]);
                    }
                    let body = if content_length == 0 {
                        Value::Null
                    } else {
                        serde_json::from_slice(
                            &request[header_end..header_end + content_length],
                        )
                        .unwrap()
                    };
                    let reason = if status < 400 { "OK" } else { "Bad Request" };
                    write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                        response_body.len(),
                    )
                    .unwrap();
                    body
                })
                .collect()
        });
        (format!("http://{address}/v1"), handle)
    }

    fn provider_test_profile(base_url: String, api_key: &str) -> RelayProfile {
        RelayProfile {
            base_url,
            api_key: api_key.to_string(),
            relay_mode: codex_plus_core::settings::RelayMode::PureApi,
            protocol: codex_plus_core::settings::RelayProtocol::Responses,
            ..RelayProfile::default()
        }
    }

    #[test]
    fn manager_provider_probe_retries_without_max_output_tokens() {
        let api_key = "sk-manager-fallback-secret";
        let (base_url, server) = spawn_provider_test_server(vec![
            (
                400,
                r#"{"error":{"message":"Upstream request failed","type":"upstream_error"}}"#
                    .to_string(),
            ),
            (200, format!(r#"{{"id":"response-ok","echo":"{api_key}"}}"#)),
        ]);
        let profile = provider_test_profile(base_url, api_key);

        let result = tauri::async_runtime::block_on(test_relay_profile_with_compatibility(
            &profile, "gpt-test",
        ))
        .unwrap();
        let bodies = server.join().unwrap();

        assert_eq!(bodies.len(), 2);
        assert_eq!(bodies[0]["max_output_tokens"], 16);
        assert!(bodies[1].get("max_output_tokens").is_none());
        assert_eq!(result.http_status, 200);
        assert!(result.compatibility_fallback_used);
        assert_eq!(result.initial_http_status, Some(400));
        assert!(!result.response_preview.contains(api_key));
        assert!(result.response_preview.contains("[REDACTED]"));
    }

    #[test]
    fn manager_provider_probe_does_not_retry_non_allowlisted_errors() {
        let (base_url, server) = spawn_provider_test_server(vec![(
            401,
            r#"{"error":{"message":"Unknown parameter: max_output_tokens","type":"invalid_request_error"}}"#
                .to_string(),
        )]);
        let profile = provider_test_profile(base_url, "sk-manager-no-retry");

        let result = tauri::async_runtime::block_on(test_relay_profile_with_compatibility(
            &profile, "gpt-test",
        ))
        .unwrap();
        let bodies = server.join().unwrap();

        assert_eq!(bodies.len(), 1);
        assert_eq!(result.http_status, 401);
        assert!(!result.compatibility_fallback_used);
        assert_eq!(result.initial_http_status, None);
    }

    #[test]
    fn manager_provider_probe_fallback_is_strictly_allowlisted() {
        assert!(responses_output_limit_fallback_allowed(
            codex_plus_core::settings::RelayProtocol::Responses,
            400,
            r#"{"error":{"message":"Unknown parameter: max_output_tokens","type":"invalid_request_error"}}"#,
        ));
        assert!(responses_output_limit_fallback_allowed(
            codex_plus_core::settings::RelayProtocol::Responses,
            400,
            r#"{"error":{"message":"Upstream request failed","type":"upstream_error"}}"#,
        ));
        assert!(!responses_output_limit_fallback_allowed(
            codex_plus_core::settings::RelayProtocol::Responses,
            400,
            r#"{"error":{"message":"model not found","type":"invalid_request_error"}}"#,
        ));
        assert!(!responses_output_limit_fallback_allowed(
            codex_plus_core::settings::RelayProtocol::ChatCompletions,
            400,
            r#"{"error":{"message":"Unknown parameter: max_output_tokens"}}"#,
        ));
    }

    #[test]
    fn quick_provider_test_surfaces_compatibility_fallback() {
        let (base_url, server) = spawn_provider_test_server(vec![
            (
                400,
                r#"{"error":{"message":"Upstream request failed","type":"upstream_error"}}"#
                    .to_string(),
            ),
            (
                200,
                r#"{"id":"response-ok","status":"completed"}"#.to_string(),
            ),
        ]);
        let mut profile = provider_test_profile(base_url, "sk-quick-fallback");
        profile.name = "Quick Test".to_string();
        profile.test_model = "gpt-test".to_string();

        let result = tauri::async_runtime::block_on(super::test_relay_profile(profile));
        assert_eq!(result.status, "ok");
        assert_eq!(result.payload.http_status, 200);
        assert!(result.payload.compatibility_fallback_used);
        assert_eq!(result.payload.initial_http_status, Some(400));
        assert!(result.message.contains("兼容重试"));
        let bodies = server.join().unwrap();
        assert_eq!(bodies.len(), 2);
    }

    #[test]
    fn provider_doctor_surfaces_compatibility_fallback() {
        let (base_url, server) = spawn_provider_test_server(vec![
            (200, r#"{"data":[{"id":"gpt-test"}]}"#.to_string()),
            (
                400,
                r#"{"error":{"message":"Upstream request failed","type":"upstream_error"}}"#
                    .to_string(),
            ),
            (
                200,
                r#"{"id":"response-ok","status":"completed"}"#.to_string(),
            ),
        ]);
        let mut profile = provider_test_profile(base_url, "sk-doctor-fallback");
        profile.name = "Doctor Test".to_string();
        profile.test_model = "gpt-test".to_string();

        let result = tauri::async_runtime::block_on(super::diagnose_relay_profile(profile));
        assert_eq!(result.status, "ok");
        assert!(result.payload.compatibility_fallback_used);
        assert_eq!(result.payload.initial_http_status, Some(400));
        assert_eq!(result.payload.request_http_status, Some(200));
        let bodies = server.join().unwrap();
        let request_check = result
            .payload
            .checks
            .iter()
            .find(|check| check.id == "request")
            .unwrap();

        assert_eq!(bodies.len(), 3);
        assert_eq!(bodies[1]["max_output_tokens"], 16);
        assert!(bodies[2].get("max_output_tokens").is_none());
        assert_eq!(request_check.status, "ok");
        assert!(request_check.detail.contains("兼容重试"));
    }

    #[test]
    fn provider_doctor_output_redacts_provider_oauth_identity_and_endpoint_sentinels() {
        let api_key = "sk-provider-doctor-secret";
        let oauth_token = "oauth-provider-doctor-token";
        let account_email = "provider-doctor@example.test";
        let (base_url, server) = spawn_provider_test_server(vec![
            (200, r#"{"data":[{"id":"gpt-test"}]}"#.to_string()),
            (
                200,
                format!(
                    r#"{{"apiKey":"{api_key}","accessToken":"{oauth_token}","account":"{account_email}"}}"#
                ),
            ),
        ]);
        let mut profile = provider_test_profile(base_url.clone(), api_key);
        profile.name = "Doctor Secret Audit".to_string();
        profile.test_model = "gpt-test".to_string();
        profile.auth_contents = format!(
            r#"{{"auth_mode":"chatgpt","tokens":{{"access_token":"{oauth_token}","account_email":"{account_email}"}}}}"#
        );

        let result = tauri::async_runtime::block_on(super::diagnose_relay_profile(profile));
        let serialized = serde_json::to_string(&result).unwrap();
        server.join().unwrap();

        assert!(!serialized.contains(api_key));
        assert!(!serialized.contains(oauth_token));
        assert!(!serialized.contains(account_email));
        assert!(!serialized.contains(&base_url));
    }

    #[test]
    fn diagnostic_and_auth_status_surfaces_discard_dynamic_secret_and_identity_strings() {
        let detail = json!({
            "apiKey": "sk-diagnostic-secret",
            "nested": {
                "accessToken": "oauth-diagnostic-token",
                "account": "diagnostic@example.test",
                "baseUrl": "https://private.example.test/v1"
            },
            "attempt": 2,
            "retry": true
        });
        let sanitized = sanitize_diagnostic_detail(detail);
        let serialized = serde_json::to_string(&sanitized).unwrap();

        assert!(!serialized.contains("sk-diagnostic-secret"));
        assert!(!serialized.contains("oauth-diagnostic-token"));
        assert!(!serialized.contains("diagnostic@example.test"));
        assert!(!serialized.contains("https://private.example.test/v1"));
        assert_eq!(sanitized["attempt"], 2);
        assert_eq!(sanitized["retry"], true);
        assert_eq!(
            sanitize_ui_manager_event("sk-secret-event"),
            "manager.ui.event"
        );
        assert_eq!(
            redacted_account_label(true, Some("diagnostic-account@example.test".to_string())),
            Some("ChatGPT account".to_string())
        );

        let visible_config = redact_live_config_for_output(
            r#"model = "gpt-test"
model_provider = "Relay"

[model_providers.Relay]
name = "OpenAI"
base_url = "https://private.example.test/v1"
experimental_bearer_token = "sk-config-output-secret"
http_headers = { Authorization = "Bearer oauth-config-output-token", "x-keep" = "yes" }
"#,
        );
        assert!(visible_config.contains("model = \"gpt-test\""));
        assert!(visible_config.contains("x-keep"));
        assert!(!visible_config.contains("https://private.example.test/v1"));
        assert!(!visible_config.contains("sk-config-output-secret"));
        assert!(!visible_config.contains("oauth-config-output-token"));

        let escaped_config = redact_live_config_for_output(
            r#"model_provider = "Relay"
[model_providers.Relay]
base_url = "https:\u002f\u002fescaped.example.test\u002fv1"
experimental_bearer_token = "sk\u002descaped-secret"
http_headers = { "x-openai-api-key" = "sk-header-secret", Cookie = "oauth-cookie-secret", "x-openai-actor-authorization" = "local-image-extension" }
token = "oauth-generic-token"
bearer_token = "sk-generic-bearer"
client_secret = "generic-client-secret"
password = "generic-provider-password"
"#,
        );
        assert!(!escaped_config.contains("escaped.example.test"));
        assert!(!escaped_config.contains("sk\\u002descaped-secret"));
        assert!(!escaped_config.contains("sk-header-secret"));
        assert!(!escaped_config.contains("oauth-cookie-secret"));
        assert!(!escaped_config.contains("oauth-generic-token"));
        assert!(!escaped_config.contains("sk-generic-bearer"));
        assert!(!escaped_config.contains("generic-client-secret"));
        assert!(!escaped_config.contains("generic-provider-password"));
        assert!(escaped_config.contains("local-image-extension"));

        let unknown_key = sanitize_diagnostic_detail(json!({
            "sk-secret-in-key": true,
            "status": "ok",
            "targetRelayId": "provider-a"
        }));
        let unknown_key_text = serde_json::to_string(&unknown_key).unwrap();
        assert!(!unknown_key_text.contains("sk-secret-in-key"));
        assert!(unknown_key_text.contains("ok"));
        assert!(!unknown_key_text.contains("provider-a"));
        for key in [
            "status",
            "command",
            "targetRelayMode",
            "launchMode",
            "version",
        ] {
            let injected = sanitize_diagnostic_detail_for_event(
                "manager.ui.switchRelayProfile.ok",
                Value::Object(
                    [(
                        key.to_string(),
                        Value::String("sk-safe-field-secret".to_string()),
                    )]
                    .into_iter()
                    .collect(),
                ),
            );
            assert!(
                !serde_json::to_string(&injected)
                    .unwrap()
                    .contains("sk-safe-field-secret")
            );
        }
        let panic_detail = sanitize_diagnostic_detail(json!({
            "payload": "sk-panic-secret",
            "location": { "file": "/private/oauth-panic-token.rs", "line": 42 }
        }));
        assert!(
            !serde_json::to_string(&panic_detail)
                .unwrap()
                .contains("sk-panic-secret")
        );
        assert!(!include_str!("lib.rs").contains("diagnostic_log::append_diagnostic_log"));

        let profile = provider_test_profile(
            "https://private.example.test/v1".to_string(),
            "sk-boundary-secret",
        );
        let leaked_prefix = "sk-boundary-se";
        let sanitized_probe = sanitize_provider_test_result(
            &profile,
            CommandResult {
                status: "failed".to_string(),
                message: format!("HTTP 401: {leaked_prefix}"),
                payload: RelayProfileTestPayload {
                    http_status: 401,
                    endpoint: "https://private.example.test/v1/responses".to_string(),
                    response_preview: leaked_prefix.to_string(),
                    compatibility_fallback_used: false,
                    initial_http_status: None,
                },
            },
        );
        let sanitized_probe_text = serde_json::to_string(&sanitized_probe).unwrap();
        assert!(!sanitized_probe_text.contains(leaked_prefix));

        let safe_models = sanitize_provider_model_ids(
            &profile,
            vec![
                "gpt-safe".to_string(),
                "sk-boundary-secret".to_string(),
                "oauth-model-token@example.test".to_string(),
                "https://private.example.test/v1".to_string(),
            ],
        );
        assert_eq!(safe_models, vec!["gpt-safe"]);
        assert_eq!(
            sanitize_provider_model_ids(&profile, vec!["v1".to_string()]),
            vec!["v1"]
        );
    }
}

#[cfg(test)]
mod command_settlement_tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, PartialEq, Eq)]
    struct Payload {
        detail: String,
    }

    #[test]
    fn a_panicked_blocking_command_answers_the_caller_instead_of_dropping_the_reply() {
        let result = tauri::async_runtime::block_on(async {
            let task = tauri::async_runtime::spawn_blocking(|| -> CommandResult<Payload> {
                panic!("the blocking body panicked");
            });
            settle_blocking(task, "提交中断。", || Payload {
                detail: "interrupted".to_string(),
            })
            .await
        });

        assert_eq!(result.status, "failed");
        assert_eq!(result.message, "提交中断。");
        assert_eq!(result.payload.detail, "interrupted");
    }

    #[test]
    fn a_completed_blocking_command_keeps_its_own_result() {
        let result = tauri::async_runtime::block_on(async {
            let task = tauri::async_runtime::spawn_blocking(|| {
                ok(
                    "done",
                    Payload {
                        detail: "committed".to_string(),
                    },
                )
            });
            settle_blocking(task, "提交中断。", || Payload {
                detail: "interrupted".to_string(),
            })
            .await
        });

        assert_eq!(result.status, "ok");
        assert_eq!(result.payload.detail, "committed");
    }
}
