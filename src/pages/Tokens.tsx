import { useCallback, useEffect, useState } from "react";
import { endOfDay, startOfDay, subDays } from "date-fns";
import { api, onUsageUpdated } from "@/lib/api";
import type { GroupBy, UsageAggregate } from "@/lib/types";
import { TokenTable } from "@/components/TokenTable";
import { UsageChart } from "@/components/UsageChart";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";

export default function Tokens() {
  const [groupBy, setGroupBy] = useState<GroupBy>("model");
  const [rows, setRows] = useState<UsageAggregate[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [days, setDays] = useState(7);

  const refresh = useCallback(() => {
    const to = Math.floor(endOfDay(new Date()).getTime() / 1000);
    const from = Math.floor(
      startOfDay(subDays(new Date(), days - 1)).getTime() / 1000,
    );
    api
      .getUsage({ from, to }, groupBy)
      .then(setRows)
      .catch((e: unknown) => setError(String(e)));
  }, [days, groupBy]);

  useEffect(() => {
    refresh();
    let unlisten: (() => void) | undefined;
    onUsageUpdated(refresh).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [refresh]);

  return (
    <div className="space-y-6">
      {/* Top Header & Filters */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-[var(--color-foreground)]">
            Tokens & Costs
          </h1>
          <p className="text-xs text-[var(--color-muted-foreground)]">
            Detailed token telemetry, breakdown by models, and real-time cost estimation.
          </p>
        </div>

        {/* Segmented Controls */}
        <div className="flex flex-wrap items-center gap-2">
          {/* Days Filter */}
          <div className="flex rounded-xl border border-[var(--color-border)] bg-[var(--color-card)] p-1">
            {[1, 7, 30].map((d) => (
              <button
                key={d}
                type="button"
                onClick={() => setDays(d)}
                className={cn(
                  "rounded-lg px-3 py-1 text-xs font-semibold transition-all",
                  days === d
                    ? "bg-orange-500 text-white shadow-sm shadow-orange-500/30"
                    : "text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]",
                )}
              >
                {d}D
              </button>
            ))}
          </div>

          {/* GroupBy Filter */}
          <div className="flex rounded-xl border border-[var(--color-border)] bg-[var(--color-card)] p-1">
            {(["model", "app"] as const).map((g) => (
              <button
                key={g}
                type="button"
                onClick={() => setGroupBy(g)}
                className={cn(
                  "rounded-lg px-3 py-1 text-xs font-semibold capitalize transition-all",
                  groupBy === g
                    ? "bg-orange-500 text-white shadow-sm shadow-orange-500/30"
                    : "text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]",
                )}
              >
                By {g}
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

      {/* Chart Card */}
      <Card>
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Usage Distribution</CardTitle>
              <p className="mt-1 text-xs text-[var(--color-muted-foreground)]">
                Input, output, and cached token curves over selected timeframe
              </p>
            </div>
            <div className="flex items-center gap-3 text-xs font-mono">
              <span className="flex items-center gap-1.5 text-sky-400">
                <span className="h-2 w-2 rounded-full bg-sky-400" /> Input
              </span>
              <span className="flex items-center gap-1.5 text-emerald-400">
                <span className="h-2 w-2 rounded-full bg-emerald-400" /> Output
              </span>
              <span className="flex items-center gap-1.5 text-orange-400">
                <span className="h-2 w-2 rounded-full bg-orange-400" /> Cache
              </span>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <UsageChart rows={rows} />
        </CardContent>
      </Card>

      {/* Table Card */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle>Telemetry Breakdown</CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          <TokenTable rows={rows} />
        </CardContent>
      </Card>
    </div>
  );
}
