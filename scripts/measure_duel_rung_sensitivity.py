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

⛔⛔ **AND IT READS `seed=`, NOT `now=`.** `FighterState::new` stores the seed in
`state.noise` and advances it on every draw, so the end-of-bout value is a
STREAM POSITION. Two seats can share a seed and still end on different
positions by drawing a different NUMBER of samples — which is what a divergent
bout does. ⇒ Reading the end position would pass a shared-seed bout as "seeds
differ" at rungs 3, 5, 6 and 8, the rungs this sweep exists to measure. The
check is sound in one direction only (equal end positions DO imply a shared
stream), and that is the direction it does not need. There is no fallback to
`now=`, because the fallback is the defect.

⚠ **THE RATIO IS NOT THE MEASUREMENT.** Several mechanisms died on this row
behind a per-seat ratio, so every row here prints both seats' starts, damage and
ABSOLUTE FIRST-SEEN TICK. A bout that fails to seat produces a perfect-looking
1.00 for the wrong reason; such a run is reported UNMEASURABLE and never as
lockstep.

⭐⭐ **MEASURED 2026-09-10, all 15 rows, every one carrying two DISTINCT
`seed=` values so the shared-stream confound is excluded by reading rather than
by inference:**

| fighter | 1 | 3 | 5 | 6 | 9 |
|---|---|---|---|---|---|
| `medic` | SEP | SEP | SEP | SEP | **LOCKSTEP** |
| `special_patent_clerk` | SEP | SEP | SEP | SEP | **LOCKSTEP** |
| `npc_carl_stargan` (control) | SEP | SEP | **LOCKSTEP** | SEP | **LOCKSTEP** |

⇒ **Both load-bearing fighters are monotone and break exactly where the jitter's
ceiling reaches zero.** A live jitter IS sufficient to break lockstep. ⛔ It is
NOT the cause of the roster's lockstep rows: the jitter is dead at rung 9 for
ALL twenty fighters and seventeen of them diverged anyway.

⚠ **AND THE CONTROL WAS NEVER A CONTROL.** Carl is LOCKSTEP at rung 5 with
**7/7 starts and 0/0 damage**, and SEPARATED at rungs 1, 3 and 6 where he takes
20-44. ⇒ He does not hold "acts normally" fixed while the jitter varies -- he
varies both -- so his rows measure his ACTIVITY, not the mechanism. **A control
has to hold the mechanism's PRECONDITION fixed, and enough behaviour for a
one-tick nudge to matter is a precondition here.** The reading that fits is that
Carl measures the FLOOR of the effect rather than its absence; that is a reading
chosen after seeing the data and is labelled as one.

⛔ **An open content question this sweep raised and did not answer:** Carl deals
55/62 damage at rung 3 and 44/66 at rung 6, and he is in
`KNOWN_BARE_REGISTRATIONS` -- the exemption list for ids that author NOTHING.
A character with no authored body, policy or moveset is fighting. Filed on
D-CPU-INERT as an observation.

    python3 scripts/measure_duel_rung_sensitivity.py --run   # slow, 15 duels
    python3 scripts/measure_duel_rung_sensitivity.py         # fold the log
    python3 scripts/measure_duel_rung_sensitivity.py --fill  # only missing rows
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
RUNGS = (1, 3, 5, 6, 9)
PROBE = re.compile(r"^\[(duel|gap|body|stance|moves|dealt|brain)\]")


