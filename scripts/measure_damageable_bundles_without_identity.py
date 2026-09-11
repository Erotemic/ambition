#!/usr/bin/env python3
"""Damageable bodies assembled in ONE bundle, and whether that bundle names a `SimId`.

⭐⭐ **THE QUESTION IS NOT "WHO FORGOT AN IDENTITY" — IT IS "WHO IS BUILT OUTSIDE
THE CONSTRUCTION TRANSACTION".** Measured 2026-09-10:
`ambition_platformer2d_shared_tangle::construction`'s `commit_entity` spawns the
root and inserts `planned.sim_id`, the origin and the transaction stamp BEFORE
the recipe runs, and says why in its own comment — *"so a recipe cannot forget
them"*. A body built through that road therefore CANNOT lack identity. The
population that can is the bodies assembled somewhere else, and those are exactly
the ones a bundle-shaped scan can see.

⛔⛔ **THIS IS A LOWER BOUND AND ITS BOUND IS THE WHOLE POINT.** A body that
receives `CenteredAabb` from one insert and `ActorFaction` from another is
invisible here — an earlier version of this scan reported "2 sites out of 2" and
I deleted it for describing its own method. That was an over-correction: it was a
severe undercount, but BOTH of its rows were real leads and one
(`bosses/cut_rope/victory.rs`) is a damageable NPC with no `SimId` anywhere in
its file. ⇒ A lower bound that names real rows is worth keeping WITH ITS BOUND
STATED. It is not a population and this output never calls it one.

⚠ **A ROW HERE IS A LEAD, NOT A VERDICT.** Two things make one benign:
  * the site is a CONSTRUCTION RECIPE spawning into an already-identified root
    (`spawn_*_into(.., root.entity(), ..)`) — identity is already on the entity;
  * the identity arrives in a later `insert` on the same entity.
Both need reading. The crate column is the first filter: `..._actor_spawn` and
`..._actor_monolith/src/construction/` host recipes.

    python3 scripts/measure_damageable_bundles_without_identity.py
    python3 scripts/measure_damageable_bundles_without_identity.py --all
"""

from __future__ import annotations

import argparse
import collections
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
ROOTS = ("crates", "game", "tools")

# What makes an entity a candidate victim for the strike resolver: `StrikeVictim`
# matches on `CenteredAabb` + `ActorFaction`. Bundles that CARRY those count too,
# or the scan sees only the hand-written half of the tree.
DAMAGEABLE = (
    "CenteredAabb",
    "EnemyActorBundle",
    "FeatureRenderedBundle",
    "FeatureLifecycleBundle",
    "PlayerSimulationBundle",
    "PlayerIdentityBundle",
)
FACTION = ("ActorFaction", "EnemyActorBundle", "PlayerIdentityBundle")
IDENTITY = ("SimId",)
# ⛔⛔ A SITE THAT ASSEMBLES ONE OF THESE IS NOT A LEAD, AND CALLING IT ONE
# DISPATCHES AN AGENT AT A NON-DEFECT. `ambition_platformer2d_runtime`'s
# `ensure_sim_id` runs at the head of every simulation host and gives any body
# with `BodyKinematics` and no `SimId` an identity DERIVED from an authored
# `FeatureId` (`SimId::placement`) or from `PrimaryPlayer` (`SimId::player_slot`).
# A spawn that names a feature id has therefore already answered the question
# this script asks — later, by a derive, but before anything reads identity.
#
# ⇒ MEASURED 2026-09-11: every "lead" this script reported was one of these.
# `match_activation.rs:90` builds a `FeatureRenderedBundle` from the SEAT's
# feature id, and a live-match census
# (`ambition_demo_smash_app/tests/every_fighter_in_a_match_carries_identity.rs`)
# found 2 of 2 seated fighters identified.
DERIVABLE = ("FeatureId", "FeatureRenderedBundle", "PrimaryPlayer")

SPAWN = re.compile(
    r"\.(?:spawn|spawn_batch|spawn_empty|insert|insert_if_new|try_insert)"
    r"|\bspawn_(?:room_scoped|session_scoped|room_in_session)\s*\("
)

RECIPE_HOSTS = ("ambition_platformer2d_actor_spawn",)
RECIPE_PATHS = ("/src/construction/",)


def blank(span: str) -> str:
    """Same span, same height, no content — so line numbers survive stripping."""
    return "\n" * span.count("\n")


def strip_test_mods(text: str) -> str:
    out, i = [], 0
    pattern = re.compile(r"#\[cfg\(test\)\]\s*(?:(?:///?[^\n]*|//![^\n]*|#\[[^\]]*\])\s*)*(?:pub(?:\(crate\))?\s+)?mod\s+\w+\s*\{")
    while True:
        match = pattern.search(text, i)
        if not match:
            out.append(text[i:])
            break
        out.append(text[i : match.start()])
        j, depth = match.end() - 1, 0
        while j < len(text):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        out.append(blank(text[match.start() : j + 1]))
        i = j + 1
    return "".join(out)


