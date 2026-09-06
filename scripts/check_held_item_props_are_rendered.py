#!/usr/bin/env python3
"""Every held-item prop the RENDERER draws is the one the GAME points at.

The sibling of `check_gauntlet_props_are_rendered.py`, one shelf over and for
the same reason. `game/ambition_content/src/items/held_visuals.rs` registers
`HeldItemArtEntry::new(<id>, <path>, <extent>)`; the drawings for the ids the
Projectile Polygon authored come from `HELD_ITEM_ICON_SPECS` in the
`ambition_sprite2d_renderer` submodule, whose `key` is that same id and whose
`filename` lands in `sprites/props/`. Two lists, one fact -- which picture a
held item wears -- and until this script nothing compared them.

⛔ THE ART IS NOT EVIDENCE, same as next door: zero `props/*.png` are tracked
(the sprites are generated and gitignored), so a check that the PNG exists
answers a question about the machine it runs on. Both lists are SOURCE.

⚠ THE HISTORY THIS GUARDS, because it is why the loose direction is loose.
Until 2026-09-06 all three of her items wore BORROWED art: the bomb wore
`gauntlet_bomb.png`, the mine wore `mark_beacon.png`, the ponytail wore
`javelin.png` -- a thrown stick standing in for a tress, which the registry's
own comment called "honestly a placeholder". That state was deliberate and
correct while no drawing existed: a wrong picture the player can see beats a
missing one only the log explains. So:

  * renderer draws an id the game does not register        -> reported, not
    failed. Art legitimately lands in the submodule before the game wires it,
    and reddening the superproject for that punishes the normal order.
  * game registers an id the renderer draws, at a DIFFERENT path -> FAIL. The
    drawing is wasted and the item wears the wrong picture in a shipped build.
    This is the one a rename causes, in either repo, silently.
  * game points at `sprites/props/<id>.png` with NO spec for that id -> FAIL,
    but ONLY for an id in a family this list owns. Nothing will ever write that
    file, so the item draws as an ERROR per spawn ("unknown held item art")
    and a placeholder quad. A deliberate BORROW is not this: it names another
    item's art, not a file named after itself.

⛔ AND THE FAMILY RESTRICTION IS NOT CAUTION, IT IS A MEASUREMENT. The first
draft failed that clause for any self-named path, and against the live tree it
immediately fired on `axe` and `javelin` -- both correct, both drawn by a
DIFFERENT renderer target. "Self-named art must come from THIS list" assumed
this list is the only producer of props named after their item, and it is not.
The family (the id's first `_`-segment, so `polygon` here) is derived from the
spec keys rather than written down, so the population grows with the list and
never with a hardcoded prefix. If every spec is deleted the families vanish
with them -- and the empty-list check above fires instead, which is the case
that would otherwise go quiet.

Exit 0 agreement, 1 a disagreement, 3 the renderer is not importable (an
ordinary state on a checkout without the submodule, and a DIFFERENT fact from
"the lists disagree")."""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
HELD_VISUALS = REPO_ROOT / "game/ambition_content/src/items/held_visuals.rs"
RENDERER_ROOT = REPO_ROOT / "tools" / "ambition_sprite2d_renderer"
PROP_DIR = "sprites/props/"


def registered_art() -> dict[str, str]:
    """`id -> art path` for every `HeldItemArtEntry::new` literal in the registry.

    Raises when the file parses to nothing: a refactor that renames the
    constructor would otherwise make this check pass over an empty corpus,
    which is the way a guard stops being one.
    """
    text = HELD_VISUALS.read_text(encoding="utf-8")
    # Comments first: this file argues with itself at length, and the prose
    # quotes both ids and paths. A scan that reads them compares the guard
    # against its own documentation.
    text = re.sub(r"//[^\n]*", "", text)
    pairs = re.findall(
        r'HeldItemArtEntry::new\(\s*"([^"]+)"\s*,\s*"([^"]+)"', text, re.S
    )
    if not pairs:
        raise SystemExit(
            f"{HELD_VISUALS.relative_to(REPO_ROOT)}: no HeldItemArtEntry::new "
            "literals found. If the constructor was renamed, update this check "
            "-- do not delete it: the registrations it guards are still there."
        )
    return dict(pairs)


