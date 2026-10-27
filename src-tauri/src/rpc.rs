//! Private, bounded NDJSON transport. This is the desktop command registry; legacy direct-write
//! helpers deliberately do not appear here. No transport error serializes input or panic text.
use std::collections::HashSet;
use std::io;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use crate::{commands, model_catalog, provider_native_capability};

pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;
const MAX_PENDING: usize = 32;
const MAX_REQUEST_ID: u64 = 9_007_199_254_740_991;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: u64,
    command: String,
    #[serde(default = "empty_args")]
    args: Value,
}

fn empty_args() -> Value {
    json!({})
}

#[derive(Debug, Clone, Copy)]
struct RpcError(&'static str, &'static str);

const INVALID_ARGS: RpcError = RpcError("InvalidArguments", "命令参数无效。");
const PANICKED: RpcError = RpcError("CommandInterrupted", "核心命令中断，请重新读取状态后再试。");

fn error_frame(id: Option<u64>, error: RpcError) -> Value {
    json!({ "id": id, "error": { "code": error.0, "message": error.1 } })
}

fn keys(args: &Value, allowed: &[&str]) -> Result<(), RpcError> {
    let object = args.as_object().ok_or(INVALID_ARGS)?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(INVALID_ARGS);
    }
    Ok(())
}

fn field<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T, RpcError> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null)).map_err(|_| INVALID_ARGS)
}

fn encoded(value: impl Serialize) -> Result<Value, RpcError> {
    serde_json::to_value(value).map_err(|_| RpcError("SerializationFailed", "核心响应无法序列化。"))
}

async fn blocking<T: Serialize + Send + 'static>(
    action: impl FnOnce() -> T + Send + 'static,
) -> Result<Value, RpcError> {
    encoded(
        tokio::task::spawn_blocking(action)
            .await
            .map_err(|_| PANICKED)?,
    )
}

async fn dispatch(command: &str, args: Value) -> Result<Value, RpcError> {
    match command {
        "health" => {
            keys(&args, &[])?;
            Ok(
                json!({ "protocolVersion": 1, "version": env!("CARGO_PKG_VERSION"),
                "settingsPath": codex_plus_core::paths::default_settings_path(),
                "codexHome": codex_plus_core::codex_sqlite::default_codex_home_dir() }),
            )
        }
        "load_settings" => {
            keys(&args, &[])?;
            encoded(commands::load_settings().await)
        }
        "save_settings" => {
            keys(&args, &["settings"])?;
            encoded(commands::save_settings(field(&args, "settings")?).await)
        }
        "list_local_sessions" => {
            keys(&args, &["request"])?;
            encoded(commands::list_local_sessions(field(&args, "request")?).await)
        }
        "permanently_delete_local_sessions" => {
            keys(&args, &["request"])?;
            encoded(commands::permanently_delete_local_sessions(field(&args, "request")?).await)
        }
        "load_session_lifecycle_settings" => {
            keys(&args, &[])?;
            encoded(commands::load_session_lifecycle_settings().await)
        }
        "save_session_lifecycle_settings" => {
            keys(&args, &["settings"])?;
            encoded(commands::save_session_lifecycle_settings(field(&args, "settings")?).await)
        }
        "preview_session_archive" => {
            keys(&args, &["request"])?;
            encoded(commands::preview_session_archive(field(&args, "request")?).await)
        }
        "archive_local_session" => {
            keys(&args, &["request"])?;
            encoded(commands::archive_local_session(field(&args, "request")?).await)
        }
        "restore_local_session" => {
            keys(&args, &["request"])?;
            encoded(commands::restore_local_session(field(&args, "request")?).await)
        }
        "run_session_archive_maintenance" => {
            keys(&args, &["force"])?;
            encoded(commands::run_session_archive_maintenance(field(&args, "force")?).await)
        }
        "open_external_url" => {
            keys(&args, &["url"])?;
            let url = field(&args, "url")?;
            blocking(move || commands::open_external_url(url)).await
        }
        "restart_codex_host" => {
            keys(&args, &[])?;
            encoded(commands::restart_codex_host().await)
        }
        "relay_status" => {
            keys(&args, &[])?;
            encoded(commands::relay_status().await)
        }
        "read_relay_files" => {
            keys(&args, &[])?;
            encoded(commands::read_relay_files().await)
        }
        "check_env_conflicts" => {
            keys(&args, &[])?;
            encoded(commands::check_env_conflicts().await)
        }
        "remove_env_conflicts" => {
            keys(&args, &["request"])?;
            let request = field(&args, "request")?;
            blocking(move || commands::remove_env_conflicts(request)).await
        }
        "write_diagnostic_event" => {
            keys(&args, &["event", "detail"])?;
            let event = field(&args, "event")?;
            let detail = field(&args, "detail")?;
            blocking(move || commands::write_diagnostic_event(event, detail)).await
        }
        "extract_relay_common_config" => {
            keys(&args, &["request"])?;
            let request = field(&args, "request")?;
            blocking(move || commands::extract_relay_common_config(request)).await
        }
        "test_relay_profile" => {
            keys(&args, &["profile"])?;
            encoded(commands::test_relay_profile(field(&args, "profile")?).await)
        }
        "diagnose_relay_profile" => {
            keys(&args, &["profile"])?;
            encoded(commands::diagnose_relay_profile(field(&args, "profile")?).await)
        }
        "fetch_relay_profile_models" => {
            keys(&args, &["profile"])?;
            encoded(commands::fetch_relay_profile_models(field(&args, "profile")?).await)
        }
        "commit_provider_detail" => {
            keys(&args, &["request"])?;
            encoded(commands::commit_provider_detail(field(&args, "request")?).await)
        }
        "scan_provider_compatibility" => {
            keys(&args, &[])?;
            encoded(commands::scan_provider_compatibility().await)
        }
        "adapt_active_sessions_to_current_provider" => {
            keys(&args, &["scanGeneration"])?;
            encoded(
                commands::adapt_active_sessions_to_current_provider(field(
                    &args,
                    "scanGeneration",
                )?)
                .await,
            )
        }
        "model_catalog_status" => {
            keys(&args, &[])?;
            encoded(model_catalog::model_catalog_status().await)
        }
        "adopt_external_model_catalog" => {
            keys(&args, &["request"])?;
            encoded(model_catalog::adopt_external_model_catalog(field(&args, "request")?).await)
        }
        "inspect_provider_native_capabilities" => {
            keys(&args, &["request"])?;
            encoded(
                provider_native_capability::inspect_provider_native_capabilities(field(
                    &args, "request",
                )?)
                .await,
            )
        }
        "transform_provider_native_capability_draft" => {
            keys(&args, &["request"])?;
            encoded(
                provider_native_capability::transform_provider_native_capability_draft(field(
                    &args, "request",
                )?)
                .await,
            )
        }
        _ => Err(RpcError("UnknownCommand", "此核心命令不可用。")),
    }
}

