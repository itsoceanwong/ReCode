import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import type {
  CursorStatus,
  InjectionTarget,
  PricingRow,
  SessionView,
  TelemetryStatus,
} from "@/lib/types";

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
  const [sessions, setSessions] = useState<SessionView[]>([]);
  const [targets, setTargets] = useState<InjectionTarget[]>([]);
  const [pricing, setPricingRows] = useState<PricingRow[]>([]);

  async function refresh() {
    const [s, t, sess, price, cstat] = await Promise.all([
      api.getSettings(),
      api.telemetryStatus(),
      api.getSessions(),
      api.getPricing(),
      api.cursorStatus().catch(() => null),
    ]);
    setPrompt(s.default_prompt ?? "");
    setOffset(s.continue_offset_seconds ?? "120");
    setNotifyOnly(s.notify_only === "true");
    setCursorEnabled(s.cursor_enabled !== "false");
    setTel(t);
    setSessions(sess);
    setPricingRows(price);
    setCursorStat(cstat);
    try {
      setTargets(await api.listInjectionTargets());
    } catch {
      setTargets([]);
    }
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
      setStatus("Saved.");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>Continue prompt</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 max-w-xl">
          <label className="block space-y-1.5 text-sm">
            <span className="font-medium">Default prompt</span>
            <Input value={prompt} onChange={(e) => setPrompt(e.target.value)} />
          </label>
          <label className="block space-y-1.5 text-sm">
            <span className="font-medium">Offset (seconds after reset)</span>
            <Input
              type="number"
              value={offset}
              onChange={(e) => setOffset(e.target.value)}
            />
          </label>
          <label className="flex items-center gap-3 text-sm">
            <Switch checked={notifyOnly} onCheckedChange={setNotifyOnly} />
            Notify-only mode (do not inject)
          </label>
          <Button type="button" onClick={() => void saveBasics()}>
            Save
          </Button>
          {status && (
            <p className="text-sm text-[var(--color-primary)]">{status}</p>
          )}
          {error && (
            <p className="text-sm text-[var(--color-destructive)]">{error}</p>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Claude telemetry</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm max-w-xl">
          <p>
            Status:{" "}
            {tel?.present ? "configured in settings.json" : "not present"} ·
            port {tel?.port || "—"}
          </p>
          {tel?.ccswitch_detected && (
            <p className="rounded-md bg-[var(--color-muted)] p-3 text-xs">
              cc-switch detected. Add the same OTEL env keys to its Common
              Config Snippet so provider switches do not wipe telemetry. ReCode
              will self-heal ~/.claude/settings.json when possible.
            </p>
          )}
          <div className="flex gap-2">
            <Button
              size="sm"
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
              Enable
            </Button>
            <Button
              size="sm"
              variant="outline"
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
              Disable
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Cursor usage (local SQLite)</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm max-w-xl">
          <p className="text-xs text-[var(--color-muted-foreground)]">
            Best-effort read of Cursor&apos;s state.vscdb (bubble tokenCount, with
            composer context-meter fallback). Marked approximate. Does not read
            auth tokens.
          </p>
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
            Enable Cursor local scan
          </label>
          {cursorStat && (
            <div className="rounded-md bg-[var(--color-muted)] p-3 text-xs space-y-1">
              <p>DB: {cursorStat.db_found ? "found" : "not found"}</p>
              {cursorStat.db_path && (
                <p className="break-all opacity-80">{cursorStat.db_path}</p>
              )}
              <p>
                Schema: {cursorStat.schema_ok ? "ok" : "unknown"} · last insert:{" "}
                {cursorStat.last_inserted}
              </p>
              {cursorStat.detail && <p>{cursorStat.detail}</p>}
            </div>
          )}
          <div className="flex flex-wrap gap-2">
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
              Scan now
            </Button>
          </div>
          <div className="grid grid-cols-3 gap-2 items-end border-t border-[var(--color-border)] pt-3">
            <label className="space-y-1 text-xs col-span-1">
              <span>Manual model</span>
              <Input
                value={manualModel}
                onChange={(e) => setManualModel(e.target.value)}
              />
            </label>
            <label className="space-y-1 text-xs">
              <span>Input</span>
              <Input
                type="number"
                value={manualIn}
                onChange={(e) => setManualIn(e.target.value)}
              />
            </label>
            <label className="space-y-1 text-xs">
              <span>Output</span>
              <Input
                type="number"
                value={manualOut}
                onChange={(e) => setManualOut(e.target.value)}
              />
            </label>
            <Button
              size="sm"
              className="col-span-3"
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
              Add manual usage row
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Sessions / auto-continue</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {sessions.length === 0 ? (
            <p className="text-sm text-[var(--color-muted-foreground)]">
              No sessions discovered yet.
            </p>
          ) : (
            sessions.map((s) => (
              <div
                key={s.id}
                className="flex flex-wrap items-center gap-3 border-b border-[var(--color-border)] py-2 text-sm"
              >
                <div className="min-w-[12rem] grow">
                  <div className="font-medium">
                    {s.source} · {s.model || "unknown"}
                  </div>
                  <div className="text-xs text-[var(--color-muted-foreground)]">
                    {s.cwd || s.id}
                  </div>
                </div>
                <Switch
                  checked={s.auto_continue_enabled}
                  onCheckedChange={(enabled) =>
                    void api.setSessionAutocontinue(s.id, enabled).then(refresh)
                  }
                />
                <select
                  className="h-9 max-w-[16rem] rounded-md border border-[var(--color-border)] bg-[var(--color-card)] px-2 text-xs"
                  value={s.target_ref ?? ""}
                  onChange={(e) => {
                    const reference = e.target.value;
                    const t = targets.find((x) => x.reference === reference);
                    if (!t) return;
                    void api
                      .setSessionAutocontinue(
                        s.id,
                        s.auto_continue_enabled,
                        s.continue_prompt,
                        t,
                      )
                      .then(refresh);
                  }}
                >
                  <option value="">Injection target…</option>
                  {targets.map((t) => (
                    <option key={t.reference} value={t.reference}>
                      {t.reference}
                    </option>
                  ))}
                </select>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={!s.target_ref}
                  onClick={() => {
                    const t = targets.find((x) => x.reference === s.target_ref);
                    if (!t) return;
                    void api
                      .testInjection(t, "ReCode test message")
                      .then((o) => setStatus(`Test: ${JSON.stringify(o)}`));
                  }}
                >
                  Send test
                </Button>
              </div>
            ))
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Pricing (USD / 1M tokens)</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          {pricing.map((p) => (
            <div
              key={p.model}
              className="grid grid-cols-5 gap-2 text-xs items-center"
            >
              <span className="font-medium truncate">{p.model}</span>
              {(
                [
                  ["input_pm", p.input_pm],
                  ["output_pm", p.output_pm],
                  ["cache_read_pm", p.cache_read_pm],
                  ["cache_write_pm", p.cache_write_pm],
                ] as const
              ).map(([key, val]) => (
                <Input
                  key={key}
                  type="number"
                  step="0.01"
                  defaultValue={val}
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
              ))}
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

export default SettingsPage;
