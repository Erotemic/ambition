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
            HELD_ITEM_ICON_SPECS,
        )
    except Exception:
        return None
    return {spec.key: PROP_DIR + spec.filename for spec in HELD_ITEM_ICON_SPECS}


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
    print(
        f"ok: {len(drawn)} rendered held-item props all match the path the "
        "game points at"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
