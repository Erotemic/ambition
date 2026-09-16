#!/usr/bin/env python3
"""A registrar method's (kind, sentence) pair may be spelled in ONE place.

⛔⛤ **THE DEFECT THIS EXISTS FOR, MADE BY AN AGENT ON 2026-09-15.** Every
`RollbackRegistrar` method used to spell its `RollbackEntryKind` and its `detail`
sentence TWICE — once on the RECORDING road
(`ambition_platformer2d_runtime/src/rollback/registrar.rs`, which writes the
descriptor the schema baseline and every census read) and once on the INSTALLING
road (`ambition_platformer2d_rollback_ggrs/src/registration.rs`, which adds the
snapshot plugin and the checksum system). Splitting
`resource-canonical-custom-checksum` out of `resource-canonical` changed the
recording road only, and one registration arrived under two different kinds.

⚠ IT WAS CAUGHT, BUT BY ACCIDENT. `RollbackRegistry`'s conflicting-registration
check fired at app build — cross-evidence, not a designed guard, and it can only
see names BOTH roads reach. A kind spelled wrongly on a registration that only
one road installs had nothing checking it at all.

⇒ The pairs now live in `ambition_platformer2d_core::rollback_kind::spelling`,
beside the trait that declares the methods, and both roads reference the const.
THIS CHECK KEEPS THEM THERE: a literal `RollbackEntryKind::X` sitting next to a
literal `detail::Y` on either road is the duplication coming back.

⭐ WHY THE PAIR AND NOT THE METHOD. MEASURED 2026-09-16: across both roads there
were exactly 18 distinct literal (kind, detail) pairs and each occurred EXACTLY
TWICE — a perfect 1:1 with zero disagreements. Two separate parsers of mine got
the METHOD attribution wrong (one grouped by the first match after each `fn` and
invented four recording-only methods; another swallowed the file tail into the
last method and reported three disagreements that were not there). The pair needs
no attribution, so this check reads what the earlier parsers could not.

⚠ WHAT IT DELIBERATELY DOES NOT FLAG. A method whose `detail` comes from the
CALLER — the `*_custom_checksum` family — names a kind with no literal sentence
beside it. There is nothing to collapse there, and demanding a const would be
demanding a table of one-element rows.
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]

ROADS = [
    "crates/ambition_platformer2d_runtime/src/rollback/registrar.rs",
    "crates/ambition_platformer2d_rollback_ggrs/src/registration.rs",
]
TABLE = "crates/ambition_platformer2d_core/src/rollback_kind.rs"

#: A kind literal immediately followed by a sentence literal: the duplicated pair.
PAIR = re.compile(r"RollbackEntryKind::(\w+),\s*detail::(\w+)\b")
#: The collapsed form both roads must use instead.
SPELLED = re.compile(r"spelling::(\w+)\.kind")


def offenders(text: str) -> list[tuple[str, str]]:
    return [(m.group(1), m.group(2)) for m in PAIR.finditer(text)]


def main() -> int:
    table = (ROOT / TABLE).read_text(encoding="utf-8")
    consts = set(re.findall(r"pub const (\w+): Spelling", table))

    # ⛔ ANTI-VACUITY, AND IT IS THE WHOLE POINT HERE. This check reports clean
    # when it finds no duplicated pair — which is also what it reports if a file
    # MOVED and it is reading nothing. A source-text guard whose corpus vanished
    # looks exactly like a repository that fixed the defect.
    if len(consts) < 10:
        print(
            f"⛔⛔ only {len(consts)} `Spelling` const(s) in {TABLE}; a clean "
            "verdict would be a claim about the scan, not about the roads"
        )
        return 1

    findings: list[str] = []
    collapsed = 0
    for road in ROADS:
        path = ROOT / road
        if not path.exists():
            print(f"⛔⛔ {road} is not there; this guard has lost a road")
            return 1
        text = path.read_text(encoding="utf-8")
        used = set(SPELLED.findall(text))
        collapsed += len(used)
        unknown = sorted(used - consts)
        if unknown:
            print(f"⛔⛔ {road} names {unknown}, which {TABLE} does not declare")
            return 1
        for kind, sentence in offenders(text):
            findings.append(
                f"  {road}\n"
                f"     RollbackEntryKind::{kind} + detail::{sentence} written here"
            )

    if collapsed < len(consts):
        print(
            f"⛔⛔ the roads reference only {collapsed} of {len(consts)} declared "
            "pairs. A const nobody reads is a spelling that moved, not one that "
            "collapsed — and the road it left is unguarded."
        )
        return 1

    if findings:
        print(
            f"⛔ {len(findings)} registrar site(s) spell a (kind, sentence) pair "
            "beside the code instead of referencing the one declaration:\n"
        )
        print("\n".join(findings))
        print(
            "\n⇒ ADD THE PAIR TO `rollback_kind::spelling` AND REFERENCE IT FROM\n"
            "  BOTH ROADS. A pair written next to one road is a pair the other\n"
            "  road can disagree with, and the only thing that noticed last time\n"
            "  was a runtime conflict check that covers names both roads reach."
        )
        return 1

    print(
        f"Every (kind, sentence) pair is spelled once: {len(consts)} declared in "
        f"{TABLE}, {collapsed} references across {len(ROADS)} roads, 0 literal "
        "pairs beside the code."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
