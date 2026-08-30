import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import type {
  CursorStatus,
  PricingRow,
  TelemetryStatus,
} from "@/lib/types";
import { Zap, Radio, Database, DollarSign } from "lucide-react";

export function SettingsPage() {
  const [prompt, setPrompt] = useState("");
  const [offset, setOffset] = useState("120");
  const [notifyOnly, setNotifyOnly] = useState(false);
  const [cursorEnabled, setCursorEnabled] = useState(true);
  const [cursorStat, setCursorStat] = useState<CursorStatus | null>(null);
  const [manualModel, setManualModel] = useState("cursor-auto");
  const [manualIn, setManualIn] = useState("0");
  const [manualOut, setManualOut] = useState("0");
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tel, setTel] = useState<TelemetryStatus | null>(null);
  const [pricing, setPricingRows] = useState<PricingRow[]>([]);

  async function refresh() {
    const [s, t, price, cstat] = await Promise.all([
      api.getSettings(),
      api.telemetryStatus(),
      api.getPricing(),
      api.cursorStatus().catch(() => null),
    ]);
    setPrompt(s.default_prompt ?? "");
    setOffset(s.continue_offset_seconds ?? "120");
    setNotifyOnly(s.notify_only === "true");
    setCursorEnabled(s.cursor_enabled !== "false");
    setTel(t);
    setPricingRows(price);
    setCursorStat(cstat);
  }

  useEffect(() => {
    refresh().catch((e: unknown) =>
      setError(e instanceof Error ? e.message : String(e)),
    );
  }, []);

  async function saveBasics() {
    setError(null);
    try {
      await api.setSetting("default_prompt", prompt);
      await api.setSetting("continue_offset_seconds", offset);
      await api.setSetting("notify_only", notifyOnly ? "true" : "false");
      setStatus("Settings saved successfully.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-[var(--color-foreground)]">
          Settings
        </h1>
        <p className="text-xs text-[var(--color-muted-foreground)]">
          Configure prompt auto-continue, telemetry hooks, local scanner, and pricing rates.
        </p>
      </div>

      {status && (
        <div className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 p-3 text-xs text-emerald-400">
          {status}
        </div>
      )}
      {error && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-400">
          {error}
        </div>
      )}

      {/* Grid of Settings Modules */}
      <div className="grid gap-6 md:grid-cols-2">
        {/* Auto-Continue Config */}
        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Zap className="h-4 w-4 text-orange-400" />
              <CardTitle>Auto-Continue Injections</CardTitle>
            </div>
            <CardDescription>
              Prompt delivered automatically to the active terminal upon window reset
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <label className="block space-y-1.5 text-xs">
              <span className="font-semibold">Default Prompt Message</span>
              <Input
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                placeholder="Continue working on the task..."
              />
            </label>
            <label className="block space-y-1.5 text-xs">
              <span className="font-semibold">Reset Buffer Offset (Seconds)</span>
              <Input
                type="number"
                value={offset}
                onChange={(e) => setOffset(e.target.value)}
              />
            </label>
            <label className="flex items-center gap-3 text-xs">
              <Switch checked={notifyOnly} onCheckedChange={setNotifyOnly} />
              <span>Notify-only mode (Send notification without terminal injection)</span>
            </label>
            <Button type="button" onClick={() => void saveBasics()} className="w-full">
              Save Preferences
            </Button>
          </CardContent>
        </Card>

        {/* Claude Telemetry */}
        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Radio className="h-4 w-4 text-sky-400" />
              <CardTitle>Claude Code Telemetry</CardTitle>
            </div>
            <CardDescription>
              OTLP receiver hook for real-time token tracking
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-xs">
            <div className="rounded-xl border border-[var(--color-border)] bg-[var(--color-card-elevated)] p-3 space-y-1">
              <div className="flex items-center justify-between">
                <span className="text-[var(--color-muted-foreground)]">Status</span>
                <span className="font-mono font-semibold text-emerald-400">
                  {tel?.present ? "Configured & Active" : "Not Configured"}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[var(--color-muted-foreground)]">OTLP Port</span>
                <span className="font-mono">{tel?.port || "—"}</span>
              </div>
            </div>

            {tel?.ccswitch_detected && (
              <div className="rounded-xl border border-amber-500/20 bg-amber-500/10 p-3 text-[11px] text-amber-300">
                cc-switch detected. ReCode will preserve and self-heal settings.json hooks.
              </div>
            )}

            <div className="flex gap-2 pt-2">
              <Button
                size="sm"
                className="flex-1"
                onClick={() =>
                  void api
                    .enableTelemetry()
                    .then((r) => {
                      setStatus(`Telemetry enabled on port ${r.port}`);
                      return refresh();
                    })
                    .catch((e: unknown) =>
                      setError(e instanceof Error ? e.message : String(e)),
                    )
                }
              >
                Enable Hook
              </Button>
              <Button
                size="sm"
                variant="outline"
                className="flex-1"
                onClick={() =>
                  void api
                    .disableTelemetry()
                    .then(() => {
                      setStatus("Telemetry disabled");
                      return refresh();
                    })
                    .catch((e: unknown) =>
                      setError(e instanceof Error ? e.message : String(e)),
                    )
                }
              >
                Disable Hook
              </Button>
            </div>
          </CardContent>
        </Card>

        {/* Cursor Usage Local SQLite */}
        <Card className="md:col-span-2">
          <CardHeader>
            <div className="flex items-center gap-2">
              <Database className="h-4 w-4 text-purple-400" />
              <CardTitle>Cursor Usage Scanner (Local SQLite)</CardTitle>
            </div>
            <CardDescription>
              Best-effort read of Cursor&apos;s state.vscdb (bubble tokenCount with composer fallback)
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-xs">
            <div className="flex items-center justify-between">
              <label className="flex items-center gap-3">
                <Switch
                  checked={cursorEnabled}
                  onCheckedChange={(enabled) => {
                    setCursorEnabled(enabled);
                    void api
                      .setSetting("cursor_enabled", enabled ? "true" : "false")
                      .then(() =>
                        enabled ? api.scanCursorNow().then(() => refresh()) : refresh(),
                      );
                  }}
                />
                <span>Enable Cursor local background scan</span>
              </label>
              <Button
                size="sm"
                variant="outline"
                onClick={() =>
                  void api
                    .scanCursorNow()
                    .then((n) => {
                      setStatus(`Cursor scan inserted ${n} row(s)`);
                      return refresh();
                    })
                    .catch((e: unknown) =>
                      setError(e instanceof Error ? e.message : String(e)),
                    )
                }
              >
                Scan Now
              </Button>
            </div>

            {cursorStat && (
              <div className="rounded-xl border border-[var(--color-border)] bg-[var(--color-card-elevated)] p-3 space-y-1">
                <div className="flex justify-between">
                  <span>DB Status: {cursorStat.db_found ? "Found" : "Not Found"}</span>
                  <span>Schema: {cursorStat.schema_ok ? "OK" : "Unknown"}</span>
                </div>
                {cursorStat.db_path && (
                  <p className="font-mono text-[10px] opacity-70 truncate">{cursorStat.db_path}</p>
                )}
                <p className="text-[11px] text-[var(--color-muted-foreground)]">
                  Last insert: {cursorStat.last_inserted} {cursorStat.detail ? `· ${cursorStat.detail}` : ""}
                </p>
              </div>
            )}

            {/* Manual Usage Entry */}
            <div className="grid grid-cols-1 sm:grid-cols-4 gap-2 items-end border-t border-[var(--color-border)] pt-3">
              <label className="space-y-1">
                <span>Manual Model</span>
                <Input
                  value={manualModel}
                  onChange={(e) => setManualModel(e.target.value)}
                  placeholder="cursor-auto"
                />
              </label>
              <label className="space-y-1">
                <span>Input Tokens</span>
                <Input
                  type="number"
                  value={manualIn}
                  onChange={(e) => setManualIn(e.target.value)}
                />
              </label>
              <label className="space-y-1">
                <span>Output Tokens</span>
                <Input
                  type="number"
                  value={manualOut}
                  onChange={(e) => setManualOut(e.target.value)}
                />
              </label>
              <Button
                size="sm"
                variant="outline"
                onClick={() =>
                  void api
                    .setManualUsage(
                      manualModel || "cursor-auto",
                      Number(manualIn) || 0,
                      Number(manualOut) || 0,
                    )
                    .then(() => {
                      setStatus("Manual Cursor usage saved");
                      return refresh();
                    })
                    .catch((e: unknown) =>
                      setError(e instanceof Error ? e.message : String(e)),
                    )
                }
              >
                Add Manual Entry
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Pricing Table */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <DollarSign className="h-4 w-4 text-emerald-400" />
            <CardTitle>Model Pricing Matrix (USD per 1M Tokens)</CardTitle>
          </div>
          <CardDescription>
            Configure rates to accurately estimate costs across different LLM providers
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          {pricing.length === 0 ? (
            <div className="p-6 text-center text-xs text-[var(--color-muted-foreground)]">
              No pricing rows defined.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Model Identifier</TableHead>
                  <TableHead>Input ($/1M)</TableHead>
                  <TableHead>Output ($/1M)</TableHead>
                  <TableHead>Cache Read ($/1M)</TableHead>
                  <TableHead>Cache Write ($/1M)</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {pricing.map((p) => (
                  <TableRow key={p.model}>
                    <TableCell className="font-mono font-medium text-xs text-[var(--color-foreground)]">
                      {p.model}
                    </TableCell>
                    {(
                      [
                        ["input_pm", p.input_pm],
                        ["output_pm", p.output_pm],
                        ["cache_read_pm", p.cache_read_pm],
                        ["cache_write_pm", p.cache_write_pm],
                      ] as const
                    ).map(([key, val]) => (
                      <TableCell key={key}>
                        <Input
                          type="number"
                          step="0.01"
                          aria-label={`${p.model} ${key}`}
                          defaultValue={val}
                          className="h-8 font-mono text-xs max-w-[7rem]"
                          onBlur={(e) => {
                            const n = Number(e.target.value);
                            if (!Number.isFinite(n)) return;
                            const next = { ...p, [key]: n };
                            void api
                              .setPricing(p.model, {
                                input_pm: next.input_pm,
                                output_pm: next.output_pm,
                                cache_read_pm: next.cache_read_pm,
                                cache_write_pm: next.cache_write_pm,
                              })
                              .then(refresh);
                          }}
                        />
                      </TableCell>
                    ))}
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default SettingsPage;
