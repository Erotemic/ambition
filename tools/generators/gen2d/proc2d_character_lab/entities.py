from __future__ import annotations

from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Dict, Iterable, List, Tuple

import yaml
from PIL import Image, ImageColor, ImageDraw, ImageFont

try:
    RESAMPLING = Image.Resampling
except AttributeError:  # pragma: no cover
    RESAMPLING = Image

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


def rgba(hex_color: str, alpha: int = 255) -> Color:
    r, g, b = ImageColor.getrgb(hex_color)
    return (r, g, b, alpha)


def with_alpha(color: Color, alpha: int) -> Color:
    return (color[0], color[1], color[2], alpha)


def bbox(cx: float, cy: float, w: float, h: float) -> Tuple[float, float, float, float]:
    return (cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0)


def font(size: int):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=max(8, int(size)))
        except OSError:
            pass
    return ImageFont.load_default()


def downsample(img: Image.Image, size: Tuple[int, int]) -> Image.Image:
    return img.resize(size, RESAMPLING.LANCZOS)


def render(draw_fn: Callable[[ImageDraw.ImageDraw, float], None], size: Tuple[int, int] = (128, 128), supersample: int = 4) -> Image.Image:
    s = max(1, int(supersample))
    img = Image.new("RGBA", (size[0] * s, size[1] * s), (0, 0, 0, 0))
    draw_fn(ImageDraw.Draw(img), float(s))
    return downsample(img, size)


def poly_scaled(points: Iterable[Point], s: float) -> List[Point]:
    return [(x * s, y * s) for x, y in points]


def draw_gem(d: ImageDraw.ImageDraw, center: Point, radius: float, color: Color, outline: Color, s: float) -> None:
    cx, cy = center
    pts = [(cx, cy - radius), (cx + radius * 0.85, cy - radius * 0.10), (cx + radius * 0.55, cy + radius), (cx - radius * 0.55, cy + radius), (cx - radius * 0.85, cy - radius * 0.10)]
    d.polygon(poly_scaled(pts, s), fill=color, outline=outline)
    d.line(poly_scaled([(cx, cy - radius), (cx, cy + radius), (cx + radius * 0.85, cy - radius * 0.10)], s), fill=with_alpha((255, 255, 255, 255), 110), width=max(1, int(1.2 * s)))


