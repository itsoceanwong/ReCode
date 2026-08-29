mod commands;
mod config_writer;
mod db;
mod injector;
#[cfg(target_os = "macos")]
mod injector_macos;
#[cfg(target_os = "windows")]
mod injector_windows;
mod limits;
mod models;
mod otlp;
mod paths;
mod pricing;
mod providers;
mod scheduler;
mod state;
mod watcher;

use tauri::{Emitter, Manager};

use crate::db::Db;
use crate::otlp::OtlpServer;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let debug = std::env::var_os("RECODE_DEBUG").is_some();
            let app_data = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("app_data_dir: {e}"))?;
            std::fs::create_dir_all(&app_data)?;
            let db_path = app_data.join("recode.db");
            let debug_log_path = app_data.join("debug-startup.log");
            let mut debug_log = if debug {
                Some(
                    std::fs::File::create(&debug_log_path)
                        .map_err(|e| format!("debug-startup.log: {e}"))?,
                )
            } else {
                None
            };
            let mut dlog = |msg: &str| {
                eprintln!("{msg}");
                if let Some(f) = debug_log.as_mut() {
                    use std::io::Write;
                    let _ = writeln!(f, "{msg}");
                    let _ = f.flush();
                }
            };
            if debug {
                dlog(&format!(
                    "[startup] app_data_dir = {}",
                    app_data.display()
                ));
                dlog(&format!("[startup] db_path      = {}", db_path.display()));
                dlog(&format!(
                    "[startup] debug_log    = {}",
                    debug_log_path.display()
                ));
            }
            let db = Db::open(&db_path).map_err(|e| e.to_string())?;

            let seed = include_str!("../resources/pricing-seed.json");
            pricing::seed_pricing_if_empty(&db, seed).map_err(|e| e.to_string())?;

            let recode_home = paths::recode_dir();
            let preferred: u16 = db
                .get_setting("otlp_port")
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let telemetry_enabled = db
                .get_setting("telemetry_enabled")
                .ok()
                .flatten()
                .map(|v| v == "true")
                .unwrap_or(false);
            let cursor_enabled = db
                .get_setting("cursor_enabled")
                .ok()
                .flatten()
                .map(|v| v != "false")
                .unwrap_or(true);

            if debug {
                dlog(&format!(
                    "[startup] recode_dir         = {}",
                    recode_home.display()
                ));
                dlog(&format!("[startup] otlp_port_pref     = {preferred}"));
                dlog(&format!("[startup] telemetry_enabled  = {telemetry_enabled}"));
                dlog(&format!("[startup] cursor_enabled     = {cursor_enabled}"));
            }

            let state = AppState::new(db, preferred, cursor_enabled);
            if let Err(e) = watcher::start(app.handle().clone(), state.clone()) {
                eprintln!("watcher start: {e}");
            } else if debug {
                dlog("[startup] watcher started");
            }

            // Cursor scan can be slow on large stores — do not block first paint.
            {
                let handle = app.handle().clone();
                let state_scan = state.clone();
                let debug_scan = debug;
                let debug_log_path_scan = debug_log_path.clone();
                std::thread::spawn(move || {
                    let result = state_scan
                        .cursor
                        .lock()
                        .map_err(|e| e.to_string())
                        .and_then(|mut c| c.scan(&state_scan.db).map_err(|e| e.to_string()));
                    match result {
                        Ok(n) => {
                            if debug_scan {
                                let msg = format!("[startup] cursor scan: {n} new event(s)");
                                eprintln!("{msg}");
                                if let Ok(mut f) = std::fs::OpenOptions::new()
                                    .append(true)
                                    .open(&debug_log_path_scan)
                                {
                                    use std::io::Write;
                                    let _ = writeln!(f, "{msg}");
                                }
                            }
                            if n > 0 {
                                let _ = handle.emit("usage_updated", ());
                            }
                        }
                        Err(e) => eprintln!("cursor scan: {e}"),
                    }
                });
            }

            let handle = app.handle().clone();
            let state_for_otlp = state.clone();
            let debug_log_path_otlp = debug_log_path.clone();
            tauri::async_runtime::spawn(async move {
                match OtlpServer::start(handle, state_for_otlp.clone(), preferred).await {
                    Ok(port) => {
                        if debug {
                            let msg = format!("[startup] otlp listening on port {port}");
                            eprintln!("{msg}");
                            if let Ok(mut f) = std::fs::OpenOptions::new()
                                .append(true)
                                .open(&debug_log_path_otlp)
                            {
                                use std::io::Write;
                                let _ = writeln!(f, "{msg}");
                            }
                        }
                        if let Ok(mut p) = state_for_otlp.otlp_port.lock() {
                            *p = port;
                        }
                        let _ = state_for_otlp.db.set_setting("otlp_port", &port.to_string());
                        if telemetry_enabled {
                            let _ = config_writer::ensure_telemetry(port, true);
                        }
                    }
                    Err(e) => eprintln!("otlp start: {e}"),
                }
            });

            scheduler::start(app.handle().clone(), state.clone());
            if debug {
                dlog("[startup] scheduler started");
            }
            app.manage(state);
            if debug {
                dlog("[startup] setup complete");
                if let Some(win) = app.get_webview_window("main") {
                    win.open_devtools();
                    dlog("[startup] webview DevTools opened");
                } else {
                    dlog("[startup] main window not ready for DevTools yet");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_setting,
            commands::get_dashboard,
            commands::get_usage,
            commands::get_sessions,
            commands::set_session_autocontinue,
            commands::set_manual_limit,
            commands::clear_manual_limit,
            commands::get_pricing,
            commands::set_pricing,
            commands::enable_telemetry,
            commands::disable_telemetry,
            commands::telemetry_status,
            commands::list_injection_targets,
            commands::test_injection,
            commands::open_accessibility_settings,
            commands::get_autocontinue_log,
            commands::cursor_status,
            commands::scan_cursor_now,
            commands::set_manual_usage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
