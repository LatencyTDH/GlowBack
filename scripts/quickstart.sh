#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required for the GlowBack quickstart" >&2
  exit 1
fi

LOG_FILE="$(mktemp)"
trap 'rm -f "$LOG_FILE"' EXIT

echo "==> Running the GlowBack 5-minute quickstart"
echo "    Repo: $ROOT_DIR"

cargo run --locked --example basic_usage -p gb-types | tee "$LOG_FILE"

if ! grep -q "✅ All basic functionality working!" "$LOG_FILE"; then
  echo "error: quickstart did not reach the expected success marker" >&2
  exit 1
fi

if ! grep -q "🎊 Strategy library complete with 4 different strategies!" "$LOG_FILE"; then
  echo "error: quickstart did not print the strategy library summary" >&2
  exit 1
fi

echo
echo "Quickstart succeeded."
echo "Next steps: docs/getting-started.md, docs/examples/index.md, docs/tutorials/reproducing-a-run.md"
