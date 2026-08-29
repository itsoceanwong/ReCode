import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AutocontinueLogEntry,
  CursorStatus,
  DashboardData,
  GroupBy,
  InjectionTarget,
  InjectOutcome,
  PricingRow,
  SessionView,
  SettingsMap,
  TelemetryStatus,
  UsageAggregate,
  UsageRange,
} from "./types";

export const api = {
  getSettings: () => invoke<SettingsMap>("get_settings"),
  setSetting: (key: string, value: string) =>
    invoke<void>("set_setting", { key, value }),

  getDashboard: () => invoke<DashboardData>("get_dashboard"),
  getUsage: (range: UsageRange, groupBy: GroupBy) =>
    invoke<UsageAggregate[]>("get_usage", { range, groupBy }),
  getSessions: () => invoke<SessionView[]>("get_sessions"),

  setSessionAutocontinue: (
    id: string,
    enabled: boolean,
    prompt?: string | null,
    target?: InjectionTarget | null,
  ) =>
    invoke<void>("set_session_autocontinue", {
      id,
      enabled,
      prompt: prompt ?? null,
      target: target ?? null,
    }),

  setManualLimit: (
    source: string,
    kind: string,
    resetsAt: number,
    usedPercent?: number | null,
  ) =>
    invoke<void>("set_manual_limit", {
      source,
      kind,
      resetsAt,
      usedPercent: usedPercent ?? null,
    }),
  clearManualLimit: (source: string, kind: string) =>
    invoke<void>("clear_manual_limit", { source, kind }),

  getPricing: () => invoke<PricingRow[]>("get_pricing"),
  setPricing: (
    model: string,
    rates: Omit<PricingRow, "model">,
  ) => invoke<void>("set_pricing", { model, rates }),

  enableTelemetry: () => invoke<{ port: number }>("enable_telemetry"),
  disableTelemetry: () => invoke<void>("disable_telemetry"),
  telemetryStatus: () => invoke<TelemetryStatus>("telemetry_status"),

  listInjectionTargets: () =>
    invoke<InjectionTarget[]>("list_injection_targets"),
  testInjection: (target: InjectionTarget, text: string) =>
    invoke<InjectOutcome>("test_injection", { target, text }),
  openAccessibilitySettings: () =>
    invoke<void>("open_accessibility_settings"),

  getAutocontinueLog: () =>
    invoke<AutocontinueLogEntry[]>("get_autocontinue_log"),

  cursorStatus: () => invoke<CursorStatus>("cursor_status"),
  scanCursorNow: () => invoke<number>("scan_cursor_now"),
  setManualUsage: (
    model: string,
    input: number,
    output: number,
    ts?: number | null,
  ) =>
    invoke<void>("set_manual_usage", {
      model,
      input,
      output,
      ts: ts ?? null,
    }),
};

export function onUsageUpdated(cb: () => void): Promise<UnlistenFn> {
  return listen("usage_updated", () => cb());
}

export function onLimitsUpdated(cb: () => void): Promise<UnlistenFn> {
  return listen("limits_updated", () => cb());
}

export function onAutocontinueFired(cb: () => void): Promise<UnlistenFn> {
  return listen("autocontinue_fired", () => cb());
}

export function onTelemetryStatusChanged(cb: () => void): Promise<UnlistenFn> {
  return listen("telemetry_status_changed", () => cb());
}
