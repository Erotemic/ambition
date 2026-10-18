#!/usr/bin/env bash
# Mean frame time of a room standing still, rendered on the CPU (llvmpipe).
#
#   scripts/room_frame_cost_software.sh ROOM [QUALITY] [SECONDS]
#
# On a machine with no GPU (the agent VM) the renderer is llvmpipe, which runs
# every shader on the CPU: what a fragment shader costs a GPU becomes wall
# time here. A room's frame time against another room's, or against the same
# room at another quality tier, says which one draws more; the milliseconds
# are not a GPU's. Runs the game window under xvfb at 1600x900, waits for the
# room to settle (the first half of SECONDS), and prints the mean of the
# census frame windows after that.
#
# Other work on the machine moves it a lot: two runs of one room 40 minutes
# apart, with builds running, gave 132 and 90 ms (2026-10-18). Compare runs
# taken back to back on a quiet machine.
#
# BIN defaults to target/profiling/ambition_game_bin:
#   cargo build -p ambition_app --bin ambition_game_bin --profile profiling
set -euo pipefail
room="${1:?room id}"
quality="${2:-high}"
seconds="${3:-40}"
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
bin="${BIN:-$repo_root/target/profiling/ambition_game_bin}"
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
AMBITION_PROFILE_CENSUS=1 AMBITION_QUALITY_PROFILE="$quality" \
    timeout "$seconds" xvfb-run -a -s "-screen 0 1600x900x24" "$bin" --start-room "$room" >"$log" 2>&1 || true
grep -q 'device_type: Cpu' "$log" || echo "warning: the adapter is not llvmpipe; these are not software-render numbers" >&2
awk -v from="$((seconds / 2))" '
    /\[census\] frame / {
        for (i = 1; i <= NF; i++) { split($i, kv, "="); v[kv[1]] = kv[2] }
        if (v["t"] + 0 >= from) { total += v["mean"] * v["frames"]; n += v["frames"] }
    }
    END {
        if (n == 0) { print "no census frames after " from " s" > "/dev/stderr"; exit 1 }
        printf "%.1f ms/frame over %d frames  '"$room"' '"$quality"'\n", total / n, n
    }' "$log"
