import { useCallback, useEffect, useMemo, useState } from "react";
import { api, onAutocontinueFired, onLimitsUpdated } from "@/lib/api";
import type { AutocontinueLogEntry, DashboardData } from "@/lib/types";
import { LimitCard } from "@/components/LimitCard";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Coins,
  DollarSign,
  Zap,
  Activity,
} from "lucide-react";

export default function Dashboard() {
  const [data, setData] = useState<DashboardData | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(() => {
    api
      .getDashboard()
      .then(setData)
      .catch((e: unknown) => setError(String(e)));
  }, []);

  useEffect(() => {
    refresh();
    const unsubs: Array<() => void> = [];
    onLimitsUpdated(refresh).then((fn) => unsubs.push(fn));
    onAutocontinueFired(refresh).then((fn) => unsubs.push(fn));
    const id = window.setInterval(refresh, 15_000);
    return () => {
      unsubs.forEach((u) => u());
      window.clearInterval(id);
    };
  }, [refresh]);

  const bySource = useMemo(() => {
    const map = new Map<string, typeof data extends null ? never : NonNullable<typeof data>["limits"]>();
    for (const w of data?.limits ?? []) {
      const list = map.get(w.source) ?? [];
      list.push(w);
      map.set(w.source, list);
    }
    return map;
  }, [data]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-bold tracking-tight text-[var(--color-foreground)]">
          Dashboard
        </h1>
        <p className="text-xs text-[var(--color-muted-foreground)]">
          Live agent consumption, rate limit countdowns, and automated injections.
        </p>
      </div>

      {error && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-400">
          {error}
        </div>
      )}

      {/* 4-Card Hero Stats Grid */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {/* Today's Tokens */}
        <Card className="relative overflow-hidden border-orange-500/20 bg-gradient-to-br from-[var(--color-card)] to-[var(--color-card-elevated)]">
          <CardContent className="p-5">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-[var(--color-muted-foreground)]">Today's Tokens</span>
              <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-orange-500/15 text-orange-400">
                <Coins className="h-4 w-4" />
              </div>
            </div>
            <div className="mt-3">
              <div className="text-2xl font-extrabold font-mono tracking-tight text-[var(--color-foreground)]">
                {(data?.summary.today_tokens ?? 0).toLocaleString()}
              </div>
              <p className="mt-1 text-[11px] text-[var(--color-muted-foreground)]">
                Aggregated local agent usage
              </p>
            </div>
          </CardContent>
        </Card>

        {/* Today's Cost */}
        <Card className="relative overflow-hidden border-emerald-500/20 bg-gradient-to-br from-[var(--color-card)] to-[var(--color-card-elevated)]">
          <CardContent className="p-5">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-[var(--color-muted-foreground)]">Est. Today's Cost</span>
              <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-emerald-500/15 text-emerald-400">
                <DollarSign className="h-4 w-4" />
              </div>
            </div>
            <div className="mt-3">
              <div className="text-2xl font-extrabold font-mono tracking-tight text-emerald-400">
                ${(data?.summary.today_cost_usd ?? 0).toFixed(4)}
              </div>
              <p className="mt-1 text-[11px] text-[var(--color-muted-foreground)]">
                Calculated by model pricing seed
              </p>
            </div>
          </CardContent>
        </Card>

        {/* Active Limits */}
        <Card className="relative overflow-hidden border-sky-500/20 bg-gradient-to-br from-[var(--color-card)] to-[var(--color-card-elevated)]">
          <CardContent className="p-5">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-[var(--color-muted-foreground)]">Monitored Limits</span>
              <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-sky-500/15 text-sky-400">
                <Activity className="h-4 w-4" />
              </div>
            </div>
            <div className="mt-3">
              <div className="text-2xl font-extrabold font-mono tracking-tight text-sky-400">
                {bySource.size} Sources
              </div>
              <p className="mt-1 text-[11px] text-[var(--color-muted-foreground)]">
                Claude, Codex & Snapshots
              </p>
            </div>
          </CardContent>
        </Card>

        {/* Auto Continue Status */}
        <Card className="relative overflow-hidden border-purple-500/20 bg-gradient-to-br from-[var(--color-card)] to-[var(--color-card-elevated)]">
          <CardContent className="p-5">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-[var(--color-muted-foreground)]">Auto-Continue</span>
              <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-purple-500/15 text-purple-400">
                <Zap className="h-4 w-4" />
              </div>
            </div>
            <div className="mt-3">
              <div className="text-2xl font-extrabold tracking-tight text-purple-400">
                Active
              </div>
              <p className="mt-1 text-[11px] text-[var(--color-muted-foreground)]">
                Auto-inject on window reset
              </p>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Rate Limits Grid */}
      <div>
        <h2 className="mb-3 text-sm font-semibold tracking-tight text-[var(--color-foreground)]">
          Rate Limit Windows
        </h2>
        {bySource.size === 0 ? (
          <div className="rounded-2xl border border-[var(--color-border)] bg-[var(--color-card)] p-8 text-center text-xs text-[var(--color-muted-foreground)]">
            No limit windows discovered yet. Start Claude Code with telemetry or trigger Codex snapshots.
          </div>
        ) : (
          <div className="grid gap-4 md:grid-cols-2">
            {[...bySource.entries()].map(([source, windows]) => (
              <LimitCard key={source} source={source} windows={windows} />
            ))}
          </div>
        )}
      </div>

      {/* Activity Log */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between pb-3">
          <div>
            <CardTitle>Recent Auto-Continue Activity</CardTitle>
            <p className="mt-1 text-xs text-[var(--color-muted-foreground)]">
              Latest prompt injection attempts to terminal sessions
            </p>
          </div>
          <Zap className="h-4 w-4 text-orange-400" />
        </CardHeader>
        <CardContent>
          <LogList entries={data?.recent_autocontinue ?? []} />
        </CardContent>
      </Card>
    </div>
  );
}

function LogList({ entries }: { entries: AutocontinueLogEntry[] }) {
  if (entries.length === 0) {
    return (
      <div className="py-6 text-center text-xs text-[var(--color-muted-foreground)]">
        No auto-continue events recorded yet.
      </div>
    );
  }
  return (
    <div className="divide-y divide-[var(--color-border)] overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-card-elevated)] text-xs">
      {entries.map((e) => (
        <div key={e.id} className="flex items-center justify-between p-3.5 transition-colors hover:bg-[var(--color-muted)]/40">
          <div className="flex items-center gap-3">
            <span className="flex h-2 w-2 rounded-full bg-emerald-400" />
            <div>
              <span className="font-semibold text-[var(--color-foreground)]">{e.status}</span>
              {e.target && (
                <span className="ml-2 font-mono text-[11px] text-[var(--color-muted-foreground)]">
                  → {e.target}
                </span>
              )}
            </div>
          </div>
          <span className="font-mono text-[11px] text-[var(--color-muted-foreground)]">
            {new Date(e.fired_at * 1000).toLocaleTimeString()}
          </span>
        </div>
      ))}
    </div>
  );
}
