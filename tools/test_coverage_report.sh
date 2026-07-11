#!/usr/bin/env bash
# Per-file Rust line and #[test] counts for Ambition's maintained workspace code.
#
# The repository is split across three top-level Rust domains:
#
#   crates/  reusable engine and support crates
#   game/    game, content, app, and demo crates
#   tests/   sequestered workspace-policy and cross-repository test crates
#
# This is a quick reconnaissance tool for code/test/data distribution. It is
# not a coverage substitute: use `cargo llvm-cov` for executable line/branch
# coverage.
#
# Usage:
#   ./tools/test_coverage_report.sh           # all three domains
#   ./tools/test_coverage_report.sh all       # same as the default
#   ./tools/test_coverage_report.sh engine    # crates/
#   ./tools/test_coverage_report.sh game      # game/
#   ./tools/test_coverage_report.sh tests     # tests/
#
# Output contains:
#   1. a detailed Rust file list sorted by test count and line count;
#   2. per-crate rollups grouped under crates/, game/, and tests/;
#   3. per-domain and repository totals.
#
# The rollups also count every regular non-Rust file beneath each package root
# and sum its on-disk byte size. This includes manifests, policy data, RON/TOML/
# YAML/JSON, fixtures, images, audio, and other checked-in data. Build output
# under target/ and VCS metadata under .git/ are excluded.

set -Eeuo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-all}"

usage() {
  cat <<'USAGE'
usage: ./tools/test_coverage_report.sh [all|engine|game|tests]

  all      scan crates/, game/, and tests/ (default)
  engine   scan reusable engine/support crates under crates/
  game     scan game/content/app/demo crates under game/
  tests    scan top-level policy and repository-test crates under tests/
USAGE
}

case "$MODE" in
  all)
    scopes=(engine game tests)
    ;;
  engine|game|tests)
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
    tests) printf '%s\n' "$REPO_ROOT/tests" ;;
    *) return 2 ;;
  esac
}

scope_folder() {
  case "$1" in
    engine) printf '%s\n' 'crates/' ;;
    game) printf '%s\n' 'game/' ;;
    tests) printf '%s\n' 'tests/' ;;
    *) printf '%s/\n' "$1" ;;
  esac
}

count_tests() {
  # Count ordinary Rust test attributes, including qualified forms such as
  # #[tokio::test]. The report is intentionally a lightweight source count.
  grep -Ec '#\[[[:space:]]*([[:alnum:]_]+::)?test([[:space:]]*\([^]]*\))?[[:space:]]*\]' "$1" || true
}

human_bytes() {
  # Keep exact byte totals in the intermediate data and format only for display.
  awk -v bytes="$1" 'BEGIN {
    split("B KiB MiB GiB TiB", unit, " ")
    value = bytes + 0
    idx = 1
    while (value >= 1024 && idx < 5) {
      value /= 1024
      idx += 1
    }
    if (idx == 1) {
      printf "%.0f %s", value, unit[idx]
    } else if (value >= 100) {
      printf "%.0f %s", value, unit[idx]
    } else if (value >= 10) {
      printf "%.1f %s", value, unit[idx]
    } else {
      printf "%.2f %s", value, unit[idx]
    }
  }'
}

rust_rows_file="$(mktemp)"
data_rows_file="$(mktemp)"
crate_rows_file="$(mktemp)"
trap 'rm -f "$rust_rows_file" "$data_rows_file" "$crate_rows_file"' EXIT

for scope in "${scopes[@]}"; do
  root="$(scope_root "$scope")"
  if [[ ! -d "$root" ]]; then
    echo "error: expected Rust domain does not exist: ${root#$REPO_ROOT/}" >&2
    exit 1
  fi

  rust_found=0
  while IFS= read -r -d '' file; do
    rust_found=1
    tests="$(count_tests "$file")"
    lines="$(wc -l < "$file")"
    relative="${file#$REPO_ROOT/}"
    printf '%s\t%s\t%s\t%s\n' "$scope" "$tests" "$lines" "$relative" >> "$rust_rows_file"
  done < <(
    find "$root" \
      \( -type d \( -name target -o -name .git \) -prune \) -o \
      \( -type f -name '*.rs' -print0 \) \
      | sort -z
  )

  if [[ "$rust_found" -eq 0 ]]; then
    echo "error: no Rust files found under ${root#$REPO_ROOT/}" >&2
    exit 1
  fi

  while IFS= read -r -d '' file; do
    bytes="$(stat -c '%s' "$file")"
    relative="${file#$REPO_ROOT/}"
    printf '%s\t%s\t%s\n' "$scope" "$bytes" "$relative" >> "$data_rows_file"
  done < <(
    find "$root" \
      \( -type d \( -name target -o -name .git \) -prune \) -o \
      \( -type f ! -name '*.rs' -print0 \) \
      | sort -z
  )
done

