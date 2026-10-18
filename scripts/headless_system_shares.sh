#!/usr/bin/env bash
# Where a headless room's simulation spends its main thread, by system, with no
# Tracy and no renderer.
#
#   scripts/headless_system_shares.sh [ROOM] [UPDATES] [SYSTEM]
#
# Runs the profiling binary headless in ROOM (default hall_of_characters) for
# UPDATES app updates (default 1500) under `perf record --call-graph dwarf`,
# keeps the last 60% of the run (past startup), and prints each system's
# inclusive share of the main thread's samples. With SYSTEM (a substring of a
# system's path) it prints that system's callees and leaves instead.
#
# ⭐ WHY NOT TRACY. On 2026-10-18 the Tracy build made the same tick ~5x dearer
# (VM, autoplay route: 21 ms of simulation sets a tick with Tracy, 3.85 ms
# without). A span per system is the cost, and it lands on the cheap systems
# most. Rank systems here; take milliseconds from a `--no-tracy` profile.
#
# ⚠ The "executor" row is real cost, not attribution noise: with ~800 systems
# run a tick, the per-system work (parameter fetch, archetype checks, deferred
# application) was ~28% of the main thread in the hall (2026-10-18), and
# `update_archetypes` another ~7%. See performance-and-composition-findings.md
# (F0, F39).
#
# Build first: cargo build -p ambition_app --bin ambition_game_bin --profile profiling
set -euo pipefail
room="${1:-hall_of_characters}"
updates="${2:-1500}"
focus="${3:-}"
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
bin="${BIN:-$repo_root/target/profiling/ambition_game_bin}"
out="${OUT:-$(mktemp -d)}"
comm="$(basename "$bin" | cut -c1-15)"

if [[ ! -s "$out/perf.data" ]]; then
    AMBITION_PROFILE_CENSUS=1 perf record -q -F 999 --call-graph dwarf,16384 -o "$out/perf.data" -- \
        "$bin" --headless --headless-ticks "$updates" --start-room "$room" > /dev/null 2> "$out/game.err"
fi
if [[ ! -s "$out/stacks.txt" ]]; then
    perf script -i "$out/perf.data" --comm "$comm" -F time,ip,sym --no-inline --time 40%-100% 2>/dev/null \
        | (command -v rustfilt > /dev/null && rustfilt || cat) > "$out/stacks.txt"
fi
echo "perf data and stacks: $out" >&2
python3 "$repo_root/scripts/lib/system_shares.py" "$out/stacks.txt" ${focus:+"$focus"}
