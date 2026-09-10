#!/usr/bin/env python3
"""The three exact-LOCKSTEP duels, re-fought at five authored rungs.

⭐ **THE QUESTION.** `measure_duel_roster.py` found three mirror bouts whose two
seats are BIT-IDENTICAL — same damage, same starts, ratio exactly 1.00 — which
makes them unmeasurable as fighter results. The candidate on the table is that
the per-seat `fighter_cognition_seed` is CONSUMED but INEFFECTIVE at rung 9:

    decision.rs:472   (sample * profile.execution_noise * cfg.interval()).round()

⛔⛔ **AND RUNG 9 IS THE ONE RUNG WHERE THAT IS ZERO FOR EVERY POSSIBLE SAMPLE
— by 3e-8 of float slack.** `execution_noise = 0.45 - t*0.35` (profile.rs:75)
with `t = (level-1)/8`, and `interval()` is 5 (`DEFAULT_DECISION_INTERVAL_TICKS`).
`|sample|` is `next_signed_unit().abs()`, which reaches EXACTLY 1.0. So the
product's ceiling per rung, computed in f32 the way the shipped code does:

| rung | execution_noise | noise x interval | max jitter | P(jitter>0) | L3 rollouts |
|---:|---|---:|---:|---:|---|
| 3 | 0.36250 | 1.8125 | 2 | 0.72 | off |
| 5 | 0.27500 | 1.3750 | 1 | 0.64 | off |
| 6 | 0.23125 | 1.1562 | 1 | 0.57 | **on** |
| 8 | 0.14375 | 0.7187 | 1 | 0.30 | on |
| 9 | 0.10000 | **0.49999997** | **0** | **0.00** | on |

⇒ At rung 9 the ceiling is `0.4999999701976776`, which `round()` takes to 0 for
every sample including 1.0. **The jitter is not small at the top rung, it is
identically zero, and it lands 3e-8 under the tie point rather than at it.**

⛔⛔ **WHICH ALREADY REFUTES THE STRONG FORM OF THE CANDIDATE, using the sweep
we have and no new runs.** Every one of the 20 measured fighters duelled at rung
9, so jitter was zero in ALL twenty bouts — and SEVENTEEN of them diverged
anyway. ⇒ **A dead jitter is not sufficient for lockstep**, so it cannot be what
separates the three from the seventeen. Something else breaks symmetry in the
seventeen, and this sweep does not name it.

⭐ **WHAT THIS SWEEP CAN STILL SETTLE** is the weaker and still-useful claim:
whether a LIVE jitter is sufficient to BREAK lockstep. Rung 8 is the sharpest
version — nearest rung to 9, L3 rollouts held ON across both, so jitter is very
nearly the only thing that changes. Rungs 3 and 5 sit below the rollout
threshold and change more than one thing at once.

⚠ **RUNG 6 IS A CONFOUNDED STEP.** `rollout_depth`/`rollout_k` switch on at
level >= 6, so a 5 -> 6 difference is not a noise difference. It is carried
because it brackets that boundary, not because it isolates anything.

⛔⛔ **AND ONE OF THE THREE SUBJECTS CANNOT ANSWER THE QUESTION.**
`npc_carl_stargan` is a bare registration — he authors no body, no policy and no
moveset, presses 3 moves in a whole duel and spends 0.5% of it in reach. He is
lockstep because he does almost nothing, not because noise failed to reach him.
⇒ **His staying lockstep at rung 3 is NOT evidence about the jitter.** He is
carried as a NEGATIVE CONTROL: he should stay lockstep at every rung, and if he
separates, the jitter reaches even a 3-start fighter, which is a finding about
the instrument. The load-bearing subjects are `medic` and `special_patent_clerk`,
both of which press ~50 moves per seat.

⛔⛔ **AND THE SWEEP HAS A CONFOUND THAT LOOKS EXACTLY LIKE ITS ANSWER.** The
jitter is drawn per seat from `fighter_cognition_seed`. If a mirror bout hands
BOTH seats the same seed, a live jitter is applied symmetrically and lockstep
survives at every rung — which reads as "still bit-identical at rung 3" and
would be scored as killing the candidate, when what it measured is that the
seeds are equal. ⇒ **Every row asserts the two seats' seeds DIFFER**, read from
the harness's own `[brain]` probe, and a row that cannot show two distinct seeds
is UNMEASURABLE rather than lockstep.

⚠ **THE RATIO IS NOT THE MEASUREMENT.** Several mechanisms died on this row
behind a per-seat ratio, so every row here prints both seats' starts, damage and
ABSOLUTE FIRST-SEEN TICK. A bout that fails to seat produces a perfect-looking
1.00 for the wrong reason; such a run is reported UNMEASURABLE and never as
lockstep.

    python3 scripts/measure_duel_rung_sensitivity.py --run   # slow, 15 duels
    python3 scripts/measure_duel_rung_sensitivity.py         # fold the log
"""

