#!/usr/bin/env python3
"""A drawable that draws a BODY must say whose body, in the one shared spelling.

⛔⛔ THE DEFECT THIS EXISTS FOR. "Which body does this drawable belong to" was
spelled THREE ways in `ambition_render` and omitted once: `FlylineVisual { body }`,
`TrapdoorVisual { body }` and `TetherVisual { body }` said `body`;
`HitFlashOverlay { source }` said `source`; `SlashVisual { owner }` said `owner`;
and `MorphBallVisual` recorded no owner at all.

⇒ A consumer could not ask *"what else draws this body?"* because there was no
question to ask. Portal composition therefore saw only the base
`FeatureVisual`/`PlayerVisual` sprite: a far-side character could have its base
art correctly clipped while its hit-flash silhouette drew whole over the pane,
and a MORPHED player bypassed composition entirely.

⭐ `PresentationOf(Entity)` is now that question, and this guard is the fallback
where a type cannot enforce it -- a Bevy spawn is a tuple, so nothing makes a new
overlay declare an owner. The census below is the population as measured; a NEW
body-naming component must be classified, which is the moment to decide whether
it needs the seam.

⚠ IT ASKS A NARROW QUESTION AND SAYS SO. It matches components holding an
`Entity` field named `body`, `owner` or `source` -- the three spellings the tree
actually used. A fourth spelling would slip past, which is exactly why the seam
matters more than this check: the guard catches the population growing, the
COMPONENT is what makes the answer askable.
"""
from __future__ import annotations

import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
SEAM = "PresentationOf"
FIELD = re.compile(r"^\s*(?:pub )?(body|owner|source): Entity,")

#: Components that name a body, and what each is. A drawable that DRAWS the body
#: needs the seam; one that merely references it does not, and says why here.
CENSUS = {
    "FlylineVisual": "draws a wire holding the body up — has the seam",
    "TrapdoorVisual": "draws the door the submerged body went through — has the seam",
    "TetherVisual": "draws the line from the reaching body — has the seam",
    "HitFlashOverlay": "the body's own silhouette, a sibling mesh — has the seam",
    "SlashVisual": "the swing's art, placed in the owner's frame — has the seam",
    "PortalCaptureParallaxLayerVisual": (
        "NOT a body: its `source` is a portal rig's parallax layer. Excluded "
        "after reading it, not after matching the field name."
    ),
}


def components_naming_a_body() -> dict[str, str]:
    """component name -> the file that defines it."""
    files = subprocess.run(
        ["git", "ls-files", "crates/ambition_render/src"],
        cwd=REPO, capture_output=True, text=True, check=True,
    ).stdout.split()
    found: dict[str, str] = {}
    for rel in files:
        if not rel.endswith(".rs") or rel.endswith("tests.rs") or "/tests/" in rel:
            continue
        current = None
        for line in (REPO / rel).read_text(encoding="utf-8", errors="replace").splitlines():
            # ⚠ `pub(crate) struct` and `pub(super) struct` are structs too. The
            # first cut matched only `pub struct` and silently missed
            # `SlashVisual`, which this guard then reported as a STALE census
            # entry -- a scanner blind spot wearing the costume of a stale row.
            struct = re.match(r"^(?:pub(?:\([\w:]+\))? )?struct (\w+)", line.strip())
            if struct:
                current = struct.group(1)
                continue
            if current and FIELD.match(line):
                found[current] = rel
                current = None
            elif line.startswith("}") or line.lstrip().startswith("fn ") or line.lstrip().startswith("pub fn "):
                # ⛔ A FUNCTION PARAMETER IS NOT A FIELD. `slash_visuals.rs` has
                # `fn spawn_one(.., owner: Entity, ..)`, which the first cut
                # counted as a second body-naming component -- and I had already
                # repeated that over-count in a message before this guard existed.
                current = None
    return found


def main() -> int:
    found = components_naming_a_body()
    if not found:
        print(
            "FAIL: no component in `ambition_render` names a body at all — the "
            "pattern this guard\n  scans for has changed, so every claim below is "
            "vacuous.", file=sys.stderr,
        )
        return 1

    unknown = sorted(set(found) - set(CENSUS))
    if unknown:
        print(
            f"FAIL: {len(unknown)} component(s) name a body and are not "
            "classified:", file=sys.stderr,
        )
        for name in unknown:
            print(f"  {name}  ({found[name]})", file=sys.stderr)
        print(
            f"\n  ⛔ ANSWER ONE QUESTION: does this component DRAW the body?\n"
            f"  * YES — spawn it with `{SEAM}(body)` too, so a consumer that "
            "knows nothing about it\n    can still ask whose body it draws. "
            "Portal composition is one such consumer, and a\n    drawable it "
            "cannot see is one that draws over a pane it should be clipped by.\n"
            "  * NO — add it to CENSUS with the reason, one line.",
            file=sys.stderr,
        )
        return 1

    stale = sorted(set(CENSUS) - set(found))
    if stale:
        print(
            f"FAIL: {len(stale)} census entr(y/ies) no longer name a body:",
            file=sys.stderr,
        )
        for name in stale:
            print(f"  {name} — {CENSUS[name]}", file=sys.stderr)
        print("\n  A stale census makes this look tighter than it is. Drop it.",
              file=sys.stderr)
        return 1

    print(f"ok: {len(found)} component(s) name a body, all classified.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
