# The Noodly Temple: a level of keys and doors (started 2026-10-12)

Jon, 2026-10-12: "I want to build a level that has the FSM as the boss at the
end. It should be a puzzle style level that keys you have to find and doors to
unlock. This should be a fairly sophisticated level."

## What it is

Seven rooms round one hall, from a door in the central hub ("to the Noodly
Temple"). The rooms are written by
`tools/ambition_ldtk_tools/specs/noodly_temple/make_specs.py`. Change the
script and run it with `--apply`; do not change the rooms in the map.

| Room | Behind | What it asks | What it gives |
| --- | --- | --- | --- |
| `noodly_narthex` | the hub door | nothing: the sexton says what the temple is | (the alms, behind a green door) |
| `noodly_nave` | open | the hall, with a shrine | a way to each other room |
| `noodly_scullery` | open | a fight of three waves that shuts the room | the Marinara Key (red) |
| `noodly_cistern` | a red door | three wheels and five sluices | the Pesto Key (green) |
| `noodly_belfry` | a red door | three bells: a wall that breaks, the meatballs, a green door | the Parmesan Key (gold) |
| `noodly_reliquary` | a green door and a gold door | a second fight | the Colander Key |
| `noodly_sanctum` | the colander door | the Flying Spaghetti Monster | the end |

The player flies and blinks, so no height and no gap shuts a way. Each way
that is shut is shut by solid wall with one opening and a door in the opening.

The cistern is the puzzle of the level. Wheel I opens the low sluice and shuts
the sluice of the well and the first sluice of the lock. The key is past the
lock, so the player must turn wheel I back and go the long way round (through
the gallery and down the well). The least number of turns is four.

The belfry can not be finished before the cistern: its third bell is behind a
green door. Each key but the last opens a second door with a reward behind it
(the alms, the larder, the cell of the cistern).

## How a key and a door are authored

- A key is a `PickupSpawn`: `kind: "flag:<flag>"`, `respawn: "Never"`,
  `sprite: "<sheet>"`. The player reads "picked up <name>".
- A door is a `LockWall`: `gated_by: "<flag>"` and `look: "<sheet>"`. It
  stands until the flag is set, and the sheet draws it. A sheet that is a
  column that tiles (`temple_gate_iron`) fills a wall of any height.
- A gate that answers a lever asks `world.switch_on <switch id>`. The first
  word `not` turns a gate round: it stands while its condition is true.
- A fight is an `EncounterTrigger`, a `LockWall` with no `gated_by`, and a
  `Switch` that names the fight (`target_encounter`): without the switch the
  fight starts again each time a body enters the trigger. A grille that the
  end of the fight opens asks `encounter.cleared <id>`. The waves are in
  `data/encounters/goblin_encounter.ron`.
- The sheets are targets of the sprite renderer
  (`targets/props/_temple_kit.py`), and `ambition_content::noodly_temple`
  registers them.

## The check

`python specs/noodly_temple/check_temple.py` (from `tools/ambition_ldtk_tools`)
walks a model of the level. It fails when the monster, a key or a room can
not be reached, and when the player can be in a place from which the door of
the hub can not be reached again. Run it after each change of the rooms.
`game/ambition_app/tests/the_noodly_temple.rs` asks the game what the model
takes as given.

## Open

- The rooms are in the `boss` biome with no art of their own. A temple biome
  (a scene, a skin, a door) is not made.
- A door that the player can not open says nothing when he touches it. The
  writing by each door says what it wants.
- The keys the player has are not shown on the screen.
- The meatballs of the belfry are drawn as the spike ball of each
  `DamageVolume`.
- The key of a fight is in the room past its grille. The chest that the
  engine gives at the end of each fight has two hearts in it.
