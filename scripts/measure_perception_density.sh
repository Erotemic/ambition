#!/usr/bin/env bash
# Sweep PERCEPTION DENSITY at fixed population and report what each viewer KEPT.
#
# ⭐⭐ WHY THIS EXISTS. `bounded-perception-and-attention.md` measures `kept`
# saturating at ~14.4 across 65 -> 130 bodies and concludes, in its own words,
# that "the hall CANNOT demonstrate why attention is needed - its geometry
# already solves it". The regime an attention budget is FOR is DENSITY: fighters
# packed inside one another's viewports, where `kept` keeps rising. The only way
# to reach that regime in an existing room is to widen the viewport at fixed
# population, which is what AMBITION_PERCEPTION_VIEWPORT_HALF does.
#
# ⛔⛔ THIS REPORTS COUNTS, NOT TIMINGS, AND THAT IS DELIBERATE. On a shared
# build box a wall-clock reading is a reading of who else is compiling: five
# identical hall runs on 2026-09-02 gave byte-identical asset counts and
# frame-spike totals of 61, 4, 9, 6, 52. `kept` and `offered` are counts and are
# reproducible here; ms/tick is not. Take the milliseconds on a quiet machine,
# with interleaved arms, or not at all.
#
# ⛔ ONE AXIS AT A TIME. Population (AMBITION_ACTOR_POPULATION_CAP) and extent
# both move `kept`. This script pins population and varies extent, and prints
# the pinned value on every row so a reader cannot mistake which experiment they
# are holding.
#
# Usage:
#   scripts/measure_perception_density.sh [--population N] [--ticks N] [--extents "a b c"]
set -euo pipefail

POPULATION="${POPULATION:-130}"
# ⛔⛔ THE CAST MUST BE RE-BRAINED OR THERE IS NOTHING TO MEASURE. The hall is
# authored `stand_still`, and increment 1's `PerceptionRequirement::None` gate
# means such a brain never builds a `WorldView` at all — so `perception_census`
# records ZERO views and the census row carries no `kept=` field. Measured
# 2026-09-02: without this the sweep prints "NO CENSUS ROW" for every arm, which
# is the harness correctly refusing to invent data. `medium_striker`'s template
# is one of the two that consume a view.
BRAIN="${BRAIN:-ambition::medium_striker}"
TICKS="${TICKS:-600}"
EXTENTS="${EXTENTS:-480x320 960x640 1440x960 1920x1280 2880x1920}"

while [ $# -gt 0 ]; do
    case "$1" in
        --population) POPULATION="$2"; shift 2 ;;
        --ticks)      TICKS="$2";      shift 2 ;;
        --extents)    EXTENTS="$2";    shift 2 ;;
        --brain)      BRAIN="$2";      shift 2 ;;
        -h|--help)    sed -n '2,25p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

cd "$(git rev-parse --show-toplevel)"

# ⛔ BUILD ONCE, OUTSIDE THE LOOP. A cargo build inside the sweep would put a
# compile between two arms, which is the contention this script exists to avoid
# reporting - and it would do it unevenly, only on the first arm.
echo "building hall_bench (once, before any arm)..." >&2
cargo build --example hall_bench --profile profiling >&2

BIN="target/profiling/examples/hall_bench"
[ -x "$BIN" ] || { echo "hall_bench not at $BIN" >&2; exit 1; }

printf '%-12s %8s %9s %8s %10s   %s\n' extent views offered kept kept_max note
for extent in $EXTENTS; do
    # ⛔ THE CENSUS ROW IS THE LAST ONE: earlier windows include startup, whose
    # `ticks=1` window reads 0.000 for every phase and understated a published
    # figure by up to a third once already.
    row=$(
        AMBITION_PROFILE_CENSUS=1 \
        AMBITION_ACTOR_POPULATION_CAP="$POPULATION" \
        AMBITION_ACTOR_BRAIN_PROFILE="$BRAIN" \
        AMBITION_PERCEPTION_VIEWPORT_HALF="$extent" \
        "$BIN" --ticks "$TICKS" 2>&1 \
        | grep '\[census\] sim_phases' | grep 'kept=' | tail -1 || true
    )
    if [ -z "$row" ]; then
        printf '%-12s %8s %9s %8s %10s   %s\n' "$extent" - - - - \
            "NO CENSUS ROW - is the cast re-brained? a None-requirement brain builds no view"
        continue
    fi
    views=$(sed -n 's/.* views=\([0-9]*\).*/\1/p' <<<"$row")
    offered=$(sed -n 's/.* offered=\([0-9.]*\).*/\1/p' <<<"$row")
    kept=$(sed -n 's/.* kept=\([0-9.]*\).*/\1/p' <<<"$row")
    kept_max=$(sed -n 's/.* kept_max=\([0-9]*\).*/\1/p' <<<"$row")
    printf '%-12s %8s %9s %8s %10s   pop=%s ticks=%s\n' \
        "$extent" "${views:--}" "${offered:--}" "${kept:--}" "${kept_max:--}" \
        "$POPULATION" "$TICKS"
done

cat >&2 <<'NOTE'

⭐ WHAT TO READ. `offered` is what the scan walked and should be flat across
   extents (the scan sees every peer whatever the viewport). `kept` is what each
   viewer actually built a PerceivedActor for, and is the term an attention
   budget bounds. A budget of K is worth having exactly where `kept` rises past
   K and keeps going.
⛔ A run whose `offered` MOVES with extent is measuring something else - the
   population changed under you, or the room did. Stop and find out which.
⚠ These runs are CAPPED (AMBITION_ACTOR_POPULATION_CAP) and WIDENED
   (AMBITION_PERCEPTION_VIEWPORT_HALF). Neither is the shipped hall, and no
   number here describes it.
NOTE
