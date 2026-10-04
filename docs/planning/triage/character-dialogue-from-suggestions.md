# Interact dialogue should read the same IR the barks already do

**Status:** shelved 2026-07-26. Design settled; nothing built.

## Finding

A generated character speaks its own lines through ambient barks but not
through *interact*:

| channel | reads | result |
|---|---|---|
| ambient bark | catalog `fallback_dialogue` | the character's own line |
| interact conversation | the Yarn node named by the LDtk `dialogue_id` | the `generic_npc` placeholder |

`npc_dialogue_request`
(`crates/ambition_platformer2d_actor_monolith/src/features/npcs.rs`) reads only
the LDtk `dialogue_id`, treats a blank value as absent, and falls through to
`generic_npc`. That placeholder is correct current behaviour, not a bug.

The Hall is covered by authoring instead: catalog rows carry a
`hall_dialogue_id`, and `known_dialogue_ids`
(`game/ambition_content/src/dialogue/yarn.rs`) adds those ids to the validator's
accepted set. The residual is small, and each item is one line of authoring:

- Some catalog characters have `hall_dialogue_id: None`.
- A few `NpcSpawn` placements have no `dialogue_id`. The fix is a commit in the
  `game/ambition_map_assets` submodule.
- Few rows declare a `fallback_dialogue`.

Re-scope against these counts before you build a generator.

## Decision (Jon, 2026-07-26)

Generate one conversation per character from `fallback_dialogue`. A
hand-authored node with the same title overrides it by existing, which is the
rule the bark fallback already follows. Do not add a second, non-Yarn dialogue
path.

Shape, if built:

- A regen script writes committed Yarn text (like `music_registry.ron`) to one
  generated `.yarn` file, one node `character_<id>` per catalog character, and
  skips any title that the authored set already defines.
- Every character gets a node (the generic line when it has no suggestions), so
  the runtime needs no "does it exist" branch.
- A generated node is one line from the bark pool and a Close. No choices, no
  state.
- `npc_dialogue_request` routes a blank or absent `dialogue_id` to
  `character_<character_id>` instead of `generic_npc`.
- Regen works on a fresh clone. The generator reads the same `CharacterNotes`
  as `tools/ambition_ldtk_tools/ambition_ldtk_tools/character_notes.py`.

## Evidence command

```bash
grep -c "hall_dialogue_id: None" game/ambition_content/assets/data/character_catalog.ron
grep -c "fallback_dialogue" game/ambition_content/assets/data/character_catalog.ron
rg -n "generic_npc" crates/ambition_platformer2d_actor_monolith/src/features/npcs.rs
```

## Owner

None.

Q85 (2026-10-04, [`../maintainer-decisions.md`](../maintainer-decisions.md)):
Hall actors may lack dialogue for now; Hall dialogue is expected future content
work and does not block the Hall. This note stays the place for that work.

## Trigger to promote

The cast grows faster than hand-authored Hall dialogue, or a playtest finds
generic placeholder conversations on characters that players meet.
