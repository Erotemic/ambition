#!/usr/bin/env python3
"""Every gauntlet prop the GAME declares is one the RENDERER actually draws.

`game/ambition_content/src/items/held_visuals.rs` declares
`GAUNTLET_PROP_IDS`, and each id becomes a `sprites/props/gauntlet_<id>.png`
entry in the render manifest. The art itself comes from
`GAUNTLET_ICON_SPECS` in the `ambition_sprite2d_renderer` submodule. Two
lists, one fact -- the wielded-gauntlet id vocabulary -- and until this script
nothing compared them.

⭐ WHY A GUARD AND NOT ONE AUTHORITY. The elegant repair for a fact held twice
is to delete one holder, and here neither can go. The renderer's list is not a
copy of the game's: each entry carries a DRAWING FUNCTION, so the ids are only
its key set. The game's list cannot be derived from the renderer either,
without a codegen step across a language and a submodule boundary. A key set
shared by a Rust const and a Python module cannot be a type in either, so it
gets the fallback.

⛔ THE ART IS NOT EVIDENCE. Zero `gauntlet_*.png` files are tracked -- the
sprites are generated and gitignored -- so a check that the PNGs exist answers
a question about the machine it runs on, not about the tree. Both lists are
SOURCE and both are in the tree, which is why this compares declarations.

⚠ ASYMMETRIC ON PURPOSE, and the asymmetry is the interesting half:

  * game declares an id the renderer does not draw  -> FAIL. The manifest
    points at a PNG nothing will ever write, so the gauntlet shows a
    placeholder in a shipped build and no test says why.
  * renderer draws an id the game does not declare  -> reported, not failed.
    The renderer is a separately-pinned submodule; art legitimately lands
    there before the game wires it, and reddening the superproject for that
    would punish the normal authoring order. The cost of the loose direction
    is a wasted render, which is why it is only worth a line of output.

Exit 0 agreement, 1 a declared prop with no drawing, 3 the renderer is not
importable (an ordinary state on a checkout without the submodule, and a
DIFFERENT fact from "the lists disagree" -- see
`scripts/lib/sprite_install_names.py` for the check that used to exit 0 here
and so was off on every machine that had not run a setup step)."""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
HELD_VISUALS = REPO_ROOT / "game/ambition_content/src/items/held_visuals.rs"
RENDERER_ROOT = REPO_ROOT / "tools" / "ambition_sprite2d_renderer"


def declared_ids() -> list[str]:
    """The ids `GAUNTLET_PROP_IDS` names, in source order.

    Raises when the const is absent or empty. A parser that silently returns
    `[]` because a refactor renamed the const would make this whole check pass
    over an empty corpus, which is the way a guard stops being one.
    """
    text = HELD_VISUALS.read_text(encoding="utf-8")
    match = re.search(
        r"const GAUNTLET_PROP_IDS:\s*&\[&str\]\s*=\s*&\[(.*?)\];", text, re.S
    )
    if match is None:
        raise SystemExit(
            f"{HELD_VISUALS.relative_to(REPO_ROOT)}: GAUNTLET_PROP_IDS not found. "
            "If it was renamed or moved, update this check -- do not delete it: "
            "the manifest entries it guards are still there."
        )
    ids = re.findall(r'"([a-z0-9_]+)"', match.group(1))
    if not ids:
        raise SystemExit("GAUNTLET_PROP_IDS parsed as EMPTY; the check would be vacuous")
    return ids


def drawn_ids() -> list[str] | None:
    """The ids the renderer declares specs for, or `None` if it is unreachable.

    ⭐ Asks the renderer by importing it rather than parsing its source: the
    spec list is Python, and a regex over it would be a second model of a thing
    that can simply be read.
    """
    if RENDERER_ROOT.is_dir() and str(RENDERER_ROOT) not in sys.path:
        sys.path.append(str(RENDERER_ROOT))
    try:
        from ambition_sprite2d_renderer.targets.icons.item_icons import (
            GAUNTLET_ICON_SPECS,
        )
    except Exception:
        return None
    ids = []
    for spec in GAUNTLET_ICON_SPECS:
        stem = Path(spec.filename).stem
        ids.append(stem[len("gauntlet_"):] if stem.startswith("gauntlet_") else stem)
    return ids


def main() -> int:
    declared = declared_ids()
    drawn = drawn_ids()
    if drawn is None:
        print(
            "cannot check: the ambition_sprite2d_renderer package is not importable.\n"
            "  This is NOT a pass -- the gauntlet declarations went unexamined.\n"
            "  fix: git submodule update --init tools/ambition_sprite2d_renderer",
            file=sys.stderr,
        )
        return 3
    if not drawn:
        print("GAUNTLET_ICON_SPECS is EMPTY; the check would be vacuous", file=sys.stderr)
        return 1

    undrawn = [i for i in declared if i not in drawn]
    unused = [i for i in drawn if i not in declared]

    if unused:
        print(
            "note: the renderer draws "
            + ", ".join(unused)
            + (" and the game declares no prop for it" if len(unused) == 1
               else " and the game declares no props for them")
            + " (art ahead of wiring, or a drawing left behind by a removed prop)."
        )
    if undrawn:
        print(
            "FAIL: "
            + ", ".join(undrawn)
            + " -- declared in GAUNTLET_PROP_IDS with no GAUNTLET_ICON_SPECS entry.\n"
            "  The render manifest points at sprites/props/gauntlet_<id>.png and "
            "nothing will ever write it, so the gauntlet draws as a placeholder.\n"
            "  fix: add the spec in the renderer, or drop the id here.",
            file=sys.stderr,
        )
        return 1
    print(f"ok: {len(declared)} declared gauntlet props all have a drawing")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
