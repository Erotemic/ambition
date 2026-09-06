"""Everything the Smash stage declares on arrival is handed back on departure.

⛔⛔ THIS GUARD EXISTS BECAUSE A POISON WALKED OUT UNHARMED. Deleting the
give-back branch for `SmashLimitFill` failed NO test: the "declares nothing"
arm asks an app that never entered the stage, so it cannot see a rule that was
declared and then left standing. ⇒ A pair of arms for ARRIVAL is not a pair for
DEPARTURE, and the resources on this seam are the ones that reach every body in
whatever app composed the ruleset — three separate bugs in one day
(`99ab15e32`, `PlayerManaRegen`, `SmashLimitFill`), each fixed alone.

⭐ WHAT IT CHECKS AND WHY THAT SHAPE. The declaring system captures a PRIOR
struct, inserts its own answers, and on leaving restores each captured value.
The failure mode is a fourth answer added to the arrival branch with no matching
restore — so this compares the two branches' COUNTS and requires every field of
the prior struct to be read in the departure branch. It cannot check that the
right TYPE was restored; what it holds is that nobody adds a declaration and
forgets the give-back, which is the mistake that actually happened.

⚠ TEXTUAL, AND THE ALTERNATIVE IS WORSE. The behavioural version is a test per
resource in the app crate, which is what already exists and what the poison
walked through: those tests are green when a give-back is missing, because the
arm that would notice asks a world that never entered the state.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
RULESET = REPO / "game/ambition_demo_smash/src/lib.rs"
SYSTEM = "fn the_stage_declares_smashs_presentation_and_gives_it_back"
PRIOR = "struct SmashPresentationPrior"


def _system_body(text: str) -> str:
    start = text.index(SYSTEM)
    # The declaring system is the last item this guard cares about; take to the
    # next top-level `fn ` so a later system's inserts cannot leak in.
    rest = text[start + len(SYSTEM):]
    end = rest.index("\nfn ") if "\nfn " in rest else len(rest)
    return rest[:end]


def test_every_answer_the_stage_declares_is_given_back():
    text = RULESET.read_text()

    # ⛔ ANTI-VACUITY FIRST, in the order that survives a rename: if the system
    # or the prior is gone, every count below is zero and they would agree.
    assert SYSTEM in text, (
        f"`{SYSTEM}` is gone. If the stage declares its answers somewhere else "
        "now, point this guard there — do not delete it: three separate bugs "
        "came from this seam."
    )
    assert PRIOR in text, f"`{PRIOR}` is gone; the give-back has no record to restore from"

    body = _system_body(text)
    code = "\n".join(line.split("//", 1)[0] for line in body.splitlines())

    # ⛔ SPLIT THE BRANCHES, DO NOT FILTER BY BINDING NAME. The departure branch
    # restores with `Some(value) => commands.insert_resource(value)`, so a naive
    # scan of the whole body counts every give-back as a declaration and the two
    # sides always "disagree". The first draft of this guard did exactly that.
    split = code.index("} else if")
    arrival, departure = code[:split], code[split:]

    # Arrival: everything inserted EXCEPT the prior record itself.
    inserts = [
        m for m in re.findall(r"commands\.insert_resource\(\s*([A-Za-z_:][\w:]*)", arrival)
        if "SmashPresentationPrior" not in m
    ]
    # Departure: one `match prior.<field>` per captured answer.
    restores = re.findall(r"match\s+prior\.(\w+)\s*\{", departure)

    assert len(inserts) >= 3, (
        f"only {len(inserts)} declaration(s) found in the arrival branch; this "
        "guard has lost its subject rather than found a tidy system"
    )
    assert len(inserts) == len(restores), (
        f"the stage DECLARES {len(inserts)} answer(s) and gives back "
        f"{len(restores)}: {sorted(inserts)} against {sorted(restores)}.\n"
        "  ⇒ A declaration with no restore outlives the mode. These resources "
        "reach every body in whatever app composed the ruleset, so one left "
        "standing re-caps and empties mana for a game nobody is playing — which "
        "is exactly how the dive drill broke.\n"
        "  fix: add the field to `SmashPresentationPrior`, capture it on "
        "arrival, and restore it on departure."
    )

    prior_block = text[text.index(PRIOR):]
    prior_block = prior_block[: prior_block.index("\n}")]
    fields = re.findall(r"^\s+(\w+):", prior_block, re.M)
    missing = sorted(set(fields) - set(restores))
    assert not missing, (
        f"`SmashPresentationPrior` captures {missing} and the departure branch "
        "never restores them — a value saved and never handed back is the same "
        "leak with a record of what was lost."
    )