from __future__ import annotations

import argparse
import collections
import os
import pathlib
import re
import subprocess

REPO = pathlib.Path(__file__).resolve().parents[1]
LOG = REPO / "target/duel_rung_sweep.log"

# The three bit-identical bouts from `measure_duel_roster.py`, plus the reason
# each is here. Carl is the control; the other two carry the question.
SUBJECTS = {
    "medic": "load-bearing (49 starts/seat, 25% in reach)",
    "special_patent_clerk": "load-bearing (51 starts/seat, 19% in reach)",
    "npc_carl_stargan": "NEGATIVE CONTROL (bare registration, 3 starts, 0.5%)",
}
RUNGS = (3, 5, 6, 8, 9)
PROBE = re.compile(r"^\[(duel|gap|body|stance|moves|dealt|brain)\]")


def sweep() -> None:
    with LOG.open("w", encoding="utf-8") as out:
        for fighter in SUBJECTS:
            for rung in RUNGS:
                out.write(f"=== {fighter} rung{rung}\n")
                out.flush()
                proc = subprocess.run(
                    [
                        "cargo", "test", "-q", "-p", "ambition_app", "--test", "app_it",
                        "--features", "rl_sim", "--",
                        "smash_cpus_damage_each_other::two_cpus", "--nocapture",
                    ],
                    capture_output=True, text=True, cwd=REPO,
                    env={
                        **os.environ,
                        "AMBITION_DUEL_FIGHTER": fighter,
                        "AMBITION_DUEL_RUNG": str(rung),
                    },
                )
                for line in (proc.stdout + proc.stderr).split("\n"):
                    if PROBE.match(line) or "panicked at" in line:
                        out.write(line + "\n")
                out.flush()
        out.write("SWEEP DONE\n")


def parse() -> dict[tuple[str, int], dict]:
    rows: dict[tuple[str, int], dict] = {}
    cur: tuple[str, int] | None = None
    for line in LOG.read_text(encoding="utf-8").split("\n"):
        if line.startswith("==="):
            _, fighter, rung = line.split()
            cur = (fighter, int(rung.removeprefix("rung")))
            rows[cur] = {"panics": []}
            continue
        if cur is None:
            continue
        row = rows[cur]
        if m := re.search(r"= ([\d.]+) / ([\d.]+) per minute", line):
            row["dmg"] = (float(m.group(1)), float(m.group(2)))
        if m := re.search(r"ran (\d+) ticks", line):
            row["window"] = int(m.group(1))
        if m := re.search(r"first seen at absolute tick (\S+) / (\S+)", line):
            row["first"] = (m.group(1).strip(";"), m.group(2).strip(";"))
        if m := re.match(r"^\[moves\] seat (\d): (\d+) starts across (\d+)", line):
            row.setdefault("starts", []).append(int(m.group(2)))
            row.setdefault("distinct", []).append(int(m.group(3)))
        if m := re.match(r"^\[dealt\] seat (\d): ([\d.]+) damage", line):
            row.setdefault("dealt", []).append(float(m.group(2)))
        if m := re.match(r"^\[brain\] seat (\d): noise=(\S+)", line):
            row.setdefault("seeds", []).append(m.group(2))
        if "panicked at" in line:
            row["panics"].append(line)
    return rows


