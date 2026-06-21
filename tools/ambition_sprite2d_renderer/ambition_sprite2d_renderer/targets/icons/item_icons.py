from __future__ import annotations

"""Procedural ability and item icons for Ambition review builds.

The Rust game does not consume these yet; the goal is to keep ability icon art in
one deterministic Python pipeline alongside sprites.  Each icon is deliberately
simple at 64x64: strong silhouette, dark outline, one accent glow, and no text.
"""

import math
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Dict, Iterable, List, Tuple

import yaml
from PIL import Image, ImageColor, ImageDraw
from ambition_sprite2d_renderer.core.draw import rgba, with_alpha, bbox

try:
    RESAMPLING = Image.Resampling
except AttributeError:  # pragma: no cover
    RESAMPLING = Image

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]








def scaled(points: Iterable[Point], s: float) -> List[Point]:
    return [(x * s, y * s) for x, y in points]


@dataclass(frozen=True)
class IconSpec:
    key: str
    filename: str
    category: str
    gameplay_hint: str
    accent: str
    drawer: str


ICON_SPECS: List[IconSpec] = [
    IconSpec(
        "blink",
        "ability_blink.png",
        "movement",
        "short-range precision teleport",
        "#72E7FF",
        "blink",
    ),
    IconSpec(
        "dash",
        "ability_dash.png",
        "movement",
        "quick horizontal burst",
        "#FFB15E",
        "dash",
    ),
    IconSpec(
        "double_jump",
        "ability_double_jump.png",
        "movement",
        "extra mid-air jump",
        "#93FF72",
        "double_jump",
    ),
    IconSpec(
        "wall_jump",
        "ability_wall_jump.png",
        "movement",
        "kick off vertical surfaces",
        "#7EA7FF",
        "wall_jump",
    ),
    IconSpec(
        "ledge_grab",
        "ability_ledge_grab.png",
        "movement",
        "catch and climb ledges",
        "#9FE66A",
        "ledge_grab",
    ),
    IconSpec(
        "climb",
        "ability_climb.png",
        "movement",
        "climb ladders and climbable surfaces",
        "#D8B069",
        "climb",
    ),
    IconSpec(
        "swim", "ability_swim.png", "movement", "move underwater", "#58D6FF", "swim"
    ),
    IconSpec(
        "fastfall",
        "ability_fastfall.png",
        "movement",
        "drop quickly out of the air",
        "#B98CFF",
        "fastfall",
    ),
    IconSpec(
        "hover",
        "ability_hover.png",
        "movement",
        "short hover or jet assist",
        "#FFE36E",
        "hover",
    ),
    IconSpec(
        "slash", "ability_slash.png", "combat", "close melee strike", "#C58AFF", "slash"
    ),
    IconSpec(
        "block",
        "ability_block.png",
        "combat",
        "brace against incoming hits",
        "#C8D7FF",
        "block",
    ),
    IconSpec(
        "projectile",
        "ability_projectile.png",
        "combat",
        "fire an energy shot",
        "#6BE9FF",
        "projectile",
    ),
    IconSpec(
        "charge",
        "ability_charge.png",
        "combat",
        "charge a stronger action",
        "#FF86D7",
        "charge",
    ),
    IconSpec(
        "stomp",
        "ability_stomp.png",
        "combat",
        "downward impact attack",
        "#FF7059",
        "stomp",
    ),
    IconSpec(
        "interact",
        "ability_interact.png",
        "utility",
        "activate objects and talk",
        "#FFF18A",
        "interact",
    ),
    IconSpec(
        "map", "ability_map.png", "utility", "open or expand the map", "#83BDFF", "map"
    ),
    IconSpec(
        "radio",
        "ability_radio.png",
        "utility",
        "set the music radio",
        "#FF86D7",
        "radio",
    ),
    IconSpec(
        "health", "item_health.png", "item", "restore health", "#38E983", "health"
    ),
    IconSpec("key", "item_key.png", "item", "unlock a door or gate", "#FFD65A", "key"),
    IconSpec("coin", "item_coin.png", "item", "currency pickup", "#FFD65A", "coin"),
]


