import type { UsageAggregate } from "@/lib/types";
import { AccuracyBadge } from "@/components/AccuracyBadge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

function fmt(n: number) {
  return new Intl.NumberFormat().format(n);
}

function money(n: number) {
  return `$${n.toFixed(4)}`;
}

export function TokenTable({ rows }: { rows: UsageAggregate[] }) {
  if (rows.length === 0) {
    return (
      <p className="text-sm text-[var(--color-muted-foreground)]">No usage events yet.</p>
    );
  }
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>App</TableHead>
          <TableHead>Model</TableHead>
          <TableHead>Input</TableHead>
          <TableHead>Output</TableHead>
          <TableHead>Cache R</TableHead>
          <TableHead>Cache W</TableHead>
          <TableHead>Total</TableHead>
          <TableHead>Cost</TableHead>
          <TableHead>Accuracy</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((r) => {
          const total =
            r.input + r.output + r.cache_read + r.cache_write + r.reasoning;
          return (
            <TableRow key={`${r.source}-${r.model}-${r.origin}`}>
              <TableCell className="capitalize">{r.source}</TableCell>
              <TableCell>{r.model || "—"}</TableCell>
              <TableCell>{fmt(r.input)}</TableCell>
              <TableCell>{fmt(r.output)}</TableCell>
              <TableCell>{fmt(r.cache_read)}</TableCell>
              <TableCell>{fmt(r.cache_write)}</TableCell>
              <TableCell>{fmt(total)}</TableCell>
              <TableCell>{money(r.cost_usd)}</TableCell>
              <TableCell>
                <AccuracyBadge origin={r.origin} />
              </TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}
