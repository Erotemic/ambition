#!/usr/bin/env python3
"""**Dress an LDtk world's editor in the art the ENGINE actually draws.**

Jon, 2026-08-05: *"Something I would very much like is if the mary-o ldtk file
was able to use the sprites and tiles so the editor looked more like the level
we were building."*

Opening a world in LDtk showed grey IntGrid cells and coloured rectangles. The
running game draws the same level out of `assets/sprites/entities/*.png` — the
masonry of `solid_tile.png`, the interrobang plate of `bonus_block_tile.png`,
the spikes of `hazard_tile.png`. Nothing was missing but the wiring: LDtk only
renders what a `.ldtk` registers, and no world registered any of it.

This tool is that wiring, and it takes the art from ONE place — the engine's own
sprite folder — so an editor cell cannot drift from the block the simulation
spawns underneath it. Two halves:

* **IntGrid layers get auto-layer rules.** Painting `Solid` in the editor draws
  masonry immediately, and keeps drawing it as the author edits, because the
  rules are evaluated live by LDtk from the cells themselves. Nothing is baked,
  so a repaint can never leave stale art behind — the failure mode of a painted
  Tiles layer (`tileset paint`).
* **Entity defs get a `tileRect`.** A `ChestSpawn` looks like the chest, a
  `MovingPlatform` like the platform.

⚠ **the art is 32px and the grid is 16px, so one tile takes FOUR cells.** Every
engine tile texture is authored at 32×32 (or 32×16 / 16×32) and the game repeats
it at native scale across a block's footprint, while collision is authored on a
16px grid. So each texture is cut on the 16px grid and re-assembled by cell
parity: one `Single` rule per quadrant, `xModulo`/`yModulo` = the texture's size
in cells and `xOffset`/`yOffset` = which quadrant. Four rules reproduce a 32×32
texture exactly, phased to the level origin.

⭐ **the atlas is generated, never authored.** LDtk needs one image per tileset
and the engine ships one PNG per sprite, so this composes them onto a 16px grid
and records where each landed. It is written next to the sprites it is made of
(and gitignored with them, like `editor_icons.png`), so a fresh clone rebuilds
it from `regen_sprites.sh` rather than carrying a binary in git.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python3 -m ambition_ldtk_tools asset editor-art \\
    game/ambition_demo_mary_o/assets/worlds/mary_o.ldtk --in-place
```

A world adds its own nouns through a sidecar named after it —
`<world>.editor_art.json` beside the `.ldtk` — which names extra art files and
maps its own entity defs onto them. Mary-O's pipes and blocks come from there;
this module knows only what the ENGINE draws, which is the part every world
shares.

`--preview OUT.png` renders the level exactly as these rules will draw it, which
is the only way to look at the result without opening the editor.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

from ambition_ldtk_tools.ldtk import (
    LdtkTransaction,
    alloc_uid,
    default_sprite_assets_dir,
    find_tileset,
    layer_defs,
    load_project,
    rel_to_ldtk,
    tileset_defs,
)
from ambition_ldtk_tools.edit.visual_manifest import (
    apply_entity_icons,
    prune_unused_tilesets,
    upsert_tilesets,
)

#: The atlas's cell size. It matches the IntGrid grid every Ambition world is
#: authored on, which is what lets a rule address half of a 32px texture.
CELL = 16

#: How wide the generated atlas is, in cells. Fixed so a tile id
#: (`row * width + col`) is stable across regenerations.
ATLAS_COLS = 32

#: IntGrid VALUE identifier -> the engine sprite the game draws for it.
#:
#: Keyed by the value's name rather than its number: the number is per-layer
#: (`Collision` 1 is Solid, `Water` 1 is ClearWater) and the name is what an
#: author reads. A value with no entry keeps its flat editor colour, which is
#: the honest look for a concept with no art yet.
ENGINE_INTGRID_ART: dict[str, str] = {
    "Solid": "solid_tile",
    "OneWayUp": "one_way_tile",
    "BlinkSoft": "soft_blink_tile",
    "BlinkHard": "hard_blink_tile",
    "Hazard": "hazard_tile",
    "Ladder": "ladder_tile",
    "ClearWater": "water_surface_tile",
    "MurkyWater": "water_surface_tile",
}

#: Entity def identifier -> the engine sprite the game spawns for it.
#:
#: ⚠ **only where the mapping is 1:1.** `EnemySpawn`/`NpcSpawn`/`BossSpawn` name
#: a CHARACTER through a field, so one representative sprite would claim a
#: specific enemy stands there; they get the engine's generic actor art, which
#: says "a body spawns here" and nothing more. Per-instance art needs the
#: field's values to carry tiles, which is a schema change and a separate job.
ENGINE_ENTITY_ART: dict[str, Any] = {
    # The one spawner that IS 1:1 — every world starts the same body.
    "PlayerStart": {"sheet": "player_robot_v3", "frame": 0},
    "Solid": "solid_block",
    "OneWayPlatform": "one_way_platform",
    "BlinkWall": "soft_blink_wall",
    "HazardBlock": "hazard_spikes",
    "DamageVolume": "hazard_spikes",
    "PogoOrb": "pogo_orb",
    "BreakablePogoOrb": "pogo_orb",
    "ReboundPad": "rebound_pad",
    "MovingPlatform": "moving_platform",
    "BreakablePlatform": "breakable_intact",
    "ChestSpawn": "chest_closed",
    "PickupSpawn": "pickup_ability",
    "GroundItem": "pickup_currency",
    "Switch": "switch_armed",
    "LockWall": "lock_wall_tile",
    "NpcSpawn": "npc_terminal",
    "BossSpawn": "boss_core",
    "EnemySpawn": "sandbag_dummy",
    "LoadingZone": "door_zone",
    "WaterVolume": "water_surface_tile",
    "ShrineSpawn": "save_point",
    "MorphBallSpawn": "morph_ball",
}

#: `tileRenderMode` per entity. The default (`Cover`) fills the authored box,
#: which is right for a wall or a platform whose box IS the shape. A spawner's
#: box is a region rather than a silhouette, so its art is fitted instead of
#: stretched.
FIT_INSIDE = {
    "PlayerStart",
    "EnemySpawn",
    "NpcSpawn",
    "BossSpawn",
    "PickupSpawn",
    "GroundItem",
    "ChestSpawn",
    "Switch",
    "ShrineSpawn",
    "MorphBallSpawn",
    "PogoOrb",
    "BreakablePogoOrb",
    "MorphBallSpawn",
}


@dataclass(frozen=True)
class ArtSource:
    """One image going into the atlas: a whole PNG, or a frame cut from a sheet.

    ⭐ **a character's editor icon is a FRAME, not a file.** Actors ship as one
    spritesheet with every animation on it, so pointing an entity def at the PNG
    would put the whole contact sheet in the box. `crop` takes the frame out and
    `trim` drops the transparent margin the renderer pads frames with, so the
    icon is the character at the size the box gives it.
    """

    key: str
    path: Path
    crop: tuple[int, int, int, int] | None = None
    trim: bool = False

    def load(self) -> Any:
        from PIL import Image

        image = Image.open(self.path).convert("RGBA")
        if self.crop is not None:
            x, y, w, h = self.crop
            image = image.crop((x, y, x + w, y + h))
        if self.trim:
            box = image.getbbox()
            if box is not None:
                image = image.crop(box)
        return image


@dataclass(frozen=True)
class Placement:
    """Where an art source landed in the atlas, in pixels and in cells."""

    key: str
    x: int
    y: int
    w: int
    h: int

    @property
    def cols(self) -> int:
        return max(1, -(-self.w // CELL))

    @property
    def rows(self) -> int:
        return max(1, -(-self.h // CELL))

    def tile_id(self, col: int, row: int) -> int:
        return ((self.y // CELL) + row) * ATLAS_COLS + (self.x // CELL) + col

    def rect(self, tileset_uid: int) -> dict[str, int]:
        return {"tilesetUid": tileset_uid, "x": self.x, "y": self.y, "w": self.w, "h": self.h}


def sidecar_path(ldtk: Path) -> Path:
    """`foo.ldtk` -> `foo.editor_art.json`, the world's own art overlay."""
    return ldtk.with_suffix("").with_name(f"{ldtk.stem}.editor_art.json")


