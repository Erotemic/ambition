#!/usr/bin/env python3
"""Move whole levels (rooms) from one LDtk world file to another.

Jon (2026-10-29): "I want to make it such that the ldtk files are broken up by
the dimension they corresponds to in the game." One world file is one
dimension (`worlds.ron`), and the rooms of every file join one room set, so a
room keeps its identifier, its entity iids (save keys and encounter ids name
them) and every `LoadingZone` that leads to it by room id, wherever it lives.

The move refuses, and writes nothing, when:

- a level is not in the source, or is in the target already;
- an `EntityRef` would cross the two files (from a moved level to an entity
  that stays, or from a level that stays to an entity that moves): a reference
  resolves only inside its own file;
- a layer, entity or tileset def that a moved level uses is not the same in
  the target's defs (its uid, and all of it: its fields, IntGrid values,
  auto-layer rules): the level would mean something else there.

What it rewrites in each moved level: its `uid` (and its layers' `levelId`),
minted from the target's counter, so the two files never spend one uid; and the
`worldIid` of each of its references, which names the file it is in. The
target's `nextUid` stays one past what it uses.

Usage:

    ambition_ldtk_tools world move-levels <source.ldtk> <target.ldtk> \\
        --level primeval_trailhead --level primeval_run ...  [--prefix primeval_]
        [--init --identifier ambition-dinosaur-world --uid-base 600000]

`--init` makes the target first (`world init` from the source's defs) with
its counter at `--uid-base`. Validate the pair afterwards with
`validate <primary> --secondary-world <target>`.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

from ambition_ldtk_tools.ldtk.io import load_project, write_project


def _levels(project: dict) -> list[dict]:
    return project.setdefault("levels", [])


def _refs(level: dict):
    """Every EntityRef value in a level's entities, with its owner."""
    for layer in level.get("layerInstances") or []:
        for entity in layer.get("entityInstances", []):
            for field in entity.get("fieldInstances", []):
                value = field.get("__value")
                for item in value if isinstance(value, list) else [value]:
                    if isinstance(item, dict) and "entityIid" in item:
                        yield entity, field, item


def _entity_iids(level: dict) -> set[str]:
    return {e["iid"] for layer in level.get("layerInstances") or [] for e in layer.get("entityInstances", [])}


def _def_uids(level: dict) -> set[tuple[str, int, str]]:
    """The defs a level uses: (kind, uid, identifier)."""
    used = set()
    for layer in level.get("layerInstances") or []:
        used.add(("layer", layer["layerDefUid"], layer["__identifier"]))
        if layer.get("__tilesetDefUid") is not None:
            used.add(("tileset", layer["__tilesetDefUid"], ""))
        for entity in layer.get("entityInstances", []):
            used.add(("entity", entity["defUid"], entity["__identifier"]))
    return used


def _defs_index(project: dict) -> dict[tuple[str, int], dict]:
    defs = project.get("defs", {})
    index = {("layer", d["uid"]): d for d in defs.get("layers", [])}
    index.update({("entity", d["uid"]): d for d in defs.get("entities", [])})
    index.update({("tileset", d["uid"]): d for d in defs.get("tilesets", [])})
    return index


def plan(source: dict, target: dict, names: list[str]) -> list[str]:
    """Why the move cannot be made (empty: it can)."""
    problems = []
    by_name = {level["identifier"]: level for level in _levels(source)}
    in_target = {level["identifier"] for level in _levels(target)}
    moving = []
    for name in names:
        if name not in by_name:
            problems.append(f"level '{name}' is not in the source")
        elif name in in_target:
            problems.append(f"level '{name}' is in the target already")
        else:
            moving.append(by_name[name])
    moved_iids = set().union(*(_entity_iids(level) for level in moving)) if moving else set()
    for level in _levels(source):
        moves = level["identifier"] in names
        for entity, field, ref in _refs(level):
            if moves != (ref["entityIid"] in moved_iids):
                where = "moves" if moves else "stays"
                problems.append(f"{level['identifier']} ({where}): {entity['__identifier']}.{field['__identifier']} "
                                f"refers to {ref['entityIid']} across the two files")
    source_defs, target_defs = _defs_index(source), _defs_index(target)
    for level in moving:
        for kind, uid, identifier in sorted(_def_uids(level)):
            have = target_defs.get((kind, uid))
            if have is None:
                problems.append(f"{level['identifier']}: the target has no {kind} def {uid} ({identifier or '?'})")
            elif have != source_defs.get((kind, uid)):
                problems.append(f"{level['identifier']}: the target's {kind} def {uid} ({have.get('identifier')}) is not the source's")
    return problems


