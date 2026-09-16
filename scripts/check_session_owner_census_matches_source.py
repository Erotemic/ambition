#!/usr/bin/env python3
"""C03's session-owner census must still be the census SOURCE has.

⛔⛤ **IT DRIFTED BY FOUR AND NOBODY NOTICED UNTIL THE CAMPAIGN BECAME
STARTABLE.** `consolidation-plan.md` opened C03 with *"Source explicitly groups
32 App resources as gameplay-session or activated-generation state"* and named
`SessionScopedResources` (25). MEASURED 2026-09-16, field by field, it is 29 —
`StocksMatchSettled`, `SuddenDeathEntered`, `LiveMatchTicks` and
`SessionMatchOrdinal` became members while the campaign waited on its gates. The
drift is not neglect; it is the campaign's own SUBJECT moving, which is what any
census pinned to a commit does. ⇒ A campaign whose starting census is four rows
stale starts by consolidating a set it has not enumerated.

⭐ **SO THE PLAN CARRIES A MACHINE-READABLE COPY AND THIS COMPARES IT TO
SOURCE.** The prose stays for readers; the HTML comment is the one the guard
reads. That is deliberately a SECOND copy of the number — the point is not to
avoid a second copy, which prose already was, but to make the second copy
CHECKABLE.

⚠ **IT COUNTS RESOURCES, NOT FIELDS, and the distinction is what the first
parser got wrong.** `SessionScopedResources` and `SessionOwnedCheckpointState`
are `SystemParam` bundles whose every field is one `ResMut<'w, T>` — one resource
each. `SessionMechanics` is ONE resource that happens to have six fields, and
counting its fields reads the total as 41 instead of 36.
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
PLAN = REPO / "docs/planning/consolidation/consolidation-plan.md"
MARKER = re.compile(r"<!--\s*session-owner-census:\s*(.+?)\s*-->")

#: group name -> (file, how to count it)
BUNDLES = {
    "SessionScopedResources": "crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs",
    "SessionOwnedCheckpointState": "crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs",
}
#: ⚠ NOT a bundle: one resource with six fields. Counting its fields is the
#: mistake this guard's docstring records, so it is asserted to EXIST and is
#: never counted by field.
SINGLETON = (
    "SessionMechanics",
    "crates/ambition_platformer2d_actor_monolith/src/session/mechanics.rs",
)

RESMUT_FIELD = re.compile(r"^\s{4}(?:pub\s+)?\w+:\s*ResMut<'w,\s*.+?>,\s*$", re.M)


def declared() -> dict[str, int]:
    match = MARKER.search(PLAN.read_text(encoding="utf-8"))
    if not match:
        raise SystemExit(
            "⛔⛔ the plan carries no `session-owner-census` marker. A guard that "
            "cannot find its subject must refuse, not report clean."
        )
    return {
        name: int(value)
        for name, value in (pair.split("=") for pair in match.group(1).split())
    }


def bundle_members(rel: str, name: str) -> int:
    text = (REPO / rel).read_text(encoding="utf-8")
    start = text.index(f"pub struct {name}")
    end = text.index("\n}\n", start)
    return len(RESMUT_FIELD.findall(text[start:end]))


def main() -> int:
    stated = declared()
    findings = []

    for name, rel in BUNDLES.items():
        if name not in stated:
            findings.append(f"  the marker does not state a count for {name}")
            continue
        real = bundle_members(rel, name)
        # ⛔ ANTI-VACUITY. A struct whose fields stopped matching the pattern
        # reads as zero, and zero would quietly "disagree" forever or, if the
        # marker were ever 0, agree with nothing.
        if real < 3:
            findings.append(
                f"  {name} parsed as {real} `ResMut` field(s) in {rel}; the "
                "scan is broken, not the census"
            )
            continue
        if real != stated[name]:
            findings.append(
                f"  {name}: the plan says {stated[name]}, {rel} has {real}"
            )

    name, rel = SINGLETON
    if f"pub struct {name}" not in (REPO / rel).read_text(encoding="utf-8"):
        findings.append(f"  {name} is not declared in {rel} any more")
    elif stated.get(name) != 1:
        findings.append(
            f"  {name} is ONE resource; the plan says {stated.get(name)}. "
            "Counting its FIELDS is the error this guard exists to prevent."
        )

    if findings:
        print("⛔ C03's session-owner census no longer matches source:\n")
        print("\n".join(findings))
        print(
            "\n⇒ RE-DERIVE THE CENSUS AND UPDATE BOTH THE MARKER AND THE PROSE.\n"
            "  A campaign whose starting census is stale starts by consolidating a\n"
            "  set it has not enumerated — and the drift is usually the campaign's\n"
            "  own subject moving while it waited on its gates."
        )
        return 1

    total = sum(stated.values())
    print(
        f"C03's session-owner census matches source: "
        + ", ".join(f"{k}={v}" for k, v in sorted(stated.items()))
        + f" ({total} App resources)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
