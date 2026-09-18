#!/usr/bin/env python3
"""ONE owner for *"how many workspace packages does the facade drag in?"*

⛔⛤ **SIX PLANNING PAGES STATED THIS NUMBER AND THEY GAVE TWO ANSWERS.**
MEASURED 2026-09-18 across `docs/planning`:

    51   engine/architecture-reassessment.md
    51   engine/project-build-and-distribution.md
    51   engine/capability-and-runtime-composition.md
    51   engine/architecture-review-coverage.md  — beside its OWN ⚠ note saying 48
    48   engine/architecture-review-findings.md

⇒ The re-measurement landed on 2026-09-10 and reached one page. The page that
carried it kept its own stale 51 in the sentence above the correction, and the
three other owners never heard. **A fact with six owners is a fact with no
owner**, which is the whole thesis of the consolidation census, applied to the
census's own corpus.

⛔⛔ **AND THE SECOND HALF OF THE CLAIM IS NOW FALSE, NOT MERELY STALE.** Three
pages say the mandatory closure *"includes facade -> host -> render"*. It does
not: the host's manifest makes `ambition_render` OPTIONAL and the facade takes
the host with `default-features = false`, so `ambition_render` is not in the
normal graph at all. ⇒ That is the more dangerous of the two errors, because it
describes an architectural property backwards — a reader planning a render
decoupling would start from a path that is already cut.

# # The method, and why it is stated rather than assumed

This reproduces the traversal those pages published, verbatim in behaviour:
normal `[dependencies]` only, non-optional only, `path` entries only, breadth
first from `ambition_platformer2d`. ⚠ It is a LOWER bound and stays one — it
excludes optional edges, feature activation, build/dev/target-specific edges,
inherited dependency features and external crates, and Cargo's resolved profile
can require more. The number's value is as a ratchet on ONE definition, not as
a measure of weight.

# # What this checks

1. The closure matches `CLOSURE`, so the ratchet cannot drift unnoticed.
2. `ambition_render` stays OUT of it. This is the architectural property, and
   it is the one that can regress by a single `optional = true` being dropped.
3. **Every planning page that restates the number agrees with the measurement.**
   That is the part that collapses the six owners: a page may still explain the
   number, but it may no longer disagree about it.

⚠ A FALLING closure is progress, not a failure — the whole point of the render
split was to lower it. The check fails on any CHANGE and names the direction, so
the number moves deliberately and every page moves with it.
"""

from __future__ import annotations

import re
import tomllib
from collections import deque
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

#: The facade every page traverses from.
FACADE = "ambition_platformer2d"

#: MEASURED 2026-09-18 at this method. Reproduces the 2026-09-10 reading at
#: `939d6aaa5` and the 2026-09-17 `cargo tree` reading exactly.
CLOSURE = 48

#: ⛔ This package must not re-enter the mandatory graph. The host declares it
#: `optional = true` and the facade takes the host with
#: `default-features = false`; dropping either makes the renderer a
#: no-default-features consumer's compile dependency again.
MUST_STAY_OUT = "ambition_render"

#: ⛔ Anti-vacuity. If the workspace member list stops parsing, every count
#: above collapses to zero and the render assertion passes for the wrong reason.
FLOORS = {"workspace packages": 60}

#: How a page spells the number. The noun phrase is the pages' own.
RESTATEMENT = re.compile(r"(\d+)\s+other workspace packages")

#: A reading this closure genuinely HAD, and the reference point that makes it
#: true. A page may state one of these — history is worth recording — but only
#: on a line that marks it as history.
#:
#: ⛔⛤ **THIS EXISTS BECAUSE THE FIRST VERSION OF THIS CHECK FORBADE HISTORY.**
#: Within an hour of landing, restructuring
#: `capability-and-runtime-composition.md` into a baseline column and a
#: re-measured column made it fail: the baseline column correctly says 51. A
#: check that cannot tell *"this is what it was"* from *"this is what it is"*
#: pushes an author to delete the provenance, which is the opposite of the
#: collapse it was built for.
#:
#: ⚠ **AND THE EXEMPTION IS THE PLACE A DEFECT WOULD HIDE**, so it is narrow and
#: counted: a historical value is allowed only on a line containing
#: `HISTORY_MARKER`, and `main()` prints how many such statements exist. A page
#: that quietly relabels a current number as a baseline shows up as a rising
#: count in a green run, not as silence.
HISTORICAL: dict[int, str] = {
    51: (
        "the `300004d601af1e633cfaee969f079cf9bb368ca8` review baseline, when "
        "`ambition_render` was still mandatory through the host"
    ),
}

#: ⛔⛔ **THE MARKER HAS ONE KEEPER AND THIS IS NOT IT.** `check_planning_citations`
#: owns *"this coordinate is wrong on purpose"* as `cite-ok`, and
#: `marker_suppresses`'s own doc block says any new reader calls it rather than
#: re-spelling the rule — because a fourth spelling of it once made a documented
#: affordance a trap: a marker one line BELOW its subject was legal by the
#: documented rule and reddened a guard that only accepted the same line. ⇒ This
#: check imports that predicate instead of inventing `HISTORY_MARKER`, so an
#: author who follows the documentation gets a green from both.
#:
#: ⚠ My first attempt WAS a fourth spelling — *"the line must contain the word
#: `baseline`"* — and it failed immediately on a table whose `baseline` sits in
#: the HEADER row. The marker has to be local and deliberate, which is exactly
#: what `cite-ok` already is.
from check_planning_citations import MARKER, marker_suppresses  # noqa: E402


