#!/usr/bin/env python3
"""The `SessionWorldRef`/`SessionWorldMut` population has ONE owner, checked.

⛔⛤ **THE DEFECT THIS EXISTS FOR, FOUND 2026-09-19: TWO CORRECT NUMBERS THAT
READ AS A CONTRADICTION.** `session.rs` said the aliases were `193` of the
production uses; `BEVY-SESSION-ROOT` in `architecture-census.md` said `183` and
declares itself that population's one owner. Neither was stale. `183` counts the
PARAMETER FORM `SessionWorldRef<`; `193` counts the bare name, which a `use`
import also matches. Ten imports, two numbers, and the method written down in
neither place — so a reader comparing them learns nothing except that somebody
is wrong.

⚠ **A COUNT IN PROSE HAS NO WAY TO NOTICE THE TREE MOVING**, which is the other
half. `consolidation-plan.md`'s per-spelling table read `163/91`, `22/14`, `4/2`
and `11/6` from 2026-09-16, and by 2026-09-19 three of the four had moved. It
also mixed two methods INSIDE ITSELF: the header said *"comments stripped and
tests excluded"* while the helper rows carried all-files counts.

⇒ So both documents now publish machine-readable markers and this compares each
to ONE live measurement. That is what makes the census row's "one owner" claim
true rather than asserted: the plan's split cannot describe a different
population from the census's total, because neither is free to describe anything
but the tree.

⚠ **THE METHOD IS PART OF THE RULE, NOT A DETAIL OF IT.** Everything below is
measured over `crates/` + `game/`, test files dropped, `#[cfg(test)]` modules
stripped and comments stripped, matching the alias as a PARAMETER (`Name<`) and
the helper functions as CALLS (`name(`). Changing any of that changes the number
by more than the drift this guard exists to catch.
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts" / "lib"))
sys.path.insert(0, str(REPO / "scripts"))

from test_paths import (  # noqa: E402
    file_is_test_only,
    is_test_path,
    strip_test_modules,
)

from a_rollback_arm_must_refuse_a_frozen_world import code_only  # noqa: E402

CENSUS = REPO / "docs/planning/consolidation/architecture-census.md"
PLAN = REPO / "docs/planning/consolidation/consolidation-plan.md"

#: The aliases are counted as parameters and the helpers as calls, because that
#: is the question each spelling answers: an alias is USED by instantiating it,
#: a function by calling it. A bare-name count of either also matches its import.
SPELLINGS = {
    "SessionWorldRef": re.compile(r"\bSessionWorldRef\s*<"),
    "SessionWorldMut": re.compile(r"\bSessionWorldMut\s*<"),
    "live_session_world_root": re.compile(r"\blive_session_world_root\s*\("),
    "session_root_for_scope": re.compile(r"\bsession_root_for_scope\s*\("),
}
ALIASES = ("SessionWorldRef", "SessionWorldMut")

CENSUS_MARKER = re.compile(
    r"<!--\s*alias-census:\s*parameter_form=(\d+)\s+files=(\d+)\s*-->"
)
PLAN_MARKER = re.compile(r"<!--\s*alias-split:\s*([^>]*?)\s*-->")
SPLIT_ENTRY = re.compile(r"(\w+)=(\d+)/(\d+)")

#: ⛔ ANTI-VACUITY. A regex that stopped matching would compare 0 against a
#: marker and merely look wrong; a scan that lost its corpus would compare 0
#: against 0 if the markers were regenerated from it. MEASURED 2026-09-19: 183.
FLOOR = 150


def measure() -> dict[str, tuple[int, set[pathlib.Path]]]:
    out: dict[str, tuple[int, set[pathlib.Path]]] = {
        name: (0, set()) for name in SPELLINGS
    }
    for directory in ("crates", "game"):
        for path in sorted(REPO.joinpath(directory).rglob("*.rs")):
            raw = path.read_text(encoding="utf-8", errors="ignore")
            if is_test_path(path) or file_is_test_only(raw):
                continue
            body = code_only(strip_test_modules(raw))
            for name, pattern in SPELLINGS.items():
                hits = len(pattern.findall(body))
                if hits:
                    count, files = out[name]
                    out[name] = (count + hits, files | {path})
    return out


def alias_total(live: dict[str, tuple[int, set[pathlib.Path]]]) -> tuple[int, int]:
    total = sum(live[name][0] for name in ALIASES)
    files: set[pathlib.Path] = set()
    for name in ALIASES:
        files |= live[name][1]
    return total, len(files)


def main() -> int:
    live = measure()
    total, total_files = alias_total(live)

    if total < FLOOR:
        print(
            f"⛔⛔ measured {total} alias parameter uses, below the floor of {FLOOR}. "
            "That is a claim about this scan, not about the tree."
        )
        return 1

    bad = []

    census_text = CENSUS.read_text(encoding="utf-8")
    census_hit = CENSUS_MARKER.search(census_text)
    if not census_hit:
        bad.append(
            f"{CENSUS.relative_to(REPO)} carries no `alias-census` marker, so the row "
            "that calls itself this population's one owner publishes nothing checkable"
        )
    else:
        stated, stated_files = int(census_hit.group(1)), int(census_hit.group(2))
        if (stated, stated_files) != (total, total_files):
            bad.append(
                f"{CENSUS.relative_to(REPO)} states {stated} uses in {stated_files} "
                f"files; source has {total} in {total_files}"
            )

    plan_text = PLAN.read_text(encoding="utf-8")
    plan_hit = PLAN_MARKER.search(plan_text)
    if not plan_hit:
        bad.append(
            f"{PLAN.relative_to(REPO)} carries no `alias-split` marker, so its "
            "per-spelling table cannot be held to the tree or to the census total"
        )
    else:
        split = {
            name: (int(count), int(files))
            for name, count, files in SPLIT_ENTRY.findall(plan_hit.group(1))
        }
        for name in SPELLINGS:
            if name not in split:
                bad.append(
                    f"{PLAN.relative_to(REPO)}'s split omits `{name}`, so that row is "
                    "prose again"
                )
                continue
            count, files = live[name]
            if split[name] != (count, len(files)):
                bad.append(
                    f"{PLAN.relative_to(REPO)} states `{name}` = {split[name][0]} in "
                    f"{split[name][1]} files; source has {count} in {len(files)}"
                )
        # ⛔⛤ A CROSS-DOCUMENT SUM CHECK WAS HERE AND IT COULD NOT FAIL, which
        # the poison run showed by never firing it alone. Both markers are
        # already pinned to the SAME live measurement, so the plan's alias rows
        # summing to anything but the census total requires one of them to
        # disagree with the tree — and that arm has already fired. Keeping it
        # would have described the single-owner property to a reader as
        # something this guard enforces separately, when it is a consequence of
        # measuring once and comparing twice.

    if bad:
        print("the alias census and the tree disagree:")
        for line in bad:
            print(f"  {line}")
        return 1

    print(
        f"ok: {total} `SessionWorldRef<`/`SessionWorldMut<` parameter uses in "
        f"{total_files} production files, agreeing with the census row and with the "
        "plan's per-spelling split"
    )
    for name in SPELLINGS:
        count, files = live[name]
        print(f"  {name}: {count} in {len(files)} file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
