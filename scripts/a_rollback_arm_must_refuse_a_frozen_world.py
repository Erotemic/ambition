#!/usr/bin/env python3
"""Every sync-test rollback arm is accounted for, by a check or by a decision.

⛔⛤ **A SYNC-TEST SESSION THAT INVALIDATES KEEPS ACCEPTING `sim.step()` AND STOPS
ADVANCING `SimTick`.** The step returns an observation every time. Nothing panics,
nothing prints, and every assertion after the invalidation runs over a frozen
world — where it agrees with itself, forever. Measured 2026-09-16 and recorded in
`docs/planning/queue.md`'s ROLLBACK-DEAD-SESSION: one system writing a
rollback-registered resource outside its sanctioned road desyncs the sync test,
and the desync presents as a stopped clock rather than as a failure.

⚠ **THIS DOES NOT DECIDE THE PROPERTY, AND COULD NOT.** "This arm's assertions
are unsatisfiable by a frozen world" is not readable from source — the census
found SEVEN different mechanisms producing it, from a population floor of 116,280
finite floats to a door that must open within 60 frames to an equality against a
non-rollback control. The row is right that a guard cannot judge that.

⇒ **WHAT IT DOES IS STOP THE CENSUS ROTTING.** The row's own warning was
*"it proves the CURRENT 21 arms are safe; it says nothing about the
twenty-second"* — and by 2026-09-16 the population was **25**, four files past
the census that certified it. This routes the decision instead of making it: a new
sync-test arm either calls the health API or arrives here for a sentence about
what a frozen world breaks in it.

⛔⛤ **AND THE CENSUS THAT CERTIFIED THE POPULATION COULD NOT SEE ALL OF IT.** It
swept `crates/` and `game/`; the 26th member is
`examples/capability_demo/tests/rollback_round_trip.rs`, in a root neither that
sweep nor the row's follow-up looked at. It reads the health API and is safe —
but "21 arms, all safe" was measured over a population that never contained it.
That is the same defect the row records one section up about `tools/`: a scan
root is a citation, and a member outside it is invisible rather than absent. This
checker enumerates `git ls-files` instead.

⚠ **AND A HEALTH CALL IS NOT THE STRONGER ANSWER.** The row measured that too:
*"an arm that demands a room change has a better liveness check than one that
reads `rollback_health()` once at the end, because its check is load-bearing for
what the arm is actually about."* Counting calls to a safety API measures
vigilance; the `ADJUDICATED` entries below measure safety.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

#: The call that builds one. Every arm in the population reaches it.
SYNC_TEST = re.compile(r"\bwith_sync_test_rollback_settings\b")

#: An explicit liveness read. Sufficient, not necessary — see the module doc.
HEALTH = re.compile(r"\brollback_health\s*\(\s*\)|\bsession_health\b")

#: Files that build the fixture without being an arm.
NOT_AN_ARM = {
    "crates/ambition_sim_harness/src/options.rs": "the API definition itself",
    "game/ambition_app/examples/hall_bench.rs": "a benchmark example: it asserts "
    "nothing, so there is no verdict for a frozen world to falsify",
}

#: Arms with no health call whose assertions a frozen world cannot satisfy, each
#: with the MECHANISM that makes that true. ⛔ A row here is a reading of the
#: arm, not a waiver: name what breaks, so the next reader can check it still does.
ADJUDICATED: dict[str, str] = {
    "game/ambition_app/tests/canonical_state_is_finite.rs": "population floor — "
    "`finite_seen >= ENCODED_FLOAT_FLOOR` against a measured 116,280",
    "game/ambition_app/tests/input_stream_under_rollback.rs": "the recorded stream "
    "length is compared against the tick count",
    "game/ambition_app/tests/rollback_provoked_actor.rs": "`load_runs` must move, "
    "and `assert_rolled_back`",
    "game/ambition_app/tests/d71_transaction_census.rs": "explicit preconditions "
    "`room_changes > 0` and `transactions > 0`",
    "game/ambition_app/tests/carried_item_crosses_rooms.rs": "`walk_through_the_door_to` "
    "panics after 60 frames with no room change",
    "game/ambition_app/tests/door_entry.rs": "asserts the room changed after the "
    "authored hold",
    "game/ambition_app/tests/a_move_keeps_its_occurrence_across_a_rewind.rs": "the "
    "rollback reading is compared for EQUALITY against a fixed-tick control whose "
    "own floor is `reached > 0`, so a frozen rollback world disagrees with it "
    "(read 2026-09-16)",
    "game/ambition_app/tests/does_a_presence_probed_row_move_when_its_value_does.rs":
    "`the_reading_is_about_the_subject` floors `resimulations` and the distinct "
    "census count at the compared frames; a frozen world resimulates nothing "
    "(read 2026-09-16)",
    "game/ambition_app/tests/how_much_of_the_peer_checksum_actually_varies.rs": "each "
    "arm opens with `audit.live_comparisons > 0`, which a stopped clock fails "
    "(read 2026-09-16)",
}


def _tracked_rust() -> list[str]:
    return subprocess.run(
        ["git", "-C", str(REPO), "ls-files", "*.rs"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()


def code_only(text: str) -> str:
    """`text` with comments and string literals blanked out, newlines preserved.

    ⛔⛤ **THE PATTERNS BELOW RAN AGAINST RAW SOURCE, WHICH IS A FALSE-GREEN HOLE
    IN THE UNSAFE DIRECTION — NAMED BY THE GPT ARCHITECTURE REVIEW OF
    2026-09-16.** `HEALTH` is what CERTIFIES an arm as non-vacuous, so a file
    containing only

    ```text
    // session_health should be checked here someday
    ```

    satisfied the guard while checking nothing.

    ⛔⛤ **AND THE FIRST REPAIR WAS A REGULAR EXPRESSION, WHICH THE SAME REVIEW
    POISONED THROUGH THE NEXT DAY.** It handled `//`, `/* */` and `"..."`, and
    Rust has more literal forms than that:

    ```rust
    let explanation = r#"foo" rollback_health() "bar"#;
    ```

    is a single raw string whose inner `"` ends the pattern's match, leaving
    `rollback_health()` standing as apparent code. ⇒ **Do not extend the regex one
    literal form at a time.** This is a scanner, and it blanks `//` comments,
    NESTED `/* */` comments (Rust allows them), and every string form: plain,
    byte, and raw with an arbitrary `#` count, byte-raw included.

    ⭐ **BOTH PATTERNS ARE STRIPPED, AND THE TWO DIRECTIONS ARE NOT SYMMETRIC.** A
    comment naming `HEALTH` certifies an arm that checks nothing, which is
    silent. A comment naming `SYNC_TEST` only pulls a non-arm INTO the population,
    where it has to be adjudicated by hand — loud, and safe. Both are stripped
    anyway, because a population found by prose is not the population, and `main`
    floors the count so a scanner that ate the file cannot read as "no arms".

    ⚠ Character literals are deliberately NOT handled: no health call fits in
    one, and `'` is also a lifetime, so recognising them costs more than it buys.
    """
    out = []
    i = 0
    n = len(text)

    def blank(chunk: str) -> str:
        return "".join("\n" if ch == "\n" else " " for ch in chunk)

    while i < n:
        ch = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j == -1 else j
            out.append(blank(text[i:j]))
            i = j
            continue
        if text.startswith("/*", i):
            depth = 0
            j = i
            while j < n:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                    if depth == 0:
                        break
                else:
                    j += 1
            out.append(blank(text[i:j]))
            i = j
            continue
        # A raw string: an optional `b`, then `r`, then any number of `#`, then `"`.
        m = _RAW_OPEN.match(text, i)
        if m:
            hashes = m.group("hashes")
            close = '"' + hashes
            j = text.find(close, m.end())
            j = n if j == -1 else j + len(close)
            out.append(blank(text[i:j]))
            i = j
            continue
        # A plain or byte string, where a backslash escapes the next character.
        if ch == '"' or (ch == "b" and text.startswith('b"', i)):
            j = i + (2 if ch == "b" else 1)
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            out.append(blank(text[i:j]))
            i = j
            continue
        out.append(ch)
        i += 1
    return "".join(out)


#: The opening of a raw string: `r"`, `r#"`, `br##"` and so on. The `#` run has
#: to be captured because the CLOSER must match its length — that is the whole
#: reason a regex over the literal cannot do this job.
_RAW_OPEN = re.compile(r'b?r(?P<hashes>#*)"')


def sync_test_arms(paths: list[str] | None = None) -> dict[str, bool]:
    """`{repo-relative path: reads a health API}` for every sync-test fixture."""
    found: dict[str, bool] = {}
    for rel in paths if paths is not None else _tracked_rust():
        if "/target/" in rel or rel.startswith(".worktrees"):
            continue
        try:
            text = (REPO / rel).read_text(errors="replace")
        except OSError:
            continue
        text = code_only(text)
        if SYNC_TEST.search(text):
            found[rel] = bool(HEALTH.search(text))
    return found


def main() -> int:
    arms = sync_test_arms()
    if not arms:
        # ⛔ The population is found by a SPELLING, and a rename empties it in
        # silence. An empty census would otherwise print the same clean verdict.
        print("⛔ NO sync-test rollback fixture found at all — the scan is wrong.")
        return 1
    checked = {rel for rel, health in arms.items() if health}
    loose = sorted(set(arms) - checked - set(ADJUDICATED) - set(NOT_AN_ARM))
    stale_adjudications = sorted(set(ADJUDICATED) - set(arms))
    stale_exemptions = sorted(set(NOT_AN_ARM) - set(arms))
    print(
        f"{len(arms)} sync-test rollback fixture(s): {len(checked)} read a health "
        f"API, {len(ADJUDICATED)} adjudicated, {len(NOT_AN_ARM)} not arms"
    )
    for rel in stale_adjudications:
        print(f"✔ GONE, remove from ADJUDICATED: {rel}")
    for rel in stale_exemptions:
        print(f"✔ GONE, remove from NOT_AN_ARM: {rel}")
    for rel in loose:
        print(
            f"\n⛔ {rel}\n"
            "   builds a sync-test rollback session and never reads its health."
        )
    if loose:
        print(
            "\n⇒ AN INVALIDATED SYNC-TEST SESSION KEEPS ACCEPTING `step()` AND STOPS\n"
            "  ADVANCING `SimTick`, silently. Either read `rollback_health()`, or —\n"
            "  better — make sure some assertion here is unsatisfiable by a frozen\n"
            "  world and add the file to ADJUDICATED naming which one."
        )
    if loose or stale_adjudications or stale_exemptions:
        return 1
    print("ok: every sync-test rollback arm is checked or accounted for.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
