use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::config_writer;
use crate::paths;
use crate::providers::claude::ClaudeProvider;
use crate::providers::codex::CodexProvider;
use crate::state::AppState;

pub fn start(app: AppHandle, state: AppState) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<PathBuf>();

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                for path in event.paths {
                    let _ = tx.send(path);
                }
            }
        },
        notify::Config::default(),
    )?;

    let sessions = paths::codex_dir().join("sessions");
    if sessions.exists() {
        let _ = watcher.watch(&sessions, RecursiveMode::Recursive);
    }
    let archived = paths::codex_dir().join("archived_sessions");
    if archived.exists() {
        let _ = watcher.watch(&archived, RecursiveMode::Recursive);
    }
    let status = paths::recode_statusfile();
    if let Some(parent) = status.parent() {
        let _ = std::fs::create_dir_all(parent);
        if !status.exists() {
            let _ = std::fs::File::create(&status);
        }
        let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
    }
    let settings = paths::claude_settings();
    if let Some(parent) = settings.parent() {
        if parent.exists() {
            let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
        }
    }
    if let Some(cursor_db) = paths::cursor_state_db() {
        if let Some(parent) = cursor_db.parent() {
            let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
        }
    }

    std::mem::forget(watcher);

    let codex = Arc::new(Mutex::new(CodexProvider::new()));
    let claude = Arc::new(Mutex::new(ClaudeProvider::new()));
    let heal_skip = Arc::new(std::sync::atomic::AtomicBool::new(false));

    {
        if let Ok(mut c) = codex.lock() {
            let _ = c.scan_all(&state.db);
        }
    }

    let app2 = app.clone();
    let state2 = state.clone();
    let codex2 = Arc::clone(&codex);
    let claude2 = Arc::clone(&claude);
    let heal_skip2 = Arc::clone(&heal_skip);

    tauri::async_runtime::spawn(async move {
        let mut last: HashMap<PathBuf, Instant> = HashMap::new();
        while let Some(path) = rx.recv().await {
            let now = Instant::now();
            if let Some(prev) = last.get(&path) {
                if now.duration_since(*prev) < Duration::from_millis(500) {
                    continue;
                }
            }
            last.insert(path.clone(), now);
            tokio::time::sleep(Duration::from_millis(500)).await;

            let path_str = path.to_string_lossy().to_lowercase();
            if path_str.contains("rollout-") && path_str.ends_with(".jsonl") {
                if let Ok(mut c) = codex2.lock() {
                    if c.scan_file(&state2.db, &path).ok().unwrap_or(0) > 0 {
                        let _ = app2.emit("usage_updated", ());
                        let _ = app2.emit("limits_updated", ());
                    }
                }
            } else if path.file_name().and_then(|n| n.to_str()) == Some("claude-status.jsonl") {
                if let Ok(mut c) = claude2.lock() {
                    if c.parse_statusfile(&state2.db).ok().unwrap_or(0) > 0 {
                        let _ = app2.emit("usage_updated", ());
                        let _ = app2.emit("limits_updated", ());
                    }
                }
            } else if path.file_name().and_then(|n| n.to_str()) == Some("settings.json") {
                if heal_skip2.load(std::sync::atomic::Ordering::SeqCst) {
                    continue;
                }
                let enabled = state2
                    .db
                    .get_setting("telemetry_enabled")
                    .ok()
                    .flatten()
                    .map(|v| v == "true")
                    .unwrap_or(false);
                if enabled && !config_writer::telemetry_present() {
                    let port = *state2.otlp_port.lock().unwrap_or_else(|e| e.into_inner());
                    heal_skip2.store(true, std::sync::atomic::Ordering::SeqCst);
                    let _ = config_writer::ensure_telemetry(port, true);
                    heal_skip2.store(false, std::sync::atomic::Ordering::SeqCst);
                    let _ = app2.emit("telemetry_status_changed", ());
                }
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("state.vscdb"))
            {
                let enabled = state2
                    .db
                    .get_setting("cursor_enabled")
                    .ok()
                    .flatten()
                    .map(|v| v != "false")
                    .unwrap_or(true);
                if enabled {
                    if let Ok(mut c) = state2.cursor.lock() {
                        c.set_enabled(true);
                        match c.scan(&state2.db) {
                            Ok(n) if n > 0 => {
                                let _ = app2.emit("usage_updated", ());
                            }
                            Ok(_) => {}
                            Err(e) => eprintln!("cursor scan: {e}"),
                        }
                    }
                }
            }
        }
    });

    // Periodic Cursor rescan (mtime gate inside provider) — catches WAL-only writes.
    let state3 = state.clone();
    let app3 = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let enabled = state3
                .db
                .get_setting("cursor_enabled")
                .ok()
                .flatten()
                .map(|v| v != "false")
                .unwrap_or(true);
            if !enabled {
                continue;
            }
            if let Ok(mut c) = state3.cursor.lock() {
                c.set_enabled(true);
                if c.scan(&state3.db).ok().unwrap_or(0) > 0 {
                    let _ = app3.emit("usage_updated", ());
                }
            }
        }
    });

    Ok(())
}
