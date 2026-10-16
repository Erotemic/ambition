#!/usr/bin/env bash
# Interleaved A/B/C over the main-world executor policy of standalone Mary-O
# (findings page `docs/planning/engine/performance-and-composition-findings.md`,
# P2 and F22). One binary, one variable: AMBITION_SCHEDULE_EXECUTOR =
# parallel | frame-serial | serial. Arms are interleaved (rep1: a,b,c; rep2:
# a,b,c), so load from other work on a shared box spreads over all arms.
#
# Usage: scripts/executor_ab.sh [OUTDIR]; then scripts/executor_ab_report.py OUTDIR
#   REPS=<n>    reps per arm          [3]
#   WARMUP=<n>  frames before the shot [1200]
# Build first:
#   cargo build --profile profiling -p ambition_demo_mary_o_app --features capture --bin capture_mary_o
# Remove with the switch, after the decision.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPS="${REPS:-3}"
WARMUP="${WARMUP:-1200}"
OUT="${1:-$ROOT/target/executor_ab}"
BIN="$ROOT/target/profiling/capture_mary_o"
mkdir -p "$OUT"
cd "$ROOT"
for rep in $(seq 1 "$REPS"); do
  for arm in parallel frame-serial serial; do
    log="$OUT/${arm}.${rep}"
    /usr/bin/time -f "time wall=%e user=%U sys=%S" \
      env AMBITION_PROFILE_CENSUS=1 AMBITION_SCHEDULE_EXECUTOR="$arm" \
      timeout 1200 "$BIN" "$log.png" --warmup "$WARMUP" > "$log.out" 2> "$log.err"
    echo "$arm rep=$rep exit=$? $(grep '^time ' "$log.err")"
  done
done
