# Composing regions (dense, interconnected, LDtk-tweakable)

**Execution:** [WORLD-LAYERS](../queue.md#world-layers--fold-the-remaining-long-dimensions-into-connected-regions---done-2026-11-18)
selects the remaining layout work. Recheck existing secrets and connections
before adding them; the region descriptions below include completed work.

Jon, 2026-10-30:
- "focus on compositions of levels, rather than having levels continue out in flat zones, think of how hollow knight / silk song do it. There are large interconnected regions with paths between them and the world is dense, it doesn't go on horizontally or vertically forever. It mixes between them, and has winding and secret passages."
- "I need [it] to be laid out in a way where it's pretty good by default, but I can also go in and manually tweak and reposition the levels / shift the world around in ldtk."
- "It needs to feel like a connected world, not a series of disconnected doors."

## Rules

Read the [level connection rules](../../concepts/level-connections.md)
before you lay out rooms.

- **A region is a 2D cluster of rooms that touch on the flat map** and join by edge exits (side and vertical). Doors are for entering a building or a separate place; a region's inside is joined by its edges.
- Give each visible passage door a destination. Use a side loading zone
  where adjacent rooms share a walk.
- **Loops, not corridors.** A room has more than one way in where it can. Every region has at least one loop.
- **Shortcuts are opened from the far side.** A shortcut is a lock that is shut until found from beyond it. Example: a lever behind a cracked floor (`LockWall` gated by `world.switch_on <lever>`).
- **Secrets are hidden on the map.** An edge exit whose zone id starts `secret_` joins the region, but the map shows the room behind it only once it is visited (`MapRoomNode::secret_exits`). Mark both sides `secret_`. A false wall is a gap in a wall solid at the floor; the room's art covers it.
- **Verticality without feeling underground:** stack floors in buildings and dungeons; outdoors, prefer climbs, towers and drops over cellars.
- **Don't append flat zones.** Don't add another room east or west of a region's last room.

## Placement: LDtk owns it

- `area create --replace-existing --keep-position` regenerates a room where the LDtk editor has it. The spec's `world_x`/`world_y` place only a new room.
- Generators that follow this keep positions by default and take `--relayout` to put every room back where the generator says. So far that is `castle_godwin/make_specs.py` and `castle_kitchens/make_specs.py`; `district_kit.make_rooms(..., keep_position=True)` gives the same to any district generator.
- After moving rooms in the editor, `python specs/check_flat_map.py <world>.ldtk` lists every edge exit whose two sides no longer meet: the edges don't touch, the walk lines differ, or a vertical join doesn't overlap.

## Contents: one owner per room

Jon, 2026-11-04: "we need to make sure we know what data is the truth... What happens if I edit ldtk and then the spec clobbers the ldtk?"

- A generated room's truth is its generator, until someone edits the room in LDtk. `area create --replace-existing` builds the whole level again, so an edit inside a room would be lost.
- The record `<world>.generated.json`, beside the world in the map submodule, holds a fingerprint of each room as its generator last wrote it (`ambition_ldtk_tools/generated_record.py`). The fingerprint leaves out placement, uids and iids.
- `area create --replace-existing` refuses to replace a room that changed since then. `district_kit.make_rooms` checks every room before it writes any of them, and records them after the last one (and again after the generator's own later paint and entity steps).
- `python -m ambition_ldtk_tools level ownership status --ldtk <world>` lists each room: `generated`, `HAND-EDITED`, `ldtk-owned` or `unrecorded`.
- To keep an edit: `level ownership adopt <room>`. The room is then LDtk-owned and its generator skips it; the LDtk file is its truth from then on. To discard the edit: run the generator with `--overwrite-hand-edits`.
- Generators are for the first pass and fast worldbuilding. A room that is polished by hand graduates to LDtk by `adopt`.

## Castle Godwin (the first composed region)

```
                                   [great hall]──door──▶ throne room (one room: the boss)
[forest]─door─▶[gatehouse][barracks]─▶[tower]──┘
                  │stair      ┊trapdoor (lever)  │ the tower is the spine
[guild room]┄secret┄[kitchens][complaints][cells]─┘└─[workshop]
```

- **Upper floor:** gatehouse, barracks and the lab tower side by side. The barracks join the tower partway up its climb (y 704). The tower's top joins the great hall, and its foot joins the workshop.
- In the gatehouse, the floor passes below the guard catwalks. Steps
  connect the guard platform to the watch gallery. The ladder at the east
  end connects the gallery to the barracks exit.
- In the barracks, service walks connect the lower bunks. Ladders beside
  the bunks connect the bunks to each other and to the floor.
- In the lab tower, two service ladders connect the exits at different
  heights. Two walks cross the shaft between the ladders.
- In the great hall, steps from both sides connect the floor to the
  galleries and the central balcony. The muster also uses the dais.
- In the throne arena, steps beside the coils and the east gate connect
  the floor, the two lower galleries, and the high gallery. The practice
  arena in the Hall of Bosses uses the same steps and bench.
- **Lower floor:** a stair down through the gatehouse's floor into the kitchens, then east through the Complaints Department and the cells into the tower lower down (y 1344). The whole castle is a loop.
- In the kitchens, crates at both ends give access to the pantry loft.
  The floor route passes the hearth and the staff.
- At Complaints, the paperwork crates give access to the counter. The
  floor passes the ticket queue and leads to the cells.
- In the cells, the filing shelf connects the desk and the ladder to
  the barracks trapdoor. The floor continues to the tower.
- The lever in the cells opens the cracked trapdoor from both sides.
- In the workshop, the boiler ladder and the test stands give access to
  the upper walk. Jump across the hoist opening to reach the parts chest
  from the boiler. The floor route leads to Sprocket.
- **Secret:** the kitchens' west wall is false at the floor, and the Guild's room is behind it.
- In the Guild Room, jump from the floor to the lectern or the minutes
  desk. A jump also connects the two platforms. The strike fund and the
  Clubman are on the floor.
- **Doors left:** the front door (forest to gatehouse), and the great hall to the throne room. The throne room is the Wolfenstein/Inugami fight and its chase; a boss cannot cross rooms, so it stays one room, under the castle.
- **Tests:** `game/ambition_app/tests/the_castle_kitchens.rs`.

## The Evil outdoors (the second)

The cemetery wall gives travellers a raised crossing through the gate
forest. Steps connect it to the road on both sides of the graves. A
conscript starts on the wall. The spectral wolf starts on the road.

```
 [Gallows Hill]══bridge══[Searchlight Ridge]─────────────┐
   │bell tower             ┊gap (drop)        │watchtower
 [Lower Godwin]──[allotments]──────────────[moor]──[gate forest]
                   │plot 13                   │crypt stair
                 [catacombs]────────────────[crypt]
```

- **Surface row:** the hamlet, the allotments and the moor, joined at the floor; the moor joins the gate forest (and the castle).
- In Lower Godwin, the flour bin and rack lead to the bakery loft. The
  crier's platform leads to the east landing. Jump between the landings
  or take the ladder up to Gallows Hill.
- Inside Crumb's bakery, the loading shelf connects the ovens and the
  flour rack. The floor continues to the press behind the sacks.
- At the allotments, the seed chest and the Plot 13 landings give access
  to the greenhouse roof from the west. The crop trellis and the watering
  shelf give access from the east. The Failed Prototype starts on the
  trellis. A Crawling Conscript starts in the soil lane below it.
- On Searchlight Moor, cross the bog by the broken boardwalk or through
  the water. A Searchlight Crawler starts on the boardwalk. The cargo deck
  and the founder's statue give access to the watchtower from opposite
  sides. The mast ladder leads to the ridge.
- In the Crooked Lantern, the map shows the road, the ridge, and the
  crypt. Climb from the wood box or the bar to read it. Tobias and
  Gudrun meet visitors on the floor.
- **Under:** the crypt stair in the moor, west through the catacombs, up the shaft under plot 13 into the allotments.
  In the Founder's Crypt, plinths give access to the funeral walk and the
  sarcophagus. A Shock Warden guards the floor below.
  In the catacombs, the broken vault has a raised walk to the shrine. A
  conscript is on the walk and a crawler is below it.
  In the ossuary, both side plinths give access to the signet altar. The
  floor leads past the six busts to the broken wall.
- **Over:** the moor's watchtower up onto the ridge; west along it, over the plank bridge, onto Gallows Hill and down the hamlet's bell tower. The bridge is dropped through onto the allotments' greenhouse (a shortcut down).
  On the ridge, jump between the gantries and the sentry hut roof.
  A Searchlight Crawler starts above the patrol lane on the east gantry.
  On Gallows Hill, hold Up at either foot of the terrace to take the
  stone walk. The old wall gives access from the footpath. The Spectral
  Wolf starts on the terrace.
- **Tests:** `game/ambition_app/tests/the_searchlight_moor.rs`.

## The Pirate Port (the third)

```
 [plaza]═high/low═[waterfront]═high/low═[docks]──[reef]──[lighthouse point]
   │well             │drain (grate: lever below)
 [cellars]──arch──[smugglers' tunnel]┄planks┄[hoard]
```

- **Two roads over:** the low road (street, pier) and the high road (galleries, roofs, gantry) join the plaza, the waterfront and the docks.
- **Docks:** ladders connect both banks of each gap to the water. A hatch
  past the brig lets swimmers reach the east end of the pier.
- **Under:** down the plaza's well into the cellars, east through the arch into the smugglers' tunnel, up the drain into the waterfront's street. The drain's grate is shut until the lever under it is pulled (a shortcut opened from below).
- The freight carrier crosses the flooded cut in the tunnel. Swimmers can
  cross below it and climb out at either bank. The quartermaster guards the
  east loading dock.
- **Secret:** the plank wall at the tunnel's east end; the hoard behind it.
- **Reef:** jump from the stern to the bow, or swim across the gap. A ladder
  on each hull connects the water to its deck.
- **Tests:** `game/ambition_app/tests/the_wreckers_reef.rs`.

## The Ninja Village (the fourth)

```
 [shrine]═terrace/roofs═[street]═roofs/branches═[forest]──[bamboo pass]
   │slab (lever below)    ┊loose board (secret)
 [────── the hidden way ──────]
```

- **Two roads over:** the floor, and the roofs from the shrine's terrace to the forest's branches.
- **Cedar Forest:** a fallen cedar connects the ground path to the fourth
  branch and the crows' hoard.
- **Under:** a loose board in the street (a one-way floor, dropped through; `secret_`) into the hidden way, west to a stone slab in the shrine's floor, shut until the lever under it is pulled.
- The floor of the Hidden Way leads directly to the shrine lever. The
  training gallery gives visitors a raised route, with a jump over the
  nightingale floor below. The savings chest is next to the weapons rack
  on the east gallery.
- **Tests:** `game/ambition_app/tests/the_ninja_village.rs`.

## The primeval trail (the fifth)

```
                 [treetops]─────────────gap (drop)┐
                   │great tree                    ▼
 [trailhead]──[run]──────────────────[rookery]──[ridge]──[canopy]──[tar pits]──[smoking mountain]
                                                   │sinkhole
                                               [fossil beds]
```

- **Over:** the rangers' rope ladder up the great tree in the run, out of its top into the treetops (bridges, the abandoned canopy research station, a pterosaur's nest); a gap in the leaves at the east end drops into the rookery.
- **Under:** the ridge road's sinkhole down to the fossil beds.
- **Truth:** `dinosaur.ldtk`; every room is LDtk-owned (`dinosaur.generated.json`). The primeval scripts that first wrote them (`make_specs.py`, `treetops.py`, `side_exits.py`, `tar_pits.py`) were older than their rooms and were deleted for LEVEL-TRUTH; the last version is at `a910d3952e`.
- **Tests:** `game/ambition_app/tests/the_tar_pits.rs`.

## Eureka (the sixth)

```
 [test range]──[gate court]═walk/street═[quad]──────────────[ground station]──tether up
                                          │manhole   ↑stair
                    [Lab 0]┄bricked┄[steam plant]──[collider]
```

- At the Gate Court, the founders' plinth, the bench, and the east steps
  give access to the raised walk. The street passes the gate dialer and
  Ottoline Hum. Both routes lead to the quad.
- In the quad, the raised walk passes the Patent Office, café, and
  fountain on the way to the lecture hall terrace. The café bench and
  awning give access from the street. The manhole below the patent
  balcony leads to the steam plant.
- At the ground station, steps on both sides of the car lead to the
  galleries. The west gallery has the departures board. The east
  gallery has the altimeter. Board the car from the hall floor.
- During the ascent, the roof landings catch riders who leave the car
  beside the shaft. The opening between the landings leads back to
  the ground station.
- At the orbital terminus, the floor leads past Dobbs to the docking
  bay. Steps at either end give access to the arrivals board and the
  viewing gallery. Jump across the gap at the shaft to use both decks.
- In the docking bay, the west gantry and the east ladder lead to the
  walk under the hauler. The shaft at its east end goes up to the
  observation deck. The floor continues to the tether terminus.
- On the observation deck, hold Up at either ramp to reach the viewing
  walk. The step below the center also gives access. Neil and Wen meet
  visitors at the window on the floor. Continue east to the tram spoke.
  At the far end, climb above the express lift to enter the orchard.
- In the habitation ring, hold Up at either stair to reach the grow deck.
  The floor joins the service deck to the observation deck. The ladder
  beside Gus's galley leads down to customs on the gate dock.
- On the service deck, either ladder gives access to the maintenance
  walks. Jump across the gap by the air filters. The hatch at the west
  end opens directly into the damaged spur with the customs pass.
- **Under:** down the manhole in the quad's street into the steam plant (catwalks, boilers), east on its floor into the Collider, up its stair to the quad: a loop.
- **Secret:** a bricked door at the plant's west end (a breakable wall), Lab 0 behind it.
- **Tests:** `the_eureka_tether.rs`, `the_orbital_station.rs`, and
  `the_gnu_ton_handle.rs` in `game/ambition_app/tests/`.

## The math town (the seventh)

```
 [aqueduct]──[town street]──[fractal woods]──[fibonacci meadow]
                              │knot (lever)  │shaft
                            [roots]──────[mobius mine]
```

- **Under:** down the mine shaft from the woods, west on the mine's floor into the roots of the great fractal tree, up through a knot of them (shut until the lever under it is pulled) into the woods again.
- **Tests:** `game/ambition_app/tests/the_zeno_aqueduct.rs`.

## Not yet

- **A hit-breakable wall** (`district_kit.breakable_wall`, broken for good in the save) is the other kind of secret: the catacombs' cracked wall hides the ossuary of the other founders. A breakable draws UNDER the room's scenery (the block band), so the room's art leaves a window over it (the canvas's alpha cut away).
- **Ladders** draw over the room's scenery and under the cast (`CLIMBABLE_Z`); the room's art does not draw them.
- Every region of the five dimensions now has a loop. Next: more secrets (breakable walls), and joining regions to each other.