async fn read_frame(reader: &mut (impl AsyncBufRead + Unpin)) -> io::Result<Option<Vec<u8>>> {
    let mut frame = Vec::new();
    loop {
        let bytes = reader.fill_buf().await?;
        if bytes.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err(io::ErrorKind::UnexpectedEof.into())
            };
        }
        let count = bytes
            .iter()
            .position(|b| *b == b'\n')
            .map_or(bytes.len(), |i| i + 1);
        if frame.len() + count > MAX_FRAME_BYTES {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let complete = bytes[count - 1] == b'\n';
        frame.extend_from_slice(&bytes[..count]);
        reader.consume(count);
        if complete {
            return Ok(Some(frame));
        }
    }
}

async fn write_frame(writer: &mut (impl AsyncWrite + Unpin), frame: Value) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(&frame)?;
    if bytes.len() + 1 > MAX_FRAME_BYTES {
        bytes = serde_json::to_vec(&error_frame(
            frame.get("id").and_then(Value::as_u64),
            RpcError("ResponseTooLarge", "核心响应过大；请重新读取状态。"),
        ))?;
    }
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await
}

pub async fn serve() -> io::Result<()> {
    // A panic payload can contain a provider key; the transport returns only static errors.
    std::panic::set_hook(Box::new(|_| {}));
    let mut stdout = tokio::io::stdout();
    let (focus_tx, mut focus_rx) = mpsc::channel(1);
    let _owner = match crate::runtime_owner::acquire(focus_tx) {
        Ok(owner) => owner,
        Err(error) => {
            write_frame(
                &mut stdout,
                json!({"event":"startupError", "code":error.code()}),
            )
            .await?;
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    };
    commands::scrub_legacy_managed_config_store();
    write_frame(
        &mut stdout,
        json!({"event":"ready", "protocolVersion":1, "version":env!("CARGO_PKG_VERSION")}),
    )
    .await?;
    let (tx, mut rx) = mpsc::channel(MAX_PENDING);
    let reader = tokio::spawn(async move {
        let mut stdin = BufReader::new(tokio::io::stdin());
        loop {
            let frame = read_frame(&mut stdin).await;
            let done = !matches!(&frame, Ok(Some(_)));
            if tx.send(frame).await.is_err() || done {
                break;
            }
        }
    });
    let mut tasks = JoinSet::new();
    let mut pending = HashSet::new();
    let mut reading = true;
    let mut focus_open = true;
    while reading || !tasks.is_empty() {
        tokio::select! {
            frame = rx.recv(), if reading => {
                let bytes = match frame {
                    Some(Ok(Some(bytes))) => bytes,
                    Some(Ok(None)) | None => { reading = false; continue; }
                    Some(Err(_)) => {
                        write_frame(&mut stdout, error_frame(None, RpcError("InvalidFrame", "核心消息帧无效。"))).await?;
                        reading = false;
                        continue;
                    }
                };
                let request = match serde_json::from_slice::<Request>(&bytes) {
                    Ok(request) if request.id <= MAX_REQUEST_ID && request.command.len() <= 96 && request.args.is_object() => request,
                    _ => {
                        write_frame(&mut stdout, error_frame(None, RpcError("InvalidRequest", "核心请求无效。"))).await?;
                        reading = false;
                        continue;
                    }
                };
                let id = request.id;
                if pending.contains(&id) {
                    write_frame(&mut stdout, error_frame(None, RpcError("DuplicateRequest", "核心请求编号重复。"))).await?;
                    reading = false;
                } else if tasks.len() >= MAX_PENDING {
                    write_frame(&mut stdout, error_frame(Some(id), RpcError("CoreBusy", "核心繁忙，请稍后重试。"))).await?;
                } else {
                    pending.insert(id);
                    tasks.spawn(async move {
                        let result = tokio::spawn(async move { dispatch(&request.command, request.args).await })
                            .await.unwrap_or(Err(PANICKED));
                        (id, result)
                    });
                }
            }
            Some(outcome) = tasks.join_next(), if !tasks.is_empty() => {
                let (id, result) = outcome.map_err(|_| io::ErrorKind::Other)?;
                pending.remove(&id);
                let frame = match result {
                    Ok(result) => json!({"id":id,"result":result}),
                    Err(error) => error_frame(Some(id), error),
                };
                write_frame(&mut stdout, frame).await?;
            }
            event = focus_rx.recv(), if focus_open => {
                if event.is_some() { write_frame(&mut stdout, json!({"event":"focus"})).await?; }
                else { focus_open = false; }
            }
        }
    }
    reader.abort();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_bypass_commands_and_never_echoes_their_input() {
        for command in [
            "save_relay_file",
            "apply_relay_injection",
            "clear_relay_injection",
            "save_active_relay_profile",
        ] {
            let error = dispatch(command, json!({"secret":"secret-sentinel"}))
                .await
                .unwrap_err();
            assert_eq!(error.0, "UnknownCommand");
            assert!(
                !error_frame(Some(7), error)
                    .to_string()
                    .contains("secret-sentinel")
            );
        }
        assert_eq!(
            dispatch("health", json!({"unexpected":"secret-sentinel"}))
                .await
                .unwrap_err()
                .0,
            "InvalidArguments"
        );
    }

    #[tokio::test]
    async fn framing_rejects_truncation_and_oversize_without_unbounded_allocation() {
        assert!(read_frame(&mut BufReader::new(&b"{}"[..])).await.is_err());
        let too_big = vec![b'x'; MAX_FRAME_BYTES + 1];
        assert!(
            read_frame(&mut BufReader::new(too_big.as_slice()))
                .await
                .is_err()
        );
        let input = b"{\"id\":1}\n{\"id\":2}\n";
        let mut reader = BufReader::with_capacity(3, &input[..]);
        assert_eq!(
            read_frame(&mut reader).await.unwrap().unwrap(),
            b"{\"id\":1}\n"
        );
        assert_eq!(
            read_frame(&mut reader).await.unwrap().unwrap(),
            b"{\"id\":2}\n"
        );
        assert!(read_frame(&mut reader).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn command_boundary_preserves_correlation_payload_and_static_parse_failures() {
        let health = dispatch("health", json!({})).await.unwrap();
        assert_eq!(health["protocolVersion"], 1);
        let error = dispatch(
            "commit_provider_detail",
            json!({"request":"secret-sentinel"}),
        )
        .await
        .unwrap_err();
        assert_eq!(error.0, "InvalidArguments");
        assert!(
            !error_frame(Some(9), error)
                .to_string()
                .contains("secret-sentinel")
        );
    }
}
