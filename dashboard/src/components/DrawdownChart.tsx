import type { ReactNode } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import type { EquityCurvePoint } from "../types";

interface Props {
  data: EquityCurvePoint[];
}

function fmtPct(v: number) {
  return `${v.toFixed(2)}%`;
}

export default function DrawdownChart({ data }: Props) {
  if (data.length === 0) return null;

  return (
    <ResponsiveContainer width="100%" height={200}>
      <AreaChart data={data} margin={{ top: 8, right: 24, left: 16, bottom: 8 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
        <XAxis
          dataKey="timestamp"
          tick={{ fontSize: 11 }}
          tickFormatter={(v: string) => v.slice(5, 10)}
        />
        <YAxis
          tick={{ fontSize: 11 }}
          tickFormatter={fmtPct}
          domain={["auto", 0]}
        />
        <Tooltip
          formatter={(v: number | undefined): ReactNode => fmtPct(v ?? 0)}
          labelFormatter={(v: ReactNode): ReactNode => `Date: ${String(v).slice(0, 10)}`}
        />
        <Area
          type="monotone"
          dataKey="drawdown"
          stroke="#ef4444"
          fill="#fee2e2"
          strokeWidth={1.5}
          name="Drawdown"
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}
