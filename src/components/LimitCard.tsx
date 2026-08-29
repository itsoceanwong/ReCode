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
  onSetManual,
  onClearManual,
}: {
  source: string;
  windows: LimitWindow[];
  onSetManual: (kind: string, resetsAt: number, used?: number) => void;
  onClearManual: (kind: string) => void;
}) {
  const five = windows.find((w) => w.window_kind === "five_hour");
  const seven = windows.find((w) => w.window_kind === "seven_day");

  return (
    <div className="rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-sm">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-base font-semibold capitalize">{source}</h3>
      </div>
      <WindowRow
        label="5 hour"
        win={five}
        onSet={(resetsAt, used) => onSetManual("five_hour", resetsAt, used)}
        onClear={() => onClearManual("five_hour")}
      />
      <div className="my-3 border-t border-[var(--color-border)]" />
      <WindowRow
        label="7 day"
        win={seven}
        onSet={(resetsAt, used) => onSetManual("seven_day", resetsAt, used)}
        onClear={() => onClearManual("seven_day")}
      />
    </div>
  );
}

function WindowRow({
  label,
  win,
  onSet,
  onClear,
}: {
  label: string;
  win?: LimitWindow;
  onSet: (resetsAt: number, used?: number) => void;
  onClear: () => void;
}) {
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
      <div className="flex items-center justify-between text-xs text-[var(--color-muted-foreground)]">
        <span>
          {pct != null ? `${pct.toFixed(1)}% used` : "no data"}
          {win?.is_manual ? " · manual" : ""}
        </span>
        <span className="flex gap-2">
          <button
            type="button"
            className="underline-offset-2 hover:underline"
            onClick={() => {
              const minutes = window.prompt("Reset in how many minutes?", "5");
              if (!minutes) return;
              const n = Number(minutes);
              if (!Number.isFinite(n)) return;
              const usedRaw = window.prompt("Used percent (optional)", "0");
              const used = usedRaw != null && usedRaw !== "" ? Number(usedRaw) : undefined;
              onSet(Math.floor(Date.now() / 1000) + Math.floor(n * 60), used);
            }}
          >
            Set manual
          </button>
          {win?.is_manual && (
            <button type="button" className="underline-offset-2 hover:underline" onClick={onClear}>
              Clear
            </button>
          )}
        </span>
      </div>
    </div>
  );
}
