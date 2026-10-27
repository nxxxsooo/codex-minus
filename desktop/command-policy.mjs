export const CORE_COMMANDS = Object.freeze([
  "load_settings", "save_settings", "list_local_sessions", "permanently_delete_local_sessions",
  "load_session_lifecycle_settings", "save_session_lifecycle_settings", "preview_session_archive",
  "archive_local_session", "restore_local_session", "run_session_archive_maintenance",
  "open_external_url", "restart_codex_host", "relay_status", "read_relay_files", "check_env_conflicts",
  "remove_env_conflicts", "write_diagnostic_event", "extract_relay_common_config", "test_relay_profile",
  "diagnose_relay_profile", "fetch_relay_profile_models", "commit_provider_detail",
  "scan_provider_compatibility", "adapt_active_sessions_to_current_provider", "model_catalog_status",
  "adopt_external_model_catalog", "inspect_provider_native_capabilities", "transform_provider_native_capability_draft",
]);

export function allowedCommand(command) {
  return typeof command === "string" && CORE_COMMANDS.includes(command);
}

export function trustedSender(event, window, allowedOrigin) {
  if (!window || window.isDestroyed() || event.sender !== window.webContents) return false;
  const frame = event.senderFrame;
  if (!frame || frame !== window.webContents.mainFrame || frame.parent !== null) return false;
  try {
    const url = new URL(frame.url);
    return `${url.protocol}//${url.host}` === allowedOrigin
      && (url.protocol !== "codex-minus:" || url.pathname === "/index.html");
  } catch { return false; }
}

export function validTrayLabels(args) {
  return !!args && ["showLabel", "quitLabel", "windowTitle"].every((key) =>
    typeof args[key] === "string" && args[key].length > 0 && args[key].length <= 128);
}
