#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required for the GlowBack Python wheel smoke test" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 is required for the GlowBack Python wheel smoke test" >&2
  exit 1
fi

OUT_DIR="${GLOWBACK_WHEEL_OUT_DIR:-$(mktemp -d)}"
VENV_DIR="$(mktemp -d)"
LOG_FILE="$(mktemp)"
KEEP_OUT_DIR=0
if [[ -n "${GLOWBACK_WHEEL_OUT_DIR:-}" ]]; then
  KEEP_OUT_DIR=1
  mkdir -p "$OUT_DIR"
fi

cleanup() {
  rm -rf "$VENV_DIR"
  rm -f "$LOG_FILE"
  if [[ "$KEEP_OUT_DIR" -eq 0 ]]; then
    rm -rf "$OUT_DIR"
  fi
}
trap cleanup EXIT

echo "==> Running the GlowBack Python wheel smoke test"
echo "    Repo: $ROOT_DIR"
echo "    Wheel output: $OUT_DIR"
echo "    Venv: $VENV_DIR"

python3 -m pip install --user --upgrade pip maturin virtualenv
USER_BASE="$(python3 -c 'import site; print(site.USER_BASE)')"
VIRTUALENV_BIN="$USER_BASE/bin/virtualenv"
MATURIN_BIN="$USER_BASE/bin/maturin"
"$VIRTUALENV_BIN" "$VENV_DIR"
# shellcheck disable=SC1091
source "$VENV_DIR/bin/activate"

"$MATURIN_BIN" build --release --locked --generate-stubs --manifest-path crates/gb-python/Cargo.toml --out "$OUT_DIR"

shopt -s nullglob
wheels=("$OUT_DIR"/*.whl)
shopt -u nullglob
if [[ ${#wheels[@]} -ne 1 ]]; then
  echo "error: expected exactly one wheel in $OUT_DIR, found ${#wheels[@]}" >&2
  ls -1 "$OUT_DIR" >&2 || true
  exit 1
fi

WHEEL_PATH="${wheels[0]}"
echo "==> Installing $WHEEL_PATH"
python -m pip install --force-reinstall "$WHEEL_PATH"
python - <<'PY' | tee "$LOG_FILE"
from pathlib import Path
import glowback

module_root = Path(glowback.__file__).resolve().parent
required = [module_root / "__init__.pyi", module_root / "py.typed"]
missing = [str(path) for path in required if not path.exists()]
if missing:
    raise SystemExit(f"missing generated type stubs: {', '.join(missing)}")
print(f"Generated type stubs present: {required[0].name}, {required[1].name}")
PY
python examples/python_sdk_quickstart.py | tee -a "$LOG_FILE"

if ! grep -q "✅ Python SDK quickstart completed successfully" "$LOG_FILE"; then
  echo "error: Python wheel smoke test did not reach the expected success marker" >&2
  exit 1
fi

echo
echo "Python wheel smoke test succeeded."
echo "Wheel artifact: $WHEEL_PATH"
