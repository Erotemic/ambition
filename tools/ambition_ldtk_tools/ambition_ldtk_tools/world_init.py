#!/usr/bin/env python3
"""Scaffold a new Ambition LDtk world file by cloning sandbox.ldtk's defs.

Ambition's runtime currently loads `sandbox.ldtk` as the single source of
truth for entity/layer/field definitions (uid-keyed). Authoring a second
LDtk source file for narrative content (e.g. the intro sequence, future
real-game-map zones) requires that the new file's defs match sandbox.ldtk
by uid — otherwise the runtime merge would have to remap every entity
instance's `defUid` and every layer instance's `layerDefUid`.

This subcommand:

- reads sandbox.ldtk's `defs` and project metadata,
- writes a new `.ldtk` file with the same defs but no levels,
- offsets the new file's `nextUid` by a large buffer so that subsequent
  `area create --ldtk <new>` calls mint level / entity iids that cannot
  collide with sandbox.ldtk's iids when both files are merged in memory
  at load time.

The resulting file is a valid standalone LDtk project (the LDtk GUI will
open it). Run `area create --ldtk <new>.ldtk <spec.yaml>` to add levels.

Usage:

    python -m ambition_ldtk_tools world init \\
        crates/ambition_content/assets/worlds/intro.ldtk \\
        --identifier ambition-intro-world

The optional `--source` flag points at a different defs donor (defaults
to sandbox.ldtk). The optional `--uid-offset` controls the buffer applied
to nextUid (default 100000 — sandbox.ldtk currently has nextUid ~4346
and grows slowly, so 100000 gives several decades of headroom).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

DEFAULT_SOURCE = Path("crates/ambition_content/assets/worlds/sandbox.ldtk")
DEFAULT_UID_OFFSET = 100_000


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        prog="ambition_ldtk_tools world init",
        description=__doc__.splitlines()[0],
    )
    ap.add_argument(
        "target",
        type=Path,
        help="Path to the new .ldtk file to scaffold (must not exist)",
    )
    ap.add_argument(
        "--source",
        type=Path,
        default=DEFAULT_SOURCE,
        help=f"Defs donor LDtk file (default: {DEFAULT_SOURCE})",
    )
    ap.add_argument(
        "--identifier",
        type=str,
        default=None,
        help=(
            "Project iid for the new world (default: derived from target "
            "filename, e.g. intro.ldtk → 'ambition-intro-world')"
        ),
    )
    ap.add_argument(
        "--uid-offset",
        type=int,
        default=DEFAULT_UID_OFFSET,
        help=(
            "Bump the new file's nextUid by this much above the source's "
            f"nextUid so iids don't collide on merge (default {DEFAULT_UID_OFFSET})"
        ),
    )
    ap.add_argument(
        "--force",
        action="store_true",
        help="Overwrite the target file if it already exists",
    )
    args = ap.parse_args(argv)

    target: Path = args.target
    source: Path = args.source

    if target.exists() and not args.force:
        return _fail(f"target {target} already exists; pass --force to overwrite")
    if not source.exists():
        return _fail(f"source defs file {source} not found")

    source_project = json.loads(source.read_text())

    iid = args.identifier or _default_world_iid(target)
    source_next_uid = int(source_project.get("nextUid", 1))
    new_next_uid = source_next_uid + int(args.uid_offset)

    # Clone the project shell. Top-level fields are mostly editor metadata —
    # we copy verbatim and then override the few that must differ from the
    # source (iid, levels, toc, nextUid).
    new_project = {
        "__header__": dict(source_project.get("__header__", {})),
        "iid": iid,
        "jsonVersion": source_project.get("jsonVersion", "1.5.3"),
        "appBuildId": source_project.get("appBuildId", 473703),
        "nextUid": new_next_uid,
        "identifierStyle": source_project.get("identifierStyle", "Free"),
        "toc": [],
        "worldLayout": source_project.get("worldLayout", "Free"),
        "worldGridWidth": source_project.get("worldGridWidth", 256),
        "worldGridHeight": source_project.get("worldGridHeight", 256),
        "defaultLevelWidth": source_project.get("defaultLevelWidth", 1900),
        "defaultLevelHeight": source_project.get("defaultLevelHeight", 1024),
        "defaultPivotX": source_project.get("defaultPivotX", 0),
        "defaultPivotY": source_project.get("defaultPivotY", 0),
        "defaultGridSize": source_project.get("defaultGridSize", 16),
        "defaultEntityWidth": source_project.get("defaultEntityWidth", 32),
        "defaultEntityHeight": source_project.get("defaultEntityHeight", 32),
        "bgColor": source_project.get("bgColor", "#20242F"),
        "defaultLevelBgColor": source_project.get("defaultLevelBgColor", "#20242F"),
        "minifyJson": False,
        "externalLevels": False,
        "exportTiled": False,
        "simplifiedExport": False,
        "imageExportMode": source_project.get("imageExportMode", "None"),
        "exportLevelBg": True,
        "pngFilePattern": None,
        "backupOnSave": False,
        "backupLimit": 10,
        "backupRelPath": None,
        "levelNamePattern": source_project.get("levelNamePattern", "Level_%idx"),
        "tutorialDesc": None,
        "customCommands": [],
        "flags": list(source_project.get("flags", ["IgnoreBackupSuggest"])),
        # `defs` is copied verbatim so entity/layer/field uids match the
        # source. The runtime merge relies on shared defs to avoid uid
        # remapping; never edit defs on the new file in isolation —
        # always re-run `world init` after the source's defs change.
        "defs": json.loads(json.dumps(source_project.get("defs", {}))),
        # `worlds` is the modern LDtk multi-world container (unused
        # by Ambition today). Mirror the source's shape if present
        # so the LDtk GUI doesn't complain.
        "worlds": list(source_project.get("worlds", [])),
        "levels": [],
        # Tilesets / enums are part of defs in modern LDtk; not
        # required at top level. Mirror only if the source has them
        # at the top level (very old LDtk projects do).
    }

    # Some sources have additional opaque top-level keys (e.g.
    # `pngFilePattern`, schema-only fields). Copy any key we haven't
    # already populated so the new file round-trips cleanly.
    for key, value in source_project.items():
        if key in new_project:
            continue
        if key == "levels":
            continue
        new_project[key] = json.loads(json.dumps(value))

    # Write via the editor-style serializer so the LDtk GUI can open
    # the result without re-saving. Falls back to `json.dumps` if the
    # serializer isn't importable (the in-place tool is colocated, so
    # this branch is just a guardrail).
    target.parent.mkdir(parents=True, exist_ok=True)
    try:
        from ambition_ldtk_tools.editor_format import dump_editor_style

        target.write_text(dump_editor_style(new_project))
    except ImportError:
        target.write_text(json.dumps(new_project, indent="\t"))

    print(
        f"wrote {target} (iid={iid}, defs from {source}, "
        f"nextUid={new_next_uid} = {source_next_uid} + {args.uid_offset})"
    )
    print(
        f"next: author levels via "
        f"`python -m ambition_ldtk_tools area create <spec.yaml> --ldtk {target}`"
    )
    return 0


def _default_world_iid(target: Path) -> str:
    # intro.ldtk → ambition-intro-world; town.ldtk → ambition-town-world.
    stem = target.stem.lower().replace("_", "-").strip("-") or "extra"
    return f"ambition-{stem}-world"


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
