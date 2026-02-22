import { useEffect, useRef, useState } from "react";
import { useParams, Link } from "react-router-dom";
import { getBacktest, getResults, streamBacktest } from "../api";
import type { BacktestEvent, BacktestResult, BacktestStatus } from "../types";
import StateBadge from "../components/StateBadge";
import EquityCurveChart from "../components/EquityCurveChart";
import DrawdownChart from "../components/DrawdownChart";
import MetricsGrid from "../components/MetricsGrid";

export default function BacktestDetail() {
  const { runId } = useParams<{ runId: string }>();
  const [status, setStatus] = useState<BacktestStatus | null>(null);
  const [result, setResult] = useState<BacktestResult | null>(null);
  const [events, setEvents] = useState<BacktestEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);

  // Fetch status
  useEffect(() => {
    if (!runId) return;
    getBacktest(runId).then(setStatus).catch((e) => setError(String(e)));
  }, [runId]);

  // WebSocket for live progress
  useEffect(() => {
    if (!runId || !status) return;
    if (status.state === "completed" || status.state === "failed") return;

    const ws = streamBacktest(runId);
    wsRef.current = ws;

    ws.onmessage = (msg) => {
      const event: BacktestEvent = JSON.parse(msg.data);
      setEvents((prev) => [...prev, event]);

      if (event.type === "progress") {
        setStatus((prev) =>
          prev ? { ...prev, progress: event.payload.progress as number } : prev,
        );
      }
      if (event.type === "state") {
        const newState = event.payload.state as BacktestStatus["state"];
        setStatus((prev) =>
          prev ? { ...prev, state: newState } : prev,
        );
        if (newState === "completed" && runId) {
          getResults(runId).then(setResult).catch(() => {});
        }
      }
    };

    return () => {
      ws.close();
      wsRef.current = null;
    };
  }, [runId, status?.state]); // eslint-disable-line react-hooks/exhaustive-deps

  // Fetch results when completed
  useEffect(() => {
    if (!runId || !status) return;
    if (status.state === "completed") {
      getResults(runId).then(setResult).catch(() => {});
    }
  }, [runId, status?.state]); // eslint-disable-line react-hooks/exhaustive-deps

  if (error) {
    return (
      <div className="rounded-lg border border-red-200 bg-red-50 p-6">
        <p className="text-red-700">{error}</p>
        <Link to="/backtests" className="mt-2 inline-block text-emerald-600 hover:underline">
          ← Back to list
        </Link>
      </div>
    );
  }

  if (!status) return <p className="text-gray-500">Loading…</p>;

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Link to="/backtests" className="text-gray-400 hover:text-gray-600">←</Link>
        <div>
          <h1 className="text-2xl font-bold">
            Run <span className="font-mono text-emerald-600">{status.run_id.slice(0, 8)}</span>
          </h1>
          <p className="text-sm text-gray-500">
            Created {new Date(status.created_at).toLocaleString()}
          </p>
        </div>
        <StateBadge state={status.state} />
      </div>

      {/* Progress bar (while running) */}
      {(status.state === "queued" || status.state === "running") && (
        <div className="rounded-lg border border-blue-100 bg-blue-50 p-4">
          <div className="mb-2 flex items-center justify-between text-sm">
            <span className="font-medium text-blue-700">Running…</span>
            <span className="text-blue-600">{Math.round(status.progress * 100)}%</span>
          </div>
          <div className="h-2 w-full overflow-hidden rounded-full bg-blue-200">
            <div
              className="h-full rounded-full bg-blue-600 transition-all duration-300"
              style={{ width: `${status.progress * 100}%` }}
            />
          </div>
        </div>
      )}

      {/* Error */}
      {status.state === "failed" && status.error && (
        <div className="rounded-lg border border-red-200 bg-red-50 p-4 text-red-700">
          {status.error}
        </div>
      )}

      {/* Results */}
      {result && (
        <>
          <section>
            <h2 className="mb-4 text-lg font-semibold">Performance Metrics</h2>
            <MetricsGrid metrics={result.metrics_summary} />
          </section>

          <section>
            <h2 className="mb-4 text-lg font-semibold">Equity Curve</h2>
            <div className="rounded-lg border border-gray-200 bg-white p-4 shadow-sm">
              <EquityCurveChart data={result.equity_curve} />
            </div>
          </section>

          <section>
            <h2 className="mb-4 text-lg font-semibold">Drawdown</h2>
            <div className="rounded-lg border border-gray-200 bg-white p-4 shadow-sm">
              <DrawdownChart data={result.equity_curve} />
            </div>
          </section>

          {result.logs.length > 0 && (
            <section>
              <h2 className="mb-4 text-lg font-semibold">Logs</h2>
              <div className="rounded-lg border border-gray-200 bg-gray-900 p-4 text-sm text-gray-200">
                {result.logs.map((line, i) => (
                  <div key={i} className="font-mono">{line}</div>
                ))}
              </div>
            </section>
          )}
        </>
      )}

      {/* Live event stream */}
      {events.length > 0 && (
        <section>
          <h2 className="mb-4 text-lg font-semibold">Event Stream</h2>
          <div className="max-h-60 overflow-y-auto rounded-lg border border-gray-200 bg-gray-900 p-4 text-xs text-gray-300">
            {events.map((ev) => (
              <div key={ev.event_id} className="font-mono">
                <span className="text-gray-500">[{ev.type}]</span>{" "}
                {JSON.stringify(ev.payload)}
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
