pub mod commands;

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
    let _ = codex_plus_core::diagnostic_log::append_diagnostic_log(
        "manager.start",
        serde_json::json!({
            "version": env!("CARGO_PKG_VERSION")
        }),
    );
    let Some(_guard) = acquire_single_instance_guard() else {
        return;
    };
    commands::scrub_managed_context_store();
    let run_result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let url = "/index.html";
            let mut main_window_builder =
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App(url.into()))
                    .title("Codex-- 管理工具")
                    .inner_size(1180.0, 820.0)
                    .min_inner_size(960.0, 720.0);
            if let Some(icon) = app.default_window_icon().cloned() {
                main_window_builder = main_window_builder.icon(icon)?;
            }
            let main_window = main_window_builder.build()?;
            install_tray(app)?;
            register_main_window_events(main_window);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_settings,
            commands::save_settings,
            commands::list_local_sessions,
            commands::delete_local_session,
            commands::open_external_url,
            commands::relay_status,
            commands::read_relay_files,
            commands::check_env_conflicts,
            commands::remove_env_conflicts,
            commands::save_relay_file,
            commands::write_diagnostic_event,
            commands::backfill_relay_profile_from_live,
            commands::extract_relay_common_config,
            commands::test_relay_profile,
            commands::diagnose_relay_profile,
            commands::fetch_relay_profile_models,
            commands::switch_relay_profile,
            commands::apply_relay_injection,
            commands::apply_pure_api_injection,
            commands::clear_relay_injection,
            update_tray_labels
        ])
        .run(tauri::generate_context!());
    if let Err(error) = run_result {
        let _ = codex_plus_core::diagnostic_log::append_diagnostic_log(
            "manager.run_failed",
            serde_json::json!({
                "error": error.to_string()
            }),
        );
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
        WindowEvent::Resized(_) => {
            if matches!(minimized_window.is_minimized(), Ok(true)) {
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

/// Restores and focuses an existing manager window on Windows.
///
/// This is a no-op on other platforms.
pub fn focus_existing_manager_window() {
    #[cfg(windows)]
    {
        let current_process_id = std::process::id();
        for process in codex_plus_core::windows_enumerate_processes() {
            if process.process_id == current_process_id {
                continue;
            }
            if process
                .exe_file
                .eq_ignore_ascii_case("codex-minus.exe")
            {
                let _ = codex_plus_core::windows_activate_process_window(process.process_id);
                break;
            }
        }
    }
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
        let _ = codex_plus_core::diagnostic_log::append_diagnostic_log(
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
                let _ = codex_plus_core::diagnostic_log::append_diagnostic_log(
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
            let _ = codex_plus_core::diagnostic_log::append_diagnostic_log(
                "manager.already_running",
                serde_json::json!({
                    "guard_port": codex_plus_core::ports::manager_guard_port()
                }),
            );
            focus_existing_manager_window();
            None
        }
        Err(error) => {
            let _ = codex_plus_core::diagnostic_log::append_diagnostic_log(
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
                    let _ = codex_plus_core::diagnostic_log::append_diagnostic_log(
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
