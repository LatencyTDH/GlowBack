/* ── GlowBack API types (mirrors api/app/models.py) ── */

export type RunState = "queued" | "running" | "completed" | "failed";

export interface StrategyConfig {
  name: string;
  params: Record<string, unknown>;
}

export interface ExecutionConfig {
  slippage_bps?: number | null;
  commission_bps?: number | null;
  latency_ms?: number | null;
}

export interface BacktestRequest {
  symbols: string[];
  start_date: string;
  end_date: string;
  resolution: string;
  strategy: StrategyConfig;
  execution: ExecutionConfig;
  initial_capital: number;
  currency: string;
  timezone: string;
}

export interface BacktestStatus {
  run_id: string;
  state: RunState;
  progress: number;
  created_at: string;
  started_at: string | null;
  finished_at: string | null;
  error: string | null;
}

export interface EquityCurvePoint {
  timestamp: string;
  value: number;
  cash: number;
  positions: number;
  total_pnl: number;
  returns: number;
  daily_return: number;
  drawdown: number;
}

export interface BacktestResult {
  run_id: string;
  metrics_summary: Record<string, number>;
  equity_curve: EquityCurvePoint[];
  trades: Record<string, unknown>[];
  exposures: Record<string, unknown>[];
  logs: string[];
}

export interface BacktestEvent {
  event_id: number;
  run_id: string;
  type: "log" | "progress" | "metric" | "state";
  timestamp: string;
  payload: Record<string, unknown>;
}
