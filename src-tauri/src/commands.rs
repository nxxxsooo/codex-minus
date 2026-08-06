use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
}

impl Default for SessionLifecycleSettings {
    fn default() -> Self {
        Self {
            archive_enabled: false,
            first_run_reviewed: false,
            retention_days: 30,
            last_completed_at_ms: None,
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
    tauri::async_runtime::spawn_blocking(|| load_settings_blocking())
        .await
        .expect("blocking command panicked")
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
    tauri::async_runtime::spawn_blocking(move || save_settings_blocking(settings))
        .await
        .expect("blocking command panicked")
}

fn save_settings_blocking(settings: BackendSettings) -> CommandResult<SettingsPayload> {
    let settings = normalize_settings_before_save(settings);
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let result = (|| -> anyhow::Result<()> {
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        let bytes = serialize_settings_without_profile_auth(&settings)?;
        live_state::commit_locked(&[FileMutation::bytes(
            codex_plus_core::paths::default_settings_path(),
            bytes,
        )])
    })();
    match result {
        Ok(()) => settings_payload("设置已保存。", "设置保存后重新读取失败"),
        Err(error) => failed(
            &format!("保存设置失败：{error}"),
            SettingsPayload {
                settings,
                settings_path: codex_plus_core::paths::default_settings_path()
                    .to_string_lossy()
                    .to_string(),
                user_scripts: user_script_inventory(),
            },
        ),
    }
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
        return app_dir.join("codex.exe");
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

pub(crate) fn discover_target_codex_cli() -> anyhow::Result<PathBuf> {
    let settings = SettingsStore::default().load().unwrap_or_default();
    let saved = settings.codex_app_path.trim();
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
    let Ok(mut child) = Command::new(cli)
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
    let mut child = Command::new(cli)
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
            match Command::new("/usr/bin/pgrep")
                .args(["-x", name])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
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
pub async fn run_session_archive_maintenance() -> CommandResult<ArchiveMaintenancePayload> {
    tauri::async_runtime::spawn_blocking(run_session_archive_maintenance_blocking)
        .await
        .expect("blocking command panicked")
}

fn run_session_archive_maintenance_blocking() -> CommandResult<ArchiveMaintenancePayload> {
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
    let due = settings.archive_enabled
        && settings.first_run_reviewed
        && settings
            .last_completed_at_ms
            .is_none_or(|last| current_time_ms.saturating_sub(last) >= ARCHIVE_CHECK_INTERVAL_MS);
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
    scrub_managed_context_state(&mut settings);
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
                set_provider_config_bearer(&profile.config_contents, &api_key, false)
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
    let doc: toml_edit::DocumentMut = config_contents.parse().ok()?;
    let provider_id = doc.get("model_provider")?.as_str()?.trim();
    doc.get("model_providers")?
        .as_table()?
        .get(provider_id)?
        .as_table()?
        .get("experimental_bearer_token")?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn set_provider_config_bearer(
    config_contents: &str,
    api_key: &str,
    requires_openai_auth: bool,
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
    doc["model_providers"][provider_id.as_str()]["requires_openai_auth"] =
        toml_edit::value(requires_openai_auth);
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
        Err(error) => failed(
            &format!("读取配置文件失败：{error}"),
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
        Err(error) => failed(
            &format!("保存配置文件失败：{error}"),
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
        "auth.json 由官方客户端管理，Codex-- 不接受认证文件写入"
    );
    Ok(())
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayProfileSwitchRequest {
    pub settings: BackendSettings,
    #[serde(default)]
    pub previous_active_relay_id: String,
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
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let previous_provider = current_effective_provider_from_home(&home);
    let previous_active_relay_id = request.previous_active_relay_id;
    let target_relay_id = request.settings.active_relay_id.clone();
    log_manager_event(
        "manager.switch_relay_profile.start",
        json!({
            "previousActiveRelayId": previous_active_relay_id,
            "targetRelayId": target_relay_id
        }),
    );
    match commit_relay_profile_transaction(request.settings, &previous_active_relay_id) {
        Ok(settings) => {
            let status = codex_plus_core::relay_config::relay_status_from_home(&home);
            let current_provider = current_effective_provider_from_home(&home);
            let provider_changed = previous_provider != current_provider;
            log_manager_event(
                "manager.switch_relay_profile.ok",
                json!({
                    "targetRelayId": settings.active_relay_id,
                    "configured": status.configured,
                    "previousProvider": previous_provider,
                    "currentProvider": current_provider,
                    "providerChanged": provider_changed,
                    "sessionScansScheduled": usize::from(provider_changed)
                }),
            );
            ok(
                "供应商已切换。",
                relay_switch_payload(settings, status, None, previous_provider, current_provider),
            )
        }
        Err(error) => {
            let status = codex_plus_core::relay_config::relay_status_from_home(&home);
            let current_provider = current_effective_provider_from_home(&home);
            let settings = SettingsStore::default()
                .load()
                .map(sanitize_settings_for_output)
                .unwrap_or_default();
            log_manager_event(
                "manager.switch_relay_profile.failed",
                json!({
                    "previousActiveRelayId": previous_active_relay_id,
                    "activeRelayId": settings.active_relay_id,
                    "error": error.to_string()
                }),
            );
            failed(
                &format!("供应商切换失败：{error}"),
                relay_switch_payload(settings, status, None, previous_provider, current_provider),
            )
        }
    }
}

fn commit_relay_profile_transaction(
    mut settings: BackendSettings,
    previous_active_relay_id: &str,
) -> anyhow::Result<BackendSettings> {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let _guard = live_state::lock()?;
    live_state::prepare_secret_paths(&home)?;
    live_state::recover_locked()?;
    migrate_legacy_profile_auth_locked()?;

    let auth_path = home.join("auth.json");
    let auth_before = read_optional_bytes(&auth_path)?;
    if !previous_active_relay_id.trim().is_empty()
        && previous_active_relay_id != settings.active_relay_id
    {
        backfill_profile_config_only(&home, &mut settings, previous_active_relay_id)?;
    }
    settings = normalize_settings_before_save(settings);
    anyhow::ensure!(
        settings.relay_profiles_enabled,
        "供应商配置总开关已关闭，未写入 live 配置"
    );

    let candidate = stage_active_relay_config(&home, &settings)?;
    let catalog_plan = crate::model_catalog::plan_active_profile(&home, &settings, &candidate)?;
    let (protected_config, context_snapshot) =
        context_protected_config(&home, &catalog_plan.config_contents)?;
    let settings_bytes = serialize_settings_without_profile_auth(&settings)?;
    let settings_path = codex_plus_core::paths::default_settings_path();
    let config_path = home.join("config.toml");
    let mut mutations = catalog_plan.mutations;
    mutations.push(FileMutation::bytes(settings_path, settings_bytes));
    mutations.push(FileMutation::text(config_path, protected_config));
    live_state::commit_locked_verified(&mutations, || {
        verify_context_tables(&home, &context_snapshot)?;
        anyhow::ensure!(
            read_optional_bytes(&auth_path)? == auth_before,
            "live auth changed concurrently; provider transaction was rolled back"
        );
        Ok(())
    })?;
    Ok(sanitize_settings_for_output(settings))
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

fn stage_active_relay_config(home: &Path, settings: &BackendSettings) -> anyhow::Result<String> {
    let profile = settings.active_relay_profile();
    anyhow::ensure!(
        profile.relay_mode != codex_plus_core::settings::RelayMode::Aggregate,
        "聚合供应商依赖已移除的本地代理，不能写入 live 配置"
    );
    with_private_staging_home("provider", |stage_home| {
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
                    set_provider_config_bearer(&projection.config_contents, &api_key, false)?;
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
            config = set_provider_config_bearer(&config, &api_key, false)?;
        }
        Ok(config)
    })
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
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = codex_plus_core::paths::default_app_state_dir().join("private-staging");
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
    let event = sanitize_manager_event(&event);
    match codex_plus_core::diagnostic_log::append_diagnostic_log(&event, detail) {
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

#[tauri::command]
pub async fn test_relay_profile(profile: RelayProfile) -> CommandResult<RelayProfileTestPayload> {
    let profile_name = if profile.name.trim().is_empty() {
        "未命名供应商"
    } else {
        profile.name.trim()
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
    match codex_plus_core::relay_config::test_relay_profile(&profile, &test_model).await {
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
                    "已向「{profile_name}」用模型「{test_model}」发送 hi，HTTP {}。{detail}",
                    result.http_status
                ),
                payload: RelayProfileTestPayload {
                    http_status: result.http_status,
                    endpoint: result.endpoint,
                    response_preview: result.response_preview,
                },
            }
        }
        Err(error) => failed(
            &format!("测试「{profile_name}」失败：{error}"),
            RelayProfileTestPayload {
                http_status: 0,
                endpoint: String::new(),
                response_preview: String::new(),
            },
        ),
    }
}

#[tauri::command]
pub async fn fetch_relay_profile_models(
    profile: RelayProfile,
) -> CommandResult<RelayProfileModelsPayload> {
    let profile_name = if profile.name.trim().is_empty() {
        "未命名供应商"
    } else {
        profile.name.trim()
    };
    match codex_plus_core::model_catalog::fetch_relay_profile_model_ids(&profile).await {
        Ok((models, endpoint)) => {
            if let Err(error) =
                crate::model_catalog::record_provider_evidence(&profile.id, &endpoint, &models)
            {
                return failed(
                    &format!("模型已获取，但供应商证据保存失败：{error}"),
                    RelayProfileModelsPayload { models, endpoint },
                );
            }
            ok(
                &format!("已从「{profile_name}」获取 {} 个模型。", models.len()),
                RelayProfileModelsPayload { models, endpoint },
            )
        }
        Err(error) => failed(
            &format!("从「{profile_name}」获取模型失败：{error}"),
            RelayProfileModelsPayload {
                models: Vec::new(),
                endpoint: String::new(),
            },
        ),
    }
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
        };
        return ok("Provider Doctor：官方登录供应商无需 API 诊断。", payload);
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
        };
        return failed("Provider Doctor：配置不完整。", payload);
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

    match codex_plus_core::model_catalog::fetch_relay_profile_model_ids(&profile).await {
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

    match codex_plus_core::relay_config::test_relay_profile(&profile, &test_model).await {
        Ok(result) => {
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
                        "{} 返回 HTTP {}，响应内容为空。",
                        result.endpoint, result.http_status
                    )
                } else {
                    format!(
                        "{} 返回 HTTP {}：{}",
                        result.endpoint, result.http_status, preview
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
    CommandResult {
        status: status.to_string(),
        message,
        payload: ProviderDoctorPayload {
            profile_name,
            model: test_model,
            summary,
            recommendation,
            checks,
        },
    }
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
    match commit_relay_profile_transaction(settings, &active_id) {
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
    let _ = codex_plus_core::diagnostic_log::append_diagnostic_log(event, detail);
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
    RelayPayload {
        authenticated: status.authenticated,
        auth_source: status.auth_source,
        account_label: status.account_label,
        config_path: status.config_path,
        configured: status.configured,
        requires_openai_auth: status.requires_openai_auth,
        has_bearer_token: status.has_bearer_token,
        backup_path,
    }
}

fn relay_switch_payload(
    settings: BackendSettings,
    status: codex_plus_core::relay_config::RelayStatus,
    backup_path: Option<String>,
    previous_provider: String,
    current_provider: String,
) -> RelaySwitchPayload {
    let provider_changed = previous_provider != current_provider;
    RelaySwitchPayload {
        settings,
        relay: relay_payload(status, backup_path),
        settings_path: codex_plus_core::paths::default_settings_path()
            .to_string_lossy()
            .to_string(),
        user_scripts: user_script_inventory(),
        previous_provider,
        current_provider,
        provider_changed,
    }
}

/// Codex-- 核心保证：供应商切换/注入永远不改动 config.toml 里不属于供应商的
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

/// 销毁 settings 存储中的 managed context 副本：残缺的 `[mcp_servers.*]` 拷贝
/// 曾经就存在这里，切换时会被回填、下次再被合并/过滤写回 config.toml。
fn scrub_managed_context_state(settings: &mut BackendSettings) -> bool {
    let mut dirty = false;
    if !settings.relay_context_config_contents.is_empty() {
        settings.relay_context_config_contents = String::new();
        dirty = true;
    }
    for profile in &mut settings.relay_profiles {
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

pub fn scrub_managed_context_store() {
    let home = codex_plus_core::relay_config::default_codex_home_dir();
    let result = (|| -> anyhow::Result<bool> {
        let _guard = live_state::lock()?;
        live_state::prepare_secret_paths(&home)?;
        live_state::recover_locked()?;
        migrate_legacy_profile_auth_locked()?;
        let mut settings = SettingsStore::default().load()?;
        let dirty = scrub_managed_context_state(&mut settings);
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
        Ok(true) => log_manager_event("manager.context_guard.store_scrubbed", json!({})),
        Ok(false) => {}
        Err(error) => log_manager_event(
            "manager.context_guard.store_scrub_failed",
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
        config_contents: read_optional_text_file(&config_path)?,
        auth_status: live_auth_status_payload(home),
    })
}

fn live_auth_status_payload(home: &Path) -> LiveAuthStatusPayload {
    let status = codex_plus_core::relay_config::chatgpt_auth_status_from_home(home);
    LiveAuthStatusPayload {
        authenticated: status.authenticated,
        source: status.source,
        account_label: status.account_label,
        action_required: (!status.authenticated)
            .then(|| "请在官方 Codex/ChatGPT 客户端中登录。".to_string()),
    }
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
        Ok(settings) => Ok(SettingsPayload {
            settings: sanitize_settings_for_output(settings),
            settings_path,
            user_scripts: user_script_inventory(),
        }),
        Err(error) => Err((
            error,
            SettingsPayload {
                settings: BackendSettings::default(),
                settings_path,
                user_scripts: user_script_inventory(),
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
    if !settings_path.exists() {
        return Ok(());
    }
    live_state::ensure_owner_only_file(&settings_path)?;
    let raw = std::fs::read_to_string(&settings_path)?;
    let contains_profile_auth = serde_json::from_str::<Value>(&raw)
        .ok()
        .and_then(|value| value.get("relayProfiles").cloned())
        .and_then(|value| value.as_array().cloned())
        .is_some_and(|profiles| {
            profiles.iter().any(|profile| {
                profile
                    .get("authContents")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
            })
        });
    if !contains_profile_auth {
        return Ok(());
    }

    let settings = SettingsStore::default().load()?;
    let settings = normalize_settings_before_save(settings);
    let bytes = serialize_settings_without_profile_auth(&settings)?;
    // Credential migration intentionally has no prior-file backup. The old file is
    // secured first and then atomically replaced so OAuth copies cannot survive in
    // a recovery artifact.
    live_state::atomic_write_owner_only(&settings_path, &bytes)?;
    log_manager_event(
        "manager.profile_auth_migration.completed",
        json!({ "profileCount": settings.relay_profiles.len() }),
    );
    Ok(())
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
        adaptation_available: false,
        adaptation_message: "当前上游 provider-sync 尚无 active-only 范围；为避免扫描或改写归档历史，适配写入已禁用。".to_string(),
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
        let payload = provider_compatibility_payload();
        let message = if scan_generation != payload.scan_generation {
            "兼容性检查结果已过期，请重新检查。"
        } else {
            "当前上游 provider-sync 不能限定为活动会话；Codex-- 已阻止全历史回退，升级上游接口后才会开放适配。"
        };
        CommandResult {
            status: "not_implemented".to_string(),
            message: message.to_string(),
            payload,
        }
    })
    .await
    .expect("blocking command panicked")
}

#[cfg(test)]
mod session_lifecycle_tests {
    use super::*;

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
        assert!(!result.adaptation_available);
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
    fn scrub_clears_managed_copy_and_selections() {
        let mut settings = BackendSettings::default();
        settings.relay_context_config_contents =
            "[mcp_servers.memory]\nenabled = true\n".to_string();
        let mut profile = RelayProfile::default();
        profile.context_selection.mcp_servers = vec!["memory".to_string()];
        profile.context_selection_initialized = true;
        settings.relay_profiles.push(profile);

        assert!(scrub_managed_context_state(&mut settings));
        assert!(settings.relay_context_config_contents.is_empty());
        assert!(!settings.relay_profiles[0].context_selection_initialized);
        assert!(
            settings.relay_profiles[0]
                .context_selection
                .mcp_servers
                .is_empty()
        );
        // 二次执行应为 no-op
        assert!(!scrub_managed_context_state(&mut settings));
    }

    #[test]
    fn raw_auth_save_is_rejected() {
        assert!(validate_relay_file_save_kind("auth").is_err());
        assert!(validate_relay_file_save_kind("config").is_ok());
    }

    #[test]
    fn normalization_removes_oauth_copies_and_projects_pure_api_key_to_config() {
        let mut settings = BackendSettings::default();
        settings.relay_profiles = vec![RelayProfile {
            id: "pure".to_string(),
            relay_mode: codex_plus_core::settings::RelayMode::PureApi,
            base_url: "https://example.test/v1".to_string(),
            upstream_base_url: "https://example.test/v1".to_string(),
            auth_contents: r#"{
                "OPENAI_API_KEY": "sk-test",
                "auth_mode": "chatgpt",
                "tokens": {"access_token": "oauth", "refresh_token": "refresh"}
            }"#
            .to_string(),
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
        assert!(!persisted.contains("oauth"));
        assert!(!persisted.contains("refresh"));
        assert!(!persisted.contains("authContents"));
    }

    #[test]
    fn context_transaction_preserves_unrelated_root_settings() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("config.toml"),
            format!("{LIVE_CONFIG}\napproval_policy = \"never\"\n[profiles.work]\nmodel = \"x\"\n"),
        )
        .unwrap();
        let candidate = r#"model_provider = "Other"
model = "other"

[model_providers.Other]
name = "Other"
base_url = "https://other.test"
"#;
        let (protected, snapshot) = context_protected_config(home.path(), candidate).unwrap();
        assert!(protected.contains("approval_policy = \"never\""));
        assert!(protected.contains("[profiles.work]"));
        assert!(protected.contains("[mcp_servers.memory]"));
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
            config_contents: set_provider_config_bearer("", "sk-stage", false).unwrap(),
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
