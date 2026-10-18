#!/usr/bin/env bash
# Instructions one simulation tick costs in a room, measured deterministically.
#
#   scripts/sim_tick_instructions.sh [ROOM] [BIN...]
#
# Runs `headless --bench` (exactly one fixed 60 Hz tick per step, idle input)
# for LO and HI ticks (default 300 and 1500) under `perf stat -e
# instructions:u`, and prints (HI - LO) / (HI - LO ticks): startup cancels, and
# the same tick count does the same work on every run, so two binaries compare
# to well under a percent where the wall-time headless runs (`--headless` of
# the game binary, which catches up on wall time) scattered by ±15%.
# Instructions count every thread, so background loaders and file watchers are
# in it; they are small next to the simulation once the room is loaded.
#
# BIN defaults to target/profiling/headless:
#   cargo build -p ambition_app_tools --bin headless --profile profiling
set -euo pipefail
room="${1:-hall_of_characters}"
shift || true
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
bins=("$@")
[[ ${#bins[@]} -gt 0 ]] || bins=("$repo_root/target/profiling/headless")
lo="${LO:-300}"
hi="${HI:-1500}"

count() {
    perf stat -x, -e instructions:u -- "$1" "$2" --bench --start-room "$room" 2>&1 >/dev/null \
        | awk -F, '/instructions/ {print $1}'
}

for bin in "${bins[@]}"; do
    a="$(count "$bin" "$lo")"
    b="$(count "$bin" "$hi")"
    python3 -c "print(f'{(int($b) - int($a)) / ($hi - $lo) / 1e6:.3f} Minstr/tick  $room  $bin')"
done
