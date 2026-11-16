# Hobford polish: getting the intro right (started 2026-11-14)

The top item of the queue (Jon, 2026-11-14): "Getting the intro right should
be considered the top task in the queue." Polish is the primary task, not a
step on the way to the planning backlog. Go back to architecture work only
when there is nothing left that would make Hobford richer or more
believable. Jon expects to give feedback on each pass; each pass does the
best it can and reports with screenshots.

The level itself is described in [level 1](level-1.md); the rules this page
applies are in the [level design guide](../../authoring/level-design-guide.md).

## Jon's notes (2026-11-14, after playing level 1)

1. **Gate, not ring.** The gate is a triangle: call it a gate.
2. **Pacing.** The sequence and timing of the dialogue are wrong.
3. **Soft lock.** It is easy to get stuck behind the clearing's barrier.
4. **Arrivals.** Goblins must not spawn in: they enter through the door, and
   the camera shows it without panning to them. Things are placed so that
   the camera and the dialogue show the scene.
5. **Doors.** The world uses doors too much. A door goes into an interior;
   connected rooms join by edge zones, which also makes LDtk placement
   simple and the world feel connected.
6. **Background.** The gate is drawn in the region's background; in the room
   where the gate stands, the background must not show it too.
7. **The first view outside.** Leaving the workshop, the gate is in the
   background, and that is right: the first scene outside exhibits the
   world, gives a taste of the choices the player will have, and makes them
   want to explore. (Only the room where the gate itself stands drops it
   from its background.)
8. **Rich and believable.** Use the hills: vertical movement up paths and
   slopes, not platforms put in the air. A mountainous region beside the
   town gives more natural height.

## Plan and state

