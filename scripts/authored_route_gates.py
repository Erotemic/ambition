#!/usr/bin/env python3
"""Every authored use of the condition catalog: route gates AND dialogue lines.

⛔⛔ THE POINT IS THE DENOMINATOR. `capability-progression-and-world-gating.md`
records that five of seven gate families are now reachable from a route
(`world.flag_set`, `inventory.holds`, `held.is_held`, `body.can`, `body.fits`,
`world.switch_on`), and that no shipped level authors any of the new ones. That
is easy to read as "a migration is owed". It is not: the corpus of authored
route gates is TINY, and until you have counted it you cannot tell a vocabulary
that is unused from a vocabulary that has nothing to be used on.

⭐ A `LockWall` with no `gated_by` is not a defect — it belongs to the encounter
lock, which is a different writer (`contribute_encounter_lock_walls`). Both
classes are printed so the split is visible rather than assumed.

⛔⛔ TWO CONSUMERS, NOT ONE, AND THE SECOND IS THE BUSIER. This script counted
only walls on 2026-09-04 and reported a corpus of three. `ConditionCatalog` has a
second authored road: `ambition_conversation/src/dialog/authored_conditions.rs`
installs a Yarn verb `condition(id, arg)`, so every `.yarn` line calling it is
an authored use of the same vocabulary. Counting one consumer and calling the
result "the authored corpus" is the denominator error this script exists to
prevent, committed inside the instrument itself.

Usage:  python3 scripts/authored_route_gates.py
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from collections import Counter


def published_conditions() -> list[str]:
    """Every `domain.question` a crate publishes, DERIVED from the source.

    ⛔⛔ THIS WAS A HAND-KEPT LIST AND IT WAS WRONG TWICE WITHIN ONE DAY. It held
    seven ids when written on 2026-09-04 and was stale by that evening
    (`boss.cleared`, `quest.active` shipped hours later); and it listed
    `held.is_held`, **which does not exist** — `ambition_held_items` declares
    `DOMAIN = "custody"`, so the id is `custody.is_held`. ⇒ A census whose own
    vocabulary is wrong reports a condition nobody published as unauthored and
    misses the ones that are.

    Derived from the two lines every provider carries in one file:
    `pub const DOMAIN: &str = "<domain>";` and `ConditionId::new(DOMAIN, "<q>")`.

    ⚠ A provider spelling its id another way is invisible to this, so an empty
    result is REFUSED rather than reported — zero would be a finding the script
    invented about the repository.
    """
    ids: list[str] = []
    found = subprocess.run(
        # ⚠ `--untracked`, MEASURED: a provider added but not yet `git add`ed is
        # invisible to a bare `git grep`, so this silently under-reported a new
        # condition during the very session that added two. Poison-verified by
        # writing an untracked probe provider and watching the derived list NOT
        # grow.
        ["git", "grep", "-l", "--untracked", "ConditionId::new(DOMAIN,", "--", "crates", "game"],
        capture_output=True,
        text=True,
        check=False,
    ).stdout.split()
    for path in found:
        text = open(path, encoding="utf-8", errors="replace").read()
        domain = re.search(r'pub const DOMAIN: &str = "([^"]+)"', text)
        if not domain:
            continue
        for question in re.findall(r'ConditionId::new\(DOMAIN, "([^"]+)"\)', text):
            ids.append(f"{domain.group(1)}.{question}")
    if not ids:
        raise SystemExit(
            "no published conditions found: the `pub const DOMAIN` / "
            "`ConditionId::new(DOMAIN, ..)` shape this derivation reads has "
            "changed. Reporting zero here would be a finding the script invented."
        )
    return sorted(set(ids))


#: Yarn functions bound to a published condition under a different name.
#: Authored content reaches the same condition through these, so a census that
#: counts only `condition(id, arg)` under-reports the vocabulary's real use.
NAMED_ALIASES = {
    "boss_cleared": "boss.cleared",
    "quest_active": "quest.active",
}


def worlds() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "*.ldtk"], capture_output=True, text=True, check=True
    ).stdout.split()
    return sorted(out)


def main() -> int:
    rows: list[tuple[str, str, str | None, str | None]] = []
    unreadable: list[str] = []
    for path in worlds():
        try:
            data = json.load(open(path, encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            # ⛔ NOT silently skipped: this script's whole output is a COUNT, and
            # a dropped file lowers it without saying so.
            unreadable.append(f"{path}: {error}")
            continue
        for level in data.get("levels", []):
            for layer in level.get("layerInstances", []) or []:
                for entity in layer.get("entityInstances", []) or []:
                    if entity.get("__identifier") != "LockWall":
                        continue
                    fields = {
                        f["__identifier"]: f.get("__value")
                        for f in entity.get("fieldInstances", [])
                    }
                    rows.append(
                        (path, level.get("identifier", "?"), fields.get("id"), fields.get("gated_by"))
                    )

    gated = [r for r in rows if r[3]]
    ungated = [r for r in rows if not r[3]]

    print("AUTHORED ROUTE GATES\n")
    for path, level, wall_id, gate in rows:
        kind = f"gated_by={gate!r}" if gate else "no gated_by (encounter lock)"
        print(f"  {path}\n     level={level!r} id={wall_id!r} {kind}")

    print(f"\nworlds scanned: {len(worlds())}")
    print(f"LockWall instances: {len(rows)}  ({len(gated)} gated, {len(ungated)} encounter)")

    conditions = Counter(
        (g[3].split()[0] if "." in g[3].split()[0] else "world.flag_set") for g in gated
    )
    print("\nconditions actually authored:")
    for name, count in sorted(conditions.items()):
        print(f"  {count:>3}  {name}")

    # ── the second road: authored dialogue ──────────────────────────────────
    yarn = subprocess.run(
        ["git", "ls-files", "*.yarn"], capture_output=True, text=True, check=True
    ).stdout.split()
    calls: list[tuple[str, str]] = []
    for path in sorted(yarn):
        try:
            text = open(path, encoding="utf-8").read()
        except OSError as error:
            unreadable.append(f"{path}: {error}")
            continue
        for match in re.finditer(r'condition\(\s*"([^"]+)"', text):
            calls.append((path, match.group(1)))
        # ⛔⛔ THE SECOND SPELLING, AND LEAVING IT OUT INVERTS THE ANSWER.
        # A condition can be reached from authored `.yarn` two ways: the generic
        # `condition(id, arg)` verb, and a NAMED function bound to the same
        # condition for content that predates it (`boss_cleared(id)`,
        # `quest_active(id)` — see `ambition_content/src/yarn_vocabulary.rs`).
        # Counting only the generic form reported `boss.cleared` and
        # `quest.active` as "authored NOWHERE" on the very day they were
        # published *because* their authored callers existed — the exact
        # opposite of the truth, from an instrument that counted one spelling.
        # ⇒ A new alias belongs here the moment it is bound.
        for verb, condition_id in NAMED_ALIASES.items():
            for _ in re.finditer(rf"\b{verb}\(", text):
                calls.append((path, condition_id))

    print(f"\ndialogue files: {len(yarn)}  (using condition(): "
          f"{len({p for p, _ in calls})})")
    print(f"condition() calls: {len(calls)}")
    by_id = Counter(cid for _, cid in calls)
    for name, count in sorted(by_id.items()):
        print(f"  {count:>3}  {name}")

    print(
        f"\nTOTAL authored uses of the condition vocabulary: {len(gated) + len(calls)}"
        f"  ({len(gated)} route gates + {len(calls)} dialogue lines)"
    )
    published = published_conditions()
    used = set(conditions) | set(by_id)
    unused = [c for c in published if c not in used]
    if unused:
        print(f"published but authored NOWHERE ({len(unused)} of {len(published)}):")
        for name in unused:
            print(f"       {name}")

    if unreadable:
        print(
            f"\n⛔ {len(unreadable)} world(s) could not be READ, so the counts above "
            "are incomplete and describe only what answered:"
        )
        for line in unreadable:
            print(f"  {line}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
