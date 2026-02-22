interface Props {
  metrics: Record<string, number>;
}

interface Card {
  key: string;
  label: string;
  fmt: (v: number) => string;
}

const fmtUsd = (v: number) => `$${v.toLocaleString(undefined, { maximumFractionDigits: 0 })}`;
const fmtPct = (v: number) => `${v.toFixed(2)}%`;
const fmtRatio = (v: number) => v.toFixed(3);

const CARDS: Card[] = [
  { key: "initial_capital", label: "Initial Capital", fmt: fmtUsd },
  { key: "final_value", label: "Final Value", fmt: fmtUsd },
  { key: "total_return", label: "Total Return", fmt: fmtPct },
  { key: "annualized_return", label: "Annualized Return", fmt: fmtPct },
  { key: "volatility", label: "Volatility", fmt: fmtPct },
  { key: "sharpe_ratio", label: "Sharpe Ratio", fmt: fmtRatio },
  { key: "sortino_ratio", label: "Sortino Ratio", fmt: fmtRatio },
  { key: "max_drawdown", label: "Max Drawdown", fmt: fmtPct },
  { key: "calmar_ratio", label: "Calmar Ratio", fmt: fmtRatio },
  { key: "var_95", label: "VaR (95%)", fmt: fmtPct },
  { key: "cvar_95", label: "CVaR (95%)", fmt: fmtPct },
  { key: "total_trades", label: "Total Trades", fmt: (v) => String(Math.round(v)) },
  { key: "win_rate", label: "Win Rate", fmt: fmtPct },
  { key: "profit_factor", label: "Profit Factor", fmt: fmtRatio },
];

function color(key: string, v: number): string {
  if (["total_return", "annualized_return", "sharpe_ratio", "sortino_ratio", "calmar_ratio", "profit_factor", "win_rate"].includes(key))
    return v > 0 ? "text-green-600" : v < 0 ? "text-red-600" : "";
  if (["max_drawdown", "var_95", "cvar_95"].includes(key))
    return v > 5 ? "text-red-600" : "text-yellow-600";
  return "";
}

export default function MetricsGrid({ metrics }: Props) {
  return (
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
      {CARDS.map(({ key, label, fmt }) => {
        const v = metrics[key];
        if (v === undefined) return null;
        return (
          <div key={key} className="rounded-lg border border-gray-200 bg-white p-4 shadow-sm">
            <p className="text-xs font-medium text-gray-500">{label}</p>
            <p className={`mt-1 text-lg font-semibold ${color(key, v)}`}>{fmt(v)}</p>
          </div>
        );
      })}
    </div>
  );
}