def verdict(row: dict) -> str:
    """LOCKSTEP / SEPARATED / UNMEASURABLE — and the third is not the first.

    ⛔ A bout that never seated both fighters has no seats to compare, so its
    columns are trivially equal. That is UNMEASURABLE, not lockstep.
    """
    if "window" not in row or len(row.get("starts", [])) != 2:
        return "UNMEASURABLE"
    if row["window"] < 1000:
        return "UNMEASURABLE"
    # ⛔⛔ TWO SEATS ON THE SAME SEED CANNOT ANSWER THIS QUESTION AT ANY RUNG.
    # The jitter is drawn per seat from `fighter_cognition_seed`; if both seats
    # were handed the SAME seed, a live jitter is applied SYMMETRICALLY and
    # lockstep survives every rung. Such a run would read as "still bit-identical
    # at rung 3" and be scored as killing the candidate, when what it actually
    # measured is that the seeds are equal. A run whose seats' seeds are equal --
    # or a run whose seeds we could not read -- is UNMEASURABLE for this row.
    seeds = row.get("seeds", [])
    if len(seeds) != 2:
        return "UNMEASURABLE"
    if seeds[0] == seeds[1]:
        return "UNMEASURABLE"
    same = (
        row["starts"][0] == row["starts"][1]
        and row.get("dealt", [0, 1])[0] == row.get("dealt", [0, 1])[1]
        and row.get("distinct", [0, 1])[0] == row.get("distinct", [0, 1])[1]
    )
    return "LOCKSTEP" if same else "SEPARATED"


def fold() -> int:
    if not LOG.exists():
        print(f"no log at {LOG}; run with --run")
        return 1
    rows = parse()
    print(f"{'fighter':<22}{'rung':>5}{'starts':>12}{'dealt':>16}"
          f"{'first seen':>14}{'window':>8}{'seeds':>8}  verdict")
    by_fighter: dict[str, list[str]] = collections.defaultdict(list)
    for (fighter, rung), row in rows.items():
        v = verdict(row)
        by_fighter[fighter].append(v)
        starts = "/".join(str(s) for s in row.get("starts", [])) or "-"
        dealt = "/".join(f"{d:g}" for d in row.get("dealt", [])) or "-"
        first = "/".join(row.get("first", ())) or "-"
        seeds = row.get("seeds", [])
        seed_col = ("differ" if len(seeds) == 2 and seeds[0] != seeds[1]
                    else "SAME" if len(seeds) == 2 else "-")
        print(f"{fighter:<22}{rung:>5}{starts:>12}{dealt:>16}"
              f"{first:>14}{row.get('window', '-'):>8}{seed_col:>8}  {v}")

    print()
    for fighter, note in SUBJECTS.items():
        print(f"  {fighter}: {note}")
    print()

    # ⛔ Index the rows directly. Folding `by_fighter` in log order assumes the
    # log was written in RUNGS order, which is a fact about the writer and not
    # about the run being read.
    load_bearing = [f for f, n in SUBJECTS.items() if n.startswith("load-bearing")]
    at_three = {f: verdict(rows[(f, 3)]) if (f, 3) in rows else None
                for f in load_bearing}
    # ⛔ UNMEASURABLE is not disagreement. A rung-3 row that never seated says
    # nothing about the candidate, and reading it as a SPLIT would report a
    # finding about the harness as a finding about the fighters.
    if any(v is None or v == "UNMEASURABLE" for v in at_three.values()):
        missing = [f for f, v in at_three.items() if v is None or v == "UNMEASURABLE"]
        print(f"⚠ INCOMPLETE: no usable rung-3 row for {', '.join(missing)};")
        print("   the rounding candidate is neither killed nor supported.")
        return 1
    if all(v == "LOCKSTEP" for v in at_three.values()):
        print("⛔ A LIVE JITTER DOES NOT BREAK LOCKSTEP. Both load-bearing")
        print("   fighters are still bit-identical at rung 3, where the jitter")
        print("   is nonzero on ~72% of decisions. The seed is not the")
        print("   variable at any rung, and the candidate is fully dead.")
    elif all(v == "SEPARATED" for v in at_three.values()):
        print("⭐ Both load-bearing fighters SEPARATE at rung 3 — a live jitter")
        print("   IS sufficient to break lockstep. ⛔ That does NOT make it the")
        print("   cause: jitter is dead at rung 9 for all 20 fighters and 17 of")
        print("   them diverged anyway. Read the rung-8 row — it is the same")
        print("   result with L3 rollouts held constant.")
    else:
        print("⚠ SPLIT at rung 3 — the fighters disagree, so the cause is not a")
        print("   property of the rung alone. Read the rows, not this line.")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--run", action="store_true", help="run the 12 duels (slow)")
    args = ap.parse_args()
    if args.run:
        sweep()
    return fold()


if __name__ == "__main__":
    raise SystemExit(main())
