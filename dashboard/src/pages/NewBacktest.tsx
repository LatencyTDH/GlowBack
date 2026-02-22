import { type FormEvent, useState } from "react";
import { useNavigate } from "react-router-dom";
import { createBacktest } from "../api";

const STRATEGIES = [
  { value: "buy_and_hold", label: "Buy & Hold" },
  { value: "sma_crossover", label: "SMA Crossover" },
  { value: "rsi", label: "RSI" },
  { value: "mean_reversion", label: "Mean Reversion" },
];

const RESOLUTIONS = ["tick", "second", "minute", "hour", "day"];

export default function NewBacktest() {
  const navigate = useNavigate();
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [symbols, setSymbols] = useState("AAPL");
  const [startDate, setStartDate] = useState("2024-01-01");
  const [endDate, setEndDate] = useState("2024-12-31");
  const [resolution, setResolution] = useState("day");
  const [strategy, setStrategy] = useState("buy_and_hold");
  const [capital, setCapital] = useState("1000000");
  const [slippage, setSlippage] = useState("5");
  const [commission, setCommission] = useState("10");

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      const status = await createBacktest({
        symbols: symbols.split(",").map((s) => s.trim()).filter(Boolean),
        start_date: new Date(startDate).toISOString(),
        end_date: new Date(endDate).toISOString(),
        resolution,
        strategy: { name: strategy, params: {} },
        execution: {
          slippage_bps: Number(slippage) || null,
          commission_bps: Number(commission) || null,
        },
        initial_capital: Number(capital),
        currency: "USD",
        timezone: "UTC",
      });
      navigate(`/backtests/${status.run_id}`);
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="mx-auto max-w-2xl">
      <h1 className="mb-6 text-2xl font-bold">New Backtest</h1>

      <form onSubmit={onSubmit} className="space-y-6 rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        {/* Symbols */}
        <div>
          <label className="mb-1 block text-sm font-medium text-gray-700">Symbols</label>
          <input
            type="text"
            value={symbols}
            onChange={(e) => setSymbols(e.target.value)}
            placeholder="AAPL, MSFT, GOOG"
            className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-emerald-500 focus:ring-emerald-500 focus:outline-none"
            required
          />
          <p className="mt-1 text-xs text-gray-400">Comma-separated ticker symbols</p>
        </div>

        {/* Date range */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700">Start Date</label>
            <input
              type="date"
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-emerald-500 focus:ring-emerald-500 focus:outline-none"
              required
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700">End Date</label>
            <input
              type="date"
              value={endDate}
              onChange={(e) => setEndDate(e.target.value)}
              className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-emerald-500 focus:ring-emerald-500 focus:outline-none"
              required
            />
          </div>
        </div>

        {/* Strategy & Resolution */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700">Strategy</label>
            <select
              value={strategy}
              onChange={(e) => setStrategy(e.target.value)}
              className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-emerald-500 focus:ring-emerald-500 focus:outline-none"
            >
              {STRATEGIES.map((s) => (
                <option key={s.value} value={s.value}>{s.label}</option>
              ))}
            </select>
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700">Resolution</label>
            <select
              value={resolution}
              onChange={(e) => setResolution(e.target.value)}
              className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-emerald-500 focus:ring-emerald-500 focus:outline-none"
            >
              {RESOLUTIONS.map((r) => (
                <option key={r} value={r}>{r}</option>
              ))}
            </select>
          </div>
        </div>

        {/* Capital & Execution */}
        <div className="grid grid-cols-3 gap-4">
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700">Initial Capital ($)</label>
            <input
              type="number"
              value={capital}
              onChange={(e) => setCapital(e.target.value)}
              min="1"
              className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-emerald-500 focus:ring-emerald-500 focus:outline-none"
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700">Slippage (bps)</label>
            <input
              type="number"
              value={slippage}
              onChange={(e) => setSlippage(e.target.value)}
              min="0"
              className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-emerald-500 focus:ring-emerald-500 focus:outline-none"
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700">Commission (bps)</label>
            <input
              type="number"
              value={commission}
              onChange={(e) => setCommission(e.target.value)}
              min="0"
              className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-emerald-500 focus:ring-emerald-500 focus:outline-none"
            />
          </div>
        </div>

        {error && (
          <div className="rounded-md bg-red-50 p-3 text-sm text-red-700">{error}</div>
        )}

        <button
          type="submit"
          disabled={submitting}
          className="w-full rounded-md bg-emerald-600 py-2.5 text-sm font-medium text-white shadow hover:bg-emerald-700 disabled:opacity-50"
        >
          {submitting ? "Launching…" : "Launch Backtest"}
        </button>
      </form>
    </div>
  );
}
