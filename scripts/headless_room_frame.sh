#!/usr/bin/env bash
# The WHOLE shipped host — every schedule the windowed binary runs, minus the
# render app — headless in one room, no Tracy: the census phase split per frame.
# This is the closest thing to the windowed main-thread cost that a machine
# without a GPU can measure, and it is on the shipped program (the `profiling`
# cargo profile WITHOUT `--features profile`; see instrumentation_tax.sh).
#
#   scripts/headless_room_frame.sh                       # hall_of_characters, 3000 ticks, 3 reps
#   ROOM=central_hub_complex TICKS=2000 REPS=2 scripts/headless_room_frame.sh
#   CAPS="2 130" scripts/headless_room_frame.sh          # per population cap (AMBITION_ACTOR_POPULATION_CAP)
#
# Builds into target/notrace unless NO_BUILD=1. Reps are interleaved across caps;
# the first rep is printed but should be read as the page-cache warm-up.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$repo_root"
ROOM="${ROOM:-hall_of_characters}"; TICKS="${TICKS:-3000}"; REPS="${REPS:-3}"; CAPS="${CAPS:-}"
BIN=target/notrace/profiling/ambition_game_bin
if [[ "${NO_BUILD:-0}" != 1 ]]; then
    if [[ ! -d target/notrace/profiling/deps && -d target/profiling/deps ]]; then
        mkdir -p target/notrace/profiling
        cp -al target/profiling/deps target/notrace/profiling/deps
        cp -a target/profiling/.fingerprint target/profiling/build target/notrace/profiling/
    fi
    CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=target/notrace cargo build -p ambition_app --bin ambition_game_bin --profile profiling
fi
out="${OUT:-$repo_root/dev/ambition_dev_measurements/profiles/headless-room-frame-$(date -u +%Y%m%dT%H%M%SZ)}"; mkdir -p "$out"
echo "room=$ROOM ticks=$TICKS reps=$REPS caps=${CAPS:-<uncapped>} out=$out" | tee "$out/results.txt"
for rep in $(seq 1 "$REPS"); do
  for cap in ${CAPS:-uncapped}; do
    log="$out/$cap.rep$rep.stderr"
    envs=(AMBITION_PROFILE_CENSUS=1 AMBITION_HEADLESS_GAMEPLAY_ROOM="$ROOM")
    [[ "$cap" != uncapped ]] && envs+=(AMBITION_ACTOR_POPULATION_CAP="$cap")
    env "${envs[@]}" "$BIN" --headless --headless-ticks "$TICKS" > "$out/$cap.rep$rep.stdout" 2> "$log"
    python3 - "$cap" "$rep" "$log" <<'PY' | tee -a "$out/results.txt"
import sys, re
cap, rep, log = sys.argv[1:]
rows = []; measuring = False
for line in open(log, errors="replace"):
    if "headless gameplay in" in line: measuring = True
    m = re.search(r"\[census\] phases t=\S+ frames=(\d+) (.*)", line)
    if m and measuring:
        d = {k: float(v) for k, v in (kv.split("=") for kv in m.group(2).split())}
        rows.append((int(m.group(1)), d))
rows = rows[1:]  # the window that straddles the room entry
med = lambda xs: sorted(xs)[len(xs)//2] if xs else float("nan")
keys = ["First","PreUpdate","StateTransition","RunFixedMainLoop","Update","SpawnScene","PostUpdate","Last","outside"]
frame = med([sum(d.values()) for _, d in rows])
parts = " ".join(f"{k}={med([d.get(k,0) for _,d in rows]):.3f}" for k in keys)
print(f"cap={cap:8s} rep{rep} windows={len(rows)} frame={frame:.3f} ms  {parts}", flush=True)
PY
  done
done
echo "results: $out/results.txt"