def chest_closed(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#221714")
    d.ellipse(bbox(64*s, 92*s, 74*s, 15*s), fill=(0, 0, 0, 45))
    d.rounded_rectangle((28*s, 52*s, 100*s, 91*s), radius=8*s, fill=rgba("#8D4B22"), outline=outline, width=max(1, int(2*s)))
    d.rounded_rectangle((25*s, 42*s, 103*s, 66*s), radius=12*s, fill=rgba("#C98231"), outline=outline, width=max(1, int(2*s)))
    d.rectangle((28*s, 63*s, 100*s, 70*s), fill=rgba("#F0B84A"), outline=outline, width=max(1, int(1*s)))
    d.rounded_rectangle((56*s, 58*s, 72*s, 78*s), radius=3*s, fill=rgba("#FFE477"), outline=outline, width=max(1, int(1*s)))


def chest_open(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#221714")
    d.ellipse(bbox(64*s, 94*s, 78*s, 15*s), fill=(0, 0, 0, 45))
    d.polygon(poly_scaled([(31,56), (64,33), (97,56), (92,67), (64,51), (36,67)], s), fill=rgba("#D78B34"), outline=outline)
    d.rounded_rectangle((28*s, 62*s, 100*s, 92*s), radius=8*s, fill=rgba("#8D4B22"), outline=outline, width=max(1, int(2*s)))
    for x in [47, 61, 76]:
        d.line([(x*s, 59*s), (x*s, 32*s)], fill=rgba("#FFF18A", 120), width=max(1, int(2*s)))
    draw_gem(d, (64, 63), 11, rgba("#6BE9FF"), outline, s)


def breakable_intact(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#241714")
    d.rounded_rectangle((27*s, 38*s, 101*s, 92*s), radius=6*s, fill=rgba("#8A5736"), outline=outline, width=max(1, int(2*s)))
    for x in (45, 70, 92):
        d.line([(x*s, 41*s), ((x-7)*s, 90*s)], fill=rgba("#B98255"), width=max(1, int(2*s)))
    d.line([(28*s, 64*s), (101*s, 61*s)], fill=rgba("#5C3928"), width=max(1, int(3*s)))


def breakable_cracked(d: ImageDraw.ImageDraw, s: float) -> None:
    breakable_intact(d, s)
    d.line(poly_scaled([(63, 39), (58, 55), (66, 62), (55, 75), (60, 92)], s), fill=rgba("#130B0A"), width=max(1, int(2*s)))
    d.line(poly_scaled([(66, 62), (83, 70), (95, 87)], s), fill=rgba("#130B0A"), width=max(1, int(1.5*s)))


def breakable_broken(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#241714")
    shards = [[(34,74),(55,58),(58,91),(30,93)], [(57,50),(79,47),(72,82),(49,72)], [(82,62),(101,72),(91,93),(73,86)]]
    for pts in shards:
        d.polygon(poly_scaled(pts, s), fill=rgba("#8A5736"), outline=outline)
    d.ellipse(bbox(65*s, 94*s, 75*s, 12*s), fill=(0,0,0,45))


def pickup_health(d: ImageDraw.ImageDraw, s: float) -> None:
    draw_gem(d, (64, 58), 28, rgba("#38E983"), rgba("#0C2A1C"), s)
    d.rounded_rectangle((57*s, 43*s, 71*s, 73*s), radius=3*s, fill=rgba("#FFFFFF"))
    d.rounded_rectangle((49*s, 51*s, 79*s, 65*s), radius=3*s, fill=rgba("#FFFFFF"))
    d.ellipse(bbox(64*s, 93*s, 43*s, 10*s), fill=(0,0,0,36))


def pickup_currency(d: ImageDraw.ImageDraw, s: float) -> None:
    d.ellipse(bbox(64*s, 61*s, 50*s, 50*s), fill=rgba("#FFD65A"), outline=rgba("#5C4112"), width=max(1, int(2*s)))
    d.ellipse(bbox(64*s, 61*s, 34*s, 34*s), outline=rgba("#FFF3A4"), width=max(1, int(3*s)))
    d.text((57*s, 45*s), "$", fill=rgba("#5C4112"), font=font(int(28*s)))
    d.ellipse(bbox(64*s, 94*s, 42*s, 10*s), fill=(0,0,0,34))


def pickup_ability(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#0B1930")
    d.ellipse(bbox(64*s, 62*s, 52*s, 52*s), fill=rgba("#1A2452"), outline=outline, width=max(1, int(2*s)))
    for r,a in [(45,70),(33,95),(22,135)]:
        d.ellipse(bbox(64*s, 62*s, r*s, r*s), outline=rgba("#6BE9FF", a), width=max(1, int(1.4*s)))
    d.polygon(poly_scaled([(56,44),(80,62),(62,65),(72,84),(48,63),(66,60)], s), fill=rgba("#B98CFF"), outline=outline)


def hazard_spikes(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#24060B")
    d.ellipse(bbox(64*s, 92*s, 80*s, 13*s), fill=(0,0,0,40))
    for i in range(5):
        x = 28 + i*18
        d.polygon(poly_scaled([(x,91),(x+10,39),(x+20,91)], s), fill=rgba("#F04450"), outline=outline)
        d.polygon(poly_scaled([(x+8,55),(x+10,39),(x+12,57)], s), fill=rgba("#FFB1B5"))


def npc_terminal(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#101820")
    d.rounded_rectangle((39*s, 31*s, 89*s, 90*s), radius=8*s, fill=rgba("#27364E"), outline=outline, width=max(1, int(2*s)))
    d.rounded_rectangle((45*s, 38*s, 83*s, 63*s), radius=5*s, fill=rgba("#07131E"), outline=outline)
    d.ellipse(bbox(55*s, 51*s, 7*s, 10*s), fill=rgba("#6BE9FF"))
    d.ellipse(bbox(73*s, 51*s, 7*s, 10*s), fill=rgba("#6BE9FF"))
    d.rectangle((49*s, 72*s, 79*s, 77*s), fill=rgba("#C98CFF"))
    d.line([(45*s, 91*s),(35*s,104*s)], fill=outline, width=max(1,int(3*s)))
    d.line([(83*s, 91*s),(93*s,104*s)], fill=outline, width=max(1,int(3*s)))


def boss_core(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#1B0624")
    d.ellipse(bbox(64*s, 64*s, 72*s, 72*s), fill=rgba("#7520A5"), outline=outline, width=max(1, int(3*s)))
    for ang in range(0, 360, 45):
        import math
        a = math.radians(ang)
        x = 64 + math.cos(a)*47
        y = 64 + math.sin(a)*47
        d.line([(64*s,64*s),(x*s,y*s)], fill=rgba("#EC4DFF",120), width=max(1,int(2*s)))
    d.ellipse(bbox(64*s,64*s,30*s,30*s), fill=rgba("#1B0826"), outline=rgba("#FF78FF"), width=max(1,int(2*s)))
    d.ellipse(bbox(64*s,64*s,12*s,12*s), fill=rgba("#FFFFFF"))


def sandbag_dummy(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#2A1D13")
    d.ellipse(bbox(64*s, 97*s, 46*s, 10*s), fill=(0,0,0,40))
    d.rounded_rectangle((45*s, 30*s, 83*s, 91*s), radius=17*s, fill=rgba("#B58A5D"), outline=outline, width=max(1,int(2*s)))
    d.line([(48*s,45*s),(80*s,45*s)], fill=rgba("#6A4C32"), width=max(1,int(2*s)))
    d.line([(52*s,58*s),(76*s,76*s)], fill=rgba("#6A4C32"), width=max(1,int(3*s)))
    d.line([(76*s,58*s),(52*s,76*s)], fill=rgba("#6A4C32"), width=max(1,int(3*s)))
    d.rectangle((58*s,24*s,70*s,35*s), fill=rgba("#755236"), outline=outline)


def moving_platform(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#10253A")
    d.ellipse(bbox(64*s, 83*s, 88*s, 13*s), fill=(0,0,0,40))
    d.rounded_rectangle((20*s, 55*s, 108*s, 75*s), radius=9*s, fill=rgba("#4CB4FF"), outline=outline, width=max(1,int(2*s)))
    d.rectangle((28*s, 61*s, 100*s, 67*s), fill=rgba("#BCEBFF"))
    for x in (35, 64, 93):
        d.ellipse(bbox(x*s, 78*s, 12*s, 12*s), fill=rgba("#12263A"), outline=rgba("#7ED6FF"))


def rebound_pad(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#3A1904")
    d.rounded_rectangle((21*s, 68*s, 107*s, 91*s), radius=8*s, fill=rgba("#F38E2A"), outline=outline, width=max(1,int(2*s)))
    d.polygon(poly_scaled([(27,68),(41,40),(55,68),(69,40),(83,68),(97,40),(103,68)], s), fill=rgba("#FFD26A"), outline=outline)
    d.line([(64*s,82*s),(64*s,45*s)], fill=rgba("#FFFFFF",170), width=max(1,int(2*s)))


def pogo_orb(d: ImageDraw.ImageDraw, s: float) -> None:
    outline = rgba("#07251A")
    d.ellipse(bbox(64*s, 64*s, 44*s, 44*s), fill=rgba("#29E88B"), outline=outline, width=max(1,int(2*s)))
    d.ellipse(bbox(57*s, 55*s, 14*s, 14*s), fill=rgba("#D9FFF0"))
    d.arc((29*s,29*s,99*s,99*s), 20, 330, fill=rgba("#77FFD0",170), width=max(1,int(3*s)))


def soft_blink_wall(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rounded_rectangle((33*s, 23*s, 95*s, 105*s), radius=9*s, fill=rgba("#5632B5",170), outline=rgba("#211052"), width=max(1,int(2*s)))
    for x in (44, 60, 76, 91):
        d.line([(x*s,28*s),((x-12)*s,101*s)], fill=rgba("#B897FF",105), width=max(1,int(2*s)))


def hard_blink_wall(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rounded_rectangle((33*s, 23*s, 95*s, 105*s), radius=6*s, fill=rgba("#841ABF",220), outline=rgba("#260038"), width=max(1,int(3*s)))
    for y in (38, 60, 82):
        d.line([(37*s,y*s),(91*s,(y+8)*s)], fill=rgba("#FF74FF",150), width=max(1,int(2*s)))


def solid_block(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rounded_rectangle((25*s, 31*s, 103*s, 95*s), radius=5*s, fill=rgba("#42495C"), outline=rgba("#161A24"), width=max(1,int(2*s)))
    d.line([(29*s,49*s),(101*s,49*s)], fill=rgba("#687089"), width=max(1,int(2*s)))
    d.line([(55*s,32*s),(55*s,95*s)], fill=rgba("#2C3240"), width=max(1,int(2*s)))


def one_way_platform(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rounded_rectangle((18*s, 58*s, 110*s, 74*s), radius=6*s, fill=rgba("#677699"), outline=rgba("#1A2235"), width=max(1,int(2*s)))
    for x in (32, 50, 68, 86):
        d.polygon(poly_scaled([(x,51),(x+7,40),(x+14,51)], s), fill=rgba("#B4C6F4"))


def door_zone(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rounded_rectangle((42*s, 25*s, 86*s, 100*s), radius=12*s, fill=rgba("#2A324C",210), outline=rgba("#F1B33B"), width=max(1,int(3*s)))
    d.ellipse(bbox(77*s, 64*s, 5*s, 5*s), fill=rgba("#F1B33B"))
    d.arc((32*s,12*s,96*s,113*s), 80, 280, fill=rgba("#F1B33B",130), width=max(1,int(2*s)))


def edge_exit(d: ImageDraw.ImageDraw, s: float) -> None:
    d.rectangle((27*s, 26*s, 101*s, 102*s), fill=rgba("#0D2230",130), outline=rgba("#43E9FF",200), width=max(1,int(2*s)))
    for x in (44, 62, 80):
        d.line([(x*s,35*s),(x*s,94*s)], fill=rgba("#43E9FF",120), width=max(1,int(2*s)))
    d.polygon(poly_scaled([(52,49),(77,64),(52,79)], s), fill=rgba("#43E9FF"))


def projectile_energy(d: ImageDraw.ImageDraw, s: float) -> None:
    d.ellipse(bbox(64*s,64*s,38*s,24*s), fill=rgba("#6BE9FF",210), outline=rgba("#0B2B36"), width=max(1,int(2*s)))
    d.polygon(poly_scaled([(30,64),(54,52),(54,76)], s), fill=rgba("#C58AFF",150))
    d.ellipse(bbox(72*s,60*s,10*s,8*s), fill=rgba("#FFFFFF",200))


ENTITY_SPECS: List[EntitySpriteSpec] = [
    EntitySpriteSpec("chest_closed", "chest_closed.png", "FeatureVisualKind::Chest", "ChestClosed", "closed treasure chest"),
    EntitySpriteSpec("chest_open", "chest_open.png", "FeatureVisualKind::Chest", "ChestOpened", "opened reward chest"),
    EntitySpriteSpec("breakable_intact", "breakable_intact.png", "FeatureVisualKind::Breakable", "BreakableIntact", "intact breakable block"),
    EntitySpriteSpec("breakable_cracked", "breakable_cracked.png", "FeatureVisualKind::Breakable", "BreakableCracking", "damaged breakable block"),
    EntitySpriteSpec("breakable_broken", "breakable_broken.png", "FeatureVisualKind::Breakable", "BreakableBroken", "broken debris state"),
    EntitySpriteSpec("pickup_health", "pickup_health.png", "FeatureVisualKind::Pickup", "PickupKind::Health", "health pickup"),
    EntitySpriteSpec("pickup_currency", "pickup_currency.png", "FeatureVisualKind::Pickup", "PickupKind::Currency", "currency pickup"),
    EntitySpriteSpec("pickup_ability", "pickup_ability.png", "FeatureVisualKind::Pickup", "PickupKind::Ability", "ability pickup"),
    EntitySpriteSpec("hazard_spikes", "hazard_spikes.png", "FeatureVisualKind::Hazard", "DamageVolume/HazardBlock", "spike hazard"),
    EntitySpriteSpec("npc_terminal", "npc_terminal.png", "FeatureVisualKind::Npc", "InteractionKind::Npc", "talkable terminal NPC"),
    EntitySpriteSpec("boss_core", "boss_core.png", "FeatureVisualKind::Boss", "BossDormant/BossPhase", "boss core placeholder"),
    EntitySpriteSpec("sandbag_dummy", "sandbag_dummy.png", "FeatureVisualKind::Sandbag", "sandbag_infinite/sandbag_finite", "combat-practice sandbag"),
    EntitySpriteSpec("moving_platform", "moving_platform.png", "ActorKind::MovingPlatform", "MovingPlatformVisual", "time-reference moving platform"),
    EntitySpriteSpec("rebound_pad", "rebound_pad.png", "BlockKind::Rebound", "SurfaceContact::Rebound", "momentum rebound pad"),
    EntitySpriteSpec("pogo_orb", "pogo_orb.png", "BlockKind::PogoOrb", "SurfaceContact::PogoRefresh", "pogo refresh orb"),
    EntitySpriteSpec("soft_blink_wall", "soft_blink_wall.png", "BlockKind::BlinkWall", "BlinkWallTier::Soft", "soft blink-passable wall"),
    EntitySpriteSpec("hard_blink_wall", "hard_blink_wall.png", "BlockKind::BlinkWall", "BlinkWallTier::Hard", "hard blink wall"),
    EntitySpriteSpec("solid_block", "solid_block.png", "BlockKind::Solid", "SurfaceCollision::Solid", "solid room block tile"),
    EntitySpriteSpec("one_way_platform", "one_way_platform.png", "BlockKind::OneWay", "SurfaceCollision::OneWayUp", "one-way platform tile"),
    EntitySpriteSpec("door_zone", "door_zone.png", "LoadingZoneActivation::Door", "Door", "interior door loading zone"),
    EntitySpriteSpec("edge_exit", "edge_exit.png", "LoadingZoneActivation::EdgeExit", "EdgeExit", "edge-exit loading zone"),
    EntitySpriteSpec("projectile_energy", "projectile_energy.png", "ActorKind::Projectile", "future projectile", "small energy projectile placeholder"),
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
}


def render_entity_sprite(spec: EntitySpriteSpec, supersample: int = 4) -> Image.Image:
    try:
        draw_fn = DRAWERS[spec.key]
    except KeyError as ex:
        raise KeyError(f"no drawer registered for {spec.key!r}") from ex
    return render(draw_fn, spec.size, supersample)


def build_entity_contact_sheet(tiles: List[Tuple[EntitySpriteSpec, Image.Image]]) -> Image.Image:
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


def write_entity_sprites(out_dir: str | Path = Path("assets/entities"), supersample: int = 4) -> List[Path]:
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
