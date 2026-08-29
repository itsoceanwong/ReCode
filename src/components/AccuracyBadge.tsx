import type { Origin } from "@/lib/types";
import { Badge } from "@/components/ui/badge";

export function AccuracyBadge({ origin }: { origin: Origin | string }) {
  const label =
    origin === "otlp"
      ? "accurate"
      : origin === "statusline"
        ? "session totals"
        : "approx";
  return <Badge title={`origin=${origin}`}>{label}</Badge>;
}