def sweep(only: set[tuple[str, int]] | None = None, append: bool = False) -> None:
    with LOG.open("a" if append else "w", encoding="utf-8") as out:
        for fighter in SUBJECTS:
            for rung in RUNGS:
                if only is not None and (fighter, rung) not in only:
                    continue
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
                # ⛔ CAPTURE THE PANIC'S MESSAGE, NOT JUST ITS HEADER. The
                # header says a duel failed; the line after it says WHY, and
                # "the match was decided at tick 900" is a result while "the
                # seating never happened" is a broken run. Dropping the message
                # makes those two share the verdict UNMEASURABLE.
                lines = (proc.stdout + proc.stderr).split("\n")
                for i, line in enumerate(lines):
                    if PROBE.match(line):
                        out.write(line + "\n")
                    elif "panicked at" in line:
                        out.write(line + "\n")
                        for follow in lines[i + 1 : i + 5]:
                            if follow.startswith("note:") or not follow.strip():
                                break
                            out.write(f"[why] {follow.strip()}\n")
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
        # ⛔⛔ `seed=`, NEVER `now=`. `FighterState::new` stores the seed IN
        # `state.noise` and advances it on every draw, so the end-of-bout value
        # is a STREAM POSITION. Two seats can start from the SAME seed and end
        # on different positions merely by drawing a different NUMBER of
        # samples -- which is what happens at every rung where the bout
        # diverges. ⇒ Reading `now=` would pass a shared-seed bout as "seeds
        # differ" at exactly rungs 3, 5, 6 and 8, the rungs this sweep exists
        # to measure, and the confound would walk through the check built to
        # exclude it. A row with no `seed=` is UNMEASURABLE; there is no
        # fallback, because the fallback is the defect.
        # ⚠ Capture whatever `seed=` holds, INCLUDING the harness's
        # `<no fighter brain at birth>` sentinel. Matching only hex would drop
        # that seat silently and the row would read as an unparseable log --
        # "these two seats have distinct streams" and "one of these seats has
        # no stream" are different facts and must not share a verdict.
        if m := re.match(r"^\[brain\] seat (\d): .*?\bseed=(\S+|<[^>]*>)", line):
            row.setdefault("seeds", []).append(m.group(2))
        if line.startswith("[why]"):
            row.setdefault("why", []).append(line[len("[why] "):])
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
    if any(not sd.startswith("0x") for sd in seeds):
        # A seat born on something other than a fighter brain has no stream at
        # all, so "the seeds differ" is not a question about it.
        return "NO_FIGHTER_BRAIN"
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
    # ⛔ A LOG OUTLIVES THE SWEEP THAT WROTE IT. `RUNGS` changed once already --
    # rung 8 was dropped when it turned out to name no published policy -- and
    # its stale blocks are still in the log. Folding them would print rows from
    # a configuration this script no longer sweeps, which is a table quietly
    # describing a different experiment.
    stale = {r for (_, r) in rows if r not in RUNGS}
    if stale:
        print(f"⚠ ignoring {sorted(stale)}: rung(s) in the log that this sweep "
              f"no longer covers; re-run with --run for a clean log\n")
        rows = {k: v for k, v in rows.items() if k[1] in RUNGS}
    print(f"{'fighter':<22}{'rung':>5}{'starts':>12}{'dealt':>12}"
          f"{'first seen':>14}{'window':>8}{'seeds':>9}  verdict")
    by_fighter: dict[str, list[str]] = collections.defaultdict(list)
    order = list(SUBJECTS)
    for (fighter, rung), row in sorted(
        rows.items(), key=lambda kv: (order.index(kv[0][0]), kv[0][1])
    ):
        v = verdict(row)
        by_fighter[fighter].append(v)
        starts = "/".join(str(s) for s in row.get("starts", [])) or "-"
        dealt = "/".join(f"{d:g}" for d in row.get("dealt", [])) or "-"
        first = "/".join(
            f.removeprefix("Some(").removesuffix(")") for f in row.get("first", ())
        ) or "-"
        seeds = row.get("seeds", [])
        if len(seeds) != 2:
            seed_col = "-"
        elif any(not sd.startswith("0x") for sd in seeds):
            seed_col = "NOBRAIN"
        else:
            seed_col = "differ" if seeds[0] != seeds[1] else "SAME"
        print(f"{fighter:<22}{rung:>5}{starts:>12}{dealt:>12}"
              f"{first:>14}{row.get('window', '-'):>8}{seed_col:>9}  {v}")
        if v in {"UNMEASURABLE", "NO_FIGHTER_BRAIN"}:
            why = row.get("why") or ["no reason captured"]
            print(f"{'':>22}     ⤷ {why[0][:110]}")

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
    if any(v is None or v in {"UNMEASURABLE", "NO_FIGHTER_BRAIN"}
           for v in at_three.values()):
        missing = [f for f, v in at_three.items()
                   if v is None or v in {"UNMEASURABLE", "NO_FIGHTER_BRAIN"}]
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
        print("   them diverged anyway.")
        print("⚠ No seatable pair isolates the jitter with L3 rollouts held")
        print("   constant: of the five published rungs, 1/3/5 are rollouts-off")
        print("   and 6/9 are on. The monotone series is the evidence; a")
        print("   controlled contrast does not exist in this composition.")
    else:
        print("⚠ SPLIT at rung 3 — the fighters disagree, so the cause is not a")
        print("   property of the rung alone. Read the rows, not this line.")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--run", action="store_true", help="run all 15 duels (slow)")
    ap.add_argument(
        "--fill", action="store_true",
        help="re-run only rows that are missing or UNMEASURABLE, appending to the log",
    )
    args = ap.parse_args()
    if args.run:
        sweep()
    elif args.fill:
        # ⚠ Re-running an UNMEASURABLE row is worth it only because the RUNNER
        # changed: it now captures the panic's message and not just its header,
        # so "the match was DECIDED" stops sharing a verdict with "the seating
        # never happened". Filling with an unchanged runner would just reproduce
        # the same empty row at the cost of a duel.
        have = parse() if LOG.exists() else {}
        todo = {
            (f, r)
            for f in SUBJECTS
            for r in RUNGS
            if (f, r) not in have
            or verdict(have[(f, r)]) in {"UNMEASURABLE", "NO_FIGHTER_BRAIN"}
        }
        if not todo:
            print("nothing to fill")
        else:
            print(f"filling {len(todo)} row(s): {sorted(todo)}")
            sweep(only=todo, append=True)
    return fold()


if __name__ == "__main__":
    raise SystemExit(main())