def load_sidecar(ldtk: Path) -> dict[str, Any]:
    path = sidecar_path(ldtk)
    if not path.is_file():
        return {}
    try:
        data = json.loads(path.read_text())
    except json.JSONDecodeError as ex:
        raise SystemExit(f"{path}: not valid JSON ({ex})")
    if not isinstance(data, dict):
        raise SystemExit(f"{path}: expected an object")
    return data


def art_key(reference: Any) -> str:
    """A stable atlas key for an art reference, whatever shape it came in."""
    if isinstance(reference, dict):
        return f"sheet:{reference['sheet']}#{int(reference.get('frame', 0))}"
    return str(reference)


def sheet_frame_rect(
    sheet: str, sprites_dir: Path, animation: str, frame: int
) -> tuple[int, int, int, int]:
    """Where one animation frame sits on a character sheet.

    ⛔ **`frame_width` × `frame_height` is NOT the packing pitch.** A published
    sheet is atlas-packed — frames are trimmed to their own bounds and placed
    wherever they fit, so `idle` frame 0 of the player is a 71×101 rect at
    (1390, 1) on a sheet whose declared frame size is 224×224. Multiplying an
    index by the frame size lands on whatever happens to be there, which is what
    the first attempt at this did and why the player's icon came out as a strip
    of three other robots. The sidecar records every rect; read them.
    """
    import yaml

    sidecar = sprites_dir / f"{sheet}_spritesheet.yaml"
    if not sidecar.is_file():
        raise SystemExit(f"sheet {sheet!r}: no {sidecar.name} beside the PNG")
    data = yaml.safe_load(sidecar.read_text()) or {}
    rows = [row for row in data.get("rows") or [] if row.get("animation") == animation]
    if not rows:
        available = sorted({str(row.get("animation")) for row in data.get("rows") or []})
        raise SystemExit(
            f"sheet {sheet!r}: no {animation!r} animation (has: {', '.join(available)})"
        )
    rects = rows[0].get("rects") or []
    if frame >= len(rects):
        raise SystemExit(
            f"sheet {sheet!r}: {animation!r} has {len(rects)} frames, asked for {frame}"
        )
    rect = rects[frame]
    return int(rect["x"]), int(rect["y"]), int(rect["w"]), int(rect["h"])


