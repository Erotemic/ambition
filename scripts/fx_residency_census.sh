#!/usr/bin/env bash
# Which FX sheets are resident in a room, headless — the `resident by road`
# census of the shipped program after N ticks in ROOM, with the per-target
# `[image]` lines for the fx-sheet road so the answer names sheets, not a sum.
#
#   scripts/fx_residency_census.sh                          # central_hub_complex, 600 ticks
#   ROOM=hall_of_characters TICKS=900 scripts/fx_residency_census.sh
#
# Builds the profiling binary into target/notrace (NO_BUILD=1 to skip), like
# headless_room_frame.sh. Potato tier on llvmpipe/no GPU: read COUNTS, not MP.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$repo_root"
ROOM="${ROOM:-central_hub_complex}"; TICKS="${TICKS:-600}"
BIN=target/notrace/profiling/ambition_game_bin
if [[ "${NO_BUILD:-0}" != 1 ]]; then
    CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=target/notrace cargo build -q -p ambition_app --bin ambition_game_bin --profile profiling
fi
log="$(mktemp -p "${TMPDIR:-/tmp}" fx-census-XXXX.stderr)"
AMBITION_PROFILE_CENSUS=1 AMBITION_HEADLESS_GAMEPLAY_ROOM="$ROOM" "$BIN" --headless --headless-ticks "$TICKS" >/dev/null 2>"$log"
echo "room=$ROOM ticks=$TICKS  ($log)"
grep -E 'resident by road' "$log" | tail -1
echo "fx-sheet demands, in order:"
grep -E '^\[image\] .*via fx-sheet' "$log" | sed -E 's/^\[image\] +([0-9.]+)s +f *([0-9]+) .* (\S+_spritesheet\.png) .*/  f\2 (\1s) \3/' 
