from __future__ import annotations

from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Dict, Iterable, List, Tuple

import yaml
from PIL import Image, ImageDraw

from ambition_sprite2d_renderer.core.draw import (
    bbox,
    font,
    poly_scaled,
    rgba,
    with_alpha,
)
from ambition_sprite2d_renderer.core.pipeline import (
    CROP_GROUND,
    CROP_NONE,
    CROP_TIGHT,
    render_frame,
)

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]


@dataclass(frozen=True)
class EntitySpriteSpec:
    key: str
    filename: str
    category: str
    state: str
    gameplay_hint: str
    size: Tuple[int, int] = (128, 128)
    # Set False for sprites that should be saved at full canvas size,
    # not auto-cropped to their alpha bbox. Tile sprites (designed
    # to repeat seamlessly across a surface) need their full canvas
    # preserved so `Sprite::image_mode = Tiled` repeats them at the
    # authored scale instead of stretching the cropped extent.
    tight_crop: bool = True
    # Set True for sprites that STAND on the ground (doors, standing
    # props). The crop then keeps the bottom edge flush (no bottom
    # padding) so the lowest opaque pixel — the sprite's "feet" — is the
    # texture's bottom edge. The runtime plants that edge on the bottom
    # (floor) face of the entity box so the sprite never floats.
    ground: bool = False


def _render_supersampled(
    draw_fn: Callable[[ImageDraw.ImageDraw, float], None],
    size: Tuple[int, int] = (128, 128),
    supersample: int = 4,
    tight_crop: bool = True,
    crop_padding: int = 4,
    ground: bool = False,
) -> Image.Image:
    """Rasterize a draw fn at `size * supersample`, downsample, then
    optionally crop to the alpha bounding box plus a small padding so
    the resulting PNG has minimal transparent margin.

    Why crop: the sandbox stretches the texture to fill the entity's
    collision box (`Sprite::custom_size`). If the actual artwork
    occupies only ~30% of the 128×128 canvas (which most authored
    drawers do — they leave breathing room around the subject), the
    in-game sprite ends up visibly smaller than the collision box,
    surrounded by an apparent halo of transparency. Cropping to the
    artwork's bounding box normalizes content density so a stretched
    sprite fills the collision box at the artist's intended scale.

    Tile drawers (designed to repeat seamlessly across a surface)
    pass `tight_crop=False` to preserve their full canvas — those
    sprites tile via `Sprite::image_mode` instead of stretching.
    """
    crop = CROP_NONE if not tight_crop else (CROP_GROUND if ground else CROP_TIGHT)
    return render_frame(
        draw_fn, size, supersample=supersample, crop=crop, crop_padding=crop_padding
    )


def draw_gem(
    d: ImageDraw.ImageDraw,
    center: Point,
    radius: float,
    color: Color,
    outline: Color,
    s: float,
) -> None:
    cx, cy = center
    pts = [
        (cx, cy - radius),
        (cx + radius * 0.85, cy - radius * 0.10),
        (cx + radius * 0.55, cy + radius),
        (cx - radius * 0.55, cy + radius),
        (cx - radius * 0.85, cy - radius * 0.10),
    ]
    d.polygon(poly_scaled(pts, s), fill=color, outline=outline)
    d.line(
        poly_scaled(
            [
                (cx, cy - radius),
                (cx, cy + radius),
                (cx + radius * 0.85, cy - radius * 0.10),
            ],
            s,
        ),
        fill=with_alpha((255, 255, 255, 255), 110),
        width=max(1, int(1.2 * s)),
    )


