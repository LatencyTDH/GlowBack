#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${1:-${ROOT_DIR}/dist}"
ARTIFACT_BASENAME="${ARTIFACT_BASENAME:-glowback-engine-linux-x86_64}"
BINARY_PATH="${BINARY_PATH:-${ROOT_DIR}/target/release/gb-engine-service}"

if [[ ! -x "${BINARY_PATH}" ]]; then
  echo "expected release binary at ${BINARY_PATH}" >&2
  exit 1
fi

mkdir -p "${DIST_DIR}"
rm -rf "${DIST_DIR:?}/${ARTIFACT_BASENAME}"
rm -f \
  "${DIST_DIR}/${ARTIFACT_BASENAME}.tar.gz" \
  "${DIST_DIR}/${ARTIFACT_BASENAME}.tar.gz.sha256" \
  "${DIST_DIR}/${ARTIFACT_BASENAME}.metadata.json"

export ROOT_DIR ARTIFACT_BASENAME

STAGING_DIR="${DIST_DIR}/${ARTIFACT_BASENAME}"
mkdir -p "${STAGING_DIR}"

install -m 0755 "${BINARY_PATH}" "${STAGING_DIR}/gb-engine-service"
cp "${ROOT_DIR}/LICENSE" "${STAGING_DIR}/LICENSE"
cp "${ROOT_DIR}/README.md" "${STAGING_DIR}/README.md"

commit_sha="${GITHUB_SHA:-$(git -C "${ROOT_DIR}" rev-parse HEAD 2>/dev/null || echo unknown)}"
build_time="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

cat > "${STAGING_DIR}/BUILD_INFO.txt" <<EOF
Repository: ${GITHUB_REPOSITORY:-local}
Commit: ${commit_sha}
Workflow: ${GITHUB_WORKFLOW:-local}
Run ID: ${GITHUB_RUN_ID:-local}
Run Attempt: ${GITHUB_RUN_ATTEMPT:-0}
Built At (UTC): ${build_time}
EOF

python3 <<'PY' > "${DIST_DIR}/${ARTIFACT_BASENAME}.metadata.json"
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path

root = Path(os.environ["ROOT_DIR"])
artifact_basename = os.environ["ARTIFACT_BASENAME"]
commit_sha = os.environ.get("GITHUB_SHA")
if not commit_sha:
    try:
        commit_sha = subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            text=True,
        ).strip()
    except Exception:
        commit_sha = "unknown"

metadata = {
    "artifact_name": artifact_basename,
    "repository": os.environ.get("GITHUB_REPOSITORY", "local"),
    "commit": commit_sha,
    "workflow": os.environ.get("GITHUB_WORKFLOW", "local"),
    "run_id": os.environ.get("GITHUB_RUN_ID", "local"),
    "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT", "0"),
    "built_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "files": [
        "gb-engine-service",
        "LICENSE",
        "README.md",
        "BUILD_INFO.txt",
    ],
}
json.dump(metadata, fp=os.sys.stdout, indent=2)
os.sys.stdout.write("\n")
PY

tar -C "${DIST_DIR}" -czf "${DIST_DIR}/${ARTIFACT_BASENAME}.tar.gz" "${ARTIFACT_BASENAME}"
sha256sum "${DIST_DIR}/${ARTIFACT_BASENAME}.tar.gz" > "${DIST_DIR}/${ARTIFACT_BASENAME}.tar.gz.sha256"

echo "Created release bundle: ${DIST_DIR}/${ARTIFACT_BASENAME}.tar.gz"
