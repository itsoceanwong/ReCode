export type Source = "claude" | "codex" | "cursor";
export type WindowKind = "five_hour" | "seven_day";
export type Origin = "otlp" | "statusline" | "jsonl";
export type TargetKind = "desktop_app" | "terminal";
export type GroupBy = "app" | "model";

export interface LimitWindow {
  source: string;
  window_kind: string;
  used_percent: number | null;
  resets_at: number | null;
  is_manual: boolean;
  updated_at: number;
}

export interface UsageAggregate {
  source: string;
  model: string;
  input: number;
  output: number;
  cache_read: number;
  cache_write: number;
  reasoning: number;
  cost_usd: number;
  origin: Origin;
}

export interface SessionView {
  id: string;
  source: string;
  cwd: string | null;
  model: string | null;
  auto_continue_enabled: boolean;
  continue_prompt: string | null;
  target_kind: TargetKind | null;
  target_ref: string | null;
  last_seen: number | null;
  display_name: string;
  project: string | null;
}

export interface PricingRow {
  model: string;
  input_pm: number;
  output_pm: number;
  cache_read_pm: number;
  cache_write_pm: number;
}

export interface SettingsMap {
  [key: string]: string;
}

export interface DashboardSummary {
  today_tokens: number;
  today_cost_usd: number;
}

export interface AutocontinueLogEntry {
  id: number;
  session_id: string | null;
  fired_at: number;
  target: string | null;
  status: string;
  detail: string | null;
}

export interface DashboardData {
  limits: LimitWindow[];
  summary: DashboardSummary;
  recent_autocontinue: AutocontinueLogEntry[];
}

export interface TelemetryStatus {
  present: boolean;
  ccswitch_detected: boolean;
  port: number;
}

export interface InjectionTarget {
  kind: TargetKind;
  reference: string;
}

export type InjectOutcome =
  | { status: "sent" }
  | { status: "window_not_found" }
  | { status: "no_permission" }
  | { status: "error"; detail: string }
  | { status: "notify_only" };

export interface UsageRange {
  from: number;
  to: number;
}

export interface CursorStatus {
  enabled: boolean;
  db_found: boolean;
  db_path: string | null;
  last_inserted: number;
  schema_ok: boolean;
  detail: string | null;
}
