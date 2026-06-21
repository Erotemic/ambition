"""Procedural biome background / foreground parallax art.

The sandbox treats these PNGs as generated runtime assets, not source
files.  Regenerate them with::

    python -m ambition_sprite2d_renderer draw-backgrounds \
        --out-dir crates/ambition_gameplay_core/assets/sprites/backgrounds

Each foreground is a 512x512 transparent PNG with sparse, soft edge
motifs.  The runtime stretches it over the visible camera with overscan
and applies a near-camera parallax drift, so the shapes should read as
atmosphere and edge framing rather than gameplay geometry.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Dict, Iterable, List, Tuple

import yaml
from PIL import Image, ImageColor, ImageDraw, ImageFilter
from ambition_sprite2d_renderer.core.draw import rgba, bbox, poly_scaled

try:
    RESAMPLING = Image.Resampling
except AttributeError:  # pragma: no cover
    RESAMPLING = Image

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]
DrawFn = Callable[[ImageDraw.ImageDraw, float], None]


@dataclass(frozen=True)
class BackgroundLayerSpec:
    key: str
    filename: str
    biome: str
    layer: str
    parallax_factor: float
    description: str
    size: Tuple[int, int] = (512, 512)








def soft_layer(
    size: Tuple[int, int], blur: float = 3.0
) -> Tuple[Image.Image, ImageDraw.ImageDraw]:
    layer = Image.new("RGBA", size, (0, 0, 0, 0))
    return layer, ImageDraw.Draw(layer)


def composite_soft(base: Image.Image, layer: Image.Image, blur: float = 2.0) -> None:
    if blur > 0:
        layer = layer.filter(ImageFilter.GaussianBlur(radius=blur))
    base.alpha_composite(layer)


def draw_mist_band(
    d: ImageDraw.ImageDraw, y: float, color: Color, s: float, amp: float = 10.0
) -> None:
    pts: List[Point] = []
    for i in range(0, 17):
        x = i * 32.0
        wave = ((i % 4) - 1.5) * amp
        pts.append((x, y + wave))
    d.line(poly_scaled(pts, s), fill=color, width=max(1, int(8 * s)))


def hub_foreground(d: ImageDraw.ImageDraw, s: float) -> None:
    # Structural silhouettes, mostly framing the top/left/right edges.
    shadow = rgba("#080817", 86)
    brace = rgba("#16132D", 82)
    blue = rgba("#1B2945", 56)
    d.polygon(
        poly_scaled(
            [(0, 0), (512, 0), (512, 34), (380, 25), (250, 42), (80, 28), (0, 48)], s
        ),
        fill=shadow,
    )
    for x in (42, 438):
        d.line(
            poly_scaled([(x, 0), (x + 22, 126)], s),
            fill=brace,
            width=max(2, int(7 * s)),
        )
        d.line(
            poly_scaled([(x + 35, 0), (x - 14, 142)], s),
            fill=rgba("#0E0B22", 70),
            width=max(2, int(4 * s)),
        )
    d.line(
        poly_scaled([(0, 430), (112, 404), (188, 424)], s),
        fill=blue,
        width=max(2, int(9 * s)),
    )
    d.line(
        poly_scaled([(342, 424), (430, 404), (512, 418)], s),
        fill=blue,
        width=max(2, int(9 * s)),
    )
    # A small broken sign silhouette near a corner; not centered enough to read as interactive.
    d.rounded_rectangle(
        (398 * s, 72 * s, 500 * s, 104 * s), radius=4 * s, fill=rgba("#111428", 58)
    )
    d.line(
        poly_scaled([(386, 38), (412, 82)], s),
        fill=rgba("#101226", 64),
        width=max(1, int(3 * s)),
    )


def lab_foreground(d: ImageDraw.ImageDraw, s: float) -> None:
    cable = rgba("#07101A", 92)
    pipe = rgba("#0B1522", 76)
    teal = rgba("#42F2E7", 45)
    glass = rgba("#D4FFFF", 28)
    # Side pipes and glass panel edges.
    d.rounded_rectangle((0, 50 * s, 26 * s, 454 * s), radius=8 * s, fill=pipe)
    d.rounded_rectangle((484 * s, 30 * s, 512 * s, 470 * s), radius=8 * s, fill=pipe)
    d.line(poly_scaled([(28, 62), (28, 442)], s), fill=teal, width=max(1, int(2 * s)))
    d.line(poly_scaled([(482, 80), (482, 430)], s), fill=teal, width=max(1, int(2 * s)))
    d.polygon(poly_scaled([(40, 0), (82, 0), (56, 188), (24, 180)], s), fill=glass)
    d.polygon(poly_scaled([(430, 0), (462, 0), (492, 178), (460, 190)], s), fill=glass)
    # Loose cables droop from the top edge. Curves stay thin and sparse.
    for x, sag, end in [(120, 56, 216), (175, 88, 170), (336, 70, 230), (390, 48, 155)]:
        pts = [(x, 0), (x - 14, sag), (x + 10, sag + 32), (x - 4, end)]
        d.line(poly_scaled(pts, s), fill=cable, width=max(1, int(4 * s)), joint="curve")
    d.line(
        poly_scaled([(0, 486), (512, 474)], s),
        fill=rgba("#1DD8D2", 25),
        width=max(1, int(8 * s)),
    )


def basement_foreground(d: ImageDraw.ImageDraw, s: float) -> None:
    stone = rgba("#0F0B12", 104)
    stone2 = rgba("#211826", 76)
    warm = rgba("#FFB04A", 55)
    # Large broken pillar edge on one side; center remains open.
    d.polygon(
        poly_scaled(
            [
                (0, 0),
                (76, 0),
                (60, 92),
                (88, 162),
                (70, 250),
                (92, 340),
                (64, 512),
                (0, 512),
            ],
            s,
        ),
        fill=stone,
    )
    d.polygon(
        poly_scaled(
            [(432, 132), (512, 110), (512, 512), (460, 512), (446, 382), (470, 300)], s
        ),
        fill=rgba("#120D15", 88),
    )
    # Cracked arch silhouette near the top.
    d.arc(
        (70 * s, -98 * s, 442 * s, 238 * s),
        start=188,
        end=350,
        fill=stone2,
        width=max(3, int(14 * s)),
    )
    # Bottom foreground slabs.
    d.polygon(
        poly_scaled(
            [(0, 472), (118, 452), (204, 482), (512, 458), (512, 512), (0, 512)], s
        ),
        fill=rgba("#161016", 92),
    )
    for p in [
        ((44, 370), (58, 382)),
        ((470, 248), (484, 258)),
        ((130, 456), (146, 461)),
    ]:
        d.line(poly_scaled(p, s), fill=warm, width=max(1, int(2 * s)))


def cove_foreground(d: ImageDraw.ImageDraw, s: float) -> None:
    reed = rgba("#071413", 86)
    rope = rgba("#120D10", 72)
    palm = rgba("#062018", 70)
    mist = rgba("#BFEFE8", 24)
    # Reeds only in bottom corners.
    for base_x in list(range(8, 118, 17)) + list(range(404, 508, 18)):
        h = 58 + (base_x % 41)
        lean = -12 if base_x < 200 else 10
        d.line(
            poly_scaled([(base_x, 512), (base_x + lean, 512 - h)], s),
            fill=reed,
            width=max(1, int(3 * s)),
        )
        d.polygon(
            poly_scaled(
                [
                    (base_x + lean, 512 - h),
                    (base_x + lean + 8, 512 - h + 18),
                    (base_x + lean - 3, 512 - h + 13),
                ],
                s,
            ),
            fill=reed,
        )
    # Top palm fronds entering from edges.
    d.polygon(
        poly_scaled(
            [(0, 0), (126, 0), (60, 26), (158, 44), (42, 46), (112, 76), (0, 60)], s
        ),
        fill=palm,
    )
    d.polygon(
        poly_scaled(
            [(512, 0), (392, 0), (458, 30), (358, 46), (470, 50), (404, 78), (512, 62)],
            s,
        ),
        fill=palm,
    )
    # Rope/rigging diagonals hug the side/top.
    d.line(poly_scaled([(0, 126), (144, 38)], s), fill=rope, width=max(1, int(4 * s)))
    d.line(poly_scaled([(512, 102), (372, 28)], s), fill=rope, width=max(1, int(4 * s)))
    draw_mist_band(d, 438, mist, s, amp=5.0)
    draw_mist_band(d, 474, rgba("#E0FFFF", 18), s, amp=4.0)


def skybridge_foreground(d: ImageDraw.ImageDraw, s: float) -> None:
    haze = rgba("#FFFFFF", 26)
    streak = rgba("#DDEEFF", 38)
    cable = rgba("#21314C", 48)
    # Soft bright haze at edges, no strong occluding silhouettes.
    d.ellipse(bbox(-10 * s, 256 * s, 120 * s, 620 * s), fill=haze)
    d.ellipse(bbox(522 * s, 256 * s, 120 * s, 620 * s), fill=haze)
    for y in (82, 164, 360, 430):
        d.line(
            poly_scaled([(30, y), (220, y - 24), (404, y - 12)], s),
            fill=streak,
            width=max(1, int(3 * s)),
        )
    d.line(poly_scaled([(0, 170), (512, 88)], s), fill=cable, width=max(1, int(2 * s)))
    d.line(
        poly_scaled([(0, 316), (512, 250)], s),
        fill=rgba("#23365C", 38),
        width=max(1, int(2 * s)),
    )


def boss_foreground(d: ImageDraw.ImageDraw, s: float) -> None:
    dark = rgba("#090006", 92)
    shard = rgba("#7B0C16", 62)
    ember = rgba("#FF5B2E", 70)
    # Extreme edge spikes only; boss tells in the center stay clear.
    for x in range(0, 512, 42):
        h = 18 + (x % 5) * 8
        d.polygon(
            poly_scaled([(x, 512), (x + 20, 512 - h), (x + 40, 512)], s), fill=dark
        )
    for x in range(22, 512, 62):
        h = 15 + (x % 4) * 7
        d.polygon(
            poly_scaled([(x, 0), (x + 16, h), (x + 34, 0)], s), fill=rgba("#090006", 70)
        )
    for pts in [
        [(450, 82), (500, 112), (476, 160)],
        [(14, 382), (72, 356), (52, 426)],
        [(414, 436), (492, 420), (458, 486)],
    ]:
        d.polygon(poly_scaled(pts, s), fill=shard)
    for x, y in [(105, 450), (384, 74), (468, 384), (58, 118)]:
        d.ellipse(bbox(x * s, y * s, 4 * s, 4 * s), fill=ember)


def water_foreground(d: ImageDraw.ImageDraw, s: float) -> None:
    kelp = rgba("#061E1F", 78)
    haze = rgba("#76E7D6", 24)
    bubble = rgba("#C6FFFF", 50)
    caustic = rgba("#9DFFF0", 34)
    # Side kelp curtains, sparse and outside the central play lane.
    for base_x in (18, 42, 72, 454, 484, 506):
        lean = 18 if base_x < 200 else -18
        pts = [
            (base_x, 512),
            (base_x + lean, 420),
            (base_x - lean * 0.2, 330),
            (base_x + lean * 0.5, 250),
        ]
        d.line(poly_scaled(pts, s), fill=kelp, width=max(2, int(7 * s)), joint="curve")
        d.ellipse(
            bbox((base_x + lean) * s, 408 * s, 16 * s, 38 * s), fill=rgba("#08302D", 56)
        )
    draw_mist_band(d, 108, caustic, s, amp=8.0)
    draw_mist_band(d, 210, rgba("#B7FFF2", 24), s, amp=6.0)
    d.rectangle((0, 0, 512 * s, 512 * s), fill=haze)
    for x, y, r in [
        (110, 430, 5),
        (126, 392, 3),
        (388, 356, 4),
        (404, 314, 3),
        (456, 214, 5),
    ]:
        d.ellipse(
            bbox(x * s, y * s, r * s, r * s), outline=bubble, width=max(1, int(1 * s))
        )


def cave_foreground(d: ImageDraw.ImageDraw, s: float) -> None:
    rock = rgba("#05070B", 100)
    rock2 = rgba("#11131C", 76)
    drip = rgba("#7EA5B7", 48)
    # Top-edge stalactites and rock lips.
    d.polygon(
        poly_scaled(
            [
                (0, 0),
                (512, 0),
                (512, 28),
                (432, 24),
                (402, 92),
                (372, 24),
                (304, 32),
                (284, 128),
                (250, 36),
                (156, 26),
                (134, 102),
                (108, 26),
                (0, 40),
            ],
            s,
        ),
        fill=rock,
    )
    # Side contours and corner darkness.
    d.polygon(
        poly_scaled([(0, 50), (38, 64), (26, 192), (52, 318), (28, 512), (0, 512)], s),
        fill=rock2,
    )
    d.polygon(
        poly_scaled(
            [(512, 42), (472, 72), (488, 210), (456, 346), (484, 512), (512, 512)], s
        ),
        fill=rock2,
    )
    d.ellipse(bbox(0 * s, 512 * s, 260 * s, 120 * s), fill=rgba("#020308", 80))
    d.ellipse(bbox(512 * s, 512 * s, 260 * s, 120 * s), fill=rgba("#020308", 80))
    for x, h in [(118, 38), (286, 50), (405, 34)]:
        d.line(
            poly_scaled([(x, 72), (x - 3, 72 + h)], s),
            fill=drip,
            width=max(1, int(2 * s)),
        )


DRAWERS: Dict[str, DrawFn] = {
    "hub_foreground": hub_foreground,
    "lab_foreground": lab_foreground,
    "basement_foreground": basement_foreground,
    "cove_foreground": cove_foreground,
    "skybridge_foreground": skybridge_foreground,
    "boss_foreground": boss_foreground,
    "water_foreground": water_foreground,
    "cave_foreground": cave_foreground,
}


BACKGROUND_LAYER_SPECS: List[BackgroundLayerSpec] = [
    BackgroundLayerSpec(
        "hub_foreground",
        "hub_foreground.png",
        "hub",
        "foreground",
        1.08,
        "dark violet structural edge silhouettes",
    ),
    BackgroundLayerSpec(
        "lab_foreground",
        "lab_foreground.png",
        "lab",
        "foreground",
        1.10,
        "cables, pipes, dim glass and teal instrument lines",
    ),
    BackgroundLayerSpec(
        "basement_foreground",
        "basement_foreground.png",
        "basement",
        "foreground",
        1.12,
        "broken pillars, cracked arch silhouettes, dusty slabs",
    ),
    BackgroundLayerSpec(
        "cove_foreground",
        "cove_foreground.png",
        "cove",
        "foreground",
        1.06,
        "reeds, rigging, palm edges and low sea mist",
    ),
    BackgroundLayerSpec(
        "skybridge_foreground",
        "skybridge_foreground.png",
        "skybridge",
        "foreground",
        1.04,
        "wispy bright haze, wind streaks and thin bridge cables",
    ),
    BackgroundLayerSpec(
        "boss_foreground",
        "boss_foreground.png",
        "boss",
        "foreground",
        1.05,
        "very sparse spikes, red shards and embers at extreme edges",
    ),
    BackgroundLayerSpec(
        "water_foreground",
        "water_foreground.png",
        "water",
        "foreground",
        1.08,
        "side kelp, bubble trails, shimmer bands and blue-green haze",
    ),
    BackgroundLayerSpec(
        "cave_foreground",
        "cave_foreground.png",
        "cave",
        "foreground",
        1.10,
        "stalactites, rock contours, drips and corner darkness",
    ),
]


def render_background_layer(
    spec: BackgroundLayerSpec, supersample: int = 2
) -> Image.Image:
    try:
        draw_fn = DRAWERS[spec.key]
    except KeyError as ex:
        raise KeyError(f"no background drawer registered for {spec.key!r}") from ex

    s = max(1, int(supersample))
    size = (spec.size[0] * s, spec.size[1] * s)
    hard = Image.new("RGBA", size, (0, 0, 0, 0))
    draw_fn(ImageDraw.Draw(hard), float(s))

    # A light whole-layer blur keeps foreground art atmospheric. It is
    # intentionally weaker than full defocus so cables/reeds still read as
    # edge texture when stretched over the viewport.
    soft = hard.filter(ImageFilter.GaussianBlur(radius=max(0.0, 1.15 * s)))
    return soft.resize(spec.size, RESAMPLING.LANCZOS)


def build_background_contact_sheet(
    tiles: List[Tuple[BackgroundLayerSpec, Image.Image]],
) -> Image.Image:
    cols = 4
    label_h = 28
    cell = 154
    rows = (len(tiles) + cols - 1) // cols
    sheet = Image.new("RGBA", (cols * cell, rows * (cell + label_h)), (10, 12, 18, 255))
    d = ImageDraw.Draw(sheet)
    for idx, (spec, img) in enumerate(tiles):
        col = idx % cols
        row = idx // cols
        x = col * cell
        y = row * (cell + label_h)
        thumb = img.resize((128, 128), RESAMPLING.LANCZOS)
        sheet.alpha_composite(thumb, (x + 13, y + label_h))
        d.text((x + 6, y + 7), spec.key[:22], fill=(230, 238, 255, 255))
    return sheet


def write_background_layers(out_dir: str | Path, supersample: int = 2) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs: List[Path] = []
    tiles: List[Tuple[BackgroundLayerSpec, Image.Image]] = []
    for spec in BACKGROUND_LAYER_SPECS:
        img = render_background_layer(spec, supersample=supersample)
        path = out_dir / spec.filename
        img.save(path)
        outputs.append(path)
        tiles.append((spec, img))

    contact = build_background_contact_sheet(tiles)
    contact_path = out_dir / "background_contact_sheet.png"
    contact.save(contact_path)
    outputs.append(contact_path)

    manifest = {
        "generated_by": "ambition_sprite2d_renderer.authoring.backgrounds",
        "notes": [
            "Generated PNGs are runtime artifacts and should not be checked in.",
            "Foreground layers are sparse, transparent edge framing for camera-relative parallax.",
        ],
        "layers": [asdict(spec) for spec in BACKGROUND_LAYER_SPECS],
    }
    manifest_path = out_dir / "background_manifest.yaml"
    with open(manifest_path, "w", encoding="utf8") as file:
        yaml.safe_dump(manifest, file, sort_keys=False)
    outputs.append(manifest_path)
    return outputs
