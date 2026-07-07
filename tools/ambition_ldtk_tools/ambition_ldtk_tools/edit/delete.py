#!/usr/bin/env python3
"""Delete entity instances from an LDtk level.

Companion to `entity add` / `entity set-field` / `entity move`. Use
this to remove an authored entity — most often a stale `LoadingZone`
when you're relocating it to a different level, or a leftover prop
that no longer belongs in the room.

Spec format (YAML or JSON):

    level_id: central_hub_basement
    deletes:
      - target:
          # Either select by `iid` (precise; survives renames):
          iid: LoadingZone-4346
          # …or by entity identifier + a field/value match:
          # identifier: LoadingZone
          # match:
          #   id: intro_wake_door

You may also pass `--all-matching` to delete *every* matching entity
(useful for bulk-clearing a deprecated identifier across a level).
Without that flag, ambiguous selectors error out — same strict
semantics as `set-field` so a typo doesn't quietly nuke the wrong
door.

The repair + validate pass runs on the way out, identical to the
other edit commands. Pass `--no-repair` to skip the post-pass.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/delete.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.edit.postprocess import run_repair_and_validate
from ambition_ldtk_tools.ldtk.transaction import LdtkTransaction

from ambition_ldtk_tools.area_authoring import (  # noqa: E402
    load_project,
    write_project,
)
from ambition_ldtk_tools.edit.set_field import (  # noqa: E402
    find_ambition_layer,
    find_level,
    load_spec,
    _entity_field_value,
)


def select_for_delete(layer: dict, target: dict, all_matching: bool) -> list[dict]:
    """Like set_field.select_entities, but `--all-matching` relaxes the
    "exactly one" rule. Without that flag, the same strict semantics
    apply (no zero-match, no ambiguous-match)."""
    instances = layer.get("entityInstances", [])
    iid = target.get("iid")
    if iid is not None:
        matched = [e for e in instances if e.get("iid") == iid]
        if not matched:
            raise SystemExit(f"no entity with iid '{iid}' in level")
        return matched
    identifier = target.get("identifier")
    match = target.get("match") or {}
    if identifier is None:
        raise SystemExit(
            "target must include either `iid` or `identifier` (with optional `match`)"
        )
    candidates = [e for e in instances if e.get("__identifier") == identifier]
    for fname, fvalue in match.items():
        candidates = [e for e in candidates if _entity_field_value(e, fname) == fvalue]
    if not candidates:
        raise SystemExit(f"no entity '{identifier}' matched fields {match!r}")
    if len(candidates) > 1 and not all_matching:
        ids = [c.get("iid", "<no-iid>") for c in candidates]
        raise SystemExit(
            f"target '{identifier}' / {match!r} is ambiguous, matched: {ids}. "
            f"Tighten the match selector, use iid, or pass --all-matching."
        )
    return candidates


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("spec", type=Path)
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=REPO_ROOT
        / "crates"
        / "ambition_actors"
        / "assets"
        / "ambition"
        / "worlds"
        / "sandbox.ldtk",
    )
    parser.add_argument("--in-place", action="store_true")
    parser.add_argument("--output", type=Path, default=None)
    parser.add_argument("--backup", action="store_true")
    parser.add_argument("--no-repair", action="store_true")
    parser.add_argument(
        "--all-matching",
        action="store_true",
        help=(
            "Allow a single target to delete multiple matching entities. "
            "Without this flag, an ambiguous match errors out (safer default)."
        ),
    )
    parser.add_argument(
        "--schema",
        type=Path,
        default=REPO_ROOT
        / "tools"
        / "ambition_ldtk_tools"
        / "schemas"
        / "ldtk"
        / "JSON_SCHEMA.json",
    )
    args = parser.parse_args(argv)

    if not args.in_place and args.output is None:
        return _fail("choose --in-place or --output <path>")

    spec = load_spec(args.spec)
    if not isinstance(spec, dict):
        return _fail(f"spec must be a mapping, got {type(spec).__name__}")
    level_id = spec.get("level_id")
    if not level_id:
        return _fail("spec missing required `level_id`")
    deletes = spec.get("deletes") or []
    if not isinstance(deletes, list) or not deletes:
        return _fail("spec missing required `deletes` list")

    tx = LdtkTransaction(
        args.ldtk,
        in_place=args.in_place,
        output=args.output,
        backup=args.backup,
    )
    project = tx.project
    level = find_level(project, level_id)
    layer = find_ambition_layer(level)
    instances = layer.get("entityInstances", [])

    total_deleted = 0
    summaries: list[str] = []
    for entry in deletes:
        target = entry.get("target") or {}
        matched = select_for_delete(layer, target, args.all_matching)
        for ent in matched:
            iid = ent.get("iid", "<no-iid>")
            ident = ent.get("__identifier", "<unknown>")
            px = ent.get("px", ["?", "?"])
            instances.remove(ent)
            summaries.append(f"  - deleted {ident} (iid={iid}) at px={px}")
            total_deleted += 1

    layer["entityInstances"] = instances

    for line in summaries:
        print(line)
    print(
        f"deleted {total_deleted} entit{'y' if total_deleted == 1 else 'ies'} from '{level_id}'"
    )

    if summaries:
        tx.note_changed(summaries)
    target_path = tx.finish(
        noop_message="entity delete: no matching entities were deleted",
        write_message="wrote {path}",
    )

    if target_path is None or args.no_repair:
        return 0
    return run_repair_and_validate(target_path, args.schema)


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
