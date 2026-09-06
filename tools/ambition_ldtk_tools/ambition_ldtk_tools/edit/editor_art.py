#!/usr/bin/env python3
"""Wire engine sprite art into the LDtk editor representation.

IntGrid values receive generated AutoLayer art, entity definitions receive
`tileRect` previews when their mapping is one-to-one, and field-dependent entity
art can select a tile from an enum value. The generated atlas is derived from the
engine sprite directory rather than authored independently.

Engine textures may span multiple 16px LDtk cells; generated AutoLayer rules
select the appropriate texture quadrant by cell parity. World-specific mappings
live in `<world>.editor_art.json` beside the LDtk project.

Usage::

    PYTHONPATH=tools/ambition_ldtk_tools \
      python3 -m ambition_ldtk_tools asset editor-art <world.ldtk> --in-place

`--preview OUT.png` renders the editor-art result without opening LDtk."""

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

# : The atlas's cell size. It matches the IntGrid grid every Ambition world is
# : authored on, which is what lets a rule address half of a 32px texture.
CELL = 16

# : How wide the generated atlas is, in cells.
ATLAS_COLS = 32

# : IntGrid VALUE identifier -> the engine sprite the game draws for it.
# :
# : Keyed by the value's name rather than its number: the number is per-layer
# : (`Collision` 1 is Solid, `Water` 1 is ClearWater) and the name is what an
# : author reads. A value with no entry keeps its flat editor colour, which is
# : the honest look for a concept with no art yet.
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

# : Entity def identifier -> the engine sprite the game spawns for it.
# :
# : only where the mapping is 1:1. `EnemySpawn`/`NpcSpawn`/`BossSpawn` name
# : a CHARACTER through a field, so one representative sprite would claim a
# : specific enemy stands there; they get the engine's generic actor art, which
# : says "a body spawns here" and nothing more. Per-instance art needs the
# : field's values to carry tiles, which is a schema change and a separate job.
ENGINE_ENTITY_ART: dict[str, Any] = {
    # The one spawner that IS 1:1 — every world starts the same body.
    "PlayerStart": {"sheet": "player_robot_v3", "frame": 0},
    "Solid": "solid_block",
    "OneWayPlatform": "one_way_platform",
    "BlinkWall": "soft_blink_wall",
    "HazardBlock": "hazard_tile",
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

# : `tileRenderMode` per entity. The default (`Cover`) fills the authored box,
# : which is right for a wall or a platform whose box IS the shape. A spawner's
# : box is a region rather than a silhouette, so its art is fitted instead of
# : stretched.
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

     a character's editor icon is a FRAME, not a file. Actors ship as one
    spritesheet with every animation on it, so pointing an entity def at the PNG
    would put the whole contact sheet in the box. `crop` takes the frame out and
    `trim` drops the transparent margin the renderer pads frames with, so the
    icon is the character at the size the box gives it.
    """

    key: str
    path: Path
    crop: tuple[int, int, int, int] | None = None
    trim: bool = False
    flip_y: bool = False

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
        if self.flip_y:
            # a pipe hanging from a ceiling wears its lip UNDERNEATH, which
            # is the whole difference a `mouth: Down` makes and is one flip of
            # the art the game already ships.
            image = image.transpose(Image.FLIP_TOP_BOTTOM)
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
        flipped = "^" if reference.get("flip_y") else ""
        if "sheet" in reference:
            return (
                f"sheet:{reference['sheet']}#"
                f"{reference.get('animation', 'idle')}:{int(reference.get('frame', 0))}{flipped}"
            )
        return f"{reference['art']}{flipped}"
    return str(reference)


def sheet_frame_rect(
    sheet: str, sprites_dir: Path, animation: str, frame: int
) -> tuple[int, int, int, int]:
    """Where one animation frame sits on a character sheet.

     `frame_width` × `frame_height` is NOT the packing pitch. A published
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
    if isinstance(reference, dict) and "sheet" in reference:
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
        return ArtSource(
            key,
            path,
            crop=crop,
            trim=bool(reference.get("trim", True)),
            flip_y=bool(reference.get("flip_y")),
        )
    if isinstance(reference, dict):
        plain = resolve_art(str(reference["art"]), sprites_dir, ldtk)
        return ArtSource(
            key, plain.path, crop=plain.crop, trim=plain.trim,
            flip_y=bool(reference.get("flip_y")),
        )
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
) -> tuple[list[ArtSource], dict[str, str], dict[str, str], dict[str, dict[str, str]]]:
    """Work out which art this world needs, and what each thing points at.

    Returns the sources to pack, the IntGrid-value -> art-key map, the
    entity-def -> art-key map and the per-FIELD-value maps. Only art the world
    can actually SHOW is collected: a value or entity def the project does not
    define contributes nothing, so an atlas stays as small as the world using it.
    """
    intgrid_art: dict[str, str] = dict(ENGINE_INTGRID_ART)
    intgrid_art.update(sidecar.get("intgrid_art") or {})
    entity_art: dict[str, str] = dict(ENGINE_ENTITY_ART)
    entity_art.update(sidecar.get("entity_art") or {})
    # A field_art entry is either `{value: art}` or `{"enum": name, "values":
    # {value: art}}` — the second form for a field with no vocabulary manifest
    # to declare its enum. Normalise to the second shape.
    field_art: dict[str, dict[str, Any]] = {}
    for path, entry in (sidecar.get("field_art") or {}).items():
        entry = dict(entry or {})
        if "values" in entry:
            field_art[str(path)] = {
                "enum": entry.get("enum"),
                "values": dict(entry["values"] or {}),
            }
        else:
            field_art[str(path)] = {"enum": None, "values": entry}

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

    used_fields = {
        path: {
            "enum": entry["enum"],
            "values": {
                value: art_key(reference) for value, reference in entry["values"].items()
            },
        }
        for path, entry in field_art.items()
        if path.partition(".")[0] in present_entities
    }

    references: dict[str, Any] = {}
    for name, reference in list(intgrid_art.items()) + list(entity_art.items()):
        if name in present_values or name in present_entities:
            references[art_key(reference)] = reference
    for path, entry in field_art.items():
        if path.partition(".")[0] not in present_entities:
            continue
        for reference in entry["values"].values():
            references[art_key(reference)] = reference
    sources = [
        resolve_art(references[key], sprites_dir, ldtk) for key in sorted(references)
    ]
    return sources, used_intgrid, used_entities, used_fields


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


