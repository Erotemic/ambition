# Composing regions (dense, interconnected, LDtk-tweakable)

Jon, 2026-10-30:
- "focus on compositions of levels, rather than having levels continue out in flat zones, think of how hollow knight / silk song do it. There are large interconnected regions with paths between them and the world is dense, it doesn't go on horizontally or vertically forever. It mixes between them, and has winding and secret passages."
- "I need [it] to be laid out in a way where it's pretty good by default, but I can also go in and manually tweak and reposition the levels / shift the world around in ldtk."
- "It needs to feel like a connected world, not a series of disconnected doors."

## Rules

- **A region is a 2D cluster of rooms that touch on the flat map** and join by edge exits (side and vertical). Doors are for entering a building or a separate place; a region's inside is joined by its edges.
- **Loops, not corridors.** A room has more than one way in where it can. Every region has at least one loop.
- **Shortcuts are opened from the far side.** A shortcut is a lock that is shut until found from beyond it. Example: a lever behind a cracked floor (`LockWall` gated by `world.switch_on <lever>`).
- **Secrets are hidden on the map.** An edge exit whose zone id starts `secret_` joins the region, but the map shows the room behind it only once it is visited (`MapRoomNode::secret_exits`). Mark both sides `secret_`. A false wall is a gap in a wall solid at the floor; the room's art covers it.
- **Verticality without feeling underground:** stack floors in buildings and dungeons; outdoors, prefer climbs, towers and drops over cellars.
- **Don't append flat zones.** Don't add another room east or west of a region's last room.

## Placement: LDtk owns it

- `area create --replace-existing --keep-position` regenerates a room where the LDtk editor has it. The spec's `world_x`/`world_y` place only a new room.
- Generators that follow this keep positions by default and take `--relayout` to put every room back where the generator says. So far that is `castle_godwin/make_specs.py` and `castle_kitchens/make_specs.py`; `district_kit.make_rooms(..., keep_position=True)` gives the same to any district generator.
- After moving rooms in the editor, `python specs/check_flat_map.py <world>.ldtk` lists every edge exit whose two sides no longer meet: the edges don't touch, the walk lines differ, or a vertical join doesn't overlap.

## Castle Godwin (the first composed region)

```
                                   [great hall]──door──▶ throne room (one room: the boss)
[forest]─door─▶[gatehouse][barracks]─▶[tower]──┘
                  │stair      ┊trapdoor (lever)  │ the tower is the spine
[guild room]┄secret┄[kitchens][complaints][cells]─┘└─[workshop]
```

- **Upper floor:** gatehouse, barracks and the lab tower side by side. The barracks join the tower partway up its climb (y 704). The tower's top joins the great hall, and its foot joins the workshop.
- **Lower floor:** a stair down through the gatehouse's floor into the kitchens, then east through the Complaints Department and the cells into the tower lower down (y 1344). The whole castle is a loop.
- **Shortcut:** the lever in the cells opens the cracked trapdoor in the barracks' floor.
- **Secret:** the kitchens' west wall is false at the floor, and the Guild's room is behind it.
- **Doors left:** the front door (forest to gatehouse), and the great hall to the throne room. The throne room is the Wolfenstein/Inugami fight and its chase; a boss cannot cross rooms, so it stays one room, under the castle.
- **Tests:** `game/ambition_app/tests/the_castle_kitchens.rs`.

## The Evil outdoors (the second)

```
 [Gallows Hill]══bridge══[Searchlight Ridge]─────────────┐
   │bell tower             ┊gap (drop)        │watchtower
 [Lower Godwin]──[allotments]──────────────[moor]──[gate forest]
                   │plot 13                   │crypt stair
                 [catacombs]────────────────[crypt]
```

- **Surface row:** the hamlet, the allotments and the moor, joined at the floor; the moor joins the gate forest (and the castle).
- **Under:** the crypt stair in the moor, west through the catacombs, up the shaft under plot 13 into the allotments.
- **Over:** the moor's watchtower up onto the ridge; west along it, over the plank bridge, onto Gallows Hill and down the hamlet's bell tower. The bridge is dropped through onto the allotments' greenhouse (a shortcut down).
- **Tests:** `game/ambition_app/tests/the_searchlight_moor.rs`.

## The Pirate Port (the third)

```
 [plaza]═high/low═[waterfront]═high/low═[docks]──[reef]──[lighthouse point]
   │well             │drain (grate: lever below)
 [cellars]──arch──[smugglers' tunnel]┄planks┄[hoard]
```

- **Two roads over:** the low road (street, pier) and the high road (galleries, roofs, gantry) join the plaza, the waterfront and the docks.
- **Under:** down the plaza's well into the cellars, east through the arch into the smugglers' tunnel, up the drain into the waterfront's street. The drain's grate is shut until the lever under it is pulled (a shortcut opened from below).
- **Secret:** the plank wall at the tunnel's east end; the hoard behind it.
- **Tests:** `game/ambition_app/tests/the_wreckers_reef.rs`.

## The Ninja Village (the fourth)

```
 [shrine]═terrace/roofs═[street]═roofs/branches═[forest]──[bamboo pass]
   │slab (lever below)    ┊loose board (secret)
 [────── the hidden way ──────]
```

- **Two roads over:** the floor, and the roofs from the shrine's terrace to the forest's branches.
- **Under:** a loose board in the street (a one-way floor, dropped through; `secret_`) into the hidden way, west to a stone slab in the shrine's floor, shut until the lever under it is pulled.
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
- **Generator:** `specs/primeval/treetops.py` (the run and the rookery are edited in place).
- **Tests:** `game/ambition_app/tests/the_tar_pits.rs`.

## Eureka (the sixth)

```
 [test range]──[gate court]═walk/street═[quad]──────────────[ground station]──tether up
                                          │manhole   ↑stair
                    [Lab 0]┄bricked┄[steam plant]──[collider]
```

- **Under:** down the manhole in the quad's street into the steam plant (catwalks, boilers), east on its floor into the Collider, up its stair to the quad: a loop.
- **Secret:** a bricked door at the plant's west end (a breakable wall), Lab 0 behind it.
- **Tests:** `game/ambition_app/tests/the_eureka_tether.rs`.

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
