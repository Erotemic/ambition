#!/usr/bin/env bash
# Per-file Rust line and #[test] counts for Ambition's main workspace code.
#
# The repository is split across three top-level Rust domains:
#
#   crates/  reusable engine and support crates
#   game/    game, content, app, and demo crates
#   tests/   sequestered workspace-policy and cross-repository test crates
#
# This is a quick reconnaissance tool for code/test distribution. It is not a
# coverage substitute: use `cargo llvm-cov` for executable line/branch coverage.
#
# Usage:
#   ./tools/test_coverage_report.sh           # all three domains
#   ./tools/test_coverage_report.sh all       # same as the default
#   ./tools/test_coverage_report.sh engine    # crates/**.rs
#   ./tools/test_coverage_report.sh game      # game/**.rs
#   ./tools/test_coverage_report.sh policy    # tests/**.rs
#
# Output is sorted by test count and then line count, followed by per-domain
# totals. Rust fixtures, examples, build scripts, src/, and integration tests
# are included because they are all maintained Rust source in these domains.

set -Eeuo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-all}"

usage() {
  cat <<'USAGE'
usage: ./tools/test_coverage_report.sh [all|engine|game|policy]

  all      scan crates/, game/, and tests/ (default)
  engine   scan reusable engine/support crates under crates/
  game     scan game/content/app/demo crates under game/
  policy   scan top-level policy and repository-test crates under tests/
USAGE
}

case "$MODE" in
  all)
    scopes=(engine game policy)
    ;;
  engine|game|policy)
    scopes=("$MODE")
    ;;
  -h|--help|help)
    usage
    exit 0
    ;;
  *)
    echo "error: unknown scope: $MODE" >&2
    usage >&2
    exit 2
    ;;
esac

scope_root() {
  case "$1" in
    engine) printf '%s\n' "$REPO_ROOT/crates" ;;
    game) printf '%s\n' "$REPO_ROOT/game" ;;
    policy) printf '%s\n' "$REPO_ROOT/tests" ;;
    *) return 2 ;;
  esac
}

count_tests() {
  # Count ordinary Rust test attributes, including qualified forms such as
  # #[tokio::test]. The report is intentionally a lightweight source count.
  grep -Ec '#\[[[:space:]]*([[:alnum:]_]+::)?test([[:space:]]*\([^]]*\))?[[:space:]]*\]' "$1" || true
}

rows_file="$(mktemp)"
trap 'rm -f "$rows_file"' EXIT

for scope in "${scopes[@]}"; do
  root="$(scope_root "$scope")"
  if [[ ! -d "$root" ]]; then
    echo "error: expected Rust domain does not exist: ${root#$REPO_ROOT/}" >&2
    exit 1
  fi

  found=0
  while IFS= read -r -d '' file; do
    found=1
    tests="$(count_tests "$file")"
    lines="$(wc -l < "$file")"
    relative="${file#$REPO_ROOT/}"
    printf '%s\t%s\t%s\t%s\n' "$scope" "$tests" "$lines" "$relative" >> "$rows_file"
  done < <(find "$root" -type f -name '*.rs' -print0 | sort -z)

  if [[ "$found" -eq 0 ]]; then
    echo "error: no Rust files found under ${root#$REPO_ROOT/}" >&2
    exit 1
  fi
done

printf '%-8s %-7s %-7s %s\n' 'scope' 'tests' 'lines' 'file'
printf '%-8s %-7s %-7s %s\n' '-----' '-----' '-----' '----'
sort -t $'\t' -k2,2n -k3,3n -k4,4 "$rows_file" \
  | awk -F '\t' '{ printf "%-8s %-7s %-7s %s\n", $1, $2, $3, $4 }'

printf '\n%-8s %-7s %-9s %-9s\n' 'scope' 'files' 'tests' 'lines'
printf '%-8s %-7s %-9s %-9s\n' '-----' '-----' '-----' '-----'

awk -F '\t' '
  {
    files[$1] += 1
    tests[$1] += $2
    lines[$1] += $3
    total_files += 1
    total_tests += $2
    total_lines += $3
  }
  END {
    order[1] = "engine"
    order[2] = "game"
    order[3] = "policy"
    for (i = 1; i <= 3; i++) {
      scope = order[i]
      if (files[scope] > 0) {
        printf "%-8s %-7d %-9d %-9d\n", scope, files[scope], tests[scope], lines[scope]
      }
    }
    printf "%-8s %-7d %-9d %-9d\n", "total", total_files, total_tests, total_lines
  }
' "$rows_file"
