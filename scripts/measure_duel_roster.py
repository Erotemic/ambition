#!/usr/bin/env python3
"""Every fighter the Smash grid offers, duelled against itself at rung 9.

⭐ **THE QUESTION THIS ANSWERS.** `two_cpus_in_the_shipped_composition_damage_each_other`
guards one fighter named by a const, and a guard's population is the thing to ask
about before its threshold. Measured 2026-09-10 over the 21 ids the assembled
grid offers: **20 measured, 15 clear the 0.5 gate, 1 sits AT it, 4 fail, and the
4 do not share a mechanism** — two barely press, two press constantly and convert
nothing. ⇒ The row is two rows.

⛔⛔ **THE POPULATION IS THE ASSEMBLED GRID, NOT A LIST OF MOVESETS.** The first
run of this sweep used `authored_movesets::tables()` — whose own header warns it
is *"NOT THE SELECTABLE CAST"* — and was about to report three different causes
in one column: a bare registration, an id the grid does not carry, and a fighter
that genuinely does not fight. The grid comes from the harness's own refusal
message, so it is what the composed app can seat rather than a list anyone keeps.

⛔ **AND A GATE FAILURE IS NOT A HARNESS FAILURE.** Both surface as a panic. One
is `A_REAL_FIGHT` firing on a fighter that was measured and fell short; the other
is a dependency's despawn hook demanding a resource the headless composition
lacks (`npc_alice`, `bevy_render`'s `sync_component.rs`), and that fighter was
never measured at all. Folding them together puts a bad fighter beside an
unrunnable one.

⚠ **THE RATE'S DENOMINATOR IS NOT REPRODUCIBLE.** `both_seated_ticks` is the
window, and seating waits on wall-clock asset IO: three runs of one fighter gave
windows 3596 / 3613 / 3601 with first-seat ticks 22 / 5 / 17 — **the sums are all
3618**, and every other field byte-identical. The fight reproduces; the window
does not, so damage/min wobbles in its third digit. This reports the window so a
reader can see it rather than averaging it away.

⭐⭐ **RE-SWEPT 2026-09-10 after the dismount fix, and the mount column answered
the question the re-sweep existed for: EXACTLY ONE of the twenty measured bouts
had a mount die.** `npc_pirate_admiral`, one death — enough to make every column
of his row a fighter-vs-brute measurement, with no dose-response.

Cross-checked rather than taken on the column's word, by diffing every fighter
against the pre-fix log:

| | before | after | mounts died |
|---|---|---|---|
| 18 of 20 rows | — | **bit-identical** | 0 |
| `npc_pirate_admiral` | 1.26 / 1.07, hitstun [525, 324], 2 KO | **0.84 / 0.76, [161, 142], 4 KO** | **1** |
| `projectile_polygon` | 1.63 / 1.54, [318, 270] | 1.63 / 1.54, [318, **263**] | 0 |

⇒ **The column and the diff agree on which row the road touched**, which is what
makes either believable. **Nineteen rows never needed discarding.**

⚠ `projectile_polygon` is not a dismount effect: its damage is identical and
only seat 1's hitstun moved, by 7 ticks, and a re-run reproduced the OLD value.
Small run-to-run variation in one field of one fighter.

⛔⛔ **AND THE FIRST DIFF OF THAT ROW USED THE WRONG COLUMN.** Damage-per-minute
divides by `both_seated_ticks` — the window this file's own header documents as
non-reproducible — while the POOL FRACTION printed beside it on the same line is
window-independent. ⇒ **A derived figure inherits the noise of its DENOMINATOR**,
and reading the noisy one nearly reported window wobble as a behavioural change.
Diff `took X / Y of pool`, never `= X / Y per minute`.

⚠ **The admiral's post-fix row is PROVISIONAL.** It was measured against the
first version of the dismount fix, which was later found to disable the rebuild
for every production rider. The other nineteen had zero mount deaths and so
never reached that road under any version.

⛔⛔ **EVERY NUMBER IN THIS TABLE CARRIES THREE STAMPS, AND TWO OF THEM WERE
DISCOVERED THE HARD WAY.** A figure that leaves this file without all three is a
figure that will mislead somebody in a month.

  1. **A COMMIT.** The 21-id sweep folded here was measured at `8dbb5e91d`.
  2. **A PROFILE.** Every bout runs the `NoWindow` / `backends: None` composition.
     That is not "the shipped game" — it is the shipped game minus a
     render-to-texture path that cannot run without an adapter. ⚠ It is a
     CONFIGURATION, not a property of the machine: a software Vulkan adapter is
     enough for `VisibleRenderMode::OffscreenGpu`, so "headless" here means the
     profile and never the hardware.
  3. ⛔⛔ **A HOST. MEASURED 2026-09-10: TWO MACHINES RUNNING THE SAME COMMIT AND
     THE SAME COMMAND PRODUCE DIFFERENT FIGHTS.**

**The cross-host evidence, `npc_alice` at rung 9, same sha, same command:**

| | host A (this one) | host B |
|---|---|---|
| duel ran | **3173, decided at 3178** | **3613, undecided** |
| dmg/min | 1.38 / 1.55 | 1.58 / 1.59 |
| hitstun | [236, 292] | [419, 340] |
| seat 1 starts | 48 | 78 |
| knockouts | 4 | 3 |
| **birth seeds** | **0xd3b5b696… / 0xd2b5b509…** | **identical** |
| **`[sym]` split** | **tick 522 (+79.49, −127.21)** | **identical** |

⇒ **Identical to 0.01px for 522 ticks, then divergent — and the divergence
enters exactly when the mirror breaks and not before.** Three consecutive runs
on each host are individually bit-identical, so this is not run-to-run wobble.

**Three mechanisms proposed and all three refuted:** asset-decode timing (the
sheet landed at frame 5 and frame 8 on one host with bit-identical fights),
executor/system order (`GgrsSchedule` sets `SingleThreadedExecutor` explicitly,
`rollback_ggrs/src/lib.rs:142`), and seeding (the seeds match exactly).
⚠ **UNATTRIBUTED, and the surviving candidate — entity storage order following
host-dependent spawn order — is a CANDIDATE and not a conclusion.** Three dead
mechanisms do not make a fourth likely. It is filed on S7 as a determinism
question, because a rollback sim whose outcome depends on the host is a bigger
row than any fighter's.

⚠ **AND A DECIDED BOUT AND AN UNDECIDED ONE ARE NOT THE SAME KIND OF ROW.**
Host A's Alice was decided by the fight, so her window is stable; host B's ran
to the budget and carries the seating jitter. **An earlier version of this file
said flatly that the window is non-reproducible. That is true only of undecided
bouts.**

⚠ **`npc_alice` is no longer UNMEASURABLE** — she was the only fighter reaching a
`NoWindow` camera despawn, fixed at `4d3e0507a`. **Her row is not folded into the
table above because that table is stamped to `8dbb5e91d` and hers is not**, and
mixing stamps is how a table stops meaning anything.

    python3 scripts/measure_duel_roster.py --run     # run the sweep (slow)
    python3 scripts/measure_duel_roster.py           # fold an existing log
"""

