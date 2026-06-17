#!/usr/bin/env python3
"""One-shot: set `music_track: tech_bros_disruption` for intro
lab/underground rooms in `intro.ldtk`.

The intro slice rooms are mostly Lab + Cave (underground) biomes,
and we want the tech-bro corporate-dystopia track to play across
all of them so the slice has a unified sonic identity instead of
falling back to the lofi default. Each level's `fieldInstances`
already has a `music_track` field def (uid 4173); some levels
explicitly set the value, most omit it and fall back to the
SandboxDataSpec default.

This script:

1. Walks every level in intro.ldtk.
2. If the level is in the `TARGETED_LEVELS` list AND its
   `music_track` field is either missing or holds the default
   (empty / null / lofi), sets it to `tech_bros_disruption`.
3. Skips levels that already carry a different explicit track
   (e.g. `pirate_sky_arena` keeps `pirates_guild_black_flag_jig`).

Idempotent: running twice is a no-op.

Run with:

    python3 scripts/migrate_intro_music_tracks.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
LDTK_PATH = REPO_ROOT / "crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk"

# Intro slice levels that should play the tech_bros_disruption track.
# `pirate_sky_arena` is intentionally NOT in this list — it's a sky
# pirate scene and already routes to pirates_guild_black_flag_jig.
TARGETED_LEVELS = (
    "intro_wake_room",
    "intro_raid_corridor",
    "intro_escape_shaft",
    "combat_calibration_lab",
    "gate_stack_lower",
    "first_system_boss",
    "alice_relay",
    "bob_relay",
    "drain_alley",
    "under_town_pipes",
)

DESIRED_TRACK = "tech_bros_disruption"

# Field def uid for `music_track` on intro levels (read from
# defs.levelFields[].uid where identifier == "music_track").
MUSIC_TRACK_FIELD_DEF_UID = 4173


def find_field_def_uid(project: dict, identifier: str) -> int:
    for f in project.get("defs", {}).get("levelFields", []):
        if f.get("identifier") == identifier:
            return int(f["uid"])
    raise SystemExit(
        f"level field def '{identifier}' not found in project; "
        f"either the schema changed or this script needs updating."
    )


def make_field_instance(value: str, uid: int) -> dict:
    return {
        "__identifier": "music_track",
        "__type": "String",
        "__value": value,
        "__tile": None,
        "defUid": uid,
        "realEditorValues": [
            {"id": "V_String", "params": [value]},
        ],
    }


def migrate(project: dict) -> tuple[int, int]:
    """Return (levels_set, levels_skipped_explicit)."""
    uid = find_field_def_uid(project, "music_track")
    set_count = 0
    skip_explicit = 0
    for level in project.get("levels", []):
        ident = level.get("identifier")
        if ident not in TARGETED_LEVELS:
            continue
        fields = level.setdefault("fieldInstances", [])
        existing = None
        for f in fields:
            if f.get("__identifier") == "music_track":
                existing = f
                break
        if existing is not None:
            cur = existing.get("__value")
            if cur and cur != DESIRED_TRACK:
                # Level already has a deliberate non-default override.
                # Preserve it rather than stomp.
                skip_explicit += 1
                continue
            existing["__value"] = DESIRED_TRACK
            existing["realEditorValues"] = [
                {"id": "V_String", "params": [DESIRED_TRACK]},
            ]
        else:
            fields.append(make_field_instance(DESIRED_TRACK, uid))
        set_count += 1
    return set_count, skip_explicit


def main() -> int:
    project = json.loads(LDTK_PATH.read_text())
    set_count, skipped = migrate(project)
    LDTK_PATH.write_text(json.dumps(project, indent="\t"))
    print(
        f"set music_track={DESIRED_TRACK!r} on {set_count} level(s); "
        f"skipped {skipped} with a different explicit override."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
