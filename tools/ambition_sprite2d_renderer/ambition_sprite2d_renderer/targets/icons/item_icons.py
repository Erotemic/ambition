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

try:
    RESAMPLING = Image.Resampling
except AttributeError:  # pragma: no cover
    RESAMPLING = Image

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]


def rgba(value: str, alpha: int = 255) -> Color:
    r, g, b = ImageColor.getrgb(value)
    return (r, g, b, alpha)


def with_alpha(color: Color, alpha: int) -> Color:
    return (color[0], color[1], color[2], alpha)


def bbox(cx: float, cy: float, w: float, h: float) -> Tuple[float, float, float, float]:
    return (cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0)


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
    IconSpec("blink", "ability_blink.png", "movement", "short-range precision teleport", "#72E7FF", "blink"),
    IconSpec("dash", "ability_dash.png", "movement", "quick horizontal burst", "#FFB15E", "dash"),
    IconSpec("double_jump", "ability_double_jump.png", "movement", "extra mid-air jump", "#93FF72", "double_jump"),
    IconSpec("wall_jump", "ability_wall_jump.png", "movement", "kick off vertical surfaces", "#7EA7FF", "wall_jump"),
    IconSpec("ledge_grab", "ability_ledge_grab.png", "movement", "catch and climb ledges", "#9FE66A", "ledge_grab"),
    IconSpec("climb", "ability_climb.png", "movement", "climb ladders and climbable surfaces", "#D8B069", "climb"),
    IconSpec("swim", "ability_swim.png", "movement", "move underwater", "#58D6FF", "swim"),
    IconSpec("fastfall", "ability_fastfall.png", "movement", "drop quickly out of the air", "#B98CFF", "fastfall"),
    IconSpec("hover", "ability_hover.png", "movement", "short hover or jet assist", "#FFE36E", "hover"),
    IconSpec("slash", "ability_slash.png", "combat", "close melee strike", "#C58AFF", "slash"),
    IconSpec("block", "ability_block.png", "combat", "brace against incoming hits", "#C8D7FF", "block"),
    IconSpec("projectile", "ability_projectile.png", "combat", "fire an energy shot", "#6BE9FF", "projectile"),
    IconSpec("charge", "ability_charge.png", "combat", "charge a stronger action", "#FF86D7", "charge"),
    IconSpec("stomp", "ability_stomp.png", "combat", "downward impact attack", "#FF7059", "stomp"),
    IconSpec("interact", "ability_interact.png", "utility", "activate objects and talk", "#FFF18A", "interact"),
    IconSpec("map", "ability_map.png", "utility", "open or expand the map", "#83BDFF", "map"),
    IconSpec("radio", "ability_radio.png", "utility", "set the music radio", "#FF86D7", "radio"),
    IconSpec("health", "item_health.png", "item", "restore health", "#38E983", "health"),
    IconSpec("key", "item_key.png", "item", "unlock a door or gate", "#FFD65A", "key"),
    IconSpec("coin", "item_coin.png", "item", "currency pickup", "#FFD65A", "coin"),
]


