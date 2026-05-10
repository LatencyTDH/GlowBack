#!/usr/bin/env python3
"""Generate a compact benchmark summary from Criterion output."""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SCENARIO_RE = re.compile(r"(?P<symbols>\d+)symbols_(?P<days>\d+)days")


@dataclass
class BenchmarkRow:
    scenario: str
    symbols: int | None
    days: int | None
    implementation: str
    group_id: str
    title: str
    mean_ns: float
    ci_lower_ns: float
    ci_upper_ns: float
    throughput_elements: int | None
    throughput_elements_per_second: float | None
    raw_path: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--criterion-root",
        default="target/criterion/strategy_context_scaling",
        help="Criterion benchmark group root to scan.",
    )
    parser.add_argument(
        "--output-dir",
        required=True,
        help="Directory that will receive summary.md, summary.json, and copied raw Criterion output.",
    )
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def human_duration(ns: float) -> str:
    if ns >= 1_000_000_000:
        return f"{ns / 1_000_000_000:.2f} s"
    if ns >= 1_000_000:
        return f"{ns / 1_000_000:.2f} ms"
    if ns >= 1_000:
        return f"{ns / 1_000:.2f} µs"
    return f"{ns:.0f} ns"


def human_throughput(value: float | None) -> str:
    if value is None:
        return "n/a"
    if value >= 1_000_000:
        return f"{value / 1_000_000:.2f} M bars/s"
    if value >= 1_000:
        return f"{value / 1_000:.2f} K bars/s"
    return f"{value:.2f} bars/s"


def parse_rows(criterion_root: Path) -> list[BenchmarkRow]:
    rows: list[BenchmarkRow] = []
    for benchmark_path in sorted(criterion_root.rglob("new/benchmark.json")):
        estimates_path = benchmark_path.with_name("estimates.json")
        if not estimates_path.exists():
            continue

        benchmark = read_json(benchmark_path)
        estimates = read_json(estimates_path)

        mean = estimates.get("mean") or estimates.get("slope")
        if not mean:
            continue

        scenario = benchmark.get("value_str") or benchmark.get("directory_name", "")
        match = SCENARIO_RE.fullmatch(scenario)
        throughput = benchmark.get("throughput", {}).get("Elements")
        mean_ns = float(mean["point_estimate"])
        ci = mean["confidence_interval"]
        throughput_per_second = None
        if throughput:
            throughput_per_second = throughput / (mean_ns / 1_000_000_000)

        rows.append(
            BenchmarkRow(
                scenario=scenario,
                symbols=int(match.group("symbols")) if match else None,
                days=int(match.group("days")) if match else None,
                implementation=benchmark.get("function_id") or benchmark.get("full_id", "unknown"),
                group_id=benchmark.get("group_id", "unknown"),
                title=benchmark.get("title") or benchmark.get("full_id", "unknown"),
                mean_ns=mean_ns,
                ci_lower_ns=float(ci["lower_bound"]),
                ci_upper_ns=float(ci["upper_bound"]),
                throughput_elements=throughput,
                throughput_elements_per_second=throughput_per_second,
                raw_path=str(benchmark_path.parent.relative_to(criterion_root.parent)),
            )
        )

    rows.sort(key=lambda row: ((row.symbols or 0), (row.days or 0), row.scenario, row.implementation))
    return rows


def build_comparisons(rows: list[BenchmarkRow]) -> list[dict[str, Any]]:
    by_scenario: dict[str, dict[str, BenchmarkRow]] = {}
    for row in rows:
        by_scenario.setdefault(row.scenario, {})[row.implementation] = row

    comparisons: list[dict[str, Any]] = []
    for scenario, implementations in sorted(by_scenario.items()):
        legacy = implementations.get("legacy_full_rebuild")
        incremental = implementations.get("incremental_buffers")
        if not legacy or not incremental:
            continue
        comparisons.append(
            {
                "scenario": scenario,
                "symbols": legacy.symbols,
                "days": legacy.days,
                "baseline": legacy.implementation,
                "candidate": incremental.implementation,
                "speedup": legacy.mean_ns / incremental.mean_ns,
                "throughput_gain": (
                    incremental.throughput_elements_per_second / legacy.throughput_elements_per_second
                    if incremental.throughput_elements_per_second and legacy.throughput_elements_per_second
                    else None
                ),
            }
        )
    return comparisons


def render_markdown(rows: list[BenchmarkRow], comparisons: list[dict[str, Any]], criterion_root: Path) -> str:
    generated_at = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%SZ")
    git_sha = os.environ.get("GITHUB_SHA") or os.environ.get("GIT_SHA") or "local"
    lines = [
        "# GlowBack benchmark report",
        "",
        f"- Generated: {generated_at}",
        f"- Commit: `{git_sha}`",
        f"- Source: `{criterion_root}`",
        "- Command: `cargo bench -p gb-engine --bench strategy_context -- --noplot`",
        "",
        "## Benchmarks",
        "",
        "| Scenario | Implementation | Mean time | 95% CI | Throughput |",
        "| --- | --- | ---: | ---: | ---: |",
    ]

    for row in rows:
        lines.append(
            "| "
            f"{row.scenario} | {row.implementation} | {human_duration(row.mean_ns)} | "
            f"{human_duration(row.ci_lower_ns)} – {human_duration(row.ci_upper_ns)} | "
            f"{human_throughput(row.throughput_elements_per_second)} |"
        )

    if comparisons:
        lines.extend(
            [
                "",
                "## Hot-path comparison",
                "",
                "| Scenario | Faster path | Speedup vs legacy | Throughput gain |",
                "| --- | --- | ---: | ---: |",
            ]
        )
        for comparison in comparisons:
            throughput_gain = comparison.get("throughput_gain")
            throughput_text = f"{throughput_gain:.2f}x" if throughput_gain is not None else "n/a"
            lines.append(
                f"| {comparison['scenario']} | {comparison['candidate']} | {comparison['speedup']:.2f}x | {throughput_text} |"
            )

    lines.extend(
        [
            "",
            "## Artifact contents",
            "",
            "- `summary.json`: machine-readable benchmark summary",
            "- `raw/criterion/`: copied Criterion output for drill-down and historical inspection",
        ]
    )
    return "\n".join(lines) + "\n"


def main() -> int:
    args = parse_args()
    criterion_root = Path(args.criterion_root).resolve()
    output_dir = Path(args.output_dir).resolve()

    rows = parse_rows(criterion_root)
    if not rows:
        raise SystemExit(f"No benchmark results found under {criterion_root}")

    comparisons = build_comparisons(rows)
    output_dir.mkdir(parents=True, exist_ok=True)

    raw_dir = output_dir / "raw" / "criterion"
    if raw_dir.exists():
        shutil.rmtree(raw_dir)
    shutil.copytree(criterion_root, raw_dir)

    payload = {
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "git_sha": os.environ.get("GITHUB_SHA") or os.environ.get("GIT_SHA"),
        "benchmarks": [asdict(row) for row in rows],
        "comparisons": comparisons,
    }

    (output_dir / "summary.json").write_text(json.dumps(payload, indent=2) + "\n")
    (output_dir / "summary.md").write_text(render_markdown(rows, comparisons, criterion_root))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
