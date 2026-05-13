#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required for the GlowBack Python SDK quickstart" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 is required for the GlowBack Python SDK quickstart" >&2
  exit 1
fi

VENV_DIR="$(mktemp -d)"
LOG_FILE="$(mktemp)"
cleanup() {
  rm -rf "$VENV_DIR"
  rm -f "$LOG_FILE"
}
trap cleanup EXIT

echo "==> Running the GlowBack Python SDK quickstart"
echo "    Repo: $ROOT_DIR"
echo "    Venv: $VENV_DIR"

python3 -m pip install --user --upgrade pip maturin virtualenv
USER_BASE="$(python3 -c 'import site; print(site.USER_BASE)')"
VIRTUALENV_BIN="$USER_BASE/bin/virtualenv"
"$VIRTUALENV_BIN" "$VENV_DIR"
# shellcheck disable=SC1091
source "$VENV_DIR/bin/activate"

maturin develop -m crates/gb-python/Cargo.toml
python examples/python_sdk_quickstart.py | tee "$LOG_FILE"

if ! grep -q "✅ Python SDK quickstart completed successfully" "$LOG_FILE"; then
  echo "error: Python SDK quickstart did not reach the expected success marker" >&2
  exit 1
fi

echo
echo "Python SDK quickstart succeeded."
echo "Next steps: docs/api/python.md, docs/tutorials/notebook.md"
