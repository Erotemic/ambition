# The Noodly Temple: a level of keys and doors (started 2026-10-12)

Jon, 2026-10-12: "I want to build a level that has the FSM as the boss at the
end. It should be a puzzle style level that keys you have to find and doors to
unlock. This should be a fairly sophisticated level."

Jon, the same day, while the level was made: "We also want a much more noodly
biome." and "It might also be fun to have a pool noodle prop near the pools
that the character can pick up and use as a weapon."

## What it is

Seven rooms round one hall, from a door in the central hub ("to the Noodly
Temple"). The rooms are written by
`tools/ambition_ldtk_tools/specs/noodly_temple/make_specs.py`. Change the
script and run it with `--apply`; do not change the rooms in the map.

| Room | Behind | What it asks | What it gives |
| --- | --- | --- | --- |
| `noodly_narthex` | the hub door | nothing: Ramen-nujan says what the temple is | (the alms, behind a green door) |
| `noodly_nave` | open | the hall, with a shrine | a way to each other room |
| `noodly_scullery` | open | a fight of three waves that shuts the room | the Marinara Key (red) |
| `noodly_cistern` | a red door | three wheels and five gates | the Pesto Key (green) |
| `noodly_belfry` | a red door | three bells: a wall that breaks, the meatballs, a green door | the Parmesan Key (gold) |
| `noodly_reliquary` | a green door and a gold door | a second fight | the Colander Key |
| `noodly_sanctum` | the colander door | the Flying Spaghetti Monster | the end |

The player flies and blinks, so no height and no gap shuts a way. Each way
that is shut is shut by solid wall with one opening and a door in the opening.

The cistern is the puzzle of the level. Wheel I opens the low gate and shuts
the gate of the well and the first gate of the lock. The key is past the
lock, so the player must turn wheel I back and go the long way round (through
the gallery and down the well). The least number of turns is four.

The pools of the cistern are pits in a dry deck, and no gate is at a pool
(Jon, 2026-10-12): the game has no water that flows, so a gate that held water
back would open on water that stays where it is. The gates are dry gates that
a wheel moves, not sluices.

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
- A wall that opens while its room is live goes up into its lintel, and one
  that shuts comes down (`slide_lock_wall_visuals`). The picture is cut at the
  lintel; a grille gets shorter from its foot. The wall of the simulation
  opens and shuts at once. A wall that stands when a room is entered does not
  slide and makes no sound.
- A fight is an `EncounterTrigger`, a `LockWall` with no `gated_by`, and a
  `Switch` that names the fight (`target_encounter`): without the switch the
  fight starts again each time a body enters the trigger. A grille that the
  end of the fight opens asks `encounter.cleared <id>`. The waves are in
  `data/encounters/goblin_encounter.ron`.
- The sheets are targets of the sprite renderer
  (`targets/props/_temple_kit.py`), and `ambition_content::noodly_temple`
  registers them.

## The noodly biome

The rooms select the authored `"noodly"` theme. All of it is pasta.
The art is in the sprite renderer:

- The scene (`backgrounds/noodly.py`): the roof of the world is a colander,
  and the light is what comes through its holes. Hills of meatballs in steam,
  forks that stand like spires, columns of rigatoni with arches of macaroni,
  and noodles that hang between them. Near, great columns and curtains of
  noodles. The foreground is a few noodles that hang from above.
- The ground (`terrain/skins.py`, the row `noodly`): noodles in a press (the
  material `noodle`), marinara that runs down the edge in round drips with
  cheese and basil on it (the cap `sauce`), noodles that hang under each
  block (the underside `noodles`), and a ledge of lasagne (the one-way
  `lasagne`).
- On the ground (`terrain/decor.py`): meatballs, a nest of noodles, a pot on
  the boil, garlic, breadsticks, basil. The door is the door of a kitchen.
  The motes are steam.

Publish it with `python -m ambition_sprite2d_renderer.backgrounds draw
--out-dir <assets>/backgrounds/parallax_layers noodly`, the same for
`.terrain` to `<assets>/room_dressing`, then
`./scripts/regen/quality_variants.sh --backgrounds-only --target 'noodly*'`.

## The pool noodle

A held item, `pool_noodle`: a tube of foam, long, quick and weak (a swing
of 88 reach and 1 damage, with a knockback of 300). `Attack` over it picks it
up, and `Attack` swings it.

It is an item of a platform fighter (Jon, 2026-10-12: "It's like a smash
item (will eventually become a smash item)"). Its swing is the swing of the
item (`MeleeActionSpec::Batter`), not of the body that holds it: the move
names the clip `item_swing` (so it does not take the hit polygon of the
robot's blade), it draws no blade arc, and the noodle turns in the hand
through the swing (`HeldItemArtEntry::gripped_at`, `ItemSwingFact`,
`item_swing_angle`). Up, down and back are the same swing turned. Its sounds
are two cues the game has (`player.dash` in the air, `player.pogo` on a hit)
until Jon chooses its own from `untracked/sfx-candidates/pool_noodle/`. While
it is held it is the whole melee vocabulary of the holder, as each held
weapon is: the robot has no smash attack with it. Two lie in the cistern: one on the landing by the basin, one on the shelf
of the vault. It is a row of the held-item table
(`ambition_characters::brain::action_set`), a renderer target (`pool_noodle`)
and an art entry (`ambition_content::items::held_visuals`). A room places one
with a `GroundItem` (`held_item: "pool_noodle"`).

## The check

`python specs/noodly_temple/check_temple.py` (from `tools/ambition_ldtk_tools`)
walks a model of the level. It fails when the monster, a key or a room can
not be reached, and when the player can be in a place from which the door of
the hub can not be reached again. Run it after each change of the rooms.
`game/ambition_app/tests/the_noodly_temple.rs` asks the game what the model
takes as given. Its `a_player_walks_in_takes_the_four_keys_and_beats_the_monster`
plays the level from the hub to the monster's defeat with the player's own
inputs (the room graph and `common::walk::fight`): the wheels are turned in
the order the check prints, and no step moves the body or writes health.

## Open

- The hall of biomes has no room for `noodly`.
- The pool noodle does not bend when it is swung, and its swing has no
  trail. Its sounds are not its own yet. It is not a Smash item yet (no
  forward smash with it, no spawn in a match).
- The robot's body plays its blade pose while it swings the noodle.
- A door that the player can not open says nothing when he touches it. The
  writing by each door says what it wants.
- The keys the player has are not shown on the screen.
- The meatballs of the belfry are drawn as the spike ball of each
  `DamageVolume`.
- The key of a fight is in the room past its grille. The chest that the
  engine gives at the end of each fight has two hearts in it.
