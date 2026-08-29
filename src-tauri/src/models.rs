use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitWindow {
    pub source: String,
    pub window_kind: String,
    pub used_percent: Option<f64>,
    pub resets_at: Option<i64>,
    pub is_manual: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAggregate {
    pub source: String,
    pub model: String,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub reasoning: i64,
    pub cost_usd: f64,
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionView {
    pub id: String,
    pub source: String,
    pub cwd: Option<String>,
    pub model: Option<String>,
    pub auto_continue_enabled: bool,
    pub continue_prompt: Option<String>,
    pub target_kind: Option<String>,
    pub target_ref: Option<String>,
    pub last_seen: Option<i64>,
    pub display_name: String,
    pub project: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingRow {
    pub model: String,
    pub input_pm: f64,
    pub output_pm: f64,
    pub cache_read_pm: f64,
    pub cache_write_pm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocontinueLog {
    pub id: i64,
    pub session_id: Option<String>,
    pub fired_at: i64,
    pub target: Option<String>,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub today_tokens: i64,
    pub today_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub limits: Vec<LimitWindow>,
    pub summary: DashboardSummary,
    pub recent_autocontinue: Vec<AutocontinueLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRange {
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    DesktopApp,
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionTarget {
    pub kind: TargetKind,
    pub reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum InjectOutcome {
    Sent,
    WindowNotFound,
    NoPermission,
    Error { detail: String },
    NotifyOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryStatus {
    pub present: bool,
    pub ccswitch_detected: bool,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingRates {
    pub input_pm: f64,
    pub output_pm: f64,
    pub cache_read_pm: f64,
    pub cache_write_pm: f64,
}
