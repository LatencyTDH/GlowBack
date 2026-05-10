# Reproducing a Run

GlowBack backtest results now include a `manifest` payload that captures the
engine version, dataset fingerprint, execution knobs, and a replayable request
shape for the run.

## Fetch a completed run

```bash
curl -s \
  -H "X-API-Key: $API_KEY" \
  http://localhost:8000/backtests/<run-id>/results > run-result.json
```

The response contains a top-level `manifest` object.

## Replay locally

Use the shared Python runtime helper to rerun the exact backtest request encoded
in the manifest:

```python
import json
from pathlib import Path

from glowback_runtime import compare_manifest_metrics, replay_manifest

result = json.loads(Path("run-result.json").read_text())
manifest = result["manifest"]
replay = replay_manifest(manifest)
comparison = compare_manifest_metrics(manifest, replay, tolerance=1e-6)

print(comparison)
```

## What gets captured

The current manifest slice includes:

- engine crate + version
- strategy id/name + parameter payload
- data source, symbol universe, resolution, date range, and per-symbol bar counts
- execution knobs used by the engine-backed API path
- a replay-ready request payload
- headline metrics for replay comparison

## Tolerances

For the current built-in strategy replay path, the documented expectation is an
exact match on the captured headline metrics when replaying deterministic sample
or CSV-backed runs. The helper uses a default absolute tolerance of `1e-6`.