printf '%-8s %-7s %-7s %s\n' 'scope' 'tests' 'lines' 'file'
printf '%-8s %-7s %-7s %s\n' '-----' '-----' '-----' '----'
sort -t $'\t' -k2,2n -k3,3n -k4,4 "$rust_rows_file" \
  | awk -F '\t' '{ printf "%-8s %-7s %-7s %s\n", $1, $2, $3, $4 }'

# Aggregate Rust measurements by the direct child of each domain. In this
# workspace those children are the package roots; nested fixtures and support
# modules remain charged to the package that owns them.
awk -F '\t' '
  {
    split($4, path, "/")
    crate = path[2]
    if (crate == "") {
      crate = "(domain root)"
    }
    key = $1 SUBSEP crate
    rust_files[key] += 1
    tests[key] += $2
    rust_lines[key] += $3
    scopes[key] = $1
    crates[key] = crate
  }
  END {
    for (key in rust_files) {
      printf "%s\t%s\t%d\t%d\t%d\n", \
        scopes[key], crates[key], rust_files[key], tests[key], rust_lines[key]
    }
  }
' "$rust_rows_file" > "$crate_rows_file"

# Add non-Rust file counts and exact byte totals to the same crate rows. This
# second pass also creates rows for a package that happens to contain data but
# no Rust source, although the domain-level non-vacuity check above still
# requires each selected domain to contain Rust.
awk -F '\t' '
  FNR == NR {
    key = $1 SUBSEP $2
    rust_files[key] = $3
    tests[key] = $4
    rust_lines[key] = $5
    scopes[key] = $1
    crates[key] = $2
    next
  }
  {
    split($3, path, "/")
    crate = path[2]
    if (crate == "") {
      crate = "(domain root)"
    }
    key = $1 SUBSEP crate
    data_files[key] += 1
    data_bytes[key] += $2
    scopes[key] = $1
    crates[key] = crate
  }
  END {
    for (key in scopes) {
      printf "%s\t%s\t%d\t%d\t%d\t%d\t%.0f\n", \
        scopes[key], crates[key], rust_files[key] + 0, tests[key] + 0, \
        rust_lines[key] + 0, data_files[key] + 0, data_bytes[key] + 0
    }
  }
' "$crate_rows_file" "$data_rows_file" > "${crate_rows_file}.merged"
mv "${crate_rows_file}.merged" "$crate_rows_file"

printf '\nPer-crate totals\n'
current_scope=""
while IFS=$'\t' read -r scope crate rust_files tests rust_lines data_files data_bytes; do
  if [[ "$scope" != "$current_scope" ]]; then
    if [[ -n "$current_scope" ]]; then
      printf '\n'
    fi
    folder="$(scope_folder "$scope")"
    printf '[%s]\n' "$folder"
    printf '%-40s %10s %9s %11s %11s %12s\n' \
      'crate' 'rust files' 'tests' 'rust lines' 'data files' 'data size'
    printf '%-40s %10s %9s %11s %11s %12s\n' \
      '-----' '----------' '-----' '----------' '----------' '---------'
    current_scope="$scope"
  fi
  printf '%-40s %10s %9s %11s %11s %12s\n' \
    "$crate" "$rust_files" "$tests" "$rust_lines" "$data_files" "$(human_bytes "$data_bytes")"
done < <(sort -t $'\t' -k1,1 -k2,2 "$crate_rows_file")

printf '\n%-8s %10s %9s %11s %11s %12s\n' \
  'scope' 'rust files' 'tests' 'rust lines' 'data files' 'data size'
printf '%-8s %10s %9s %11s %11s %12s\n' \
  '-----' '----------' '-----' '----------' '----------' '---------'

while IFS=$'\t' read -r scope rust_files tests rust_lines data_files data_bytes; do
  printf '%-8s %10s %9s %11s %11s %12s\n' \
    "$scope" "$rust_files" "$tests" "$rust_lines" "$data_files" "$(human_bytes "$data_bytes")"
done < <(
  awk -F '\t' '
    {
      rust_files[$1] += $3
      tests[$1] += $4
      rust_lines[$1] += $5
      data_files[$1] += $6
      data_bytes[$1] += $7
      total_rust_files += $3
      total_tests += $4
      total_rust_lines += $5
      total_data_files += $6
      total_data_bytes += $7
    }
    END {
      order[1] = "engine"
      order[2] = "game"
      order[3] = "tests"
      for (i = 1; i <= 3; i++) {
        scope = order[i]
        if (rust_files[scope] > 0 || data_files[scope] > 0) {
          printf "%s\t%d\t%d\t%d\t%d\t%.0f\n", \
            scope, rust_files[scope], tests[scope], rust_lines[scope], \
            data_files[scope], data_bytes[scope]
        }
      }
      printf "total\t%d\t%d\t%d\t%d\t%.0f\n", \
        total_rust_files, total_tests, total_rust_lines, \
        total_data_files, total_data_bytes
    }
  ' "$crate_rows_file"
)
