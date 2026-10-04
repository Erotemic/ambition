"""Canonical, id-free view of an LDtk project.

The semantic diff (``semantic_diff.py``) and the ``diff normalize`` command use
this module. It reads a project and keeps only the authored meaning:

- Objects have names, not numbers. A definition is keyed by its identifier. A
  uid reference becomes ``kind:identifier``. A level is keyed by its identifier.
- Order that has no meaning is removed. Entities, definition lists and tiles at
  different positions are sorted. Layer definition order stays, because it is
  draw order. Tiles at one position keep their stack order.
- Each raw key belongs to a known class. A key is semantic, or it is noise of a
  named category (uid, iid, derived ``__`` cache, editor metadata, editor
  display). A key that this module does not know is semantic. Thus a new LDtk
  key can make a false alarm, but it cannot hide a change.

This module never writes an LDtk file.
"""

from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable

CANONICAL_SCHEMA = "ambition-ldtk-canonical/1"

# Noise categories. The diff report counts each one.
UID = "uid_renumbered"
IID = "iid_renumbered"
ORDER = "reordered"
DERIVED = "derived_cache"
EDITOR = "editor_metadata"
DISPLAY = "editor_display"
NULL_FIELD = "null_field_omitted"
TILE_CACHE = "tile_rule_cache"

NOISE_DESCRIPTIONS = {
    UID: "uid numbers changed (uid, *Uid, defUid, levelId, nextUid); names still match",
    IID: "iid strings changed; objects matched by content, not by iid",
    ORDER: "list order changed where order has no meaning",
    DERIVED: "derived '__' cache values changed (recomputed from other data)",
    EDITOR: "editor or export settings changed (appBuildId, toc, seed, realEditorValues, ...)",
    DISPLAY: "editor-only display changed (colors, docs, opacity, editor* field options)",
    NULL_FIELD: "a field instance with a null value was written or omitted",
    TILE_CACHE: "tile 't'/'d' values changed (tile id and auto-rule uid cache)",
}

# ---------------------------------------------------------------------------
# Key classification tables. A key not in a table is semantic.
# ---------------------------------------------------------------------------

PROJECT_SEMANTIC = {"worldLayout", "worldGridWidth", "worldGridHeight", "externalLevels", "jsonVersion"}
PROJECT_HANDLED = {"defs", "levels", "worlds"}
PROJECT_NOISE = {
    "iid": IID,
    "dummyWorldIid": IID,
    "nextUid": UID,
    "toc": DERIVED,
    "__header__": EDITOR,
    "appBuildId": EDITOR,
    "identifierStyle": EDITOR,
    "defaultLevelWidth": EDITOR,
    "defaultLevelHeight": EDITOR,
    "defaultPivotX": EDITOR,
    "defaultPivotY": EDITOR,
    "defaultGridSize": EDITOR,
    "defaultEntityWidth": EDITOR,
    "defaultEntityHeight": EDITOR,
    "bgColor": EDITOR,
    "defaultLevelBgColor": EDITOR,
    "minifyJson": EDITOR,
    "exportTiled": EDITOR,
    "simplifiedExport": EDITOR,
    "imageExportMode": EDITOR,
    "exportLevelBg": EDITOR,
    "pngFilePattern": EDITOR,
    "backupOnSave": EDITOR,
    "backupLimit": EDITOR,
    "backupRelPath": EDITOR,
    "levelNamePattern": EDITOR,
    "tutorialDesc": EDITOR,
    "customCommands": EDITOR,
    "flags": EDITOR,
}

WORLD_SEMANTIC = {"worldLayout", "worldGridWidth", "worldGridHeight"}
WORLD_HANDLED = {"identifier", "levels"}
WORLD_NOISE = {"iid": IID, "defaultLevelWidth": EDITOR, "defaultLevelHeight": EDITOR}

