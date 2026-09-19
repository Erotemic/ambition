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
#: MEASURED 2026-09-19. A new entry is a decision about `Q132`'s invariant and
#: belongs in a commit that says so.
DECLARED: dict[str, str] = {
    "crates/ambition_platformer2d_provider/src/lifecycle.rs": (
        "the provider's session activation — the canonical road, and the one "
        "that publishes a root as part of a prepared bundle"
    ),
    "crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs": (
        "`insert_session_world_component`'s fallback, *\"for small direct hosts "
        "and focused tests that intentionally assemble the same root one "
        "component at a time\"*. ⚠ It mints `active_scope.unwrap_or(SessionScopeId(0))` "
        "— an anonymous default identity when no session is active, which is the "
        "shape `Q132`'s scoping rule names. ONE production caller today: "
        "`game/ambition_app/src/app/dev_runtime.rs:626`"
    ),
    "game/ambition_demo_mary_o/src/lib.rs": (
        "`MaryODemoContentPlugin`, a direct-entry demo content layer. ⚠ NO "
        "production caller: `add_demo_content` is installed only by tests, and "
        "neither `ambition_demo_mary_o_app` nor `ambition_app` adds it"
    ),
    "game/ambition_demo_sanic/src/lib.rs": (
        "the Sanic equivalent, and the same shape — installed by tests only"
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

    total = sum(len(v) for v in found.values())
    print(
        f"ok: {total} production `SessionRoot` construction(s) across "
        f"{len(found)} declared file(s), scanned over {len(sources)} production file(s)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