# : The rule group this tool owns. Anything an author adds by hand lives in a
# : group with another name and survives every re-run untouched.
RULE_GROUP_NAME = "Engine art"

# : Suffix for the AutoLayer that draws a source IntGrid layer's art.
ART_LAYER_SUFFIX = "Art"

# The art belongs on a layer of its : own, and LDtk has the two knobs that make the pair readable: :
# : * `displayOpacity` — the layer's own alpha. Left at 1.0, so SELECTING the : collision layer to
# paint shows exactly the cells, crisply, art hidden. : * `inactiveOpacity` — what it fades to while
# another layer is current, which : is the state you read the level in: a tint saying Solid here,
# Hazard there, : over art you can still see. : : written only when the art layer is CREATED. These
# are editor preferences : that live in the file, so re-running must not argue with an author who
# has : since dragged the sliders.
COLLISION_OPACITY = 1.0
COLLISION_INACTIVE_OPACITY = 0.35


def art_layer_identifier(source: dict[str, Any]) -> str:
    return f"{source['identifier']}{ART_LAYER_SUFFIX}"


def build_art_layer(project: dict[str, Any], source: dict[str, Any], tileset_uid: int) -> dict:
    """An AutoLayer that draws another layer's IntGrid.

     `autoSourceLayerDefUid` is the whole point — this layer owns no cells
    of its own. It reads the collision the author paints and answers with art,
    which is why the two can be shown, hidden and dimmed independently while
    remaining incapable of disagreeing about where a wall is.
    """
    return {
        "__type": "AutoLayer",
        "identifier": art_layer_identifier(source),
        "type": "AutoLayer",
        "uid": alloc_uid(project),
        "doc": (
            f"Editor art for {source['identifier']}, generated by "
            "`ambition_ldtk_tools asset editor-art`. Paint the source layer; this "
            "one answers with the engine's own texture for whatever is there."
        ),
        "uiColor": None,
        "gridSize": int(source.get("gridSize") or CELL),
        "guideGridWid": 0,
        "guideGridHei": 0,
        "displayOpacity": 1.0,
        "inactiveOpacity": 1.0,
        "hideInList": False,
        "hideFieldsWhenInactive": True,
        "canSelectWhenInactive": True,
        "renderInWorldView": True,
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "parallaxFactorX": 0,
        "parallaxFactorY": 0,
        "parallaxScaling": True,
        "requiredTags": [],
        "excludedTags": [],
        "autoTilesKilledByOtherLayerUid": None,
        "uiFilterTags": [],
        "useAsyncRender": False,
        "intGridValues": [],
        "intGridValuesGroups": [],
        "autoRuleGroups": [],
        "autoSourceLayerDefUid": int(source["uid"]),
        "tilesetDefUid": tileset_uid,
        "tilePivotX": 0,
        "tilePivotY": 0,
        "biomeFieldUid": None,
    }