from __future__ import annotations

import argparse
import collections
import pathlib
import re
import subprocess

REPO = pathlib.Path(__file__).resolve().parents[1]
LOG = REPO / "target/duel_roster_sweep.log"
CATALOG = REPO / "game/ambition_content/src/character_catalog.rs"
GATE = 0.5


def rosters() -> tuple[set[str], set[str]]:
    """`(PLAYABLE_ROSTER, KNOWN_BARE_REGISTRATIONS)` — two published vocabularies.

    ⚠ A grid id is not a playable one and neither is a bare registration, and the
    three answer different questions.

    ⛔ **AND THE BARE LIST IS EMPTY AS OF 2026-09-10.** It carried
    `npc_carl_stargan`, and this file's own header used to cite his 0.00 as "a
    character with no authored body being seatable as a fighter". He authors a
    locomotion, a moveset and vitals; the EXEMPTION was stale, not the roster.
    A row classified `bare-registration` today would be a fact about a list
    nobody had re-derived.
    """
    src = CATALOG.read_text(encoding="utf-8")
    block = src[src.index("pub const PLAYABLE_ROSTER") :]
    playable = set(re.findall(r'^\s*"([a-z_0-9]+)",', block[: block.index("];")], re.M))
    # ⛔⛔ BOUND THE SCAN BY THE LIST, NOT BY A WINDOW WIDTH. This read a fixed
    # 400 characters past the name, so when the list was emptied to `&[]` on
    # 2026-09-10 the regex kept reading and returned `{"unused"}` -- a string
    # literal from unrelated code below it. A parser with no terminator does not
    # return "nothing"; it returns whatever comes next, and the caller cannot
    # tell the two apart.
    bare_at = src.index("KNOWN_BARE_REGISTRATIONS")
    open_at = src.index("&[", bare_at)
    close_at = src.index("];", open_at)
    # ⚠ THE ID, NOT THE REASON. Each entry is `("<id>", "<why>")`, and a regex
    # over every quoted word in the block returns both -- caught by a poison
    # that planted `("npc_poison_probe", "why")` and got a set of two. Real
    # reasons are prose and would not have matched, so this would have stayed
    # invisible until an exemption arrived with a one-word reason.
    bare = set(re.findall(r'\(\s*"([a-z_0-9]+)"', src[open_at:close_at]))
    return playable, bare


