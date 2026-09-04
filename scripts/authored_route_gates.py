#!/usr/bin/env python3
"""Every authored use of the condition catalog: route gates AND dialogue lines.

⛔⛔ THE POINT IS THE DENOMINATOR, so THIS HEADER STATES NO COUNTS. The reading
this script exists to refuse is: "the engine publishes gate families that no
shipped level authors, therefore a migration is owed". It is not owed — the
corpus of authored route gates is TINY, and until you have counted it you cannot
tell a vocabulary that is unused from a vocabulary that has nothing to be used
on. ⇒ RUN IT; the numbers live in the output, which derives them.

⛔⛔ AND THE ABSENCE OF NUMBERS HERE IS THE FIX, NOT AN OMISSION. This docstring
used to open with "five of seven gate families" and name them by hand. Both
halves rotted within a day: two more conditions were published (making the
denominator nine), and one of the hand-written ids -- `held.is_held` -- named
no published condition at all. A tool whose whole purpose is to stop a stale
denominator claim had one in its own operational header, and a reader who
stopped at the docstring got the wrong figure from the instrument that exists
to correct it. A description that can go stale independently of the code is
a second authority; delete it rather than synchronise it.

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
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


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


VOCABULARY = "game/ambition_content/src/yarn_vocabulary.rs"


def named_aliases() -> dict[str, str]:
    """Yarn functions bound to a published condition under a different name.

    Authored content reaches the same condition through these, so a census that
    counts only `condition(id, arg)` under-reports the vocabulary's real use.

    ⛔⛔ THIS WAS A HAND-KEPT MAP AND IT WENT STALE THREE TIMES IN ONE DAY, the
    third time within hours of the second correction being written. Each time
    the failure was the same and pointed the same way: a condition with authored
    callers was reported as *"authored NOWHERE"* on the day it was published,
    **precisely because those callers existed under the alias.**

      2026-09-04 morning   `boss_cleared`, `quest_active` bound; map listed neither
      2026-09-04 midday    map corrected by hand; `can_afford` bound hours later
      2026-09-04 evening   `wallet.can_afford` reported unauthored with TEN callers,
                           more than any published condition had

    ⇒ Derived from the source instead. The binding is a three-line shape in
    `yarn_vocabulary.rs` — `register_system(ask_x)`, `add_function("name", var)`,
    and a `ConditionId::parse("domain.question")` inside `ask_x` — so the map is
    read off the code that creates it and cannot lag the code by construction.
    ⚠ A map this census keeps by hand is the same defect as the vocabulary it
    was already deriving; see `published_conditions`.
    """
    text = (REPO / VOCABULARY).read_text(encoding="utf-8")
    # `let <var> = commands.register_system(<fn>);`
    systems = dict(
        re.findall(r"let\s+(\w+)\s*=\s*commands\.register_system\((\w+)\)", text)
    )
    # `.add_function("<yarn name>", <var>)`
    bound = dict(re.findall(r'add_function\(\s*"(\w+)"\s*,\s*(\w+)\)', text))
    aliases: dict[str, str] = {}
    for yarn_name, var in bound.items():
        fn = systems.get(var)
        if not fn:
            continue  # a closure over the mirror, not a catalog question
        body = text.split(f"fn {fn}(", 1)
        if len(body) < 2:
            continue
        asked = re.search(r'ConditionId::parse\(\s*"([a-z_]+\.[a-z_]+)"', body[1])
        if asked:
            aliases[yarn_name] = asked.group(1)
    if not aliases:
        raise SystemExit(
            f"derived NO named aliases from {VOCABULARY}. Either the binding shape "
            "changed or the file moved — and an empty map silently under-reports "
            "every aliased condition as unauthored, which is the defect this "
            "derivation exists to stop."
        )
    return aliases


def worlds() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "*.ldtk"], capture_output=True, text=True, check=True
    ).stdout.split()
    return sorted(out)


def main() -> int:
    aliases = named_aliases()
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
            calls.append((path, match.group(1), "generic"))
        # ⛔⛔ THE SECOND SPELLING, AND LEAVING IT OUT INVERTS THE ANSWER.
        # A condition can be reached from authored `.yarn` two ways: the generic
        # `condition(id, arg)` verb, and a NAMED function bound to the same
        # condition for content that predates it (`boss_cleared(id)`,
        # `quest_active(id)` — see `ambition_content/src/yarn_vocabulary.rs`).
        # Counting only the generic form reported `boss.cleared` and
        # `quest.active` as "authored NOWHERE" on the very day they were
        # published *because* their authored callers existed — the exact
        # opposite of the truth, from an instrument that counted one spelling.
        # ⇒ Derived from `yarn_vocabulary.rs` rather than kept here; see
        #   `named_aliases`, and the three staleness dates in its docstring.
        for verb, condition_id in aliases.items():
            for _ in re.finditer(rf"\b{verb}\(", text):
                calls.append((path, condition_id, "alias"))

    # ⛔ NAME THE TWO ROADS SEPARATELY. This printed `condition() calls: N` over
    # the COMBINED population, which was the right denominator under a label that
    # was false for two thirds of it — 10 generic calls and 18 alias calls read
    # as 28 `condition(...)` calls, in the one tool whose whole purpose is to
    # stop a denominator being quoted wrong. The combined figure is what the
    # published/unused census needs; the label just has to say so.
    generic = [c for c in calls if c[2] == "generic"]
    alias = [c for c in calls if c[2] == "alias"]
    print(f"\ndialogue files: {len(yarn)}  (using condition vocabulary: "
          f"{len({p for p, _, _ in calls})})")
    print(f"dialogue condition uses: {len(calls)}")
    print(f"  generic condition(): {len(generic):>3}"
          f"   in {len({p for p, _, _ in generic})} file(s)")
    print(f"  named aliases:       {len(alias):>3}"
          f"   in {len({p for p, _, _ in alias})} file(s)")
    by_id = Counter(cid for _, cid, _ in calls)
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
