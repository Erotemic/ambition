# The Art dimension

**State:** BUILT (2026-11-03). Fifteen rooms, seven biomes, the cast, the
gate, the four quarters and the machines of the concept sheets are in the
game, and their art is published. What is at the end of this document is
ideas for later, not missing work.

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
| generator | `game/ambition_map_assets/ambition_content/generators/art_dimension/make_specs.py` |
| dialogue | `dialogue/sandbox/art.yarn` |
| encounters | `art_gallery_vigil`, `art_audition` (`data/encounters/goblin_encounter.ron`) |
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

## The four quarters

Jon (2026-11-02): "Expand it out: art is more than paintings. Music,
performance, fiction, teknis." Teknis is techne, the art of making: craft,
design and engineering done as an art. Each quarter is a room out in the
world, a building by a door in it, a biome, three creatures and its people.

```
 [ the amphitheatre      ][ the sketchbook cliffs ][ the resonant reach   ]-door->[ concert hall ]
   stage door v   gap v      ^ lift          gap v
 [ the story stacks ][ the painter's plaza ][ the sculpture garden ]
   door v                    drain v              well v
 [ library of endings ]   [ the inkfall caverns                      ][ the workshop deeps ]-door->[ loom hall ]
 [ backstage ]
```

The stacks, the amphitheatre, the cliffs and the plaza are a third loop:
up the lift, west along the cliffs to the amphitheatre, down its gap into
the stacks, and east to the plaza. The reach and the deeps are branches.

| quarter | rooms | biome | music | what it asks of you |
| --- | --- | --- | --- | --- |
| fiction | `art_story_stacks`, `art_library_of_endings` | `art_stacks` | `graphite_and_rain` | the Author writes you an ending: a choice in dialogue sets `art_ending_happy` or `art_ending_sad`, and that ending's door opens (the sad one pays more) |
| performance | `art_amphitheatre`, `art_backstage` | `art_stage` | `everything_is_fine_please_dance` | the audition: step onto the stage and the stage shuts (`art_audition`: stagehands, mimes, critics); when it is over, the curtain opens on the stage door |
| music | `art_resonant_reach`, `art_concert_hall` | `art_resonance` | `liszt_hungarian_rhapsody_no2_orchestra`, `..._piano` | the chord: three chimes on the staves of the reach (`art_chime_do`, `_mi`, `_sol`); each opens one bar of the hall's stage |
| teknis | `art_workshop_deeps`, `art_loom_hall` | `art_forge` | `something_worth_building`, `luddites_guild_loom_and_liberty` | the bridge builders: a pit of kiln fire; the crane's lever (`art_bridge_crane`) swings in a bridge (LockWalls gated by `not world.switch_on`) |

The stacks' signs do not agree with each other: the narrator is not
reliable.

| quarter | creatures | people (parodies, unless said) |
| --- | --- | --- |
| fiction | `bookworm`, `red_herring`, `paper_tiger` (all idioms) | Edgar Allan Crow (Poe, as a raven), Agatha Christly (Christie), the Author (original) |
| performance | `stagehand`, `critic`, `mime` | Marcel Mimeau (Marceau: speaks only in stage directions), Will Shakesbeard (Shakespeare: blank verse) |
| music | `discord_note`, `feedback_wail`, `war_drum` | Ludwig van Beathoven (Beethoven: very loud), Johann Sebastian Batch (Bach: fugues as processes) |
| teknis | `clay_golem`, `loom_spider`, `kiln_sprite` | Leonardo da Vinchy (da Vinci), Antoni Gaudy (Gaudi: nothing straight) |

The art of each quarter is in the renderer: `backgrounds/art_quarters.py`
(the four scenes), the terrain rows of the four biomes,
`targets/props/art_{stacks,resonance,stage,forge}_props.py`, a target for
each creature in `targets/characters/`, and a chargen spec for each person in
`data/townsfolk/`.

## The concept sheets' machines

The four things to use from the concept sheets are built:

- **The Curator's studio** (`art_curators_studio`), by a door in the plaza:
  canvases that nobody finished, a map of the quarters, and a storeroom
  behind a wall of scribble.
- **The eraser switches**: a lever drawn as a big eraser (`art_eraser`) rubs
  out a wall of pencil scribble (a LockWall drawn as `art_scribble_wall`,
  gated by `world.switch_on art_eraser_<place>`). One guards the cyan
  pigment at the end of the cliffs; one guards the studio's storeroom.
- **The crayon rail**: a crayon on its side (`art_crayon_rail`, a platform
  look) rides to and fro under the last page of the cliffs.
- **The brush seesaws**: brushes that rock on their middle
  (`art_brush_seesaw`, a swinging line plank) over two pools of ink in the
  caverns.

The art is `targets/props/art_studio_props.py` in the renderer.

## The Picture Palace

Jon (2026-11-03): "In the art dimension I want writers / directors with: T-71
to parody a T-70 Terminator (with Cameron / Hurd) and NDR-115 to parody a
NDR-114 Bicentennial Man. Need an Asimov parody. Also need MAL-9000 for
HAL-9000 parody, with Clarke and Kubrick."

A film studio (`art_picture_palace`) by a door at the east end of the
Amphitheatre. Its three sound stages each hold the people who made a machine
and the machine, which keeps going after they stop:

| stage | the makers | the machine |
| --- | --- | --- |
| The future war | James Camera-Ron (director), Gale Anne Herd (producer) | T-71, the heavy model before the famous one; it holds a jacket it cannot put on |
| The positronic man | Isaac Azimuth (the three laws) | NDR-115, a household android who becomes human one part at a time |
| The space odyssey | Arthur C. Clerk, Stanley Cubrick (and his cube, the monolith) | MAL-9000, a ship's computer in a wall panel |

MAL-9000 will not open the pod bay door (`art_pod_bay_door`, a LockWall). Pull
its memory module (the lever `art_mal_memory`) and the door opens on a chest
(`pulling_mals_memory_opens_the_pod_bay_door`). The machines talk and do not
fight. Their sheets are `targets/characters/art_t71.py`, `art_ndr115.py` and
`art_mal9000.py`; the props are `targets/props/art_picture_palace_props.py`.

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

## Ideas for later

- A score of its own for each quarter and for the caverns: each plays a cue
  of the music renderer that fits it now (the table of the quarters).
- More arts with their own idea, not a list of references: film, dance,
  architecture as a place of its own, games.
