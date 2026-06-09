#!/usr/bin/env bash
# Run JSCPD clone detection in Docker from the Git repository root.
#
# Note: the filename intentionally follows the requested spelling
# "run_jspcd.sh", but the tool this script runs is JSCPD.
#
# Examples:
#   scripts/run_jspcd.sh
#   scripts/run_jspcd.sh --min-lines 12 --min-tokens 120
#   JSCPD_TARGETS="crates/ambition_sandbox/src crates/ambition_platformer_runtime/src" scripts/run_jspcd.sh

set -Eeuo pipefail

if ! command -v git >/dev/null 2>&1; then
    echo "error: git is required" >&2
    exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
    echo "error: docker is required to run JSCPD via this script" >&2
    exit 1
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

JSCPD_IMAGE="${JSCPD_IMAGE:-megabytelabs/jscpd:latest}"
JSCPD_REPORT_DIR="${JSCPD_REPORT_DIR:-.agent/reports/jscpd}"
JSCPD_MIN_LINES="${JSCPD_MIN_LINES:-8}"
JSCPD_MIN_TOKENS="${JSCPD_MIN_TOKENS:-80}"
JSCPD_FORMATS="${JSCPD_FORMATS:-rust,python,toml,markdown,yaml,json,bash}"

# Space-separated target paths. Override with JSCPD_TARGETS if needed.
# shellcheck disable=SC2206
#JSCPD_TARGET_ARRAY=(${JSCPD_TARGETS:-crates scripts tools})
JSCPD_TARGET_ARRAY=(${JSCPD_TARGETS:-crates scripts})

mkdir -p "$JSCPD_REPORT_DIR"

cat <<MSG
[jscpd] repo: $REPO_ROOT
[jscpd] image: $JSCPD_IMAGE
[jscpd] output: $JSCPD_REPORT_DIR
[jscpd] targets: ${JSCPD_TARGET_ARRAY[*]}
MSG

exec docker run --rm \
    --user "$(id -u):$(id -g)" \
    -e HOME=/tmp \
    -v "$REPO_ROOT":/work \
    -w /work \
    "$JSCPD_IMAGE" \
    --format "$JSCPD_FORMATS" \
    --min-lines 50 \
    --min-lines "$JSCPD_MIN_LINES" \
    --min-tokens "$JSCPD_MIN_TOKENS" \
    --reporters console,json,markdown \
    --output "$JSCPD_REPORT_DIR" \
    --gitignore \
    --ignore ".venv" \
    --ignore ".venv/**" \
    --ignore "**/.venv/**" \
    --ignore ".venv/**" \
    --ignore "**/.venv" \
    --ignore ".venv" \
    --ignore "tools/ambition_sfx_renderer/.venv" \
    --ignore "**/target/**" \
    --ignore "**/.git/**" \
    --ignore "**/.agent/**" \
    --ignore "**/debug_traces/**" \
    --ignore "**/.worktrees/**" \
    --ignore "**/snapshots/**" \
    --ignore "**/node_modules/**" \
    "${JSCPD_TARGET_ARRAY[@]}" \
    "$@"
