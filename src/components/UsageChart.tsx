import {
  AreaChart,
  Area,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { UsageAggregate } from "@/lib/types";

const tickStyle = {
  fontSize: 10,
  fill: "var(--color-muted-foreground)",
  fontFamily: "monospace",
} as const;

const numberFmt = new Intl.NumberFormat();
function fmtNumber(n: number) {
  return numberFmt.format(n);
}

// Custom Glassmorphic Tooltip
interface TooltipPayloadItem {
  name: string;
  value: number;
  color: string;
}

interface CustomTooltipProps {
  active?: boolean;
  payload?: TooltipPayloadItem[];
  label?: string;
}

function CustomTooltip({ active, payload, label }: CustomTooltipProps) {
  if (!active || !payload || !payload.length) return null;
  return (
    <div className="rounded-xl border border-white/10 bg-[#121520]/95 p-3.5 shadow-2xl backdrop-blur-xl text-xs space-y-2">
      <div className="font-semibold text-white tracking-wide border-b border-white/10 pb-1.5">
        {label}
      </div>
      <div className="space-y-1 font-mono text-[11px]">
        {payload.map((entry) => (
          <div key={entry.name} className="flex items-center justify-between gap-4">
            <span className="flex items-center gap-1.5 capitalize" style={{ color: entry.color }}>
              <span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: entry.color }} />
              {entry.name}:
            </span>
            <span className="font-bold text-white">{fmtNumber(entry.value)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

export function UsageChart({ rows }: { rows: UsageAggregate[] }) {
  const data = rows.map((r) => ({
    name: r.model || r.source,
    input: r.input,
    output: r.output,
    cache: r.cache_read + r.cache_write,
    cost: Number(r.cost_usd.toFixed(4)),
  }));

  if (data.length === 0) {
    return (
      <div className="flex h-64 items-center justify-center text-xs text-[var(--color-muted-foreground)]">
        No usage data recorded for the selected window.
      </div>
    );
  }

  return (
    <div className="h-80 md:h-96 lg:h-[420px] w-full min-w-0 pt-2">
      <ResponsiveContainer width="100%" height="100%" minWidth={0} minHeight={260} debounce={50}>
        <AreaChart data={data} margin={{ top: 10, right: 20, left: 0, bottom: 0 }}>
          <defs>
            <linearGradient id="amberGlow" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#F97316" stopOpacity={0.45} />
              <stop offset="95%" stopColor="#F97316" stopOpacity={0.0} />
            </linearGradient>
            <linearGradient id="skyGlow" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#38BDF8" stopOpacity={0.4} />
              <stop offset="95%" stopColor="#38BDF8" stopOpacity={0.0} />
            </linearGradient>
            <linearGradient id="emeraldGlow" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#34D399" stopOpacity={0.4} />
              <stop offset="95%" stopColor="#34D399" stopOpacity={0.0} />
            </linearGradient>
          </defs>

          <CartesianGrid
            strokeDasharray="3 3"
            stroke="var(--color-border)"
            vertical={false}
            opacity={0.6}
          />
          <XAxis
            dataKey="name"
            tick={tickStyle}
            stroke="var(--color-border)"
            tickLine={false}
          />
          <YAxis
            tick={tickStyle}
            stroke="var(--color-border)"
            tickLine={false}
            tickFormatter={(v: number) => fmtNumber(v)}
          />
          <Tooltip content={<CustomTooltip />} />

          {/* Smooth Curves */}
          <Area
            type="monotone"
            dataKey="input"
            name="Input Tokens"
            stroke="#38BDF8"
            strokeWidth={2.5}
            fillOpacity={1}
            fill="url(#skyGlow)"
          />
          <Area
            type="monotone"
            dataKey="output"
            name="Output Tokens"
            stroke="#34D399"
            strokeWidth={2.5}
            fillOpacity={1}
            fill="url(#emeraldGlow)"
          />
          <Area
            type="monotone"
            dataKey="cache"
            name="Cache Tokens"
            stroke="#F97316"
            strokeWidth={2.5}
            fillOpacity={1}
            fill="url(#amberGlow)"
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
