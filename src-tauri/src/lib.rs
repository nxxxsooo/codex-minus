pub mod commands;
mod live_state;
mod model_catalog;
mod platform_command;
pub mod provider_commit;
pub mod provider_native_capability;
mod session_adaptation;

#[cfg(test)]
mod provider_commit_transaction_tests;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

const TRAY_ID: &str = "codex_minus_tray";

static APP_EXITING: AtomicBool = AtomicBool::new(false);
const TRAY_MENU_SHOW: &str = "tray_show_main";
const TRAY_MENU_QUIT: &str = "tray_quit_app";

pub fn run() {
    install_panic_logger();
    let _ = commands::append_manager_diagnostic(
        "manager.start",
        serde_json::json!({
            "version": env!("CARGO_PKG_VERSION")
        }),
    );
    let Some(_guard) = acquire_single_instance_guard() else {
        return;
    };
    commands::scrub_legacy_managed_config_store();
    let app_result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let url = "/index.html";
            let mut main_window_builder =
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App(url.into()))
                    .title("Codex Minus")
                    .inner_size(1180.0, 820.0)
                    .min_inner_size(960.0, 720.0);
            if let Some(icon) = app.default_window_icon().cloned() {
                main_window_builder = main_window_builder.icon(icon)?;
            }
            let main_window = main_window_builder.build()?;
            install_tray(app)?;
            register_main_window_events(main_window);
            install_instance_activation_listener(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_settings,
            commands::save_settings,
            commands::list_local_sessions,
            commands::delete_local_session,
            commands::load_session_lifecycle_settings,
            commands::save_session_lifecycle_settings,
            commands::preview_session_archive,
            commands::archive_local_session,
            commands::restore_local_session,
            commands::run_session_archive_maintenance,
            commands::open_external_url,
            commands::restart_codex_host,
            commands::relay_status,
            commands::read_relay_files,
            commands::check_env_conflicts,
            commands::remove_env_conflicts,
            commands::write_diagnostic_event,
            commands::extract_relay_common_config,
            commands::test_relay_profile,
            commands::diagnose_relay_profile,
            commands::fetch_relay_profile_models,
            commands::commit_provider_detail,
            commands::scan_provider_compatibility,
            commands::adapt_active_sessions_to_current_provider,
            model_catalog::model_catalog_status,
            model_catalog::adopt_external_model_catalog,
            provider_native_capability::inspect_provider_native_capabilities,
            provider_native_capability::transform_provider_native_capability_draft,
            update_tray_labels
        ])
        .build(tauri::generate_context!());
    match app_result {
        Ok(app) => app.run(|app_handle, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = event
            {
                let _ = commands::append_manager_diagnostic(
                    "manager.reopen",
                    serde_json::json!({
                        "had_visible_windows": has_visible_windows
                    }),
                );
                show_main_window(app_handle);
            }

            #[cfg(not(target_os = "macos"))]
            let _ = (app_handle, event);
        }),
        Err(error) => {
            let _ = commands::append_manager_diagnostic(
                "manager.run_failed",
                serde_json::json!({
                    "error": error.to_string()
                }),
            );
        }
    }
}

fn install_tray<R: tauri::Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, TRAY_MENU_SHOW, "显示主窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, TRAY_MENU_QUIT, "退出程序", true, None::<&str>)?;
    let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            TRAY_MENU_SHOW => {
                show_main_window(app);
            }
            TRAY_MENU_QUIT => {
                APP_EXITING.store(true, Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => {
                show_main_window(&tray.app_handle());
            }
            _ => {}
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }

    let _ = tray_builder.build(app)?;
    Ok(())
}

fn register_main_window_events<R: tauri::Runtime>(window: tauri::WebviewWindow<R>) {
    let event_window = window.clone();
    let minimized_window = event_window.clone();
    let close_event_window = event_window.clone();

    event_window.on_window_event(move |event| match event {
        WindowEvent::Resized(size) => {
            if should_query_minimized_state(size.width, size.height, cfg!(windows))
                && matches!(minimized_window.is_minimized(), Ok(true))
            {
                let _ = minimized_window.hide();
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            if APP_EXITING.load(Ordering::SeqCst) {
                return;
            }

            api.prevent_close();
            let _ = close_event_window.hide();
        }
        _ => {}
    });
}

fn should_query_minimized_state(width: u32, height: u32, is_windows: bool) -> bool {
    !is_windows || width == 0 || height == 0
}

#[tauri::command]
fn update_tray_labels<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    show_label: String,
    quit_label: String,
    window_title: String,
) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let show_item = MenuItem::with_id(&app, TRAY_MENU_SHOW, &show_label, true, None::<&str>);
        let quit_item = MenuItem::with_id(&app, TRAY_MENU_QUIT, &quit_label, true, None::<&str>);
        if let (Ok(show), Ok(quit)) = (show_item, quit_item) {
            if let Ok(menu) = Menu::with_items(&app, &[&show, &quit]) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&window_title);
    }
}

