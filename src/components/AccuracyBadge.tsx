import type { Origin } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

export function AccuracyBadge({ origin }: { origin: Origin | string }) {
  const isOtlp = origin === "otlp";
  const isStatusline = origin === "statusline";

  const label = isOtlp
    ? "OTLP Exact"
    : isStatusline
      ? "Session Total"
      : "Estimated";

  return (
    <Badge
      title={`origin=${origin}`}
      className={cn(
        "text-[10px] font-mono",
        isOtlp && "border-emerald-500/30 bg-emerald-500/10 text-emerald-400",
        isStatusline && "border-sky-500/30 bg-sky-500/10 text-sky-400",
        !isOtlp && !isStatusline && "border-amber-500/30 bg-amber-500/10 text-amber-400",
      )}
    >
      {label}
    </Badge>
  );
}