def drawn_art() -> dict[str, str] | None:
    """`key -> art path` for every held-item spec, or `None` if unreachable.

    Asks the renderer by importing it rather than parsing its source, for the
    reason the gauntlet check gives: the spec list is Python, and a regex over
    it would be a second model of a thing that can simply be read.
    """
    if RENDERER_ROOT.is_dir() and str(RENDERER_ROOT) not in sys.path:
        sys.path.append(str(RENDERER_ROOT))
    try:
        from ambition_sprite2d_renderer.targets.icons.item_icons import (
            GAUNTLET_ICON_SPECS,
            HELD_ITEM_ICON_SPECS,
        )
    except ImportError as error:
        # ⛔⛔ A BARE `except Exception` HERE REPORTED ITS OWN BLINDNESS AS A FACT
        # ABOUT THE MACHINE, and this check sat SKIPPED for as long as that was
        # true: an ImportError on a NAME came back as `None`, and six tests
        # skipped with "renderer submodule not importable" — which was FALSE.
        # The submodule was right there.
        #
        # ⚠ THE DIAGNOSIS THAT CAME WITH THAT FIX WAS WRONG, and it is worth
        # keeping the correction next to the fix. It read the ImportError as
        # "the renderer consolidated `HELD_ITEM_ICON_SPECS` into `ICON_SPECS`"
        # and repointed this check at the gauntlet list alone. The submodule had
        # done no such thing: the list exists on the renderer's `origin/main` at
        # the exact commit this superproject pins, and the ImportError was a
        # STALE WORKING COPY of the submodule — which is the normal state here,
        # because `git submodule update` is forbidden in this repo. ⇒ Repointing
        # left the check comparing ZERO pairs while printing `ok`.
        #
        # ⇒ Distinguish the two, because they want opposite responses: a missing
        # MODULE is a machine that cannot check, and a missing NAME is exactly
        # the renderer drift this file exists to catch.
        if "ambition_sprite2d_renderer" in str(error) and "cannot import name" not in str(error):
            return None
        raise SystemExit(
            "the renderer is importable but does not export what this check "
            f"reads: {error}\n"
            "  That is DRIFT, not an unavailable machine. Repoint this check at "
            "the renderer's current spec list rather than letting it skip."
        )
    # ⭐⭐ BOTH LISTS, BECAUSE THE RENDERER DRAWS HELD ITEMS FROM BOTH AND EITHER
    # ONE ALONE MAKES THIS CHECK VACUOUS.
    #
    #   * `HELD_ITEM_ICON_SPECS` — the three the Projectile Polygon authored.
    #     These are the ones the game REGISTERS, so they are the pairs actually
    #     compared. Reading the gauntlet list alone compared nothing at all.
    #   * the `held_item` half of `GAUNTLET_ICON_SPECS` (7 of its 14) — drawn,
    #     and not registered as held items today, so they land in the "art ahead
    #     of wiring" note. Keeping them means the day one IS wired, this check
    #     already holds it.
    #
    # ⚠ NOT `ICON_SPECS`, which holds the 20 movement/combat/utility ability
    # tiles and no held items at all: filtering it produces an empty map.
    return {
        spec.key: PROP_DIR + spec.filename
        for spec in (
            *HELD_ITEM_ICON_SPECS,
            *(s for s in GAUNTLET_ICON_SPECS if s.category == "held_item"),
        )
    }


