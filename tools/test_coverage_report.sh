#!/usr/bin/env bash
# Per-module #[test] count for the ambition_gameplay_core crate (the former
# standalone ambition_engine now lives under src/engine_core/).
#
# Quick reconnaissance tool: when picking a module to add tests to,
# run this from the repo root to see which Rust files have the
# weakest coverage relative to their size. Outputs three columns:
# tests, lines, file. Sorted by tests ascending so under-tested
# files float to the top.
#
# Usage:
#   ./tools/test_coverage_report.sh           # default: whole sandbox crate
#   ./tools/test_coverage_report.sh engine    # only src/engine_core/
#   ./tools/test_coverage_report.sh sandbox   # whole sandbox crate src
#
# This is a triage tool, not a coverage substitute. `cargo llvm-cov`
# is the right answer when you actually want line/branch coverage —
# this script only counts `#[test]` annotations.

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "${1:-both}" in
  engine)
    targets=("$REPO_ROOT/crates/ambition_gameplay_core/src/engine_core")
    ;;
  sandbox|both|*)
    # One crate now; its src already contains engine_core/.
    targets=("$REPO_ROOT/crates/ambition_gameplay_core/src")
    ;;
esac

printf "%-7s %-7s %s\n" "tests" "lines" "file"
printf "%-7s %-7s %s\n" "-----" "-----" "----"

for target in "${targets[@]}"; do
  find "$target" -name '*.rs' -type f | sort | while read -r f; do
    tests="$(grep -c '#\[test\]' "$f" || true)"
    lines="$(wc -l < "$f")"
    rel="${f#$REPO_ROOT/}"
    printf "%-7s %-7s %s\n" "$tests" "$lines" "$rel"
  done
done | sort -n -k1
