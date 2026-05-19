#!/usr/bin/env python3
"""Author a new Ambition LDtk active area / level from a small YAML/JSON spec.

The pain of hand-authoring an LDtk level is:

- correct entity / field `defUid` references,
- valid `intGridCsv` of the right size,
- editor-roundtrip metadata (`realEditorValues`, `__pivot`, `__smartColor`),
- placement that doesn't overlap an existing level,
- the `activeArea` level field that Ambition uses to group levels.

This tool takes a high-level spec (entity kind + grid/px position + named
fields), builds a complete level dict in memory, appends it to the existing
sandbox `.ldtk` file, then delegates the editor-roundtrip metadata fill-in
to the existing `ambition_ldtk_tools repair` pass. The resulting file is
guaranteed to validate cleanly with `ambition_ldtk_tools validate` (both
Ambition semantic + LDtk JSON schema) before this tool exits.

Spec format (YAML or JSON; YAML preferred for readability):

    id: mob_lab                    # required: activeArea string
    level_id: mob_lab              # required: level identifier
    world_x: 14000                 # required: placement
    world_y: 1024
    px_wid: 1800                   # required: level pixel size
    px_hei: 900
    grid_size: 16                  # optional, defaults to project defaultGridSize
    fill_collision: solid_border   # optional: 'empty' | 'solid_border' | 'solid_floor'
    bg_color: "#1a1a24"            # optional, defaults to project defaultLevelBgColor
    entities:
      - type: PlayerStart
        px: [60, 60]
        size: [28, 46]             # optional, defaults from defs
        fields:
          name: lab_start
      - type: Solid                # ← *static-collision* type: lowered to IntGrid
        px: [0, 800]
        size: [1800, 100]
        fields: { name: floor }
      - type: LoadingZone
        px: [0, 600]
        size: [60, 100]
        fields:
          id: lab_exit
          name: lab_exit
          activation: walk
          target_room: central_hub_complex
          target_zone: lab_door
          bidirectional: true

Field values are coerced to the type declared in `defs.entities[*].fieldDefs`
so the spec can stay loose (`true` / `1.5` / `"hello"`).

Optional biome / audio metadata (added to the project's
`defs.levelFields` by `tools/add_biome_level_fields.py`):

    biome: cave            # → biome label, drives ambient selection
    music_track: cave_loop # → MusicTrack.id from sandbox.ron
    ambient_profile: damp  # → ambient sfx / particle profile id
    visual_theme: blue     # → palette / shader-variant id

These are top-level spec keys (not under `entities:`) and are
written as level field instances. The validator/runtime treat them
as optional, so omitting any of them is safe.

Optional `connect_to:` list creates reciprocal `LoadingZone` entities
in existing target levels:

    connect_to:
      - target_room: central_hub_complex   # required
        px: [240, 600]                     # required: target-side pos
        size: [16, 96]                     # required: target-side size
        id: lab_door                       # optional: source `LoadingZone.id`
        target_zone: lab_entry             # optional: source-side LoadingZone
        activation: Door                   # optional, defaults to Door
        bidirectional: true                # optional, defaults to true

The helper rejects connecting to a missing target_room or placing the
new LoadingZone on top of an existing entity rectangle in the target
level. Bring-your-own loading zone in the spec's `entities:` list,
then declare a `connect_to` for the reciprocal back-link to skip
hand-editing the target level.

Dry-run preview
---------------

`--dry-run` builds the level entirely in memory, prints a
human-readable summary (entity counts by type, exit links, IntGrid
cell totals, reciprocal LoadingZones), and exits without writing the
file or running repair/validate. Use it before committing a spec to
the live `sandbox.ldtk` to verify the result matches intent.

Static-collision lowering
-------------------------

`Solid`, `OneWayPlatform`, and `BlinkWall` entities in the `entities:`
list are *automatically lowered* into IntGrid cells on the Collision
layer rather than emitted as entity instances on the Ambition layer.
This produces the same per-project canonical representation as
`tools/ldtk_intgrid_migration.py`:

  - The runtime collision world is identical (the rect-merge pass in
    `int_grid_value_to_block` reconstructs the same merged blocks).
  - The LDtk editor renders these as paintable IntGrid cells, so a
    human can edit the geometry per cell instead of moving big
    rectangles.
  - There is one collision representation, not two — every gameplay
    level in the project goes through IntGrid.

Use rect spec for those types: `px: [x, y], size: [w, h]`. The size
is required (the IntGrid lowering needs an explicit footprint to
paint). Other entities (PlayerStart, LoadingZone, Switch, NPC,
EncounterTrigger, LockWall, …) stay on the Ambition layer.
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

PKG_DIR = Path(__file__).resolve().parent


def load_spec(path: Path) -> dict:
    text = path.read_text()
    if path.suffix.lower() in {".yaml", ".yml"}:
        try:
            import yaml  # type: ignore
        except ImportError as ex:  # pragma: no cover
            raise SystemExit(f"YAML spec but pyyaml not installed: {ex}")
        return yaml.safe_load(text)
    return json.loads(text)


def load_project(path: Path) -> dict:
    return json.loads(path.read_text())


def write_project(path: Path, project: dict) -> None:
    from .editor_format import dump_editor_style

    path.write_text(dump_editor_style(project))


def find_entity_def(project: dict, identifier: str) -> dict:
    for ed in project["defs"]["entities"]:
        if ed.get("identifier") == identifier:
            return ed
    known = sorted(e["identifier"] for e in project["defs"]["entities"])
    suggestion = _closest_match(identifier, known)
    msg = f"unknown entity identifier '{identifier}'."
    if suggestion:
        msg += f" Did you mean '{suggestion}'?"
    msg += " Known identifiers: " + ", ".join(known)
    raise SystemExit(msg)


def _closest_match(name: str, candidates: list[str]) -> str | None:
    """Return the closest case-insensitive match using a similarity ratio.

    The author tool is hand-driven by agents, so misspellings are common.
    A best-effort suggestion saves a re-prompt cycle. Uses difflib for
    typo-style matches (one or two char edits) and falls back to
    prefix/substring for rare cases.
    """
    if not candidates:
        return None
    import difflib

    lname = name.lower()
    for c in candidates:
        if c.lower() == lname:
            return c
    matches = difflib.get_close_matches(
        lname, [c.lower() for c in candidates], n=1, cutoff=0.6
    )
    if matches:
        for c in candidates:
            if c.lower() == matches[0]:
                return c
    for c in candidates:
        lc = c.lower()
        if lc.startswith(lname) or lname.startswith(lc):
            return c
    for c in candidates:
        if lname in c.lower() or c.lower() in lname:
            return c
    return None


def find_level(project: dict, level_id: str) -> dict | None:
    """Locate a level by `identifier`. Returns `None` if missing."""
    for lev in project.get("levels", []):
        if lev.get("identifier") == level_id:
            return lev
    return None


def find_layer_in_level(level: dict, layer_identifier: str) -> dict | None:
    for layer in level.get("layerInstances", []):
        if layer.get("__identifier") == layer_identifier:
            return layer
    return None


def find_layer_def(project: dict, identifier: str) -> dict:
    for ld in project["defs"]["layers"]:
        if ld.get("identifier") == identifier:
            return ld
    raise SystemExit(f"missing layer def '{identifier}' in project")


def find_layer_def_optional(project: dict, identifier: str) -> dict | None:
    for ld in project["defs"]["layers"]:
        if ld.get("identifier") == identifier:
            return ld
    return None


def ensure_climbable_layer_def(project: dict) -> dict:
    """Ensure the project has a Climbable IntGrid layer def. If it
    doesn't, add one (mirroring the Water layer's shape but with
    Ladder/Vine/Wall intGridValues) and add an empty Climbable layer
    instance to every existing level so the schema stays consistent.

    Returns the layer def. Idempotent: if the def already exists,
    returns it without modifying the project.

    Mirrors the runtime's Climbable IntGrid value mapping in
    `crates/ambition_sandbox/src/ldtk_world.rs`:
        1 = Ladder, 2 = Vine, 3 = Wall
    """
    existing = find_layer_def_optional(project, "Climbable")
    if existing is not None:
        return existing

    # Allocate a fresh uid by bumping nextUid.
    next_uid = int(project.get("nextUid", 1))
    project["nextUid"] = next_uid + 1
    grid_size = int(project.get("defaultGridSize", 16))

    layer_def = {
        "__type": "IntGrid",
        "identifier": "Climbable",
        "type": "IntGrid",
        "uid": next_uid,
        "doc": "Climbable surfaces: 1=Ladder, 2=Vine, 3=Wall.",
        "uiColor": None,
        "gridSize": grid_size,
        "guideGridWid": 0,
        "guideGridHei": 0,
        "displayOpacity": 0.6,
        "inactiveOpacity": 0.4,
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
        "intGridValues": [
            {"value": 1, "identifier": "Ladder", "color": "#C28447", "tile": None, "groupUid": 0},
            {"value": 2, "identifier": "Vine", "color": "#5FA452", "tile": None, "groupUid": 0},
            {"value": 3, "identifier": "Wall", "color": "#9B7A4A", "tile": None, "groupUid": 0},
        ],
        "intGridValuesGroups": [],
        "autoRuleGroups": [],
        "autoSourceLayerDefUid": None,
        "tilesetDefUid": None,
        "tilePivotX": 0,
        "tilePivotY": 0,
        "biomeFieldUid": None,
    }
    project["defs"]["layers"].append(layer_def)

    # Add an empty Climbable layer instance to every existing level so
    # the layer schema stays consistent across the project. Levels
    # without ladders just have a Climbable layer of all-zero IntGrid
    # cells.
    for level in project.get("levels", []):
        if any(
            lyr.get("__identifier") == "Climbable"
            for lyr in level.get("layerInstances", [])
        ):
            continue
        c_wid = level["pxWid"] // grid_size
        c_hei = level["pxHei"] // grid_size
        # Allocate a fresh iid for this layer instance.
        layer_iid, _ = allocate_iid(project, "Climbable")
        empty_layer = {
            "__identifier": "Climbable",
            "__type": "IntGrid",
            "iid": layer_iid,
            "layerDefUid": layer_def["uid"],
            "intGridCsv": [0] * (c_wid * c_hei),
            "__cWid": c_wid,
            "__cHei": c_hei,
            "__gridSize": grid_size,
            "__opacity": 1,
            "__pxTotalOffsetX": 0,
            "__pxTotalOffsetY": 0,
            "__tilesetDefUid": None,
            "__tilesetRelPath": None,
            "levelId": level["uid"],
            "pxOffsetX": 0,
            "pxOffsetY": 0,
            "visible": True,
            "optionalRules": [],
            "autoLayerTiles": [],
            "seed": level["uid"],
            "overrideTilesetUid": None,
            "gridTiles": [],
            "entityInstances": [],
        }
        level.setdefault("layerInstances", []).append(empty_layer)
    return layer_def


CLIMBABLE_INTGRID_VALUES = {
    "Ladder": 1,
    "Vine": 2,
    "Wall": 3,
}


def paint_climbable_layer(
    csv: list[int],
    c_wid: int,
    c_hei: int,
    grid_size: int,
    cells: list[dict],
) -> int:
    """Paint Climbable IntGrid cells from a list of {kind, px, size}
    rectangles. `kind` must be one of "Ladder", "Vine", "Wall".
    Returns the count of cells painted across all rects.
    """
    painted = 0
    for cell in cells:
        kind = cell.get("kind")
        value = CLIMBABLE_INTGRID_VALUES.get(kind)
        if value is None:
            raise SystemExit(
                f"climbable cell missing or invalid 'kind' (got {kind!r}); "
                f"must be one of {sorted(CLIMBABLE_INTGRID_VALUES)}"
            )
        px = cell.get("px")
        size = cell.get("size")
        if px is None or len(px) != 2:
            raise SystemExit(f"climbable cell {kind} missing 'px: [x, y]'")
        if size is None or len(size) != 2:
            raise SystemExit(f"climbable cell {kind} missing 'size: [w, h]'")
        painted += paint_intgrid_rect(
            csv,
            c_wid,
            c_hei,
            grid_size,
            int(px[0]),
            int(px[1]),
            int(size[0]),
            int(size[1]),
            value,
        )
    return painted


def coerce_field_value(human_type: str, raw):
    """Coerce a YAML-loaded value to the LDtk parser-facing type."""
    if raw is None:
        return None
    if human_type == "String":
        return str(raw)
    if human_type == "Bool":
        if isinstance(raw, bool):
            return raw
        if isinstance(raw, str):
            return raw.lower() in {"true", "yes", "1"}
        return bool(raw)
    if human_type in {"Int"}:
        return int(raw)
    if human_type in {"Float"}:
        return float(raw)
    # Fall back to whatever LDtk's repair / validator wants; pass through.
    return raw


def make_field_instance(field_def: dict, value):
    """Build a minimal field instance. `ambition_ldtk_tools repair` will fill
    editor metadata for common types when it is missing/stale."""
    instance = {
        "__identifier": field_def["identifier"],
        "__type": field_def.get("__type"),
        "__value": value,
        "__tile": None,
        "defUid": field_def["uid"],
    }
    return instance


def build_active_area_field(project: dict, area_id: str) -> dict:
    """Build the `activeArea` String field instance on a level."""
    level_field = None
    for f in project["defs"].get("levelFields", []):
        if f.get("identifier") == "activeArea":
            level_field = f
            break
    if level_field is None:
        raise SystemExit(
            "project is missing the `activeArea` level field def; "
            "this tool only supports the standard Ambition project shape"
        )
    return make_field_instance(level_field, area_id)


# Optional level-field identifiers handled by `build_level_field_instances`.
# These map directly to `defs.levelFields` entries created by
# `tools/add_biome_level_fields.py`. Specs may set any subset; missing
# fields are simply not emitted as level field instances.
OPTIONAL_LEVEL_FIELDS = ("biome", "music_track", "ambient_profile", "visual_theme")


def build_level_field_instances(project: dict, spec: dict) -> list[dict]:
    """Build level field instances for `activeArea` plus the optional
    biome / music / ambient / visual seam.

    The biome seam fields are looked up dynamically from
    `defs.levelFields`. If a spec sets one of them but the project is
    missing the corresponding level field def (i.e. the migration
    wasn't run), the helper raises a clear error pointing at the
    migration script instead of silently dropping the value.
    """
    instances = [build_active_area_field(project, spec["id"])]
    level_fields = {f.get("identifier"): f for f in project["defs"].get("levelFields") or []}
    for ident in OPTIONAL_LEVEL_FIELDS:
        if ident not in spec:
            continue
        value = spec[ident]
        if value is None:
            continue
        field_def = level_fields.get(ident)
        if field_def is None:
            raise SystemExit(
                f"spec sets level field '{ident}' but the project has no "
                f"matching levelField def. Run "
                f"`python tools/add_biome_level_fields.py <ldtk>` first."
            )
        coerced = coerce_field_value(field_def.get("__type", "String"), value)
        instances.append(make_field_instance(field_def, coerced))
    return instances


def make_intgrid_csv(c_wid: int, c_hei: int, fill: str) -> list[int]:
    """Build the IntGrid `intGridCsv` array for a Collision layer.

    fill modes:
      empty         — all zero (no collision; entities provide all geometry).
      solid_border  — value 1 (Solid) on the outer ring, 0 inside.
      solid_floor   — value 1 on the bottom row only.
    """
    csv = [0] * (c_wid * c_hei)
    if fill == "empty":
        return csv
    if fill == "solid_floor":
        for x in range(c_wid):
            csv[(c_hei - 1) * c_wid + x] = 1
        return csv
    if fill == "solid_border":
        for x in range(c_wid):
            csv[x] = 1
            csv[(c_hei - 1) * c_wid + x] = 1
        for y in range(c_hei):
            csv[y * c_wid] = 1
            csv[y * c_wid + c_wid - 1] = 1
        return csv
    raise SystemExit(f"unknown fill_collision mode '{fill}'")


# IntGrid value mapping. Mirrored from `tools/ldtk_intgrid_migration.py`
# so the authoring path and the migration path agree on what each value
# means. Keep both in sync if values change.
INTGRID_VALUE_SOLID = 1
INTGRID_VALUE_ONE_WAY = 2
INTGRID_VALUE_BLINK_SOFT = 3
INTGRID_VALUE_BLINK_HARD = 4
INTGRID_VALUE_HAZARD = 5


def entity_to_intgrid_value(ent_spec: dict) -> int | None:
    """Return the IntGrid value a static-collision entity should be
    *lowered* to, or `None` for entities that stay as entity instances.

    The runtime treats IntGrid-derived blocks and entity-derived
    Solid/OneWay/Blink/Hazard blocks as collision-equivalent (after
    the rectangle-merge pass in `int_grid_value_to_block`), but
    IntGrid is the canonical representation across the project:
    every level is on IntGrid, and the LDtk editor handles per-cell
    painting for free. Authoring static surfaces as entities just
    to have the tool re-emit them as entities is a needless detour,
    so this hook auto-lowers them at build time.

    Note: `DamageVolume` is intentionally NOT lowered — those
    entities can carry motion paths (`path_points`/`path_speed`)
    and per-volume damage that IntGrid cells can't represent.
    Use HazardBlock for static damage surfaces and DamageVolume
    only for moving / variable-damage hazards.
    """
    ident = ent_spec.get("type")
    if ident == "Solid":
        return INTGRID_VALUE_SOLID
    if ident == "OneWayPlatform":
        return INTGRID_VALUE_ONE_WAY
    if ident == "BlinkWall":
        tier = (ent_spec.get("fields") or {}).get("tier", "Soft")
        return INTGRID_VALUE_BLINK_HARD if str(tier) == "Hard" else INTGRID_VALUE_BLINK_SOFT
    if ident == "HazardBlock":
        return INTGRID_VALUE_HAZARD
    return None


def paint_intgrid_rect(
    csv: list[int],
    c_wid: int,
    c_hei: int,
    grid_size: int,
    px: int,
    py: int,
    width: int,
    height: int,
    value: int,
) -> int:
    """Paint `value` into every IntGrid cell that overlaps the px-space
    rect `[px, py, px+width, py+height)`. Mirror of
    `tools/ldtk_intgrid_migration.fill_cells` so authoring and
    migration produce byte-identical CSVs for the same input rect.
    Returns the count of cells painted."""
    cx0 = px // grid_size
    cy0 = py // grid_size
    cx1 = (px + width + grid_size - 1) // grid_size
    cy1 = (py + height + grid_size - 1) // grid_size
    painted = 0
    for cy in range(cy0, cy1):
        for cx in range(cx0, cx1):
            if 0 <= cx < c_wid and 0 <= cy < c_hei:
                csv[cy * c_wid + cx] = value
                painted += 1
    return painted


def allocate_iid(project: dict, identifier: str) -> tuple[str, int]:
    """Return a fresh `<Identifier>-NNNN` iid and bump the project's nextUid.

    Ambition's existing iids use this short form (not full UUIDs) and the
    validator/loader accept it. The integer suffix is taken from `nextUid`
    so it doesn't collide.
    """
    next_uid = int(project.get("nextUid", 1))
    project["nextUid"] = next_uid + 1
    return f"{identifier}-{next_uid:04d}", next_uid


def build_entity_instance(
    project: dict,
    ent_spec: dict,
    grid_size: int,
    level_world_x: int = 0,
    level_world_y: int = 0,
) -> dict:
    identifier = ent_spec["type"]
    ent_def = find_entity_def(project, identifier)
    px = ent_spec.get("px")
    if px is None:
        raise SystemExit(f"entity '{identifier}' missing required 'px'")
    if len(px) != 2:
        raise SystemExit(f"entity '{identifier}' px must be [x, y]")
    width = int(ent_spec.get("size", [ent_def.get("width", 16), ent_def.get("height", 16)])[0])
    height = int(ent_spec.get("size", [ent_def.get("width", 16), ent_def.get("height", 16)])[1])

    # Grid coordinate is px / gridSize (LDtk's convention).
    grid_x = int(px[0]) // grid_size
    grid_y = int(px[1]) // grid_size

    iid, _ = allocate_iid(project, identifier)
    instance = {
        "__identifier": identifier,
        "__grid": [grid_x, grid_y],
        "__pivot": [0, 0],
        "__tags": [],
        "__tile": None,
        # `__smartColor` is required by the official LDtk JSON schema even
        # for editor-roundtrip-clean files. Pull from the entity def's
        # color (Ambition entity defs always set one); fall back to white.
        "__smartColor": ent_def.get("color", "#FFFFFF"),
        # `__worldX` / `__worldY` are LDtk-computed cached fields that
        # downstream consumers (notably `bevy_ecs_ldtk`) use to position
        # the entity in world space without recomputing
        # `level.worldX + entity.px`. Missing them caused the new
        # basement-door LoadingZones to render at the world (0,0)-rooted
        # frame in 2026-05-07, layering on top of central_hub_main
        # instead of below it. Always populate so the editor-roundtrip
        # invariant holds.
        "__worldX": level_world_x + int(px[0]),
        "__worldY": level_world_y + int(px[1]),
        "iid": iid,
        "width": width,
        "height": height,
        "defUid": ent_def["uid"],
        "px": [int(px[0]), int(px[1])],
        "fieldInstances": [],
    }

    # Build field instances. The spec can omit fields — `ambition_ldtk_tools repair`
    # plus the Ambition validator both tolerate missing fields on most entity
    # types; we simply emit instances for the fields the spec provided.
    spec_fields = dict(ent_spec.get("fields") or {})
    if "name" in ent_spec and "name" not in spec_fields:
        # Convenience: top-level `name:` is treated as a fields.name.
        spec_fields = {"name": ent_spec["name"], **spec_fields}

    # Strict-but-helpful field validation: an unknown field is almost
    # always a typo (see `lab_door` vs `lock_door` historically). Catch
    # it at build time with a suggestion instead of producing an LDtk
    # file that round-trips but silently drops the value.
    known_fields = [f["identifier"] for f in ent_def.get("fieldDefs", [])]
    for fname in spec_fields:
        if fname in known_fields:
            continue
        suggestion = _closest_match(fname, known_fields)
        msg = (
            f"entity '{identifier}' has no field '{fname}'."
        )
        if suggestion:
            msg += f" Did you mean '{suggestion}'?"
        if known_fields:
            msg += " Known: " + ", ".join(known_fields)
        raise SystemExit(msg)

    for field_def in ent_def.get("fieldDefs", []):
        fname = field_def["identifier"]
        if fname not in spec_fields:
            continue
        try:
            value = coerce_field_value(field_def.get("__type", "String"), spec_fields[fname])
        except (ValueError, TypeError) as ex:
            raise SystemExit(
                f"entity '{identifier}' field '{fname}' expects {field_def.get('__type')!r}; "
                f"could not coerce {spec_fields[fname]!r}: {ex}"
            )
        instance["fieldInstances"].append(make_field_instance(field_def, value))
    return instance


def build_level(project: dict, spec: dict) -> dict:
    level_id = spec["level_id"]
    area_id = spec["id"]
    world_x = int(spec["world_x"])
    world_y = int(spec["world_y"])
    px_wid = int(spec["px_wid"])
    px_hei = int(spec["px_hei"])
    grid_size = int(spec.get("grid_size", project.get("defaultGridSize", 16)))
    bg_color = spec.get("bg_color", project.get("defaultLevelBgColor", "#000000"))

    if px_wid % grid_size or px_hei % grid_size:
        raise SystemExit(
            f"px_wid ({px_wid}) and px_hei ({px_hei}) must be multiples of grid_size ({grid_size})"
        )

    # Reject overlap with existing levels in the world frame so the tool
    # never silently squats on existing geometry.
    for lev in project.get("levels", []):
        if (
            world_x < lev["worldX"] + lev["pxWid"]
            and world_x + px_wid > lev["worldX"]
            and world_y < lev["worldY"] + lev["pxHei"]
            and world_y + px_hei > lev["worldY"]
        ):
            raise SystemExit(
                f"new level at ({world_x},{world_y}) {px_wid}x{px_hei} overlaps "
                f"existing level '{lev['identifier']}' at ({lev['worldX']},{lev['worldY']}) "
                f"{lev['pxWid']}x{lev['pxHei']}"
            )

    c_wid = px_wid // grid_size
    c_hei = px_hei // grid_size
    fill = spec.get("fill_collision", "empty")
    csv = make_intgrid_csv(c_wid, c_hei, fill)

    collision_def = find_layer_def(project, "Collision")
    ambition_def = find_layer_def(project, "Ambition")

    level_iid, level_uid = allocate_iid(project, level_id)
    collision_iid, _ = allocate_iid(project, "Collision")
    ambition_iid, _ = allocate_iid(project, "Ambition")

    # Split entities into "stays as an entity" vs "lower into IntGrid".
    # Solid / OneWayPlatform / BlinkWall belong on the Collision layer;
    # everything else stays on the Ambition entity layer. This keeps
    # the spec ergonomic (author by rect) while producing the same
    # canonical IntGrid representation as `ldtk_intgrid_migration.py`.
    entity_instances: list[dict] = []
    lowered_count = 0
    lowered_cells = 0
    for ent_spec in spec.get("entities", []):
        value = entity_to_intgrid_value(ent_spec)
        if value is None:
            entity_instances.append(build_entity_instance(
                project, ent_spec, grid_size, world_x, world_y,
            ))
            continue
        px = ent_spec.get("px")
        if px is None or len(px) != 2:
            raise SystemExit(
                f"static-collision entity '{ent_spec.get('type')}' missing required 'px: [x, y]'"
            )
        size = ent_spec.get("size")
        if size is None or len(size) != 2:
            raise SystemExit(
                f"static-collision entity '{ent_spec.get('type')}' missing required 'size: [w, h]' "
                "(IntGrid lowering needs an explicit footprint)"
            )
        lowered_cells += paint_intgrid_rect(
            csv,
            c_wid,
            c_hei,
            grid_size,
            int(px[0]),
            int(px[1]),
            int(size[0]),
            int(size[1]),
            value,
        )
        lowered_count += 1
    if lowered_count:
        print(
            f"  lowered {lowered_count} static-collision entit{'y' if lowered_count == 1 else 'ies'} "
            f"into {lowered_cells} IntGrid cells"
        )

    base_layer = {
        "__cWid": c_wid,
        "__cHei": c_hei,
        "__gridSize": grid_size,
        "__opacity": 1,
        "__pxTotalOffsetX": 0,
        "__pxTotalOffsetY": 0,
        "__tilesetDefUid": None,
        "__tilesetRelPath": None,
        "levelId": level_uid,
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "visible": True,
        "optionalRules": [],
        "autoLayerTiles": [],
        "seed": level_uid,
        "overrideTilesetUid": None,
        "gridTiles": [],
        "entityInstances": [],
    }

    collision_layer = {
        "__identifier": "Collision",
        "__type": "IntGrid",
        "iid": collision_iid,
        "layerDefUid": collision_def["uid"],
        "intGridCsv": csv,
        **base_layer,
    }
    # base_layer's `entityInstances` is empty; correct for Collision.
    collision_layer["entityInstances"] = []
    # Override the shared `iid`/`layerDefUid` (Python dict merge order keeps
    # ours first because we expanded `**base_layer` last; re-set below).
    collision_layer["iid"] = collision_iid
    collision_layer["layerDefUid"] = collision_def["uid"]

    ambition_layer = {
        "__identifier": "Ambition",
        "__type": "Entities",
        "iid": ambition_iid,
        "layerDefUid": ambition_def["uid"],
        "intGridCsv": [],
        **base_layer,
    }
    ambition_layer["iid"] = ambition_iid
    ambition_layer["layerDefUid"] = ambition_def["uid"]
    ambition_layer["entityInstances"] = entity_instances

    # Optional Climbable IntGrid layer. When the spec has an
    # `intgrid.climbable` block, we ensure the project has the
    # Climbable layer def (idempotent — already there if a previous
    # apply ran), then paint the spec's cells onto a new layer
    # instance for this level. Levels that don't author climbables
    # still get a Climbable layer instance via
    # `ensure_climbable_layer_def`'s migration pass.
    climbable_cells = (spec.get("intgrid") or {}).get("climbable") or []
    if climbable_cells:
        climbable_def = ensure_climbable_layer_def(project)
    else:
        climbable_def = find_layer_def_optional(project, "Climbable")

    layer_instances = [collision_layer, ambition_layer]
    if climbable_def is not None:
        climbable_iid, _ = allocate_iid(project, "Climbable")
        climb_csv = [0] * (c_wid * c_hei)
        if climbable_cells:
            painted = paint_climbable_layer(
                climb_csv, c_wid, c_hei, grid_size, climbable_cells
            )
            print(f"  painted {painted} Climbable IntGrid cells")
        climbable_layer = {
            "__identifier": "Climbable",
            "__type": "IntGrid",
            "iid": climbable_iid,
            "layerDefUid": climbable_def["uid"],
            "intGridCsv": climb_csv,
            **base_layer,
        }
        climbable_layer["iid"] = climbable_iid
        climbable_layer["layerDefUid"] = climbable_def["uid"]
        climbable_layer["entityInstances"] = []
        layer_instances.append(climbable_layer)

    level = {
        "identifier": level_id,
        "iid": level_iid,
        "uid": level_uid,
        "worldX": world_x,
        "worldY": world_y,
        "worldDepth": 0,
        "pxWid": px_wid,
        "pxHei": px_hei,
        "__bgColor": bg_color,
        "bgColor": bg_color,
        "useAutoIdentifier": False,
        "bgRelPath": None,
        "bgPos": None,
        "bgPivotX": 0.5,
        "bgPivotY": 0.5,
        "__smartColor": "#FFFFFF",
        "__bgPos": None,
        "externalRelPath": None,
        "fieldInstances": build_level_field_instances(project, spec),
        "layerInstances": layer_instances,
        "__neighbours": [],
    }
    return level


def summarize_level(level: dict, lowered_count: int = 0, lowered_cells: int = 0) -> str:
    """Build a one-screen human-readable summary of a level for previews.

    The summary is the same shape `--dry-run` prints and what the live
    path prints to stderr after writing the file, so an agent reading
    either output can extract the same facts: identifier, footprint,
    entity counts by type, IntGrid lowering, and exit links.
    """
    lines: list[str] = []
    lines.append(
        f"level '{level['identifier']}' "
        f"(activeArea '{_level_field(level, 'activeArea') or '?'}'): "
        f"{level['pxWid']}x{level['pxHei']} at ({level['worldX']},{level['worldY']})"
    )
    biome_bits = []
    for ident in OPTIONAL_LEVEL_FIELDS:
        v = _level_field(level, ident)
        if v:
            biome_bits.append(f"{ident}={v}")
    if biome_bits:
        lines.append("  metadata: " + ", ".join(biome_bits))
    ambition = find_layer_in_level(level, "Ambition")
    if ambition is None:
        lines.append("  (no Ambition entity layer found)")
    else:
        per_kind: dict[str, list[dict]] = {}
        for inst in ambition.get("entityInstances", []):
            per_kind.setdefault(inst["__identifier"], []).append(inst)
        for kind in sorted(per_kind):
            lines.append(f"  {kind}: {len(per_kind[kind])}")
            for inst in per_kind[kind]:
                fields = {
                    f["__identifier"]: f.get("__value")
                    for f in inst.get("fieldInstances", [])
                }
                tag = fields.get("name") or fields.get("id") or inst["iid"]
                exit_info = ""
                if kind == "LoadingZone":
                    target = fields.get("target_room")
                    target_zone = fields.get("target_zone")
                    activation = fields.get("activation")
                    bidir = fields.get("bidirectional")
                    exit_info = (
                        f"  → {target}/{target_zone} "
                        f"({activation}{'/bi' if bidir else ''})"
                    )
                lines.append(
                    f"    - {tag} px={inst['px']} size=({inst['width']}x{inst['height']})"
                    + exit_info
                )
    if lowered_count:
        lines.append(
            f"  IntGrid: lowered {lowered_count} static-collision rects "
            f"into {lowered_cells} cells"
        )
    return "\n".join(lines)


def _level_field(level: dict, identifier: str):
    """Read a top-level field instance value by identifier, or `None`."""
    for f in level.get("fieldInstances", []):
        if f.get("__identifier") == identifier:
            return f.get("__value")
    return None


def add_reciprocal_loading_zone(
    project: dict, connection: dict, source_level_id: str
) -> dict:
    """Insert a reciprocal `LoadingZone` into an existing target level.

    The connection spec describes the *source-side* loading zone the
    new level already declares (by id) and the geometry of the
    target-side companion. The helper finds the target level by
    identifier, builds a fresh `LoadingZone` entity instance pointing
    back at the source level, and appends it to the target level's
    Ambition layer. Returns the new entity instance for preview.

    Required spec keys:
      target_room: identifier of the existing level to extend.
      px:          [x, y] of the new loading zone in target-level coords.
      size:        [w, h] of the new loading zone (16x96 is a common door).

    Optional spec keys:
      id:               loading zone id (defaults to `<source>_return`).
      name:             entity name (defaults to id).
      target_zone:      the source-side `LoadingZone.id` to point at
                        (defaults to `<source_level_id>_entry`).
      activation:       'walk' | 'Door' (defaults to 'Door').
      bidirectional:    bool (defaults to true).

    Validation: the helper rejects connections to a missing target
    level, a missing Ambition layer in the target, or a placement
    that overlaps any existing entity in the target's Ambition layer.
    """
    target_room = connection.get("target_room")
    if not target_room:
        raise SystemExit("connect_to entry missing required 'target_room'")
    target_level = find_level(project, target_room)
    if target_level is None:
        known = ", ".join(sorted(l["identifier"] for l in project.get("levels", [])))
        raise SystemExit(
            f"connect_to target_room '{target_room}' not found. Known levels: {known}"
        )
    ambition = find_layer_in_level(target_level, "Ambition")
    if ambition is None:
        raise SystemExit(
            f"connect_to target '{target_room}' has no Ambition entity layer; "
            "cannot append a reciprocal LoadingZone"
        )
    px = connection.get("px")
    size = connection.get("size")
    if px is None or len(px) != 2:
        raise SystemExit("connect_to entry missing required 'px: [x, y]'")
    if size is None or len(size) != 2:
        raise SystemExit("connect_to entry missing required 'size: [w, h]'")

    new_x, new_y = int(px[0]), int(px[1])
    new_w, new_h = int(size[0]), int(size[1])

    # Optional: snap door y to the nearest Collision surface so the
    # door visually rests on a floor instead of hovering in mid-air.
    # Authors who already know the right y leave the flag off and the
    # snap is a no-op.
    if connection.get("snap_to_surface"):
        snapped_x, snapped_y, surface_kind = snap_door_to_surface(
            project,
            target_room,
            new_x,
            door_w=new_w,
            door_h=new_h,
            prefer_y=new_y,
        )
        if snapped_y != new_y:
            print(
                f"connect_to: snapped door y in '{target_room}' from {new_y} to "
                f"{snapped_y} (rests on {surface_kind} at x={snapped_x})"
            )
        new_x, new_y = snapped_x, snapped_y

    # Reject overlap with any existing entity rect in the target level.
    for inst in ambition.get("entityInstances", []):
        ix, iy = inst["px"]
        iw, ih = inst["width"], inst["height"]
        if (
            new_x < ix + iw
            and new_x + new_w > ix
            and new_y < iy + ih
            and new_y + new_h > iy
        ):
            raise SystemExit(
                f"connect_to placement overlaps existing entity '{inst['__identifier']}' "
                f"in '{target_room}' at ({ix},{iy}) {iw}x{ih}"
            )

    source_id = connection.get("id") or f"{source_level_id}_return"
    target_zone = connection.get("target_zone") or f"{source_level_id}_entry"
    activation = str(connection.get("activation", "Door"))
    bidirectional = bool(connection.get("bidirectional", True))

    grid_size = int(project.get("defaultGridSize", 16))
    ent_spec = {
        "type": "LoadingZone",
        "px": [new_x, new_y],
        "size": [new_w, new_h],
        "fields": {
            "id": source_id,
            "name": connection.get("name", source_id),
            "activation": activation,
            "target_room": source_level_id,
            "target_zone": target_zone,
            "bidirectional": bidirectional,
        },
    }
    # Reciprocal LoadingZones live INSIDE the target level, so their
    # `__worldX` / `__worldY` are computed against the target's
    # worldX / worldY (NOT the source level's).
    target_world_x = int(target_level.get("worldX", 0))
    target_world_y = int(target_level.get("worldY", 0))
    instance = build_entity_instance(
        project, ent_spec, grid_size, target_world_x, target_world_y,
    )
    ambition.setdefault("entityInstances", []).append(instance)
    return instance


def run_repair_and_validate(project_path: Path, schema: Path | None) -> int:
    """Run the existing repair + validator scripts and forward their exit code."""
    cmd_repair = [sys.executable, "-m", "ambition_ldtk_tools.repair", str(project_path), "--in-place"]
    print(f"$ {' '.join(cmd_repair)}")
    r = subprocess.run(cmd_repair)
    if r.returncode != 0:
        return r.returncode
    cmd_val = [sys.executable, "-m", "ambition_ldtk_tools.validate", str(project_path)]
    if schema is not None:
        cmd_val.extend(["--schema", str(schema), "--require-schema"])
    print(f"$ {' '.join(cmd_val)}")
    return subprocess.run(cmd_val).returncode


# Collision IntGrid value mapping. Keep in sync with the project's
# Collision layer defs in `sandbox.ldtk`. The sandbox treats Solid (1)
# and OneWayUp (2) as floors a door can rest on; BlinkSoft / BlinkHard
# / Hazard are intentionally excluded as door-bases (BlinkWalls move,
# Hazards damage you).
_COLLISION_SOLID = 1
_COLLISION_ONEWAY_UP = 2
_DOOR_SUPPORTING_VALUES = frozenset({_COLLISION_SOLID, _COLLISION_ONEWAY_UP})


def snap_door_to_surface(
    project: dict,
    target_room: str,
    x: int,
    door_w: int = 48,
    door_h: int = 96,
    prefer_y: int | None = None,
) -> tuple[int, int, str]:
    """Find the door y so a `door_w × door_h` rect rests flush on a surface.

    Reads the target room's Collision IntGrid and looks for the topmost
    cell row R where every cell column the door spans (`x..x+door_w`)
    contains a Solid or OneWayUp value. The door's bottom edge then
    lands at `R * gridSize`, so the returned door y is `R*gridSize - door_h`.

    When `prefer_y` is given, the surface closest to (but at or below)
    `prefer_y + door_h` wins — useful when authors already had a y in
    mind from `door free-spots` and want the snap to honor that row's
    intent rather than always picking the highest reachable surface.

    Returns `(x, snapped_y, surface_kind)` where surface_kind is
    'Solid' or 'OneWayUp'. Raises `SystemExit` if no continuous surface
    exists in the door's column range.
    """
    target_level = find_level(project, target_room)
    if target_level is None:
        known = ", ".join(sorted(l["identifier"] for l in project.get("levels", [])))
        raise SystemExit(
            f"snap target_room '{target_room}' not found. Known levels: {known}"
        )
    collision = find_layer_in_level(target_level, "Collision")
    if collision is None:
        raise SystemExit(f"'{target_room}' has no Collision IntGrid layer")
    grid_size = int(collision.get("__gridSize", 16))
    c_wid = int(collision["__cWid"])
    c_hei = int(collision["__cHei"])
    csv = collision.get("intGridCsv") or []
    if len(csv) != c_wid * c_hei:
        raise SystemExit(
            f"'{target_room}' Collision csv length {len(csv)} != cWid*cHei={c_wid * c_hei}"
        )

    cx_start = x // grid_size
    cx_end_excl = (x + door_w + grid_size - 1) // grid_size
    if cx_start < 0 or cx_end_excl > c_wid:
        raise SystemExit(
            f"door x={x} (cells {cx_start}..{cx_end_excl - 1}) is outside the room's "
            f"collision grid (0..{c_wid - 1})"
        )

    # Walk every row top-down. The first row whose cells span the
    # door's column range with all Solid/OneWayUp wins. We also need
    # the rows *above* the surface (where the door body sits) to be
    # empty — otherwise the door geometry intersects level geometry.
    door_rows = max(1, door_h // grid_size)
    candidates: list[tuple[int, str]] = []
    for r in range(c_hei):
        row_cells = [csv[r * c_wid + c] for c in range(cx_start, cx_end_excl)]
        if not all(v in _DOOR_SUPPORTING_VALUES for v in row_cells):
            continue
        # Surface row at r. Door body occupies rows [r - door_rows, r-1].
        body_start = r - door_rows
        if body_start < 0:
            continue
        body_clear = True
        for body_r in range(body_start, r):
            for c in range(cx_start, cx_end_excl):
                if csv[body_r * c_wid + c] != 0:
                    body_clear = False
                    break
            if not body_clear:
                break
        if not body_clear:
            continue
        kind = "Solid" if all(v == _COLLISION_SOLID for v in row_cells) else "OneWayUp"
        candidates.append((r, kind))

    if not candidates:
        raise SystemExit(
            f"snap_to_surface: no continuous Solid/OneWayUp surface under door at "
            f"x={x} (cells {cx_start}..{cx_end_excl - 1}) wide enough for {door_w}x{door_h}"
        )

    if prefer_y is None:
        # Default: pick the LOWEST surface (largest r). Authors usually
        # mean "this door sits on the floor", not "this door dangles
        # off the ceiling".
        chosen_r, chosen_kind = candidates[-1]
    else:
        # Pick the candidate whose snapped door y is closest to prefer_y.
        chosen_r, chosen_kind = min(
            candidates,
            key=lambda rk: abs((rk[0] * grid_size - door_h) - prefer_y),
        )
    snapped_y = chosen_r * grid_size - door_h
    return (x, snapped_y, chosen_kind)


def even_space_entities(
    project: dict,
    target_room: str,
    identifier: str,
    y_row: int | None = None,
    y_tolerance: int = 32,
    start_x: int | None = None,
    end_x: int | None = None,
    strategy: str = "preserve-ends",
    snap_to_grid: bool = False,
    dry_run: bool = False,
) -> int:
    """Even-space entities of one type along the x axis in a room.

    Selects every entity in `target_room`'s Ambition layer whose
    `__identifier == identifier`. If `y_row` is given, restricts the
    selection to entities whose px[1] sits within `y_tolerance` of
    that row (so a hub with multiple door rows can be spaced one row
    at a time).

    Strategy:
    - ``preserve-ends`` (default): keeps the leftmost + rightmost
      entity in place and even-spaces every entity between them.
    - ``fit``: distributes every entity evenly across
      ``[start_x, end_x]`` (defaults: 0..pxWid). Outermost entities
      get gaps on either side equal to the inner spacing.

    Returns 0 on success, prints a one-line plan per entity.
    Mutates `project` in place when `dry_run=False`.
    """
    target_level = find_level(project, target_room)
    if target_level is None:
        known = ", ".join(sorted(l["identifier"] for l in project.get("levels", [])))
        print(f"error: target_room '{target_room}' not found. Known levels: {known}")
        return 2
    ambition = find_layer_in_level(target_level, "Ambition")
    if ambition is None:
        print(f"error: '{target_room}' has no Ambition entity layer")
        return 2

    # Collect matching entities with original positions and width.
    matched: list[dict] = []
    for ent in ambition.get("entityInstances", []):
        if ent.get("__identifier") != identifier:
            continue
        if y_row is not None and abs(int(ent["px"][1]) - y_row) > y_tolerance:
            continue
        matched.append(ent)
    if len(matched) < 2:
        print(
            f"error: need at least 2 '{identifier}' entities to even-space; "
            f"found {len(matched)} in '{target_room}'"
            + (f" near y={y_row}" if y_row is not None else "")
        )
        return 2

    matched.sort(key=lambda e: int(e["px"][0]))
    widths = [int(e["width"]) for e in matched]

    # Decide the x range to distribute across.
    level_w = int(target_level["pxWid"])
    if strategy == "preserve-ends":
        first_x = int(matched[0]["px"][0])
        last_x = int(matched[-1]["px"][0])
        last_w = widths[-1]
        # Keep first / last in place; even-space inner entities.
        # Compute the total inner width consumed by entities (excluding
        # the first, which is fixed) and divide remaining gap budget.
        inner_w_sum = sum(widths[1:])
        span = (last_x + last_w) - first_x
        gap_budget = span - sum(widths)
        if gap_budget < 0:
            print(
                f"error: entities don't fit in span {span} (sum widths={sum(widths)})"
            )
            return 2
        n_gaps = len(matched) - 1
        gap = gap_budget / n_gaps
        new_x = []
        cursor = first_x
        for i, w in enumerate(widths):
            if i == 0:
                new_x.append(first_x)
                cursor = first_x + w + gap
            elif i == len(matched) - 1:
                new_x.append(last_x)
            else:
                new_x.append(int(round(cursor)))
                cursor += w + gap
    elif strategy == "fit":
        s = 0 if start_x is None else int(start_x)
        e = level_w if end_x is None else int(end_x)
        span = e - s
        gap_budget = span - sum(widths)
        if gap_budget < 0:
            print(
                f"error: entities don't fit in span {span} (sum widths={sum(widths)})"
            )
            return 2
        n_gaps = len(matched) + 1  # leading + between + trailing gap
        gap = gap_budget / n_gaps
        new_x = []
        cursor = s + gap
        for w in widths:
            new_x.append(int(round(cursor)))
            cursor += w + gap
    else:
        print(f"error: unknown strategy {strategy!r}; use 'preserve-ends' or 'fit'")
        return 2

    # Plan + apply.
    # By default we keep exact pixel positions so gaps come out
    # uniform — grid-snapping a fractional even-spacing produces
    # the alternating 80/80/64/80 pattern the basement bottom row
    # had after a previous `entity even-space` run with snap enabled.
    # Authors who specifically need 16px-aligned door tops opt in
    # via `--snap-to-grid`.
    grid_size = int(project.get("defaultGridSize", 16))
    print(f"# even-space {len(matched)} '{identifier}' in '{target_room}' "
          f"(strategy={strategy}, snap_to_grid={snap_to_grid})")
    changed = 0
    for ent, target in zip(matched, new_x):
        old = int(ent["px"][0])
        if snap_to_grid:
            new_pos = int(round(target / grid_size) * grid_size)
        else:
            new_pos = int(round(target))
        ident = "?"
        for fi in ent.get("fieldInstances", []):
            if fi["__identifier"] == "id":
                ident = str(fi.get("__value") or "?")
                break
        if new_pos != old:
            changed += 1
            print(f"  {ident:30s}  x: {old:>5} -> {new_pos:>5}  (delta {new_pos - old:+d})")
        else:
            print(f"  {ident:30s}  x: {old:>5}  (unchanged)")
        if not dry_run:
            ent["px"][0] = new_pos
            # Update the world-coord mirror Bevy renderers read from.
            level_world_x = int(target_level.get("worldX", 0))
            ent["__worldX"] = new_pos + level_world_x
    print(f"# {changed} entit{'y' if changed == 1 else 'ies'} repositioned")
    return 0


def list_free_spots(project: dict, target_room: str) -> int:
    """Print free 48x96 door slots in `target_room`'s LoadingZone row.

    Heuristic: collect every existing LoadingZone in the target's
    Ambition layer, group by y (the door-row coordinate), and report
    the largest gaps along x where a new 48x96 door could fit without
    overlapping. Authors copy the suggested px into their spec's
    `connect_to.px` block.

    The y-row is auto-detected as the y where most existing doors sit
    (typical basement layout). If the target has only entry/exit
    doors at varying y, the function reports each unique y row's
    gaps separately.
    """
    target_level = find_level(project, target_room)
    if target_level is None:
        known = ", ".join(sorted(l["identifier"] for l in project.get("levels", [])))
        print(f"error: target_room '{target_room}' not found. Known levels: {known}")
        return 2
    ambition = find_layer_in_level(target_level, "Ambition")
    if ambition is None:
        print(f"error: '{target_room}' has no Ambition entity layer")
        return 2

    door_w_default = 48
    door_h_default = 96

    # Collect LoadingZone entities and their AABBs.
    doors: list[tuple[int, int, int, int, str]] = []  # (x, y, w, h, id)
    for ent in ambition.get("entityInstances", []):
        if ent.get("__identifier") != "LoadingZone":
            continue
        x, y = ent["px"]
        w, h = ent["width"], ent["height"]
        ident = "?"
        for fi in ent.get("fieldInstances", []):
            if fi["__identifier"] == "id":
                ident = str(fi.get("__value") or "?")
                break
        doors.append((int(x), int(y), int(w), int(h), ident))

    if not doors:
        print(f"'{target_room}' has no existing LoadingZones; pick any spot.")
        print(f"level size: {target_level['pxWid']}x{target_level['pxHei']}")
        return 0

    # Group doors by y row. A "row" is a cluster of doors with the
    # same y (within a small fudge factor for off-by-pixel authors).
    rows: dict[int, list[tuple[int, int, int, int, str]]] = {}
    for door in doors:
        x, y, w, h, ident = door
        # Snap y to the nearest row in `rows` if within 32 px,
        # otherwise start a new row.
        snap = None
        for ry in rows:
            if abs(ry - y) <= 32:
                snap = ry
                break
        if snap is None:
            rows[y] = [door]
        else:
            rows[snap].append(door)

    level_w = int(target_level["pxWid"])
    print(f"# Free door spots in '{target_room}' ({level_w}px wide):")
    for ry in sorted(rows):
        row_doors = sorted(rows[ry], key=lambda d: d[0])
        print(f"## row at y={ry} ({len(row_doors)} existing doors)")
        # Walk the row finding gaps wider than door_w_default.
        cursor = 0
        free_gaps: list[tuple[int, int]] = []
        for (x, _y, w, _h, ident) in row_doors:
            if x - cursor >= door_w_default:
                free_gaps.append((cursor, x))
            cursor = x + w
        # Trailing gap to right edge.
        if level_w - cursor >= door_w_default:
            free_gaps.append((cursor, level_w))
        if not free_gaps:
            print(f"  (no free 48x96 gap in this row)")
            continue
        for (gap_start, gap_end) in free_gaps:
            mid = (gap_start + gap_end - door_w_default) // 2
            mid = max(gap_start, min(mid, gap_end - door_w_default))
            print(
                f"  gap x={gap_start:>5}..{gap_end:>5} ({gap_end - gap_start:>4}px wide) "
                f"-> suggested px=[{mid}, {ry}] size=[{door_w_default}, {door_h_default}]"
            )

    return 0


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "spec",
        type=Path,
        nargs="?",
        help="Path to a YAML or JSON spec describing the new area / level "
        "(optional when --list-free-spots is used)",
    )
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=Path(
            "crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk"
        ),
        help="Target LDtk file to extend (default: sandbox.ldtk)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Write the updated LDtk to this path instead of editing in place",
    )
    parser.add_argument(
        "--backup",
        action="store_true",
        help="Write <ldtk>.bak before modifying in place",
    )
    parser.add_argument(
        "--no-repair",
        action="store_true",
        help="Skip the repair + validate post-pass (debug only)",
    )
    parser.add_argument(
        "--schema",
        type=Path,
        default=Path("tools/ambition_ldtk_tools/schemas/ldtk/JSON_SCHEMA.json"),
        help="Optional official LDtk JSON schema for the post-validate pass",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help=(
            "Build the level entirely in memory and print a structured "
            "preview summary; do NOT write the file or run repair/validate"
        ),
    )
    parser.add_argument(
        "--replace-existing",
        action="store_true",
        help=(
            "Replace an existing level with the same identifier instead of "
            "failing. Intended for regenerating a spec-owned area."
        ),
    )
    parser.add_argument(
        "--list-free-spots",
        type=str,
        default=None,
        metavar="TARGET_ROOM",
        help=(
            "Don't build a new level. Instead scan the named target room's "
            "Ambition entity layer for free 48x96 gaps along its existing "
            "LoadingZone door row and print suggestions for `connect_to.px`. "
            "Use this when authoring a new room that needs a basement-style "
            "door so you don't have to find an empty corridor slot by hand."
        ),
    )
    parser.add_argument(
        "--snap-to-surface",
        type=str,
        default=None,
        metavar="TARGET_ROOM",
        help=(
            "Don't build a new level. Instead, given a target room and an x "
            "coordinate (via --x), print the door y that lands the door's "
            "bottom flush on the nearest Collision surface. Use when "
            "`door free-spots` returned an x in a row that turns out to be "
            "mid-air for that column."
        ),
    )
    parser.add_argument(
        "--x",
        type=int,
        default=None,
        help="x in pixels for --snap-to-surface",
    )
    parser.add_argument(
        "--door-w",
        type=int,
        default=48,
        help="door width in pixels (default 48) for --snap-to-surface",
    )
    parser.add_argument(
        "--door-h",
        type=int,
        default=96,
        help="door height in pixels (default 96) for --snap-to-surface",
    )
    parser.add_argument(
        "--prefer-y",
        type=int,
        default=None,
        help="optional preferred door y for --snap-to-surface tie-breaking",
    )
    parser.add_argument(
        "--even-space-entities",
        type=str,
        default=None,
        metavar="TARGET_ROOM",
        help=(
            "Don't build a new level. Even-space every entity of "
            "--entity-type in this room along the x axis. Use --y-row "
            "to restrict to one door row. Strategy via --strategy."
        ),
    )
    parser.add_argument(
        "--entity-type",
        type=str,
        default="LoadingZone",
        help="entity __identifier to even-space (default LoadingZone)",
    )
    parser.add_argument(
        "--y-row",
        type=int,
        default=None,
        help="restrict --even-space to entities near this y (within --y-tolerance)",
    )
    parser.add_argument(
        "--y-tolerance",
        type=int,
        default=32,
        help="vertical band width for --y-row (default 32px)",
    )
    parser.add_argument(
        "--start-x",
        type=int,
        default=None,
        help="start x of distribution span for --strategy fit (default 0)",
    )
    parser.add_argument(
        "--end-x",
        type=int,
        default=None,
        help="end x of distribution span for --strategy fit (default level width)",
    )
    parser.add_argument(
        "--strategy",
        type=str,
        default="preserve-ends",
        choices=["preserve-ends", "fit"],
        help="how to distribute entities along x (default preserve-ends)",
    )
    parser.add_argument(
        "--snap-to-grid",
        action="store_true",
        help=(
            "round each repositioned x to defaultGridSize (default 16). "
            "Off by default — produces uniform gaps but non-grid-aligned "
            "door tops. Turn on if you need editor-grid alignment."
        ),
    )
    args = parser.parse_args(argv)

    # `--list-free-spots` short-circuits before we touch the spec; the
    # `spec` positional is still required by argparse but its content
    # is not used in this path. Authors typically run:
    #   PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools door free-spots central_hub_basement
    # to see free door slots, then edit their actual spec.
    if args.list_free_spots:
        project = load_project(args.ldtk)
        return list_free_spots(project, args.list_free_spots)

    if args.snap_to_surface:
        if args.x is None:
            return _fail("--snap-to-surface requires --x <px>")
        project = load_project(args.ldtk)
        try:
            x, y, kind = snap_door_to_surface(
                project,
                args.snap_to_surface,
                args.x,
                door_w=args.door_w,
                door_h=args.door_h,
                prefer_y=args.prefer_y,
            )
        except SystemExit as ex:
            print(f"error: {ex}", file=sys.stderr)
            return 2
        print(f"snap x={x} y={y} surface={kind}  -> px=[{x}, {y}] size=[{args.door_w}, {args.door_h}]")
        return 0

    if args.even_space_entities:
        project = load_project(args.ldtk)
        rc = even_space_entities(
            project,
            args.even_space_entities,
            args.entity_type,
            y_row=args.y_row,
            y_tolerance=args.y_tolerance,
            start_x=args.start_x,
            end_x=args.end_x,
            strategy=args.strategy,
            snap_to_grid=args.snap_to_grid,
            dry_run=args.dry_run,
        )
        if rc != 0:
            return rc
        if args.dry_run:
            return 0
        # Write + repair + validate, mirroring `area create`.
        write_project(args.ldtk, project)
        if not args.no_repair:
            from . import repair, validate as ldtk_validate
            repair_rc = repair.main([str(args.ldtk), "--in-place"])
            if repair_rc != 0:
                return repair_rc
            return ldtk_validate.main([str(args.ldtk), "--schema", str(args.schema), "--require-schema"])
        return 0

    if args.spec is None:
        return _fail("missing required positional 'spec'")

    spec = load_spec(args.spec)
    if not isinstance(spec, dict):
        return _fail(f"spec must be a mapping, got {type(spec).__name__}")
    for required in ("id", "level_id", "world_x", "world_y", "px_wid", "px_hei"):
        if required not in spec:
            return _fail(f"spec is missing required key '{required}'")

    project = load_project(args.ldtk)

    existing_index = next(
        (
            idx
            for idx, level in enumerate(project.get("levels", []))
            if level.get("identifier") == spec["level_id"]
        ),
        None,
    )
    if existing_index is not None:
        if not args.replace_existing:
            return _fail(f"level identifier '{spec['level_id']}' already exists")
        removed = project["levels"].pop(existing_index)
        print(
            f"replacing existing level '{removed['identifier']}' "
            f"({removed['pxWid']}x{removed['pxHei']} at "
            f"{removed['worldX']},{removed['worldY']})"
        )

    level = build_level(project, spec)
    project.setdefault("levels", []).append(level)
    # `toc` is required by the LDtk JSON schema; LDtk regenerates per-level
    # TOC entries on save, so leaving the existing top-level TOC list intact
    # is the safe choice. (Adding an empty entry for the new level is also
    # acceptable; LDtk rebuilds either way.)
    project.setdefault("toc", [])

    # Optional: append reciprocal LoadingZones into existing target levels.
    reciprocal_summaries: list[str] = []
    for connection in spec.get("connect_to") or []:
        instance = add_reciprocal_loading_zone(project, connection, spec["level_id"])
        fields = {
            f["__identifier"]: f.get("__value")
            for f in instance.get("fieldInstances", [])
        }
        reciprocal_summaries.append(
            f"reciprocal LoadingZone in '{connection['target_room']}' at "
            f"px={instance['px']} size=({instance['width']}x{instance['height']}) "
            f"→ {fields.get('target_room')}/{fields.get('target_zone')}"
        )

    print("--- preview ---")
    print(summarize_level(level))
    for line in reciprocal_summaries:
        print(line)
    print("--- end preview ---")

    if args.dry_run:
        print("dry-run: no file written; repair/validate skipped")
        return 0

    target = args.output or args.ldtk
    if args.output is None and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")

    write_project(target, project)
    print(
        f"wrote new level '{level['identifier']}' (area '{spec['id']}', "
        f"{level['pxWid']}x{level['pxHei']} at {level['worldX']},{level['worldY']}) "
        f"to {target}"
    )
    if args.no_repair:
        return 0

    schema = args.schema if args.schema and args.schema.exists() else None
    return run_repair_and_validate(target, schema)


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
