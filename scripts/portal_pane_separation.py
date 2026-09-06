#!/usr/bin/env python3
"""How close do two portal panes get, in the same area?

⭐ THE QUESTION THIS ANSWERS. The far-side compositing repair subtracts a pane's
aperture from a far body's sprite, which costs at most THREE clip half-planes --
exactly the budget `PortalClipMaterial` carries. Subtracting TWO apertures from
one body needs more than three, so the road is complete only if no single body
can be far-covered by two panes at once.

That is a claim about CONTENT, not code, so it is measured rather than assumed:
if the closest two panes in any area are farther apart than the widest body, no
body can overlap both apertures and the single-pane road is total.

⚠ A NEGATIVE result here does NOT say the two-pane case is impossible in
general -- it says the SHIPPED worlds do not contain it. New authoring can, which
is why `check_portal_panes_stay_separable.py` exists to fail when it does.
"""
from __future__ import annotations
import json, math, pathlib, sys, collections

ROOT = pathlib.Path(__file__).resolve().parent.parent
WORLDS = ROOT / "game/ambition_content/assets/worlds"


def panes():
    """(area, x, y, normal, link, name) for every authored Portal."""
    out = collections.defaultdict(list)
    for world in sorted(WORLDS.glob("*.ldtk")):
        data = json.loads(world.read_text(encoding="utf-8"))
        for level in data.get("levels") or []:
            fields = {
                f.get("__identifier"): f.get("__value")
                for f in level.get("fieldInstances") or []
            }
            area = fields.get("activeArea") or level.get("identifier")
            ox, oy = level.get("worldX", 0), level.get("worldY", 0)
            for layer in level.get("layerInstances") or []:
                for e in layer.get("entityInstances") or []:
                    if e.get("__identifier") != "Portal":
                        continue
                    f = {
                        x.get("__identifier"): x.get("__value")
                        for x in e.get("fieldInstances") or []
                    }
                    px, py = e.get("px") or [0, 0]
                    out[f"{world.stem}:{area}"].append(
                        (
                            ox + px,
                            oy + py,
                            f.get("normal"),
                            f.get("link"),
                            f.get("name") or f.get("id"),
                        )
                    )
    return out


OPPOSITE = {("left", "right"), ("right", "left"), ("up", "down"), ("down", "up")}


def back_to_back(a, b) -> bool:
    """A thin-wall doorway: one link, two panes, opposing normals.

    ⭐ These are the ONE pair that sits closer together than a body is wide, and
    they are not a counter-example: "far of both" is the band BETWEEN them, which
    is the inside of the wall slab. A body standing inside a 32px wall is not a
    case this repair owes an answer to, and `compute_cone`'s doorway clamp
    already treats that slab specially.
    """
    return a[3] is not None and a[3] == b[3] and (a[2], b[2]) in OPPOSITE


def main() -> int:
    if not WORLDS.is_dir():
        print(f"skip: no worlds at {WORLDS}", file=sys.stderr)
        return 3
    groups = panes()
    if not groups:
        print(
            "FAIL: no portals found at all -- this measurement is vacuous.",
            file=sys.stderr,
        )
        return 1

    total = sum(len(v) for v in groups.values())
    print(f"portals: {total} across {len(groups)} (world:area) group(s)")

    pairs = []
    for area, pts in sorted(groups.items()):
        for i, a in enumerate(pts):
            for b in pts[i + 1 :]:
                pairs.append((math.dist(a[:2], b[:2]), area, a, b))
    pairs.sort(key=lambda r: r[0])
    if not pairs:
        print("no area holds two panes ⇒ no body can be covered by two at once")
        return 0

    doorways = [r for r in pairs if back_to_back(r[2], r[3])]
    facing = [r for r in pairs if not back_to_back(r[2], r[3])]
    print(f"pane pairs: {len(pairs)}  ({len(doorways)} back-to-back doorway)")
    for d, area, a, b in pairs[:4]:
        kind = "doorway" if back_to_back(a, b) else "distinct"
        print(f"  {d:7.1f}px  {kind:8}  {a[4]!r} ({a[2]}) || {b[4]!r} ({b[2]})  [{area}]")

    if not facing:
        print("every pair is a back-to-back doorway ⇒ single-pane road is total")
        return 0
    d, area, a, b = facing[0]
    print()
    print(f"CLOSEST NON-DOORWAY PAIR: {d:.1f}px  ({area})")
    print(
        f"⇒ only a body WIDER than {d:.1f}px could be far-covered by two panes at "
        "once, which is the case the three-clip-plane budget cannot express."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