def grid_ids() -> list[str]:
    """The ids the composed app will seat, from the harness's own refusal.

    ⭐ ASKED OF THE APP RATHER THAN OF A SOURCE FILE. Passing a deliberately
    invalid id makes the test print the assembled grid, which is the population
    that can actually be duelled — it cannot drift from what the composition
    offers, because it IS what the composition offers.
    """
    proc = subprocess.run(
        [
            "cargo", "test", "-q", "-p", "ambition_app", "--test", "app_it",
            "--features", "rl_sim", "--",
            "smash_cpus_damage_each_other::two_cpus", "--nocapture",
        ],
        capture_output=True, text=True, cwd=REPO,
        env={**__import__("os").environ, "AMBITION_DUEL_FIGHTER": "__ask_the_grid__"},
    )
    for line in (proc.stdout + proc.stderr).split("\n"):
        if "Grid:" in line:
            return re.findall(r'"([a-z_0-9]+)"', line.split("Grid:")[1])
    raise SystemExit(
        "the harness did not print its grid. Either the assembled-grid assertion "
        "moved or an invalid id no longer reaches it — do NOT fall back to a list "
        "in this file; the point is that the population comes from the app."
    )


def sweep(ids: list[str], runs: int) -> None:
    import os

    with LOG.open("w", encoding="utf-8") as out:
        for fighter in ids:
            for run in range(1, runs + 1):
                out.write(f"=== {fighter} run{run}\n")
                out.flush()
                proc = subprocess.run(
                    [
                        "cargo", "test", "-q", "-p", "ambition_app", "--test", "app_it",
                        "--features", "rl_sim", "--",
                        "smash_cpus_damage_each_other::two_cpus", "--nocapture",
                    ],
                    capture_output=True, text=True, cwd=REPO,
                    env={**os.environ, "AMBITION_DUEL_FIGHTER": fighter},
                )
                # ⛔ THE PANIC PAYLOAD IS ON THE NEXT LINE, AND THE FOLD NEEDS
                # IT. `panicked at <file>:<line>:` gives the location only. The
                # sentence that says WHY comes after it. A filter that keeps
                # only the header throws away the one fact that separates a
                # harness that broke from a cast the composition would not
                # seat — both panic, and both panic in this same file.
                payload = 0
                for line in (proc.stdout + proc.stderr).split("\n"):
                    keep = re.match(
                        r"^\[(duel|gap|body|stance|moves|dealt|mount|brain)\]", line
                    ) or ("panicked at" in line or "is not on the assembled" in line)
                    if keep:
                        payload = 2 if "panicked at" in line else 0
                    elif payload > 0 and line.strip():
                        keep = True
                        payload -= 1
                    if keep:
                        out.write(line + "\n")
                out.flush()
        out.write("SWEEP DONE\n")


