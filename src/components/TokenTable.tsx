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
      <div className="py-8 text-center text-xs text-[var(--color-muted-foreground)]">
        No usage events recorded yet.
      </div>
    );
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>App</TableHead>
          <TableHead>Model</TableHead>
          <TableHead className="text-right">Input</TableHead>
          <TableHead className="text-right">Output</TableHead>
          <TableHead className="text-right">Cache Read</TableHead>
          <TableHead className="text-right">Cache Write</TableHead>
          <TableHead className="text-right">Total Tokens</TableHead>
          <TableHead className="text-right">Est. Cost</TableHead>
          <TableHead className="text-center">Accuracy</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((r) => {
          const total =
            r.input + r.output + r.cache_read + r.cache_write + r.reasoning;
          return (
            <TableRow key={`${r.source}-${r.model}-${r.origin}`}>
              <TableCell className="font-semibold capitalize text-[var(--color-foreground)]">
                {r.source}
              </TableCell>
              <TableCell className="font-mono text-[11px] text-[var(--color-muted-foreground)] max-w-[10rem] truncate">
                {r.model || "—"}
              </TableCell>
              <TableCell className="font-mono text-right">{fmt(r.input)}</TableCell>
              <TableCell className="font-mono text-right">{fmt(r.output)}</TableCell>
              <TableCell className="font-mono text-right">{fmt(r.cache_read)}</TableCell>
              <TableCell className="font-mono text-right">{fmt(r.cache_write)}</TableCell>
              <TableCell className="font-mono text-right font-bold text-[var(--color-foreground)]">
                {fmt(total)}
              </TableCell>
              <TableCell className="font-mono text-right font-semibold text-emerald-400">
                {money(r.cost_usd)}
              </TableCell>
              <TableCell className="text-center">
                <AccuracyBadge origin={r.origin} />
              </TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}
