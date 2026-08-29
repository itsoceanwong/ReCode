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
            let app_data = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("app_data_dir: {e}"))?;
            std::fs::create_dir_all(&app_data)?;
            let db_path = app_data.join("recode.db");
            let db = Db::open(&db_path).map_err(|e| e.to_string())?;

            let seed = include_str!("../resources/pricing-seed.json");
            pricing::seed_pricing_if_empty(&db, seed).map_err(|e| e.to_string())?;

            let _ = paths::recode_dir();
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

            let state = AppState::new(db, preferred, cursor_enabled);
            if let Err(e) = watcher::start(app.handle().clone(), state.clone()) {
                eprintln!("watcher start: {e}");
            }
            // Initial Cursor scan (best-effort).
            {
                if let Ok(mut c) = state.cursor.lock() {
                    match c.scan(&state.db) {
                        Ok(n) if n > 0 => {
                            let _ = app.emit("usage_updated", ());
                        }
                        Ok(_) => {}
                        Err(e) => eprintln!("cursor scan: {e}"),
                    }
                }
            }

            let handle = app.handle().clone();
            let state_for_otlp = state.clone();
            tauri::async_runtime::spawn(async move {
                match OtlpServer::start(handle, state_for_otlp.clone(), preferred).await {
                    Ok(port) => {
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
            app.manage(state);
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
