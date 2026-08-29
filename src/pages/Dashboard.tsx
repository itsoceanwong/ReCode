import { useCallback, useEffect, useMemo, useState } from "react";
import { api, onAutocontinueFired, onLimitsUpdated } from "@/lib/api";
import type { AutocontinueLogEntry, DashboardData } from "@/lib/types";
import { LimitCard } from "@/components/LimitCard";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

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
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Dashboard</h1>
        <p className="text-sm text-[var(--color-muted-foreground)]">
          Rate limits, reset countdowns, and today&apos;s spend.
        </p>
      </div>

      {error && (
        <p className="text-sm text-[var(--color-destructive)]">{error}</p>
      )}

      <div className="grid gap-4 sm:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Today</CardTitle>
            <CardDescription>Local totals across all sources</CardDescription>
          </CardHeader>
          <CardContent className="space-y-1">
            <p className="text-2xl font-semibold">
              {(data?.summary.today_tokens ?? 0).toLocaleString()} tokens
            </p>
            <p className="text-sm text-[var(--color-muted-foreground)]">
              ${(data?.summary.today_cost_usd ?? 0).toFixed(4)}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Auto-continue</CardTitle>
            <CardDescription>Recent injection attempts</CardDescription>
          </CardHeader>
          <CardContent>
            <LogList entries={data?.recent_autocontinue ?? []} />
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        {bySource.size === 0 && (
          <p className="text-sm text-[var(--color-muted-foreground)]">
            No limit windows yet. Enable Claude telemetry/statusline or wait for Codex snapshots.
          </p>
        )}
        {[...bySource.entries()].map(([source, windows]) => (
          <LimitCard
            key={source}
            source={source}
            windows={windows}
            onSetManual={async (kind, resetsAt, used) => {
              await api.setManualLimit(source, kind, resetsAt, used ?? null);
              refresh();
            }}
            onClearManual={async (kind) => {
              await api.clearManualLimit(source, kind);
              refresh();
            }}
          />
        ))}
      </div>
    </div>
  );
}

function LogList({ entries }: { entries: AutocontinueLogEntry[] }) {
  if (entries.length === 0) {
    return <p className="text-sm text-[var(--color-muted-foreground)]">No events yet.</p>;
  }
  return (
    <ul className="max-h-40 space-y-1 overflow-auto text-xs">
      {entries.map((e) => (
        <li key={e.id} className="flex justify-between gap-2">
          <span className="truncate">
            {e.status}
            {e.target ? ` → ${e.target}` : ""}
          </span>
          <span className="shrink-0 text-[var(--color-muted-foreground)]">
            {new Date(e.fired_at * 1000).toLocaleTimeString()}
          </span>
        </li>
      ))}
    </ul>
  );
}