def build_art_layer_instance(
    level: dict[str, Any], source_instance: dict[str, Any], layer: dict
) -> dict:
    """A blank instance of the art layer, shaped like the one it shadows."""
    return {
        "__identifier": layer["identifier"],
        "__type": "AutoLayer",
        "iid": f"{layer['identifier']}-{layer['uid']}-{level.get('uid', 0)}",
        "layerDefUid": int(layer["uid"]),
        "__cWid": int(source_instance.get("__cWid") or 0),
        "__cHei": int(source_instance.get("__cHei") or 0),
        "__gridSize": int(layer["gridSize"]),
        "__opacity": 1,
        "__pxTotalOffsetX": 0,
        "__pxTotalOffsetY": 0,
        "__tilesetDefUid": int(layer["tilesetDefUid"]),
        "__tilesetRelPath": None,
        "levelId": level.get("uid", 0),
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "visible": True,
        "optionalRules": [],
        "seed": level.get("uid", 0),
        "overrideTilesetUid": None,
        # LDtk recomputes and overwrites the moment the source is edited, so the staleness window is
        # "until the first edit", and the tiles are a pure function of cells the file already
        # carries.
        "autoLayerTiles": [],
        # Required by the schema on every layer instance, even though an
        # AutoLayer owns none of these collections directly.
        "intGridCsv": [],
        "gridTiles": [],
        "entityInstances": [],
    }


def rule_matches(rule: dict[str, Any], value: int, cx: int, cy: int) -> bool:
    """Does this single-cell rule fire on a cell? LDtk's own test, in Python."""
    if int(rule.get("size", 1)) != 1:
        return False
    pattern = rule.get("pattern") or []
    if len(pattern) != 1 or int(pattern[0]) != value:
        return False
    x_mod = max(1, int(rule.get("xModulo", 1)))
    y_mod = max(1, int(rule.get("yModulo", 1)))
    return (cx - int(rule.get("xOffset", 0))) % x_mod == 0 and (
        cy - int(rule.get("yOffset", 0))
    ) % y_mod == 0


