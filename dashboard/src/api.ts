import type {
  BacktestRequest,
  BacktestResult,
  BacktestStatus,
  RunState,
} from "./types";

const BASE = "/api";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...init?.headers,
    },
  });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`API ${res.status}: ${body}`);
  }
  return res.json() as Promise<T>;
}

export async function listBacktests(
  state?: RunState,
  limit = 50,
): Promise<BacktestStatus[]> {
  const params = new URLSearchParams();
  if (state) params.set("state", state);
  params.set("limit", String(limit));
  return request<BacktestStatus[]>(`/backtests?${params}`);
}

export async function getBacktest(runId: string): Promise<BacktestStatus> {
  return request<BacktestStatus>(`/backtests/${runId}`);
}

export async function getResults(runId: string): Promise<BacktestResult> {
  return request<BacktestResult>(`/backtests/${runId}/results`);
}

export async function createBacktest(
  req: BacktestRequest,
): Promise<BacktestStatus> {
  return request<BacktestStatus>("/backtests", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

/** Returns a WebSocket that emits BacktestEvent JSON messages. */
export function streamBacktest(runId: string): WebSocket {
  const proto = location.protocol === "https:" ? "wss:" : "ws:";
  return new WebSocket(`${proto}//${location.host}${BASE}/backtests/${runId}/stream`);
}