def is_test_file(path: pathlib.Path, text: str) -> bool:
    if "tests" in path.name or "/tests/" in str(path):
        return True
    return bool(re.search(r"^\s*#!\[\s*cfg\s*\(\s*test\s*\)\s*\]", text, re.MULTILINE))


def call_bodies(text: str) -> list[tuple[int, str]]:
    """Every spawn/insert argument list, with the line its call starts on."""
    found = []
    for match in SPAWN.finditer(text):
        i = text.find("(", match.end() - 1)
        if i == -1:
            continue
        depth, j = 0, i
        while j < len(text):
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        found.append((text.count("\n", 0, match.start()) + 1, text[i : j + 1]))
    return found


def crate_of(path: pathlib.Path) -> str:
    rel = path.relative_to(REPO)
    return rel.parts[1] if len(rel.parts) > 1 else "?"


def classify() -> dict[str, list]:
    """Every damageable-bundle assembly site in the tree, split by identity.

    ⛔⛔ **ONE KEEPER FOR "WHAT IS A LEAD".** `scripts/tests/` re-implemented this
    walk so it could reach the same four lists, and the copy then did not learn
    about `DERIVABLE` when this module did — a ratchet guarding a classification
    its own subject had stopped using. The ratchet reads this function now.

    Keys: `matched` (every site the scan sees at all — the FLOOR's population),
    `identified`, `derivable`, `leads`, `recipes`.
    """
    out: dict[str, list] = {
        "matched": [], "identified": [], "derivable": [], "leads": [], "recipes": []
    }
    for root in ROOTS:
        base = REPO / root
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            raw = path.read_text(encoding="utf-8", errors="ignore")
            if is_test_file(path, raw):
                continue
            if not any(token in raw for token in DAMAGEABLE):
                continue
            body = strip_test_mods(raw)
            where = str(path.relative_to(REPO))
            crate = crate_of(path)
            recipe_host = crate in RECIPE_HOSTS or any(p in where for p in RECIPE_PATHS)
            for line, args_text in call_bodies(body):
                if not any(t in args_text for t in DAMAGEABLE):
                    continue
                if not any(t in args_text for t in FACTION):
                    continue
                row = (crate, where, line)
                out["matched"].append(row)
                if any(t in args_text for t in IDENTITY):
                    out["identified"].append(row)
                elif any(t in args_text for t in DERIVABLE):
                    out["derivable"].append(row)
                elif recipe_host:
                    out["recipes"].append(row)
                else:
                    out["leads"].append(row)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all", action="store_true", help="also list identified bundles")
    args = parser.parse_args()

    rows = classify()
    identified = rows["identified"]
    derivable = rows["derivable"]
    leads = rows["leads"]
    recipes = rows["recipes"]

    print("⛔ DAMAGEABLE BUNDLES AND THEIR IDENTITY — a LOWER BOUND, never a population.\n")
    print(f"-- assembled WITH a `SimId` in the same call: {len(identified)}")
    if args.all:
        for crate, where, line in identified:
            print(f"   {where}:{line}")
    print(f"\n-- named later by `ensure_sim_id` from an authored fact: {len(derivable)}")
    if args.all:
        for crate, where, line in derivable:
            print(f"   {where}:{line}")
    print(f"\n-- assembled with NO `SimId` and NOTHING TO DERIVE ONE FROM: "
          f"{len(leads) + len(recipes)}")
    print(f"   {len(leads)} outside a construction-recipe host  ⛔ THESE ARE THE LEADS")
    for crate, where, line in leads:
        print(f"      {where}:{line}   ({crate})")
    print(f"   {len(recipes)} inside one — identity is already on the root they build into")
    for crate, where, line in recipes:
        print(f"      · {where}:{line}")

    by_crate = collections.Counter(crate for crate, _, _ in leads)
    if by_crate:
        print("\n   leads by crate:")
        for crate, n in sorted(by_crate.items(), key=lambda kv: (-kv[1], kv[0])):
            print(f"      {crate:44s} {n}")

    print("\n⛔ WHAT THIS CANNOT SEE")
    print("   * A BODY ASSEMBLED ACROSS SEVERAL INSERTS. `CenteredAabb` from one and")
    print("     `ActorFaction` from another is invisible; this scan reads ONE call.")
    print("   * IDENTITY ARRIVING LATER on the same entity, in a follow-up `insert`.")
    print("   * A bundle TYPE that carries a faction or an aabb and is not listed in")
    print("     DAMAGEABLE / FACTION above.")
    print("   ⇒ The completeness check is a RUNTIME census over `StrikeVictim`'s own")
    print("     query. This names sites to read; it does not bound the population.")
    print("\n⭐ AND THE RUNTIME GUARD IS THE ONE THAT CLOSES IT. `ensure_sim_id`")
    print("   refuses (debug_assert + error!) when it declines a body that carries")
    print("   BOTH `CenteredAabb` and `ActorFaction` — `StrikeVictim`'s own query —")
    print("   so a damageable body nothing can name is a construction failure a test")
    print("   sees, rather than a silent `continue` that becomes the real rule.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
