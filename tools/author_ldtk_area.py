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
to the existing `repair_ambition_ldtk.py` pass. The resulting file is
guaranteed to validate cleanly with `validate_ambition_ldtk.py` (both
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

TOOLS_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS_DIR))

REPAIR_SCRIPT = TOOLS_DIR / "repair_ambition_ldtk.py"
VALIDATE_SCRIPT = TOOLS_DIR / "validate_ambition_ldtk.py"


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
    path.write_text(json.dumps(project, indent=2) + "\n")


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
    """Build a minimal field instance. `repair_ambition_ldtk.py` will fill
    `realEditorValues` from `__value` for common types."""
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


def build_entity_instance(project: dict, ent_spec: dict, grid_size: int) -> dict:
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
        "iid": iid,
        "width": width,
        "height": height,
        "defUid": ent_def["uid"],
        "px": [int(px[0]), int(px[1])],
        "fieldInstances": [],
    }

    # Build field instances. The spec can omit fields — `repair_ambition_ldtk.py`
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
            entity_instances.append(build_entity_instance(project, ent_spec, grid_size))
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

    # Reject overlap with any existing entity rect in the target level.
    new_x, new_y = int(px[0]), int(px[1])
    new_w, new_h = int(size[0]), int(size[1])
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
    instance = build_entity_instance(project, ent_spec, grid_size)
    ambition.setdefault("entityInstances", []).append(instance)
    return instance


def run_repair_and_validate(project_path: Path, schema: Path | None) -> int:
    """Run the existing repair + validator scripts and forward their exit code."""
    cmd_repair = [sys.executable, str(REPAIR_SCRIPT), str(project_path), "--in-place"]
    print(f"$ {' '.join(cmd_repair)}")
    r = subprocess.run(cmd_repair)
    if r.returncode != 0:
        return r.returncode
    cmd_val = [sys.executable, str(VALIDATE_SCRIPT), str(project_path)]
    if schema is not None:
        cmd_val.extend(["--schema", str(schema), "--require-schema"])
    print(f"$ {' '.join(cmd_val)}")
    return subprocess.run(cmd_val).returncode


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "spec",
        type=Path,
        help="Path to a YAML or JSON spec describing the new area / level",
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
        default=Path("tools/schemas/ldtk/JSON_SCHEMA.json"),
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
    args = parser.parse_args(argv)

    spec = load_spec(args.spec)
    if not isinstance(spec, dict):
        return _fail(f"spec must be a mapping, got {type(spec).__name__}")
    for required in ("id", "level_id", "world_x", "world_y", "px_wid", "px_hei"):
        if required not in spec:
            return _fail(f"spec is missing required key '{required}'")

    project = load_project(args.ldtk)

    # Check for an existing level with the same identifier so we don't clone.
    if any(l.get("identifier") == spec["level_id"] for l in project.get("levels", [])):
        return _fail(f"level identifier '{spec['level_id']}' already exists")

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
