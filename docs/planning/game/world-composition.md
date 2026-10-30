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

## Not yet

- A hit-breakable wall (Hollow Knight's secrets) is not an engine capability. The false wall is the only secret so far.
- The other regions (the pirate port and ninja village, Eureka, the math town, the primeval trail, the evil moor) are still strings of rooms. Each wants the same treatment.