def resolve_art(reference: Any, sprites_dir: Path, ldtk: Path) -> ArtSource:
    """Resolve one art reference to something the atlas can paste.

    Three shapes, in the order an author reaches for them:

    * a bare stem — the engine's entity-sprite folder (`solid_tile` ->
      `entities/solid_tile.png`), which is where every block and prop the
      simulation draws lives;
    * a path — relative to the sprite folder first and to the `.ldtk` second,
      so a world can name a prop (`props/mary_o_pipe_top`) without knowing where
      sprites are installed;
    * `{"sheet": "ai_slop", "animation": "idle", "frame": 0}` — one frame of a
      character sheet, read out of the sidecar's packed rects.
    """
    key = art_key(reference)
    if isinstance(reference, dict):
        sheet = str(reference["sheet"])
        path = (sprites_dir / f"{sheet}_spritesheet.png").resolve()
        if not path.is_file():
            raise SystemExit(f"sheet {sheet!r}: no PNG at {path}")
        crop = sheet_frame_rect(
            sheet,
            sprites_dir,
            str(reference.get("animation", "idle")),
            int(reference.get("frame", 0)),
        )
        return ArtSource(key, path, crop=crop, trim=bool(reference.get("trim", True)))
    stem_or_path = str(reference)
    if "/" in stem_or_path or stem_or_path.endswith(".png"):
        rel = stem_or_path if stem_or_path.endswith(".png") else f"{stem_or_path}.png"
        for base in (sprites_dir, ldtk.parent):
            candidate = (base / rel).resolve()
            if candidate.is_file():
                return ArtSource(key, candidate)
        raise SystemExit(f"art {stem_or_path!r}: no PNG at {sprites_dir / rel}")
    candidate = (sprites_dir / "entities" / f"{stem_or_path}.png").resolve()
    if not candidate.is_file():
        raise SystemExit(f"art {stem_or_path!r}: no PNG at {candidate}")
    return ArtSource(key, candidate)