def move(source: dict, target: dict, names: list[str]) -> list[str]:
    """Move the levels `names` (all checked by `plan`) from `source` into
    `target`. Returns a line for each."""
    out = []
    moving = [level for level in _levels(source) if level["identifier"] in names]
    source["levels"] = [level for level in _levels(source) if level["identifier"] not in names]
    for world in source.get("worlds", []) or []:
        if "levels" in world:
            world["levels"] = [level for level in world["levels"] if level["identifier"] not in names]
    for level in moving:
        level = copy.deepcopy(level)
        uid = int(target["nextUid"])
        target["nextUid"] = uid + 1
        old = level["uid"]
        level["uid"] = uid
        for layer in level.get("layerInstances") or []:
            layer["levelId"] = uid
        for _entity, _field, ref in _refs(level):
            ref["worldIid"] = target["iid"]
        # The neighbours of a level are recomputed in its new file.
        level["__neighbours"] = []
        _levels(target).append(level)
        out.append(f"  - moved '{level['identifier']}' (uid {old} -> {uid}, at {level['worldX']},{level['worldY']})")
    return out


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("source", type=Path)
    parser.add_argument("target", type=Path)
    parser.add_argument("--level", action="append", default=[], help="a level identifier to move (repeatable)")
    parser.add_argument("--prefix", action="append", default=[], help="move every level whose identifier starts so")
    parser.add_argument("--init", action="store_true", help="make the target first, from the source's defs")
    parser.add_argument("--identifier", default=None, help="with --init: the new world's iid")
    parser.add_argument("--uid-base", type=int, default=None, help="with --init: the new world's first uid")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)

    source = load_project(args.source)
    if args.init:
        if args.target.exists():
            print(f"error: {args.target} exists; drop --init to move into it", file=sys.stderr)
            return 2
        from ambition_ldtk_tools import world_init

        if args.uid_base is None:
            print("error: --init needs --uid-base (a range no other world file spends)", file=sys.stderr)
            return 2
        offset = args.uid_base - int(source.get("nextUid", 1))
        init_args = [str(args.target), "--source", str(args.source), "--uid-offset", str(offset)]
        if args.identifier:
            init_args += ["--identifier", args.identifier]
        if args.dry_run:
            target = json.loads(json.dumps(source))
            target.update(iid=args.identifier or "dry-run", nextUid=args.uid_base, levels=[])
        else:
            code = world_init.main(init_args)
            if code:
                return code
            target = load_project(args.target)
    else:
        target = load_project(args.target)

    names = list(dict.fromkeys(args.level + [level["identifier"] for level in _levels(source)
                                             if any(level["identifier"].startswith(p) for p in args.prefix)]))
    if not names:
        print("error: name the levels to move (--level / --prefix)", file=sys.stderr)
        return 2
    problems = plan(source, target, names)
    if problems:
        print("refused; nothing written:", file=sys.stderr)
        for line in problems:
            print(f"  {line}", file=sys.stderr)
        return 1
    for line in move(source, target, names):
        print(line)
    if args.dry_run:
        print(f"dry run: {len(names)} level(s) would move")
        return 0
    write_project(args.target, target)
    write_project(args.source, source)
    print(f"moved {len(names)} level(s) from {args.source} to {args.target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
