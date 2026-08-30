import { formatDistanceToNowStrict } from "date-fns";
import type { LimitWindow } from "@/lib/types";
import { Clock, ShieldAlert } from "lucide-react";
import { cn } from "@/lib/utils";

export function CountdownBadge({ resetsAt }: { resetsAt: number | null }) {
  if (resetsAt == null) {
    return (
      <span className="rounded-full border border-[var(--color-border)] px-2.5 py-0.5 text-[11px] text-[var(--color-muted-foreground)]">
        no reset
      </span>
    );
  }
  const ms = resetsAt * 1000;
  const past = ms < Date.now();
  const label = past
    ? `reset ${formatDistanceToNowStrict(ms)} ago`
    : `in ${formatDistanceToNowStrict(ms)}`;
  return (
    <span className="inline-flex items-center gap-1 rounded-full border border-orange-500/30 bg-orange-500/10 px-2.5 py-0.5 text-[11px] font-medium text-orange-400">
      <Clock className="h-3 w-3" />
      {label}
    </span>
  );
}

export function LimitCard({
  source,
  windows,
}: {
  source: string;
  windows: LimitWindow[];
}) {
  const five = windows.find((w) => w.window_kind === "five_hour");
  const seven = windows.find((w) => w.window_kind === "seven_day");

  return (
    <div className="rounded-2xl border border-[var(--color-border)] bg-[var(--color-card)] p-5 shadow-lg shadow-black/10">
      <div className="mb-4 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-orange-500/15 text-orange-400">
            <ShieldAlert className="h-4 w-4" />
          </div>
          <h3 className="text-sm font-bold tracking-tight capitalize">{source} Rate Limits</h3>
        </div>
      </div>
      <div className="space-y-4">
        <WindowRow label="5-Hour Window" win={five} />
        <div className="border-t border-[var(--color-border)]" />
        <WindowRow label="7-Day Window" win={seven} />
      </div>
    </div>
  );
}

function WindowRow({ label, win }: { label: string; win?: LimitWindow }) {
  const pct = win?.used_percent ?? null;
  const safePct = Math.min(100, Math.max(0, pct ?? 0));

  const isDanger = safePct >= 85;
  const isWarn = safePct >= 65 && !isDanger;

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs font-semibold text-[var(--color-foreground)]">{label}</span>
        <CountdownBadge resetsAt={win?.resets_at ?? null} />
      </div>

      {/* Glowing Progress Bar */}
      <div className="relative h-2.5 overflow-hidden rounded-full bg-[var(--color-muted)]">
        <div
          className={cn(
            "h-full rounded-full transition-all duration-500",
            isDanger
              ? "bg-gradient-to-r from-rose-500 to-rose-400 shadow-[0_0_12px_#f43f5e]"
              : isWarn
                ? "bg-gradient-to-r from-amber-500 to-orange-400 shadow-[0_0_12px_#f59e0b]"
                : "bg-gradient-to-r from-orange-500 to-amber-400 shadow-[0_0_12px_#f97316]",
          )}
          style={{ width: `${safePct}%` }}
        />
      </div>

      <div className="flex items-center justify-between text-[11px] text-[var(--color-muted-foreground)]">
        <span>{pct != null ? `${pct.toFixed(1)}% consumed` : "no data"}</span>
        <span>{pct != null ? `${(100 - safePct).toFixed(1)}% remaining` : "—"}</span>
      </div>
    </div>
  );
}