LEVEL_PROPS = (
    "worldX", "worldY", "worldDepth", "pxWid", "pxHei",
    "bgColor", "bgRelPath", "bgPos", "bgPivotX", "bgPivotY", "externalRelPath",
)
LEVEL_HANDLED = {"identifier", "fieldInstances", "layerInstances", *LEVEL_PROPS}
LEVEL_NOISE = {
    "iid": IID,
    "uid": UID,
    "__bgColor": DERIVED,
    "__smartColor": DERIVED,
    "__bgPos": DERIVED,
    "__neighbours": DERIVED,
    "useAutoIdentifier": EDITOR,
}

LAYER_PROPS = ("visible", "pxOffsetX", "pxOffsetY", "overrideTilesetUid", "optionalRules")
LAYER_HANDLED = {
    "__identifier", "__type", "intGridCsv", "gridTiles", "autoLayerTiles", "entityInstances", *LAYER_PROPS,
}
LAYER_NOISE = {
    "iid": IID,
    "layerDefUid": UID,
    "levelId": UID,
    "seed": EDITOR,
    "__cWid": DERIVED,
    "__cHei": DERIVED,
    "__gridSize": DERIVED,
    "__opacity": DERIVED,
    "__pxTotalOffsetX": DERIVED,
    "__pxTotalOffsetY": DERIVED,
    "__tilesetDefUid": DERIVED,
    "__tilesetRelPath": DERIVED,
}

ENTITY_HANDLED = {"__identifier", "px", "width", "height", "fieldInstances", "__pivot", "iid"}
ENTITY_NOISE = {
    "defUid": UID,
    "__grid": DERIVED,
    "__tags": DERIVED,
    "__tile": DERIVED,
    "__smartColor": DERIVED,
    "__worldX": DERIVED,
    "__worldY": DERIVED,
}

FIELD_HANDLED = {"__identifier", "__value", "__type"}
FIELD_NOISE = {"defUid": UID, "__tile": DERIVED, "realEditorValues": EDITOR}

TILE_HANDLED = {"px", "src", "f", "a"}
TILE_NOISE = {"t": TILE_CACHE, "d": TILE_CACHE}

# Definitions: keys removed before comparison, by category.
DEF_NOISE = {
    "uid": UID,
    "invalidated": EDITOR,
    "cachedPixelData": DERIVED,
    "savedSelections": EDITOR,
    "externalFileChecksum": DERIVED,
    "__cWid": DERIVED,
    "__cHei": DERIVED,
    "__type": DERIVED,
    "doc": DISPLAY,
    "uiColor": DISPLAY,
    "color": DISPLAY,
    "icon": DISPLAY,
    "displayOpacity": DISPLAY,
    "inactiveOpacity": DISPLAY,
    "hideInList": DISPLAY,
    "hideFieldsWhenInactive": DISPLAY,
    "canSelectWhenInactive": DISPLAY,
    "renderInWorldView": DISPLAY,
    "guideGridWid": DISPLAY,
    "guideGridHei": DISPLAY,
    "uiFilterTags": DISPLAY,
    "useAsyncRender": DISPLAY,
    "showName": DISPLAY,
    "fillOpacity": DISPLAY,
    "lineOpacity": DISPLAY,
    "tileOpacity": DISPLAY,
    "hollow": DISPLAY,
    "usesWizard": DISPLAY,
    "exportToToc": DISPLAY,
    "searchable": DISPLAY,
    "useForSmartColor": DISPLAY,
}

# Entity definition keys that choose the editor icon. The diff reports them as
# ``entity_def_visual``.
ENTITY_DEF_VISUAL = ("renderMode", "tileRenderMode", "tilesetId", "tileRect", "uiTileRect", "nineSliceBorders", "tileId")

# Keys that hold a uid reference, in addition to keys that end in Uid/Uids.
UID_REF_KEYS = {"tilesetId", "defUid"}


def _is_uid_ref(key: str) -> bool:
    return key in UID_REF_KEYS or key.endswith("Uid") or key.endswith("Uids")


