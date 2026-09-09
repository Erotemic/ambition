#!/usr/bin/env python3
"""Workspace crates that DECLARE an `ambition_*` dependency and never name it.

⭐ WHY THIS EXISTS. A dependency nobody names is not free: it is an ARCHITECTURAL
CLAIM. `ambition_abilities` declared `ambition_boss_encounter` and referenced it
zero times, which said "an abilities crate needs a boss system" to every reader,
every SCC measurement and every closure argument in the planning documents — and
it dragged `ambition_encounter`, `ambition_persistence` and `ambition_cutscene`
behind it in a manifest walk. The compiler has no opinion about an unused
dependency, and neither does any test.

⛔⛔ **IT IS A CANDIDATE LIST, NOT A VERDICT, AND THE COMPILER IS THE JUDGE.** A
dependency can legitimately have no source reference:

  · it is FORWARDED (`"dep/feature"` in this crate's own feature table) — those
    are filtered out here, because the manifest names them;
  · it is named only through a MACRO expansion or a `$crate` path;
  · it is linked for a trait impl or an inventory-style registration whose only
    mention is in the other crate;
  · a build script needs it.

⛔⛔ **AND "REMOVE IT AND COMPILE" IS NOT THE RULE.** A green build does not
prove that a link-time registration, an externally supplied trait impl, or a
build script's behaviour was irrelevant — those are precisely the cases the list
above names, and every one of them compiles fine after the line is gone. Stating
the compiler as the judge would make this tool an oracle for graph tidiness,
which is the incentive the `actor_spawn` carve already cost this repository once.

⇒ **THE RULE IS: establish WHY the dependency is declared before removing it.**
Read the manifest entry's own comment and the feature table for what it forwards;
grep the crate for the DEPENDENCY'S vocabulary rather than its name (a re-export,
a `use` alias, a macro path); ask whether the dependency's plugin or registration
is what makes some behaviour exist. Only then remove, compile, and run the
behavioural witness for anything that is not an ordinary source-level use.

Measured 2026-09-09: four candidates, all four removable on that standard — and
one of them (`ambition_characters`'s `causal`) was a FEATURE whose doc promised
to *"publish this capability's causal facts"* while the crate contained no
`cfg(feature = "causal")` at all, a capability a composition could turn on, pay a
compile for, and receive nothing from. That one was decided by reading, not by
the build.

⚠ It reads the crate's whole source tree including tests, so a dependency used
only by a test still counts as referenced. That is deliberate: a test-only use is
a use, and `[dev-dependencies]` is where the manifest says so — this looks only at
normal dependencies.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))
from cargo_bin import cargo_binary  # noqa: E402


def unreferenced_in(
    declared: set[str], source: str, manifest_text: str
) -> list[str]:
    """The declared names this crate's source never mentions and its manifest
    never forwards.

    ⭐ SPLIT OUT SO THE THREE ANSWERS ARE SEPARATELY TESTABLE, which is
    D-BUILD-GRAPH-BLINDNESS's whole point: DECLARED-BUT-UNUSED, FEATURE-GATED and
    ACTUALLY LINKED are different facts, and a measurement that conflates them is
    worse than none. This function answers only the first, from text; the closure
    question belongs to `cargo tree` and lives in
    `check_absence_contracts.py`'s featureless-facade contract.
    """
    return [
        name
        for name in sorted(declared)
        if not re.search(rf"\b{re.escape(name)}\b", source)
        # Forwarding a feature IS naming the dependency, in the one file where
        # naming it is the whole point.
        and not re.search(rf'"{re.escape(name)}[?]?/', manifest_text)
    ]


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    meta = json.loads(
        subprocess.run(
            [cargo_binary(), "metadata", "--no-deps", "--format-version", "1"],
            cwd=root,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )
    members = {package["name"] for package in meta["packages"]}
    rows: list[tuple[str, list[str]]] = []
    for package in meta["packages"]:
        manifest = Path(package["manifest_path"])
        source = "\n".join(
            path.read_text(errors="ignore")
            for path in manifest.parent.rglob("*.rs")
        )
        manifest_text = manifest.read_text()
        declared = {
            dependency["name"]
            for dependency in package["dependencies"]
            # `kind is None` is a NORMAL dependency; dev and build kinds are a
            # different claim and are not this script's subject.
            if dependency["name"] in members and dependency["kind"] is None
        }
        unreferenced = unreferenced_in(declared, source, manifest_text)
        if unreferenced:
            rows.append((package["name"], unreferenced))

    for name, unreferenced in sorted(rows):
        print(f"{name}: {', '.join(unreferenced)}")
    print(
        f"\n{len(rows)} crate(s) declare an unreferenced ambition dependency "
        f"(of {len(members)} workspace members)"
    )
    print(
        "⚠ candidates, not violations. Establish WHY each is declared "
        "(feature forwarding, a registration, a trait impl, a build script) "
        "before removing it; a green build is not the judge — see this "
        "script's own docstring."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
