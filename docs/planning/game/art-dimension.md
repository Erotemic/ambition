# The Art dimension

**State:** BUILT — first region. The rooms, the biome, the cast and the gate
are in the game. The open work is at the end of this document.

Jon, in the original brainstorm: "The art dimension is also seriously
underdeveloped, and its very important. It's how humans communicate with each
other. The best ideas can be useless if not communicated well. The art
dimension should have multiple components: visual, audio, conceptual. The
inhabitants of this dimension likely hate the gates the most."

The concept sheets (2026-11-01) are in the assets submodule:
`assets/assets/concept_art/misc/ChatGPT Image Nov 1, 2026, 06_22_0{4,5,6} PM-{1,2,3}.png`.
They show the four places, the props, and the cast below.

## What it is

A planet-like dimension (`world-layers-and-travel.md`) where art is the
material of the world. Paint runs in rivers, the cliffs are pages of a
sketchbook, statues are not finished, and ink is under all of it. Things
that people make here come alive.

The people of the dimension make things by hand. A gate brings strangers
who make things fast and do not sign them. That is why they hate the gates.
The Curator keeps the plaza's ring open because a closed door is also a
statement.

| | |
| --- | --- |
| world file | `art.ldtk` (`data/worlds.ron`: `world.art_ldtk`) |
| biome / theme | `art`; the cliffs are `art_sketchbook`, the caverns `art_inkfall` |
| gate address | `EASEL`, the ring of the Painter's Plaza (`data/gate_network.ron`) |
| generator | `tools/ambition_ldtk_tools/specs/art_dimension/make_specs.py` |
| dialogue | `dialogue/sandbox/art.yarn` |
| encounter | `art_gallery_vigil` (`data/encounters/goblin_encounter.ron`) |
| test | `game/ambition_app/tests/the_art_dimension.rs` |

There is no Rust content module. The rooms, the cast, the props and the
dialogue are data.

## The region

```
                  [ the sketchbook cliffs              ]
                     ^ palette lift          gap (drop) v
 ring > [ the painter's plaza ][ the sculpture garden ]-door->[ the unfinished gallery ]
               drain v (grate)            ink well v
        [ the inkfall caverns                          ]
```

The rooms out in the world join by side and floor exits on a flat map
(`specs/check_flat_map.py art.ldtk` reports no finding).

- **The plaza** (`art_painters_plaza`): the ring and its terminal at the
  west end, the Curator, Pablo Peckasso and Rob Boss, easels and paint
  pots. Not dangerous. The **palette lift** waits a hop over the floor; a
  rider who stands on it goes up through the roof into the cliffs. The
  **drain** in the floor has a grate (`art_drain_grate`) that stays shut
  until the lever under it, in the caverns, is pulled: a shortcut that
  opens from the far side.
- **The sketchbook cliffs** (`art_sketchbook_cliffs`): paper ledges up to
  the **cyan** pigment at the east end. Doodle imps in threes, origami
  birds. A gap in the last page drops into the garden (one way).
- **The sculpture garden** (`art_sculpture_garden`): two living statues, a
  brush knight, Frida Collage. A scaffold up the head that nobody finished
  goes to the **yellow** pigment. The ink well goes down to the caverns.
  The **colour gate** is three doors under one lintel; each door is a
  `LockWall` gated by `world.flag_set art_pigment_<colour>` and drawn as
  `art_colour_gate_<colour>`. Behind them is the door to the gallery.
- **The inkfall caverns** (`art_inkfall_caverns`): pools of ink that hurt,
  ink wisps, the **magenta** pigment on a shelf, the drain's lever, and
  ladders up the drain and up the well.
- **The unfinished gallery** (`art_gallery_unfinished`): a building. The
  vigil (three waves: doodle imps, ink wisps, a brush knight) shuts the
  room; when it is over, the grille of the store room opens on a chest.

The two loops: plaza, cliffs, garden, plaza; and plaza, garden, caverns,
plaza (up the drain once the lever is pulled).

The pigments are keys: a `PickupSpawn` with `flag:art_pigment_<colour>`,
taken once, drawn as `art_pigment_<colour>`.

## The cast

All of them are rows of `character_catalog.ron` and targets of the sprite
renderer (`targets/characters/`, on the shared kit `_art_cast_kit.py`). None
needs Rust.

| id | what it is | in play |
| --- | --- | --- |
| `doodle_imp` | a scribble with teeth | fast and weak, in threes |
| `ink_wisp` | a drop of living ink | floats, spits ink (a skirmisher) |
| `origami_bird` | a paper crane | a floating wanderer: no contact damage, not an enemy |
| `brush_knight` | armour that holds a big brush | a telegraphed sweep |
| `living_statue` | a marble head and shoulders that nobody finished | slow and heavy; it falls forward onto you |
| `npc_art_curator` | the keeper of the dimension | friendly; talks |

The painters are parodies, as Johannes Kepplar and Pierre de Fermette are
(chargen specs in the renderer's `data/townsfolk/`):

| id | parody of | where |
| --- | --- | --- |
| `npc_art_rob_boss` | Bob Ross: happy little mountains | the plaza, by the lift |
| `npc_art_pablo_peckasso` | Picasso, as a pigeon in a beret: he paints you from two sides | the plaza |
| `npc_art_frida_collage` | Frida Kahlo: her own face, from the scraps of other pictures | the garden |

Each of them, and the Curator, says why the people here hate the gates.

## The biome

The art is in the sprite renderer:

- the scenes (`backgrounds/art_dimension.py`): `art` (a sky of washed
  colour, gold frames that float in it, mesas that run with paint, crayons
  as tall as trees), `art_sketchbook` (a sky of paper, clouds and hills in
  pencil, pages that stand up as cliffs) and `art_inkfall` (a cave of
  violet stone where magenta and cyan ink falls and glows), with a
  foreground for `art`;
- the ground (`terrain/skins.py`): `art` is canvas with a cap of paint that
  drips over the edge and a palette for a one-way ledge; `art_sketchbook`
  is paper with a pencil edge and a ruler; `art_inkfall` is rock under a
  cap of ink;
- on the ground (`terrain/decor.py`): paint pots, brush jars, crayon
  stubs, crumpled paper, pencil cups, ink bottles; the motes are flecks of
  paint, specks of graphite, and glows in the ink.
- the props (`targets/props/art_props.py`): the pigments, the colour
  gate's doors, the drain grate, the palette lift, the easel, the giant
  crayon, the paint pots.

## Audio

The plaza, the cliffs, the garden and the gallery play
`artists_guild_chiaroscuro` (the Artists Guild theme, a 6/8 gallery waltz).
The caverns play `mages_guild_arcane_lanterns`.

## Open work

- A cue of its own for the caverns and the gallery (the music renderer).
- The sprites and backgrounds of the dimension are in the renderer and in
  `sprites.yaml`, but not yet published to the content store: publish
  them (`scripts/regen/sprites.sh --target <name>`, then the backgrounds).
- The other places of the concept sheets that are not built: the Curator's
  studio, the crayon rails, the brush seesaws, the eraser switches.
- The conceptual part of the dimension (literature, theatre): more rooms,
  each with its own idea, not a list of references.