fn show_main_window<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Restores and focuses an existing manager window.
pub fn focus_existing_manager_window() {
    #[cfg(unix)]
    if notify_existing_manager(&instance_activation_socket_path()) {
        return;
    }

    #[cfg(windows)]
    {
        let current_process_id = std::process::id();
        for process in codex_plus_core::windows_enumerate_processes() {
            if process.process_id == current_process_id {
                continue;
            }
            if process.exe_file.eq_ignore_ascii_case("codex-minus.exe") {
                let _ = codex_plus_core::windows_activate_process_window(process.process_id);
                break;
            }
        }
    }
}

#[cfg(unix)]
fn instance_activation_socket_path() -> std::path::PathBuf {
    codex_plus_core::paths::default_app_state_dir().join("manager-activate.sock")
}

#[cfg(unix)]
fn notify_existing_manager(socket_path: &std::path::Path) -> bool {
    for _ in 0..20 {
        if std::os::unix::net::UnixStream::connect(socket_path).is_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    false
}

#[cfg(unix)]
fn install_instance_activation_listener<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>) {
    use std::io::ErrorKind;
    use std::os::unix::net::UnixListener;

    let socket_path = instance_activation_socket_path();
    if let Some(parent) = socket_path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        log_instance_activation_listener_error(&socket_path, &error);
        return;
    }
    if let Err(error) = std::fs::remove_file(&socket_path)
        && error.kind() != ErrorKind::NotFound
    {
        log_instance_activation_listener_error(&socket_path, &error);
        return;
    }

    let listener = match UnixListener::bind(&socket_path) {
        Ok(listener) => listener,
        Err(error) => {
            log_instance_activation_listener_error(&socket_path, &error);
            return;
        }
    };

    std::thread::spawn(move || {
        for connection in listener.incoming() {
            match connection {
                Ok(_) => {
                    let activation_app = app_handle.clone();
                    let _ = app_handle.run_on_main_thread(move || {
                        show_main_window(&activation_app);
                    });
                }
                Err(error) => {
                    log_instance_activation_listener_error(&socket_path, &error);
                    break;
                }
            }
        }
    });
}

#[cfg(not(unix))]
fn install_instance_activation_listener<R: tauri::Runtime>(_app_handle: tauri::AppHandle<R>) {}

#[cfg(unix)]
fn log_instance_activation_listener_error(socket_path: &std::path::Path, error: &std::io::Error) {
    let _ = commands::append_manager_diagnostic(
        "manager.activation_listener_failed",
        serde_json::json!({
            "socket_path": socket_path,
            "error": error.to_string()
        }),
    );
}

fn install_panic_logger() {
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|message| (*message).to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "非字符串 panic payload".to_string());
        let location = panic_info.location().map(|location| {
            serde_json::json!({
                "file": location.file(),
                "line": location.line(),
                "column": location.column()
            })
        });
        let _ = commands::append_manager_diagnostic(
            "manager.panic",
            serde_json::json!({
                "payload": payload,
                "location": location
            }),
        );
    }));
}

fn acquire_single_instance_guard() -> Option<codex_plus_core::ports::LoopbackPortGuard> {
    match codex_plus_core::ports::acquire_resilient_loopback_port_guard(
        codex_plus_core::ports::manager_guard_port(),
    ) {
        Ok(guard) => {
            if let Some(fallback_lock_path) = guard.fallback_path() {
                let _ = commands::append_manager_diagnostic(
                    "manager.guard_fallback",
                    serde_json::json!({
                        "requested_guard_port": codex_plus_core::ports::manager_guard_port(),
                        "fallback_lock_path": fallback_lock_path
                    }),
                );
            }
            Some(guard)
        }
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::AddrInUse | std::io::ErrorKind::WouldBlock
            ) =>
        {
            let _ = commands::append_manager_diagnostic(
                "manager.already_running",
                serde_json::json!({
                    "guard_port": codex_plus_core::ports::manager_guard_port()
                }),
            );
            focus_existing_manager_window();
            None
        }
        Err(error) => {
            let _ = commands::append_manager_diagnostic(
                "manager.guard_failed",
                serde_json::json!({
                    "guard_port": codex_plus_core::ports::manager_guard_port(),
                    "error": error.to_string()
                }),
            );
            match std::net::TcpListener::bind(("127.0.0.1", 0)) {
                Ok(listener) => Some(codex_plus_core::ports::LoopbackPortGuard::listener(
                    listener,
                )),
                Err(fallback_error) => {
                    let _ = commands::append_manager_diagnostic(
                        "manager.guard_fallback_failed",
                        serde_json::json!({
                            "error": fallback_error.to_string()
                        }),
                    );
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn existing_instance_notification_connects_to_activation_socket() {
        let temp_dir = tempfile::tempdir().unwrap();
        let socket_path = temp_dir.path().join("manager-activate.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket_path).unwrap();

        assert!(notify_existing_manager(&socket_path));
        assert!(listener.accept().is_ok());
    }

    #[test]
    fn windows_normal_resize_skips_synchronous_minimized_query() {
        assert!(!should_query_minimized_state(1180, 820, true));
        assert!(should_query_minimized_state(0, 820, true));
        assert!(should_query_minimized_state(1180, 0, true));
        assert!(should_query_minimized_state(1180, 820, false));
    }
}
