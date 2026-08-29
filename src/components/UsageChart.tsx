import {
  Bar,
  BarChart,
  CartesianGrid,
  Legend,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { UsageAggregate } from "@/lib/types";

const tickStyle = {
  fontSize: 11,
  fill: "var(--color-muted-foreground)",
} as const;

const tooltipStyle = {
  backgroundColor: "var(--color-card)",
  border: "1px solid var(--color-border)",
  borderRadius: "0.5rem",
  color: "var(--color-foreground)",
  boxShadow: "0 4px 12px color-mix(in oklch, var(--color-foreground) 12%, transparent)",
} as const;

export function UsageChart({ rows }: { rows: UsageAggregate[] }) {
  const data = rows.map((r) => ({
    name: r.model || r.source,
    input: r.input,
    output: r.output,
    cache: r.cache_read + r.cache_write,
    cost: Number(r.cost_usd.toFixed(4)),
  }));

  if (data.length === 0) {
    return null;
  }

  return (
    <div className="h-64 w-full">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={data}>
          <CartesianGrid
            strokeDasharray="3 3"
            stroke="var(--color-border)"
          />
          <XAxis
            dataKey="name"
            tick={tickStyle}
            stroke="var(--color-border)"
          />
          <YAxis tick={tickStyle} stroke="var(--color-border)" />
          <Tooltip
            contentStyle={tooltipStyle}
            labelStyle={{ color: "var(--color-foreground)" }}
            itemStyle={{ color: "var(--color-foreground)" }}
            cursor={{ fill: "var(--color-muted)", opacity: 0.35 }}
          />
          <Legend wrapperStyle={{ color: "var(--color-foreground)" }} />
          <Bar dataKey="input" stackId="t" fill="#2f5d8a" />
          <Bar dataKey="output" stackId="t" fill="#3d8b7a" />
          <Bar dataKey="cache" stackId="t" fill="#a3b18a" />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