def _base(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(bbox(32 * s, 54 * s, 42 * s, 8 * s), fill=(0, 0, 0, 48))
    d.rounded_rectangle(
        (8 * s, 8 * s, 56 * s, 56 * s),
        radius=13 * s,
        fill=rgba("#121826"),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.rounded_rectangle(
        (12 * s, 12 * s, 52 * s, 52 * s),
        radius=10 * s,
        fill=rgba("#1E2940"),
        outline=with_alpha(accent, 150),
        width=max(1, int(1.5 * s)),
    )
    d.ellipse(
        bbox(32 * s, 32 * s, 32 * s, 32 * s),
        outline=with_alpha(accent, 62),
        width=max(1, int(1 * s)),
    )


def icon_blink(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for r, a in [(28, 96), (20, 125), (11, 175)]:
        d.ellipse(
            bbox(32 * s, 32 * s, r * s, r * s),
            outline=with_alpha(accent, a),
            width=max(1, int(1.3 * s)),
        )
    d.polygon(
        scaled([(25, 20), (43, 32), (31, 35), (38, 47), (20, 34), (32, 31)], s),
        fill=rgba("#FFFFFF", 220),
        outline=rgba("#05070D"),
    )


def icon_dash(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for y in (23, 32, 41):
        d.line(
            [(13 * s, y * s), (30 * s, (y - 2) * s)],
            fill=with_alpha(accent, 105),
            width=max(1, int(2 * s)),
        )
    d.polygon(
        scaled(
            [(26, 17), (50, 32), (26, 47), (31, 36), (13, 36), (13, 28), (31, 28)], s
        ),
        fill=accent,
        outline=rgba("#05070D"),
    )


def icon_double_jump(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.arc(
        (13 * s, 31 * s, 33 * s, 53 * s),
        start=205,
        end=18,
        fill=with_alpha(accent, 180),
        width=max(1, int(3 * s)),
    )
    d.arc(
        (28 * s, 12 * s, 50 * s, 36 * s),
        start=205,
        end=18,
        fill=with_alpha(accent, 220),
        width=max(1, int(3 * s)),
    )
    d.polygon(
        scaled([(45, 12), (52, 17), (43, 22)], s), fill=accent, outline=rgba("#05070D")
    )
    d.polygon(
        scaled([(29, 31), (36, 36), (27, 41)], s), fill=accent, outline=rgba("#05070D")
    )


def icon_wall_jump(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rectangle(
        (13 * s, 13 * s, 23 * s, 52 * s),
        fill=rgba("#566173"),
        outline=rgba("#05070D"),
        width=max(1, int(1 * s)),
    )
    for y in (20, 31, 42):
        d.line(
            [(14 * s, y * s), (22 * s, y * s)],
            fill=rgba("#9AA6BA"),
            width=max(1, int(1 * s)),
        )
    d.line([(25 * s, 42 * s), (49 * s, 20 * s)], fill=accent, width=max(1, int(4 * s)))
    d.polygon(
        scaled([(49, 20), (44, 33), (36, 25)], s), fill=accent, outline=rgba("#05070D")
    )


def icon_ledge_grab(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle(
        (15 * s, 15 * s, 50 * s, 25 * s),
        radius=4 * s,
        fill=rgba("#69758D"),
        outline=rgba("#05070D"),
        width=max(1, int(1.5 * s)),
    )
    d.line([(27 * s, 25 * s), (27 * s, 45 * s)], fill=accent, width=max(1, int(4 * s)))
    d.line([(37 * s, 25 * s), (37 * s, 45 * s)], fill=accent, width=max(1, int(4 * s)))
    d.arc(
        (23 * s, 39 * s, 41 * s, 55 * s),
        start=180,
        end=360,
        fill=accent,
        width=max(1, int(3 * s)),
    )


def icon_climb(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for x in (21, 43):
        d.line(
            [(x * s, 13 * s), (x * s, 52 * s)],
            fill=rgba("#D8B069"),
            width=max(1, int(3 * s)),
        )
    for y in (20, 31, 42):
        d.line(
            [(20 * s, y * s), (44 * s, y * s)], fill=accent, width=max(1, int(3 * s))
        )
    d.polygon(
        scaled([(32, 14), (39, 23), (25, 23)], s), fill=accent, outline=rgba("#05070D")
    )


def icon_swim(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for y in (38, 46):
        d.arc(
            (11 * s, (y - 10) * s, 32 * s, (y + 8) * s),
            start=180,
            end=360,
            fill=with_alpha(accent, 180),
            width=max(1, int(2 * s)),
        )
        d.arc(
            (30 * s, (y - 10) * s, 53 * s, (y + 8) * s),
            start=180,
            end=360,
            fill=with_alpha(accent, 180),
            width=max(1, int(2 * s)),
        )
    d.polygon(
        scaled([(20, 24), (38, 16), (49, 27), (34, 31)], s),
        fill=rgba("#E7FFFF"),
        outline=rgba("#05070D"),
    )


def icon_fastfall(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.polygon(
        scaled(
            [(32, 51), (18, 32), (27, 32), (27, 14), (37, 14), (37, 32), (46, 32)], s
        ),
        fill=accent,
        outline=rgba("#05070D"),
    )
    for x in (18, 46):
        d.line(
            [(x * s, 16 * s), (x * s, 40 * s)],
            fill=with_alpha(accent, 95),
            width=max(1, int(2 * s)),
        )


def icon_hover(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(
        bbox(32 * s, 25 * s, 25 * s, 16 * s),
        fill=rgba("#EFFFFF"),
        outline=rgba("#05070D"),
        width=max(1, int(1.5 * s)),
    )
    for x in (25, 32, 39):
        d.polygon(
            scaled([(x, 34), (x - 4, 52), (x + 4, 52)], s),
            fill=with_alpha(accent, 190),
            outline=rgba("#05070D"),
        )


def icon_slash(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.arc(
        (11 * s, 10 * s, 57 * s, 58 * s),
        start=210,
        end=25,
        fill=with_alpha(accent, 225),
        width=max(1, int(6 * s)),
    )
    d.polygon(
        scaled([(43, 14), (53, 25), (39, 24)], s), fill=accent, outline=rgba("#05070D")
    )
    d.line(
        [(22 * s, 43 * s), (43 * s, 22 * s)],
        fill=rgba("#FFFFFF", 225),
        width=max(1, int(3 * s)),
    )


def icon_block(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.polygon(
        scaled([(32, 11), (50, 20), (46, 44), (32, 53), (18, 44), (14, 20)], s),
        fill=accent,
        outline=rgba("#05070D"),
    )
    d.polygon(
        scaled([(32, 17), (43, 23), (40, 40), (32, 46), (24, 40), (21, 23)], s),
        fill=rgba("#EFFFFF", 180),
    )


def icon_projectile(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(
        bbox(39 * s, 31 * s, 22 * s, 16 * s),
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(1.5 * s)),
    )
    d.polygon(scaled([(12, 32), (30, 22), (30, 42)], s), fill=with_alpha(accent, 120))
    d.ellipse(bbox(45 * s, 27 * s, 5 * s, 4 * s), fill=rgba("#FFFFFF", 220))


def icon_charge(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for i, r in enumerate((33, 24, 14)):
        d.ellipse(
            bbox(32 * s, 32 * s, r * s, r * s),
            outline=with_alpha(accent, 85 + i * 50),
            width=max(1, int(2 * s)),
        )
    d.ellipse(bbox(32 * s, 32 * s, 7 * s, 7 * s), fill=rgba("#FFFFFF"))


def icon_stomp(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle(
        (24 * s, 14 * s, 42 * s, 41 * s),
        radius=5 * s,
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.rounded_rectangle(
        (19 * s, 39 * s, 47 * s, 49 * s),
        radius=4 * s,
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    for x in (18, 32, 46):
        d.line(
            [(x * s, 53 * s), ((x + 5) * s, 57 * s)],
            fill=with_alpha(accent, 130),
            width=max(1, int(2 * s)),
        )


def icon_interact(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(
        bbox(30 * s, 32 * s, 21 * s, 21 * s),
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.line([(41 * s, 23 * s), (52 * s, 16 * s)], fill=accent, width=max(1, int(3 * s)))
    d.line([(43 * s, 33 * s), (56 * s, 33 * s)], fill=accent, width=max(1, int(3 * s)))
    d.line([(40 * s, 43 * s), (51 * s, 51 * s)], fill=accent, width=max(1, int(3 * s)))


def icon_map(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.polygon(
        scaled([(15, 18), (29, 13), (29, 47), (15, 52)], s),
        fill=rgba("#EFFFFF"),
        outline=rgba("#05070D"),
    )
    d.polygon(
        scaled([(29, 13), (43, 18), (43, 52), (29, 47)], s),
        fill=with_alpha(accent, 190),
        outline=rgba("#05070D"),
    )
    d.polygon(
        scaled([(43, 18), (53, 13), (53, 47), (43, 52)], s),
        fill=rgba("#EFFFFF"),
        outline=rgba("#05070D"),
    )
    d.line(
        [
            (20 * s, 27 * s),
            (25 * s, 25 * s),
            (30 * s, 32 * s),
            (37 * s, 29 * s),
            (49 * s, 35 * s),
        ],
        fill=rgba("#05070D"),
        width=max(1, int(1.5 * s)),
    )


def icon_radio(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle(
        (16 * s, 25 * s, 50 * s, 49 * s),
        radius=6 * s,
        fill=rgba("#27364E"),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.line([(21 * s, 25 * s), (38 * s, 13 * s)], fill=accent, width=max(1, int(2 * s)))
    d.ellipse(
        bbox(29 * s, 38 * s, 12 * s, 12 * s),
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(1 * s)),
    )
    for x in (39, 45):
        d.line(
            [(x * s, 32 * s), (x * s, 44 * s)],
            fill=rgba("#EFFFFF", 190),
            width=max(1, int(1.3 * s)),
        )


def icon_health(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle(
        (27 * s, 17 * s, 37 * s, 47 * s),
        radius=3 * s,
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.rounded_rectangle(
        (17 * s, 27 * s, 47 * s, 37 * s),
        radius=3 * s,
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )


def icon_key(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(
        bbox(25 * s, 30 * s, 18 * s, 18 * s),
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.ellipse(bbox(25 * s, 30 * s, 7 * s, 7 * s), fill=rgba("#1E2940"))
    d.line([(34 * s, 31 * s), (53 * s, 31 * s)], fill=accent, width=max(1, int(5 * s)))
    for x in (44, 51):
        d.line(
            [(x * s, 31 * s), (x * s, 40 * s)], fill=accent, width=max(1, int(4 * s))
        )


def icon_coin(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(
        bbox(32 * s, 32 * s, 34 * s, 34 * s),
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.ellipse(
        bbox(32 * s, 32 * s, 22 * s, 22 * s),
        outline=rgba("#FFF3A4"),
        width=max(1, int(2 * s)),
    )
    d.rectangle((30 * s, 21 * s, 34 * s, 43 * s), fill=rgba("#6E4A12"))


# ---- Wielded-gauntlet icons (sandbox ground / held items) -------------------
# Distinct from the review-only ability icons above: these ARE consumed by the
# runtime (`item_pickup::item_sprite` / `ItemArt`), rendered into `sprites/props/`
# by `write_gauntlet_props`. Each is one strong geometric silhouette so the
# gauntlets read apart on the ground instead of sharing a brown quad.


def icon_shockwave(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    # heavy slam-core: a metal orb whose glowing equator throws the ring.
    d.ellipse(
        bbox(32 * s, 34 * s, 30 * s, 30 * s),
        fill=rgba("#3A3F4C"),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.ellipse(bbox(26 * s, 28 * s, 10 * s, 8 * s), fill=rgba("#5A6072", 200))
    d.ellipse(
        bbox(32 * s, 37 * s, 36 * s, 13 * s),
        outline=with_alpha(accent, 235),
        width=max(1, int(2.4 * s)),
    )
    d.ellipse(
        bbox(32 * s, 37 * s, 23 * s, 8 * s),
        outline=with_alpha(accent, 150),
        width=max(1, int(1.6 * s)),
    )
    d.ellipse(
        bbox(32 * s, 34 * s, 7 * s, 7 * s),
        fill=with_alpha(accent, 240),
        outline=rgba("#05070D"),
    )


def icon_volley(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(
        bbox(19 * s, 32 * s, 9 * s, 9 * s),
        fill=with_alpha(accent, 130),
        outline=rgba("#05070D"),
        width=max(1, int(1.5 * s)),
    )
    for dy in (-11, 0, 11):
        ty = 32 + dy
        d.polygon(
            scaled([(25, ty - 4), (44, ty - dy * 0.22), (25, ty + 4)], s),
            fill=accent,
            outline=rgba("#05070D"),
        )


def icon_beam(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle(
        (15 * s, 29 * s, 50 * s, 35 * s),
        radius=3 * s,
        fill=with_alpha(accent, 165),
        outline=rgba("#05070D"),
        width=max(1, int(1.5 * s)),
    )
    d.rounded_rectangle(
        (16 * s, 31 * s, 49 * s, 33 * s), radius=1 * s, fill=rgba("#FFFFFF", 235)
    )
    d.polygon(
        scaled([(12, 26), (21, 32), (12, 38)], s), fill=accent, outline=rgba("#05070D")
    )


def icon_vortex(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    # a singularity gem: dark sphere with the swirl etched on it + a glow halo.
    d.ellipse(
        bbox(32 * s, 32 * s, 40 * s, 40 * s),
        outline=with_alpha(accent, 70),
        width=max(1, int(1.4 * s)),
    )
    d.ellipse(
        bbox(32 * s, 33 * s, 30 * s, 30 * s),
        fill=rgba("#1C1830"),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    for diam, a0, a1, a in [(24, 20, 250, 165), (15, 140, 360, 205)]:
        d.arc(
            bbox(32 * s, 33 * s, diam * s, diam * s),
            a0,
            a1,
            fill=with_alpha(accent, a),
            width=max(1, int(2.2 * s)),
        )
    d.ellipse(bbox(32 * s, 33 * s, 5 * s, 5 * s), fill=rgba("#FFFFFF", 235))


def icon_sentry(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle(
        (19 * s, 39 * s, 45 * s, 50 * s),
        radius=3 * s,
        fill=rgba("#1E2940"),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.pieslice(
        bbox(32 * s, 40 * s, 24 * s, 24 * s),
        180,
        360,
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.rounded_rectangle(
        (31 * s, 31 * s, 52 * s, 36 * s),
        radius=1.5 * s,
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(1.5 * s)),
    )
    d.ellipse(bbox(32 * s, 40 * s, 6 * s, 6 * s), fill=rgba("#FFFFFF", 220))


def icon_dive(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    # a winged dive-dart pointing down (the lunging strike as a thrown blade).
    d.polygon(
        scaled([(26, 24), (16, 19), (26, 33)], s),
        fill=with_alpha(accent, 205),
        outline=rgba("#05070D"),
    )
    d.polygon(
        scaled([(38, 24), (48, 19), (38, 33)], s),
        fill=with_alpha(accent, 205),
        outline=rgba("#05070D"),
    )
    d.polygon(
        scaled([(32, 13), (38, 22), (35, 47), (32, 53), (29, 47), (26, 22)], s),
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.line(
        [(32 * s, 18 * s), (32 * s, 45 * s)],
        fill=rgba("#FFFFFF", 150),
        width=max(1, int(1.4 * s)),
    )


def icon_meteor(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for i in range(3):
        x0 = 46 - i * 6
        d.line(
            [(x0 * s, (13 + i * 3) * s), ((x0 - 12) * s, (25 + i * 3) * s)],
            fill=with_alpha(accent, 130),
            width=max(1, int(2.2 * s)),
        )
    d.ellipse(
        bbox(28 * s, 41 * s, 15 * s, 15 * s),
        fill=accent,
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.ellipse(bbox(25 * s, 38 * s, 5 * s, 5 * s), fill=rgba("#FFFFFF", 215))


# ---- Item-shaped held-item props (physical objects, not icon tiles) ----------
# These render WITHOUT the `_base` panel (see `render_item_object`) so each reads
# as a thing lying on the ground -- a bomb, a hook, a crystal -- rather than a
# symbol on a square. Wired into the runtime exactly like the gauntlet icons
# (one `gauntlet_<id>.png` per held-item id). `accent` is the object's hero hue.


def icon_bomb(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(
        bbox(30 * s, 39 * s, 34 * s, 34 * s),
        fill=rgba("#23262E"),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.ellipse(bbox(23 * s, 32 * s, 10 * s, 7 * s), fill=rgba("#5A6072", 200))
    d.rounded_rectangle(
        (26 * s, 18 * s, 35 * s, 25 * s),
        radius=2 * s,
        fill=rgba("#6B5536"),
        outline=rgba("#05070D"),
        width=max(1, int(1.5 * s)),
    )
    d.arc(
        (30 * s, 7 * s, 49 * s, 25 * s),
        start=120,
        end=300,
        fill=rgba("#9A7B4F"),
        width=max(1, int(2.4 * s)),
    )
    d.ellipse(bbox(45 * s, 11 * s, 8 * s, 8 * s), fill=with_alpha(accent, 235))
    d.ellipse(bbox(45 * s, 11 * s, 3 * s, 3 * s), fill=rgba("#FFFFFF", 240))


def icon_grapple(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.line(
        [(32 * s, 25 * s), (32 * s, 50 * s)], fill=accent, width=max(1, int(3.5 * s))
    )
    d.ellipse(
        bbox(32 * s, 53 * s, 10 * s, 10 * s), outline=accent, width=max(1, int(2.5 * s))
    )
    d.arc(
        (14 * s, 12 * s, 33 * s, 33 * s),
        start=0,
        end=150,
        fill=accent,
        width=max(1, int(3 * s)),
    )
    d.arc(
        (31 * s, 12 * s, 50 * s, 33 * s),
        start=30,
        end=180,
        fill=accent,
        width=max(1, int(3 * s)),
    )
    d.line([(32 * s, 25 * s), (32 * s, 13 * s)], fill=accent, width=max(1, int(3 * s)))
    for tx, ty in [(15, 16), (49, 16), (32, 12)]:
        d.ellipse(
            bbox(tx * s, ty * s, 5 * s, 5 * s),
            fill=rgba("#FFF3D0"),
            outline=rgba("#05070D"),
        )


def icon_gravity_grenade(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(
        bbox(32 * s, 38 * s, 26 * s, 26 * s),
        outline=with_alpha(accent, 90),
        width=max(1, int(1.5 * s)),
    )
    d.rounded_rectangle(
        (23 * s, 26 * s, 41 * s, 50 * s),
        radius=9 * s,
        fill=rgba("#2A2740"),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.rounded_rectangle(
        (27 * s, 17 * s, 37 * s, 27 * s),
        radius=2 * s,
        fill=rgba("#4A4668"),
        outline=rgba("#05070D"),
        width=max(1, int(1.5 * s)),
    )
    for diam, a0, a1 in [(16, 20, 250), (9, 140, 360)]:
        d.arc(
            bbox(32 * s, 38 * s, diam * s, diam * s),
            a0,
            a1,
            fill=with_alpha(accent, 225),
            width=max(1, int(2 * s)),
        )
    d.ellipse(bbox(32 * s, 38 * s, 4 * s, 4 * s), fill=rgba("#FFFFFF", 235))


def icon_mark_recall(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for r, a in [(30, 60), (22, 95)]:
        d.ellipse(
            bbox(32 * s, 34 * s, r * s, r * s),
            outline=with_alpha(accent, a),
            width=max(1, int(1.3 * s)),
        )
    d.polygon(
        scaled([(26, 20), (38, 20), (40, 50), (24, 50)], s),
        fill=rgba("#2E3A40"),
        outline=rgba("#05070D"),
    )
    d.polygon(
        scaled([(32, 27), (37, 34), (32, 41), (27, 34)], s),
        fill=with_alpha(accent, 235),
        outline=rgba("#05070D"),
    )
    d.line(
        [(32 * s, 41 * s), (32 * s, 47 * s)],
        fill=with_alpha(accent, 205),
        width=max(1, int(2 * s)),
    )


def icon_blink_crystal(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.polygon(
        scaled([(32, 13), (45, 32), (32, 55), (19, 32)], s),
        fill=with_alpha(accent, 150),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.polygon(
        scaled([(32, 20), (40, 33), (32, 46), (24, 33)], s),
        fill=with_alpha(accent, 210),
    )
    d.line(
        [(32 * s, 13 * s), (32 * s, 55 * s)],
        fill=rgba("#FFFFFF", 200),
        width=max(1, int(1.5 * s)),
    )
    d.polygon(scaled([(32, 22), (36, 32), (32, 30)], s), fill=rgba("#FFFFFF", 230))
    for gx, gy in [(47, 19), (17, 45)]:
        d.line(
            [((gx - 3) * s, gy * s), ((gx + 3) * s, gy * s)],
            fill=with_alpha(accent, 185),
            width=max(1, int(1.4 * s)),
        )
        d.line(
            [(gx * s, (gy - 3) * s), (gx * s, (gy + 3) * s)],
            fill=with_alpha(accent, 185),
            width=max(1, int(1.4 * s)),
        )


def icon_puppy_slug_gun(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.polygon(
        scaled([(20, 39), (31, 39), (28, 53), (19, 53)], s),
        fill=rgba("#2A2740"),
        outline=rgba("#05070D"),
    )
    d.rounded_rectangle(
        (15 * s, 27 * s, 42 * s, 40 * s),
        radius=4 * s,
        fill=rgba("#3A3550"),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.polygon(
        scaled([(41, 25), (47, 21), (46, 31)], s),
        fill=with_alpha(accent, 230),
        outline=rgba("#05070D"),
    )
    d.polygon(
        scaled([(51, 25), (57, 22), (52, 32)], s),
        fill=with_alpha(accent, 230),
        outline=rgba("#05070D"),
    )
    d.ellipse(
        bbox(47 * s, 33 * s, 17 * s, 15 * s),
        fill=with_alpha(accent, 210),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.ellipse(bbox(44 * s, 31 * s, 3 * s, 3 * s), fill=rgba("#05070D"))
    d.ellipse(bbox(50 * s, 34 * s, 5 * s, 4 * s), fill=rgba("#23262E"))


def icon_fireball(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    # A flaming sphere: soft glow, flame tongues licking up, a hot orange body,
    # a white-hot core. Reads as fire whether held in hand or streaking in flight
    # (no orientation needed -- it's radial), so it doubles as the in-flight shot.
    d.ellipse(bbox(32 * s, 36 * s, 44 * s, 48 * s), fill=with_alpha(accent, 45))
    # Flame tongues crowning the top -- tips well above the body so they read as
    # fire licking up, not a planet. Outer tongues lean outward, paler.
    for fx, fy, fw, fh, a in [
        (32, 5, 14, 30, 220),
        (24, 11, 10, 26, 195),
        (40, 12, 10, 25, 195),
        (17, 19, 9, 20, 160),
        (47, 20, 8, 18, 160),
    ]:
        d.polygon(
            scaled([(fx, fy), (fx + fw / 2, fy + fh), (fx - fw / 2, fy + fh)], s),
            fill=with_alpha(accent, a),
        )
    d.ellipse(
        bbox(32 * s, 39 * s, 34 * s, 38 * s),
        fill=with_alpha(accent, 238),
        outline=rgba("#05070D"),
        width=max(1, int(2 * s)),
    )
    d.ellipse(bbox(32 * s, 41 * s, 23 * s, 25 * s), fill=rgba("#FFB347", 242))
    d.ellipse(bbox(31 * s, 42 * s, 12 * s, 13 * s), fill=rgba("#FFF3D0", 246))
    d.ellipse(bbox(30 * s, 43 * s, 5 * s, 5 * s), fill=rgba("#FFFFFF", 250))


GAUNTLET_ICON_SPECS: List[IconSpec] = [
    IconSpec(
        "shockwave",
        "gauntlet_shockwave.png",
        "gauntlet",
        "ground-slam ring",
        "#FFD166",
        "shockwave",
    ),
    IconSpec(
        "volley",
        "gauntlet_volley.png",
        "gauntlet",
        "ranged spread shots",
        "#8AE66A",
        "volley",
    ),
    IconSpec(
        "beam", "gauntlet_beam.png", "gauntlet", "aimed line lance", "#FF5E5E", "beam"
    ),
    IconSpec(
        "vortex",
        "gauntlet_vortex.png",
        "gauntlet",
        "crowd-control singularity",
        "#B083FF",
        "vortex",
    ),
    IconSpec(
        "sentry",
        "gauntlet_sentry.png",
        "gauntlet",
        "deployable turret",
        "#5E9BFF",
        "sentry",
    ),
    IconSpec(
        "dive",
        "gauntlet_dive.png",
        "gauntlet",
        "lunging dash strike",
        "#FF9F45",
        "dive",
    ),
    IconSpec(
        "meteor",
        "gauntlet_meteor.png",
        "gauntlet",
        "overhead area rain",
        "#FFC857",
        "meteor",
    ),
    IconSpec(
        "bomb", "gauntlet_bomb.png", "held_item", "lobbed timed bomb", "#FFD166", "bomb"
    ),
    IconSpec(
        "grapple",
        "gauntlet_grapple.png",
        "held_item",
        "grappling hook",
        "#C9A24B",
        "grapple",
    ),
    IconSpec(
        "gravity_grenade",
        "gauntlet_gravity_grenade.png",
        "held_item",
        "localized gravity well",
        "#8E7BFF",
        "gravity_grenade",
    ),
    IconSpec(
        "mark_recall",
        "gauntlet_mark_recall.png",
        "held_item",
        "recall beacon",
        "#5ED6C0",
        "mark_recall",
    ),
    IconSpec(
        "blink",
        "gauntlet_blink.png",
        "held_item",
        "short-range teleport shard",
        "#72E7FF",
        "blink_crystal",
    ),
    IconSpec(
        "puppy_slug_gun",
        "gauntlet_puppy_slug_gun.png",
        "held_item",
        "puppy-slug blaster",
        "#FF8FB0",
        "puppy_slug_gun",
    ),
    IconSpec(
        "fireball",
        "gauntlet_fireball.png",
        "held_item",
        "lobbed fire shot",
        "#FF6A2A",
        "fireball",
    ),
]


DRAWERS: Dict[str, Callable[[ImageDraw.ImageDraw, float, Color], None]] = {
    "blink": icon_blink,
    "dash": icon_dash,
    "double_jump": icon_double_jump,
    "wall_jump": icon_wall_jump,
    "ledge_grab": icon_ledge_grab,
    "climb": icon_climb,
    "swim": icon_swim,
    "fastfall": icon_fastfall,
    "hover": icon_hover,
    "slash": icon_slash,
    "block": icon_block,
    "projectile": icon_projectile,
    "charge": icon_charge,
    "stomp": icon_stomp,
    "interact": icon_interact,
    "map": icon_map,
    "radio": icon_radio,
    "health": icon_health,
    "key": icon_key,
    "coin": icon_coin,
    "shockwave": icon_shockwave,
    "volley": icon_volley,
    "beam": icon_beam,
    "vortex": icon_vortex,
    "sentry": icon_sentry,
    "dive": icon_dive,
    "meteor": icon_meteor,
    "bomb": icon_bomb,
    "grapple": icon_grapple,
    "gravity_grenade": icon_gravity_grenade,
    "mark_recall": icon_mark_recall,
    "blink_crystal": icon_blink_crystal,
    "puppy_slug_gun": icon_puppy_slug_gun,
    "fireball": icon_fireball,
}


def render_icon(
    spec: IconSpec, size: Tuple[int, int] = (64, 64), supersample: int = 4
) -> Image.Image:
    s = max(1, int(supersample))
    img = Image.new("RGBA", (size[0] * s, size[1] * s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    accent = rgba(spec.accent)
    _base(d, float(s), accent)
    DRAWERS[spec.drawer](d, float(s), accent)
    return img.resize(size, RESAMPLING.LANCZOS)


def render_item_object(
    spec: IconSpec, size: Tuple[int, int] = (64, 64), supersample: int = 4
) -> Image.Image:
    """Like `render_icon` but WITHOUT the `_base` panel -- the drawer's shape is
    the whole sprite, so it reads as a physical item on the ground rather than a
    symbol on a tile. Used for the wielded held-item ground props. No baked drop
    shadow (the ECS visual layer owns cast shadows; a baked one would shift the
    alpha bbox and float the item)."""
    s = max(1, int(supersample))
    img = Image.new("RGBA", (size[0] * s, size[1] * s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    DRAWERS[spec.drawer](d, float(s), rgba(spec.accent))
    return img.resize(size, RESAMPLING.LANCZOS)


def write_icon_contact_sheet(
    out_dir: Path, icon_paths: List[Path], columns: int = 5
) -> Path:
    thumbs = [Image.open(path).convert("RGBA") for path in icon_paths]
    cell = 80
    rows = max(1, math.ceil(len(thumbs) / columns))
    sheet = Image.new("RGBA", (columns * cell, rows * cell), (18, 20, 28, 255))
    for i, img in enumerate(thumbs):
        x = (i % columns) * cell + 8
        y = (i // columns) * cell + 8
        sheet.alpha_composite(img, (x, y))
    path = out_dir / "ability_icon_contact_sheet.png"
    sheet.save(path)
    return path


def write_item_icons(
    out_dir: str | Path, *, size: Tuple[int, int] = (64, 64)
) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs: List[Path] = []
    icon_paths: List[Path] = []
    manifest = []
    for spec in ICON_SPECS:
        path = out_dir / spec.filename
        render_icon(spec, size).save(path)
        icon_paths.append(path)
        outputs.append(path)
        manifest.append(asdict(spec) | {"width": size[0], "height": size[1]})
    manifest_path = out_dir / "ability_icon_manifest.yaml"
    manifest_path.write_text(
        yaml.safe_dump({"icons": manifest}, sort_keys=False), encoding="utf8"
    )
    outputs.append(manifest_path)
    outputs.append(write_icon_contact_sheet(out_dir, icon_paths))
    return outputs


def write_gauntlet_props(
    out_dir: str | Path, *, size: Tuple[int, int] = (64, 64)
) -> List[Path]:
    """Render the wielded-gauntlet ground-item icons into ``out_dir`` (the sandbox
    ``sprites/props/`` dir). Unlike ``write_item_icons`` (the review-only ability
    set), these icons ARE consumed by the runtime via ``item_pickup::item_sprite``
    / ``ItemArt`` — one ``gauntlet_<id>.png`` per wielded gauntlet."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs: List[Path] = []
    for spec in GAUNTLET_ICON_SPECS:
        path = out_dir / spec.filename
        # Panel-free render: the gauntlets/held-items read as items lying on the
        # ground, not symbols on a square tile (the `_base` panel).
        render_item_object(spec, size).save(path)
        outputs.append(path)
    return outputs


# ---- World props (taller than icons) ------------------------------------------
#
# Not 64x64 symbols: free-standing world prop sprites with their own aspect.
# The shrine moved to `targets/props/shrine.py`; keep a tiny compatibility
# wrapper here so the runtime's existing import path continues to work.

from ..props.shrine import write_shrine_prop


def draw_mark_beacon(d: ImageDraw.ImageDraw, s: float) -> None:
    """A glowing recall beacon: a teal crystal pillar rising in a column of light,
    with the Mark/Recall diamond glyph + concentric rings at its heart. Stands at
    the dropped mark so the player can see where Blink will recall them to."""
    teal = rgba("#5ED6C0")
    outline = rgba("#05070D")
    w = max(1, int(2 * s))
    # Column of light rising to the sky (two nested translucent wedges).
    d.polygon(scaled([(24, 3), (34, 98), (14, 98)], s), fill=with_alpha(teal, 36))
    d.polygon(scaled([(24, 14), (29, 94), (19, 94)], s), fill=with_alpha(teal, 58))
    # Outer glow around the crystal.
    d.ellipse(bbox(24 * s, 66 * s, 42 * s, 86 * s), fill=with_alpha(teal, 30))
    # Base plinth at the mark.
    d.polygon(
        scaled([(12, 105), (36, 105), (32, 96), (16, 96)], s),
        fill=rgba("#22303A"),
        outline=outline,
        width=w,
    )
    # Crystal pillar (tall diamond section) with a brighter inner core.
    d.polygon(
        scaled([(24, 26), (37, 64), (24, 100), (11, 64)], s),
        fill=with_alpha(teal, 150),
        outline=outline,
        width=w,
    )
    d.polygon(
        scaled([(24, 40), (31, 64), (24, 88), (17, 64)], s), fill=with_alpha(teal, 215)
    )
    d.line(
        scaled([(24, 26), (24, 100)], s),
        fill=rgba("#FFFFFF", 170),
        width=max(1, int(1.5 * s)),
    )
    # Concentric recall rings + the bright diamond glyph at the heart.
    for r, a in [(28, 70), (19, 115)]:
        d.ellipse(
            bbox(24 * s, 66 * s, r * s, r * s),
            outline=with_alpha(teal, a),
            width=max(1, int(1.3 * s)),
        )
    d.polygon(
        scaled([(24, 57), (31, 66), (24, 75), (17, 66)], s),
        fill=rgba("#EAFFFB", 240),
        outline=outline,
    )
    # Apex spark crowning the column.
    for r, a in [(8, 95), (4, 185)]:
        d.ellipse(bbox(24 * s, 8 * s, r * s, r * s), fill=with_alpha(teal, a))
    d.ellipse(bbox(24 * s, 8 * s, 3 * s, 3 * s), fill=rgba("#FFFFFF", 235))


def render_mark_beacon(
    size: Tuple[int, int] = (48, 112), supersample: int = 4
) -> Image.Image:
    s = max(1, int(supersample))
    img = Image.new("RGBA", (size[0] * s, size[1] * s), (0, 0, 0, 0))
    draw_mark_beacon(ImageDraw.Draw(img), float(s))
    return img.resize(size, RESAMPLING.LANCZOS)


def write_mark_beacon_prop(
    out_dir: str | Path, *, size: Tuple[int, int] = (48, 112)
) -> Path:
    """Render the Mark/Recall world beacon into ``out_dir`` (the sandbox
    ``sprites/props/`` dir) as ``mark_beacon.png``. Consumed at runtime by
    ``mark_recall::sync_mark_beacon_visual`` -- the persistent marker at the
    dropped recall point. 3:7 aspect; the runtime ``custom_size`` scales it."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / "mark_beacon.png"
    render_mark_beacon(size).save(path)
    return path


# ---- Tack-on target API -------------------------------------------------------
#
# One module, one target ("item_icons") that batches every ability/item
# icon in `ICON_SPECS` into a single output dir.

TARGET_NAME = "item_icons"
SHEET_FILES = (
    *[spec.filename for spec in ICON_SPECS],
    "ability_icon_manifest.yaml",
    "ability_icon_contact_sheet.png",
)


def render(out_dir: str | Path, **opts) -> List[Path]:
    """Render every ability/item icon in ``ICON_SPECS`` into ``out_dir``."""
    size = opts.get("size", (64, 64))
    return write_item_icons(out_dir, size=size)
