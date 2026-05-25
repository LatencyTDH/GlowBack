#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required for the GlowBack CSV data tutorial" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 is required for the GlowBack CSV data tutorial" >&2
  exit 1
fi

VENV_DIR="$(mktemp -d)"
LOG_FILE="$(mktemp)"
cleanup() {
  rm -rf "$VENV_DIR"
  rm -f "$LOG_FILE"
}
trap cleanup EXIT

echo "==> Running the GlowBack CSV data tutorial"
echo "    Repo: $ROOT_DIR"
echo "    Venv: $VENV_DIR"

python3 -m pip install --user --upgrade pip maturin virtualenv
USER_BASE="$(python3 -c 'import site; print(site.USER_BASE)')"
VIRTUALENV_BIN="$USER_BASE/bin/virtualenv"
"$VIRTUALENV_BIN" "$VENV_DIR"
# shellcheck disable=SC1091
source "$VENV_DIR/bin/activate"

maturin develop -m crates/gb-python/Cargo.toml
python examples/csv_data_tutorial.py | tee "$LOG_FILE"

if ! grep -q "✅ CSV data tutorial completed successfully" "$LOG_FILE"; then
  echo "error: CSV data tutorial did not reach the expected success marker" >&2
  exit 1
fi

echo
echo "CSV data tutorial succeeded."
echo "Next steps: docs/tutorials/csv-data.md, docs/examples/index.md"