def _packages() -> dict[str, tuple[Path, dict]]:
    workspace = tomllib.loads((REPO / "Cargo.toml").read_text())
    out: dict[str, tuple[Path, dict]] = {}
    for member in workspace["workspace"]["members"]:
        directory = (REPO / member).resolve()
        manifest = tomllib.loads((directory / "Cargo.toml").read_text())
        out[manifest["package"]["name"]] = (directory, manifest)
    return out


def closure() -> dict[str, list[str]]:
    """`{package: the path the traversal reached it by}`, including the facade."""
    packages = _packages()
    by_directory = {directory: name for name, (directory, _) in packages.items()}

    edges: dict[str, list[str]] = {}
    for name, (directory, manifest) in packages.items():
        edges[name] = []
        for dep in manifest.get("dependencies", {}).values():
            if not isinstance(dep, dict) or dep.get("optional", False):
                continue
            if "path" in dep:
                target = by_directory.get((directory / dep["path"]).resolve())
                if target:
                    edges[name].append(target)

    paths = {FACADE: [FACADE]}
    queue = deque([FACADE])
    while queue:
        source = queue.popleft()
        for target in sorted(edges.get(source, ())):
            if target not in paths:
                paths[target] = paths[source] + [target]
                queue.append(target)
    return paths


def restatements() -> dict[str, list[tuple[int, int]]]:
    """`{page: [(line, number), ..]}` for every LIVE restatement of the closure.

    A statement of a `HISTORICAL` value on a line marked with `HISTORY_MARKER`
    is not a live restatement and is returned by `history()` instead.
    """
    found: dict[str, list[tuple[int, int]]] = {}
    for page in sorted((REPO / "docs" / "planning").rglob("*.md")):
        rel = page.relative_to(REPO).as_posix()
        lines = page.read_text(errors="replace").splitlines()
        for index, line in enumerate(lines, 1):
            for match in RESTATEMENT.finditer(line):
                value = int(match.group(1))
                if value in HISTORICAL and marker_suppresses(lines, index):
                    continue
                found.setdefault(rel, []).append((index, value))
    return found


def history() -> list[tuple[str, int, int]]:
    """`(page, line, value)` for every statement the `HISTORICAL` table allows."""
    out: list[tuple[str, int, int]] = []
    for page in sorted((REPO / "docs" / "planning").rglob("*.md")):
        rel = page.relative_to(REPO).as_posix()
        lines = page.read_text(errors="replace").splitlines()
        for index, line in enumerate(lines, 1):
            for match in RESTATEMENT.finditer(line):
                value = int(match.group(1))
                if value in HISTORICAL and marker_suppresses(lines, index):
                    out.append((rel, index, value))
    return out


def main() -> int:
    packages = _packages()
    if len(packages) < FLOORS["workspace packages"]:
        print(
            f"FAIL: only {len(packages)} workspace package(s) parsed, below the floor of "
            f"{FLOORS['workspace packages']} — every reading below would be about the "
            "parser rather than the tree"
        )
        return 1

    paths = closure()
    reached = len(paths) - 1
    problems: list[str] = []

    if reached != CLOSURE:
        direction = "FELL" if reached < CLOSURE else "GREW"
        problems.append(
            f"the facade's mandatory closure {direction}: {CLOSURE} -> {reached}.\n"
            f"    A fall is progress and a rise is weight; either way update `CLOSURE` here "
            f"AND every page `restatements()` lists, in the same commit."
        )

    if MUST_STAY_OUT in paths:
        problems.append(
            f"`{MUST_STAY_OUT}` is back in the facade's mandatory graph, by\n"
            f"    {' -> '.join(paths[MUST_STAY_OUT])}\n"
            f"    Suspect a dropped `optional = true` on the host's dependency, or a "
            f"`default-features` flip on the facade's dependency on the host."
        )

    disagreeing = {
        page: [(line, value) for line, value in rows if value != reached]
        for page, rows in restatements().items()
    }
    disagreeing = {page: rows for page, rows in disagreeing.items() if rows}
    if disagreeing:
        detail = "\n".join(
            f"    {page}:{line} says {value}, measured {reached}"
            for page, rows in sorted(disagreeing.items())
            for line, value in rows
        )
        problems.append(
            "a planning page states a closure that is not the measured one:\n"
            + detail
            + "\n    A page may explain this number. It may not disagree about it — six "
            "owners with two answers is what this check exists to end."
        )

    if problems:
        print("\n".join(f"FAIL: {p}" for p in problems))
        return 1

    pages = restatements()
    past = history()
    print(
        f"ok: the facade's mandatory closure is {reached} other workspace package(s) "
        f"of {len(packages)}, `{MUST_STAY_OUT}` is outside it, and all "
        f"{sum(len(r) for r in pages.values())} live restatement(s) across {len(pages)} "
        f"planning page(s) agree"
    )
    if past:
        # ⛔ COUNTED, because this is the one road a stale number can still take:
        # relabel it as a baseline and the check goes quiet. A rising number here
        # in an otherwise green run is the signal.
        print(
            f"   plus {len(past)} historical statement(s) the `HISTORICAL` table allows: "
            + ", ".join(f"{page}:{line} = {value}" for page, line, value in past)
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
