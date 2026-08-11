use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};

use crate::commands::CommandResult;
use crate::live_state;

const POLICY_VERSION: u32 = 1;
const POLICY_FILE: &str = "network-policy.json";
const TEST_ORIGIN: &str = "https://chatgpt.com/backend-api/codex/models";
const TEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PROXY_URL_BYTES: usize = 2048;
const MAX_BYPASS_ENTRIES: usize = 128;
const MAX_BYPASS_ENTRY_BYTES: usize = 255;

const PROXY_ENVIRONMENT_NAMES: &[&str] = &["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY"];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkPolicyMode {
    #[default]
    Auto,
    Direct,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
struct SavedNetworkPolicy {
    version: u32,
    mode: NetworkPolicyMode,
    custom_proxy_url: String,
    custom_no_proxy: Vec<String>,
}

impl Default for SavedNetworkPolicy {
    fn default() -> Self {
        Self {
            version: POLICY_VERSION,
            mode: NetworkPolicyMode::Auto,
            custom_proxy_url: String::new(),
            custom_no_proxy: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveNetworkPolicyRequest {
    mode: NetworkPolicyMode,
    #[serde(default)]
    custom_proxy_url: String,
    #[serde(default)]
    custom_no_proxy: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPolicyStatusPayload {
    pub mode: NetworkPolicyMode,
    pub custom_proxy_url: String,
    pub custom_no_proxy: String,
    pub source: String,
    pub endpoint: Option<String>,
    pub bypass_count: usize,
    pub supported: bool,
    pub action_required: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPolicyTestPayload {
    pub source: String,
    pub endpoint: Option<String>,
    pub bypass_count: usize,
    pub supported: bool,
    pub category: String,
    pub duration_ms: u128,
    pub action_required: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ProxyObservation {
    environment: BTreeMap<String, String>,
    unsupported_reason: Option<String>,
}

impl ProxyObservation {
    fn has_endpoint(&self) -> bool {
        ["HTTPS_PROXY", "ALL_PROXY", "HTTP_PROXY"]
            .iter()
            .any(|name| {
                self.environment
                    .get(*name)
                    .is_some_and(|value| !value.is_empty())
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedNetworkPolicy {
    pub mode: NetworkPolicyMode,
    pub source: String,
    pub environment: Vec<(OsString, OsString)>,
    pub endpoint: Option<String>,
    pub bypass_count: usize,
    pub supported: bool,
    pub action_required: Option<String>,
}

impl ResolvedNetworkPolicy {
    fn unsupported(mode: NetworkPolicyMode, source: &str, action: impl Into<String>) -> Self {
        Self {
            mode,
            source: source.to_string(),
            environment: Vec::new(),
            endpoint: None,
            bypass_count: 0,
            supported: false,
            action_required: Some(action.into()),
        }
    }

    fn direct(mode: NetworkPolicyMode, source: &str) -> Self {
        Self {
            mode,
            source: source.to_string(),
            environment: Vec::new(),
            endpoint: None,
            bypass_count: 0,
            supported: true,
            action_required: None,
        }
    }

    fn from_observation(mode: NetworkPolicyMode, source: &str, value: ProxyObservation) -> Self {
        if let Some(reason) = value.unsupported_reason {
            return Self::unsupported(mode, source, reason);
        }
        let endpoint = preferred_endpoint(&value.environment).map(redact_proxy_url);
        let bypass_count = value
            .environment
            .get("NO_PROXY")
            .map(|value| normalized_bypass_entries(value).len())
            .unwrap_or(0);
        Self {
            mode,
            source: source.to_string(),
            environment: value
                .environment
                .into_iter()
                .flat_map(|(name, value)| {
                    [
                        (OsString::from(&name), OsString::from(&value)),
                        (
                            OsString::from(name.to_ascii_lowercase()),
                            OsString::from(value),
                        ),
                    ]
                })
                .collect(),
            endpoint,
            bypass_count,
            supported: true,
            action_required: None,
        }
    }

    pub(crate) fn ensure_supported(&self) -> anyhow::Result<()> {
        ensure!(
            self.supported,
            "{}",
            self.action_required
                .as_deref()
                .unwrap_or("Manager 网络策略不可用")
        );
        Ok(())
    }
}

#[tauri::command]
pub async fn manager_network_policy_status() -> CommandResult<NetworkPolicyStatusPayload> {
    tauri::async_runtime::spawn_blocking(network_policy_status_blocking)
        .await
        .expect("blocking command panicked")
}

#[tauri::command]
pub async fn save_manager_network_policy(
    request: SaveNetworkPolicyRequest,
) -> CommandResult<NetworkPolicyStatusPayload> {
    tauri::async_runtime::spawn_blocking(move || save_network_policy_blocking(request))
        .await
        .expect("blocking command panicked")
}

#[tauri::command]
pub async fn test_manager_network_policy() -> CommandResult<NetworkPolicyTestPayload> {
    let resolved = match tauri::async_runtime::spawn_blocking(resolve_current_policy).await {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            return failed_test("unsupported-policy", error.to_string(), 0);
        }
        Err(error) => return failed_test("other", error.to_string(), 0),
    };
    if !resolved.supported {
        return CommandResult {
            status: "failed".to_string(),
            message: resolved
                .action_required
                .clone()
                .unwrap_or_else(|| "Manager 网络策略不可用。".to_string()),
            payload: test_payload_from_resolved(&resolved, "unsupported-policy", 0),
        };
    }

    let started = Instant::now();
    let result = test_resolved_policy(&resolved).await;
    let duration_ms = started.elapsed().as_millis();
    match result {
        Ok(status) if status.as_u16() == 407 => CommandResult {
            status: "failed".to_string(),
            message: "代理要求认证；v1 不读取或保存代理凭据。".to_string(),
            payload: test_payload_from_resolved(&resolved, "proxy-auth-unsupported", duration_ms),
        },
        Ok(_) => CommandResult {
            status: "ok".to_string(),
            message: "Manager 网络连接测试通过。".to_string(),
            payload: test_payload_from_resolved(&resolved, "ok", duration_ms),
        },
        Err(error) => {
            let category = classify_reqwest_error(&error);
            CommandResult {
                status: "failed".to_string(),
                message: test_failure_message(category).to_string(),
                payload: test_payload_from_resolved(&resolved, category, duration_ms),
            }
        }
    }
}

fn network_policy_status_blocking() -> CommandResult<NetworkPolicyStatusPayload> {
    match resolve_current_policy_with_saved() {
        Ok((saved, resolved)) => CommandResult {
            status: "ok".to_string(),
            message: if resolved.supported {
                "Manager 网络策略已解析。".to_string()
            } else {
                resolved
                    .action_required
                    .clone()
                    .unwrap_or_else(|| "Manager 网络策略需要处理。".to_string())
            },
            payload: status_payload(&saved, &resolved),
        },
        Err(error) => CommandResult {
            status: "failed".to_string(),
            message: format!("Manager 网络策略读取失败：{error}"),
            payload: NetworkPolicyStatusPayload::default(),
        },
    }
}

fn save_network_policy_blocking(
    request: SaveNetworkPolicyRequest,
) -> CommandResult<NetworkPolicyStatusPayload> {
    let result = (|| -> anyhow::Result<(SavedNetworkPolicy, ResolvedNetworkPolicy)> {
        let _guard = live_state::lock()?;
        let path = network_policy_path();
        let saved = saved_policy_from_request(request)?;
        save_policy_to(&path, &saved)?;
        let resolved = resolve_policy(&saved)?;
        Ok((saved, resolved))
    })();
    match result {
        Ok((saved, resolved)) => CommandResult {
            status: "ok".to_string(),
            message: if resolved.supported {
                "Manager 网络策略已保存。".to_string()
            } else {
                resolved
                    .action_required
                    .clone()
                    .unwrap_or_else(|| "Manager 网络策略已保存，但当前不可用。".to_string())
            },
            payload: status_payload(&saved, &resolved),
        },
        Err(error) => CommandResult {
            status: "failed".to_string(),
            message: format!("Manager 网络策略保存失败：{error}"),
            payload: NetworkPolicyStatusPayload::default(),
        },
    }
}

pub(crate) fn resolve_current_policy() -> anyhow::Result<ResolvedNetworkPolicy> {
    resolve_current_policy_with_saved().map(|(_, resolved)| resolved)
}

fn resolve_current_policy_with_saved() -> anyhow::Result<(SavedNetworkPolicy, ResolvedNetworkPolicy)>
{
    let _guard = live_state::lock()?;
    let path = network_policy_path();
    let saved = load_policy_from(&path)?;
    let resolved = resolve_policy(&saved)?;
    Ok((saved, resolved))
}

fn network_policy_path() -> PathBuf {
    codex_plus_core::paths::default_app_state_dir().join(POLICY_FILE)
}

fn load_policy_from(path: &Path) -> anyhow::Result<SavedNetworkPolicy> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SavedNetworkPolicy::default());
        }
        Err(error) => return Err(error.into()),
    };
    live_state::ensure_owner_only_file(path)?;
    let policy: SavedNetworkPolicy =
        serde_json::from_slice(&bytes).context("network policy JSON is invalid")?;
    ensure!(
        policy.version <= POLICY_VERSION,
        "network policy comes from a newer manager version"
    );
    validate_saved_policy(&policy)?;
    Ok(policy)
}

fn save_policy_to(path: &Path, policy: &SavedNetworkPolicy) -> anyhow::Result<()> {
    validate_saved_policy(policy)?;
    live_state::atomic_write_owner_only(path, &serde_json::to_vec_pretty(policy)?)
}

fn saved_policy_from_request(
    request: SaveNetworkPolicyRequest,
) -> anyhow::Result<SavedNetworkPolicy> {
    let custom_proxy_url = if request.custom_proxy_url.trim().is_empty() {
        String::new()
    } else {
        normalize_custom_proxy_url(&request.custom_proxy_url)?
    };
    let custom_no_proxy = normalized_bypass_entries(&request.custom_no_proxy);
    let policy = SavedNetworkPolicy {
        version: POLICY_VERSION,
        mode: request.mode,
        custom_proxy_url,
        custom_no_proxy,
    };
    validate_saved_policy(&policy)?;
    Ok(policy)
}

fn validate_saved_policy(policy: &SavedNetworkPolicy) -> anyhow::Result<()> {
    ensure!(policy.version > 0, "network policy version is invalid");
    ensure!(
        policy.custom_no_proxy.len() <= MAX_BYPASS_ENTRIES,
        "custom bypass list is too long"
    );
    for entry in &policy.custom_no_proxy {
        ensure!(!entry.trim().is_empty(), "custom bypass entry is empty");
        ensure!(
            entry.len() <= MAX_BYPASS_ENTRY_BYTES && !entry.chars().any(char::is_control),
            "custom bypass entry is invalid"
        );
    }
    match policy.mode {
        NetworkPolicyMode::Custom => {
            ensure!(
                !policy.custom_proxy_url.is_empty(),
                "custom proxy URL is required"
            );
            ensure!(
                normalize_custom_proxy_url(&policy.custom_proxy_url)? == policy.custom_proxy_url,
                "custom proxy URL is not normalized"
            );
        }
        _ => {
            if !policy.custom_proxy_url.is_empty() {
                normalize_custom_proxy_url(&policy.custom_proxy_url)?;
            }
        }
    }
    Ok(())
}

fn normalize_custom_proxy_url(value: &str) -> anyhow::Result<String> {
    let value = value.trim();
    ensure!(
        !value.is_empty() && value.len() <= MAX_PROXY_URL_BYTES,
        "custom proxy URL is invalid"
    );
    ensure!(
        !value.chars().any(char::is_control),
        "custom proxy URL contains control characters"
    );
    let url = reqwest::Url::parse(value).context("custom proxy URL is invalid")?;
    ensure!(
        matches!(url.scheme(), "http" | "https" | "socks5" | "socks5h"),
        "custom proxy scheme is unsupported"
    );
    ensure!(url.host_str().is_some(), "custom proxy URL has no host");
    ensure!(
        url.username().is_empty() && url.password().is_none(),
        "custom proxy credentials are not supported"
    );
    ensure!(
        (url.path().is_empty() || url.path() == "/")
            && url.query().is_none()
            && url.fragment().is_none(),
        "custom proxy URL must not contain a path, query, or fragment"
    );
    Ok(normalized_proxy_origin(&url))
}

fn normalized_proxy_origin(url: &reqwest::Url) -> String {
    let host = url.host_str().unwrap_or_default();
    let host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    match url.port() {
        Some(port) => format!("{}://{host}:{port}", url.scheme()),
        None => format!("{}://{host}", url.scheme()),
    }
}

fn normalized_bypass_entries(value: &str) -> Vec<String> {
    value
        .split([',', ';', '\n', '\r', '\t', ' '])
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_ascii_lowercase())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn resolve_policy(policy: &SavedNetworkPolicy) -> anyhow::Result<ResolvedNetworkPolicy> {
    validate_saved_policy(policy)?;
    match policy.mode {
        NetworkPolicyMode::Direct => Ok(ResolvedNetworkPolicy::direct(
            NetworkPolicyMode::Direct,
            "direct",
        )),
        NetworkPolicyMode::Custom => {
            let mut environment = BTreeMap::new();
            let endpoint = normalize_custom_proxy_url(&policy.custom_proxy_url)?;
            environment.insert("HTTP_PROXY".to_string(), endpoint.clone());
            environment.insert("HTTPS_PROXY".to_string(), endpoint.clone());
            environment.insert("ALL_PROXY".to_string(), endpoint);
            if !policy.custom_no_proxy.is_empty() {
                environment.insert("NO_PROXY".to_string(), policy.custom_no_proxy.join(","));
            }
            Ok(ResolvedNetworkPolicy::from_observation(
                NetworkPolicyMode::Custom,
                "custom",
                ProxyObservation {
                    environment,
                    unsupported_reason: None,
                },
            ))
        }
        NetworkPolicyMode::Auto => {
            let process = process_proxy_observation(std::env::vars_os());
            let system = discover_system_proxy();
            Ok(resolve_auto_observations(process, system))
        }
    }
}

fn resolve_auto_observations(
    process: ProxyObservation,
    system: ProxyObservation,
) -> ResolvedNetworkPolicy {
    if process.unsupported_reason.is_some() || process.has_endpoint() {
        return ResolvedNetworkPolicy::from_observation(
            NetworkPolicyMode::Auto,
            "process-environment",
            process,
        );
    }
    if system.unsupported_reason.is_some() || system.has_endpoint() {
        return ResolvedNetworkPolicy::from_observation(
            NetworkPolicyMode::Auto,
            system_source_name(),
            system,
        );
    }
    ResolvedNetworkPolicy::direct(NetworkPolicyMode::Auto, "direct-fallback")
}

fn process_proxy_observation(
    pairs: impl IntoIterator<Item = (OsString, OsString)>,
) -> ProxyObservation {
    let mut values = BTreeMap::<String, BTreeSet<String>>::new();
    for (name, value) in pairs {
        let name = name.to_string_lossy().to_ascii_uppercase();
        if !PROXY_ENVIRONMENT_NAMES.contains(&name.as_str()) {
            continue;
        }
        let value = value.to_string_lossy().trim().to_string();
        if !value.is_empty() {
            values.entry(name).or_default().insert(value);
        }
    }
    if let Some(name) = values
        .iter()
        .find_map(|(name, variants)| (variants.len() > 1).then_some(name))
    {
        return ProxyObservation {
            environment: BTreeMap::new(),
            unsupported_reason: Some(format!(
                "检测到互相冲突的 {name} 大小写环境变量；请统一后重启 Manager。"
            )),
        };
    }
    let mut environment = values
        .into_iter()
        .filter_map(|(name, variants)| variants.into_iter().next().map(|value| (name, value)))
        .collect::<BTreeMap<_, _>>();
    if let Some(name) = ["HTTPS_PROXY", "ALL_PROXY", "HTTP_PROXY"]
        .iter()
        .find(|name| {
            environment
                .get(**name)
                .is_some_and(|value| !runtime_proxy_url_is_valid(value))
        })
    {
        return ProxyObservation {
            environment: BTreeMap::new(),
            unsupported_reason: Some(format!(
                "{name} 不是受支持的代理 URL；请修正后重启 Manager。"
            )),
        };
    }
    if let Some(value) = environment.get_mut("NO_PROXY") {
        *value = normalized_bypass_entries(value).join(",");
    }
    ProxyObservation {
        environment,
        unsupported_reason: None,
    }
}

fn discover_system_proxy() -> ProxyObservation {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("/usr/sbin/scutil")
            .arg("--proxy")
            .output();
        return match output {
            Ok(output) if output.status.success() => {
                parse_macos_proxy_output(&String::from_utf8_lossy(&output.stdout))
            }
            Ok(_) => ProxyObservation {
                environment: BTreeMap::new(),
                unsupported_reason: Some(
                    "无法读取 macOS 系统代理；请选择直连或自定义代理。".to_string(),
                ),
            },
            Err(_) => ProxyObservation {
                environment: BTreeMap::new(),
                unsupported_reason: Some(
                    "无法启动 macOS 系统代理检查；请选择直连或自定义代理。".to_string(),
                ),
            },
        };
    }
    #[cfg(windows)]
    {
        return discover_windows_system_proxy();
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        ProxyObservation {
            environment: BTreeMap::new(),
            unsupported_reason: Some("当前平台不支持系统代理发现。".to_string()),
        }
    }
}

#[cfg(target_os = "macos")]
fn parse_macos_proxy_output(contents: &str) -> ProxyObservation {
    let mut scalar = BTreeMap::<String, String>::new();
    let mut exceptions = Vec::new();
    let mut in_exceptions = false;
    for raw in contents.lines() {
        let line = raw.trim();
        if line.starts_with("ExceptionsList") && line.contains("<array>") {
            in_exceptions = true;
            continue;
        }
        if in_exceptions {
            if line == "}" {
                in_exceptions = false;
                continue;
            }
            if let Some((_, value)) = line.split_once(':') {
                let value = value.trim();
                if !value.is_empty() {
                    exceptions.push(value.to_string());
                }
            }
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            scalar.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    system_observation_from_fields(&scalar, &exceptions)
}

#[cfg(windows)]
fn discover_windows_system_proxy() -> ProxyObservation {
    let output = crate::platform_command::background_command("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        ])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            parse_windows_proxy_output(&String::from_utf8_lossy(&output.stdout))
        }
        Ok(_) => ProxyObservation::default(),
        Err(_) => ProxyObservation {
            environment: BTreeMap::new(),
            unsupported_reason: Some(
                "无法读取 Windows 系统代理；请选择直连或自定义代理。".to_string(),
            ),
        },
    }
}

#[cfg(any(windows, test))]
fn parse_windows_proxy_output(contents: &str) -> ProxyObservation {
    let mut fields = BTreeMap::new();
    for line in contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() >= 3 && parts[1].starts_with("REG_") {
            fields.insert(parts[0].to_string(), parts[2..].join(" "));
        }
    }
    let enabled = fields
        .get("ProxyEnable")
        .is_some_and(|value| value == "0x1" || value == "1");
    let automatic = fields
        .get("AutoConfigURL")
        .is_some_and(|value| !value.trim().is_empty())
        || fields
            .get("AutoDetect")
            .is_some_and(|value| value == "0x1" || value == "1");
    if automatic {
        return ProxyObservation {
            environment: BTreeMap::new(),
            unsupported_reason: Some(
                "检测到 Windows PAC/WPAD 自动代理；v1 不能安全投影，请选择直连或自定义代理。"
                    .to_string(),
            ),
        };
    }
    if !enabled {
        return ProxyObservation::default();
    }
    let mut environment = parse_windows_proxy_server(
        fields
            .get("ProxyServer")
            .map(String::as_str)
            .unwrap_or_default(),
    );
    let bypass = fields
        .get("ProxyOverride")
        .map(|value| {
            value
                .replace("<local>", "localhost,127.0.0.1")
                .replace(';', ",")
        })
        .unwrap_or_default();
    let bypass = normalized_bypass_entries(&bypass);
    if !bypass.is_empty() {
        environment.insert("NO_PROXY".to_string(), bypass.join(","));
    }
    ProxyObservation {
        environment,
        unsupported_reason: None,
    }
}

#[cfg(any(windows, test))]
fn parse_windows_proxy_server(value: &str) -> BTreeMap<String, String> {
    let mut environment = BTreeMap::new();
    if !value.contains('=') {
        if !value.trim().is_empty() {
            let proxy = with_proxy_scheme(value.trim(), "http");
            environment.insert("HTTP_PROXY".to_string(), proxy.clone());
            environment.insert("HTTPS_PROXY".to_string(), proxy);
        }
        return environment;
    }
    for item in value.split(';') {
        let Some((kind, endpoint)) = item.split_once('=') else {
            continue;
        };
        let (name, scheme) = match kind.trim().to_ascii_lowercase().as_str() {
            "http" => ("HTTP_PROXY", "http"),
            "https" => ("HTTPS_PROXY", "http"),
            "socks" => ("ALL_PROXY", "socks5h"),
            _ => continue,
        };
        if !endpoint.trim().is_empty() {
            environment.insert(name.to_string(), with_proxy_scheme(endpoint.trim(), scheme));
        }
    }
    environment
}

#[cfg(target_os = "macos")]
fn system_observation_from_fields(
    fields: &BTreeMap<String, String>,
    exceptions: &[String],
) -> ProxyObservation {
    let automatic = ["ProxyAutoConfigEnable", "ProxyAutoDiscoveryEnable"]
        .iter()
        .any(|key| fields.get(*key).is_some_and(|value| value == "1"));
    if automatic {
        return ProxyObservation {
            environment: BTreeMap::new(),
            unsupported_reason: Some(
                "检测到 macOS PAC/WPAD 自动代理；v1 不能安全投影，请选择直连或自定义代理。"
                    .to_string(),
            ),
        };
    }
    let mut environment = BTreeMap::new();
    for (enable, host, port, name, scheme) in [
        ("HTTPEnable", "HTTPProxy", "HTTPPort", "HTTP_PROXY", "http"),
        (
            "HTTPSEnable",
            "HTTPSProxy",
            "HTTPSPort",
            "HTTPS_PROXY",
            "http",
        ),
        (
            "SOCKSEnable",
            "SOCKSProxy",
            "SOCKSPort",
            "ALL_PROXY",
            "socks5h",
        ),
    ] {
        if fields.get(enable).is_some_and(|value| value == "1") {
            if let Some(host) = fields.get(host).filter(|value| !value.is_empty()) {
                let endpoint = match fields.get(port).filter(|value| !value.is_empty()) {
                    Some(port) => format!("{scheme}://{host}:{port}"),
                    None => format!("{scheme}://{host}"),
                };
                environment.insert(name.to_string(), endpoint);
            }
        }
    }
    let bypass = normalized_bypass_entries(&exceptions.join(","));
    if !bypass.is_empty() {
        environment.insert("NO_PROXY".to_string(), bypass.join(","));
    }
    ProxyObservation {
        environment,
        unsupported_reason: None,
    }
}

#[cfg(any(windows, test))]
fn with_proxy_scheme(value: &str, scheme: &str) -> String {
    if value.contains("://") {
        value.to_string()
    } else {
        format!("{scheme}://{value}")
    }
}

fn runtime_proxy_url_is_valid(value: &str) -> bool {
    reqwest::Url::parse(value).ok().is_some_and(|url| {
        matches!(url.scheme(), "http" | "https" | "socks5" | "socks5h") && url.host_str().is_some()
    })
}

fn system_source_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos-system"
    }
    #[cfg(windows)]
    {
        "windows-system"
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        "system"
    }
}

fn status_payload(
    saved: &SavedNetworkPolicy,
    resolved: &ResolvedNetworkPolicy,
) -> NetworkPolicyStatusPayload {
    NetworkPolicyStatusPayload {
        mode: saved.mode,
        custom_proxy_url: saved.custom_proxy_url.clone(),
        custom_no_proxy: saved.custom_no_proxy.join(","),
        source: resolved.source.clone(),
        endpoint: resolved.endpoint.clone(),
        bypass_count: resolved.bypass_count,
        supported: resolved.supported,
        action_required: resolved.action_required.clone(),
    }
}

fn preferred_endpoint(environment: &BTreeMap<String, String>) -> Option<&str> {
    ["HTTPS_PROXY", "ALL_PROXY", "HTTP_PROXY"]
        .iter()
        .find_map(|name| environment.get(*name).map(String::as_str))
}

fn redact_proxy_url(value: &str) -> String {
    reqwest::Url::parse(value)
        .ok()
        .map(|url| normalized_proxy_origin(&url))
        .unwrap_or_else(|| "configured-proxy".to_string())
}

async fn test_resolved_policy(
    resolved: &ResolvedNetworkPolicy,
) -> Result<reqwest::StatusCode, reqwest::Error> {
    let client = client_for_resolved_policy(resolved)?;
    client
        .get(TEST_ORIGIN)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map(|response| response.status())
}

fn client_for_resolved_policy(
    resolved: &ResolvedNetworkPolicy,
) -> Result<reqwest::Client, reqwest::Error> {
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(TEST_TIMEOUT)
        .user_agent(format!(
            "CodexMinus/{}/NetworkTest",
            env!("CARGO_PKG_VERSION")
        ));
    let environment = resolved
        .environment
        .iter()
        .map(|(name, value)| {
            (
                name.to_string_lossy().to_ascii_uppercase(),
                value.to_string_lossy().to_string(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let no_proxy = environment
        .get("NO_PROXY")
        .and_then(|value| reqwest::NoProxy::from_string(value));
    if let Some(value) = environment.get("HTTP_PROXY") {
        builder = builder.proxy(reqwest::Proxy::http(value)?.no_proxy(no_proxy.clone()));
    }
    if let Some(value) = environment.get("HTTPS_PROXY") {
        builder = builder.proxy(reqwest::Proxy::https(value)?.no_proxy(no_proxy.clone()));
    }
    if let Some(value) = environment.get("ALL_PROXY") {
        builder = builder.proxy(reqwest::Proxy::all(value)?.no_proxy(no_proxy));
    }
    builder.build()
}

fn classify_reqwest_error(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        return "timeout";
    }
    let text = error.to_string().to_ascii_lowercase();
    if text.contains("dns") || text.contains("resolve") {
        "dns"
    } else if text.contains("certificate") || text.contains("tls") {
        "tls"
    } else if error.is_connect() {
        "proxy-connect"
    } else {
        "other"
    }
}

fn test_failure_message(category: &str) -> &'static str {
    match category {
        "timeout" => "Manager 网络连接测试超时；请检查代理进程、节点与分流规则。",
        "dns" => "Manager 网络无法解析测试域名；请检查 DNS 或代理解析模式。",
        "tls" => "Manager 网络 TLS 验证失败；请检查代理证书与系统信任。",
        "proxy-connect" => "Manager 网络无法建立连接；请检查代理地址、进程与节点。",
        _ => "Manager 网络连接测试失败；请检查当前网络策略。",
    }
}

fn test_payload_from_resolved(
    resolved: &ResolvedNetworkPolicy,
    category: &str,
    duration_ms: u128,
) -> NetworkPolicyTestPayload {
    NetworkPolicyTestPayload {
        source: resolved.source.clone(),
        endpoint: resolved.endpoint.clone(),
        bypass_count: resolved.bypass_count,
        supported: resolved.supported,
        category: category.to_string(),
        duration_ms,
        action_required: resolved.action_required.clone(),
    }
}

fn failed_test(
    category: &str,
    message: String,
    duration_ms: u128,
) -> CommandResult<NetworkPolicyTestPayload> {
    CommandResult {
        status: "failed".to_string(),
        message,
        payload: NetworkPolicyTestPayload {
            category: category.to_string(),
            duration_ms,
            ..NetworkPolicyTestPayload::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(mode: NetworkPolicyMode, proxy: &str, no_proxy: &[&str]) -> SavedNetworkPolicy {
        SavedNetworkPolicy {
            version: POLICY_VERSION,
            mode,
            custom_proxy_url: proxy.to_string(),
            custom_no_proxy: no_proxy.iter().map(|value| value.to_string()).collect(),
        }
    }

    #[test]
    fn absent_policy_defaults_to_auto_without_writing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(POLICY_FILE);
        assert_eq!(
            load_policy_from(&path).unwrap(),
            SavedNetworkPolicy::default()
        );
        assert!(!path.exists());
    }

    #[test]
    fn policy_save_is_owner_only_and_invalid_save_preserves_previous() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(POLICY_FILE);
        let valid = saved(NetworkPolicyMode::Direct, "", &[]);
        save_policy_to(&path, &valid).unwrap();
        assert_eq!(load_policy_from(&path).unwrap(), valid);
        let invalid = saved(
            NetworkPolicyMode::Custom,
            "http://user:pass@proxy:7890",
            &[],
        );
        assert!(save_policy_to(&path, &invalid).is_err());
        assert_eq!(load_policy_from(&path).unwrap(), valid);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn custom_policy_rejects_credentials_and_normalizes_bypass() {
        assert!(normalize_custom_proxy_url("http://user:pass@localhost:7890").is_err());
        assert!(normalize_custom_proxy_url("file:///tmp/proxy").is_err());
        assert_eq!(
            normalize_custom_proxy_url(" socks5h://127.0.0.1:7890/ ").unwrap(),
            "socks5h://127.0.0.1:7890"
        );
        assert_eq!(
            normalized_bypass_entries("LOCALHOST; .Example.com,localhost"),
            vec![".example.com", "localhost"]
        );
    }

    #[test]
    fn process_proxy_is_coherent_and_conflicts_fail_closed() {
        let same = process_proxy_observation([
            (
                OsString::from("HTTPS_PROXY"),
                OsString::from("http://proxy:1"),
            ),
            (
                OsString::from("https_proxy"),
                OsString::from("http://proxy:1"),
            ),
            (OsString::from("OPENAI_API_KEY"), OsString::from("secret")),
        ]);
        assert_eq!(same.environment.len(), 1);
        assert!(same.unsupported_reason.is_none());

        let conflict = process_proxy_observation([
            (
                OsString::from("HTTPS_PROXY"),
                OsString::from("http://proxy:1"),
            ),
            (
                OsString::from("https_proxy"),
                OsString::from("http://proxy:2"),
            ),
        ]);
        assert!(conflict.unsupported_reason.is_some());
        assert!(conflict.environment.is_empty());

        let invalid = process_proxy_observation([(
            OsString::from("HTTPS_PROXY"),
            OsString::from("not a proxy URL"),
        )]);
        assert!(invalid.unsupported_reason.is_some());
        assert!(invalid.environment.is_empty());
    }

    #[test]
    fn auto_precedence_uses_one_source_then_direct_fallback() {
        let process = ProxyObservation {
            environment: BTreeMap::from([(
                "HTTPS_PROXY".to_string(),
                "http://process:7890".to_string(),
            )]),
            unsupported_reason: None,
        };
        let system = ProxyObservation {
            environment: BTreeMap::from([(
                "HTTPS_PROXY".to_string(),
                "http://system:7890".to_string(),
            )]),
            unsupported_reason: None,
        };
        let resolved = resolve_auto_observations(process.clone(), system.clone());
        assert_eq!(resolved.source, "process-environment");
        assert_eq!(resolved.endpoint.as_deref(), Some("http://process:7890"));

        let resolved = resolve_auto_observations(ProxyObservation::default(), system);
        assert_eq!(resolved.source, system_source_name());
        assert_eq!(resolved.endpoint.as_deref(), Some("http://system:7890"));

        let resolved =
            resolve_auto_observations(ProxyObservation::default(), ProxyObservation::default());
        assert_eq!(resolved.source, "direct-fallback");
        assert!(resolved.environment.is_empty());
    }

    #[test]
    fn unsupported_system_state_does_not_fall_back_to_direct() {
        let resolved = resolve_auto_observations(
            ProxyObservation::default(),
            ProxyObservation {
                environment: BTreeMap::new(),
                unsupported_reason: Some("PAC unsupported".to_string()),
            },
        );
        assert!(!resolved.supported);
        assert_eq!(resolved.source, system_source_name());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_static_proxy_fixture_maps_endpoints_bypass_and_pac() {
        let static_fixture = r#"<dictionary> {
  ExceptionsList : <array> {
    0 : localhost
    1 : *.local
  }
  HTTPEnable : 1
  HTTPPort : 7890
  HTTPProxy : 127.0.0.1
  HTTPSEnable : 1
  HTTPSPort : 7890
  HTTPSProxy : 127.0.0.1
  ProxyAutoConfigEnable : 0
}"#;
        let observation = parse_macos_proxy_output(static_fixture);
        assert_eq!(
            observation
                .environment
                .get("HTTPS_PROXY")
                .map(String::as_str),
            Some("http://127.0.0.1:7890")
        );
        assert_eq!(
            observation.environment.get("NO_PROXY").map(String::as_str),
            Some("*.local,localhost")
        );
        let pac = parse_macos_proxy_output(
            "<dictionary> {\n  ProxyAutoConfigEnable : 1\n  ProxyAutoConfigURLString : http://proxy/pac\n}",
        );
        assert!(pac.unsupported_reason.is_some());
    }

    #[test]
    fn windows_static_proxy_fixture_maps_endpoints_bypass_and_pac() {
        let static_fixture = r#"HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Internet Settings
    ProxyEnable    REG_DWORD    0x1
    ProxyServer    REG_SZ    http=127.0.0.1:7890;https=127.0.0.1:7890;socks=127.0.0.1:7891
    ProxyOverride    REG_SZ    <local>;*.local
"#;
        let observation = parse_windows_proxy_output(static_fixture);
        assert_eq!(
            observation
                .environment
                .get("HTTPS_PROXY")
                .map(String::as_str),
            Some("http://127.0.0.1:7890")
        );
        assert_eq!(
            observation.environment.get("ALL_PROXY").map(String::as_str),
            Some("socks5h://127.0.0.1:7891")
        );
        assert_eq!(
            observation.environment.get("NO_PROXY").map(String::as_str),
            Some("*.local,127.0.0.1,localhost")
        );

        let pac = parse_windows_proxy_output(
            "    ProxyEnable REG_DWORD 0x0\n    AutoConfigURL REG_SZ http://proxy/pac\n",
        );
        assert!(pac.unsupported_reason.is_some());
        let wpad = parse_windows_proxy_output(
            "    ProxyEnable REG_DWORD 0x0\n    AutoDetect REG_DWORD 0x1\n",
        );
        assert!(wpad.unsupported_reason.is_some());
    }

    #[test]
    fn resolved_snapshot_duplicates_only_canonical_proxy_names() {
        let resolved = ResolvedNetworkPolicy::from_observation(
            NetworkPolicyMode::Custom,
            "custom",
            ProxyObservation {
                environment: BTreeMap::from([
                    ("HTTPS_PROXY".to_string(), "http://proxy:7890".to_string()),
                    ("NO_PROXY".to_string(), "localhost".to_string()),
                ]),
                unsupported_reason: None,
            },
        );
        let names = resolved
            .environment
            .iter()
            .map(|(name, _)| name.to_string_lossy().to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            names,
            BTreeSet::from([
                "HTTPS_PROXY".to_string(),
                "NO_PROXY".to_string(),
                "https_proxy".to_string(),
                "no_proxy".to_string(),
            ])
        );
    }
}
