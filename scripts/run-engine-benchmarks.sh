#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${1:-artifacts/benchmarks/strategy-context}"
BENCHMARK_GROUP="strategy_context_scaling"

cd "$ROOT_DIR"
rm -rf "target/criterion/${BENCHMARK_GROUP}" "$OUTPUT_DIR"

cargo bench -p gb-engine --bench strategy_context -- --noplot
python3 scripts/benchmark_report.py \
  --criterion-root "target/criterion/${BENCHMARK_GROUP}" \
  --output-dir "$OUTPUT_DIR"
