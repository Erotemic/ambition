#!/usr/bin/env python3
"""Which `UserSettings` fields does DETERMINISTIC SIMULATION read?

The 2026-09-13 architecture review's priority 1: `UserSettings` is waived in
`rollback_coverage.rs` as *"user settings, forward-only"* — the category `Q119`
already ruled is not one — while the settings menu mutates it at runtime and
simulation systems read it. This inventories the readers so the split can be
decided from evidence rather than from the type name.

⛔⛤ **THE KEY IS A TYPE, NOT A FIELD NAME, AND THAT IS THE WHOLE DIFFERENCE FROM
`scripts/measure_identity_field_consumers.py`.** That script tried to classify
`Q122`'s fields by grepping `.field` and answered *"50 of 50 mechanical"*, because
`.id` and `.body` are spelled the same way in a hundred structs. `UserSettings`
is one named type, so `Res<…UserSettings>` finds its readers and nothing else —
and each hit is reported with its file so the attribution can be checked rather
than trusted.

⚠ **SCHEDULE ATTRIBUTION IS THE HALF THAT CAN BE WRONG, AND IT SAYS SO.** A
reader matters only if it runs in the SIMULATION schedule — under the rollback
host that is `GgrsSchedule`, where a resimulation of confirmed frames reads it.
This script decides that by looking for the reader's name in an `add_systems`
call whose schedule argument is `sim` / `sim_schedule()`. ⇒ A system registered
through an intermediate or a set this scan cannot follow is reported as
`UNATTRIBUTED`, never as safe.

    python3 scripts/measure_user_settings_in_simulation.py
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The reader signature, in every qualification it is written in.
READER = re.compile(r"Res<\s*(?:[A-Za-z0-9_]+::)*UserSettings\s*>")
# `fn name(` at any indentation, for attributing a hit to its enclosing function.
FN = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([a-z_][a-z0-9_]*)\s*[(<]")


def enclosing_fn(lines: list[str], index: int) -> str | None:
    """The `fn` a line belongs to.

    ⛔ SCANS UPWARD AND ACCEPTS `pub(crate) fn`. A previous census in this
    repository credited two systems to helper functions defined earlier in the
    file because its regex was `(pub )?fn`, and the wrong answers were plausible
    NAMES so nothing looked broken.
    """
    for i in range(index, -1, -1):
        match = FN.match(lines[i])
        if match:
            return match.group(1)
    return None


def code_of(line: str) -> str:
    """The line with any `//` comment tail removed.

    ⛔⛤ **PROSE ABOUT A TYPE IS NOT A READ OF IT, AND THIS SCRIPT SCORED ITS OWN
    FIX AS A FAILURE.** After `derive_slot_direction_gestures` and
    `possession_trigger_system` were migrated off `Res<UserSettings>`, the comment
    explaining the migration — *"not `Res<UserSettings>`"* — still matched, so the
    census reported both as unchanged simulation readers. The sibling scanner
    `tests/ambition_workspace_policy/src/custom/control_frame.rs` already carries
    this exact near-miss in its own test corpus.

    ⚠ This is deliberately NAIVE about `//` inside a string literal. A Rust
    parameter list has no string literals, and a naive cut that costs a false
    NEGATIVE on a line nobody writes beats a false POSITIVE on the comment every
    migration leaves behind.
    """
    head, _, _ = line.partition("//")
    return head


def without_comments(text: str) -> str:
    """The file with every comment blanked to spaces, offsets preserved.

    ⛔⛤ **RUST COMMENTS CONTAIN UNBALANCED PARENTHESES, AND THAT IS WHAT MADE 45
    READERS `UNATTRIBUTED`.** The schedule attribution below walks the balanced
    parentheses of an `add_systems(sim, …)` call. A prose `)` inside a comment
    closes the call early: MEASURED, the `add_systems(sim, …)` at
    `crates/ambition_platformer2d_runtime/src/combat_schedule.rs:640` is a
    ~14,000-character block and the scanner read 435 characters of it, which is
    why `apply_feature_hit_events` — a system the architecture review names by
    hand as a simulation reader — was reported as not scheduled anywhere.

    ⚠ Blanking rather than deleting, so every offset this function's caller
    computes still points at the same byte of the original. And `//` preceded by
    `:` is left alone, because that is a URL inside a string literal and cutting
    it would drop whatever parentheses followed on that line.
    """
    out = list(text)
    i = 0
    n = len(text)
    while i < n:
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            end = n if end < 0 else end + 2
            for j in range(i, end):
                if out[j] != "\n":
                    out[j] = " "
            i = end
            continue
        if text.startswith("//", i) and not (i and text[i - 1] == ":"):
            end = text.find("\n", i)
            end = n if end < 0 else end
            for j in range(i, end):
                out[j] = " "
            i = end
            continue
        i += 1
    return "".join(out)


def readers() -> dict[str, set[str]]:
    """Function name -> the files it is declared in."""
    out: dict[str, set[str]] = {}
    listing = subprocess.run(
        ["git", "grep", "-lE", r"Res<\s*([A-Za-z0-9_]+::)*UserSettings\s*>"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    ).stdout.split()
    for path in listing:
        if not path.endswith(".rs"):
            continue
        if "/tests/" in path or path.endswith("tests.rs") or path.endswith("_tests.rs"):
            continue
        lines = (ROOT / path).read_text().splitlines()
        for i, line in enumerate(lines):
            if not READER.search(code_of(line)):
                continue
            name = enclosing_fn(lines, i)
            if name:
                out.setdefault(name, set()).add(path)
    return out


SIM_SCHEDULE_ARG = re.compile(
    r"add_systems\(\s*(?:sim|sim_schedule|app\.sim_schedule\(\)|"
    r"[A-Za-z_:]*GgrsSchedule)\s*,\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)"
)


def identifiers_in_call(text: str, start: int) -> set[str]:
    """Every snake_case identifier inside the call whose arguments begin at `start`.

    ⚠ **THE WINDOW IS THE WHOLE CALL, NOT A FIXED BYTE COUNT.** A truncating
    window silently drops the tail of a long chain, and a long chain is exactly
    where the systems this census is about live: measured, a 2000-byte window
    missed the tail of `combat_schedule.rs`'s registration blocks. The walk runs
    on comment-stripped text so a prose `)` cannot close the call early.
    """
    depth = 0
    chunk = []
    for ch in text[start:]:
        if ch == "(":
            depth += 1
        elif ch == ")":
            if depth == 0:
                break
            depth -= 1
        chunk.append(ch)
    return set(re.findall(r"\b([a-z_][a-z0-9_]{4,})\b", "".join(chunk)))


def forwards_a_parameter(text: str, owner: str, argument: str) -> bool:
    """Is `argument` a PARAMETER of `owner` rather than a local built in place?

    A bare local would be a tuple assembled in the same function, which the
    ordinary `add_systems` scan already reads. A parameter means the names come
    from somewhere else entirely.
    """
    declaration = text.find(f"fn {owner}(")
    if declaration < 0:
        declaration = text.find(f"fn {owner}<")
    if declaration < 0:
        return False
    body = text.find("{", declaration)
    if body < 0:
        return False
    return bool(re.search(rf"\b{re.escape(argument)}\s*:", text[declaration:body]))


def find_sim_forwarders(sources: dict[str, str]) -> set[str]:
    """Functions that pass a PARAMETER of their own on to a simulation schedule.

    `install_techniques(app, offers, systems)` ends in
    `app.add_systems(sim, systems)`, so every system handed to it is scheduled
    even though its name never appears beside `add_systems`.

    ⛔⛤ **AND IT IS A FIXPOINT, BECAUSE THE REAL CHAIN IS TWO DEEP.** The call
    that ships `apply_feature_hit_events` is `install_technique` — SINGULAR —
    whose whole body is `install_techniques(app, &[(key, offer)], systems)`. A
    one-level rule finds the plural, misses the singular, and reports the review's
    one hand-named system as unscheduled; measured, that is exactly what it did.
    So a function that forwards a parameter to a KNOWN forwarder becomes one.
    """
    direct: list[tuple[str, str, str]] = []
    for text in sources.values():
        lines = text.splitlines()
        for index, line in enumerate(lines):
            match = SIM_SCHEDULE_ARG.search(line)
            if not match:
                continue
            owner = enclosing_fn(lines, index)
            if owner and forwards_a_parameter(text, owner, match.group(1)):
                direct.append((owner, match.group(1), ""))
    found = {owner for owner, _, _ in direct}

    # The closure. Each pass looks for `owner(… known_forwarder-bound param …)`:
    # a call to a known forwarder whose arguments include a bare identifier that
    # is the calling function's own parameter.
    changed = True
    while changed:
        changed = False
        for text in sources.values():
            lines = text.splitlines()
            for known in sorted(found):
                for match in re.finditer(rf"\b{re.escape(known)}\s*\(", text):
                    index = text[: match.start()].count("\n")
                    owner = enclosing_fn(lines, index)
                    if not owner or owner in found:
                        continue
                    arguments = identifiers_in_call(text, match.end())
                    if any(forwards_a_parameter(text, owner, a) for a in arguments):
                        found.add(owner)
                        changed = True
    return found


def sim_registered() -> set[str]:
    """Every system name registered into a SIMULATION schedule.

    ⚠ Matches `add_systems(sim, …)` and `add_systems(app.sim_schedule(), …)`
    blocks and collects the identifiers inside. Deliberately generous: a name
    that appears in such a block is reported as simulation-scheduled, and a
    FALSE POSITIVE here is visible (the reader is named and can be checked)
    while a false negative would be silence.
    """
    names: set[str] = set()
    listing = subprocess.run(
        ["git", "grep", "-l", "add_systems"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    ).stdout.split()
    sources = {}
    for path in listing:
        if not path.endswith(".rs"):
            continue
        # ⛔ THE ATTRIBUTION WALK RUNS ON CODE ONLY — see `without_comments`.
        sources[path] = without_comments((ROOT / path).read_text())

    # ⭐⭐ **REGISTRATION FLOWS THROUGH HELPERS, AND THAT WAS MOST OF THE
    # `UNATTRIBUTED` LIST.** `apply_feature_hit_events` — the one system the
    # architecture review names by hand — is not inside any `add_systems(sim, …)`
    # call. It is an argument to `install_technique(app, KEY, offer, (…systems…))`,
    # whose own body is `app.add_systems(sim, systems)`. A scan for the literal
    # call answers "not scheduled" about a system that ships.
    #
    # ⇒ So FIRST find the forwarders — transitively, because the real chain is two
    # deep — and then a name passed to one of those is sim-registered too. This
    # still cannot see a registration assembled from a table or behind a `cfg`, so
    # `UNATTRIBUTED` remains "not checked", never "safe".
    forwarders = set(find_sim_forwarders(sources))
    for text in sources.values():
        for forwarder in forwarders:
            for match in re.finditer(rf"\b{re.escape(forwarder)}\s*\(", text):
                names.update(identifiers_in_call(text, match.end()))

    for path, text in sources.items():
        for match in re.finditer(
            r"add_systems\(\s*(sim|sim_schedule|app\.sim_schedule\(\)|"
            r"[A-Za-z_:]*GgrsSchedule)\s*,",
            text,
        ):
            names.update(identifiers_in_call(text, match.end()))
    return names


def main() -> int:
    found = readers()
    if not found:
        print("⛔ NO `Res<UserSettings>` READER FOUND AT ALL.")
        print("   That is an INSTRUMENT failure, not a clean bill of health — the")
        print("   type was renamed or re-exported and this scan is about nothing.")
        return 1
    sim = sim_registered()
    if not sim:
        print("⛔ NO SIMULATION-SCHEDULED SYSTEM FOUND AT ALL — the attribution half")
        print("   of this instrument is broken, so every row below would read SAFE.")
        return 1

    # ⛔⛔ **THE CONTROL, AND IT IS NOT DECORATION.** `apply_feature_hit_events` is
    # verified BY HAND to run in the simulation schedule — the architecture review
    # names it, and its registration is
    # `crates/ambition_platformer2d_runtime/src/combat_schedule.rs:695`, two
    # forwarder hops from any literal `add_systems(sim, …)`. It is therefore the
    # one row whose correct answer is known independently of this script.
    #
    # ⇒ Without this, a regression in the forwarder closure makes the simulation
    # list SHRINK, and a shrinking count is exactly what progress looks like. This
    # census has already reported two confident wrong answers (a `Res<UserSettings>`
    # matched inside a comment; a prose `)` closing a registration block early), so
    # it asserts against a fact it cannot derive.
    # ⛔⛤ **AND THE CONDITION IS ABOUT THE SCHEDULE, NOT ABOUT THE READER SET.**
    # This first read `if control in found and control not in sim`, and the moment
    # `apply_feature_hit_events` was migrated OFF `UserSettings` the control's
    # subject left the population and the check stopped applying — silently, on
    # the very run that first reported ZERO simulation readers. A control that
    # switches itself off exactly when the census reaches its goal is the shape
    # `reference_a_check_that_cannot_fail` is about. Its registration is a fact
    # about the schedule and holds whatever it reads.
    control = "apply_feature_hit_events"
    if control not in sim:
        print(f"⛔ THE CONTROL FAILED: `{control}` is not attributed to a simulation")
        print("   schedule. It is registered at combat_schedule.rs:695 through")
        print("   `install_technique` → `install_techniques` → `add_systems(sim, …)`.")
        print("   The attribution half of this instrument is broken and EVERY")
        print("   `UNATTRIBUTED` row below is unreliable — including rows that")
        print("   would otherwise read as a clean bill of health.")
        return 1

    in_sim = sorted(name for name in found if name in sim)
    elsewhere = sorted(name for name in found if name not in sim)

    print(f"{len(found)} production function(s) take `Res<UserSettings>`.\n")
    print(f"⛔ READ INSIDE THE SIMULATION SCHEDULE ({len(in_sim)}) — under the rollback")
    print("   host this is `GgrsSchedule`, so a replay of frame N reads whatever the")
    print("   settings menu says NOW:")
    for name in in_sim:
        for path in sorted(found[name]):
            print(f"     {name:<44} {path}")
    print(f"\n⚠ UNATTRIBUTED TO A SIMULATION SCHEDULE ({len(elsewhere)}) — this scan did")
    print("   not find them in an `add_systems(sim, …)` block. That is NOT a clearance:")
    print("   a system registered through an intermediate, a helper, or a set this scan")
    print("   cannot follow lands here too. Check each before treating it as safe:")
    for name in elsewhere:
        for path in sorted(found[name]):
            print(f"     {name:<44} {path}")
    print(
        "\n⇒ The classification each SIMULATION reader needs (review, 2026-09-13):\n"
        "   · LOCAL INPUT INTERPRETATION (movement/aim/camera frame) — resolve at the\n"
        "     INPUT-CAPTURE boundary so deterministic simulation consumes semantic\n"
        "     intent; peers must not have to share accessibility settings.\n"
        "   · GAME-MECHANICAL POLICY (difficulty, assist, damage scaling) — needs a\n"
        "     deterministic session/match projection. Whether that is match-wide or\n"
        "     per-participant is Jon's product call; neither answer permits a read of\n"
        "     an App-local persisted resource during historical simulation.\n"
        "   · PRESENTATION — stays ordinary mutable `UserSettings`."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
