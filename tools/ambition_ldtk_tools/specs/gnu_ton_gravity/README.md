# The GNU-ton gravity handle

The gravity works of the Science dimension (Jon, 2026-10-24: GNU-ton lives
in the Science dimension, with its orbital station): four rooms of gravity
and portal puzzles, and GNU-ton at the end. The world file is
`science.ldtk`, which holds the station's rooms too. The central hub's door `gnu_ton_handle_hub_door`
(main floor, between the Hall of Biomes and the gate stack) leads in, both
ways. The arena's door goes home to it.

`make_specs.py` writes each room's `.ron` spec and, with `--apply`, makes
the rooms and the hub's door. Change the script, not the specs. Run it from
`tools/ambition_ldtk_tools`.

## How each room is made

Each puzzle room teaches ONE idea where a mistake costs nothing, tests it
where a mistake costs a short walk, and then turns it with an idea of an
earlier room. The numbers (measured in the game): a jump rises 83 px, a jump
and a double jump 139 px; a run is 270 px/s; the body is 30 x 48. A
`GravityZone` pulls at the normal gravity in its `dir`, and the smaller zone
wins where two overlap. A portal pair opens as long as its shorter end. Two
ends with the same normal turn a body round (it comes out going the other
way along the face).

## The rooms

1. **`gnu_ton_orchard` — here an apple falls up (up fields).**
   - Teach: a column of up field over plain floor. Walk in: you fall onto
     the ceiling. Walk out: you fall back to the floor. A coin is in a nook
     of the ceiling that only a body falling up reaches.
   - Test: a chasm too wide to jump (192 deep, steps out on its near side).
     A column stands over it from bank to bank: walk the ceiling across.
   - Twist: the last column swings from bank to bank (256 each way,
     0.5 rad/s). On the ceiling you stay in it only if you walk with it.
2. **`gnu_ton_vector_well` — every wall is a floor (side fields).**
   - Teach: the floor of the well pulls right. Walk in, the east wall is
     the floor; walk up it. Over the field you fall back to the floor.
   - Test: a band across the well pulls left. Walk up the east wall into it,
     fall across the well, and walk up the west wall.
   - Twist: at the top of the west wall an up field (room 1) takes you to
     the vault. Along the vault, out of the field, you drop into the passage
     of the door on. That is right, left, up, down.
3. **`gnu_ton_conservation_hall` — two places made one (portals).**
   - Teach: a shelf too tall to jump. A portal in the floor before it has
     its other end on a post on top. Walk in, come out on top.
   - Test: the shelf is closed in by a wall to the vault. The only way on is
     off its edge, into a portal that is the whole floor of the basin. Its
     other end is in the next wall, over a pit with a hazard in it. What falls
     far comes out fast and flies over.
   - Twist: the door on is in a gallery closed in by solid wall. A portal in
     the hall's floor has its other end in the gallery's floor, under an up
     field (room 1). You fall in going down, come out going up, and go on
     falling up to the ceiling. Walk out of the field and drop to the door.
4. **`gnu_ton_inversion_gallery` — the whole room turns over (switches).**
   - Teach: the lever by the door (`SetGravityUp`) turns the room's gravity
     up. The lever on the ceiling over it (`SetGravityDown`) turns it back.
   - Test: the floor stops at a chasm with a hazard. The ceiling goes on.
     Turn the room over and walk the ceiling across.
   - Twist: the ceiling has a hole with a hazard in it, too wide to jump. The
     upside-down road steps down to a portal in the ceiling (room 3). Its
     other end is on a wall that hangs from the ceiling past the hole. Past
     that, a column of DOWN field (room 1, turned) is the one place in the
     room that falls down: onto the ledge of a shrine and GNU-ton's door.
5. **`gnu_ton_shoulders` — GNU-ton.** This is the hall's arena
   (`gnu_ton_arena`, `sandbox.ldtk`) as it is: the same floor, walls,
   perches, giant and rider, so the fight is the hall's fight. You come in at
   the top and the pad under you breaks. His placement is
   `gnu_ton_handle.gnu_ton` (the hall's copy is `hall.gnu_ton`). His reward
   and music are the boss's own. The way out is a door home in an alcove past
   the east wall. A `LockWall` (`gated_by: boss.cleared gnu_ton_handle.gnu_ton`)
   closes it until his defeat is recorded.

A fall into a hazard starts the room again from its door. The rooms are
made for a robot that runs and jumps. The robot can also fly, blink and
climb walls, and no room can take that away, so only the conservation
hall's shelf and gallery are closed by solid wall.

## Proof

`game/ambition_app/tests/the_gnu_ton_handle.rs` drives the robot through
each room with a player's inputs: the trees of the orchard, the swinging
tree, the four falls of the well, the hall's two portal pairs, the gallery's
levers and its crossing, and the door chain from the hub to the arena and
home. It also checks that GNU-ton is alive at his placement, that his punch
opens his portals in this room, and that his recorded defeat opens the way
home.

## Not done

- No switch turns a `GravityZone` or a portal on or off. The levers turn the
  room's ambient gravity only.
- No enemies in the puzzle rooms.
- The hall's GNU-ton arena is not yet a practice room (`practice`), as
  `docs/planning/game/bosses.md` asks for a hall boss that has a main-game
  placement.
