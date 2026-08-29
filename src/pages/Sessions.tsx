import { useEffect, useMemo, useState } from "react";
import { endOfDay, startOfDay, subDays } from "date-fns";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { api, onAutocontinueFired, onLimitsUpdated } from "@/lib/api";
import type { InjectionTarget, LimitWindow, SessionView } from "@/lib/types";

type ToolFilter = "all" | "codex" | "claude";
type DateFilter = "today" | "7d" | "30d" | "all";

function formatLastSeen(ts: number | null): string | null {
  if (ts == null) return null;
  try {
    return new Date(ts * 1000).toLocaleString();
  } catch {
    return null;
  }
}

function inDateFilter(lastSeen: number | null, filter: DateFilter): boolean {
  if (filter === "all") return true;
  if (lastSeen == null) return false;
  const to = Math.floor(endOfDay(new Date()).getTime() / 1000);
  const days = filter === "today" ? 1 : filter === "7d" ? 7 : 30;
  const from = Math.floor(startOfDay(subDays(new Date(), days - 1)).getTime() / 1000);
  return lastSeen >= from && lastSeen <= to;
}

/** Same rule as scheduler: earliest five_hour resets_at for source + offset. */
function continueAtUnix(
  source: string,
  limits: LimitWindow[],
  offsetSeconds: number,
): number | null {
  const resets = limits
    .filter((l) => l.source === source && l.window_kind === "five_hour")
    .map((l) => l.resets_at)
    .filter((t): t is number => t != null);
  if (resets.length === 0) return null;
  return Math.min(...resets) + offsetSeconds;
}

function formatContinueAt(fireAt: number | null, nowSec: number): string {
  if (fireAt == null) return "Continue: —";
  const when = new Date(fireAt * 1000).toLocaleString();
  if (fireAt > nowSec) return `Continue: ${when}`;
  return `Continue due: ${when}`;
}

export default function Sessions() {
  const [sessions, setSessions] = useState<SessionView[]>([]);
  const [targets, setTargets] = useState<InjectionTarget[]>([]);
  const [limits, setLimits] = useState<LimitWindow[]>([]);
  const [offsetSeconds, setOffsetSeconds] = useState(120);
  const [nowSec, setNowSec] = useState(() => Math.floor(Date.now() / 1000));
  const [tool, setTool] = useState<ToolFilter>("all");
  const [project, setProject] = useState<string>("all");
  const [date, setDate] = useState<DateFilter>("today");
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    const [sess, settings, dash] = await Promise.all([
      api.getSessions(),
      api.getSettings(),
      api.getDashboard(),
    ]);
    setSessions(sess);
    setLimits(dash.limits);
    const parsed = Number(settings.continue_offset_seconds ?? "120");
    setOffsetSeconds(Number.isFinite(parsed) ? parsed : 120);
    setNowSec(Math.floor(Date.now() / 1000));
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
    const unsubs: Array<() => void> = [];
    onLimitsUpdated(() => {
      void refresh();
    }).then((fn) => unsubs.push(fn));
    onAutocontinueFired(() => {
      void refresh();
    }).then((fn) => unsubs.push(fn));
    const id = window.setInterval(() => {
      setNowSec(Math.floor(Date.now() / 1000));
      void refresh().catch(() => {});
    }, 15_000);
    return () => {
      unsubs.forEach((u) => u());
      window.clearInterval(id);
    };
  }, []);
  const projects = useMemo(() => {
    const set = new Set<string>();
    for (const s of sessions) {
      if (s.project) set.add(s.project);
    }
    return Array.from(set).sort((a, b) => a.localeCompare(b));
  }, [sessions]);

  const filtered = useMemo(() => {
    return sessions.filter((s) => {
      if (tool !== "all" && s.source !== tool) return false;
      if (project !== "all" && s.project !== project) return false;
      if (!inDateFilter(s.last_seen, date)) return false;
      return true;
    });
  }, [sessions, tool, project, date]);

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Sessions</h1>
          <p className="text-sm text-[var(--color-muted-foreground)]">
            Manage discovered sessions and auto-continue.
          </p>
        </div>
        <div className="flex flex-wrap gap-2 items-center">
          {(["all", "codex", "claude"] as const).map((t) => (
            <Button
              key={t}
              size="sm"
              variant={tool === t ? "default" : "outline"}
              onClick={() => setTool(t)}
            >
              {t === "all" ? "All" : t === "codex" ? "Codex" : "Claude Code"}
            </Button>
          ))}
          <select
            className="h-9 max-w-[12rem] rounded-md border border-[var(--color-border)] bg-[var(--color-card)] px-2 text-xs"
            value={project}
            onChange={(e) => setProject(e.target.value)}
          >
            <option value="all">All projects</option>
            {projects.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
          {(
            [
              ["today", "Today"],
              ["7d", "7d"],
              ["30d", "30d"],
              ["all", "All"],
            ] as const
          ).map(([id, label]) => (
            <Button
              key={id}
              size="sm"
              variant={date === id ? "default" : "outline"}
              onClick={() => setDate(id)}
            >
              {label}
            </Button>
          ))}
        </div>
      </div>

      {error && (
        <p className="text-sm text-[var(--color-destructive)]">{error}</p>
      )}
      {status && (
        <p className="text-sm text-[var(--color-primary)]">{status}</p>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Sessions / auto-continue</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {sessions.length === 0 ? (
            <p className="text-sm text-[var(--color-muted-foreground)]">
              No sessions discovered yet.
            </p>
          ) : filtered.length === 0 ? (
            <p className="text-sm text-[var(--color-muted-foreground)]">
              No sessions match these filters.
            </p>
          ) : (
            filtered.map((s) => (
              <div
                key={s.id}
                className="flex flex-wrap items-center gap-3 border-b border-[var(--color-border)] py-2 text-sm"
              >
                <div className="min-w-[12rem] grow">
                  <div className="font-medium">{s.display_name}</div>
                  <div className="text-xs text-[var(--color-muted-foreground)]">
                    {[s.model, s.cwd, formatLastSeen(s.last_seen)]
                      .filter(Boolean)
                      .join(" · ")}
                  </div>
                  <div className="text-xs text-[var(--color-muted-foreground)]">
                    {formatContinueAt(
                      continueAtUnix(s.source, limits, offsetSeconds),
                      nowSec,
                    )}
                  </div>
                </div>
                <Switch
                  checked={s.auto_continue_enabled}
                  onCheckedChange={(enabled) =>
                    void api
                      .setSessionAutocontinue(s.id, enabled)
                      .then(refresh)
                      .catch((e: unknown) =>
                        setError(e instanceof Error ? e.message : String(e)),
                      )
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
    </div>
  );
}
