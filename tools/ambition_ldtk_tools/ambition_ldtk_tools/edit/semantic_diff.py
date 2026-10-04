#!/usr/bin/env python3
"""Semantic LDtk diff: authored changes apart from editor and serializer noise.

A raw LDtk diff mixes two kinds of change. An authored change moves a wall,
places an entity, sets a field or adds a room. Noise renumbers uids and iids,
reorders lists, refreshes ``__`` caches and rewrites editor settings. This
module compares two projects by meaning and reports the two kinds apart.

Rules that keep the report trustworthy:

- The diff does not match objects by uid or iid. Levels, layers and
  definitions match by identifier. Entities match by content first, then by
  stable keys (the authored ``id`` field, position, configuration, sole entity
  of its type). An iid is a match key only when the iids of the
  content-matched entities did not change.
- When two entities cannot be paired safely, the diff reports an ambiguity. It
  does not guess, and it does not call them equal.
- A raw key that the diff does not know is a semantic change.

Commands (``python -m ambition_ldtk_tools diff ...``)::

    diff semantic BEFORE AFTER     # each side: a file or REV:PATH
    diff range REV1..REV2 [--repo DIR] [PATH ...]
    diff normalize FILE|REV:PATH [--out OUT] [--level LEVEL]

Exit status: 0 when no authored change is found (identical or noise only), 1
when there is an authored change or an ambiguity, 2 for a usage error.
``normalize`` exits 0 and never writes to an ``.ldtk`` path.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable

from ambition_ldtk_tools.edit.ldtk_canonical import (
    ABSENT,
    ENTITY_DEF_VISUAL,
    ENTITY_HANDLED,
    ENTITY_NOISE,
    FIELD_HANDLED,
    FIELD_NOISE,
    IID,
    LAYER_HANDLED,
    LAYER_NOISE,
    LEVEL_HANDLED,
    LEVEL_NOISE,
    NOISE_DESCRIPTIONS,
    NULL_FIELD,
    ORDER,
    PROJECT_HANDLED,
    PROJECT_NOISE,
    PROJECT_SEMANTIC,
    TILE_CACHE,
    WORLD_HANDLED,
    WORLD_NOISE,
    WORLD_SEMANTIC,
    Ent,
    FieldSet,
    Layer,
    Lvl,
    Model,
    Ref,
    build_model,
    canonical_fields,
    canonical_level,
    canonical_project,
    canonical_value,
    dumps_canonical,
    entity_signature,
    field_default,
    layer_props,
    level_props,
    read_source,
    tile_stacks,
)

REPORT_SCHEMA = "ambition-ldtk-semantic-diff/1"

# The JSON report lists at most this many cells or tiles per layer change.
MAX_CELLS = 200
# The noise section keeps this many example paths per category.
MAX_EXAMPLES = 5

# Authored fields that name an entity, tried in this order. A value pairs two
# entities only when it is unique among the unpaired entities of that type on
# both sides, so a shared value (two doors named "exit") pairs nothing.
AUTHORED_KEY_FIELDS = ("id", "character_id", "name")

# Which summary column each change kind belongs to.
ASPECT = {
    "level_added": "geometry",
    "level_removed": "geometry",
    "level_renamed": "geometry",
    "level_moved": "geometry",
    "level_resized": "geometry",
    "level_prop": "geometry",
    "layer_added": "geometry",
    "layer_removed": "geometry",
    "layer_prop": "geometry",
    "intgrid": "geometry",
    "tiles": "tiles",
    "auto_tiles": "tiles",
    "level_field": "fields",
    "entity_field": "fields",
}


@dataclass(frozen=True)
class Change:
    kind: str
    path: str
    before: Any
    after: Any
    detail: str
    tier: str = "content"
    level: str | None = None
    data: Any = None


@dataclass(frozen=True)
class Ambiguity:
    kind: str
    path: str
    detail: str
    before: list
    after: list


@dataclass
class Noise:
    counts: Counter = field(default_factory=Counter)
    examples: dict[str, list[str]] = field(default_factory=dict)
    by_level: dict[str, Counter] = field(default_factory=dict)

    def add(self, category: str, path: str, level: str | None = None, n: int = 1) -> None:
        if n <= 0:
            return
        self.counts[category] += n
        self.examples.setdefault(category, []).append(path)
        if level is not None:
            self.by_level.setdefault(level, Counter())[category] += n


@dataclass
class Report:
    before_label: str
    after_label: str
    changes: list[Change] = field(default_factory=list)
    ambiguities: list[Ambiguity] = field(default_factory=list)
    noise: Noise = field(default_factory=Noise)
    warnings: list[str] = field(default_factory=list)
    level_keys: list[str] = field(default_factory=list)
    raw_lines: tuple[int, int] | None = None

    @property
    def verdict(self) -> str:
        if self.changes:
            return "changed"
        if self.ambiguities:
            return "ambiguous"
        if self.noise.counts or (self.raw_lines and any(self.raw_lines)):
            return "noise_only"
        return "identical"


# ---------------------------------------------------------------------------
# Small helpers
# ---------------------------------------------------------------------------


def _same(a: Any, b: Any) -> bool:
    return json.dumps(a, sort_keys=True, default=repr) == json.dumps(b, sort_keys=True, default=repr)


def _deep_diff(a: Any, b: Any, path: str = "") -> list[tuple[str, Any, Any]]:
    if isinstance(a, dict) and isinstance(b, dict):
        out: list[tuple[str, Any, Any]] = []
        for key in sorted(set(a) | set(b)):
            sub = f"{path}.{key}" if path else str(key)
            if key not in a:
                out.append((sub, None, b[key]))
            elif key not in b:
                out.append((sub, a[key], None))
            else:
                out.extend(_deep_diff(a[key], b[key], sub))
        return out
    if isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
        out = []
        for i, (x, y) in enumerate(zip(a, b)):
            out.extend(_deep_diff(x, y, f"{path}[{i}]"))
        return out
    return [] if _same(a, b) else [(path, a, b)]


def _short(value: Any, limit: int = 60) -> str:
    text = json.dumps(value, sort_keys=True, default=repr)
    return text if len(text) <= limit else text[: limit - 3] + "..."


def _describe_diffs(diffs: list[tuple[str, Any, Any]], limit: int = 4) -> str:
    def one(p: str, a: Any, b: Any) -> str:
        if a is None and isinstance(b, dict):
            return f"added {p}"
        if b is None and isinstance(a, dict):
            return f"removed {p}"
        return f"{p} {_short(a, 40)} -> {_short(b, 40)}"

    parts = [one(p, a, b) for p, a, b in diffs[:limit]]
    if len(diffs) > limit:
        parts.append(f"+{len(diffs) - limit} more")
    return "; ".join(parts)


def _classify_keys(
    report: Report,
    before: dict,
    after: dict,
    *,
    handled: set[str],
    noise: dict[str, str],
    path: str,
    level: str | None,
    semantic: set[str] = frozenset(),
    tier: str = "content",
    kind: str = "unknown_key",
) -> None:
    """Count noise keys that differ. Report semantic or unknown keys that differ."""
    for key in sorted(set(before) | set(after)):
        if key in handled:
            continue
        a = before.get(key, ABSENT)
        b = after.get(key, ABSENT)
        if _same(a, b):
            continue
        if key in noise:
            report.noise.add(noise[key], f"{path}.{key}", level)
            continue
        report.changes.append(Change(
            kind if key in semantic else "unknown_key",
            f"{path}.{key}",
            None if a is ABSENT else a,
            None if b is ABSENT else b,
            f"{path}: {key} {_short(None if a is ABSENT else a)} -> {_short(None if b is ABSENT else b)}"
            + ("" if key in semantic else " (key not known to the diff; reported to be safe)"),
            tier,
            level,
        ))


def _order_changed(before: list[str], after: list[str]) -> bool:
    common = set(before) & set(after)
    return [x for x in before if x in common] != [x for x in after if x in common]


# ---------------------------------------------------------------------------
# Definitions and project settings
# ---------------------------------------------------------------------------

DEF_KINDS = {
    "layers": "layer_def",
    "entities": "entity_def",
    "tilesets": "tileset",
    "enums": "enum_def",
    "externalEnums": "enum_def",
    "levelFields": "level_field_def",
}


def _compare_defs(report: Report, mb: Model, ma: Model) -> None:
    for family, kind in DEF_KINDS.items():
        fb, fa = mb.defs[family], ma.defs[family]
        for dup in sorted(set(fb.duplicates) | set(fa.duplicates)):
            report.ambiguities.append(Ambiguity(
                "duplicate_definition", f"defs.{family}.{dup}",
                f"two {family} definitions are named {dup}; they cannot be matched by name", [dup], [dup],
            ))
        added_kind = "tileset" if kind == "tileset" else f"{kind}_added"
        removed_kind = "tileset" if kind == "tileset" else f"{kind}_removed"
        changed_kind = "tileset" if kind == "tileset" else f"{kind}_changed"
        for ident in sorted(set(fb.items) | set(fa.items)):
            a = fb.items.get(ident)
            b = fa.items.get(ident)
            path = f"defs.{family}.{ident}"
            if a is None:
                report.changes.append(Change(added_kind, ident, None, b, f"added {kind} {ident}", "definition"))
                continue
            if b is None:
                report.changes.append(Change(removed_kind, ident, a, None, f"removed {kind} {ident}", "definition"))
                continue
            nb, na = fb.noise[ident], fa.noise[ident]
            for category in sorted(set(nb) | set(na)):
                if not _same(nb.get(category), na.get(category)):
                    report.noise.add(category, path)
            if family == "entities":
                vis_a = {k: a.get(k) for k in ENTITY_DEF_VISUAL}
                vis_b = {k: b.get(k) for k in ENTITY_DEF_VISUAL}
                diffs = _deep_diff(vis_a, vis_b)
                if diffs:
                    report.changes.append(Change(
                        "entity_def_visual", ident, vis_a, vis_b,
                        f"entity def {ident} editor visual changed: {_describe_diffs(diffs)}", "definition",
                    ))
                a = {k: v for k, v in a.items() if k not in ENTITY_DEF_VISUAL}
                b = {k: v for k, v in b.items() if k not in ENTITY_DEF_VISUAL}
            diffs = _deep_diff(a, b)
            if diffs:
                report.changes.append(Change(
                    changed_kind, ident, a, b, f"{kind} {ident} changed: {_describe_diffs(diffs)}", "definition",
                    data={"paths": [p for p, _, _ in diffs]},
                ))
        if family == "layers":
            if _order_changed(fb.order, fa.order):
                report.changes.append(Change(
                    "layer_order", "defs.layers", fb.order, fa.order,
                    "layer definition order (draw order) changed", "definition",
                ))
        elif _order_changed(fb.order, fa.order):
            report.noise.add(ORDER, f"defs.{family}")


def _compare_project(report: Report, mb: Model, ma: Model) -> None:
    pb = {k: v for k, v in mb.raw.items()}
    pa = {k: v for k, v in ma.raw.items()}
    pb_defs, pa_defs = pb.get("defs") or {}, pa.get("defs") or {}
    _classify_keys(
        report, pb, pa, handled=PROJECT_HANDLED, noise=PROJECT_NOISE, path="project", level=None,
        semantic=PROJECT_SEMANTIC, tier="definition", kind="project_setting",
    )
    known_defs = set(DEF_KINDS)
    _classify_keys(report, pb_defs, pa_defs, handled=known_defs, noise={}, path="defs", level=None, tier="definition")
    wb = {str(w.get("identifier")): w for w in pb.get("worlds") or []}
    wa = {str(w.get("identifier")): w for w in pa.get("worlds") or []}
    for wid in sorted(set(wb) | set(wa)):
        if wid not in wa:
            report.changes.append(Change("world_removed", wid, wid, None, f"removed world {wid}", "content"))
        elif wid not in wb:
            report.changes.append(Change("world_added", wid, None, wid, f"added world {wid}", "content"))
        else:
            _classify_keys(
                report, wb[wid], wa[wid], handled=WORLD_HANDLED, noise=WORLD_NOISE, path=f"world {wid}",
                level=None, semantic=WORLD_SEMANTIC, kind="world_setting",
            )


# ---------------------------------------------------------------------------
# Entity matching
# ---------------------------------------------------------------------------


@dataclass
class Matching:
    pairs: list[tuple[Ent, Ent, str]] = field(default_factory=list)
    after_of: dict[int, Ent] = field(default_factory=dict)
    removed: list[Ent] = field(default_factory=list)
    added: list[Ent] = field(default_factory=list)


def _pair(m: Matching, b: Ent, a: Ent, basis: str) -> None:
    m.pairs.append((b, a, basis))
    m.after_of[id(b)] = a


def _content_match(m: Matching, eb: list[Ent], ea: list[Ent], mb: Model, ma: Model) -> tuple[list[Ent], list[Ent], int, int]:
    """Pair entities whose whole canonical content is equal (iid ignored)."""
    groups_b: dict[str, list[Ent]] = {}
    groups_a: dict[str, list[Ent]] = {}
    for e in eb:
        groups_b.setdefault(entity_signature(e, mb), []).append(e)
    for e in ea:
        groups_a.setdefault(entity_signature(e, ma), []).append(e)
    rest_b: list[Ent] = []
    rest_a: list[Ent] = []
    same_iid = 0
    new_iid = 0
    for sig in sorted(set(groups_b) | set(groups_a)):
        lb = list(groups_b.get(sig, []))
        la = list(groups_a.get(sig, []))
        # Equal content is equal meaning. Prefer the pairing that keeps iids,
        # so that duplicates do not look like iid churn.
        by_iid = {e.iid: e for e in la if e.iid}
        for e in list(lb):
            match = by_iid.get(e.iid) if e.iid else None
            if match is not None and match in la:
                _pair(m, e, match, "content")
                lb.remove(e)
                la.remove(match)
                same_iid += 1
        lb.sort(key=lambda e: str(e.iid))
        la.sort(key=lambda e: str(e.iid))
        for e, f in zip(lb, la):
            _pair(m, e, f, "content")
            new_iid += 1
        rest_b.extend(lb[len(la):])
        rest_a.extend(la[len(lb):])
    return rest_b, rest_a, same_iid, new_iid


def _key_match(
    m: Matching, rest_b: list[Ent], rest_a: list[Ent], basis: str,
    key: Callable[[Ent, Model], Any], mb: Model, ma: Model,
) -> tuple[list[Ent], list[Ent]]:
    """Pair entities whose key is unique on both sides. Leave the rest."""
    groups_b: dict[str, list[Ent]] = {}
    groups_a: dict[str, list[Ent]] = {}
    for e in rest_b:
        k = key(e, mb)
        if k is not None:
            groups_b.setdefault(json.dumps(k, sort_keys=True, default=repr), []).append(e)
    for e in rest_a:
        k = key(e, ma)
        if k is not None:
            groups_a.setdefault(json.dumps(k, sort_keys=True, default=repr), []).append(e)
    paired_b: set[int] = set()
    paired_a: set[int] = set()
    for k in sorted(set(groups_b) & set(groups_a)):
        if len(groups_b[k]) == 1 and len(groups_a[k]) == 1:
            b, a = groups_b[k][0], groups_a[k][0]
            _pair(m, b, a, basis)
            paired_b.add(id(b))
            paired_a.add(id(a))
    return [e for e in rest_b if id(e) not in paired_b], [e for e in rest_a if id(e) not in paired_a]


def _match_entities(report: Report, mb: Model, ma: Model, common: list[str]) -> Matching:
    m = Matching()
    residual: dict[str, tuple[list[Ent], list[Ent], int, int]] = {}
    total_same = 0
    total_new = 0
    for key in common:
        rb, ra, same, new = _content_match(m, mb.levels[key].entities(), ma.levels[key].entities(), mb, ma)
        residual[key] = (rb, ra, same, new)
        total_same += same
        total_new += new
    project_trust = total_same > 0 and total_new == 0
    for key in common:
        rb, ra, same, new = residual[key]
        trusted = (same > 0 and new == 0) if (same or new) else project_trust
        phases: list[tuple[str, Callable[[Ent, Model], Any]]] = [
            *(
                (f"authored {name}", lambda e, _, name=name: [e.identifier, e.authored_key(name)] if e.authored_key(name) else None)
                for name in AUTHORED_KEY_FIELDS
            ),
            ("iid", lambda e, _: [e.identifier, e.iid] if trusted and e.iid else None),
            ("position", lambda e, _: [e.identifier, e.layer, e.px]),
            ("position", lambda e, _: [e.identifier, e.px]),
            ("configuration", lambda e, model: [e.identifier, e.size, canonical_fields(e.fields, model)]),
            ("sole of its type", lambda e, _: e.identifier),
        ]
        for basis, keyfn in phases:
            if not rb or not ra:
                break
            rb, ra = _key_match(m, rb, ra, basis, keyfn, mb, ma)
        types_b: dict[str, list[Ent]] = {}
        types_a: dict[str, list[Ent]] = {}
        for e in rb:
            types_b.setdefault(e.identifier, []).append(e)
        for e in ra:
            types_a.setdefault(e.identifier, []).append(e)
        for ident in sorted(set(types_b) | set(types_a)):
            lb = types_b.get(ident, [])
            la = types_a.get(ident, [])
            if lb and la:
                path = f"{key}/{ident}"
                report.ambiguities.append(Ambiguity(
                    "entity_unpaired", path,
                    f"{key}: {len(lb)} {ident} before and {len(la)} after differ, and no stable key pairs them;"
                    " compare them by hand",
                    sorted(e.address() for e in lb), sorted(e.address() for e in la),
                ))
                report.changes.append(Change(
                    "entity_unpaired", path, len(lb), len(la),
                    f"{key}: {ident} entities changed but cannot be paired safely ({len(lb)} before, {len(la)} after)",
                    "content", key,
                ))
            else:
                m.removed.extend(lb)
                m.added.extend(la)
    return m


# ---------------------------------------------------------------------------
# Field comparison
# ---------------------------------------------------------------------------


def _refs_equal(vb: Any, va: Any, mb: Model, m: Matching) -> bool:
    if isinstance(vb, Ref) and isinstance(va, Ref):
        target = mb.by_iid.get(vb.iid) if vb.iid else None
        if target is None:
            return vb.iid == va.iid
        mapped = m.after_of.get(id(target))
        return mapped is not None and mapped.iid == va.iid
    if isinstance(vb, list) and isinstance(va, list):
        return len(vb) == len(va) and all(_refs_equal(x, y, mb, m) for x, y in zip(vb, va))
    if isinstance(vb, (Ref, list)) or isinstance(va, (Ref, list)):
        if any(isinstance(x, Ref) for x in (vb if isinstance(vb, list) else [vb])) or any(
            isinstance(x, Ref) for x in (va if isinstance(va, list) else [va])
        ):
            return False
    return _same(vb, va)


def _compare_fields(
    report: Report,
    fb: FieldSet,
    fa: FieldSet,
    *,
    kind: str,
    path: str,
    level: str,
    label: str,
    fdef: Callable[[str], dict | None],
    mb: Model,
    ma: Model,
    m: Matching,
) -> None:
    for dup in sorted(set(fb.duplicates) | set(fa.duplicates)):
        # The diff compares the first instance below. Extra instances are
        # equal only when every instance is equal on both sides.
        if fb.duplicates == fa.duplicates and _refs_equal(fb.every.get(dup), fa.every.get(dup), mb, m):
            report.warnings.append(f"{label}: field {dup} has more than one instance in both files (not changed by this diff)")
            continue
        report.ambiguities.append(Ambiguity(
            "duplicate_field", f"{path}.{dup}",
            f"{label}: field {dup} has more than one instance, and the instances differ",
            [_short(canonical_value(v, mb)) for v in fb.every.get(dup, [])],
            [_short(canonical_value(v, ma)) for v in fa.every.get(dup, [])],
        ))
    if _order_changed(fb.order, fa.order):
        report.noise.add(ORDER, f"{path}.fieldInstances", level)
    for name in sorted(set(fb.values) | set(fa.values)):
        vb = fb.values.get(name, ABSENT)
        va = fa.values.get(name, ABSENT)
        if name in fb.raw and name in fa.raw:
            _classify_keys(report, fb.raw[name], fa.raw[name], handled=FIELD_HANDLED, noise=FIELD_NOISE,
                           path=f"{path}.{name}", level=level)
        if vb is not ABSENT and va is not ABSENT:
            if fb.types.get(name) != fa.types.get(name):
                report.changes.append(Change(kind, f"{path}.{name}", fb.types.get(name), fa.types.get(name),
                                             f"{label}: field {name} type {fb.types.get(name)} -> {fa.types.get(name)}",
                                             "content", level))
                continue
            if _refs_equal(vb, va, mb, m):
                continue
        elif (vb is ABSENT and va is None) or (vb is None and va is ABSENT):
            report.noise.add(NULL_FIELD, f"{path}.{name}", level)
            continue
        show_b = None if vb is ABSENT else canonical_value(vb, mb)
        show_a = None if va is ABSENT else canonical_value(va, ma)
        note = ""
        if vb is ABSENT or va is ABSENT:
            default = field_default(fdef(name))
            shown = "unknown" if default is ABSENT else _short(default, 40)
            note = f" (the {'before' if vb is ABSENT else 'after'} file has no instance; definition default {shown})"
        report.changes.append(Change(
            kind, f"{path}.{name}", show_b, show_a,
            f"{label}: field {name} {_short(show_b) if vb is not ABSENT else '<absent>'} -> "
            f"{_short(show_a) if va is not ABSENT else '<absent>'}{note}",
            "content", level,
        ))


# ---------------------------------------------------------------------------
# IntGrid and tiles
# ---------------------------------------------------------------------------


def _grid_cells(layer: Layer, level: Lvl) -> tuple[dict[tuple[int, int], int], int]:
    raw = layer.raw
    csv = raw.get("intGridCsv") or []
    grid = int(raw.get("__gridSize") or 16)
    width = int(raw.get("__cWid") or 0) or max(1, int(level.raw.get("pxWid") or grid) // grid)
    cells = {(i % width, i // width): int(v) for i, v in enumerate(csv) if v}
    return cells, grid


def _compare_intgrid(report: Report, key: str, lid: str, lb: Layer, la: Layer, levb: Lvl, leva: Lvl) -> None:
    if not (lb.raw.get("intGridCsv") or la.raw.get("intGridCsv")):
        return
    cb, grid = _grid_cells(lb, levb)
    ca, _ = _grid_cells(la, leva)
    changed = sorted(
        ((x, y, cb.get((x, y), 0), ca.get((x, y), 0)) for (x, y) in set(cb) | set(ca) if cb.get((x, y), 0) != ca.get((x, y), 0)),
        key=lambda c: (c[1], c[0]),
    )
    if not changed:
        return
    xs = [c[0] for c in changed]
    ys = [c[1] for c in changed]
    bbox = [min(xs), min(ys), max(xs), max(ys)]
    transitions = Counter(f"{c[2]}->{c[3]}" for c in changed)
    trans_text = ", ".join(f"{t} x{n}" for t, n in sorted(transitions.items()))
    report.changes.append(Change(
        "intgrid", f"{key}/{lid}", len(cb), len(ca),
        f"{key}/{lid}: {len(changed)} cells changed in cells x {bbox[0]}..{bbox[2]}, y {bbox[1]}..{bbox[3]}"
        f" (px {bbox[0] * grid},{bbox[1] * grid}); {trans_text}",
        "content", key,
        data={
            "changed_cells": len(changed),
            "bbox_cells": bbox,
            "grid_size": grid,
            "transitions": dict(sorted(transitions.items())),
            "cells": [list(c) for c in changed[:MAX_CELLS]],
            "truncated": len(changed) > MAX_CELLS,
        },
    ))


def _compare_tiles(report: Report, key: str, lid: str, lb: Layer, la: Layer, field_name: str, kind: str) -> None:
    tb = lb.raw.get(field_name) or []
    ta = la.raw.get(field_name) or []
    if not tb and not ta:
        return
    sb, sa = tile_stacks(tb), tile_stacks(ta)
    changed = sorted((p for p in set(sb) | set(sa) if sb.get(p) != sa.get(p)), key=lambda p: (p[1], p[0]))
    if changed:
        added = sum(1 for p in changed if p not in sb)
        removed = sum(1 for p in changed if p not in sa)
        replaced = len(changed) - added - removed
        xs = [p[0] for p in changed]
        ys = [p[1] for p in changed]
        bbox = [min(xs), min(ys), max(xs), max(ys)]
        report.changes.append(Change(
            kind, f"{key}/{lid}", sum(len(v) for v in sb.values()), sum(len(v) for v in sa.values()),
            f"{key}/{lid}: {kind.replace('_', '-')} changed at {len(changed)} positions "
            f"(+{added} -{removed} ~{replaced}) in px x {bbox[0]}..{bbox[2]}, y {bbox[1]}..{bbox[3]}",
            "content", key,
            data={
                "changed_positions": len(changed),
                "added": added,
                "removed": removed,
                "replaced": replaced,
                "bbox_px": bbox,
                "positions": [[p[0], p[1], sb.get(p), sa.get(p)] for p in changed[:MAX_CELLS]],
                "truncated": len(changed) > MAX_CELLS,
            },
        ))
    # Noise inside unchanged positions: tile id / rule uid caches, and order.
    cache_b: dict[tuple[int, int], list] = {}
    cache_a: dict[tuple[int, int], list] = {}
    for tiles, cache in ((tb, cache_b), (ta, cache_a)):
        for t in tiles:
            px = t.get("px") or [0, 0]
            cache.setdefault((int(px[0]), int(px[1])), []).append([t.get("t"), t.get("d")])
    stale = sum(1 for p in sb if p in sa and sb[p] == sa[p] and cache_b.get(p) != cache_a.get(p))
    report.noise.add(TILE_CACHE, f"{key}/{lid}.{field_name}", key, stale)
    order_b = [tuple(t.get("px") or [0, 0]) for t in tb]
    order_a = [tuple(t.get("px") or [0, 0]) for t in ta]
    if not changed and order_b != order_a:
        report.noise.add(ORDER, f"{key}/{lid}.{field_name}", key)


# ---------------------------------------------------------------------------
# Levels
# ---------------------------------------------------------------------------


def _compare_level_body(report: Report, key: str, lb: Lvl, la: Lvl, mb: Model, ma: Model, m: Matching) -> None:
    _classify_keys(report, lb.raw, la.raw, handled=LEVEL_HANDLED, noise=LEVEL_NOISE, path=key, level=key)
    pb, pa = level_props(lb), level_props(la)
    pos_b, pos_a = [pb["worldX"], pb["worldY"]], [pa["worldX"], pa["worldY"]]
    size_b, size_a = [pb["pxWid"], pb["pxHei"]], [pa["pxWid"], pa["pxHei"]]
    if pos_b != pos_a:
        report.changes.append(Change("level_moved", key, pos_b, pos_a, f"{key}: moved {pos_b} -> {pos_a}", "content", key))
    if size_b != size_a:
        report.changes.append(Change("level_resized", key, size_b, size_a, f"{key}: resized {size_b} -> {size_a}", "content", key))
    for prop in sorted(pb):
        if prop in ("worldX", "worldY", "pxWid", "pxHei"):
            continue
        if not _same(pb[prop], pa[prop]):
            report.changes.append(Change("level_prop", f"{key}.{prop}", pb[prop], pa[prop],
                                         f"{key}: {prop} {_short(pb[prop])} -> {_short(pa[prop])}", "content", key))
    _compare_fields(
        report, lb.fields, la.fields, kind="level_field", path=key, level=key, label=key,
        fdef=ma.level_field_def, mb=mb, ma=ma, m=m,
    )
    for dup in sorted(set(lb.layer_duplicates) | set(la.layer_duplicates)):
        report.ambiguities.append(Ambiguity("duplicate_layer", f"{key}/{dup}", f"{key}: two layer instances are named {dup}", [dup], [dup]))
    if lb.layer_order != la.layer_order and set(lb.layer_order) == set(la.layer_order):
        report.noise.add(ORDER, f"{key}.layerInstances", key)
    for lid in sorted(set(lb.layers) | set(la.layers)):
        layer_b = lb.layers.get(lid)
        layer_a = la.layers.get(lid)
        if layer_b is None:
            report.changes.append(Change("layer_added", f"{key}/{lid}", None, lid, f"{key}: added layer instance {lid}", "content", key))
            if layer_a is not None:
                empty = Layer(lid, {}, [])
                _compare_intgrid(report, key, lid, empty, layer_a, lb, la)
                _compare_tiles(report, key, lid, empty, layer_a, "gridTiles", "tiles")
                _compare_tiles(report, key, lid, empty, layer_a, "autoLayerTiles", "auto_tiles")
            continue
        if layer_a is None:
            report.changes.append(Change("layer_removed", f"{key}/{lid}", lid, None, f"{key}: removed layer instance {lid}", "content", key))
            empty = Layer(lid, {}, [])
            _compare_intgrid(report, key, lid, layer_b, empty, lb, la)
            _compare_tiles(report, key, lid, layer_b, empty, "gridTiles", "tiles")
            _compare_tiles(report, key, lid, layer_b, empty, "autoLayerTiles", "auto_tiles")
            continue
        _classify_keys(report, layer_b.raw, layer_a.raw, handled=LAYER_HANDLED, noise=LAYER_NOISE, path=f"{key}/{lid}", level=key)
        props_b, props_a = layer_props(layer_b, mb), layer_props(layer_a, ma)
        diffs = _deep_diff(props_b, props_a)
        if diffs:
            report.changes.append(Change("layer_prop", f"{key}/{lid}", props_b, props_a,
                                         f"{key}/{lid}: {_describe_diffs(diffs)}", "content", key))
        _compare_intgrid(report, key, lid, layer_b, layer_a, lb, la)
        _compare_tiles(report, key, lid, layer_b, layer_a, "gridTiles", "tiles")
        _compare_tiles(report, key, lid, layer_b, layer_a, "autoLayerTiles", "auto_tiles")


def _compare_entities(report: Report, mb: Model, ma: Model, m: Matching) -> None:
    identity_risk = 0
    for e in sorted(m.removed, key=lambda e: e.address()):
        report.changes.append(Change(
            "entity_removed", e.address(), _entity_row(e, mb), None,
            f"{e.level}: removed {e.label()} at {e.px} from {e.layer}", "content", e.level,
        ))
    for e in sorted(m.added, key=lambda e: e.address()):
        report.changes.append(Change(
            "entity_added", e.address(), None, _entity_row(e, ma),
            f"{e.level}: added {e.label()} at {e.px} on {e.layer}", "content", e.level,
        ))
    for b, a, basis in sorted(m.pairs, key=lambda p: p[0].address()):
        path = b.address()
        key = b.level
        label = f"{key}: {b.label()} at {b.px}"
        if b.iid != a.iid:
            report.noise.add(IID, path, key)
            if b.authored_id is None and a.authored_id is None:
                identity_risk += 1
        _classify_keys(report, b.raw, a.raw, handled=ENTITY_HANDLED, noise=ENTITY_NOISE, path=path, level=key)
        if b.layer != a.layer:
            report.changes.append(Change("entity_layer", path, b.layer, a.layer, f"{label}: layer {b.layer} -> {a.layer}", "content", key))
        if b.px != a.px:
            report.changes.append(Change("entity_moved", path, b.px, a.px,
                                         f"{key}: {b.label()} moved {b.px} -> {a.px} (paired by {basis})", "content", key))
        if b.size != a.size:
            report.changes.append(Change("entity_resized", path, b.size, a.size, f"{label}: resized {b.size} -> {a.size}", "content", key))
        if b.pivot != a.pivot:
            report.changes.append(Change("entity_pivot", path, b.pivot, a.pivot, f"{label}: pivot {b.pivot} -> {a.pivot}", "content", key))
        _compare_fields(
            report, b.fields, a.fields, kind="entity_field", path=path, level=key,
            label=label + ("" if basis == "content" else f" (paired by {basis})"),
            fdef=lambda name, ident=a.identifier: ma.entity_def_field(ident, name), mb=mb, ma=ma, m=m,
        )
    # Order within a layer has no meaning.
    for key in sorted(set(mb.levels) & set(ma.levels)):
        for lid, layer in mb.levels[key].layers.items():
            after = [m.after_of.get(id(e)) for e in layer.entities]
            idx = [e.index for e in after if e is not None and e.layer == lid]
            if idx != sorted(idx):
                report.noise.add(ORDER, f"{key}/{lid}.entityInstances", key)
    if identity_risk:
        report.warnings.append(
            f"{identity_risk} entities changed iid and have no 'id' field. The runtime uses the iid as the id of"
            " such an entity (placements, props, debug labels), so state saved against the old id does not carry over."
        )


def _entity_row(e: Ent, model: Model) -> dict:
    return {
        "identifier": e.identifier,
        "layer": e.layer,
        "px": e.px,
        "size": e.size,
        "fields": {k: canonical_value(v, model) for k, v in sorted(e.fields.values.items()) if v is not None},
    }


def _compare_levels(report: Report, mb: Model, ma: Model) -> None:
    for dup in sorted(set(mb.level_duplicates) | set(ma.level_duplicates)):
        report.ambiguities.append(Ambiguity("duplicate_level", dup, f"two levels are named {dup}; they cannot be matched", [dup], [dup]))
    dupes = set(mb.level_duplicates) | set(ma.level_duplicates)
    if _order_changed(mb.level_order, ma.level_order):
        report.noise.add(ORDER, "levels")
    only_b = sorted(set(mb.levels) - set(ma.levels) - dupes)
    only_a = sorted(set(ma.levels) - set(mb.levels) - dupes)
    # A level whose whole content is unchanged under a new name is a rename.
    prints_b = {k: json.dumps(canonical_level(mb.levels[k], mb), sort_keys=True) for k in only_b}
    prints_a = {k: json.dumps(canonical_level(ma.levels[k], ma), sort_keys=True) for k in only_a}
    renamed: list[tuple[str, str]] = []
    for kb in only_b:
        hits = [ka for ka in only_a if prints_a[ka] == prints_b[kb]]
        twins = [k for k in only_b if prints_b[k] == prints_b[kb]]
        if len(hits) == 1 and len(twins) == 1:
            renamed.append((kb, hits[0]))
    for kb, ka in renamed:
        only_b.remove(kb)
        only_a.remove(ka)
        report.changes.append(Change("level_renamed", kb, kb, ka, f"renamed level {kb} -> {ka}; content unchanged", "content", ka))
    for key in only_b:
        lv = mb.levels[key]
        report.changes.append(Change("level_removed", key, _level_pos(lv), None, f"removed level {key} ({len(lv.entities())} entities)", "content", key))
    for key in only_a:
        lv = ma.levels[key]
        report.changes.append(Change("level_added", key, None, _level_pos(lv), f"added level {key} ({len(lv.entities())} entities)", "content", key))
    common = sorted((set(mb.levels) & set(ma.levels)) - dupes)
    report.level_keys = sorted((set(mb.levels) | set(ma.levels)))
    m = _match_entities(report, mb, ma, common)
    for key in common:
        _compare_level_body(report, key, mb.levels[key], ma.levels[key], mb, ma, m)
    _compare_entities(report, mb, ma, m)


def _level_pos(level: Lvl) -> list:
    return [level.raw.get("worldX"), level.raw.get("worldY")]


# ---------------------------------------------------------------------------
# Entry points
# ---------------------------------------------------------------------------


def raw_line_delta(before_text: str, after_text: str) -> tuple[int, int]:
    """Lines added and removed, counted as a line multiset (order ignored)."""
    cb = Counter(before_text.splitlines())
    ca = Counter(after_text.splitlines())
    return sum((ca - cb).values()), sum((cb - ca).values())


def compare(
    before: dict,
    after: dict,
    *,
    before_label: str = "before",
    after_label: str = "after",
    before_text: str | None = None,
    after_text: str | None = None,
) -> Report:
    report = Report(before_label, after_label)
    mb, ma = build_model(before), build_model(after)
    _compare_project(report, mb, ma)
    _compare_defs(report, mb, ma)
    _compare_levels(report, mb, ma)
    if before_text is not None and after_text is not None:
        report.raw_lines = raw_line_delta(before_text, after_text)
    report.changes.sort(key=lambda c: (c.tier, c.level or "", c.kind, c.path))
    report.ambiguities.sort(key=lambda a: (a.kind, a.path))
    return report


def semantic_changes(before: dict, after: dict) -> list[Change]:
    """The authored changes between two projects (the noise is left out)."""
    return compare(before, after).changes


def level_summary(report: Report) -> dict[str, dict[str, Any]]:
    """Per level: which aspects changed, and the noise counts."""
    rows: dict[str, dict[str, Any]] = {}
    for key in report.level_keys:
        rows[key] = {"geometry": "unchanged", "entities": "unchanged", "fields": "unchanged", "tiles": "unchanged",
                     "status": "unchanged", "noise": dict(sorted(report.noise.by_level.get(key, Counter()).items()))}
    for change in report.changes:
        if change.level not in rows:
            continue
        row = rows[change.level]
        if change.kind in ("level_added", "level_removed", "level_renamed"):
            row["status"] = change.kind.split("_")[1]
            continue
        aspect = ASPECT.get(change.kind, "entities")
        row[aspect] = "changed"
        row["status"] = "changed"
    for amb in report.ambiguities:
        key = amb.path.split("/")[0]
        if key in rows:
            rows[key]["entities"] = "ambiguous"
            rows[key]["status"] = "changed"
    return rows


def report_json(report: Report) -> dict[str, Any]:
    content = sum(1 for c in report.changes if c.tier == "content")
    return {
        "schema": REPORT_SCHEMA,
        "before": report.before_label,
        "after": report.after_label,
        "verdict": report.verdict,
        "semantic_change": report.verdict in ("changed", "ambiguous"),
        "summary": {
            "content_changes": content,
            "definition_changes": len(report.changes) - content,
            "ambiguities": len(report.ambiguities),
            "noise": dict(sorted(report.noise.counts.items())),
            "raw_lines": None if report.raw_lines is None else {"added": report.raw_lines[0], "removed": report.raw_lines[1]},
        },
        "levels": level_summary(report),
        "changes": [
            {k: v for k, v in {
                "kind": c.kind, "tier": c.tier, "level": c.level, "path": c.path,
                "before": c.before, "after": c.after, "detail": c.detail, "data": c.data,
            }.items() if v is not None or k in ("before", "after")}
            for c in report.changes
        ],
        "ambiguities": [
            {"kind": a.kind, "path": a.path, "detail": a.detail, "before": a.before, "after": a.after}
            for a in report.ambiguities
        ],
        "noise": {
            cat: {
                "count": report.noise.counts[cat],
                "meaning": NOISE_DESCRIPTIONS.get(cat, ""),
                "examples": sorted(report.noise.examples.get(cat, []))[:MAX_EXAMPLES],
            }
            for cat in sorted(report.noise.counts)
        },
        "warnings": list(report.warnings),
    }


def format_text(changes_or_report: Report | list[Change]) -> str:
    if isinstance(changes_or_report, list):
        report = Report("before", "after", changes=list(changes_or_report))
    else:
        report = changes_or_report
    verdict_text = {
        "identical": "IDENTICAL - no difference at all",
        "noise_only": "NOISE ONLY - no authored change",
        "changed": "CHANGED - authored changes found",
        "ambiguous": "AMBIGUOUS - the diff cannot prove the files are equal",
    }[report.verdict]
    content = sum(1 for c in report.changes if c.tier == "content")
    lines = [
        f"LDtk semantic diff: {report.before_label} -> {report.after_label}",
        f"Verdict: {verdict_text}",
        f"  content changes: {content}; definition changes: {len(report.changes) - content};"
        f" ambiguities: {len(report.ambiguities)}",
    ]
    if report.raw_lines is not None:
        lines.append(f"  raw text: +{report.raw_lines[0]} -{report.raw_lines[1]} lines")
    rows = level_summary(report)
    changed = {k: r for k, r in rows.items() if r["status"] != "unchanged"}
    noisy = sum(1 for r in rows.values() if r["status"] == "unchanged" and r["noise"])
    quiet = len(rows) - len(changed) - noisy
    if rows:
        lines += ["", "Levels:"]
        for key, row in changed.items():
            if row["status"] in ("added", "removed", "renamed"):
                lines.append(f"  {key}: {row['status']}")
                continue
            aspects = "; ".join(f"{a} {row[a]}" for a in ("geometry", "entities", "fields", "tiles"))
            noise = sum(row["noise"].values())
            lines.append(f"  {key}: {aspects}; noise items: {noise}")
        if noisy:
            lines.append(f"  {noisy} levels: noise only (no authored change)")
        if quiet:
            lines.append(f"  {quiet} levels: no difference")
    for tier, title in (("content", "Content changes"), ("definition", "Definition changes")):
        tier_changes = [c for c in report.changes if c.tier == tier]
        if not tier_changes:
            continue
        lines += ["", f"{title}:"]
        for change in tier_changes:
            lines.append(f"  [{change.kind}] {change.detail}")
    if report.ambiguities:
        lines += ["", "Ambiguities (not proven equal; check by hand):"]
        for amb in report.ambiguities:
            lines.append(f"  [{amb.kind}] {amb.detail}")
            if amb.before != amb.after:
                lines.append(f"      before: {', '.join(amb.before[:6])}{' ...' if len(amb.before) > 6 else ''}")
                lines.append(f"      after:  {', '.join(amb.after[:6])}{' ...' if len(amb.after) > 6 else ''}")
    if report.noise.counts:
        lines += ["", "Noise (ignored):"]
        for cat in sorted(report.noise.counts):
            example = sorted(report.noise.examples.get(cat, []))[:1]
            lines.append(f"  {cat}: {report.noise.counts[cat]}  - {NOISE_DESCRIPTIONS.get(cat, '')}"
                         + (f" (e.g. {example[0]})" if example else ""))
    if report.warnings:
        lines += ["", "Warnings:"]
        lines += [f"  {w}" for w in report.warnings]
    return "\n".join(lines) + "\n"


def _filter_kinds(report: Report, kinds: list[str]) -> None:
    if kinds:
        wanted = set(kinds)
        report.changes = [c for c in report.changes if c.kind in wanted]


def _run_semantic(args: argparse.Namespace) -> int:
    src_b = read_source(args.before)
    src_a = read_source(args.after)
    report = compare(
        json.loads(src_b.text), json.loads(src_a.text),
        before_label=src_b.label, after_label=src_a.label,
        before_text=src_b.text, after_text=src_a.text,
    )
    _filter_kinds(report, args.kind)
    if args.format == "json":
        print(json.dumps(report_json(report), indent=2, sort_keys=True, default=repr))
    else:
        print(format_text(report), end="")
    return 1 if report.verdict in ("changed", "ambiguous") else 0


def _git(repo: str, *argv: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", repo, *argv], capture_output=True, text=True)


def _run_range(args: argparse.Namespace) -> int:
    if ".." not in args.range:
        print("range must be REV1..REV2", file=sys.stderr)
        return 2
    rev_b, rev_a = args.range.split("..", 1)
    names = _git(args.repo, "diff", "--name-only", rev_b, rev_a, "--", *(args.paths or ["*.ldtk"]))
    if names.returncode != 0:
        print(names.stderr, file=sys.stderr, end="")
        return 2
    files = sorted(p for p in names.stdout.splitlines() if p.endswith(".ldtk"))
    out: dict[str, Any] = {}
    status = 0
    texts: list[str] = []
    for path in files:
        before = _git(args.repo, "show", f"{rev_b}:{path}")
        after = _git(args.repo, "show", f"{rev_a}:{path}")
        if before.returncode != 0 or after.returncode != 0:
            state = "added" if before.returncode != 0 else "removed"
            out[path] = {"file": state}
            texts.append(f"== {path}: file {state}\n")
            status = 1
            continue
        report = compare(
            json.loads(before.stdout), json.loads(after.stdout),
            before_label=f"{rev_b}:{path}", after_label=f"{rev_a}:{path}",
            before_text=before.stdout, after_text=after.stdout,
        )
        _filter_kinds(report, args.kind)
        out[path] = report_json(report)
        texts.append(f"== {path}\n{format_text(report)}")
        if report.verdict in ("changed", "ambiguous"):
            status = 1
    if args.format == "json":
        print(json.dumps({"schema": REPORT_SCHEMA, "range": args.range, "files": out}, indent=2, sort_keys=True, default=repr))
    else:
        print("\n".join(texts) if texts else f"No .ldtk file changed in {args.range}.\n", end="")
    return status


def _run_normalize(args: argparse.Namespace) -> int:
    src = read_source(args.source)
    text = dumps_canonical(canonical_project(json.loads(src.text), only_level=args.level))
    if args.out is None:
        sys.stdout.write(text)
        return 0
    out = Path(args.out)
    if out.suffix == ".ldtk" or (Path(args.source).exists() and out.resolve() == Path(args.source).resolve()):
        print("refusing to write canonical JSON over an .ldtk file; choose another --out", file=sys.stderr)
        return 2
    out.write_text(text)
    print(f"wrote {out}", file=sys.stderr)
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Semantic LDtk diff: authored changes apart from editor noise.")
    sub = ap.add_subparsers(dest="action", required=True)
    sem = sub.add_parser("semantic", help="Compare two LDtk projects (a file or REV:PATH each).")
    sem.add_argument("before")
    sem.add_argument("after")
    rng = sub.add_parser("range", help="Compare every .ldtk file that changed between two commits.")
    rng.add_argument("range", help="REV1..REV2")
    rng.add_argument("paths", nargs="*", help="Limit to these paths (default: every *.ldtk).")
    rng.add_argument("--repo", default=".", help="Git repository to read (for example a submodule path).")
    for p in (sem, rng):
        p.add_argument("--format", choices=["text", "json"], default="text")
        p.add_argument("--kind", action="append", default=[], help="Keep only this change kind; repeatable.")
    norm = sub.add_parser("normalize", help="Print the id-free canonical form (never edits the input).")
    norm.add_argument("source")
    norm.add_argument("--out", help="Write to this file instead of stdout (an .ldtk path is refused).")
    norm.add_argument("--level", help="Keep only this level.")
    args = ap.parse_args(argv)
    if args.action == "semantic":
        return _run_semantic(args)
    if args.action == "range":
        return _run_range(args)
    return _run_normalize(args)


if __name__ == "__main__":
    raise SystemExit(main())
