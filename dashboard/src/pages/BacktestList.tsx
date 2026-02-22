import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { listBacktests } from "../api";
import type { BacktestStatus } from "../types";
import StateBadge from "../components/StateBadge";

function relTime(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const sec = Math.floor(diff / 1000);
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hrs = Math.floor(min / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

export default function BacktestList() {
  const [runs, setRuns] = useState<BacktestStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const data = await listBacktests();
        if (!cancelled) setRuns(data);
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  if (loading)
    return <p className="text-gray-500">Loading…</p>;

  if (error)
    return (
      <div className="rounded-lg border border-red-200 bg-red-50 p-6">
        <p className="text-red-700">Failed to load backtests: {error}</p>
        <p className="mt-2 text-sm text-gray-500">
          Make sure the API server is running on port 8000.
        </p>
      </div>
    );

  return (
    <div>
      <div className="mb-6 flex items-center justify-between">
        <h1 className="text-2xl font-bold">Backtest Runs</h1>
        <Link
          to="/backtests/new"
          className="rounded-md bg-emerald-600 px-4 py-2 text-sm font-medium text-white shadow hover:bg-emerald-700"
        >
          + New Backtest
        </Link>
      </div>

      {runs.length === 0 ? (
        <div className="rounded-lg border border-gray-200 bg-white p-12 text-center">
          <p className="text-gray-500">No backtests yet.</p>
          <Link to="/backtests/new" className="mt-2 inline-block text-emerald-600 hover:underline">
            Run your first backtest →
          </Link>
        </div>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-gray-200 bg-white shadow-sm">
          <table className="w-full text-left text-sm">
            <thead className="border-b border-gray-100 bg-gray-50 text-xs uppercase text-gray-500">
              <tr>
                <th className="px-4 py-3">Run ID</th>
                <th className="px-4 py-3">State</th>
                <th className="px-4 py-3">Progress</th>
                <th className="px-4 py-3">Created</th>
                <th className="px-4 py-3">Duration</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {runs.map((r) => {
                const duration =
                  r.started_at && r.finished_at
                    ? `${((new Date(r.finished_at).getTime() - new Date(r.started_at).getTime()) / 1000).toFixed(1)}s`
                    : "—";
                return (
                  <tr key={r.run_id} className="hover:bg-gray-50 transition">
                    <td className="px-4 py-3">
                      <Link to={`/backtests/${r.run_id}`} className="font-mono text-emerald-600 hover:underline">
                        {r.run_id.slice(0, 8)}…
                      </Link>
                    </td>
                    <td className="px-4 py-3"><StateBadge state={r.state} /></td>
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-2">
                        <div className="h-1.5 w-24 overflow-hidden rounded-full bg-gray-200">
                          <div
                            className="h-full rounded-full bg-emerald-500 transition-all"
                            style={{ width: `${r.progress * 100}%` }}
                          />
                        </div>
                        <span className="text-xs text-gray-500">{Math.round(r.progress * 100)}%</span>
                      </div>
                    </td>
                    <td className="px-4 py-3 text-gray-500">{relTime(r.created_at)}</td>
                    <td className="px-4 py-3 text-gray-500">{duration}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
