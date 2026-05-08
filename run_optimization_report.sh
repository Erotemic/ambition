#!/usr/bin/env bash
# Generate an Ambition optimization baseline report.
#
# Output goes under target/optimization_reports/<UTC timestamp>/ by default,
# which is ignored by git via the repo's existing /target/ rule.
#
# Useful options:
#   ./run_optimization_report.sh              # normal baseline
#   ./run_optimization_report.sh --quick      # skip release/distribution builds
#   ./run_optimization_report.sh --clean      # cargo clean before measuring
#   ./run_optimization_report.sh --long-tests # include slower/noisier deep probes
#   ./run_optimization_report.sh --help
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON_BIN="${PYTHON:-python3}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
REPORT_BASE="${AMBITION_OPT_REPORT_BASE:-$SCRIPT_DIR/target/optimization_reports}"
REPORT_DIR="${AMBITION_OPT_REPORT_DIR:-$REPORT_BASE/$STAMP}"

mkdir -p "$REPORT_DIR"

echo "[optimization-report] repo: $SCRIPT_DIR"
echo "[optimization-report] output: $REPORT_DIR"
echo "[optimization-report] python: $PYTHON_BIN"

exec "$PYTHON_BIN" "$SCRIPT_DIR/tools/optimization_report/collect_optimization_report.py" \
    --repo "$SCRIPT_DIR" \
    --out "$REPORT_DIR" \
    "$@"
