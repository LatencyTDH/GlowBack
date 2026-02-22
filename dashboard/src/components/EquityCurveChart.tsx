import type { ReactNode } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from "recharts";
import type { EquityCurvePoint } from "../types";

interface Props {
  data: EquityCurvePoint[];
}

function fmtUsd(v: number) {
  return `$${v.toLocaleString(undefined, { maximumFractionDigits: 0 })}`;
}

export default function EquityCurveChart({ data }: Props) {
  if (data.length === 0) return <p className="text-gray-500 italic">No data</p>;

  const initialValue = data[0].value;

  return (
    <ResponsiveContainer width="100%" height={360}>
      <LineChart data={data} margin={{ top: 8, right: 24, left: 16, bottom: 8 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
        <XAxis
          dataKey="timestamp"
          tick={{ fontSize: 11 }}
          tickFormatter={(v: string) => v.slice(5, 10)}
        />
        <YAxis
          tick={{ fontSize: 11 }}
          tickFormatter={fmtUsd}
          domain={["auto", "auto"]}
        />
        <Tooltip
          formatter={(v: number | undefined): ReactNode => fmtUsd(v ?? 0)}
          labelFormatter={(v: ReactNode): ReactNode => `Date: ${String(v).slice(0, 10)}`}
        />
        <ReferenceLine y={initialValue} stroke="#9ca3af" strokeDasharray="3 3" label="Initial" />
        <Line
          type="monotone"
          dataKey="value"
          stroke="#10b981"
          strokeWidth={2}
          dot={false}
          name="Portfolio"
        />
      </LineChart>
    </ResponsiveContainer>
  );
}
