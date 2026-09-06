#!/usr/bin/env bash
# The instrumentation tax: what `--features profile` (bevy/trace + trace_tracy)
# costs per frame, measured on the headless production host with nothing else
# different. Four arms, interleaved per repetition, same `profiling` cargo profile:
#
#   notrace  : built WITHOUT the feature into target/notrace (hardlink-seeded from
#              target/profiling so only the affected crates rebuild)
#   nolog    : the `profile` binary as-is -- spans on, no tracy-capture attached
#   filtered : the same binary with RUST_LOG=error, which filters spans at the layer
#   cap      : the same binary with tracy-capture attached (what profile_desktop.sh does)
#
# Measured 2026-09-01 (docs/planning/engine/performance-and-iteration.md):
# 1.5 / 3.7 / 1.8 / 4.2 ms per frame. Run this again after any change to the
# tracing layer, the span count, or the Tracy pin, and compare.
#
#   scripts/instrumentation_tax.sh              # builds both binaries, 2 reps x 4000 ticks
#   TICKS=6000 REPS=3 scripts/instrumentation_tax.sh --no-build
#
# Needs tracy-capture matching the game's Tracy (scripts/setup/install_profiling_tools.sh).
# Do not run it while a cargo build is running on the same machine: the numbers
# are wall time and a build owns the cores.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
ticks="${TICKS:-4000}"; reps="${REPS:-2}"; build=yes
[[ "${1:-}" == "--no-build" ]] && build=no
out="${OUT:-$repo_root/dev/ambition_dev_measurements/profiles/instrumentation-tax-$(date -u +%Y%m%dT%H%M%SZ)}"
mkdir -p "$out"
TRACE=target/profiling/ambition_game_bin
NOTRACE=target/notrace/profiling/ambition_game_bin

if [[ "$build" == yes ]]; then
    CARGO_INCREMENTAL=0 cargo build -p ambition_app --bin ambition_game_bin --profile profiling --features profile
    if [[ ! -d target/notrace/profiling/deps ]]; then
        # deps/ are content-hashed and replaced on rebuild, so hardlinks are safe;
        # .fingerprint is rewritten in place and must be a copy (see agent_worktree.sh seed).
        mkdir -p target/notrace/profiling
        cp -al target/profiling/deps target/notrace/profiling/deps
        cp -a target/profiling/.fingerprint target/profiling/build target/notrace/profiling/
    fi
    CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=target/notrace cargo build -p ambition_app --bin ambition_game_bin --profile profiling
fi
command -v tracy-capture >/dev/null || { echo "tracy-capture not installed (scripts/setup/install_profiling_tools.sh)" >&2; exit 2; }

export AMBITION_PROFILE_CENSUS=1
# A VM without an invariant TSC refuses to start Tracy; ratios stay sound.
grep -q 'nonstop_tsc' /proc/cpuinfo || export TRACY_NO_INVARIANT_CHECK=1

summarize() { # arm rep t0 t1 t2 log
python3 - "$@" <<'PY'
import sys, re
arm, rep, t0, t1, t2, log = sys.argv[1:]
wall = float(t1) - float(t0); drain = float(t2) - float(t1)
rows = []
for line in open(log, errors="replace"):
    m = re.search(r"\[census\] phases t=\S+ frames=(\d+) (.*)", line)
    if m:
        d = dict(kv.split("=") for kv in m.group(2).split())
        rows.append((int(m.group(1)), {k: float(v) for k, v in d.items()}))
rows = rows[2:]  # warm-up windows
med = lambda xs: sorted(xs)[len(xs) // 2] if xs else float("nan")
frame = med([sum(r.values()) for _, r in rows]); pre = med([r["PreUpdate"] for _, r in rows])
print(f"{arm:9s} rep{rep} wall={wall:6.2f}s exit->capture-done={drain:5.1f}s  median window: frames/s={med([f for f,_ in rows]):5.0f} frame={frame:.3f} ms PreUpdate={pre:.3f} ms", flush=True)
PY
}

run_arm() {
    local arm="$1" rep="$2" bin="$TRACE" cap="" log="$out/$1.$2.stderr"
    local -a envs=()
    case "$arm" in
        notrace) bin="$NOTRACE" ;;
        nolog) ;;
        filtered) envs+=(RUST_LOG=error) ;;
        cap) tracy-capture -o "$out/cap.$rep.trace" -f > "$out/cap.$rep.log" 2>&1 & cap=$!; sleep 1 ;;
    esac
    local t0 t1 t2
    t0=$(date +%s.%N)
    env "${envs[@]}" "$bin" --headless --headless-ticks "$ticks" > /dev/null 2> "$log" || true
    t1=$(date +%s.%N)
    [[ -n "$cap" ]] && wait "$cap"
    t2=$(date +%s.%N)
    summarize "$arm" "$rep" "$t0" "$t1" "$t2" "$log" | tee -a "$out/results.txt"
}

echo "ticks=$ticks reps=$reps out=$out" | tee "$out/results.txt"
for rep in $(seq 1 "$reps"); do
    for arm in notrace nolog filtered cap; do run_arm "$arm" "$rep"; done
done
echo "results: $out/results.txt"