def _base(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(bbox(32*s, 54*s, 42*s, 8*s), fill=(0, 0, 0, 48))
    d.rounded_rectangle((8*s, 8*s, 56*s, 56*s), radius=13*s, fill=rgba("#121826"), outline=rgba("#05070D"), width=max(1, int(2*s)))
    d.rounded_rectangle((12*s, 12*s, 52*s, 52*s), radius=10*s, fill=rgba("#1E2940"), outline=with_alpha(accent, 150), width=max(1, int(1.5*s)))
    d.ellipse(bbox(32*s, 32*s, 32*s, 32*s), outline=with_alpha(accent, 62), width=max(1, int(1*s)))


def icon_blink(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for r, a in [(28, 96), (20, 125), (11, 175)]:
        d.ellipse(bbox(32*s, 32*s, r*s, r*s), outline=with_alpha(accent, a), width=max(1, int(1.3*s)))
    d.polygon(scaled([(25,20), (43,32), (31,35), (38,47), (20,34), (32,31)], s), fill=rgba("#FFFFFF", 220), outline=rgba("#05070D"))


def icon_dash(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for y in (23, 32, 41):
        d.line([(13*s, y*s), (30*s, (y-2)*s)], fill=with_alpha(accent, 105), width=max(1, int(2*s)))
    d.polygon(scaled([(26,17), (50,32), (26,47), (31,36), (13,36), (13,28), (31,28)], s), fill=accent, outline=rgba("#05070D"))


def icon_double_jump(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.arc((13*s, 31*s, 33*s, 53*s), start=205, end=18, fill=with_alpha(accent, 180), width=max(1, int(3*s)))
    d.arc((28*s, 12*s, 50*s, 36*s), start=205, end=18, fill=with_alpha(accent, 220), width=max(1, int(3*s)))
    d.polygon(scaled([(45,12), (52,17), (43,22)], s), fill=accent, outline=rgba("#05070D"))
    d.polygon(scaled([(29,31), (36,36), (27,41)], s), fill=accent, outline=rgba("#05070D"))


def icon_wall_jump(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rectangle((13*s, 13*s, 23*s, 52*s), fill=rgba("#566173"), outline=rgba("#05070D"), width=max(1, int(1*s)))
    for y in (20, 31, 42):
        d.line([(14*s, y*s), (22*s, y*s)], fill=rgba("#9AA6BA"), width=max(1, int(1*s)))
    d.line([(25*s, 42*s), (49*s, 20*s)], fill=accent, width=max(1, int(4*s)))
    d.polygon(scaled([(49,20), (44,33), (36,25)], s), fill=accent, outline=rgba("#05070D"))


def icon_ledge_grab(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle((15*s, 15*s, 50*s, 25*s), radius=4*s, fill=rgba("#69758D"), outline=rgba("#05070D"), width=max(1, int(1.5*s)))
    d.line([(27*s, 25*s), (27*s, 45*s)], fill=accent, width=max(1, int(4*s)))
    d.line([(37*s, 25*s), (37*s, 45*s)], fill=accent, width=max(1, int(4*s)))
    d.arc((23*s, 39*s, 41*s, 55*s), start=180, end=360, fill=accent, width=max(1, int(3*s)))


def icon_climb(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for x in (21, 43):
        d.line([(x*s, 13*s), (x*s, 52*s)], fill=rgba("#D8B069"), width=max(1, int(3*s)))
    for y in (20, 31, 42):
        d.line([(20*s, y*s), (44*s, y*s)], fill=accent, width=max(1, int(3*s)))
    d.polygon(scaled([(32,14), (39,23), (25,23)], s), fill=accent, outline=rgba("#05070D"))


def icon_swim(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for y in (38, 46):
        d.arc((11*s, (y-10)*s, 32*s, (y+8)*s), start=180, end=360, fill=with_alpha(accent, 180), width=max(1, int(2*s)))
        d.arc((30*s, (y-10)*s, 53*s, (y+8)*s), start=180, end=360, fill=with_alpha(accent, 180), width=max(1, int(2*s)))
    d.polygon(scaled([(20,24), (38,16), (49,27), (34,31)], s), fill=rgba("#E7FFFF"), outline=rgba("#05070D"))


def icon_fastfall(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.polygon(scaled([(32,51), (18,32), (27,32), (27,14), (37,14), (37,32), (46,32)], s), fill=accent, outline=rgba("#05070D"))
    for x in (18, 46):
        d.line([(x*s, 16*s), (x*s, 40*s)], fill=with_alpha(accent, 95), width=max(1, int(2*s)))


def icon_hover(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(bbox(32*s, 25*s, 25*s, 16*s), fill=rgba("#EFFFFF"), outline=rgba("#05070D"), width=max(1, int(1.5*s)))
    for x in (25, 32, 39):
        d.polygon(scaled([(x,34), (x-4,52), (x+4,52)], s), fill=with_alpha(accent, 190), outline=rgba("#05070D"))


def icon_slash(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.arc((11*s, 10*s, 57*s, 58*s), start=210, end=25, fill=with_alpha(accent, 225), width=max(1, int(6*s)))
    d.polygon(scaled([(43,14), (53,25), (39,24)], s), fill=accent, outline=rgba("#05070D"))
    d.line([(22*s, 43*s), (43*s, 22*s)], fill=rgba("#FFFFFF", 225), width=max(1, int(3*s)))


def icon_block(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.polygon(scaled([(32,11), (50,20), (46,44), (32,53), (18,44), (14,20)], s), fill=accent, outline=rgba("#05070D"))
    d.polygon(scaled([(32,17), (43,23), (40,40), (32,46), (24,40), (21,23)], s), fill=rgba("#EFFFFF", 180))


def icon_projectile(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(bbox(39*s, 31*s, 22*s, 16*s), fill=accent, outline=rgba("#05070D"), width=max(1, int(1.5*s)))
    d.polygon(scaled([(12,32), (30,22), (30,42)], s), fill=with_alpha(accent, 120))
    d.ellipse(bbox(45*s, 27*s, 5*s, 4*s), fill=rgba("#FFFFFF", 220))


def icon_charge(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for i, r in enumerate((33, 24, 14)):
        d.ellipse(bbox(32*s, 32*s, r*s, r*s), outline=with_alpha(accent, 85 + i*50), width=max(1, int(2*s)))
    d.ellipse(bbox(32*s, 32*s, 7*s, 7*s), fill=rgba("#FFFFFF"))


def icon_stomp(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle((24*s, 14*s, 42*s, 41*s), radius=5*s, fill=accent, outline=rgba("#05070D"), width=max(1, int(2*s)))
    d.rounded_rectangle((19*s, 39*s, 47*s, 49*s), radius=4*s, fill=accent, outline=rgba("#05070D"), width=max(1, int(2*s)))
    for x in (18, 32, 46):
        d.line([(x*s, 53*s), ((x+5)*s, 57*s)], fill=with_alpha(accent, 130), width=max(1, int(2*s)))


def icon_interact(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(bbox(30*s, 32*s, 21*s, 21*s), fill=accent, outline=rgba("#05070D"), width=max(1, int(2*s)))
    d.line([(41*s, 23*s), (52*s, 16*s)], fill=accent, width=max(1, int(3*s)))
    d.line([(43*s, 33*s), (56*s, 33*s)], fill=accent, width=max(1, int(3*s)))
    d.line([(40*s, 43*s), (51*s, 51*s)], fill=accent, width=max(1, int(3*s)))


def icon_map(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.polygon(scaled([(15,18), (29,13), (29,47), (15,52)], s), fill=rgba("#EFFFFF"), outline=rgba("#05070D"))
    d.polygon(scaled([(29,13), (43,18), (43,52), (29,47)], s), fill=with_alpha(accent, 190), outline=rgba("#05070D"))
    d.polygon(scaled([(43,18), (53,13), (53,47), (43,52)], s), fill=rgba("#EFFFFF"), outline=rgba("#05070D"))
    d.line([(20*s, 27*s), (25*s, 25*s), (30*s, 32*s), (37*s, 29*s), (49*s, 35*s)], fill=rgba("#05070D"), width=max(1, int(1.5*s)))


def icon_radio(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle((16*s, 25*s, 50*s, 49*s), radius=6*s, fill=rgba("#27364E"), outline=rgba("#05070D"), width=max(1, int(2*s)))
    d.line([(21*s, 25*s), (38*s, 13*s)], fill=accent, width=max(1, int(2*s)))
    d.ellipse(bbox(29*s, 38*s, 12*s, 12*s), fill=accent, outline=rgba("#05070D"), width=max(1, int(1*s)))
    for x in (39, 45):
        d.line([(x*s, 32*s), (x*s, 44*s)], fill=rgba("#EFFFFF", 190), width=max(1, int(1.3*s)))


def icon_health(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle((27*s, 17*s, 37*s, 47*s), radius=3*s, fill=accent, outline=rgba("#05070D"), width=max(1, int(2*s)))
    d.rounded_rectangle((17*s, 27*s, 47*s, 37*s), radius=3*s, fill=accent, outline=rgba("#05070D"), width=max(1, int(2*s)))


def icon_key(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(bbox(25*s, 30*s, 18*s, 18*s), fill=accent, outline=rgba("#05070D"), width=max(1, int(2*s)))
    d.ellipse(bbox(25*s, 30*s, 7*s, 7*s), fill=rgba("#1E2940"))
    d.line([(34*s, 31*s), (53*s, 31*s)], fill=accent, width=max(1, int(5*s)))
    for x in (44, 51):
        d.line([(x*s, 31*s), (x*s, 40*s)], fill=accent, width=max(1, int(4*s)))


def icon_coin(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(bbox(32*s, 32*s, 34*s, 34*s), fill=accent, outline=rgba("#05070D"), width=max(1, int(2*s)))
    d.ellipse(bbox(32*s, 32*s, 22*s, 22*s), outline=rgba("#FFF3A4"), width=max(1, int(2*s)))
    d.rectangle((30*s, 21*s, 34*s, 43*s), fill=rgba("#6E4A12"))


# ---- Wielded-gauntlet icons (sandbox ground / held items) -------------------
# Distinct from the review-only ability icons above: these ARE consumed by the
# runtime (`item_pickup::item_sprite` / `ItemArt`), rendered into `sprites/props/`
# by `write_gauntlet_props`. Each is one strong geometric silhouette so the
# gauntlets read apart on the ground instead of sharing a brown quad.


def icon_shockwave(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for diam, a in [(46, 95), (33, 140), (20, 195)]:
        d.ellipse(bbox(32 * s, 35 * s, diam * s, diam * 0.6 * s), outline=with_alpha(accent, a), width=max(1, int(1.8 * s)))
    d.ellipse(bbox(32 * s, 35 * s, 10 * s, 6 * s), fill=rgba("#FFFFFF", 235), outline=rgba("#05070D"))


def icon_volley(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.ellipse(bbox(19 * s, 32 * s, 9 * s, 9 * s), fill=with_alpha(accent, 130), outline=rgba("#05070D"), width=max(1, int(1.5 * s)))
    for dy in (-11, 0, 11):
        ty = 32 + dy
        d.polygon(scaled([(25, ty - 4), (44, ty - dy * 0.22), (25, ty + 4)], s), fill=accent, outline=rgba("#05070D"))


def icon_beam(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle((15 * s, 29 * s, 50 * s, 35 * s), radius=3 * s, fill=with_alpha(accent, 165), outline=rgba("#05070D"), width=max(1, int(1.5 * s)))
    d.rounded_rectangle((16 * s, 31 * s, 49 * s, 33 * s), radius=1 * s, fill=rgba("#FFFFFF", 235))
    d.polygon(scaled([(12, 26), (21, 32), (12, 38)], s), fill=accent, outline=rgba("#05070D"))


def icon_vortex(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for diam, a0, a1, a in [(44, 20, 230, 110), (30, 130, 340, 150), (16, 240, 90, 205)]:
        d.arc(bbox(32 * s, 32 * s, diam * s, diam * s), a0, a1, fill=with_alpha(accent, a), width=max(1, int(2.4 * s)))
    d.ellipse(bbox(32 * s, 32 * s, 6 * s, 6 * s), fill=rgba("#FFFFFF", 235), outline=rgba("#05070D"))


def icon_sentry(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.rounded_rectangle((19 * s, 39 * s, 45 * s, 50 * s), radius=3 * s, fill=rgba("#1E2940"), outline=rgba("#05070D"), width=max(1, int(2 * s)))
    d.pieslice(bbox(32 * s, 40 * s, 24 * s, 24 * s), 180, 360, fill=accent, outline=rgba("#05070D"), width=max(1, int(2 * s)))
    d.rounded_rectangle((31 * s, 31 * s, 52 * s, 36 * s), radius=1.5 * s, fill=accent, outline=rgba("#05070D"), width=max(1, int(1.5 * s)))
    d.ellipse(bbox(32 * s, 40 * s, 6 * s, 6 * s), fill=rgba("#FFFFFF", 220))


def icon_dive(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    d.polygon(scaled([(22, 14), (32, 28), (42, 14), (42, 24), (32, 38), (22, 24)], s), fill=accent, outline=rgba("#05070D"))
    for x in (25, 32, 39):
        d.line([(x * s, 42 * s), (x * s, 50 * s)], fill=with_alpha(accent, 140), width=max(1, int(2 * s)))


def icon_meteor(d: ImageDraw.ImageDraw, s: float, accent: Color) -> None:
    for i in range(3):
        x0 = 46 - i * 6
        d.line([(x0 * s, (13 + i * 3) * s), ((x0 - 12) * s, (25 + i * 3) * s)], fill=with_alpha(accent, 130), width=max(1, int(2.2 * s)))
    d.ellipse(bbox(28 * s, 41 * s, 15 * s, 15 * s), fill=accent, outline=rgba("#05070D"), width=max(1, int(2 * s)))
    d.ellipse(bbox(25 * s, 38 * s, 5 * s, 5 * s), fill=rgba("#FFFFFF", 215))


GAUNTLET_ICON_SPECS: List[IconSpec] = [
    IconSpec("shockwave", "gauntlet_shockwave.png", "gauntlet", "ground-slam ring", "#FFD166", "shockwave"),
    IconSpec("volley", "gauntlet_volley.png", "gauntlet", "ranged spread shots", "#8AE66A", "volley"),
    IconSpec("beam", "gauntlet_beam.png", "gauntlet", "aimed line lance", "#FF5E5E", "beam"),
    IconSpec("vortex", "gauntlet_vortex.png", "gauntlet", "crowd-control singularity", "#B083FF", "vortex"),
    IconSpec("sentry", "gauntlet_sentry.png", "gauntlet", "deployable turret", "#5E9BFF", "sentry"),
    IconSpec("dive", "gauntlet_dive.png", "gauntlet", "lunging dash strike", "#FF9F45", "dive"),
    IconSpec("meteor", "gauntlet_meteor.png", "gauntlet", "overhead area rain", "#FFC857", "meteor"),
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
}


def render_icon(spec: IconSpec, size: Tuple[int, int] = (64, 64), supersample: int = 4) -> Image.Image:
    s = max(1, int(supersample))
    img = Image.new("RGBA", (size[0] * s, size[1] * s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    accent = rgba(spec.accent)
    _base(d, float(s), accent)
    DRAWERS[spec.drawer](d, float(s), accent)
    return img.resize(size, RESAMPLING.LANCZOS)


def write_icon_contact_sheet(out_dir: Path, icon_paths: List[Path], columns: int = 5) -> Path:
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


def write_item_icons(out_dir: str | Path, *, size: Tuple[int, int] = (64, 64)) -> List[Path]:
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
    manifest_path.write_text(yaml.safe_dump({"icons": manifest}, sort_keys=False), encoding="utf8")
    outputs.append(manifest_path)
    outputs.append(write_icon_contact_sheet(out_dir, icon_paths))
    return outputs


def write_gauntlet_props(out_dir: str | Path, *, size: Tuple[int, int] = (64, 64)) -> List[Path]:
    """Render the wielded-gauntlet ground-item icons into ``out_dir`` (the sandbox
    ``sprites/props/`` dir). Unlike ``write_item_icons`` (the review-only ability
    set), these icons ARE consumed by the runtime via ``item_pickup::item_sprite``
    / ``ItemArt`` — one ``gauntlet_<id>.png`` per wielded gauntlet."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs: List[Path] = []
    for spec in GAUNTLET_ICON_SPECS:
        path = out_dir / spec.filename
        render_icon(spec, size).save(path)
        outputs.append(path)
    return outputs


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
