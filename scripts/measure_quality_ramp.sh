#!/usr/bin/env bash
# The placeholder ramp and the quality transition, at two tiers.
#
# Answers two questions from one pair of runs:
#   1. how many actors draw the unclaimed-body placeholder, per tier;
#   2. what the quality convergence did -- and WHEN, relative to the ramp.
#
# ⛔ (2) is why this prints timestamps rather than counts alone. A convergence
# that fires before anything is realized re-demands nothing, so a run can look
# like it exercises the swap-not-demote path while never entering it.
#
# Usage: qualityramp.sh [room] [warmup]     (needs target/debug/capture_scene)
set -uo pipefail
ROOM=${1:-hall_of_characters}
WARMUP=${2:-400}
OUT=${OUT:-$(mktemp -d)}
mkdir -p "$OUT" || { echo "cannot create $OUT" >&2; exit 2; }
[ -x ./target/debug/capture_scene ] || {
  echo "no target/debug/capture_scene -- cargo build -p ambition_app_tools --bin capture_scene" >&2
  exit 2; }
strip() { sed 's/\x1b\[[0-9;]*m//g'; }
printf '%-8s %11s %10s %s\n' profile placeholders 1st-warn transition
for prof in potato ultra; do
  AMBITION_QUALITY_PROFILE=$prof ./target/debug/capture_scene "$ROOM" player 640x360 \
    --warmup "$WARMUP" > "$OUT/$prof.log" 2>&1
  rc=$?
  # ⛔ A RUN THAT DID NOT HAPPEN MUST NOT PRINT 0. The first version of this
  # script wrote to a missing $OUT, greped a file that was never created, and
  # reported "0 placeholders" for BOTH tiers -- the cleanest possible result,
  # from measuring nothing. Absence and zero are different answers and only one
  # of them is news.
  if [ $rc -ne 0 ] || [ ! -s "$OUT/$prof.log" ]; then
    printf '%-8s %11s %10s  %s\n' "$prof" "NO RUN" "-" "capture_scene rc=$rc, log $(wc -c <"$OUT/$prof.log" 2>/dev/null || echo 0) bytes"
    continue
  fi
  log=$(strip < "$OUT/$prof.log")
  n=$(grep -c "no render family claimed" <<<"$log")
  first=$(grep -m1 "no render family claimed" <<<"$log" | cut -c12-23)
  trans=$(grep -m1 "quality transition to" <<<"$log" | sed 's/.*character_sprites: //')
  ts=$(grep -m1 "quality transition to" <<<"$log" | cut -c12-23)
  printf '%-8s %11s %10s  %s\n' "$prof" "$n" "${first:--}" "${ts:+[$ts] }${trans:-(no transition)}"
done
echo "logs in $OUT"
