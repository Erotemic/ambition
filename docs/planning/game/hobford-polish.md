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
| 5 | Edges between Hobford's outdoor rooms; the barn and other interiors by doors; then a census of every world's doors with a queue row for the rest | level 1 done: every outdoor join is an edge (the four doors are gone); the census of other worlds is open |
| 6 | A Hobford background without the gate (`hobford_gateside`) for the gate clearing and the siding | done: `hobford_gateside` is the country drawn from the same seed without the ring, scaffold and cable poles; the two rooms name it in the new level field `backdrop` (the background's theme alone; the skin stays the country's) |
| 7 | The land: hills instead of two rows of rooms joined by doors | first pass done: one line of rooms west to east (below); the lane, the meadow and the clearing are hill rooms that climb between the hilltops' height and the valley's on filled `SurfaceChain` slopes, which the navigation graph walks (`NavLegKind::Slope`). Next: the mountain behind the meadow, dressing on the long slopes |
| 8 | Props (`hill_village_props.py`, nine pieces, name-neutral ids `hill_*`): publish, place, capture | published and placed; captures pending |
| 10 | The lane as the first view: from the barn door the land falls away east; the valley, the village roofs and the gate on the far hill are in view, and the bank path is visibly the way down | first pass: the lane is high on the west hill and the bank path goes down to the mill in two pitches; the background shows the village below and the gate far off |
| 9 | People: villagers, the miller, the gate clerk, from the existing cast or new chargen residents | open |
| 11 | The Creator dies after the raid (Jon: "he just kinda stands there", and talking to him after the fight he "seems fine") | done: the final fragment's trigger was a 96 px strip outside the door, and a fight carried Scout past it into the yard, so the last words never played and he never fell. The trigger is now the whole workshop: the moment the raid is won he speaks and falls (probe: health 0, `intro_creator_dead`). His post-raid talk line is hurt, not "I'm all right" |
| 12 | The raid's survivors run off with the case (Jon: if every goblin is killed the story does not make sense) | waits on [story sequences](../engine/story-sequences.md) SS1, SS2 and SS6 (actors that leave through a door; an encounter that ends in a retreat); radiant objectives later |

## The land (rows 5, 7 and 10): the design

Level 1 was two rows of rooms, one above the other, joined by doors that
stood for paths down a bank. It is now one line of rooms west to east, and
the land's height is the land's: the barn on the west hill, the valley with
the mill, the village and the woods, and the gate on the east hill.

    [workshop]--[lane]                              [meadow]--[clearing]
                     \ bank path       hill path /              \ freight track
                      [millstream]--[village]--[woods]             [siding]

| Room | Its heights | Joins |
|---|---|---|
| workshop | the barn floor (hilltop height) | lane |
| lane | hilltop height west, the valley's east: the bank path down in two pitches with a landing | workshop (west, high); millstream (east, low) |
| millstream, village, woods | the valley floor | each to the next |
| meadow | the valley's west, hilltop height east: the hill path up, the shepherd's terrace halfway | woods (west, low); clearing (east, high) |
| clearing | the road and the gate at hilltop height; the freight track down to the valley's height | meadow (west, high); siding (east, low) |
| siding | the valley floor, a dead end below the gate | clearing |

Why one line and not a web. In a side view a room can join two rooms on
one side at two heights only if one way passes over the other, and a
filled slope's earth runs down to the room's floor: a road under a hill
path would be drawn inside the hill. A line with one height per side needs
neither a tunnel nor a bridge. (The painted `Terrain` layer draws overhangs
and caves exactly, but the navigation graph does not plan over it yet; the
first crossing in level 1 waits for that.)

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