def _def_noise_category(key: str) -> str | None:
    if key in DEF_NOISE:
        return DEF_NOISE[key]
    if key.startswith("editor"):
        return DISPLAY
    return None


# ---------------------------------------------------------------------------
# Input
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Source:
    label: str
    text: str


def read_source(spec: str) -> Source:
    """Read a file path, or ``REV:PATH`` from git.

    PATH is relative to the current directory. The module finds the git
    repository that holds PATH, so a path inside a submodule reads from the
    submodule at REV.
    """
    path = Path(spec)
    if path.exists() or ":" not in spec:
        return Source(spec, path.read_text())
    rev, _, rel = spec.partition(":")
    target = Path(rel).resolve()
    probe = target.parent
    while not probe.exists():
        probe = probe.parent
    top = subprocess.run(
        ["git", "-C", str(probe), "rev-parse", "--show-toplevel"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    inner = target.relative_to(Path(top).resolve())
    shown = subprocess.run(
        ["git", "-C", top, "show", f"{rev}:{inner.as_posix()}"],
        check=True, capture_output=True, text=True,
    )
    return Source(spec, shown.stdout)


def load_source(spec: str) -> tuple[Source, dict]:
    src = read_source(spec)
    return src, json.loads(src.text)


# ---------------------------------------------------------------------------
# Uid resolution
# ---------------------------------------------------------------------------


def uid_names(project: dict) -> dict[int, str]:
    """Map every definition uid to a stable ``kind:identifier`` name."""
    defs = project.get("defs") or {}
    names: dict[int, str] = {}

    def put(uid: Any, name: str) -> None:
        if isinstance(uid, int):
            names[uid] = name

    for layer in defs.get("layers") or []:
        lid = layer.get("identifier")
        put(layer.get("uid"), f"layer:{lid}")
        for gi, group in enumerate(layer.get("autoRuleGroups") or []):
            gname = f"{lid}/{group.get('name')}#{gi}"
            put(group.get("uid"), f"ruleGroup:{gname}")
            for ri, rule in enumerate(group.get("rules") or []):
                put(rule.get("uid"), f"rule:{gname}/{ri}")
        for value in layer.get("intGridValuesGroups") or []:
            put(value.get("uid"), f"intGridGroup:{lid}/{value.get('identifier')}")
    for ent in defs.get("entities") or []:
        eid = ent.get("identifier")
        put(ent.get("uid"), f"entity:{eid}")
        for fd in ent.get("fieldDefs") or []:
            put(fd.get("uid"), f"field:{eid}.{fd.get('identifier')}")
    for ts in defs.get("tilesets") or []:
        put(ts.get("uid"), f"tileset:{ts.get('identifier')}")
    for enum in defs.get("enums") or []:
        put(enum.get("uid"), f"enum:{enum.get('identifier')}")
    for enum in defs.get("externalEnums") or []:
        put(enum.get("uid"), f"enum:{enum.get('identifier')}")
    for fd in defs.get("levelFields") or []:
        put(fd.get("uid"), f"levelField:{fd.get('identifier')}")
    for _, level, _ in iter_levels(project):
        put(level.get("uid"), f"level:{level.get('identifier')}")
    return names


def resolve_uid(value: Any, names: dict[int, str]) -> Any:
    if isinstance(value, bool) or value is None:
        return value
    if isinstance(value, int):
        return names.get(value, f"uid:{value}")
    if isinstance(value, list):
        return [resolve_uid(v, names) for v in value]
    return value


# ---------------------------------------------------------------------------
# Definitions
# ---------------------------------------------------------------------------


def normalize_def(obj: Any, names: dict[int, str], path: str, noise: dict[str, dict[str, Any]]) -> Any:
    """Strip noise keys, resolve uid references, record what was stripped.

    ``noise`` collects ``category -> {path: raw value}``. The diff compares the
    noise of two sides to count noise; it never reads it as a change.
    """
    if isinstance(obj, dict):
        out: dict[str, Any] = {}
        for key in sorted(obj):
            value = obj[key]
            sub = f"{path}.{key}"
            category = _def_noise_category(key)
            if category is not None:
                noise.setdefault(category, {})[sub] = value
                if category == UID or not _is_uid_ref(key):
                    continue
            if _is_uid_ref(key):
                noise.setdefault(UID, {})[sub] = value
                out[key] = resolve_uid(value, names)
                continue
            out[key] = normalize_def(value, names, sub, noise)
        return out
    if isinstance(obj, list):
        return [normalize_def(v, names, f"{path}[{i}]", noise) for i, v in enumerate(obj)]
    return obj


def _keyed(rows: Iterable[dict], key: str = "identifier") -> tuple[dict[str, dict], list[str]]:
    """Index rows by identifier. Return the index and any duplicate names."""
    out: dict[str, dict] = {}
    dupes: list[str] = []
    for row in rows or []:
        name = str(row.get(key))
        if name in out:
            dupes.append(name)
        out[name] = row
    return out, sorted(set(dupes))


@dataclass
class DefFamily:
    """One family of definitions (layers, entities, ...) keyed by identifier."""

    items: dict[str, Any] = field(default_factory=dict)
    noise: dict[str, dict[str, dict[str, Any]]] = field(default_factory=dict)
    order: list[str] = field(default_factory=list)
    duplicates: list[str] = field(default_factory=list)


def _layer_def(raw: dict, names: dict[int, str], noise: dict) -> dict:
    row = dict(raw)
    values = row.pop("intGridValues", None) or []
    out = normalize_def(row, names, raw.get("identifier", "?"), noise)
    by_value: dict[str, Any] = {}
    for i, value in enumerate(values):
        by_value[str(value.get("value"))] = normalize_def(value, names, f"{raw.get('identifier')}.intGridValues[{i}]", noise)
    out["intGridValues"] = by_value
    return out


def _entity_def(raw: dict, names: dict[int, str], noise: dict) -> dict:
    row = dict(raw)
    field_defs = row.pop("fieldDefs", None) or []
    out = normalize_def(row, names, raw.get("identifier", "?"), noise)
    fields: dict[str, Any] = {}
    for fd in field_defs:
        fields[str(fd.get("identifier"))] = normalize_def(fd, names, f"{raw.get('identifier')}.{fd.get('identifier')}", noise)
    out["fieldDefs"] = fields
    return out


def def_families(project: dict, names: dict[int, str]) -> dict[str, DefFamily]:
    defs = project.get("defs") or {}
    builders = {
        "layers": _layer_def,
        "entities": _entity_def,
        "tilesets": lambda r, n, z: normalize_def(r, n, r.get("identifier", "?"), z),
        "enums": lambda r, n, z: normalize_def(r, n, r.get("identifier", "?"), z),
        "externalEnums": lambda r, n, z: normalize_def(r, n, r.get("identifier", "?"), z),
        "levelFields": lambda r, n, z: normalize_def(r, n, r.get("identifier", "?"), z),
    }
    families: dict[str, DefFamily] = {}
    for name, build in builders.items():
        rows = defs.get(name) or []
        fam = DefFamily()
        keyed, fam.duplicates = _keyed(rows)
        fam.order = [str(r.get("identifier")) for r in rows]
        for ident, raw in keyed.items():
            noise: dict[str, dict[str, Any]] = {}
            fam.items[ident] = build(raw, names, noise)
            fam.noise[ident] = noise
        families[name] = fam
    return families


# ---------------------------------------------------------------------------
# Levels, layers, entities, fields
# ---------------------------------------------------------------------------


def iter_levels(project: dict) -> list[tuple[str, dict, str | None]]:
    """Return ``(key, level, world)``. The key is ``World/Level`` in a multi-world project."""
    out: list[tuple[str, dict, str | None]] = []
    for level in project.get("levels") or []:
        out.append((str(level.get("identifier")), level, None))
    for world in project.get("worlds") or []:
        wid = str(world.get("identifier"))
        for level in world.get("levels") or []:
            out.append((f"{wid}/{level.get('identifier')}", level, wid))
    return out


class _Absent:
    """A field instance that the file does not contain."""

    def __repr__(self) -> str:
        return "<absent>"


ABSENT = _Absent()


@dataclass(frozen=True)
class Ref:
    """An EntityRef value. ``iid`` is the target; the diff resolves it by match."""

    iid: str | None
    raw: Any


def field_default(fdef: dict | None) -> Any:
    """The definition default of a field, as a plain value (for messages only)."""
    if not fdef:
        return ABSENT
    override = fdef.get("defaultOverride")
    if isinstance(override, dict) and override.get("params"):
        return override["params"][0]
    if fdef.get("isArray"):
        return []
    return None


def normalize_value(value: Any, ftype: str | None) -> Any:
    ftype = ftype or ""
    if "EntityRef" in ftype:
        if isinstance(value, list):
            return [Ref(v.get("entityIid") if isinstance(v, dict) else None, v) for v in value]
        if isinstance(value, dict):
            return Ref(value.get("entityIid"), value)
        return value
    if "Float" in ftype:
        if isinstance(value, list):
            return [float(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else v for v in value]
        if isinstance(value, int) and not isinstance(value, bool):
            return float(value)
    return value


@dataclass
class FieldSet:
    values: dict[str, Any]
    types: dict[str, str]
    raw: dict[str, dict]
    order: list[str]
    duplicates: list[str]
    every: dict[str, list[Any]] = field(default_factory=dict)


def read_fields(instances: list[dict] | None) -> FieldSet:
    values: dict[str, Any] = {}
    types: dict[str, str] = {}
    raw: dict[str, dict] = {}
    order: list[str] = []
    dupes: list[str] = []
    every: dict[str, list[Any]] = {}
    for inst in instances or []:
        name = str(inst.get("__identifier"))
        value = normalize_value(inst.get("__value"), inst.get("__type"))
        every.setdefault(name, []).append(value)
        if name in values:
            dupes.append(name)
            continue
        types[name] = str(inst.get("__type"))
        values[name] = value
        raw[name] = inst
        order.append(name)
    return FieldSet(values, types, raw, order, sorted(set(dupes)), every)


@dataclass(eq=False)
class Ent:
    level: str
    layer: str
    index: int
    raw: dict
    fields: FieldSet

    @property
    def identifier(self) -> str:
        return str(self.raw.get("__identifier"))

    @property
    def iid(self) -> str | None:
        return self.raw.get("iid")

    @property
    def px(self) -> list:
        return list(self.raw.get("px") or [0, 0])

    @property
    def size(self) -> list:
        return [self.raw.get("width"), self.raw.get("height")]

    @property
    def pivot(self) -> list | None:
        pivot = self.raw.get("__pivot")
        return [float(v) for v in pivot] if isinstance(pivot, list) else pivot

    @property
    def authored_id(self) -> str | None:
        return self.authored_key("id")

    def authored_key(self, field_name: str) -> str | None:
        value = self.fields.values.get(field_name)
        if isinstance(value, str) and value.strip():
            return value.strip()
        return None

    def label(self) -> str:
        """The type and the first authored name, for messages."""
        for name in ("id", "character_id", "name"):
            value = self.authored_key(name)
            if value:
                return f"{self.identifier} '{value}'"
        return self.identifier

    def address(self) -> str:
        px = self.px
        return f"{self.level}/{self.layer}/{self.identifier}@{px[0]},{px[1]}"


@dataclass(eq=False)
class Layer:
    identifier: str
    raw: dict
    entities: list[Ent]


@dataclass(eq=False)
class Lvl:
    key: str
    world: str | None
    raw: dict
    fields: FieldSet
    layers: dict[str, Layer]
    layer_order: list[str]
    layer_duplicates: list[str]

    def entities(self) -> list[Ent]:
        return [e for layer in self.layers.values() for e in layer.entities]


@dataclass
class Model:
    """A parsed project with the indexes the diff and the normalizer need."""

    raw: dict
    names: dict[int, str]
    defs: dict[str, DefFamily]
    levels: dict[str, Lvl]
    level_order: list[str]
    level_duplicates: list[str]
    by_iid: dict[str, Ent]
    address_counts: dict[str, int]

    def entity_def_field(self, entity: str, field_name: str) -> dict | None:
        for raw in (self.raw.get("defs") or {}).get("entities") or []:
            if raw.get("identifier") == entity:
                for fd in raw.get("fieldDefs") or []:
                    if fd.get("identifier") == field_name:
                        return fd
        return None

    def level_field_def(self, field_name: str) -> dict | None:
        for fd in (self.raw.get("defs") or {}).get("levelFields") or []:
            if fd.get("identifier") == field_name:
                return fd
        return None

    def ref_address(self, ref: Ref) -> str:
        target = self.by_iid.get(ref.iid) if ref.iid else None
        if target is None:
            return f"<dangling {ref.iid}>"
        address = target.address()
        if self.address_counts.get(address, 0) > 1:
            return f"{address} <ambiguous>"
        return address


def build_model(project: dict) -> Model:
    names = uid_names(project)
    levels: dict[str, Lvl] = {}
    order: list[str] = []
    dupes: list[str] = []
    by_iid: dict[str, Ent] = {}
    address_counts: dict[str, int] = {}
    for key, raw_level, world in iter_levels(project):
        order.append(key)
        if key in levels:
            dupes.append(key)
        layers: dict[str, Layer] = {}
        layer_order: list[str] = []
        layer_dupes: list[str] = []
        for raw_layer in raw_level.get("layerInstances") or []:
            lid = str(raw_layer.get("__identifier"))
            layer_order.append(lid)
            if lid in layers:
                layer_dupes.append(lid)
            ents = [
                Ent(key, lid, i, raw_ent, read_fields(raw_ent.get("fieldInstances")))
                for i, raw_ent in enumerate(raw_layer.get("entityInstances") or [])
            ]
            layers[lid] = Layer(lid, raw_layer, ents)
        level = Lvl(key, world, raw_level, read_fields(raw_level.get("fieldInstances")), layers, layer_order, sorted(set(layer_dupes)))
        levels[key] = level
        for ent in level.entities():
            if ent.iid:
                by_iid[ent.iid] = ent
            address_counts[ent.address()] = address_counts.get(ent.address(), 0) + 1
    return Model(project, names, def_families(project, names), levels, order, sorted(set(dupes)), by_iid, address_counts)


# ---------------------------------------------------------------------------
# Canonical values
# ---------------------------------------------------------------------------


def canonical_value(value: Any, model: Model) -> Any:
    """A JSON value with refs written as content addresses."""
    if isinstance(value, Ref):
        return {"ref": model.ref_address(value)}
    if isinstance(value, list):
        return [canonical_value(v, model) for v in value]
    return value


def canonical_fields(fields: FieldSet, model: Model) -> dict[str, Any]:
    """Field values by name. A null value is left out: null and absent are equal."""
    return {
        name: canonical_value(fields.values[name], model)
        for name in sorted(fields.values)
        if fields.values[name] is not None
    }


def canonical_entity(ent: Ent, model: Model) -> dict[str, Any]:
    return {
        "identifier": ent.identifier,
        "layer": ent.layer,
        "px": ent.px,
        "size": ent.size,
        "pivot": ent.pivot,
        "fields": canonical_fields(ent.fields, model),
    }


def entity_signature(ent: Ent, model: Model) -> str:
    return json.dumps(canonical_entity(ent, model), sort_keys=True)


def tile_stacks(tiles: list[dict] | None) -> dict[tuple[int, int], list[tuple]]:
    """Tiles by pixel position. Each position keeps its stack (draw) order."""
    stacks: dict[tuple[int, int], list[tuple]] = {}
    for tile in tiles or []:
        px = tile.get("px") or [0, 0]
        src = tile.get("src") or [0, 0]
        alpha = tile.get("a", 1)
        stacks.setdefault((int(px[0]), int(px[1])), []).append(
            (int(src[0]), int(src[1]), int(tile.get("f", 0) or 0), float(1 if alpha is None else alpha))
        )
    return stacks


def layer_props(layer: Layer, model: Model) -> dict[str, Any]:
    raw = layer.raw
    return {
        "type": raw.get("__type"),
        "visible": raw.get("visible", True),
        "pxOffsetX": raw.get("pxOffsetX", 0),
        "pxOffsetY": raw.get("pxOffsetY", 0),
        "overrideTileset": resolve_uid(raw.get("overrideTilesetUid"), model.names),
        "optionalRules": sorted(str(v) for v in resolve_uid(raw.get("optionalRules") or [], model.names)),
    }


def level_props(level: Lvl) -> dict[str, Any]:
    return {key: level.raw.get(key) for key in LEVEL_PROPS}


def _grid_rows(layer: Layer) -> list[str]:
    csv = layer.raw.get("intGridCsv") or []
    width = int(layer.raw.get("__cWid") or 0) or len(csv) or 1
    return [",".join(str(v) for v in csv[i:i + width]) for i in range(0, len(csv), width)]


def _tile_rows(tiles: list[dict] | None) -> list[list]:
    stacks = tile_stacks(tiles)
    return [
        [x, y, *tile]
        for (x, y) in sorted(stacks, key=lambda p: (p[1], p[0]))
        for tile in stacks[(x, y)]
    ]


def canonical_level(level: Lvl, model: Model) -> dict[str, Any]:
    """One level in canonical form. The level name is not part of it."""
    layers: dict[str, Any] = {}
    for lid in sorted(level.layers):
        layer = level.layers[lid]
        row: dict[str, Any] = {"props": layer_props(layer, model)}
        if layer.raw.get("intGridCsv"):
            row["intgrid"] = _grid_rows(layer)
        if layer.raw.get("gridTiles"):
            row["tiles"] = _tile_rows(layer.raw.get("gridTiles"))
        if layer.raw.get("autoLayerTiles"):
            row["auto_tiles"] = _tile_rows(layer.raw.get("autoLayerTiles"))
        layers[lid] = row
    entities = sorted(
        (canonical_entity(e, model) for e in level.entities()),
        key=lambda e: (e["layer"], e["identifier"], e["px"][1], e["px"][0], json.dumps(e, sort_keys=True)),
    )
    return {
        "props": level_props(level),
        "fields": canonical_fields(level.fields, model),
        "layers": layers,
        "entities": entities,
    }


def canonical_project(project: dict, *, only_level: str | None = None) -> dict[str, Any]:
    """The whole id-free canonical form, for ``diff normalize``."""
    model = build_model(project)
    defs: dict[str, Any] = {
        name: {ident: fam.items[ident] for ident in sorted(fam.items)} for name, fam in model.defs.items()
    }
    defs["layer_order"] = model.defs["layers"].order
    levels = {
        key: canonical_level(model.levels[key], model)
        for key in sorted(model.levels)
        if only_level is None or key == only_level
    }
    settings = {key: project.get(key) for key in sorted(PROJECT_SEMANTIC)}
    worlds = {
        str(w.get("identifier")): {key: w.get(key) for key in sorted(WORLD_SEMANTIC)}
        for w in project.get("worlds") or []
    }
    out: dict[str, Any] = {"schema": CANONICAL_SCHEMA, "settings": settings, "defs": defs, "levels": levels}
    if worlds:
        out["worlds"] = worlds
    return out


def dumps_canonical(data: Any) -> str:
    return json.dumps(data, indent=1, sort_keys=True, ensure_ascii=False) + "\n"
