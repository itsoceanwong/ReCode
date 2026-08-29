import { useCallback, useEffect, useState } from "react";
import { endOfDay, startOfDay, subDays } from "date-fns";
import { api, onUsageUpdated } from "@/lib/api";
import type { GroupBy, UsageAggregate } from "@/lib/types";
import { TokenTable } from "@/components/TokenTable";
import { UsageChart } from "@/components/UsageChart";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

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
    <div className="space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Tokens</h1>
          <p className="text-sm text-[var(--color-muted-foreground)]">
            Per-app and per-model usage with cost estimates.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          {[1, 7, 30].map((d) => (
            <Button
              key={d}
              size="sm"
              variant={days === d ? "default" : "outline"}
              onClick={() => setDays(d)}
            >
              {d}d
            </Button>
          ))}
          <Button
            size="sm"
            variant={groupBy === "app" ? "default" : "outline"}
            onClick={() => setGroupBy("app")}
          >
            By app
          </Button>
          <Button
            size="sm"
            variant={groupBy === "model" ? "default" : "outline"}
            onClick={() => setGroupBy("model")}
          >
            By model
          </Button>
        </div>
      </div>

      {error && (
        <p className="text-sm text-[var(--color-destructive)]">{error}</p>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Chart</CardTitle>
        </CardHeader>
        <CardContent>
          <UsageChart rows={rows} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Table</CardTitle>
        </CardHeader>
        <CardContent>
          <TokenTable rows={rows} />
        </CardContent>
      </Card>
    </div>
  );
}
