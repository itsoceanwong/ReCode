use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::injector;
use crate::models::{InjectOutcome, InjectionTarget, TargetKind};
use crate::state::AppState;

pub fn start(app: AppHandle, state: AppState) {
    let last_fired: Arc<Mutex<HashMap<String, i64>>> = Arc::new(Mutex::new(HashMap::new()));
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            if let Err(e) = tick(&app, &state, &last_fired).await {
                eprintln!("scheduler tick: {e}");
            }
        }
    });
}

async fn tick(
    app: &AppHandle,
    state: &AppState,
    last_fired: &Arc<Mutex<HashMap<String, i64>>>,
) -> anyhow::Result<()> {
    let settings = state.db.get_settings()?;
    let offset: i64 = settings
        .get("continue_offset_seconds")
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);
    let notify_only = settings
        .get("notify_only")
        .map(|v| v == "true")
        .unwrap_or(false);
    let default_prompt = settings
        .get("default_prompt")
        .cloned()
        .unwrap_or_else(|| "read the history, continue on the work".into());

    let sessions = state.db.enabled_autocontinue_sessions()?;
    let limits = state.db.all_limits()?;
    let now = chrono::Utc::now().timestamp();

    for session in sessions {
        let Some(resets_at) = limits
            .iter()
            .filter(|l| l.source == session.source && l.window_kind == "five_hour")
            .filter_map(|l| l.resets_at)
            .min()
        else {
            continue;
        };
        let fire_at = resets_at + offset;
        if now < fire_at {
            continue;
        }
        {
            let map = last_fired.lock().expect("last_fired");
            if map.get(&session.id) == Some(&resets_at) {
                continue;
            }
        }

        let prompt = session
            .continue_prompt
            .clone()
            .unwrap_or_else(|| default_prompt.clone());
        let target = match (session.target_kind.as_deref(), session.target_ref.as_deref()) {
            (Some(kind), Some(reference)) => InjectionTarget {
                kind: if kind == "terminal" {
                    TargetKind::Terminal
                } else {
                    TargetKind::DesktopApp
                },
                reference: reference.to_string(),
            },
            _ => {
                state.db.insert_autocontinue_log(
                    Some(&session.id),
                    None,
                    "error",
                    Some("no injection target configured"),
                )?;
                let _ = app.emit("autocontinue_fired", ());
                continue;
            }
        };

        let outcome = if notify_only {
            InjectOutcome::NotifyOnly
        } else {
            injector::send(&target, &prompt).unwrap_or_else(|e| InjectOutcome::Error {
                detail: e.to_string(),
            })
        };

        let (status, detail) = match &outcome {
            InjectOutcome::Sent => ("sent", None),
            InjectOutcome::NotifyOnly => ("sent", Some("notify_only".to_string())),
            InjectOutcome::WindowNotFound => ("window_not_found", None),
            InjectOutcome::NoPermission => ("no_permission", None),
            InjectOutcome::Error { detail } => ("error", Some(detail.clone())),
        };
        state.db.insert_autocontinue_log(
            Some(&session.id),
            Some(&target.reference),
            status,
            detail.as_deref(),
        )?;
        {
            let mut map = last_fired.lock().expect("last_fired");
            map.insert(session.id.clone(), resets_at);
        }
        let _ = app.emit("autocontinue_fired", ());
    }
    Ok(())
}