def main() -> int:
    registered = registered_art()
    drawn = drawn_art()
    if drawn is None:
        print(
            "cannot check: the ambition_sprite2d_renderer package is not "
            "importable.\n  This is NOT a pass -- the held-item art went "
            "unexamined.\n  fix: git submodule update --init "
            "tools/ambition_sprite2d_renderer",
            file=sys.stderr,
        )
        return 3
    if not drawn:
        print(
            "HELD_ITEM_ICON_SPECS is EMPTY; the check would be vacuous",
            file=sys.stderr,
        )
        return 1

    mismatched = [
        (key, registered[key], path)
        for key, path in drawn.items()
        if key in registered and registered[key] != path
    ]
    # A registration naming a file after ITSELF is claiming a drawing exists.
    # Borrowing another item's art names that item, so it never lands here.
    # Scoped to the families this spec list owns -- see the module docstring for
    # the `axe`/`javelin` measurement that put the restriction there.
    families = {key.split("_", 1)[0] for key in drawn}
    orphaned = [
        (key, path)
        for key, path in registered.items()
        if key not in drawn
        and key.split("_", 1)[0] in families
        and path == f"{PROP_DIR}{key}.png"
    ]
    unwired = [key for key in drawn if key not in registered]

    if unwired:
        print(
            "note: the renderer draws "
            + ", ".join(sorted(unwired))
            + " and the game registers no held item for it (art ahead of "
            "wiring, or a drawing left behind by a removed item)."
        )
    if mismatched:
        for key, got, want in sorted(mismatched):
            print(
                f"FAIL: held item {key!r} wears {got!r} but the renderer draws "
                f"it at {want!r}.\n"
                "  The drawing is never loaded and the item shows the wrong "
                "picture in a shipped build.\n"
                "  fix: point the HeldItemArtEntry at the rendered path, or "
                "rename the spec's filename to match a deliberate borrow.",
                file=sys.stderr,
            )
    if orphaned:
        for key, path in sorted(orphaned):
            print(
                f"FAIL: held item {key!r} points at {path!r} and no "
                "HELD_ITEM_ICON_SPECS entry draws it.\n"
                "  Nothing will ever write that file, so every spawn logs "
                "'unknown held item art' and draws a placeholder.\n"
                "  fix: add the spec in the renderer, or borrow art that "
                "exists (a path named after ANOTHER item is a borrow; one "
                "named after this id is a claim that a drawing exists).",
                file=sys.stderr,
            )
    if mismatched or orphaned:
        return 1
    # ⛔⛔ THE SUCCESS LINE USED TO NAME `len(drawn)`, WHICH IS NOT WHAT IT
    # COMPARED. `mismatched` is built from `key in registered`, so the population
    # this check actually verifies is the INTERSECTION -- and today that is
    # EMPTY: the renderer draws `bomb`/`grapple`/..., the game registers
    # `polygon_bomb`/`polygon_mine`/..., and the two vocabularies do not meet at
    # a single key. "ok: 7 ... all match" was a true-looking sentence about
    # SEVEN comparisons that never happened.
    compared = sorted(set(drawn) & set(registered))
    # ⛔⛔ A FAMILY WITH NO SPECS CAN NEVER BE IN `families`, so the `orphaned`
    # clause is structurally unable to fire for the FIRST id of a new family. The
    # restriction is derived from the DRAWN keys (the `axe`/`javelin` measurement
    # in the docstring) -- right for a family this list partly covers, blind for
    # one it does not cover at all.
    # ⚠ LOUD, NOT RED. `polygon_bomb`/`polygon_mine`/`polygon_ponytail` sit in
    # exactly that position today and it is DELIBERATE: they were moved off
    # borrowed art onto self-named paths on 2026-09-06 with the drawings still to
    # come. Reddening for that "punishes the normal order", which is this file's
    # own stated rule for the other direction.
    unclassifiable = sorted(
        key
        for key, path in registered.items()
        if key not in drawn
        and path == f"{PROP_DIR}{key}.png"
        and key.split("_", 1)[0] not in families
    )
    if unclassifiable:
        print(
            "note: "
            + ", ".join(unclassifiable)
            + " point at self-named art in a family this spec list does not "
            "cover at all,\n  so the orphan rule cannot judge them either way "
            "-- they are OUTSIDE what this check\n  can see, not verified by it. "
            "⚠ Two DIFFERENT states look identical from here: `axe` and "
            "`javelin`\n  are drawn by another renderer target and are fine, "
            "while an id whose art nothing\n  draws logs 'failed to load' every "
            "spawn. This check cannot tell them apart."
        )
    # ⛔⛔ AN EMPTY INTERSECTION IS NOT A PASS, and this arm exists because the
    # check printed `ok` while comparing ZERO pairs for the length of one commit.
    # Both halves were individually non-empty — 7 drawn, 8 registered — so every
    # anti-vacuity check above was satisfied; what was empty was the OVERLAP,
    # which is the only thing this file actually verifies. ⇒ A guard whose
    # population is an intersection has to floor the INTERSECTION.
    if not compared:
        print(
            f"NOTHING WAS COMPARED: {len(drawn)} drawn and {len(registered)} "
            "registered, and not one id appears in both.\n"
            "  Every check above passed on that, which is what makes this the "
            "dangerous state rather than a quiet one.\n"
            "  fix: the two sides have drifted apart entirely — check that this "
            "script reads the spec list the renderer actually defines, and that "
            "the submodule working copy is not stale (`git submodule update` is "
            "forbidden here, so a stale checkout is the normal way this happens).",
            file=sys.stderr,
        )
        return 1
    print(
        f"ok: {len(compared)} held-item prop(s) compared "
        f"({len(drawn)} drawn, {len(registered)} registered); every compared "
        "pair matches the path the game points at"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