def bake_auto_layer_tiles(
    project: dict[str, Any], source: dict[str, Any], layer: dict[str, Any]
) -> int:
    """Write what the rules produce into the art layer's tile cache.

    See `build_art_layer_instance` for why this is written rather than left for
    the editor. The shape is LDtk's: `px` is the tile's top-left in level
    pixels, `src` its top-left in the tileset, `d` the debug pair
    `[ruleUid, coordId]` and `f` the flip bits (never set — a rule that needed a
    flip would carry a flipped tile in the atlas instead).
    """
    rules = [
        rule
        for group in layer.get("autoRuleGroups") or []
        if group.get("active", True)
        for rule in group.get("rules") or []
        if rule.get("active", True)
    ]
    tileset = next(
        (
            candidate
            for candidate in project.get("defs", {}).get("tilesets", [])
            if int(candidate.get("uid", -1)) == int(layer.get("tilesetDefUid", -1))
        ),
        None,
    )
    if tileset is None or not rules:
        return 0
    grid = int(tileset.get("tileGridSize") or CELL)
    columns = max(1, int(tileset.get("__cWid") or 1))
    cell = int(layer.get("gridSize") or CELL)
    baked = 0
    for level in project.get("levels", []):
        instances = level.get("layerInstances") or []
        art = next(
            (i for i in instances if i.get("layerDefUid") == layer.get("uid")), None
        )
        source_instance = next(
            (i for i in instances if i.get("layerDefUid") == source.get("uid")), None
        )
        if art is None or source_instance is None:
            continue
        width = int(source_instance.get("__cWid") or 0)
        tiles: list[dict[str, Any]] = []
        for index, value in enumerate(source_instance.get("intGridCsv") or []):
            if not value or not width:
                continue
            cx, cy = index % width, index // width
            for rule in rules:
                if not rule_matches(rule, int(value), cx, cy):
                    continue
                rects = rule.get("tileRectsIds") or []
                if not rects or not rects[0]:
                    break
                tile_id = int(rects[0][0])
                tiles.append(
                    {
                        "px": [cx * cell, cy * cell],
                        "src": [(tile_id % columns) * grid, (tile_id // columns) * grid],
                        "f": 0,
                        "t": tile_id,
                        "d": [int(rule["uid"]), index],
                        "a": 1.0,
                    }
                )
                if rule.get("breakOnMatch", True):
                    break
        art["autoLayerTiles"] = tiles
        baked += len(tiles)
    return baked


def dim_source_layer(source: dict[str, Any]) -> None:
    """Let the art show through the collision colours, once there IS art."""
    source["displayOpacity"] = COLLISION_OPACITY
    source["inactiveOpacity"] = COLLISION_INACTIVE_OPACITY


def add_layer_instances(project: dict[str, Any], source: dict[str, Any], layer: dict) -> None:
    for level in project.get("levels", []):
        instances = level.get("layerInstances")
        if instances is None:
            continue
        if any(inst.get("layerDefUid") == layer["uid"] for inst in instances):
            continue
        position = next(
            (
                index + 1
                for index, inst in enumerate(instances)
                if inst.get("layerDefUid") == source["uid"]
            ),
            None,
        )
        if position is None:
            continue
        instances.insert(
            position, build_art_layer_instance(level, instances[position - 1], layer)
        )


def remove_layer_instances(project: dict[str, Any], layer: dict) -> None:
    for level in project.get("levels", []):
        instances = level.get("layerInstances")
        if instances is None:
            continue
        level["layerInstances"] = [
            inst for inst in instances if inst.get("layerDefUid") != layer.get("uid")
        ]


def apply_auto_rules(
    project: dict[str, Any],
    tileset_uid: int,
    intgrid_art: dict[str, str],
    by_key: dict[str, Placement],
) -> list[str]:
    """Give every IntGrid layer with art a sibling AutoLayer that draws it."""
    messages: list[str] = []
    layers = layer_defs(project)
    for source in list(layers):
        if source.get("type") != "IntGrid":
            continue
        rules = auto_rules_for_layer(source, intgrid_art, by_key, project)
        # Undo the first shape of this tool wherever it still exists: rules and
        # a tileset on the IntGrid layer itself, which is what hid the cells.
        source["autoRuleGroups"] = [
            group
            for group in source.get("autoRuleGroups") or []
            if group.get("name") != RULE_GROUP_NAME
        ]
        if source.get("tilesetDefUid") == tileset_uid:
            source["tilesetDefUid"] = None
        identifier = art_layer_identifier(source)
        existing = next((l for l in layers if l.get("identifier") == identifier), None)
        if not rules:
            if existing is not None:
                layers.remove(existing)
                remove_layer_instances(project, existing)
                messages.append(f"{identifier}: removed (no art for its values)")
            continue
        if existing is None:
            layer = build_art_layer(project, source, tileset_uid)
            # BELOW the collision it draws for: LDtk paints the layer list from
            # the bottom up, so a later entry is further back.
            layers.insert(layers.index(source) + 1, layer)
            dim_source_layer(source)
            messages.append(f"{identifier}: created under {source['identifier']}")
        else:
            layer = existing
            layer["autoSourceLayerDefUid"] = int(source["uid"])
            layer["tilesetDefUid"] = tileset_uid
        add_layer_instances(project, source, layer)
        layer["autoRuleGroups"] = [
            group
            for group in layer.get("autoRuleGroups") or []
            if group.get("name") != RULE_GROUP_NAME
        ] + [
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
        ]
        baked = bake_auto_layer_tiles(project, source, layer)
        messages.append(
            f"{identifier}: {len(rules)} rules on {source['identifier']}, {baked} tiles baked"
        )
    return messages


def authored_field_values(project: dict[str, Any], entity_id: str, field_name: str) -> set[str]:
    """Every non-null value placements of `entity_id` carry for one field."""
    return {
        str(field["__value"])
        for level in project.get("levels") or []
        for layer in level.get("layerInstances") or []
        for instance in layer.get("entityInstances") or []
        if instance.get("__identifier") == entity_id
        for field in instance.get("fieldInstances") or []
        if field.get("__identifier") == field_name and field.get("__value") is not None
    }


def close_field_into_enum(
    project: dict[str, Any], entity_id: str, field: dict[str, Any], spec: dict[str, Any]
) -> tuple[dict[str, Any], int] | str:
    """Turn a String field into a local enum, or say why it must stay open.

     an enum holds only what it spells. A placement carrying a value the
    enum does not list would lose it, so the values already in the level are the
    gate — the same one `def upsert-entity` applies when a vocabulary manifest
    closes a field. Returns the reason as a string when it refuses.
    """
    from ambition_ldtk_tools.edit.defs import ensure_enum_def

    values = list(spec.get("values") or {})
    authored = authored_field_values(project, entity_id, field["identifier"])
    missing = sorted(authored - set(values))
    if missing:
        return (
            f"the level authors {', '.join(repr(value) for value in missing)}, which "
            f"enum {spec['enum']} does not spell"
        )
    enum = ensure_enum_def(project, str(spec["enum"]), values)
    human = f"LocalEnum.{enum['identifier']}"
    field["__type"] = human
    field["type"] = f"F_Enum({enum['uid']})"
    retyped = 0
    for level in project.get("levels") or []:
        for layer in level.get("layerInstances") or []:
            for instance in layer.get("entityInstances") or []:
                if instance.get("__identifier") != entity_id:
                    continue
                for held in instance.get("fieldInstances") or []:
                    if held.get("__identifier") != field["identifier"]:
                        continue
                    if held.get("__type") != human:
                        held["__type"] = human
                        retyped += 1
    return enum, retyped


def apply_field_display(
    project: dict[str, Any], field_display: dict[str, str]
) -> list[str]:
    """Display a selected field value beside its entity in LDtk.

    This is editor-only presentation; the runtime does not read
    `editorDisplayMode`.
    """
    messages: list[str] = []
    for path, mode in sorted(field_display.items()):
        entity_id, _, field_name = path.partition(".")
        entity = next(
            (
                ent
                for ent in project.get("defs", {}).get("entities", [])
                if ent.get("identifier") == entity_id
            ),
            None,
        )
        field = next(
            (
                f
                for f in (entity or {}).get("fieldDefs") or []
                if f.get("identifier") == field_name
            ),
            None,
        )
        if field is None:
            messages.append(f"skipped field display for missing field {path}")
            continue
        field["editorDisplayMode"] = str(mode)
        field["editorAlwaysShow"] = True
        messages.append(f"{path}: shown as {mode}")
    return messages


def apply_field_art(
    project: dict[str, Any],
    tileset_uid: int,
    field_art: dict[str, dict[str, str]],
    by_key: dict[str, Placement],
) -> list[str]:
    """Make an entity tile follow a closed enum field.

    LDtk's `EntityTile` display mode maps enum values to tiles per instance. Use
    it only for closed vocabularies; open string-like fields such as
    `EnemySpawn.brain` must remain authorable outside a fixed dropdown.

    The VALUES belong to the world's vocabulary manifest, which is what creates
    the enum; this only fills in each value's picture.
    """
    messages: list[str] = []
    for path, spec in sorted(field_art.items()):
        enums = project.get("defs", {}).setdefault("enums", [])
        art_by_value = spec["values"]
        entity_id, _, field_name = path.partition(".")
        entity = next(
            (
                ent
                for ent in project.get("defs", {}).get("entities", [])
                if ent.get("identifier") == entity_id
            ),
            None,
        )
        if entity is None:
            messages.append(f"skipped field art for missing entity def {entity_id}")
            continue
        field = next(
            (f for f in entity.get("fieldDefs") or [] if f.get("identifier") == field_name),
            None,
        )
        if field is None:
            messages.append(f"skipped field art for missing field {path}")
            continue
        enum_identifier = str(field.get("__type") or "").removeprefix("LocalEnum.")
        enum = next((e for e in enums if e.get("identifier") == enum_identifier), None)
        if enum is None and spec.get("enum"):
            # a field with no vocabulary manifest can declare its enum
            # here. `MaryOBlock.kind` is owned by `mary_o.entities.json`,
            # which is where a GAME's own noun belongs; `EnemySpawn.brain` is an
            # ENGINE def that no per-world manifest declares, and its useful
            # values are this world's roster. So the sidecar may say what the
            # words are when nothing else does — and pays the same price for it.
            outcome = close_field_into_enum(project, entity_id, field, spec)
            if isinstance(outcome, str):
                messages.append(f"REFUSED field art for {path}: {outcome}")
                continue
            enum, retyped = outcome
            enum_identifier = enum["identifier"]
            messages.append(
                f"{path}: closed into enum {enum_identifier} ({retyped} instances retyped)"
            )
        if enum is None:
            messages.append(
                f"skipped field art for {path}: its type is {field.get('__type')!r}, "
                "not a local enum — declare it as an Enum in the entity manifest first"
            )
            continue
        if spec.get("enum") and spec["enum"] != enum_identifier:
            messages.append(
                f"skipped field art for {path}: the sidecar names enum "
                f"{spec['enum']!r} but the field is {enum_identifier!r}"
            )
            continue
        painted = 0
        for value in enum.get("values") or []:
            key = art_by_value.get(str(value.get("id")))
            if key is None:
                continue
            placement = by_key[key]
            value["tileRect"] = placement.rect(tileset_uid)
            painted += 1
        enum["iconTilesetUid"] = tileset_uid
        field["editorDisplayMode"] = "EntityTile"
        messages.append(
            f"{path}: {painted} of {len(enum.get('values') or [])} "
            f"{enum_identifier} values wear their own art"
        )
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

     through the world's own `assets/sprites` mount, not the crate that
    holds the files. Every world's assets dir carries a `sprites` symlink onto
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

    sources, intgrid_art, entity_art, field_art = collect_sources(
        project, ldtk, sprites_dir, sidecar
    )
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
    # `rel_to_ldtk` resolves symlinks, so an atlas named through this world's
    # own `assets/sprites` mount comes back addressed through the OTHER game
    # that happens to own the shared tree. Say it the neighbourly way when the
    # world has its own mount — the editor reads either, but a world reaching
    # across `game/` for its own art is a lie about who owns it.
    mount = ldtk.parent.parent / "sprites"
    if mount.is_dir() and atlas_path.parent.samefile(mount):
        tileset["relPath"] = f"../sprites/{atlas_path.name}"
    messages += apply_entity_icons(project, manifest)
    messages += apply_field_art(project, int(tileset["uid"]), field_art, by_key)
    messages += apply_field_display(project, sidecar.get("field_display") or {})
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
