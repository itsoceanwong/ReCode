use std::collections::HashMap;

use tauri::State;

use crate::config_writer;
use crate::injector;
use crate::models::{
    AutocontinueLog, DashboardData, InjectOutcome, InjectionTarget, PricingRates, PricingRow,
    SessionView, TelemetryStatus, UsageAggregate, UsageRange,
};
use crate::paths;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<HashMap<String, String>, String> {
    state.db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    state.db.set_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_dashboard(state: State<'_, AppState>) -> Result<DashboardData, String> {
    state.db.get_dashboard().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_usage(
    state: State<'_, AppState>,
    range: UsageRange,
    group_by: String,
) -> Result<Vec<UsageAggregate>, String> {
    state
        .db
        .get_usage(range.from, range.to, &group_by)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_sessions(state: State<'_, AppState>) -> Result<Vec<SessionView>, String> {
    let rows = state.db.get_sessions().map_err(|e| e.to_string())?;
    Ok(crate::session_enrich::enrich_sessions(rows))
}

#[tauri::command]
pub fn set_session_autocontinue(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
    prompt: Option<String>,
    target: Option<InjectionTarget>,
) -> Result<(), String> {
    let (kind, reference) = match target {
        Some(t) => (
            Some(injector::kind_label(&t.kind).to_string()),
            Some(t.reference),
        ),
        None => (None, None),
    };
    state
        .db
        .set_session_autocontinue(
            &id,
            enabled,
            prompt.as_deref(),
            kind.as_deref(),
            reference.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_manual_limit(
    state: State<'_, AppState>,
    source: String,
    kind: String,
    resets_at: i64,
    used_percent: Option<f64>,
) -> Result<(), String> {
    state
        .db
        .upsert_limit(&source, &kind, used_percent, Some(resets_at), true, None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_manual_limit(
    state: State<'_, AppState>,
    source: String,
    kind: String,
) -> Result<(), String> {
    state
        .db
        .clear_manual_limit(&source, &kind)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pricing(state: State<'_, AppState>) -> Result<Vec<PricingRow>, String> {
    state.db.get_pricing().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_pricing(
    state: State<'_, AppState>,
    model: String,
    rates: PricingRates,
) -> Result<(), String> {
    state
        .db
        .set_pricing(&PricingRow {
            model,
            input_pm: rates.input_pm,
            output_pm: rates.output_pm,
            cache_read_pm: rates.cache_read_pm,
            cache_write_pm: rates.cache_write_pm,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn enable_telemetry(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let port = *state.otlp_port.lock().map_err(|e| e.to_string())?;
    let port = if port == 0 { 4318 } else { port };
    config_writer::ensure_telemetry(port, true).map_err(|e| e.to_string())?;
    state
        .db
        .set_setting("telemetry_enabled", "true")
        .map_err(|e| e.to_string())?;
    state
        .db
        .set_setting("otlp_port", &port.to_string())
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "port": port }))
}

#[tauri::command]
pub fn disable_telemetry(state: State<'_, AppState>) -> Result<(), String> {
    config_writer::remove_telemetry().map_err(|e| e.to_string())?;
    state
        .db
        .set_setting("telemetry_enabled", "false")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn telemetry_status(state: State<'_, AppState>) -> Result<TelemetryStatus, String> {
    let port = *state.otlp_port.lock().map_err(|e| e.to_string())?;
    Ok(TelemetryStatus {
        present: config_writer::telemetry_present(),
        ccswitch_detected: paths::ccswitch_db().exists(),
        port,
    })
}

#[tauri::command]
pub fn list_injection_targets() -> Result<Vec<InjectionTarget>, String> {
    injector::list_targets().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn test_injection(
    state: State<'_, AppState>,
    target: InjectionTarget,
    text: String,
) -> Result<InjectOutcome, String> {
    let notify_only = state
        .db
        .get_setting("notify_only")
        .map_err(|e| e.to_string())?
        .map(|v| v == "true")
        .unwrap_or(false);
    if notify_only {
        return Ok(InjectOutcome::NotifyOnly);
    }
    injector::send(&target, &text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_accessibility_settings() -> Result<(), String> {
    injector::open_accessibility_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_autocontinue_log(state: State<'_, AppState>) -> Result<Vec<AutocontinueLog>, String> {
    state
        .db
        .get_dashboard()
        .map(|d| d.recent_autocontinue)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cursor_status(
    state: State<'_, AppState>,
) -> Result<crate::providers::cursor::CursorStatus, String> {
    let guard = state.cursor.lock().map_err(|e| e.to_string())?;
    Ok(guard.status())
}

#[tauri::command]
pub fn scan_cursor_now(state: State<'_, AppState>) -> Result<u64, String> {
    let mut guard = state.cursor.lock().map_err(|e| e.to_string())?;
    let enabled = state
        .db
        .get_setting("cursor_enabled")
        .map_err(|e| e.to_string())?
        .map(|v| v != "false")
        .unwrap_or(true);
    guard.set_enabled(enabled);
    guard.invalidate_cache();
    let n = guard.scan(&state.db).map_err(|e| e.to_string())?;
    Ok(n as u64)
}

#[tauri::command]
pub fn set_manual_usage(
    state: State<'_, AppState>,
    model: String,
    input: i64,
    output: i64,
    ts: Option<i64>,
) -> Result<(), String> {
    let ts = ts.unwrap_or_else(|| chrono::Utc::now().timestamp());
    let pricing = crate::pricing::matcher(&state.db, &model);
    let cost = crate::pricing::compute_cost(&pricing, input, output, 0, 0);
    let dedup = format!("cursor:manual:{model}:{ts}:{input}:{output}");
    state
        .db
        .insert_usage_exact(
            "cursor",
            &model,
            ts,
            input.max(0),
            output.max(0),
            0,
            0,
            0,
            cost,
            None,
            None,
            "jsonl",
            &dedup,
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}
