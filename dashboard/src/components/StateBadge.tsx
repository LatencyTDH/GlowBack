import type { RunState } from "../types";

const COLORS: Record<RunState, string> = {
  queued: "bg-yellow-100 text-yellow-800",
  running: "bg-blue-100 text-blue-800",
  completed: "bg-green-100 text-green-800",
  failed: "bg-red-100 text-red-800",
};

export default function StateBadge({ state }: { state: RunState }) {
  return (
    <span className={`inline-block rounded-full px-2.5 py-0.5 text-xs font-semibold ${COLORS[state]}`}>
      {state}
    </span>
  );
}
