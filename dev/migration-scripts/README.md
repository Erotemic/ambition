# Migration scripts (archived one-shots)

This directory holds Python scripts that performed a single mechanical
data migration during the project's history and are not expected to be
run again. They live here (rather than in `tools/`) so future readers
can find the exact transformation that landed a given commit without
mistaking them for tools to invoke during normal development.

If you need to re-run one — say, against a fresh clone of an older
branch — the scripts are self-contained: they don't import from
`ambition_ldtk_tools`'s evolving CLI surface, so they should still work
years after they were written.

## Inventory

- `rename_npc_field.py` — Phase 2 of the character-catalog refactor
  (commit `bb51061`, 2026-05-24). Renamed `NpcSpawn.name` to
  `NpcSpawn.character_id` across every level in `sandbox.ldtk` and
  `intro.ldtk`. Display names were mapped to stable character_ids via
  the legacy table that previously lived in
  `presentation/character_sprites/assets.rs::npc_sprite_label`.

- `migrate_specs_to_ron.py` — Phase 4 of the character-catalog
  refactor (commit `60bfb6f`, 2026-05-24). Converted every YAML area
  spec under `tools/ambition_ldtk_tools/specs/` to RON in-place and
  deleted the originals. Round-trip-verified via the upstream
  `python-ron` parser before writing.

## Authoring guidance

When you write a new migration script, leave it in `tools/` while
it's being developed and tested. After the migration commit lands —
i.e. the database / file / asset has been mutated and the change has
shipped — move the script here, and add an entry above with: the
commit hash, the phase / context, and a one-line description of what
the script did.

The pattern is borrowed from `dev/benchmark-candidates/` (transferable
refactor mistakes) and `dev/journals/` (>1hr-to-diagnose bug stories):
keep the engineering memory close to the code without polluting the
active tool surface.
