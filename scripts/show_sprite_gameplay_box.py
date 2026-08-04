#!/usr/bin/env python3
"""Draw a character's GAMEPLAY box on top of the art the player actually sees.

The question this answers is one nobody could answer without running the game
and squinting: **does the collision box the simulation uses line up with the
body that is drawn?**

It cannot be answered from either side alone. The generator knows where the
body is in sprite pixels (``body_metrics.body_pixel_bbox``); the runtime knows
how big the collider is in world units (``30x48`` for the default player). The
mapping between them is ``sprite_render_size`` — height is
``max(collision.x, collision.y) * collision_scale``, width follows the frame's
aspect — and ``collision_scale`` is a HAND-TUNED per-character constant. So the
alignment is whatever somebody guessed, and nothing checks the guess.

This script reimplements that mapping exactly (see ``ambition_sprite_sheet``'s
``character::sheets::geometry``) and reports the miss in both directions:

* the drawn body's size in world units, against the collider's size;
* the collider's edges in sprite pixels, against the drawn body's edges.

⚠ **it reimplements rather than calls.** That is a real duplication and it is
the honest kind: the alternative is a Rust binary that opens a window. If the
runtime formula changes, this instrument goes stale, and the docstring on
``sprite_render_size`` says so from the other side.

Usage::

    python3 scripts/show_sprite_gameplay_box.py player_robot_v3
    python3 scripts/show_sprite_gameplay_box.py mary_o_tall --collision 30x60
    python3 scripts/show_sprite_gameplay_box.py --all --quiet   # the whole cast

Writes a PNG per character and prints a clickable link to it.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import sys

REPO = Path(__file__).resolve().parent.parent
SHEET_DIRS = [
    REPO / "crates/ambition_platformer2d_actor_monolith/assets/sprites",
]

# `PLAYER_PLACEHOLDER_VISUAL_SCALE` in
# crates/ambition_sprite_sheet/src/character/sheets/geometry.rs — a
# presentation-only multiplier applied on the PLAYER path only.
PLAYER_VISUAL_SCALE = 1.16
# `DEFAULT_PLAYER_BODY_WIDTH` / `_HEIGHT`.
DEFAULT_COLLISION = (30.0, 48.0)
# The runtime default when a sheet publishes no `tuning` block.
DEFAULT_COLLISION_SCALE = 1.5


@dataclass
class Sheet:
    target: str
    path: Path
    image: Path
    frame_w: int
    frame_h: int
    collision_scale: float
    body_bbox: tuple[int, int, int, int] | None
    feet_anchor_y: float | None
    rows: list
    # The sheet's own claim that `body_pixel_bbox` is a GAMEPLAY BODY rather
    # than the alpha extent of the drawing. A character whose sheet says so is
    # sized through `BodySource::SpriteAuthored { world_per_pixel }` and never
    # touches `collision_scale`, so reporting it through the legacy formula
    # would describe a path it is not on.
    authored_body: bool = False


def load_sheet(name: str) -> Sheet:
    import pyron

    for directory in SHEET_DIRS:
        candidate = directory / f"{name}_spritesheet.ron"
        if candidate.exists():
            break
    else:
        raise SystemExit(f"no sheet ron found for {name!r} under {SHEET_DIRS[0]}")

    record = pyron.loads(candidate.read_text())[0]
    tuning = record.get("tuning") or {}
    metrics = record.get("body_metrics") or {}
    bbox = metrics.get("body_pixel_bbox")
    anchor = metrics.get("feet_anchor_norm")
    return Sheet(
        target=record["target"],
        path=candidate,
        image=candidate.parent / record["image"],
        frame_w=record["frame_width"],
        frame_h=record["frame_height"],
        collision_scale=tuning.get("collision_scale", DEFAULT_COLLISION_SCALE),
        body_bbox=(bbox["x"], bbox["y"], bbox["w"], bbox["h"]) if bbox else None,
        feet_anchor_y=anchor["y"] if anchor else None,
        rows=record.get("rows") or [],
        authored_body=bool(metrics.get("authored_body")),
    )


def world_per_pixel(sheet: Sheet, collision: tuple[float, float]) -> float:
    """The sprite-authored scale: stand this character at the authored height.

    `BodySource::SpriteAuthored` takes ONE number and everything follows from
    the art at that scale, so there is no second formula to keep in step. The
    height is the authored quantity and the scale is derived from it — pinning
    the scale instead would change how tall the character stands the first time
    a regeneration re-crops them by a pixel.
    """
    _, _, _, body_h = sheet.body_bbox
    return collision[1] / max(body_h, 1)


def render_size(sheet: Sheet, collision: tuple[float, float], visual_scale: float):
    """`sprite_render_size_scaled` — the runtime formula, reproduced."""
    if sheet.authored_body:
        wpp = world_per_pixel(sheet, collision)
        return sheet.frame_w * wpp, sheet.frame_h * wpp
    height = max(collision[0], collision[1], 8.0) * sheet.collision_scale * max(visual_scale, 0.05)
    width = height * (sheet.frame_w / sheet.frame_h)
    return width, height


def anchor_norm_y(sheet: Sheet, collision: tuple[float, float], render_h: float) -> float:
    """`feet_anchor_for_render_size` — the sprite point pinned to the body centre."""
    return (sheet.feet_anchor_y or 0.0) + (collision[1] * 0.5) / max(render_h, 1.0)


def collider_in_frame_pixels(sheet: Sheet, collision: tuple[float, float], visual_scale: float):
    """Where the world collision box lands in sprite-frame pixel space."""
    if sheet.authored_body:
        # By construction it lands ON the authored rectangle: that IS the box,
        # and the quad is placed by `ActorSpriteOffset` so the art's rectangle
        # sits on it. Nothing here to reconcile — which is the entire argument
        # for the seam.
        bx, by, bw, bh = sheet.body_bbox
        return float(bx), float(by), float(bx + bw), float(by + bh)
    rw, rh = render_size(sheet, collision, visual_scale)
    ay = anchor_norm_y(sheet, collision, rh)
    # x: the anchor is horizontally centred, so world x maps straight through.
    half_w_px = (collision[0] * 0.5) / rw * sheet.frame_w
    x0 = sheet.frame_w * 0.5 - half_w_px
    x1 = sheet.frame_w * 0.5 + half_w_px
    # y: world offset -> normalised (up positive) -> pixel row (down positive).
    def py(world_dy: float) -> float:
        norm_y = world_dy / rh + ay
        return (0.5 - norm_y) * sheet.frame_h

    return x0, py(collision[1] * 0.5), x1, py(-collision[1] * 0.5)


def drawn_body_in_world(sheet: Sheet, collision: tuple[float, float], visual_scale: float):
    """How big the AUTHORED body box is once drawn, in world units."""
    if sheet.body_bbox is None:
        return None
    rw, rh = render_size(sheet, collision, visual_scale)
    _, _, w, h = sheet.body_bbox
    return w / sheet.frame_w * rw, h / sheet.frame_h * rh


def idle_frame(sheet: Sheet):
    """Reassemble one logical frame from the TRIMMED sheet rect plus its offset."""
    from PIL import Image

    row = next((r for r in sheet.rows if r.get("animation") == "idle"), None)
    if row is None and sheet.rows:
        row = sheet.rows[0]
    if row is None or not row.get("rects"):
        raise SystemExit(f"{sheet.target}: no frames in {sheet.path}")
    rect = row["rects"][0]
    page = Image.open(sheet.image).convert("RGBA")
    trimmed = page.crop((rect["x"], rect["y"], rect["x"] + rect["w"], rect["y"] + rect["h"]))
    frame = Image.new("RGBA", (sheet.frame_w, sheet.frame_h), (0, 0, 0, 0))
    off = rect.get("off") or (0, 0)
    # `paste` with the image as its own mask, or the trimmed alpha is clobbered
    # by the transparent canvas underneath it.
    frame.paste(trimmed, (int(off[0]), int(off[1])), trimmed)
    return frame


def hurtbox_in_frame_pixels(sheet: Sheet, insets: tuple[float, float, float, float]):
    """A fractional inset of the authored body box, in frame pixels.

    The fractions are a PARAMETER rather than a constant on purpose: the
    authored hurtbox lives on the character definition in Rust, and a copy of
    its numbers here would be a second authority that drifts silently. Pass the
    ones you are checking; the picture is the point.
    """
    if sheet.body_bbox is None:
        return None
    left, right, top, bottom = insets
    bx, by, bw, bh = sheet.body_bbox
    return (bx + bw * left, by + bh * top, bx + bw * (1 - right), by + bh * (1 - bottom))


def draw(sheet: Sheet, collision, visual_scale: float, out: Path, zoom: int = 3, hurtbox=None):
    from PIL import Image, ImageDraw

    frame = idle_frame(sheet).resize(
        (sheet.frame_w * zoom, sheet.frame_h * zoom), Image.NEAREST
    )
    # A checkerboard, so transparent pixels are distinguishable from white art.
    board = Image.new("RGBA", frame.size, (44, 44, 52, 255))
    tile = 8 * zoom
    cells = ImageDraw.Draw(board)
    for y in range(0, board.height, tile):
        for x in range(0, board.width, tile):
            if (x // tile + y // tile) % 2:
                cells.rectangle([x, y, x + tile, y + tile], fill=(56, 56, 66, 255))
    board.alpha_composite(frame)
    art = ImageDraw.Draw(board)

    if sheet.body_bbox:
        bx, by, bw, bh = sheet.body_bbox
        art.rectangle(
            [bx * zoom, by * zoom, (bx + bw) * zoom, (by + bh) * zoom],
            outline=(90, 230, 120, 255),
            width=max(1, zoom // 2),
        )
    x0, y0, x1, y1 = collider_in_frame_pixels(sheet, collision, visual_scale)
    art.rectangle(
        [x0 * zoom, y0 * zoom, x1 * zoom, y1 * zoom],
        outline=(240, 80, 90, 255),
        width=max(1, zoom // 2),
    )
    legend = f"{sheet.target}  green=authored body  red=collider"
    if hurtbox is not None:
        hx0, hy0, hx1, hy1 = hurtbox
        art.rectangle(
            [hx0 * zoom, hy0 * zoom, hx1 * zoom, hy1 * zoom],
            outline=(120, 170, 255, 255),
            width=max(1, zoom // 2),
        )
        legend += "  blue=hurtbox"
    art.text((6, 6), legend, fill=(235, 235, 240, 255))
    out.parent.mkdir(parents=True, exist_ok=True)
    board.convert("RGB").save(out)
    return out


def report(sheet: Sheet, collision, visual_scale: float) -> list[str]:
    how = (
        f"AUTHORED body, world_per_pixel {world_per_pixel(sheet, collision):.4f}"
        if sheet.authored_body and sheet.body_bbox
        else f"measured body, collision_scale {sheet.collision_scale}"
    )
    lines = [f"{sheet.target}: frame {sheet.frame_w}x{sheet.frame_h}, {how}"]
    rw, rh = render_size(sheet, collision, visual_scale)
    lines.append(f"  collider {collision[0]:.0f}x{collision[1]:.0f}  ->  sprite quad {rw:.1f}x{rh:.1f}")
    drawn = drawn_body_in_world(sheet, collision, visual_scale)
    if drawn is None:
        lines.append("  no body_pixel_bbox authored — nothing to compare against")
        return lines
    lines.append(f"  drawn body in world: {drawn[0]:.1f}x{drawn[1]:.1f}")
    if drawn[0] <= 0.0 or drawn[1] <= 0.0:
        # A degenerate authored box is a real answer, not a crash: it means the
        # sheet published a zero-extent body and the scale below is undefined.
        lines.append("  ⚠ authored body box has a zero extent — no ratio to report")
        return lines
    lines.append(
        f"  collider / drawn body: {collision[0] / drawn[0]:.2f}x wide, "
        f"{collision[1] / drawn[1]:.2f}x tall"
    )
    x0, y0, x1, y1 = collider_in_frame_pixels(sheet, collision, visual_scale)
    bx, by, bw, bh = sheet.body_bbox
    lines.append(
        f"  collider in frame px: x {x0:.1f}..{x1:.1f}, y {y0:.1f}..{y1:.1f}   "
        f"(authored body x {bx}..{bx + bw}, y {by}..{by + bh})"
    )
    return lines


def link(path: Path) -> str:
    try:
        from rich.console import Console

        Console().print(f"[link=file://{path}]{path}[/link]")
        Console().print(f"[link=file://{path.parent}]{path.parent}[/link]")
        return ""
    except ImportError:
        return f"{path}\n{path.parent}"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("target", nargs="*", help="sheet target name, e.g. player_robot_v3")
    ap.add_argument("--all", action="store_true", help="every sheet that authors a body bbox")
    ap.add_argument("--collision", default="30x48", help="world collision box, WxH (default the player's)")
    ap.add_argument("--visual-scale", type=float, default=None,
                    help=f"extra presentation scale; defaults to {PLAYER_VISUAL_SCALE} for player sheets, else 1.0")
    ap.add_argument("--out", default=str(REPO / "target/sprite_gameplay_box"))
    ap.add_argument("--zoom", type=int, default=3)
    ap.add_argument("--quiet", action="store_true", help="numbers only, no PNG")
    ap.add_argument(
        "--hurtbox-inset",
        metavar="L,R,T,B",
        help="also draw a hurtbox as fractional insets of the authored body box "
        "(the character definition owns the real numbers; this only draws them)",
    )
    args = ap.parse_args()

    insets = None
    if args.hurtbox_inset:
        parts = [float(v) for v in args.hurtbox_inset.split(",")]
        if len(parts) != 4:
            ap.error("--hurtbox-inset takes four fractions: left,right,top,bottom")
        insets = tuple(parts)

    w, _, h = args.collision.partition("x")
    collision = (float(w), float(h))

    names = list(args.target)
    if args.all:
        names = sorted(
            p.name[: -len("_spritesheet.ron")]
            for p in SHEET_DIRS[0].glob("*_spritesheet.ron")
        )
    if not names:
        ap.error("name a target or pass --all")

    written = []
    for name in names:
        sheet = load_sheet(name)
        scale = args.visual_scale
        if scale is None:
            scale = PLAYER_VISUAL_SCALE if "player_robot" in sheet.target else 1.0
        if args.all and sheet.body_bbox is None:
            continue
        print("\n".join(report(sheet, collision, scale)))
        if not args.quiet:
            written.append(
                draw(
                    sheet,
                    collision,
                    scale,
                    Path(args.out) / f"{sheet.target}.png",
                    args.zoom,
                    hurtbox=hurtbox_in_frame_pixels(sheet, insets) if insets else None,
                )
            )
    for path in written:
        out = link(path)
        if out:
            print(out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