def chest_closed(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#221714")
    d.ellipse(bbox(64 * s, 92 * s, 74 * s, 15 * s), fill=(0, 0, 0, 45))
    d.rounded_rectangle(
        (28 * s, 52 * s, 100 * s, 91 * s),
        radius=8 * s,
        fill=rgba("#8D4B22"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    d.rounded_rectangle(
        (25 * s, 42 * s, 103 * s, 66 * s),
        radius=12 * s,
        fill=rgba("#C98231"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    d.rectangle(
        (28 * s, 63 * s, 100 * s, 70 * s),
        fill=rgba("#F0B84A"),
        outline=outline,
        width=max(1, int(1 * s)),
    )
    d.rounded_rectangle(
        (56 * s, 58 * s, 72 * s, 78 * s),
        radius=3 * s,
        fill=rgba("#FFE477"),
        outline=outline,
        width=max(1, int(1 * s)),
    )


def chest_open(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#221714")
    d.ellipse(bbox(64 * s, 94 * s, 78 * s, 15 * s), fill=(0, 0, 0, 45))
    d.polygon(
        poly_scaled([(31, 56), (64, 33), (97, 56), (92, 67), (64, 51), (36, 67)], s),
        fill=rgba("#D78B34"),
        outline=outline,
    )
    d.rounded_rectangle(
        (28 * s, 62 * s, 100 * s, 92 * s),
        radius=8 * s,
        fill=rgba("#8D4B22"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    for x in [47, 61, 76]:
        d.line(
            [(x * s, 59 * s), (x * s, 32 * s)],
            fill=rgba("#FFF18A", 120),
            width=max(1, int(2 * s)),
        )
    draw_gem(d, (64, 63), 11, rgba("#6BE9FF"), outline, s)


def breakable_intact(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#241714")
    d.rounded_rectangle(
        (27 * s, 38 * s, 101 * s, 92 * s),
        radius=6 * s,
        fill=rgba("#8A5736"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    for x in (45, 70, 92):
        d.line(
            [(x * s, 41 * s), ((x - 7) * s, 90 * s)],
            fill=rgba("#B98255"),
            width=max(1, int(2 * s)),
        )
    d.line(
        [(28 * s, 64 * s), (101 * s, 61 * s)],
        fill=rgba("#5C3928"),
        width=max(1, int(3 * s)),
    )


def breakable_cracked(d: ImageDraw.ImageDraw, s: float) -> None:
    breakable_intact(d, s)
    d.line(
        poly_scaled([(63, 39), (58, 55), (66, 62), (55, 75), (60, 92)], s),
        fill=rgba("#130B0A"),
        width=max(1, int(2 * s)),
    )
    d.line(
        poly_scaled([(66, 62), (83, 70), (95, 87)], s),
        fill=rgba("#130B0A"),
        width=max(1, int(1.5 * s)),
    )


def breakable_broken(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#241714")
    shards = [
        [(34, 74), (55, 58), (58, 91), (30, 93)],
        [(57, 50), (79, 47), (72, 82), (49, 72)],
        [(82, 62), (101, 72), (91, 93), (73, 86)],
    ]
    for pts in shards:
        d.polygon(poly_scaled(pts, s), fill=rgba("#8A5736"), outline=outline)
    d.ellipse(bbox(65 * s, 94 * s, 75 * s, 12 * s), fill=(0, 0, 0, 45))


def pickup_health(d: ImageDraw.ImageDraw, s: float) -> None:
    draw_gem(d, (64, 58), 28, rgba("#38E983"), rgba("#0C2A1C"), s)
    d.rounded_rectangle(
        (57 * s, 43 * s, 71 * s, 73 * s), radius=3 * s, fill=rgba("#FFFFFF")
    )
    d.rounded_rectangle(
        (49 * s, 51 * s, 79 * s, 65 * s), radius=3 * s, fill=rgba("#FFFFFF")
    )
    d.ellipse(bbox(64 * s, 93 * s, 43 * s, 10 * s), fill=(0, 0, 0, 36))


def pickup_currency(d: ImageDraw.ImageDraw, s: float) -> None:
    d.ellipse(
        bbox(64 * s, 61 * s, 50 * s, 50 * s),
        fill=rgba("#FFD65A"),
        outline=rgba("#5C4112"),
        width=max(1, int(2 * s)),
    )
    d.ellipse(
        bbox(64 * s, 61 * s, 34 * s, 34 * s),
        outline=rgba("#FFF3A4"),
        width=max(1, int(3 * s)),
    )
    d.text((57 * s, 45 * s), "$", fill=rgba("#5C4112"), font=font(int(28 * s)))
    d.ellipse(bbox(64 * s, 94 * s, 42 * s, 10 * s), fill=(0, 0, 0, 34))


def pickup_ability(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#0B1930")
    d.ellipse(
        bbox(64 * s, 62 * s, 52 * s, 52 * s),
        fill=rgba("#1A2452"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    for r, a in [(45, 70), (33, 95), (22, 135)]:
        d.ellipse(
            bbox(64 * s, 62 * s, r * s, r * s),
            outline=rgba("#6BE9FF", a),
            width=max(1, int(1.4 * s)),
        )
    d.polygon(
        poly_scaled([(56, 44), (80, 62), (62, 65), (72, 84), (48, 63), (66, 60)], s),
        fill=rgba("#B98CFF"),
        outline=outline,
    )


def hazard_spikes(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#24060B")
    d.ellipse(bbox(64 * s, 92 * s, 80 * s, 13 * s), fill=(0, 0, 0, 40))
    for i in range(5):
        x = 28 + i * 18
        d.polygon(
            poly_scaled([(x, 91), (x + 10, 39), (x + 20, 91)], s),
            fill=rgba("#F04450"),
            outline=outline,
        )
        d.polygon(
            poly_scaled([(x + 8, 55), (x + 10, 39), (x + 12, 57)], s),
            fill=rgba("#FFB1B5"),
        )


def npc_terminal(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#101820")
    d.rounded_rectangle(
        (39 * s, 31 * s, 89 * s, 90 * s),
        radius=8 * s,
        fill=rgba("#27364E"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    d.rounded_rectangle(
        (45 * s, 38 * s, 83 * s, 63 * s),
        radius=5 * s,
        fill=rgba("#07131E"),
        outline=outline,
    )
    d.ellipse(bbox(55 * s, 51 * s, 7 * s, 10 * s), fill=rgba("#6BE9FF"))
    d.ellipse(bbox(73 * s, 51 * s, 7 * s, 10 * s), fill=rgba("#6BE9FF"))
    d.rectangle((49 * s, 72 * s, 79 * s, 77 * s), fill=rgba("#C98CFF"))
    d.line(
        [(45 * s, 91 * s), (35 * s, 104 * s)], fill=outline, width=max(1, int(3 * s))
    )
    d.line(
        [(83 * s, 91 * s), (93 * s, 104 * s)], fill=outline, width=max(1, int(3 * s))
    )


def boss_core(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#1B0624")
    d.ellipse(
        bbox(64 * s, 64 * s, 72 * s, 72 * s),
        fill=rgba("#7520A5"),
        outline=outline,
        width=max(1, int(3 * s)),
    )
    for ang in range(0, 360, 45):
        import math

        a = math.radians(ang)
        x = 64 + math.cos(a) * 47
        y = 64 + math.sin(a) * 47
        d.line(
            [(64 * s, 64 * s), (x * s, y * s)],
            fill=rgba("#EC4DFF", 120),
            width=max(1, int(2 * s)),
        )
    d.ellipse(
        bbox(64 * s, 64 * s, 30 * s, 30 * s),
        fill=rgba("#1B0826"),
        outline=rgba("#FF78FF"),
        width=max(1, int(2 * s)),
    )
    d.ellipse(bbox(64 * s, 64 * s, 12 * s, 12 * s), fill=rgba("#FFFFFF"))


def sandbag_dummy(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#2A1D13")
    d.ellipse(bbox(64 * s, 97 * s, 46 * s, 10 * s), fill=(0, 0, 0, 40))
    d.rounded_rectangle(
        (45 * s, 30 * s, 83 * s, 91 * s),
        radius=17 * s,
        fill=rgba("#B58A5D"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    d.line(
        [(48 * s, 45 * s), (80 * s, 45 * s)],
        fill=rgba("#6A4C32"),
        width=max(1, int(2 * s)),
    )
    d.line(
        [(52 * s, 58 * s), (76 * s, 76 * s)],
        fill=rgba("#6A4C32"),
        width=max(1, int(3 * s)),
    )
    d.line(
        [(76 * s, 58 * s), (52 * s, 76 * s)],
        fill=rgba("#6A4C32"),
        width=max(1, int(3 * s)),
    )
    d.rectangle((58 * s, 24 * s, 70 * s, 35 * s), fill=rgba("#755236"), outline=outline)


def moving_platform(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#10253A")
    d.ellipse(bbox(64 * s, 83 * s, 88 * s, 13 * s), fill=(0, 0, 0, 40))
    d.rounded_rectangle(
        (20 * s, 55 * s, 108 * s, 75 * s),
        radius=9 * s,
        fill=rgba("#4CB4FF"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    d.rectangle((28 * s, 61 * s, 100 * s, 67 * s), fill=rgba("#BCEBFF"))
    for x in (35, 64, 93):
        d.ellipse(
            bbox(x * s, 78 * s, 12 * s, 12 * s),
            fill=rgba("#12263A"),
            outline=rgba("#7ED6FF"),
        )


def rebound_pad(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#3A1904")
    d.rounded_rectangle(
        (21 * s, 68 * s, 107 * s, 91 * s),
        radius=8 * s,
        fill=rgba("#F38E2A"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    d.polygon(
        poly_scaled(
            [(27, 68), (41, 40), (55, 68), (69, 40), (83, 68), (97, 40), (103, 68)], s
        ),
        fill=rgba("#FFD26A"),
        outline=outline,
    )
    d.line(
        [(64 * s, 82 * s), (64 * s, 45 * s)],
        fill=rgba("#FFFFFF", 170),
        width=max(1, int(2 * s)),
    )


def pogo_orb(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#07251A")
    d.ellipse(
        bbox(64 * s, 64 * s, 44 * s, 44 * s),
        fill=rgba("#29E88B"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    d.ellipse(bbox(57 * s, 55 * s, 14 * s, 14 * s), fill=rgba("#D9FFF0"))
    d.arc(
        (29 * s, 29 * s, 99 * s, 99 * s),
        20,
        330,
        fill=rgba("#77FFD0", 170),
        width=max(1, int(3 * s)),
    )


def soft_blink_wall(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rounded_rectangle(
        (33 * s, 23 * s, 95 * s, 105 * s),
        radius=9 * s,
        fill=rgba("#5632B5", 170),
        outline=rgba("#211052"),
        width=max(1, int(2 * s)),
    )
    for x in (44, 60, 76, 91):
        d.line(
            [(x * s, 28 * s), ((x - 12) * s, 101 * s)],
            fill=rgba("#B897FF", 105),
            width=max(1, int(2 * s)),
        )


def hard_blink_wall(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rounded_rectangle(
        (33 * s, 23 * s, 95 * s, 105 * s),
        radius=6 * s,
        fill=rgba("#841ABF", 220),
        outline=rgba("#260038"),
        width=max(1, int(3 * s)),
    )
    for y in (38, 60, 82):
        d.line(
            [(37 * s, y * s), (91 * s, (y + 8) * s)],
            fill=rgba("#FF74FF", 150),
            width=max(1, int(2 * s)),
        )


def solid_block(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rounded_rectangle(
        (25 * s, 31 * s, 103 * s, 95 * s),
        radius=5 * s,
        fill=rgba("#42495C"),
        outline=rgba("#161A24"),
        width=max(1, int(2 * s)),
    )
    d.line(
        [(29 * s, 49 * s), (101 * s, 49 * s)],
        fill=rgba("#687089"),
        width=max(1, int(2 * s)),
    )
    d.line(
        [(55 * s, 32 * s), (55 * s, 95 * s)],
        fill=rgba("#2C3240"),
        width=max(1, int(2 * s)),
    )


def one_way_platform(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rounded_rectangle(
        (18 * s, 58 * s, 110 * s, 74 * s),
        radius=6 * s,
        fill=rgba("#677699"),
        outline=rgba("#1A2235"),
        width=max(1, int(2 * s)),
    )
    for x in (32, 50, 68, 86):
        d.polygon(
            poly_scaled([(x, 51), (x + 7, 40), (x + 14, 51)], s), fill=rgba("#B4C6F4")
        )


def door_zone(d: ImageDraw.ImageDraw, s: float) -> None:
    # A solid, upright paneled door whose FOOT is the bottom edge of the
    # canvas. This drawer is published with `ground=True`, so the crop
    # keeps the bottom flush (no padding): the lowest opaque pixel — the
    # gold sill below — becomes the texture's bottom edge, i.e. the
    # door's "feet". The renderer then plants those feet on the bottom
    # (floor) edge of the loading-zone box, so the door stands on the
    # ground rather than floating. The fill is fully opaque and warmer
    # than the cool hub background so it reads as a solid door instead
    # of a gold wireframe.
    gold = rgba("#F1B33B")
    gold_soft = rgba("#F1B33B", 200)
    rad = max(2, int(8 * s))
    # Outer frame / jamb. Top corners rounded, bottom left square so the
    # door foot is a flat edge flush with the floor.
    d.rounded_rectangle(
        (36 * s, 10 * s, 92 * s, 127 * s),
        radius=rad,
        corners=(True, True, False, False),
        fill=rgba("#2A3148"),
        outline=gold,
        width=max(1, int(3 * s)),
    )
    # Door leaf — solid, warm bronze so it pops against the dark-blue hub.
    d.rounded_rectangle(
        (42 * s, 16 * s, 86 * s, 113 * s),
        radius=max(2, int(4 * s)),
        corners=(True, True, False, False),
        fill=rgba("#8A6A3E"),
        outline=rgba("#5E441F"),
        width=max(1, int(2 * s)),
    )
    # Two recessed panels with a simple top-left highlight / bottom-right
    # shadow for depth.
    for top, bot in ((22, 60), (68, 108)):
        d.rectangle(
            (48 * s, top * s, 80 * s, bot * s),
            fill=rgba("#6E5230"),
        )
        d.line(
            [(48 * s, bot * s), (48 * s, top * s), (80 * s, top * s)],
            fill=rgba("#C7A35F", 220),
            width=max(1, int(2 * s)),
        )
        d.line(
            [(80 * s, top * s), (80 * s, bot * s), (48 * s, bot * s)],
            fill=rgba("#4E3A22", 220),
            width=max(1, int(2 * s)),
        )
    # Doorknob on the latch (right) side, on the rail between the panels.
    d.ellipse(bbox(78 * s, 64 * s, 7 * s, 7 * s), fill=gold, outline=rgba("#7A5410"))
    d.ellipse(bbox(76 * s, 62 * s, 2 * s, 2 * s), fill=rgba("#FFF4D2", 230))
    # Gold sill/threshold at the very bottom — the door's flat foot.
    d.rectangle(
        (37 * s, 118 * s, 91 * s, 127 * s),
        fill=gold,
        outline=rgba("#7A5410"),
        width=max(1, int(1 * s)),
    )


def edge_exit(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rectangle(
        (27 * s, 26 * s, 101 * s, 102 * s),
        fill=rgba("#0D2230", 130),
        outline=rgba("#43E9FF", 200),
        width=max(1, int(2 * s)),
    )
    for x in (44, 62, 80):
        d.line(
            [(x * s, 35 * s), (x * s, 94 * s)],
            fill=rgba("#43E9FF", 120),
            width=max(1, int(2 * s)),
        )
    d.polygon(poly_scaled([(52, 49), (77, 64), (52, 79)], s), fill=rgba("#43E9FF"))


def projectile_energy(d: ImageDraw.ImageDraw, s: float) -> None:
    d.ellipse(
        bbox(64 * s, 64 * s, 38 * s, 24 * s),
        fill=rgba("#6BE9FF", 210),
        outline=rgba("#0B2B36"),
        width=max(1, int(2 * s)),
    )
    d.polygon(poly_scaled([(30, 64), (54, 52), (54, 76)], s), fill=rgba("#C58AFF", 150))
    d.ellipse(bbox(72 * s, 60 * s, 10 * s, 8 * s), fill=rgba("#FFFFFF", 200))


# ─── Tile drawers ───────────────────────────────────────────────────
#
# These are 32×32 textures designed to TILE seamlessly across a
# surface via `Sprite::image_mode = Tiled`. They fill the full canvas
# (no transparent margin), and the patterns are crafted so adjacent
# copies meet without visible seams: edge pixels match the opposite
# edge so a 32-pixel-wide horizontal repeat looks continuous.
#
# Bevy's tiled image mode renders the texture at its NATIVE pixel
# scale and repeats it to fill the sprite's `custom_size`. So a
# 32×32 tile rendered onto a 128×64 surface tiles 4 times across
# and 2 times down — no stretching.


def _seamless_brick_pattern(
    d: ImageDraw.ImageDraw, s: float, base: Color, mortar: Color, highlight: Color
) -> None:
    """Two staggered rows of bricks. The horizontal seam at row mid-
    height tiles cleanly because the bricks on row 0 align with the
    same offsets on row 2 (which is the wrapped continuation of row
    0 in a vertically-tiled rendering)."""
    # Background mortar fill.
    d.rectangle((0, 0, 32 * s, 32 * s), fill=mortar)
    # Row 0 (y=0..16): two full bricks across, plus the right half of
    # a brick wrapping from the previous tile on the left and the
    # left half wrapping forward to the next tile on the right.
    bricks_row0 = [(0, 0, 14, 14), (16, 0, 30, 14)]
    # Row 1 (y=16..32): offset by half a brick width so the staggered
    # pattern reads as masonry. Wraps at x boundaries.
    bricks_row1 = [(-7, 18, 7, 30), (9, 18, 23, 30), (25, 18, 39, 30)]
    for row in (bricks_row0, bricks_row1):
        for x0, y0, x1, y1 in row:
            d.rectangle((x0 * s, y0 * s, x1 * s, y1 * s), fill=base)
            # Top-left highlight on each brick to add depth.
            d.line(
                [(x0 * s + 1, y0 * s + 1), (x1 * s - 1, y0 * s + 1)],
                fill=highlight,
                width=max(1, int(1 * s)),
            )
            d.line(
                [(x0 * s + 1, y0 * s + 1), (x0 * s + 1, y1 * s - 1)],
                fill=highlight,
                width=max(1, int(1 * s)),
            )


def solid_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """Stone brick tile for IntGrid Solid surfaces. Cool gray
    palette so it reads as "structural wall / floor" against the
    warmer-tinted sandbox entities."""
    base = rgba("#4A536B")
    mortar = rgba("#22293A")
    highlight = rgba("#7682A0", 200)
    _seamless_brick_pattern(d, s, base, mortar, highlight)
    # Subtle noise dots so flat surfaces don't look perfectly
    # uniform when many tiles repeat.
    for x, y in [(5, 7), (21, 4), (12, 22), (28, 19)]:
        d.point((x * s, y * s), fill=highlight)


def one_way_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """One-way platform tile, 32×16 — sized to match a single
    16-px-tall IntGrid row at NATIVE scale.

    Why 32×16 and not 32×32: Bevy's `SpriteImageMode::Tiled` only
    tiles when `drawing_size >= original_size * stretch_value`.
    With a 32×32 texture stretched to a 16-tall block (the typical
    one-way row height), `ratio_y = 0.5 < 1.0` triggers a vertical
    STRETCH — the plate (8px tall in the source) compresses to 4px
    plus 12px of transparent gap. Looks broken.
    Matching the texture height to the typical block height
    (32×16) keeps `ratio_y = 1.0`, no stretch, plate fills the
    block. Wide platforms still tile horizontally (`ratio_x = N`).
    """
    plate = rgba("#677699")
    plate_top = rgba("#B4C6F4")
    edge = rgba("#1A2235")
    # The whole 16-tall canvas is the plate body, with a brighter
    # 3-tall top ridge for depth and a 1-px dark edge at the bottom
    # so the plate reads as a discrete surface.
    d.rectangle((0, 0, 32 * s, 16 * s), fill=plate)
    d.rectangle((0, 0, 32 * s, 4 * s), fill=plate_top)
    d.line([(0, 15 * s), (32 * s, 15 * s)], fill=edge, width=max(1, int(1 * s)))
    # Up-arrow markers placed where adjacent horizontal tiles align
    # cleanly: arrow centers at x=8 and x=24, half-width 3, so the
    # tile boundary at x=32/x=0 cuts through the gap between
    # arrows, not through an arrow.
    for x in (8, 24):
        d.polygon(poly_scaled([(x - 3, 11), (x, 5), (x + 3, 11)], s), fill=plate_top)


def hazard_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """Hazard / spike tile, 32×16 — same 32×16 sizing rationale as
    `one_way_tile` (most authored hazard rows are 16 px tall, so
    matching the texture height keeps spikes at native scale).

    Bright red base + lighter spike-tip highlight so the player
    reads "danger" at a glance regardless of palette. Three spikes
    per repeat unit, anchored so the rightmost spike's right edge
    meets the leftmost spike's left edge across the horizontal
    tile boundary."""
    base = rgba("#5A1418")
    spike = rgba("#F04450")
    spike_hi = rgba("#FFB1B5")
    edge = rgba("#220404")
    # Solid base in the lower 6 rows — what the player visually
    # "stands on" when they get hit.
    d.rectangle((0, 9 * s, 32 * s, 16 * s), fill=base)
    d.line([(0, 9 * s), (32 * s, 9 * s)], fill=edge, width=max(1, int(1 * s)))
    # Three spikes rooted at y=9, tips at y=1. Half-width 4 each.
    # x-positions chosen so x=5 and x=27 are 5px from each edge —
    # adjacent tiles' spikes interleave at x=±5 from the tile
    # seam, reading as one continuous strip.
    for x in (5, 16, 27):
        d.polygon(
            poly_scaled([(x - 4, 9), (x, 1), (x + 4, 9)], s), fill=spike, outline=edge
        )
        d.polygon(poly_scaled([(x - 1, 5), (x, 1), (x + 1, 5)], s), fill=spike_hi)


def soft_blink_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """Soft blink-passable wall tile. Translucent purple with a
    diagonal hatch the player learns to associate with "blinks
    through this." Diagonals start and end on tile edges so the
    repeat reads continuous."""
    fill = rgba("#5632B5", 180)
    hatch = rgba("#B897FF", 140)
    d.rectangle((0, 0, 32 * s, 32 * s), fill=fill)
    # Diagonal hatch every 8px. Lines run from (0, n) to (n, 0)
    # equivalents at the wrap boundary, but PIL doesn't wrap, so
    # we draw enough segments to visually repeat across the join.
    for offset in range(-32, 64, 8):
        d.line(
            [(offset * s, 0), ((offset + 32) * s, 32 * s)],
            fill=hatch,
            width=max(1, int(1 * s)),
        )


def hard_blink_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """Hard blink wall tile. Saturated solid purple — same palette
    family as soft, but no transparency and a denser cross-hatch so
    the player reads "I cannot blink through this" at a glance."""
    fill = rgba("#841ABF")
    edge = rgba("#260038")
    hatch = rgba("#FF74FF", 170)
    d.rectangle((0, 0, 32 * s, 32 * s), fill=fill)
    d.rectangle((0, 0, 32 * s, 32 * s), outline=edge, width=max(1, int(1 * s)))
    for offset in range(-32, 64, 6):
        d.line(
            [(offset * s, 0), ((offset + 32) * s, 32 * s)],
            fill=hatch,
            width=max(1, int(1 * s)),
        )
        d.line(
            [(offset * s, 32 * s), ((offset + 32) * s, 0)],
            fill=hatch,
            width=max(1, int(1 * s)),
        )


# ─── New entity drawers (sprites without dedicated art yet) ──────────
#
# Each of the following drawers fills a gameplay surface that the
# sandbox currently renders as a flat colored rectangle (or, in the
# morph-ball case, a Rust-side procedural texture). Adding them here
# routes through the standard `ENTITY_SPECS` / `DRAWERS` pipeline so
# they get the same crop / supersample / contact-sheet treatment as
# the existing entities, and the runtime loader picks them up
# automatically the next time `write_entity_sprites` runs.


# Lever palettes. The housing/shaft are state-independent steel; the two
# throw positions are colour-coded (red = left = armed, green = right =
# disabled) so the lever reads as a physical two-position switch rather
# than a colour-shifting gem.
_LEVER_STEEL = {
    "housing": "#2A2F3A",
    "housing_hi": "#454C5C",
    "shaft": "#5A6373",
    "shaft_hi": "#9AA4B5",
}
# Gem palettes — reused both for the small position sockets and for the
# gem mounted as the lever's knob (the original switch art lives on, now
# riding a lever arm instead of just recolouring in place).
_LEVER_RED = {
    "bezel_outer": "#3A0A0E",
    "bezel_inner": "#5C141A",
    "button_outer": "#A8202C",
    "button_inner": "#F04450",
    "dim": "#5C141A",
    "bright": "#F04450",
    "glow": "#FFB8B8",
}
_LEVER_GREEN = {
    "bezel_outer": "#0E2A14",
    "bezel_inner": "#175024",
    "button_outer": "#1F8A30",
    "button_inner": "#38E983",
    "dim": "#175024",
    "bright": "#38E983",
    "glow": "#D9FFE2",
}


def _gem(d: ImageDraw.ImageDraw, s: float, cx: float, cy: float, r: float, pal: Dict[str, str]) -> None:
    """The original switch gem — bezel + face + glow — at an arbitrary
    centre/radius so it can be mounted as the lever knob."""
    dark = rgba("#04060A")
    bz = r * 0.82
    face = r * 0.58
    d.ellipse(bbox(cx, cy, 2 * r, 2 * r), fill=rgba(pal["bezel_outer"]), outline=dark, width=max(1, int(2 * s)))
    d.ellipse(bbox(cx, cy, 2 * bz, 2 * bz), fill=rgba(pal["bezel_inner"]), outline=rgba(pal["bezel_outer"]), width=max(1, int(2 * s)))
    d.ellipse(bbox(cx, cy, 2 * face, 2 * face), fill=rgba(pal["button_outer"]), outline=dark, width=max(1, int(1 * s)))
    # Inner bright crescent, biased up-left.
    d.ellipse(
        (cx - face + 1 * s, cy - face + 1 * s, cx + face - 4 * s, cy + face - 6 * s),
        fill=rgba(pal["button_inner"], 220),
    )
    # Small specular glow.
    d.ellipse(
        (cx - face + 3 * s, cy - face + 3 * s, cx - face + 9 * s, cy - face + 7 * s),
        fill=rgba(pal["glow"], 200),
    )


def _lever(d: ImageDraw.ImageDraw, s: float, *, left: bool) -> None:
    """Shared draw for a two-position lever switch.

    `left` selects which way the handle is thrown: left → red position
    (armed), right → green position (disabled). The handle, knob colour
    and lit position socket all swap together so the state reads at a
    glance from the lever's *pose*, not just its colour.

    Both poses share an identical alpha bounding box: the symmetric
    mounting housing fixes the horizontal/bottom extent and the knob
    (always at the same height, just mirrored left/right) fixes the top.
    So with `tight_crop=True` the two state sprites crop to the same
    footprint and the switch never appears to shift or stretch when it
    toggles. (No drop shadow — cast shadows are an ECS visual-layer
    concern, not baked into the sprite.)
    """
    dark = rgba("#04060A")
    lw = max(1, int(2 * s))
    pivot = (64 * s, 82 * s)

    # Base housing: a symmetric mounting box the lever sits in. Wide
    # enough to contain both knob throws, so it (not the off-centre
    # handle) defines the crop extent and both poses share one footprint.
    d.rounded_rectangle(
        (24 * s, 78 * s, 104 * s, 114 * s),
        radius=8 * s,
        fill=rgba(_LEVER_STEEL["housing"]),
        outline=dark,
        width=lw,
    )
    # Top bevel highlight on the housing face.
    d.rounded_rectangle(
        (30 * s, 82 * s, 98 * s, 90 * s),
        radius=4 * s,
        fill=rgba(_LEVER_STEEL["housing_hi"]),
    )

    # Two throw sockets, colour-coded; the active side is lit + haloed.
    def socket(cx: float, cy: float, pal: Dict[str, str], lit: bool) -> None:
        r = 7
        d.ellipse(
            bbox(cx * s, cy * s, 2 * r * s, 2 * r * s),
            fill=rgba(pal["bright"] if lit else pal["dim"]),
            outline=dark,
            width=max(1, int(1 * s)),
        )
        if lit:
            d.ellipse(
                bbox(cx * s, cy * s, (2 * r + 10) * s, (2 * r + 10) * s),
                outline=rgba(pal["bright"], 110),
                width=lw,
            )

    socket(40, 100, _LEVER_RED, left)
    socket(88, 100, _LEVER_GREEN, not left)

    # Handle: thick steel shaft from the pivot up to the thrown side.
    knob = (40 * s, 40 * s) if left else (88 * s, 40 * s)
    d.line([pivot, knob], fill=rgba(_LEVER_STEEL["shaft"]), width=int(11 * s), joint="curve")
    d.line([pivot, knob], fill=rgba(_LEVER_STEEL["shaft_hi"]), width=int(4 * s), joint="curve")

    # Pivot hub the shaft rotates about.
    d.ellipse(bbox(64 * s, 82 * s, 20 * s, 20 * s), fill=rgba(_LEVER_STEEL["housing_hi"]), outline=dark, width=lw)
    d.ellipse(bbox(64 * s, 82 * s, 8 * s, 8 * s), fill=dark)

    # Gem mounted as the knob, coloured by the active state.
    _gem(d, s, knob[0], knob[1], 16 * s, _LEVER_RED if left else _LEVER_GREEN)


def switch_armed(d: ImageDraw.ImageDraw, s: float) -> None:
    """Encounter switch in the armed state — touching the trigger will
    start the encounter. The lever is thrown LEFT into the red position
    so the state reads as "active danger / committed action"."""
    _lever(d, s, left=True)


def switch_disabled(d: ImageDraw.ImageDraw, s: float) -> None:
    """Encounter switch in the disabled state — encounter cleared or
    de-armed. The lever is thrown RIGHT into the green position so it
    reads as "safe to walk past / encounter is off"."""
    _lever(d, s, left=False)


def lock_wall_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """32×32 tilable lock-wall texture: vertical bars over a dark
    steel backdrop with a horizontal rivet line at mid-height. Used
    for runtime-inserted lock walls in encounter rooms; tiles
    horizontally and vertically because the bar phase is
    period-aligned to 32 px and the rivet line is internal (not at
    a tile edge)."""
    backdrop_top = rgba("#1A1F2A")
    backdrop_bottom = rgba("#252B3C")
    bar = rgba("#494F60")
    bar_hi = rgba("#6F778C", 200)
    rivet = rgba("#7F8699")
    rivet_dark = rgba("#0E1118")
    # Backdrop gradient (top darker than bottom).
    for y in range(32):
        t = y / 31.0
        r = int(backdrop_top[0] + (backdrop_bottom[0] - backdrop_top[0]) * t)
        g = int(backdrop_top[1] + (backdrop_bottom[1] - backdrop_top[1]) * t)
        b = int(backdrop_top[2] + (backdrop_bottom[2] - backdrop_top[2]) * t)
        d.rectangle((0, y * s, 32 * s, (y + 1) * s), fill=(r, g, b, 255))
    # 4 vertical bars centered at x = 4, 12, 20, 28; bar half-width 2.
    for cx in (4, 12, 20, 28):
        d.rectangle(((cx - 2) * s, 0, (cx + 2) * s, 32 * s), fill=bar)
        # Highlight on the left side of each bar for cylindrical depth.
        d.line(
            [((cx - 1) * s, 0), ((cx - 1) * s, 32 * s)],
            fill=bar_hi,
            width=max(1, int(1 * s)),
        )
    # Rivet line at y=16 (internal, so horizontal tiling stays clean).
    d.line([(0, 16 * s), (32 * s, 16 * s)], fill=rivet_dark, width=max(1, int(1 * s)))
    # Rivet dots on each bar at the line.
    for cx in (4, 12, 20, 28):
        d.ellipse(bbox(cx * s, 16 * s, 3 * s, 3 * s), fill=rivet, outline=rivet_dark)


def water_surface_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """32×32 tilable water-surface ripple. Designed to overlay the
    flat water-body tint that `spawn_water_region` already draws —
    alpha is intentionally low (~0.18 max) so the underlying tint
    still shows through. Two sine waves at slightly different
    frequencies break up the obvious-grid look that a single sine
    would produce, and both have integer periods on 32 px so the
    tile is seamless on both axes."""
    import math

    # Base color: cool blue, mostly transparent.
    base = (140, 215, 240, 28)
    d.rectangle((0, 0, 32 * s, 32 * s), fill=base)
    # Crest highlights at the wave peaks. Sample the wave at each
    # vertical position and draw a faint horizontal band where
    # |amplitude| is high.
    for y in range(32):
        v = y / 32.0
        amp1 = math.sin(v * math.tau * 2.0)
        amp2 = math.sin(v * math.tau * 4.0 + 1.0)
        crest = amp1 * 0.6 + amp2 * 0.4
        if crest > 0.7:
            alpha = int(min(70, (crest - 0.7) * 280))
            d.rectangle((0, y * s, 32 * s, (y + 1) * s), fill=rgba("#E0F0FF", alpha))
    # Diagonal sparkle dots — small bright pixels at half-integer
    # offsets so they spread across the tile and meet cleanly at
    # the seam.
    sparkle = rgba("#FFFFFF", 200)
    for x, y in [(6, 8), (15, 20), (24, 12), (4, 26), (29, 4)]:
        d.ellipse(bbox(x * s, y * s, 2 * s, 2 * s), fill=sparkle)


def morph_ball(d: ImageDraw.ImageDraw, s: float) -> None:
    """Player morph-ball stance. Replaces the Rust-side procedural
    texture in `crate::body_mode` once this PNG ships. Rendered as
    a sphere in the same steel-blue palette as the player robot so
    the morph reads as "the same character, curled up"."""
    cx, cy = 64 * s, 68 * s  # bias slightly down so feet anchor to bottom
    outer_r = 44 * s
    # Ground shadow.
    d.ellipse(bbox(64 * s, 96 * s, 80 * s, 12 * s), fill=(0, 0, 0, 70))
    # Sphere body: dark outer rim with bright inner.
    d.ellipse(
        (cx - outer_r, cy - outer_r, cx + outer_r, cy + outer_r),
        fill=rgba("#3D5C7C"),
        outline=rgba("#0E1828"),
        width=max(1, int(3 * s)),
    )
    inner_r = outer_r - 6 * s
    d.ellipse(
        (cx - inner_r, cy - inner_r, cx + inner_r, cy + inner_r), fill=rgba("#5C8AB8")
    )
    # Equator stripe — visual cue that the body has rolled.
    d.ellipse(
        (cx - inner_r, cy - inner_r * 0.30, cx + inner_r, cy + inner_r * 0.30),
        fill=rgba("#7BA8D6"),
        outline=rgba("#1B2D44"),
        width=max(1, int(1 * s)),
    )
    # Top-left highlight crescent for sphericality.
    hx0 = cx - outer_r + 8 * s
    hy0 = cy - outer_r + 8 * s
    d.ellipse((hx0, hy0, hx0 + 24 * s, hy0 + 16 * s), fill=rgba("#CDE3F5", 200))
    # Central glowing core (the player's "eye" peeking out).
    d.ellipse(bbox(cx, cy, 12 * s, 12 * s), fill=rgba("#0E1828"))
    d.ellipse(bbox(cx, cy, 8 * s, 8 * s), fill=rgba("#82E0FF"))
    d.ellipse(bbox(cx - 1 * s, cy - 1 * s, 4 * s, 4 * s), fill=rgba("#FFFFFF"))


def save_point(d: ImageDraw.ImageDraw, s: float) -> None:
    """Vertical save / checkpoint pillar — a glowing core inside a
    metal frame. Not yet wired into gameplay; reserved for when the
    save system grows beyond auto-save. Cyan glow ties to the
    `pickup_ability` and `edge_exit` palette for "system / data"
    feel."""
    outline = rgba("#0A1A24")
    # Base plate.
    d.rectangle(
        (36 * s, 96 * s, 92 * s, 110 * s),
        fill=rgba("#2A3B4C"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    d.line(
        [(40 * s, 102 * s), (88 * s, 102 * s)],
        fill=rgba("#5A7990"),
        width=max(1, int(1 * s)),
    )
    # Side rails.
    for x in (40, 88):
        d.rectangle(
            ((x - 3) * s, 28 * s, (x + 3) * s, 96 * s),
            fill=rgba("#3D556B"),
            outline=outline,
            width=max(1, int(1 * s)),
        )
    # Top crossbar.
    d.rectangle(
        (36 * s, 22 * s, 92 * s, 30 * s),
        fill=rgba("#2A3B4C"),
        outline=outline,
        width=max(1, int(2 * s)),
    )
    # Glowing core.
    d.rectangle(
        (50 * s, 36 * s, 78 * s, 92 * s),
        fill=rgba("#0B2030"),
        outline=rgba("#43E9FF"),
        width=max(1, int(2 * s)),
    )
    # Inner glow.
    for inset in (4, 7, 10):
        d.rectangle(
            ((50 + inset) * s, (36 + inset) * s, (78 - inset) * s, (92 - inset) * s),
            outline=rgba("#43E9FF", max(40, 200 - inset * 28)),
            width=max(1, int(1 * s)),
        )
    # Cycling chevron marks for "active save point".
    for y in (50, 64, 78):
        d.polygon(
            poly_scaled([(58, y), (64, y - 4), (70, y), (64, y + 4)], s),
            fill=rgba("#82F0FF"),
        )


def ladder_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """16×32 tilable ladder texture: two vertical rails, three rungs.
    Sized to match a single 16-px-wide IntGrid column so the ladder
    appears at native pixel scale when authored as a 1-cell-wide
    column. Tiles vertically because the rung phase is period-32 on
    y and the rails span the full canvas."""
    rail = rgba("#7B5A33")
    rail_dark = rgba("#3A2914")
    rung = rgba("#9A7344")
    rung_hi = rgba("#C49968", 220)
    # Two vertical rails with slight depth shading.
    for cx in (3, 13):
        d.rectangle(((cx - 1) * s, 0, (cx + 1) * s, 32 * s), fill=rail)
        d.line(
            [(cx * s, 0), (cx * s, 32 * s)], fill=rail_hi(), width=max(1, int(1 * s))
        )
        d.line(
            [((cx - 1) * s, 0), ((cx - 1) * s, 32 * s)],
            fill=rail_dark,
            width=max(1, int(1 * s)),
        )
    # Rungs at y = 6, 16, 26 — keeps the rung phase cleanly tilable
    # (16 + 32k for any k is the same y after wrap).
    for y in (6, 16, 26):
        d.rectangle((3 * s, (y - 1) * s, 13 * s, (y + 1) * s), fill=rung)
        d.line(
            [(3 * s, y * s), (13 * s, y * s)], fill=rung_hi, width=max(1, int(1 * s))
        )


def rail_hi() -> Color:
    return rgba("#A38655", 220)


def acid_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """32×16 tilable acid pool surface. Bright neon-green hazard
    surface with bubble highlights so it reads as "liquid that hurts"
    distinct from spike hazards. Same 32×16 sizing rationale as
    `hazard_tile` (most authored hazard rows are 16 px tall)."""
    base = rgba("#1A4A0E")
    surface = rgba("#5BC72E")
    surface_hi = rgba("#B7FF6E")
    bubble = rgba("#E8FFB0", 230)
    # Liquid base.
    d.rectangle((0, 6 * s, 32 * s, 16 * s), fill=base)
    # Surface band.
    d.rectangle((0, 4 * s, 32 * s, 8 * s), fill=surface)
    d.line([(0, 4 * s), (32 * s, 4 * s)], fill=surface_hi, width=max(1, int(1 * s)))
    # Bubbles sitting on the surface — anchored so x=4 and x=28 are
    # safely inside the tile, with the seam between them empty.
    for x, y, r in [(4, 9, 2), (12, 11, 2), (20, 8, 2), (28, 11, 2)]:
        d.ellipse(bbox(x * s, y * s, r * 2 * s, r * 2 * s), fill=bubble)


def lava_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """32×16 tilable lava surface. Red-orange flowing surface with
    ember spots. Hotter palette than `hazard_tile`'s spikes so the
    two read as different hazard categories."""
    base = rgba("#3A0808")
    flow = rgba("#E04416")
    flow_hi = rgba("#FFC04A")
    ember = rgba("#FFE9A8")
    d.rectangle((0, 6 * s, 32 * s, 16 * s), fill=base)
    d.rectangle((0, 4 * s, 32 * s, 8 * s), fill=flow)
    # Surface ridges — a noisy line across the top.
    for x in range(0, 32, 4):
        d.line(
            [(x * s, 4 * s), ((x + 2) * s, 6 * s)],
            fill=flow_hi,
            width=max(1, int(1 * s)),
        )
        d.line(
            [((x + 2) * s, 6 * s), ((x + 4) * s, 4 * s)],
            fill=flow_hi,
            width=max(1, int(1 * s)),
        )
    # Embers floating just above the surface.
    for x, y in [(7, 2), (16, 1), (24, 3), (3, 3), (29, 1)]:
        d.ellipse(bbox(x * s, y * s, 2 * s, 2 * s), fill=ember)


def spike_ball(d: ImageDraw.ImageDraw, s: float) -> None:
    """Iron sphere with radial spikes. Hazard variant for swinging /
    rolling traps — distinct from `hazard_spikes` (a spike strip)
    and `hazard_tile` (a tilable hazard floor). Not yet wired; ships
    so future hazard mechanics have art to consume."""
    import math

    cx, cy = 64 * s, 64 * s
    outer_r = 32 * s
    # Drop shadow.
    d.ellipse(bbox(64 * s, 100 * s, 70 * s, 10 * s), fill=(0, 0, 0, 60))
    # Spike points around the sphere — 12 spikes, each a thin triangle.
    for i in range(12):
        angle = i * (360.0 / 12.0)
        a = math.radians(angle)
        tip_x = cx + math.cos(a) * (outer_r + 12 * s)
        tip_y = cy + math.sin(a) * (outer_r + 12 * s)
        b1_x = cx + math.cos(a + math.pi / 2) * 4 * s + math.cos(a) * outer_r
        b1_y = cy + math.sin(a + math.pi / 2) * 4 * s + math.sin(a) * outer_r
        b2_x = cx + math.cos(a - math.pi / 2) * 4 * s + math.cos(a) * outer_r
        b2_y = cy + math.sin(a - math.pi / 2) * 4 * s + math.sin(a) * outer_r
        d.polygon(
            [(b1_x, b1_y), (tip_x, tip_y), (b2_x, b2_y)],
            fill=rgba("#3A3F4A"),
            outline=rgba("#0A0C12"),
        )
    # Sphere body.
    d.ellipse(
        (cx - outer_r, cy - outer_r, cx + outer_r, cy + outer_r),
        fill=rgba("#4A4F5C"),
        outline=rgba("#0A0C12"),
        width=max(1, int(2 * s)),
    )
    # Dark inner cracks.
    for ang in (30, 110, 200, 290):
        a = math.radians(ang)
        x1 = cx + math.cos(a) * 6 * s
        y1 = cy + math.sin(a) * 6 * s
        x2 = cx + math.cos(a) * 22 * s
        y2 = cy + math.sin(a) * 22 * s
        d.line([(x1, y1), (x2, y2)], fill=rgba("#1A1D26"), width=max(1, int(1 * s)))
    # Top-left highlight.
    d.ellipse(bbox(cx - 12 * s, cy - 12 * s, 18 * s, 14 * s), fill=rgba("#7F8493", 180))


def bg_circuit_tile(d: ImageDraw.ImageDraw, s: float) -> None:
    """32×32 tilable circuit-board background pattern. Designed for
    biome-themed parallax / wall décor in hub-style areas; alpha is
    moderate so it sits beneath the gameplay layer without competing
    with foreground entities. Lines start and end on tile edges so
    horizontal and vertical repeats are seamless."""
    bg = rgba("#0E1A28")
    trace = rgba("#1F4060", 220)
    trace_hi = rgba("#43E9FF", 180)
    pad = rgba("#2A6A92")
    pad_hi = rgba("#82E0FF", 220)
    d.rectangle((0, 0, 32 * s, 32 * s), fill=bg)
    # Horizontal trace mid-height (seamless across vertical tiling).
    d.line([(0, 16 * s), (32 * s, 16 * s)], fill=trace, width=max(1, int(2 * s)))
    d.line([(0, 16 * s), (32 * s, 16 * s)], fill=trace_hi, width=max(1, int(1 * s)))
    # Vertical traces at x=8 and x=24.
    d.line([(8 * s, 0), (8 * s, 16 * s)], fill=trace, width=max(1, int(2 * s)))
    d.line([(24 * s, 16 * s), (24 * s, 32 * s)], fill=trace, width=max(1, int(2 * s)))
    # Solder pads at the trace junctions.
    for px, py in ((8, 16), (24, 16)):
        d.ellipse(bbox(px * s, py * s, 5 * s, 5 * s), fill=pad, outline=trace_hi)
        d.ellipse(bbox(px * s, py * s, 2 * s, 2 * s), fill=pad_hi)
    # Component squares scattered on the trace.
    d.rectangle(
        (14 * s, 14 * s, 20 * s, 18 * s),
        fill=pad,
        outline=trace_hi,
        width=max(1, int(1 * s)),
    )


ENTITY_SPECS: List[EntitySpriteSpec] = [
    EntitySpriteSpec(
        "chest_closed",
        "chest_closed.png",
        "FeatureVisualKind::Chest",
        "ChestClosed",
        "closed treasure chest",
    ),
    EntitySpriteSpec(
        "chest_open",
        "chest_open.png",
        "FeatureVisualKind::Chest",
        "ChestOpened",
        "opened reward chest",
    ),
    EntitySpriteSpec(
        "breakable_intact",
        "breakable_intact.png",
        "FeatureVisualKind::Breakable",
        "BreakableIntact",
        "intact breakable block",
    ),
    EntitySpriteSpec(
        "breakable_cracked",
        "breakable_cracked.png",
        "FeatureVisualKind::Breakable",
        "BreakableCracking",
        "damaged breakable block",
    ),
    EntitySpriteSpec(
        "breakable_broken",
        "breakable_broken.png",
        "FeatureVisualKind::Breakable",
        "BreakableBroken",
        "broken debris state",
    ),
    EntitySpriteSpec(
        "pickup_health",
        "pickup_health.png",
        "FeatureVisualKind::Pickup",
        "PickupKind::Health",
        "health pickup",
    ),
    EntitySpriteSpec(
        "pickup_currency",
        "pickup_currency.png",
        "FeatureVisualKind::Pickup",
        "PickupKind::Currency",
        "currency pickup",
    ),
    EntitySpriteSpec(
        "pickup_ability",
        "pickup_ability.png",
        "FeatureVisualKind::Pickup",
        "PickupKind::Ability",
        "ability pickup",
    ),
    EntitySpriteSpec(
        "hazard_spikes",
        "hazard_spikes.png",
        "FeatureVisualKind::Hazard",
        "DamageVolume/HazardBlock",
        "spike hazard",
    ),
    EntitySpriteSpec(
        "npc_terminal",
        "npc_terminal.png",
        "FeatureVisualKind::Npc",
        "InteractionKind::Npc",
        "talkable terminal NPC",
    ),
    EntitySpriteSpec(
        "boss_core",
        "boss_core.png",
        "FeatureVisualKind::Boss",
        "BossDormant/BossPhase",
        "boss core placeholder",
    ),
    EntitySpriteSpec(
        "sandbag_dummy",
        "sandbag_dummy.png",
        "FeatureVisualKind::Sandbag",
        "sandbag_infinite/sandbag_finite",
        "combat-practice sandbag",
    ),
    EntitySpriteSpec(
        "moving_platform",
        "moving_platform.png",
        "ActorKind::MovingPlatform",
        "MovingPlatformVisual",
        "time-reference moving platform",
    ),
    EntitySpriteSpec(
        "rebound_pad",
        "rebound_pad.png",
        "BlockKind::Rebound",
        "SurfaceContact::Rebound",
        "momentum rebound pad",
    ),
    EntitySpriteSpec(
        "pogo_orb",
        "pogo_orb.png",
        "BlockKind::PogoOrb",
        "SurfaceContact::PogoRefresh",
        "pogo refresh orb",
    ),
    EntitySpriteSpec(
        "soft_blink_wall",
        "soft_blink_wall.png",
        "BlockKind::BlinkWall",
        "BlinkWallTier::Soft",
        "soft blink-passable wall",
    ),
    EntitySpriteSpec(
        "hard_blink_wall",
        "hard_blink_wall.png",
        "BlockKind::BlinkWall",
        "BlinkWallTier::Hard",
        "hard blink wall",
    ),
    EntitySpriteSpec(
        "solid_block",
        "solid_block.png",
        "BlockKind::Solid",
        "SurfaceCollision::Solid",
        "solid room block tile",
    ),
    EntitySpriteSpec(
        "one_way_platform",
        "one_way_platform.png",
        "BlockKind::OneWay",
        "SurfaceCollision::OneWayUp",
        "one-way platform tile",
    ),
    EntitySpriteSpec(
        "door_zone",
        "door_zone.png",
        "LoadingZoneActivation::Door",
        "Door",
        "interior door loading zone",
        ground=True,
    ),
    EntitySpriteSpec(
        "edge_exit",
        "edge_exit.png",
        "LoadingZoneActivation::EdgeExit",
        "EdgeExit",
        "edge-exit loading zone",
    ),
    EntitySpriteSpec(
        "projectile_energy",
        "projectile_energy.png",
        "ActorKind::Projectile",
        "future projectile",
        "small energy projectile placeholder",
    ),
    # Tile sprites: 32×32 seamless textures rendered via Bevy's
    # `Sprite::image_mode = Tiled` so they REPEAT across IntGrid-
    # derived block surfaces (which can be arbitrary aspect ratios)
    # without smearing. `tight_crop=False` keeps the full canvas so
    # the tiling math is straightforward.
    EntitySpriteSpec(
        "solid_tile",
        "solid_tile.png",
        "BlockKind::Solid (IntGrid)",
        "ldtk solid tile",
        "tilable stone-brick floor/wall",
        size=(32, 32),
        tight_crop=False,
    ),
    # one-way / hazard tiles are 32×16 instead of 32×32 because most
    # IntGrid rows for these are exactly one cell (16 px) tall — a
    # 32×32 texture stretched to a 16-tall block would compress its
    # plate / spike pattern. See `one_way_tile`'s docstring for the
    # full Bevy `SpriteImageMode::Tiled` ratio explanation.
    EntitySpriteSpec(
        "one_way_tile",
        "one_way_tile.png",
        "BlockKind::OneWay (IntGrid)",
        "ldtk one-way tile",
        "tilable one-way platform plate",
        size=(32, 16),
        tight_crop=False,
    ),
    EntitySpriteSpec(
        "hazard_tile",
        "hazard_tile.png",
        "BlockKind::Hazard (IntGrid)",
        "ldtk hazard tile",
        "tilable spike strip",
        size=(32, 16),
        tight_crop=False,
    ),
    EntitySpriteSpec(
        "soft_blink_tile",
        "soft_blink_tile.png",
        "BlockKind::BlinkWall::Soft (IntGrid)",
        "ldtk blink-soft tile",
        "tilable soft blink wall",
        size=(32, 32),
        tight_crop=False,
    ),
    EntitySpriteSpec(
        "hard_blink_tile",
        "hard_blink_tile.png",
        "BlockKind::BlinkWall::Hard (IntGrid)",
        "ldtk blink-hard tile",
        "tilable hard blink wall",
        size=(32, 32),
        tight_crop=False,
    ),
    # New sprites: gameplay surfaces that previously rendered as flat
    # colored rectangles (or, in the morph-ball case, a Rust-side
    # procedural texture). All use the standard `tight_crop=True`
    # except the explicitly tilable variants (lock_wall_tile,
    # water_surface_tile, ladder_tile, acid_tile, lava_tile,
    # bg_circuit_tile) which preserve their full canvas for
    # `Sprite::image_mode = Tiled` use.
    EntitySpriteSpec(
        "switch_armed",
        "switch_armed.png",
        "FeatureVisualKind::Switch",
        "Switch armed (red)",
        "encounter switch — armed / will trigger on touch",
    ),
    EntitySpriteSpec(
        "switch_disabled",
        "switch_disabled.png",
        "FeatureVisualKind::Switch",
        "Switch disabled (green)",
        "encounter switch — disabled / encounter cleared",
    ),
    EntitySpriteSpec(
        "morph_ball",
        "morph_ball.png",
        "BodyMode::MorphBall",
        "player morph stance",
        "player curled into a rolling sphere",
    ),
    EntitySpriteSpec(
        "save_point",
        "save_point.png",
        "future SavePoint",
        "checkpoint pillar (planned)",
        "vertical save pillar with glowing core",
    ),
    EntitySpriteSpec(
        "spike_ball",
        "spike_ball.png",
        "future hazard",
        "swinging / rolling iron spike ball",
        "iron sphere with radial spikes",
    ),
    EntitySpriteSpec(
        "lock_wall_tile",
        "lock_wall_tile.png",
        "runtime LockWall",
        "encounter lock-wall barrier",
        "tilable bars-and-rivets lock wall",
        size=(32, 32),
        tight_crop=False,
    ),
    EntitySpriteSpec(
        "water_surface_tile",
        "water_surface_tile.png",
        "WaterRegion overlay",
        "water-surface ripple overlay",
        "tilable wavy ripple overlay for water bodies",
        size=(32, 32),
        tight_crop=False,
    ),
    EntitySpriteSpec(
        "ladder_tile",
        "ladder_tile.png",
        "future ClimbZone",
        "climbable ladder column",
        "tilable ladder column (1 cell wide, 2 cells tall)",
        size=(16, 32),
        tight_crop=False,
    ),
    EntitySpriteSpec(
        "acid_tile",
        "acid_tile.png",
        "BlockKind::Hazard variant",
        "neon-green acid pool",
        "tilable acid surface — hazard variant",
        size=(32, 16),
        tight_crop=False,
    ),
    EntitySpriteSpec(
        "lava_tile",
        "lava_tile.png",
        "BlockKind::Hazard variant",
        "red-orange lava flow",
        "tilable lava surface — hazard variant",
        size=(32, 16),
        tight_crop=False,
    ),
    EntitySpriteSpec(
        "bg_circuit_tile",
        "bg_circuit_tile.png",
        "biome decoration",
        "hub circuit-board parallax",
        "tilable circuit-board pattern for hub backdrops",
        size=(32, 32),
        tight_crop=False,
    ),
]

DRAWERS: Dict[str, Callable[[ImageDraw.ImageDraw, float], None]] = {
    "chest_closed": chest_closed,
    "chest_open": chest_open,
    "breakable_intact": breakable_intact,
    "breakable_cracked": breakable_cracked,
    "breakable_broken": breakable_broken,
    "pickup_health": pickup_health,
    "pickup_currency": pickup_currency,
    "pickup_ability": pickup_ability,
    "hazard_spikes": hazard_spikes,
    "npc_terminal": npc_terminal,
    "boss_core": boss_core,
    "sandbag_dummy": sandbag_dummy,
    "moving_platform": moving_platform,
    "rebound_pad": rebound_pad,
    "pogo_orb": pogo_orb,
    "soft_blink_wall": soft_blink_wall,
    "hard_blink_wall": hard_blink_wall,
    "solid_block": solid_block,
    "one_way_platform": one_way_platform,
    "door_zone": door_zone,
    "edge_exit": edge_exit,
    "projectile_energy": projectile_energy,
    "solid_tile": solid_tile,
    "one_way_tile": one_way_tile,
    "hazard_tile": hazard_tile,
    "soft_blink_tile": soft_blink_tile,
    "hard_blink_tile": hard_blink_tile,
    "switch_armed": switch_armed,
    "switch_disabled": switch_disabled,
    "morph_ball": morph_ball,
    "save_point": save_point,
    "spike_ball": spike_ball,
    "lock_wall_tile": lock_wall_tile,
    "water_surface_tile": water_surface_tile,
    "ladder_tile": ladder_tile,
    "acid_tile": acid_tile,
    "lava_tile": lava_tile,
    "bg_circuit_tile": bg_circuit_tile,
}


def render_entity_sprite(spec: EntitySpriteSpec, supersample: int = 4) -> Image.Image:
    try:
        draw_fn = DRAWERS[spec.key]
    except KeyError as ex:
        raise KeyError(f"no drawer registered for {spec.key!r}") from ex
    return _render_supersampled(
        draw_fn, spec.size, supersample, tight_crop=spec.tight_crop, ground=spec.ground
    )


def build_entity_contact_sheet(
    tiles: List[Tuple[EntitySpriteSpec, Image.Image]],
) -> Image.Image:
    cols = 4
    label_h = 22
    cell_w = 150
    cell_h = 154
    rows = (len(tiles) + cols - 1) // cols
    sheet = Image.new("RGBA", (cols * cell_w, rows * cell_h), (0, 0, 0, 0))
    d = ImageDraw.Draw(sheet)
    fnt = font(12)
    for idx, (spec, img) in enumerate(tiles):
        col = idx % cols
        row = idx // cols
        x = col * cell_w
        y = row * cell_h
        sheet.alpha_composite(img, (x + (cell_w - img.width) // 2, y + label_h))
        label = spec.key[:21]
        d.text((x + 6, y + 4), label, fill=(240, 244, 255, 255), font=fnt)
    return sheet


def write_entity_sprites(
    out_dir: str | Path = Path("assets/entities"), supersample: int = 4
) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs: List[Path] = []
    tiles: List[Tuple[EntitySpriteSpec, Image.Image]] = []
    for spec in ENTITY_SPECS:
        img = render_entity_sprite(spec, supersample=supersample)
        path = out_dir / spec.filename
        img.save(path)
        outputs.append(path)
        tiles.append((spec, img))
    contact = build_entity_contact_sheet(tiles)
    contact_path = out_dir / "entity_contact_sheet.png"
    contact.save(contact_path)
    outputs.append(contact_path)
    manifest = {
        "generated_by": "proc2d_character_lab.entities",
        "frame_width": 128,
        "frame_height": 128,
        "sprites": [asdict(spec) for spec in ENTITY_SPECS],
        "notes": [
            "Individual entity/state sprites are intentionally not forced into character animation rows.",
            "Rust integration can load these optionally and keep the current colored-rectangle fallback.",
        ],
    }
    manifest_path = out_dir / "entity_manifest.yaml"
    with open(manifest_path, "w", encoding="utf8") as file:
        yaml.safe_dump(manifest, file, sort_keys=False)
    outputs.append(manifest_path)
    return outputs


# ---- Tack-on target API -------------------------------------------------------
#
# One module, one target ("entities") that batches every entity sprite
# in `ENTITY_SPECS` into a single output dir. Discovery picks it up via
# the `render()` hook; the explicit `SHEET_FILES` enumerates every
# emitted filename so the default installer copies them all (not the
# `{stem}_spritesheet.{png,yaml,ron}` convention).

TARGET_NAME = "entities"
SHEET_FILES = (
    *[spec.filename for spec in ENTITY_SPECS],
    "entity_contact_sheet.png",
    "entity_manifest.yaml",
)


def render(out_dir: str | Path, **opts) -> List[Path]:
    """Render every entity sprite in ``ENTITY_SPECS`` into ``out_dir``."""
    supersample = int(opts.get("supersample", 4))
    return write_entity_sprites(out_dir, supersample=supersample)
