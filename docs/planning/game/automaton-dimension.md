# The Automaton dimension: clockwork and cells (started 2026-11-25)

**State:** BEGUN. Four rooms, three biomes, two new residents, Busy Beaver
redrawn as an automaton, one enemy, glider lifts, the gate, and the
Perfect Cellular Automaton's arena with its fight music are in the game.
What is under "Next" is not built.

Jon (2026-11-24) asked for the beginnings of the Automaton dimension, with
four concept images in the assets submodule, `assets/concept_art/misc/`:
`Automaton Dimension_ Clockwork Cityscape.png`, `Automaton Dimension_ Neon
Cellular Cosmos-2.png`, `Automaton Dimension_ Neon Life City-1.png` and
`Busy Beaver Automaton Concept Sheet.png`. They are a reference for the
look, not a layout.

Two kinds of automaton share the dimension, and the concept art keeps them
apart: the **clockwork** kind (gears, escapements, a water wheel, an
aqueduct with a train, a giant robot, automaton deer and birds, a cafe of
small robots) and the **cellular** kind (a grid in the dark where cells live
and die by a rule: gliders, blinkers, a Sierpinski triangle, a tower of
cubes in a ring). The Perfect Cellular Automaton stands in the art; this
is where its kind comes from. Busy Beaver is the third kind: the Turing
machine, the busy beaver that writes the most ones before it halts.

| | |
| --- | --- |
| world file | `automaton.ldtk` (`data/worlds.ron`: `world.automaton_ldtk`) |
| biomes | `automaton` (the clockwork Works), `automaton_life` (the Life field; also the arena's terrain), `automaton_tower` (inside the clock tower: the Tape Room); the arena's backdrop is a scene of its own, `automaton_arena` |
| gate address | `CELLS`, in the Works (`data/gate_network.ron`, posted `LEGAL`) |
| generator | `game/ambition_map_assets/ambition_content/generators/automaton/make_specs.py` |
| dialogue | `dialogue/sandbox/automaton.yarn`; the Hall's nodes in `hall.yarn` |
| art | renderer `backgrounds/automaton.py`, `targets/props/automaton_props.py`, `targets/characters/automaton_residents.py`, `targets/characters/life_spaceship.py`; Busy Beaver's redesign in `targets/characters/busy_beaver.py` |
| music | `busy_beaver` in every room (Jon, 2026-11-25: "busy beaver should be the main theme"); the PCA's fight is `glider_gun` (`dev/compose_glider_gun.py` in the music renderer) |

## The shape of it

```
        door
  [the Tape Room]
         |
  [ the Works ] ==== [ the Life field ] ==== [ the Cell-ular Arena ]
```

- **The Works** (`automaton_works`): the gate CELLS; the Escapement Cafe,
  its gear sign and its tables; the water wheel; Busy Beaver at the stair of
  blocks he is building (64 a step) up to a chest; the clock tower, whose
  door is the Tape Room's.
- **The Life field** (`automaton_life`), east: you come down off the Works'
  edge onto a floor of cells. Pillars of cubes stand up from it, their tops
  lit with live patterns. **Gliders** carry you: a glider is a moving
  platform that runs on a diagonal, its pattern turning through its four
  phases as it goes (a glider moves one cell diagonally every four
  generations). Neighbour Moore keeps the block, a still life. Spaceships
  fly the field. A Replicator starts above the east terrace, near the
  Sierpinski panel and chest. It splits into smaller copies when defeated.
- **The Tape Room** (`automaton_tape_room`), by the clock tower's door: the
  tape along the wall with its ones, the Head on its rail, the table of
  states, and the count. The inspection carriage runs between two raised
  landings. Steps connect both landings to the floor.
- **The Cell-ular Arena** (`automaton_arena`), east of the Life field, where
  the Perfect Cellular Automaton challenges people (Jon, 2026-11-25). You
  come in on a gallery at the terrace's height: the failure log (a row of
  cells per challenger, each ending in a red cell; the bottom row lit and
  empty) and the Imperfect Cellular Automaton, who heckles the log. Steps
  go down to the ground; the ring stands one jump high on it, its face lit
  with the four generations of rule 150, a post of cubes at each corner.
  The PCA waits in the middle (dialogue `automaton_pca`: "Challenge it"
  plays `glider_gun` and starts the fight). Behind: stands of cubes whose
  crowd is lit cells, light pylons, and a screen on chains showing Gosper's
  glider gun with the two gliders it has fired by generation 120.

## The people and the things that fight

| Id | Name | Where | Note |
| --- | --- | --- | --- |
| `npc_busy_beaver` | Busy Beaver | the Works | Redrawn to his automaton sheet: beacon, cell grid, panel tail. Still the foreman of Math Town's dam; here he builds. |
| `npc_automaton_neighbour_moore` | Neighbour Moore | the Life field | Knows his eight neighbours (the Moore neighbourhood) and minds the block, which never changes. |
| `npc_automaton_head` | The Head | the Tape Room | The read/write head of the Busy Beaver's machine: reads, writes, moves one cell, changes state. That is the whole of its education. |
| `perfect_cellular_automaton` | Perfect Cell-ular Automaton | the Cell-ular Arena | Built the arena on the field its kind comes from; challenge it in the ring. Also still in the hub basement's Perfect Cell-ular Arena and the Hall. |
| `imperfect_cellular_automaton` | Imperfect Cellular Automaton | the Cell-ular Arena | On the gallery by the failure log, in which it is the third and fifth rows. Cheers for the challengers. |
| `life_spaceship` | Spaceship | the Life field | The lightweight spaceship of the Game of Life: it crosses the field at half the speed of light (c/2), a cell every two generations. |

## Facts the rooms state

- BB(5) = 47,176,870 steps (the busy beaver for five states, proved by the
  bbchallenge collaboration in 2024); the Tape Room's count. The table on
  its wall is the champion's (Marxen and Buntrock): A0 1RB, A1 1LC, B0 1RC,
  B1 1RB, C0 1RD, C1 0LE, D0 1LA, D1 1LD, E0 1RH (halt), E1 0LA; it halts
  with 4,098 ones.
- A glider moves one cell diagonally every four generations; a lightweight
  spaceship moves two cells every four (c/2).
- Rule 90 draws a Sierpinski triangle from one live cell.
- Rule 150 (each cell the XOR of itself and its two neighbours) on a ring of
  sixteen cells, from two cells eight apart, comes back every four
  generations (the ring's face; the riff of `glider_gun`).
- Gosper's glider gun (1970, the year Conway's rule was published) fires a
  glider every thirty generations.
- In Life a change spreads at most one cell a generation: the rule's speed
  of light, c.
- Conway's Life: a live cell with two or three live neighbours lives; a dead
  cell with exactly three is born.

## Next

- A field whose cells really live: a room-sized Life board stepped by the
  rule, its live cells the platforms (it needs a solid the room can switch).
- Gears as platforms (a turning gear's teeth) and the clock tower's
  insides.
- The clockwork deer and birds of the concept art as wildlife; the giant
  robot as a place.
- The Perfect Cellular Automaton's link to the field: where its kind came
  from.
- The challenge starts `glider_gun` (`<<music "glider_gun">>`); whether
  `busy_beaver` comes back after the fight has not been checked.
- A boss practice room and a reward for beating the PCA in its arena.