def collect_sources(
    project: dict[str, Any], ldtk: Path, sprites_dir: Path, sidecar: dict[str, Any]
) -> tuple[list[ArtSource], dict[str, str], dict[str, str]]:
    """Work out which art this world needs, and what each thing points at.

    Returns the sources to pack, the IntGrid-value -> art-key map and the
    entity-def -> art-key map. Only art the world can actually SHOW is
    collected: a value or entity def the project does not define contributes
    nothing, so an atlas stays as small as the world that uses it.
    """
    intgrid_art: dict[str, str] = dict(ENGINE_INTGRID_ART)
    intgrid_art.update(sidecar.get("intgrid_art") or {})
    entity_art: dict[str, str] = dict(ENGINE_ENTITY_ART)
    entity_art.update(sidecar.get("entity_art") or {})

    present_values = {
        str(value.get("identifier"))
        for layer in layer_defs(project)
        for value in layer.get("intGridValues") or []
    }
    present_entities = {
        str(ent.get("identifier")) for ent in project.get("defs", {}).get("entities", [])
    }

    used_intgrid = {
        k: art_key(v) for k, v in intgrid_art.items() if k in present_values
    }
    used_entities = {
        k: art_key(v) for k, v in entity_art.items() if k in present_entities
    }

    references: dict[str, Any] = {}
    for name, reference in list(intgrid_art.items()) + list(entity_art.items()):
        if name in present_values or name in present_entities:
            references[art_key(reference)] = reference
    sources = [
        resolve_art(references[key], sprites_dir, ldtk) for key in sorted(references)
    ]
    return sources, used_intgrid, used_entities


