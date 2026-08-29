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
          <CartesianGrid strokeDasharray="3 3" stroke="#ddd" />
          <XAxis dataKey="name" tick={{ fontSize: 11 }} />
          <YAxis tick={{ fontSize: 11 }} />
          <Tooltip />
          <Legend />
          <Bar dataKey="input" stackId="t" fill="#2f5d8a" />
          <Bar dataKey="output" stackId="t" fill="#3d8b7a" />
          <Bar dataKey="cache" stackId="t" fill="#a3b18a" />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
