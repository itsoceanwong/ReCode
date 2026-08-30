import { useEffect, useMemo, useState } from "react";
import { endOfDay, startOfDay, subDays } from "date-fns";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { api, onAutocontinueFired, onLimitsUpdated } from "@/lib/api";
import type { InjectionTarget, LimitWindow, SessionView } from "@/lib/types";
import { Terminal, Send, Clock } from "lucide-react";
import { cn } from "@/lib/utils";

type ToolFilter = "all" | "codex" | "claude";
type DateFilter = "today" | "7d" | "30d" | "all";

function formatLastSeen(ts: number | null): string | null {
  if (ts == null) return null;
  try {
    return new Date(ts * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
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
  if (fireAt == null) return "No reset window";
  const when = new Date(fireAt * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  if (fireAt > nowSec) return `Scheduled at ${when}`;
  return `Due at ${when}`;
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
    onLimitsUpdated(() => void refresh()).then((fn) => unsubs.push(fn));
    onAutocontinueFired(() => void refresh()).then((fn) => unsubs.push(fn));
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
    <div className="space-y-6">
      {/* Top Header & Filter Bar */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-[var(--color-foreground)]">
            Discovered Sessions
          </h1>
          <p className="text-xs text-[var(--color-muted-foreground)]">
            Auto-continue prompt injection bindings and running terminal instances.
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          {/* Tool Filter */}
          <div className="flex rounded-xl border border-[var(--color-border)] bg-[var(--color-card)] p-1">
            {(["all", "codex", "claude"] as const).map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => setTool(t)}
                className={cn(
                  "rounded-lg px-3 py-1 text-xs font-semibold capitalize transition-all",
                  tool === t
                    ? "bg-orange-500 text-white shadow-sm shadow-orange-500/30"
                    : "text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]",
                )}
              >
                {t === "all" ? "All Tools" : t}
              </button>
            ))}
          </div>

          {/* Project Filter */}
          <select
            className="h-8 rounded-xl border border-[var(--color-border)] bg-[var(--color-card)] px-3 text-xs font-medium text-[var(--color-foreground)] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-orange-500"
            value={project}
            onChange={(e) => setProject(e.target.value)}
          >
            <option value="all">All Projects</option>
            {projects.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>

          {/* Date Filter */}
          <div className="flex rounded-xl border border-[var(--color-border)] bg-[var(--color-card)] p-1">
            {(
              [
                ["today", "Today"],
                ["7d", "7D"],
                ["30d", "30D"],
                ["all", "All"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                onClick={() => setDate(id)}
                className={cn(
                  "rounded-lg px-3 py-1 text-xs font-semibold transition-all",
                  date === id
                    ? "bg-orange-500 text-white shadow-sm shadow-orange-500/30"
                    : "text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]",
                )}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {error && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-400">
          {error}
        </div>
      )}
      {status && (
        <div className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 p-3 text-xs text-emerald-400">
          {status}
        </div>
      )}

      {/* Session Cards Stream */}
      <div className="space-y-3">
        {sessions.length === 0 ? (
          <div className="rounded-2xl border border-[var(--color-border)] bg-[var(--color-card)] p-12 text-center text-xs text-[var(--color-muted-foreground)]">
            No agent sessions discovered yet. Run Claude Code or Codex in your project terminals.
          </div>
        ) : filtered.length === 0 ? (
          <div className="rounded-2xl border border-[var(--color-border)] bg-[var(--color-card)] p-12 text-center text-xs text-[var(--color-muted-foreground)]">
            No sessions match the selected filters.
          </div>
        ) : (
          filtered.map((s) => (
            <div
              key={s.id}
              className="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-sm transition-all hover:border-orange-500/30 hover:bg-[var(--color-card-elevated)]"
            >
              <div className="flex items-center gap-3.5 min-w-[14rem]">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-orange-500/15 text-orange-400 shrink-0">
                  <Terminal className="h-5 w-5" />
                </div>
                <div>
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-sm text-[var(--color-foreground)]">
                      {s.display_name}
                    </span>
                    <span className="rounded-full bg-orange-500/10 px-2 py-0.5 text-[10px] font-mono text-orange-400 border border-orange-500/20 capitalize">
                      {s.source}
                    </span>
                  </div>
                  <div className="mt-1 flex items-center gap-2 text-xs text-[var(--color-muted-foreground)]">
                    <span className="font-mono text-[11px] truncate max-w-xs">{s.cwd || "—"}</span>
                    <span>·</span>
                    <span>Last active {formatLastSeen(s.last_seen) || "—"}</span>
                  </div>
                  <div className="mt-1 flex items-center gap-1.5 text-[11px] text-orange-400/90 font-mono">
                    <Clock className="h-3 w-3" />
                    <span>{formatContinueAt(continueAtUnix(s.source, limits, offsetSeconds), nowSec)}</span>
                  </div>
                </div>
              </div>

              {/* Controls */}
              <div className="flex items-center gap-3">
                <div className="flex items-center gap-2 mr-2">
                  <span className="text-xs text-[var(--color-muted-foreground)]">Auto-Continue</span>
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
                </div>

                <select
                  className="h-9 max-w-[14rem] rounded-xl border border-[var(--color-border)] bg-[var(--color-card-elevated)] px-3 text-xs text-[var(--color-foreground)] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-orange-500"
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
                  <option value="">Select Injection Target…</option>
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
                      .then((o) => setStatus(`Test sent to target: ${JSON.stringify(o)}`));
                  }}
                  className="gap-1.5"
                >
                  <Send className="h-3 w-3" />
                  Test
                </Button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
