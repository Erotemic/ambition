#!/usr/bin/env python3
"""One-shot migration: add the biome metadata seam to LDtk level fields.

Adds four optional `levelFields` to `sandbox.ldtk` so levels can declare
high-level intent without hardcoding every room in Rust:

  - biome           — broad biome label (`hub`, `basement`, `cave`, ...)
  - music_track     — `MusicTrack.id` from `sandbox.ron`'s music_tracks
  - ambient_profile — ambient sfx / particle profile id (future)
  - visual_theme    — palette / shader-variant id (future)

The fields are *optional*. Existing levels keep working untouched
because LDtk does not require a level-field instance for fields whose
definition exists in the project; the validator and the runtime read
them only when present.

Idempotent: running twice does nothing. Existing field defs with the
same identifier are left in place.

Run:
    python tools/add_biome_level_fields.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# The shape mirrors LDtk 1.5.3's `FieldDef` JSON keys so the editor and
# the strict validator both accept it. Most fields are constant; per-field
# overrides come from the FIELDS table below.
def make_string_level_field(identifier: str, uid: int, doc: str) -> dict:
    return {
        "identifier": identifier,
        "doc": doc,
        "__type": "String",
        "uid": uid,
        "type": "F_String",
        "isArray": False,
        "canBeNull": True,
        "arrayMinLength": None,
        "arrayMaxLength": None,
        "editorDisplayMode": "NameAndValue",
        "editorDisplayScale": 1,
        "editorDisplayPos": "Above",
        "editorLinkStyle": "ZigZag",
        "editorDisplayColor": None,
        "editorAlwaysShow": False,
        "editorShowInWorld": False,
        "editorCutLongValues": True,
        "editorTextSuffix": None,
        "editorTextPrefix": None,
        "useForSmartColor": False,
        "exportToToc": False,
        "searchable": True,
        "min": None,
        "max": None,
        "regex": None,
        "acceptFileTypes": None,
        "defaultOverride": None,
        "textLanguageMode": None,
        "symmetricalRef": False,
        "autoChainRef": False,
        "allowOutOfLevelRef": False,
        "allowedRefs": "Any",
        "allowedRefsEntityUid": None,
        "allowedRefTags": [],
        "tilesetUid": None,
    }


# (identifier, doc) pairs. The order is the order they're added.
FIELDS = [
    (
        "biome",
        "Optional biome label, e.g. 'hub', 'basement', 'cave', 'water', 'mob_arena'. Used by ambient + audio selection systems.",
    ),
    (
        "music_track",
        "Optional MusicTrack.id from sandbox.ron music_tracks. Played when the player enters this level's active area.",
    ),
    (
        "ambient_profile",
        "Optional ambient sfx / particle profile id. Layered on top of music_track.",
    ),
    (
        "visual_theme",
        "Optional palette / shader-variant id. Renderer reads this when rebuilding room visuals.",
    ),
]


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "ldtk",
        type=Path,
        help="Target LDtk file (default: sandbox.ldtk)",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Only check whether all biome fields are present; exit nonzero if any missing.",
    )
    args = parser.parse_args(argv)

    project = json.loads(args.ldtk.read_text())
    defs = project.setdefault("defs", {})
    level_fields: list = defs.setdefault("levelFields", [])
    existing = {f["identifier"] for f in level_fields}

    missing = [identifier for identifier, _ in FIELDS if identifier not in existing]
    if args.check:
        if missing:
            print(f"missing biome level fields: {missing}", file=sys.stderr)
            return 1
        print("all biome level fields present")
        return 0

    if not missing:
        print(f"all biome level fields already present in {args.ldtk}; no change")
        return 0

    next_uid = int(project.get("nextUid", 1))
    for identifier, doc in FIELDS:
        if identifier in existing:
            continue
        field = make_string_level_field(identifier, next_uid, doc)
        level_fields.append(field)
        print(f"added levelField {identifier!r} (uid {next_uid})")
        next_uid += 1
    project["nextUid"] = next_uid

    args.ldtk.write_text(json.dumps(project, indent=2) + "\n")
    print(f"wrote {args.ldtk}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