def pack_atlas(sources: Iterable[ArtSource]) -> tuple[list[Placement], int, int]:
    """Shelf-pack every source onto the 16px grid, tallest-first per row.

    Deterministic: sources arrive sorted by key and the packing is a pure
    function of their sizes, so re-running produces byte-identical output and
    every tile id stays where it was.
    """
    placements: list[Placement] = []
    col = 0
    row_start = 0
    row_height = 0
    for source in sources:
        w, h = source.load().size
        cols = max(1, -(-w // CELL))
        rows = max(1, -(-h // CELL))
        if cols > ATLAS_COLS:
            raise SystemExit(
                f"art {source.key!r} is {w}px wide, wider than the {ATLAS_COLS * CELL}px atlas"
            )
        if col + cols > ATLAS_COLS:
            col = 0
            row_start += row_height
            row_height = 0
        placements.append(
            Placement(source.key, col * CELL, row_start * CELL, w, h)
        )
        col += cols
        row_height = max(row_height, rows)
    height_cells = row_start + row_height
    return placements, ATLAS_COLS * CELL, max(CELL, height_cells * CELL)


def write_atlas(
    out: Path, sources: list[ArtSource], placements: list[Placement], size: tuple[int, int]
) -> None:
    from PIL import Image

    by_key = {source.key: source for source in sources}
    atlas = Image.new("RGBA", size, (0, 0, 0, 0))
    for placement in placements:
        atlas.alpha_composite(by_key[placement.key].load(), (placement.x, placement.y))
    out.parent.mkdir(parents=True, exist_ok=True)
    atlas.save(out)


def auto_rules_for_layer(
    layer: dict[str, Any],
    intgrid_art: dict[str, str],
    by_key: dict[str, Placement],
    project: dict[str, Any],
) -> list[dict[str, Any]]:
    """One `Single` rule per cell of each value's texture.

    A 32×32 texture is four rules — modulo 2 on both axes, one per quadrant —
    so the four 16px cells of any 2×2 cell block receive the four quarters of
    the image in the right order. A 32×16 texture (one-way platforms, spikes)
    is two rules on X only; a 16×32 one (ladders) two on Y.
    """
    rules: list[dict[str, Any]] = []
    for value in layer.get("intGridValues") or []:
        key = intgrid_art.get(str(value.get("identifier")))
        if key is None:
            continue
        placement = by_key[key]
        for row in range(placement.rows):
            for col in range(placement.cols):
                rules.append(
                    {
                        "uid": alloc_uid(project),
                        "active": True,
                        "size": 1,
                        "tileRectsIds": [[placement.tile_id(col, row)]],
                        "alpha": 1.0,
                        "chance": 1.0,
                        "breakOnMatch": True,
                        "pattern": [int(value.get("value"))],
                        "flipX": False,
                        "flipY": False,
                        "xModulo": placement.cols,
                        "yModulo": placement.rows,
                        "xOffset": col,
                        "yOffset": row,
                        "tileXOffset": 0,
                        "tileYOffset": 0,
                        "tileRandomXMin": 0,
                        "tileRandomXMax": 0,
                        "tileRandomYMin": 0,
                        "tileRandomYMax": 0,
                        "checker": "None",
                        "tileMode": "Single",
                        "pivotX": 0.0,
                        "pivotY": 0.0,
                        "outOfBoundsValue": None,
                        "perlinActive": False,
                        "perlinSeed": 0,
                        "perlinScale": 0.2,
                        "perlinOctaves": 2,
                        "invalidated": True,
                    }
                )
    return rules


#: The rule group this tool owns. Anything an author adds by hand lives in a
#: group with another name and survives every re-run untouched.
RULE_GROUP_NAME = "Engine art"


def apply_auto_rules(
    project: dict[str, Any],
    tileset_uid: int,
    intgrid_art: dict[str, str],
    by_key: dict[str, Placement],
) -> list[str]:
    messages: list[str] = []
    for layer in layer_defs(project):
        if layer.get("type") != "IntGrid":
            continue
        rules = auto_rules_for_layer(layer, intgrid_art, by_key, project)
        groups = [
            group
            for group in layer.get("autoRuleGroups") or []
            if group.get("name") != RULE_GROUP_NAME
        ]
        if not rules:
            layer["autoRuleGroups"] = groups
            continue
        groups.append(
            {
                "uid": alloc_uid(project),
                "name": RULE_GROUP_NAME,
                "color": None,
                "icon": None,
                "active": True,
                "isOptional": False,
                "rules": rules,
                "usesWizard": False,
                "biomeRequirementMode": 0,
                "requiredBiomeValues": [],
            }
        )
        layer["autoRuleGroups"] = groups
        # 1.2.0 retired `autoTilesetDefUid`; an auto-layer's source tileset is
        # `tilesetDefUid` now, on the IntGrid layer itself.
        layer["tilesetDefUid"] = tileset_uid
        messages.append(f"{layer['identifier']}: {len(rules)} auto rules")
    return messages


def build_manifest(
    atlas_identifier: str,
    atlas_path: Path,
    entity_art: dict[str, str],
    by_key: dict[str, Placement],
) -> dict[str, Any]:
    """The shape `visual_manifest.apply-manifest` already consumes."""
    icons: dict[str, Any] = {}
    for entity, key in sorted(entity_art.items()):
        placement = by_key[key]
        icons[entity] = {
            "tileset": atlas_identifier,
            "tile": [placement.x, placement.y, placement.w, placement.h],
            "tile_render_mode": "FitInside" if entity in FIT_INSIDE else "Cover",
        }
    return {
        "tilesets": [
            {
                "identifier": atlas_identifier,
                "path": str(atlas_path),
                "tile_width": CELL,
                "tile_height": CELL,
                "tags": ["engine-art"],
            }
        ],
        "entity_icons": icons,
    }


def default_atlas_path(ldtk: Path, sprites_dir: Path) -> Path:
    """Where the atlas goes, addressed the way the world addresses sprites.

    ⭐ **through the world's own `assets/sprites` mount, not the crate that
    holds the files.** Every world's assets dir carries a `sprites` symlink onto
    the shared generated tree, and `rel_to_ldtk` only rewrites a path it sees
    INSIDE that shared tree — so naming the atlas through the mount is what
    turns the tileset's `relPath` into a neighbourly `../sprites/…` instead of a
    traversal out of the game and into `crates/`.
    """
    mount = ldtk.parent.parent / "sprites"
    if mount.is_dir():
        return mount / f"ldtk_editor_art_{ldtk.stem}.png"
    return sprites_dir / f"ldtk_editor_art_{ldtk.stem}.png"


def dress(
    ldtk: Path,
    *,
    sprites_dir: Path,
    atlas: Path | None,
    in_place: bool,
    output: Path | None,
) -> int:
    transaction = LdtkTransaction(source=ldtk, in_place=in_place, output=output)
    project = transaction.project
    sidecar = load_sidecar(ldtk)
    atlas_identifier = str(sidecar.get("atlas_identifier") or "EngineArt")
    atlas_path = atlas or default_atlas_path(ldtk, sprites_dir)

    sources, intgrid_art, entity_art = collect_sources(project, ldtk, sprites_dir, sidecar)
    if not sources:
        print(f"{ldtk.name}: no engine art applies to this world")
        return 0
    placements, atlas_w, atlas_h = pack_atlas(sources)
    write_atlas(atlas_path, sources, placements, (atlas_w, atlas_h))
    by_key = {placement.key: placement for placement in placements}
    print(f"atlas: {atlas_path} ({atlas_w}x{atlas_h}, {len(placements)} sprites)")

    manifest = build_manifest(atlas_identifier, atlas_path, entity_art, by_key)
    messages = upsert_tilesets(project, ldtk, manifest)
    tileset = find_tileset(project, atlas_identifier)
    assert tileset is not None  # upsert_tilesets just wrote it
    # ⚠ `rel_to_ldtk` resolves symlinks, so an atlas named through this world's
    # own `assets/sprites` mount comes back addressed through the OTHER game
    # that happens to own the shared tree. Say it the neighbourly way when the
    # world has its own mount — the editor reads either, but a world reaching
    # across `game/` for its own art is a lie about who owns it.
    mount = ldtk.parent.parent / "sprites"
    if mount.is_dir() and atlas_path.parent.samefile(mount):
        tileset["relPath"] = f"../sprites/{atlas_path.name}"
    messages += apply_entity_icons(project, manifest)
    messages += apply_auto_rules(project, int(tileset["uid"]), intgrid_art, by_key)
    messages += prune_unused_tilesets(project)
    for message in messages:
        print(f"  {message}")
    transaction.note_changed()
    transaction.finish(write_message="wrote {path}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="ambition_ldtk_tools asset editor-art",
        description="Point a world's editor visuals at the art the engine draws",
    )
    parser.add_argument("action", nargs="?", default="editor-art", help=argparse.SUPPRESS)
    parser.add_argument("ldtk", type=Path, help="the .ldtk project to dress")
    parser.add_argument(
        "--sprites-dir",
        type=Path,
        default=None,
        help="engine sprite folder (default: the repo's installed sprites)",
    )
    parser.add_argument("--atlas", type=Path, default=None, help="where to write the atlas PNG")
    parser.add_argument("--in-place", action="store_true", help="rewrite the .ldtk")
    parser.add_argument("--output", type=Path, default=None, help="write the result elsewhere")
    parser.add_argument(
        "--preview",
        type=Path,
        default=None,
        help="render the dressed level to a PNG instead of opening LDtk",
    )
    args = parser.parse_args(argv)

    sprites_dir = args.sprites_dir or default_sprite_assets_dir(args.ldtk)
    if not args.in_place and args.output is None:
        raise SystemExit("pass --in-place or --output <path>")
    code = dress(
        args.ldtk,
        sprites_dir=sprites_dir,
        atlas=args.atlas,
        in_place=args.in_place,
        output=args.output,
    )
    if code == 0 and args.preview is not None:
        from ambition_ldtk_tools.edit.editor_art_preview import render_preview

        target = args.ldtk if args.in_place else (args.output or args.ldtk)
        out = render_preview(target, args.preview, sprites_dir=sprites_dir)
        print(f"preview: {out}")
    return code


if __name__ == "__main__":
    sys.exit(main())
