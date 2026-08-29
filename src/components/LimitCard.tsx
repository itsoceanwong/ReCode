import { formatDistanceToNowStrict } from "date-fns";
import type { LimitWindow } from "@/lib/types";

export function CountdownBadge({ resetsAt }: { resetsAt: number | null }) {
  if (resetsAt == null) {
    return (
      <span className="rounded-md border border-[var(--color-border)] px-2 py-0.5 text-xs text-[var(--color-muted-foreground)]">
        unknown
      </span>
    );
  }
  const ms = resetsAt * 1000;
  const past = ms < Date.now();
  const label = past
    ? `reset ${formatDistanceToNowStrict(ms)} ago`
    : `in ${formatDistanceToNowStrict(ms)}`;
  return (
    <span className="rounded-md border border-[var(--color-border)] bg-[var(--color-accent)] px-2 py-0.5 text-xs text-[var(--color-accent-foreground)]">
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
    <div className="rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-sm">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-base font-semibold capitalize">{source}</h3>
      </div>
      <WindowRow label="5 hour" win={five} />
      <div className="my-3 border-t border-[var(--color-border)]" />
      <WindowRow label="7 day" win={seven} />
    </div>
  );
}

function WindowRow({ label, win }: { label: string; win?: LimitWindow }) {
  const pct = win?.used_percent ?? null;
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-medium">{label}</span>
        <CountdownBadge resetsAt={win?.resets_at ?? null} />
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-[var(--color-muted)]">
        <div
          className="h-full rounded-full bg-[var(--color-primary)] transition-all"
          style={{ width: `${Math.min(100, Math.max(0, pct ?? 0))}%` }}
        />
      </div>
      <div className="text-xs text-[var(--color-muted-foreground)]">
        {pct != null ? `${pct.toFixed(1)}% used` : "no data"}
      </div>
    </div>
  );
}