| # | Work | State |
|---|---|---|
| 1 | Player-facing "ring" to "gate" in Hobford's signs, zone names and dialogue | done: signs, zone names, room titles, `intro.yarn`, the gate network's names (the work order cutscene still says HOBFORD in capitals: cutscene text has no variables) |
| 2 | The Creator's talk: the raid starts after the last answer has landed, not under the question; each beat finishes first | done: answer, a beat (`<<wait 1.5>>`), "Hang on. Was that a cart?", the raid called, "The door's barred. It'll hold.", the bar breaks, two short lines while the raiders walk in |
| 3 | The barrier lever works from both sides and the barrier stays up once lifted | done: the lever stands beside the boom on the road's side and sets `hobford_barrier_lifted`, which no pull clears; the only rooms past it (the gate, the freight track, the siding) are reached through it, so no body is ever on the far side of a shut boom |
| 4 | The raid: the barn is the interior and its yard is outside; the goblins start beyond the yard, out of shot, and walk in through the open barn door while the camera holds Scout, the Creator and the door. The arena lock is the yard gate (the encounter's own wall), so no one leaves mid-fight; the Creator's last words wait for the fight (`after_encounter` on the trigger) | done: the raiders appear in the doorway about a second after the bar breaks and break in on arrival; the probe's frame log is in the commit message |
| 5 | Edges between Hobford's outdoor rooms; the barn and other interiors by doors; then a census of every world's doors with a queue row for the rest | done: level 1's outdoor joins are all edges (the four doors are gone); the census of the other worlds is below, and its candidates are the queue row [DOOR-EDGES](../queue.md#door-edges--outdoor-joins-that-are-still-doors) |
| 6 | A Hobford background without the gate (`hobford_gateside`) for the gate clearing and the siding | done: `hobford_gateside` is the country drawn from the same seed without the ring, scaffold and cable poles; the two rooms name it in the new level field `backdrop` (the background's theme alone; the skin stays the country's) |
| 7 | The land: hills instead of two rows of rooms joined by doors | done: one line of rooms west to east (below); the lane, the meadow and the clearing are hill rooms that climb between the hilltops' height and the valley's on filled `SurfaceChain` slopes, which the navigation graph walks (`NavLegKind::Slope`). The mountain: THE FELL (`intro_fell`, 2026-11-15), east of the siding, the line's east end. The drove road climbs 832 px from the siding's floor to a summit far above the gate's hill: slopes, the sheepfold terrace with its wall to hop and the shepherd, a scree ledge, two 64 px crags, the top with its cairn and a lookout camera zone. The census walks it with level 1's kit (slope, hop, slope, hop, hop, slope, hop). Dressed (2026-11-16): rocks and flowering gorse along the drove road and on the scree, sheep in the fold and on the top (`hill_fell_*` in `hill_village_props.py`, placed by `fell()`'s `DRESSING` table; a `_slope` piece's foot follows the road's rise, so none floats over it); the fold's wall is dry stone and the cairn a heap of stones, each a prop over its solid block (scenery draws in front of blocks). Seen in `capture_scene` captures of the fold, the road and the top |
| 8 | Props (`hill_village_props.py`, nine pieces, name-neutral ids `hill_*`): publish, place, capture | done (2026-11-16): captured in place with `capture_scene` (offscreen, works on this VM). What the captures showed, fixed: the notice board stood over the inn's east end (moved east, its notices with it); the oak's limbs were still drawn as walk boards after the meadow stopped using them as ledges (redrawn as tapering boughs); and every solid block that is an object, not ground, was a cube of earth with grass on it: the well's curb, the stream's stepping stones and rock, the woods' roots, the clearing's standing stones, the meadow's wall and the shepherd's hut (each now a prop over its block: dry stone, a pier, a boulder, bark, a carved stone, a turf-roofed bothy; the blocks' numbers are their real 16 px cells). A prop kind needs a row in `prop_sheets.ron` or it draws as a placeholder box with no warning; a census of all 14 worlds found no other kind without one. The clearing's barrier drew as a dark slab (a lock wall with no look); it is now `hill_gate_barrier`, a prefab steel leaf the lever slides away (`intro.ldtk`'s `LockWall` gained the optional `look` field) |
| 10 | The lane as the first view: from the barn door the land falls away east; the valley, the village roofs and the gate on the far hill are in view, and the bank path is visibly the way down | done: the lane is high on the west hill and the bank path goes down to the mill in two pitches. On the hilltops (the lane, the meadow's top, the clearing's road) a lookout `CameraZone` lifts the frame 180 px (`LOOKOUT_LIFT`): framed on Scout, the hill's earth filled the lower half and hid the background; lifted, the village roofs, the valley and the gate on the far hill are in view from the barn door |
| 9 | People: villagers, the miller, the gatekeeper, from the existing cast or new chargen residents | done: four new chargen residents (`valley_miller`, `valley_innkeeper`, `valley_trader`, `valley_gatekeeper`; ids carry no proper names, the shown names are their trades, the lore's "Gatekeeper" as in Nagisa the Gatekeeper). The miller is at the millstream, the trader and the innkeeper in the village, the gatekeeper in the gate clearing. Each has a Yarn node that reads the world (the miller and the others know when the Creator is dead). The lost hen is now a small errand: asked, found, brought back, and the innkeeper gives a potion for it. In the Hall of Characters they have their own wing. Not checked on screen (this VM has no display) |
| 11 | The Creator dies after the raid (Jon: "he just kinda stands there", and talking to him after the fight he "seems fine") | done: the final fragment's trigger was a 96 px strip outside the door, and a fight carried Scout past it into the yard, so the last words never played and he never fell. The trigger is now the whole workshop: the moment the raid is won he speaks and falls (probe: health 0, `intro_creator_dead`). His post-raid talk line is hurt, not "I'm all right". Since SS3 he lies where he fell on every later visit (a cutscene `Fall` keeps its death record) |
| 12 | The raid's survivors run off with the case (Jon: if every goblin is killed the story does not make sense) | done with [story sequences](../engine/story-sequences.md) SS1 to SS5: when two raiders have fallen, the last runs out through the yard gate with the case (`intro_raider_runs_with_the_case`); the Creator turns and points, the camera goes ahead to the gate, and his going wins the raid. The case is not yet an object he carries; radiant objectives later |
| 13 | The first tunnel: under the valley from the mill wheel's hollow to the woods' hollow oak ([TN2](../engine/dark-rooms-and-tunnels.md)) | done (2026-11-16): THE SALT RUN (`intro_salt_run`), the smugglers' way under the gate's toll, dark (75%); see [level 1](level-1.md#the-salt-run-intro_salt_run). The woods' hollow oak is drawn now (`hill_woods_hollow_oak`): its climb was three boards in the air, and is three shelves inside a split trunk, with the way down between its roots |

## The land (rows 5, 7 and 10): the design

Level 1 was two rows of rooms, one above the other, joined by doors that
stood for paths down a bank. It is now one line of rooms west to east, and
the land's height is the land's: the barn on the west hill, the valley with
the mill, the village and the woods, the gate on the east hill, and past
the siding the fell, the mountain beside it.

    [workshop]--[lane]                              [meadow]--[clearing]
                     \ bank path       hill path /              \ freight track
                      [millstream]--[village]--[woods]             [siding]--[fell]

| Room | Its heights | Joins |
|---|---|---|
| workshop | the barn floor (hilltop height) | lane |
| lane | hilltop height west, the valley's east: the bank path down in two pitches with a landing | workshop (west, high); millstream (east, low) |
| millstream, village, woods | the valley floor | each to the next |
| meadow | the valley's west, hilltop height east: the hill path up, the shepherd's terrace halfway | woods (west, low); clearing (east, high) |
| clearing | the road and the gate at hilltop height; the freight track down to the valley's height | meadow (west, high); siding (east, low) |
| siding | the valley floor below the gate | clearing; the fell (east) |
| fell | the valley floor west, the summit (832 over it) east: the drove road up | siding |

Why one line and not a web. In a side view a room can join two rooms on
one side at two heights only if one way passes over the other, and a
filled slope's earth runs down to the room's floor: a road under a hill
path would be drawn inside the hill. A line with one height per side needs
neither a tunnel nor a bridge. (The painted `Terrain` layer draws overhangs
and caves exactly, and since 2026-11-16 the navigation graph plans over it
(TN1); the first crossing in level 1 can be painted.)

Rules the rooms follow:

- **Edges sit on flat ground.** An edge's arrival puts the body 92 px in
  from the edge at the zone's middle height (`edge_arrival`), not where it
  crossed, so each edge has a flat stretch of block ground at least that
  wide, and the slopes are inside the rooms.
- **Slopes join block tops.** A slope is a filled `SurfaceChain` from the
  top corner of one block to the top corner of the next (the generator's
  `slope()`, no steeper than 0.42, a walk rather than a scramble); things
  stand on the blocks, and the navigation graph links them by the slope.
- **Slope ends are on the 16 px grid.** The blocks are IntGrid cells. A
  slope that ended at 3418 against a block that ends at 3424 left the
  block's corner 2.4 px proud of the path, and a climber walked into it
  (the census found the freight track impassable uphill). `slope()` refuses
  an end off the grid.
- **Alice's survey line** is pegged across the lane, low enough to step
  over (a 24 px `LockWall`); it comes up when Bob's survey is in, as before.

## Door census of the other worlds (row 5, 2026-11-14)

Every `LoadingZone` with `activation: "Door"` in the 14 shipped worlds
(`game/ambition_content/assets/worlds/*.ldtk`, read through the symlinks),
each two-way pair counted once: 227 door joins. The rule is the
[level design guide's](../../authoring/level-design-guide.md): an edge
between two parts of one connected place, a door into something.

Not candidates, by kind:

- **Gate arrivals** (`the gate`, `the public gate`, `ring 1`, the survey
  rings, the arches): a zone with no target room is where the gate network
  puts a traveller, not a way between rooms.
- **The sandbox's hubs and labs** (117 joins): the developer hub's doors to
  test rooms and the halls of biomes and bosses are a menu of rooms, not a
  place. The halls of characters and bosses and their arenas are the same.
- **Crossings out of a dimension** (`to central hub`, `home to the hub`).
- **Doors into and out of interiors**, which is what a door is for: the
  shops, taverns, houses, the hotel and its Monty Hall doors, the temple's
  rooms off the nave, the castle's halls, the station's decks (stairs and
  lifts between decks inside one station), the drain cover, the Door of No
  Return (a door in the place itself).

Candidates (both rooms read as outdoors, or one place cut in two); each
wants a look at the two rooms before it changes:

| World | Join | Why it is a candidate |
|---|---|---|
| history | `history_borobudur` and `history_srivijaya` ("back to Srivijaya") | two outdoor places on one route |
| history | `history_classical_shore` and `history_colosseum` ("into the colosseum") | an open-air arena: a door if the colosseum is drawn as a building entered, an edge if it is the next stretch of ground |
| pvn | `pirate_cove` and `pirate_port_docks`; `pirate_cove` and `pirate_sky_lookout` | the shore and the docks, the shore and a lookout above it |
| pvn | `ninja_dojo` and `ninja_village_forest`; `mockingbird_arena` and `pirate_cove`, `ninja_dojo` | the older pvn rooms joined by doors from before the two towns were built |
| science | GNU-ton's chain: `gnu_ton_orchard`, `gnu_ton_vector_well`, `gnu_ton_conservation_hall`, `gnu_ton_inversion_gallery`, `gnu_ton_works_approach`, `gnu_ton_shoulders` ("on to the ...", "back to the ...") | a walked sequence along the truss joined by doors; "hall" and "gallery" may be interiors, the orchard and the well are not |
| dinosaur | `primeval_canopy` and `primeval_tyrant_hall` | a door if the hall is a building or a cave mouth, an edge otherwise |

The census script was a scratch read of the LDtk JSON (it is not kept); to
run it again, group the `LoadingZone` entities by `activation` and pair
each door with its `target_room`/`target_zone`.

**Verdicts (2026-11-16, [DOOR-EDGES](../queue.md#door-edges--outdoor-joins-that-are-still-doors)).**
Each pair's two rooms were read (their zones, their generators, and
captures with `capture_scene`); none became an edge:

| Join | Verdict |
|---|---|
| Srivijaya and Borobudur | a door: Borobudur is on Java, across the strait, and the door is "TO JAVA", a passage by sea from mid-room; every side of Srivijaya is already an era neighbour's edge in the history grid |
| the classical shore and the colosseum | a door into a building: the colosseum is entered. The building was not visible from outside; the shore now has its facade round the door (`history_colosseum_facade`: three tiers of arches, the broken top of the outer ring) |
| the pvn cove, arena, dojo and sky lookout | doors: these are the old developer-hub rooms (each has a door to `central_hub_complex`), the census's "hubs and labs" class; the dojo is also an interior ("the hall of the Shadow Clan"). The docks' and the forest's doors are those rooms' entrances from the towns |
| GNU-ton's chain | doors: the gravity works are enclosed puzzle chambers (each has a vault or ceiling that is walked on, and a mistake starts the chamber again at its door), the same class as the station's decks |
| the canopy and the tyrant's hall | a door into a lair: the hall is the T. rex's, one way in, beside the canopy's edge on up the mountain |


## Names are placeholders

Jon (2026-11-14): "I'm 90% sure I'm not going to keep Hobford." Every
person and place name must be easy to change. Rules:

- Dialogue says a place name through a Yarn variable declared once
  (`dialogue/sandbox/names.yarn`, `{$village}`), not in the line. Done for
  `intro.yarn`.
- The generator writes signs and room titles from one names table at the
  top of `generators/intro/make_specs.py` (`VILLAGE_NAME`). Done.
- Text with no variables still says the name: the cutscene RON (the work
  order's "HOBFORD"), and the gate network's display names
  (`gate_network.ron`: "Hobford Gate"). A rename edits those two by hand.
- New ids (rooms, props, flags, art) do not contain a proper name, so a
  rename touches only text. Older ids that do (`intro_hobford`,
  `hobford_ring`, the `hobford` backgrounds) are listed here as they are
  renamed.

## Notes as they are learned

- A `LockWall` with no `gated_by` is the encounter's own wall: it stands
  while the encounter is active and is down before and after it
  (`EncounterLockWallSpec`, one per area).
- Gate conditions have no negation, so a wall cannot be "shut until X,
  open during X, shut again".
- A `CutsceneTrigger` had no condition: a volume past the barn door played
  the Creator's last words whenever a body stood in it. It now has
  `after_encounter`.
- Dialogue cannot start an encounter: a reference argument
  (`encounter:<id>`) is refused on the dialogue surface by design, so
  `encounter.start` in `intro.yarn` had never run (the raid used to start
  because the trigger stood where Scout talked). A room's `while_live` line
  now takes a condition (`while_live_when`), and the talk sets the flag.
- A character's own `named_autonomous_profile` decides how far it notices
  (460 px for the lab raider); an encounter mob's `kind` does not override
  it. Staging has to put arrivals inside that distance.
- A `CameraZone` with `clamp_mode: zone_bounds` frames an interior: the
  barn's zone ends at its front wall, so the yard is out of shot until
  Scout goes out. Once a fight is on, the camera widens to frame its
  combatants.
- A threat that breaks into a conversation redirected it to
  `<opening node>__interrupted`, and the dialogue bridge then failed to
  start that node on the running Yarn runner, which closed the conversation
  silently. The bridge now stops the runner first. A `<<wait>>` in progress
  still delays the new node until it ends, so waits stay short where a
  threat can arrive.