def fold() -> int:
    playable, bare = rosters()
    runs: dict[str, list[dict]] = collections.defaultdict(list)
    cur = None
    for line in LOG.read_text(encoding="utf-8").split("\n"):
        if line.startswith("==="):
            cur = line.split()[1]
            runs[cur].append({})
            continue
        if cur is None or not runs[cur]:
            continue
        row = runs[cur][-1]
        if m := re.search(r"= ([\d.]+) / ([\d.]+) per minute", line):
            row["dmg"] = (float(m.group(1)), float(m.group(2)))
        if m := re.search(r"ran (\d+) ticks", line):
            row["window"] = int(m.group(1))
        if m := re.search(r"ticks within 60px: (\d+) of (\d+)", line):
            row["reach"] = 100.0 * int(m.group(1)) / int(m.group(2))
        if m := re.search(r"half-extent x: Some\(([\d.]+)\)", line):
            row["width"] = float(m.group(1))
        # ⭐ WHICH ROWS THE DISMOUNT FIX TOUCHED. A bout in which no mount died
        # never reached the road that rebuilt a rider's brain as a brute, so its
        # pre-fix numbers are still good; a bout in which one did was measuring
        # a fighter against a brute. Without this the whole table can only be
        # discarded, never sorted.
        if m := re.match(r"^\[mount\] mounts that died this bout: (\d+)", line):
            row["mounts_died"] = int(m.group(1))
        if m := re.match(r"^\[moves\] seat \d: (\d+) starts", line):
            row.setdefault("starts", []).append(int(m.group(1)))
        if m := re.match(r"^\[dealt\] seat \d: (\d+) damage", line):
            row.setdefault("dealt", []).append(int(m.group(1)))
        if m := re.search(r"hitstun \[(\d+), (\d+)\]", line):
            row["hitstun"] = (int(m.group(1)), int(m.group(2)))
        if "is not on the assembled" in line:
            row["verdict"] = "NOT ON THE GRID"
        elif "panicked at" in line and "verdict" not in row:
            where = line.split("panicked at")[-1].strip()
            row["where"] = where
            row["verdict"] = (
                "gate" if "smash_cpus_damage_each_other.rs" in where
                else "HARNESS BROKE: " + where.split("/")[-1].split(":")[0]
            )
        elif row.get("verdict") and "why" not in row and not line.startswith("["):
            row["why"] = line.strip()

    # ⛔⛔ TWO ROWS WITH NO NUMBERS ARE NOT ONE FACT. A fighter can miss the
    # table for two reasons, and they call for opposite actions.
    #
    # 1. THE HARNESS BROKE. Something panicked while the bout ran. MEASURED
    #    2026-09-10: `npc_alice` panicked in `bevy_render/sync_component.rs`.
    #    The fighter is not the subject of that failure. Repair the harness and
    #    the row comes back.
    #
    # 2. THE COMPOSITION WITHHELD THE CAST. `prepare_match` refuses a roster
    #    that it cannot resolve. One participant that it cannot resolve gives
    #    ZERO seats, not one seat: `ambition_match/src/prepared.rs:636` records
    #    the problem, and line 872 fails the whole preparation. The bout then
    #    runs with no fighters on the stage, and the seating floor fires. This
    #    is a true fact ABOUT THAT FIGHTER. It says the shipped composition
    #    will not seat it, which is what this sweep asks.
    #
    # ⚠ BOTH PANIC IN THE HARNESS FILE, SO THE LOCATION CANNOT SEPARATE THEM.
    # The seated-tick count separates them. Zero seated ticks in a full bout
    # means that no cast ever stood up.
    #
    # ⛔ AN EARLIER VERSION OF THIS FUNCTION CRASHED ON CASE 2. It read
    # `first["dmg"]` for every row that was not `UNMEASURABLE` or `NOT ON`. The
    # seating floor fires BEFORE the `[duel]` line prints, so a withheld cast
    # has no `dmg` key and the fold raised `KeyError`. A sweep that contains
    # one withheld fighter must still print the other twenty rows.
    for entries in runs.values():
        for row in entries:
            if row.get("verdict") != "gate" or "dmg" in row:
                continue
            why = row.get("why", "")
            seated = re.search(r"shared the stage for only (\d+) of", why)
            if seated and int(seated.group(1)) == 0:
                row["verdict"] = "CAST WITHHELD: no seat ever stood up"
            elif seated:
                row["verdict"] = f"NO NUMBERS: bout ended after {seated.group(1)} ticks"
            else:
                at = row.get("where", "").split(":")
                row["verdict"] = "NO NUMBERS: harness assertion at line " + (
                    at[1] if len(at) > 1 else "?"
                )

    print(f"{'fighter':28} {'dmg/min':13} {'reach':>6} {'starts':>9} {'width':>6}  class")
    counts = collections.Counter()
    for fighter, entries in runs.items():
        done = [r for r in entries if r.get("dmg") or r.get("verdict")]
        if not done:
            continue
        first = done[0]
        verdict = first.get("verdict", "")
        if verdict.startswith(("HARNESS BROKE", "NO NUMBERS")):
            counts["broken"] += 1
            print(f"{fighter:28} {verdict}")
            continue
        if verdict.startswith(("CAST WITHHELD", "NOT ON")):
            counts["withheld"] += 1
            print(f"{fighter:28} {verdict}")
            continue
        low = min(first["dmg"])
        band = "AT THRESHOLD" if abs(low - GATE) < 0.02 else ("BELOW GATE" if low < GATE else "")
        counts["measured"] += 1
        counts[band or "clears"] += 1
        tag = "playable" if fighter in playable else ("bare-registration" if fighter in bare else "grid-only")
        windows = sorted({r.get("window") for r in done})
        wobble = f"  ⚠ window {'/'.join(str(w) for w in windows)}" if len(windows) > 1 else ""
        starts = "/".join(str(s) for s in first.get("starts", []))
        # ⛔⛔ **A BOUT WHOSE TWO SEATS NEVER DIVERGE IS ONE SEAT'S INFORMATION
        # REPORTED TWICE.** MEASURED 2026-09-10: 3 of the 5 fighters below the
        # gate are exactly symmetric on damage, starts, damage dealt AND hitstun;
        # **0 of the 15 that clear it are.** ⇒ Those three are not fighters that
        # convert badly; they are duels this harness cannot measure.
        #
        # ⛔⛔ **AND THE OBVIOUS EXPLANATION IS FALSE — DO NOT REACH FOR IT.** This
        # comment said "a mirror match of a deterministic brain with no noise
        # input can stay in lockstep". The brains DO have noise:
        # `fighter_cognition_seed` hashes `"<character>#seat<n>"` so the seats get
        # different streams, `next_signed_unit(&mut state.noise)` consumes it in
        # `ambition_combat/src/brain/fighter/decision.rs`, and `execution_noise`
        # is 0.10 at rung 9.
        #
        # ⭐ THE INVERSION IS THE OPEN QUESTION. `npc_emmy_noether` is the one
        # character authoring `preserves_mirror_symmetry` — twins deliberately
        # SHARING a stream — and hers is the one duel that genuinely diverged. So
        # three fighters with distinct seeds and live jitter produce bit-identical
        # seats while the fighter with a shared seed does not. Nobody has a
        # mechanism for that; four have died on this row already.
        #
        # ⚠ LOCKSTEP IS NOT *BY CONSTRUCTION* LOW DAMAGE. `medic`'s mirrored seats
        # dealt 41 each and took 118 hitstun each — they landed, symmetrically.
        # What is proven is that the seats never DIVERGED, not that the fight did
        # not happen. Whether the lockstep causes the low damage is a claim this
        # instrument cannot separate, which is precisely why it cannot measure it.
        #
        # ⭐ `ladder_rig` REFUSES a degenerate bout outright rather than reporting
        # it; this one flags, because here the flag is also the evidence for the
        # finding. ⚠ Its refusal text names a noise seed, and that describes ITS
        # OWN fixture building brains without one — it is NOT a claim that the
        # shipped brain lacks a seed. It has one, per seat, and consumes it.
        symmetric = (
            len(set(first.get("starts", [0, 1]))) == 1
            and len(set(first.get("dealt", [0, 1]))) == 1
            and first["dmg"][0] == first["dmg"][1]
            and len(set(first.get("hitstun", (0, 1)))) == 1
        )
        if symmetric:
            counts["lockstep"] += 1
            band = (band + " " if band else "") + "LOCKSTEP"
        print(
            f"{fighter:28} {first['dmg'][0]:.2f} / {first['dmg'][1]:.2f}  "
            f"{first.get('reach', 0):5.1f}% {starts:>9} {first.get('width', 0):6.2f}  "
            f"{tag} {band}{wobble}"
        )

    # ⛔ ANTI-VACUITY. An empty or truncated log prints a serene header and no
    # rows, which reads exactly like a sweep nobody has run yet.
    assert counts["measured"] >= 15, (
        f"only {counts['measured']} fighters were measured; the grid offered 21 on "
        "2026-09-10. Re-run the sweep rather than reading this table."
    )
    print(
        f"\n   {counts['measured']} measured, {counts['clears']} clear the {GATE} gate, "
        f"{counts['AT THRESHOLD']} at threshold, {counts['BELOW GATE']} below, "
        f"{counts['lockstep']} LOCKSTEP."
    )
    print(
        f"   {counts['broken']} HARNESS BROKE (no fact about the fighter; repair and "
        f"re-run), {counts['withheld']} CAST WITHHELD (a fact about the fighter: this "
        "composition will not seat it)."
    )
    print("⛔ A LOCKSTEP ROW IS NOT A RESULT ABOUT THAT FIGHTER. Its two seats never")
    print("   diverged, so the bout carries one seat's information reported twice.")
    print("\n⚠ ONE FIGHT PER FIGHTER. The harness fixes its seed, so repeats reproduce")
    print("  rather than sample — 21 fighters is a wider POPULATION, not more samples.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", action="store_true", help="run the sweep (slow)")
    parser.add_argument("--runs", type=int, default=1)
    args = parser.parse_args()
    if args.run:
        sweep(grid_ids(), args.runs)
    if not LOG.is_file():
        raise SystemExit(f"no sweep log at {LOG}; run with --run first")
    return fold()


if __name__ == "__main__":
    raise SystemExit(main())
