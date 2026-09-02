#!/usr/bin/env bash
# Interleaved A/B/C over the two asset-residency levers, for a machine with no
# GPU. Reads `[image-census]`, `[image-gpu]` and `[frame-spike]` out of one
# `capture_scene` run per arm per rep.
#
# ⛔ ARMS ARE INTERLEAVED (rep1: a,b,c; rep2: a,b,c; ...) rather than blocked
# (a,a,a then b,b,b). On a shared or virtualised box a blocked layout attributes
# whatever else was running during arm A to arm A; interleaving spreads that
# noise across all three arms instead of donating it to one.
#
# Usage: scripts/asset_pacer_ab.sh [OUTDIR]
#   ROOM=<id>   room to stage        [hall_of_characters]
#   REPS=<n>    reps per arm         [3]
#   WARMUP=<n>  frames before shot   [400]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOM="${ROOM:-hall_of_characters}"
REPS="${REPS:-3}"
WARMUP="${WARMUP:-400}"
OUTDIR="${1:-${ROOT}/target/tmp/asset-pacer-ab}"
BIN="${ROOT}/target/debug/capture_scene"

[ -x "$BIN" ] || { echo "error: $BIN not built (cargo build -p ambition_app_tools --bin capture_scene)" >&2; exit 1; }
mkdir -p "$OUTDIR"

# Arm name -> the environment that defines it. "default" sets nothing.
arm_env() {
    case "$1" in
        default)  echo "" ;;
        pacer64)  echo "AMBITION_RENDER_ASSET_MB_PER_FRAME=64" ;;
        cpucopy)  echo "AMBITION_IMAGES_RENDER_WORLD_ONLY=0" ;;
        *) echo "unknown arm: $1" >&2; exit 1 ;;
    esac
}

echo "sha        $(git -C "$ROOT" rev-parse --short HEAD)"
echo "room       $ROOM"
echo "reps       $REPS   warmup $WARMUP"
echo "outdir     $OUTDIR"
echo

for rep in $(seq 1 "$REPS"); do
    for arm in default pacer64 cpucopy; do
        log="$OUTDIR/${arm}-rep${rep}.log"
        png="$OUTDIR/${arm}-rep${rep}.png"
        echo "--- rep $rep arm $arm -> $log"
        # `env` with an empty string would export a bogus var, so branch on it.
        e="$(arm_env "$arm")"
        (
            cd "$ROOT/game/ambition_app_tools"
            if [ -n "$e" ]; then
                env "$e" /usr/bin/time -v "$BIN" "$ROOM" player "$png" 640x360 --warmup "$WARMUP"
            else
                /usr/bin/time -v "$BIN" "$ROOM" player "$png" 640x360 --warmup "$WARMUP"
            fi
        ) >"$log" 2>&1 || echo "  ⚠ arm exited non-zero (see $log)"
    done
done

echo
echo "runs written to $OUTDIR; summarise with scripts/asset_pacer_ab_report.py"
