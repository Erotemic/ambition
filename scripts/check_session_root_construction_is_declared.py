#!/usr/bin/env python3
"""Every production site that CONSTRUCTS a canonical `SessionRoot` is declared.

⭐⛤ **`Q132` WAS DECIDED 2026-09-19: THERE IS EXACTLY ONE CANONICAL LIVE
`SessionRoot`, AND TWO PUBLISHED ROOTS ARE INVALID.** Two runtime arms hold the
invariant in the shipped composition —
`the_shipped_app_never_holds_two_session_roots_across_a_handoff` counts roots
every frame across a real handoff, and
`a_prepared_candidate_never_counts_as_a_canonical_session_root` supplies the
population an ordinary query cannot see, because `InactiveCandidate` is a
DISABLING component and hides a candidate from it.

⛔ **NEITHER ARM CAN SEE A COMPOSITION THAT DOES NOT EXIST YET, WHICH IS HOW
THIS INVARIANT WILL ACTUALLY BREAK.** They drive ONE route in ONE app. A new
host that spawns its own root, or a plugin that mints one on the side, is
invisible to them until somebody writes an arm for that composition too. The
countable thing is who may CONSTRUCT a root at all, and today that is four
places.

⚠ **A DECLARATION IS NOT A CONSTRUCTION, AND THE FIRST SCAN COUNTED IT.**
`pub struct SessionRoot(pub SessionScopeId);` matches `SessionRoot(` exactly as
a spawn does, so a naive count reads FIVE sites where there are four. The
declaration is excluded by shape rather than by path, because moving it would
otherwise silently re-add it.

⚠ **THIS IS A STATIC RATCHET, NOT A PROOF OF THE INVARIANT.** It cannot say
whether two of these run at once; that is what the runtime arms are for. What
it says is that the POPULATION entitled to create a canonical root has not
grown without somebody writing down why.
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts" / "lib"))
sys.path.insert(0, str(REPO / "scripts"))

from test_paths import file_is_test_only, is_test_path, strip_test_modules  # noqa: E402

from a_rollback_arm_must_refuse_a_frozen_world import code_only  # noqa: E402

#: Every production file allowed to construct a `SessionRoot`, and why.
#: RE-MEASURED 2026-09-21. A new entry is a decision about `Q132`'s invariant and
#: belongs in a commit that says so.
#:
#: ⛔⛤ **THE CANONICAL ROAD MOVED ON 2026-09-19 AND THIS LIST DID NOT FOLLOW IT
#: THE SAME DAY.** It named `ambition_platformer2d_provider/src/lifecycle.rs`
#: as *"the provider's session activation — the canonical road"*; `ef54aeff3`
#: ("A candidate is not the session, and hiding one never said that") made the
#: provider spawn a `CandidateSessionRoot` instead, and moved the one moment a
#: candidate BECOMES the session into the admission path below. ⇒ The population
#: did not grow, it MOVED — which this guard reported as one undeclared entry
#: and one declared-but-gone, the shape a move always takes here.
DECLARED: dict[str, str] = {
    "crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs": (
        "the A10 candidate-admission path, and the ONE moment a candidate stops "
        "being a candidate — `Q132`'s invariant expressed as a swap rather than "
        "an insert: `CandidateSessionRoot` is removed and `SessionRoot` inserted "
        "in the same statement. ⚠ It cannot coexist with another root because it "
        "runs AFTER the retirement barriers inside an exclusive-world call, so no "
        "frame elapses in which the retiring live root and this one both answer "
        "to `SessionRoot`. The ordering is the argument; the site says so itself"
    ),
    "crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs": (
        "`insert_session_world_component`'s fallback, *\"for small direct hosts "
        "and focused tests that intentionally assemble the same root one "
        "component at a time\"*. ⚠ It mints `active_scope.unwrap_or(SessionScopeId(0))` "
        "— an anonymous default identity when no session is active, which is the "
        "shape `Q132`'s scoping rule names. ONE production caller today: "
        "`game/ambition_app/src/app/dev_runtime.rs:626`"
    ),
    "crates/ambition_platformer2d_provider/src/lifecycle.rs": (
        "`install_direct_session_root`, the one road for a direct-entry demo "
        "(Mary-O's and Sanic's demo content layers, which used to hand-build "
        "their roots) to mint a build-time root from prepared content. ⚠ It "
        "cannot coexist with another root because it ASSERTS both conditions "
        "that would make one: it refuses a shell-gated App (a shell activation "
        "mints its own root) and refuses an App that already holds a "
        "`SessionRoot`"
    ),
}

#: ⛔ ANTI-VACUITY. A scan that stopped finding files would report an empty set
#: and pass. MEASURED 2026-09-19: 1,294 production files.
FLOOR = 1150

#: A construction, not the tuple-struct declaration.
CONSTRUCTION = re.compile(r"(?<!struct )\bSessionRoot\s*\(")
#: The declaration itself, excluded by SHAPE so moving the file cannot re-add it.
DECLARATION = re.compile(r"\bstruct\s+SessionRoot\s*\(")


def production_sources() -> list[tuple[str, str]]:
    out = []
    for root in ("crates", "game"):
        for path in sorted((REPO / root).rglob("*.rs")):
            raw = path.read_text(encoding="utf-8", errors="replace")
            if is_test_path(path) or file_is_test_only(raw):
                continue
            out.append(
                (str(path.relative_to(REPO)), code_only(strip_test_modules(raw)))
            )
    return out


def construction_sites(sources) -> dict[str, list[int]]:
    found: dict[str, list[int]] = {}
    for rel, body in sources:
        for match in CONSTRUCTION.finditer(body):
            # The declaration's own line is not a construction.
            line_start = body.rfind("\n", 0, match.start()) + 1
            line_end = body.find("\n", match.start())
            line = body[line_start : line_end if line_end >= 0 else len(body)]
            if DECLARATION.search(line):
                continue
            found.setdefault(rel, []).append(body[: match.start()].count("\n") + 1)
    return found



#: ⛔⛤ **`Q132`'S CONSEQUENCE 7 SAYS "ADD **AND RETAIN**", AND UNTIL 2026-09-19
#: THE RETAIN HALF HAD NO MECHANISM.** Both arms were named in this docstring
#: and nowhere a machine reads, so deleting either would have removed the only
#: production evidence for the invariant and reddened nothing. A witness that
#: can vanish silently is a witness with an expiry date nobody set.
#:
#: ⚠ **THE PAIR IS THE UNIT, NOT EITHER ARM.** `InactiveCandidate` is a Bevy
#: DISABLING component, so an ordinary query cannot see a prepared candidate —
#: which means the counting arm is green whether or not a candidate was ever
#: prepared. Only the second arm supplies that population. Requiring one
#: without the other would pin the half that can pass vacuously.
WITNESSES: dict[str, tuple[str, str]] = {
    "the_shipped_app_never_holds_two_session_roots_across_a_handoff": (
        "game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs",
        "counts roots every frame across a real shell handoff, so a transition that "
        "briefly publishes two reddens instead of passing",
    ),
    "a_prepared_candidate_never_counts_as_a_canonical_session_root": (
        "game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs",
        "supplies the population an ordinary query cannot see, so the arm above "
        "cannot be green merely because no candidate existed",
    ),
}


def missing_witnesses() -> list[str]:
    """Each named arm still declared in the file that is supposed to hold it."""
    out = []
    for name, (rel, why) in sorted(WITNESSES.items()):
        path = REPO / rel
        if not path.exists():
            out.append(f"{rel} is gone, and it held `{name}`, which {why}")
            continue
        if f"fn {name}(" not in path.read_text(encoding="utf-8", errors="ignore"):
            out.append(
                f"`{name}` is no longer declared in {rel}. It {why}. `Q132`'s ruling "
                "requires production-composition witnesses that a handoff never "
                "exposes two canonical roots; restore it, or move it and update this "
                "entry in the same commit."
            )
    return out


def main() -> int:
    sources = production_sources()
    if len(sources) < FLOOR:
        print(
            f"⛔⛔ scanned {len(sources)} production Rust file(s), below the floor of "
            f"{FLOOR}. That is a claim about this scan, not about the tree."
        )
        return 1

    found = construction_sites(sources)
    undeclared = sorted(set(found) - set(DECLARED))
    vanished = sorted(set(DECLARED) - set(found))

    if undeclared or vanished:
        print("the population entitled to construct a canonical `SessionRoot` moved:")
        for rel in undeclared:
            lines = ", ".join(str(n) for n in found[rel])
            print(
                f"  ⛔ UNDECLARED: {rel}:{lines} constructs a `SessionRoot`. `Q132` "
                "rules that exactly one canonical root may be live; a new place "
                "that can create one is a decision about that invariant. Declare it "
                "here with what composition it serves and why it cannot coexist "
                "with another root, or route it through the provider activation."
            )
        for rel in vanished:
            print(
                f"  ⚠ DECLARED BUT GONE: {rel} no longer constructs a `SessionRoot`. "
                "That is usually good news — delete the entry in the same commit so "
                "this list keeps meaning something."
            )
        return 1

    gone = missing_witnesses()
    if gone:
        print("the `Q132` invariant lost a production witness:")
        for line in gone:
            print(f"  ⛔ {line}")
        return 1

    total = sum(len(v) for v in found.values())
    print(
        f"ok: {total} production `SessionRoot` construction(s) across "
        f"{len(found)} declared file(s), scanned over {len(sources)} production file(s)"
    )
    print(f"  and {len(WITNESSES)} runtime witness(es) of the invariant still present")
    return 0


if __name__ == "__main__":
    sys.exit(main())
