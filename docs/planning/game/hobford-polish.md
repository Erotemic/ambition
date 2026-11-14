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
| 3 | The barrier lever works from both sides and the barrier stays up once lifted; nothing drops a body between the post roof and the boom | the barrier stays up once lifted (the lever sets `hobford_barrier_lifted`; a second pull cannot shut it). The way up to the lever from the clearing itself waits for the hills (row 7): today the ridge is reached a room back, which reads as stuck |
| 4 | The raid: the barn is the interior and its yard is outside; the goblins start beyond the yard, out of shot, and walk in through the open barn door while the camera holds Scout, the Creator and the door. The arena lock is the yard gate (the encounter's own wall), so no one leaves mid-fight; the Creator's last words wait for the fight (`after_encounter` on the trigger) | done: the raiders appear in the doorway about a second after the bar breaks and break in on arrival; the probe's frame log is in the commit message |
| 5 | Edges between Hobford's outdoor rooms; the barn and other interiors by doors; then a census of every world's doors with a queue row for the rest | open |
| 6 | A Hobford background without the gate (`hobford_gateside`) for the gate clearing and the siding | done: `hobford_gateside` is the country drawn from the same seed without the ring, scaffold and cable poles; the two rooms name it in the new level field `backdrop` (the background's theme alone; the skin stays the country's) |
| 7 | The land: replace the upper/lower rows joined by doors with hills. Filled `SurfaceChain` ground (`fill: true`, drawn in the room's terrain skin) gives slopes and paths that climb; a mountainside north of the village (quarry track, shepherd's path, the gate up on its shoulder). Needs: the navigation graph to plan over chains (it hashes them but builds no surface from them), so the route census, the test walker and NPCs can walk the hills | open |
| 8 | Props (`hill_village_props.py`, nine pieces, name-neutral ids `hill_*`): publish, place, capture | published and placed; captures pending |
| 10 | The lane as the first view: from the barn door the player sees the gate on its hill, the village roofs below, the mountain behind, and the paths that lead to each (the track up, the bank path down, the ridge), each one visibly a way to go | open |
| 9 | People: villagers, the miller, the gate clerk, from the existing cast or new chargen residents | open |

## The land (rows 5, 7 and 10): the design

Today level 1 is two rows of rooms, one above the other (the lane, meadow
and clearing up top; the millstream, village, woods and siding below),
joined by doors that stand for paths down a bank. Jon's rules turn that
into one piece of hill country:

- **The village in the valley, the workshop and the gate on hills.** The
  barn stands on the hillside west of the village; the gate stands on the
  east hill; the mountain rises north (up, in a side view) behind the
  meadow. Height is the land's, not a stack of rooms.
- **Paths are slopes.** A way between two heights is a path that climbs: a
  filled `SurfaceChain` (`fill: true`, drawn in the room's terrain skin)
  from one floor to the other, never a door and never a column of boards.
  A steep stretch is a stair cut in the bank (steps of 32 to 48), still
  walked.
- **Rooms join at their sides.** The rooms are laid out where they are walked,
  so a path that leaves one room at a height comes into the next at the same
  height (an edge zone each side). A room that holds two heights (the lane:
  the barn door's level and the bank path down to the stream) has an exit at
  each.
- **The lane is the first view.** From the barn door: the gate on its hill
  far off to the east (the region's background shows it), the village roofs
  down in the valley, the mountain behind, and the paths that lead to each
  one visibly a way to go: the track up the hillside to the ridge, the bank
  path down to the mill, the cart track east.

The rooms and how they join, west to east:

| Room | Its heights | Joins |
|---|---|---|
| workshop | the barn floor (the hillside) | lane, at the barn floor |
| lane | the barn's level; the hillside track up to the ridge; the bank path down to the stream | workshop (west); meadow by the ridge and the cart track (east, high); millstream by the bank path (east, low) |
| millstream | the stream bank, the mill's floors | lane (west, its bank path); village (east) |
| village | the street, the porches and roofs | millstream (west); woods (east); the meadow by the hill path, a slope up out of the street that leaves at the top of the room |
| meadow | the pasture; the ridge; the mountain's foot | lane (west); clearing (east, low and high); the village by the hill path |
| clearing | the road, the post, the gate on its hill; a path up to the ridge inside the room | meadow (west); the siding by the freight stair (a stair cut in the bank, an edge at the bottom of the room) |
| woods, siding | the valley floor east of the village | as today, the siding's back stair an edge |

Engine work this needs: the navigation graph builds no surfaces from
`SurfaceChain`s (it hashes them but plans only over block tops), so the
walker, the route census and NPC navigation cannot cross a slope. A chain
becomes a walked link between the surfaces at its two ends (walk toward the
far end; no jump), with the census checking a slope's steepness against the
body's walkable angle.

Order: (1) chains in the navigation graph, with its tests; (2) the lane, its
paths and the millstream beside it at the lower height; (3) the meadow and
the village's hill path; (4) the clearing's own way up to the ridge (which
ends the "stuck behind the barrier" feeling) and its freight stair; (5) the
door census of every other world, filed as a queue row.

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
