#!/usr/bin/env python3
"""One-shot migration: rename `NpcSpawn.name` to `NpcSpawn.character_id`
and map instance values via the legacy display-name → character_id
table that lived in
`crates/ambition_sandbox/src/presentation/character_sprites/assets.rs::npc_sprite_label`.

Part of Phase 2 of the character-catalog refactor (see
`TODO-character-catalog-and-hall.md`). After this script runs, LDtk
NpcSpawn entities carry the same stable id (`npc_kernel_guide`) that
the RON character catalog uses as its key. The legacy display-name
field disappears entirely; the catalog's `display_name` provides the
human-facing label.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools.edit.rename_npc_field \\
    crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \\
    crates/ambition_sandbox/assets/ambition/worlds/intro.ldtk
```

The script is idempotent — running it again on an already-migrated
file is a no-op (the `name` field def is gone, so nothing matches).

After running, do `repair --in-place` + `validate` on each file.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# Mirror of
# crates/ambition_sandbox/src/presentation/character_sprites/assets.rs::npc_sprite_label
# This is the legacy display-name → character_id table.
LEGACY_NAME_TO_CHARACTER_ID = {
    "General": "npc_general",
    "Fretjaw, Cantina Chieftain": "npc_goblin_cantina_chieftain",
    "Captain Pulse": "npc_pulse_voyager_captain",
    "Chadwick Disruptor III": "npc_tech_bro_disruptor",
    "Pirate Admiral": "npc_pirate_admiral",
    "Pirate Raider": "npc_pirate_raider",
    "Pirate Quartermaster": "npc_pirate_quartermaster",
    "Pirate Lookout": "npc_pirate_lookout",
    "Pirate Navigator": "npc_pirate_navigator",
    "Burning Flying Shark": "npc_burning_flying_shark",
    "Shadow Oni Leader": "npc_ninja_shadow_oni_leader",
    "Shadow Duelist": "npc_ninja_shadow_duelist",
    "Puppy Slug": "npc_puppy_slug",
    "Broadside Bess": "npc_pirate_heavy_broadside_bess",
    "Iron Mary": "npc_pirate_heavy_iron_mary",
    "Salt Annet": "npc_pirate_heavy_salt_annet",
    "Architect NPC": "npc_architect",
    "Kernel Guide NPC": "npc_kernel_guide",
    "Vault Keeper NPC": "npc_vault_keeper",
    "Merchant Prototype NPC": "npc_merchant_prototype",
    # Hub story-content NPCs (intro / dialogues). These names appear
    # in intro.ldtk's NpcSpawn instances. The character catalog will
    # learn each id in Phase 3 — for now we still need a stable id
    # so the rename round-trips cleanly. Any id not yet in the catalog
    # will be flagged by the runtime validator and we'll add a catalog
    # entry to close the gap.
    "Alice": "npc_alice",
    "Bob": "npc_bob",
    "Eve": "npc_eve",
    "Craig": "npc_craig",
    "Erdish": "npc_erdish",
    "Oiler": "npc_oiler",
    "Creator": "npc_creator",
    "Raid Enforcer": "npc_raid_enforcer",
    "Sandbag": "npc_sandbag",
}

# When the script can't find a mapping for an existing instance, it
# falls back to slugifying the display name. This is intentionally
# permissive — the migration prefers to never silently drop
# information. The runtime validator will flag any character_id that
# isn't in the catalog, which is the cue to either:
#   1) Add the missing legacy mapping above, OR
#   2) Add the new id to character_catalog.ron.
def slugify(name: str) -> str:
    s = name.lower()
    out = []
    last_was_space = False
    for ch in s:
        if ch.isalnum():
            out.append(ch)
            last_was_space = False
        elif not last_was_space:
            out.append("_")
            last_was_space = True
    slug = "".join(out).strip("_")
    return f"npc_{slug}" if slug else "npc_unregistered"


def migrate_file(path: Path) -> dict:
    """Edit the LDtk JSON in place; return a summary dict."""
    with path.open() as f:
        data = json.load(f)

    summary = {
        "path": str(path),
        "def_renamed": False,
        "instances_renamed": 0,
        "value_mappings": {},
        "unmapped_names": [],
    }

    # 1) Rename the field def.
    for entity_def in data.get("defs", {}).get("entities", []):
        if entity_def.get("identifier") != "NpcSpawn":
            continue
        for field_def in entity_def.get("fieldDefs", []):
            if field_def.get("identifier") == "name":
                field_def["identifier"] = "character_id"
                field_def["doc"] = (
                    "Stable character identifier; key into "
                    "`assets/data/character_catalog.ron`. "
                    "Replaced the legacy `name` field in Phase 2 "
                    "of the character-catalog refactor."
                )
                default = field_def.get("defaultOverride") or {}
                params = default.get("params") or []
                # Reset the default to a deliberately-broken id so the
                # runtime validator panics on a fresh placeholder rather
                # than silently spawning an unmapped NPC.
                if params:
                    params[0] = "npc_unregistered"
                summary["def_renamed"] = True
                break

    # 2) Rewrite every instance.
    for level in data.get("levels", []):
        for layer in level.get("layerInstances", []) or []:
            for inst in layer.get("entityInstances", []) or []:
                if inst.get("__identifier") != "NpcSpawn":
                    continue
                for fi in inst.get("fieldInstances", []) or []:
                    if fi.get("__identifier") != "name":
                        continue
                    old_value = fi.get("__value")
                    new_id = LEGACY_NAME_TO_CHARACTER_ID.get(old_value)
                    if new_id is None and old_value:
                        new_id = slugify(old_value)
                        summary["unmapped_names"].append(old_value)
                    if new_id is None:
                        new_id = "npc_unregistered"
                    fi["__identifier"] = "character_id"
                    fi["__value"] = new_id
                    summary["instances_renamed"] += 1
                    summary["value_mappings"][old_value] = new_id

    with path.open("w") as f:
        json.dump(data, f, indent="\t", ensure_ascii=False)
        f.write("\n")
    return summary


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ldtk_files", nargs="+", type=Path)
    args = parser.parse_args(argv)

    any_error = False
    for path in args.ldtk_files:
        if not path.exists():
            print(f"[error] file not found: {path}", file=sys.stderr)
            any_error = True
            continue
        summary = migrate_file(path)
        print(f"== {path} ==")
        print(f"  def renamed:       {summary['def_renamed']}")
        print(f"  instances renamed: {summary['instances_renamed']}")
        if summary["value_mappings"]:
            print(f"  value mappings:")
            for old, new in sorted(set(summary["value_mappings"].items())):
                print(f"    {old!r} -> {new!r}")
        if summary["unmapped_names"]:
            print(f"  [warn] unmapped names slugified:")
            for name in sorted(set(summary["unmapped_names"])):
                print(f"    {name!r}")

    return 1 if any_error else 0


if __name__ == "__main__":
    sys.exit(main())
