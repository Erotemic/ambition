---
id: level-connections
status: current
authority: durable-concept
aliases:
  - side loading zones
  - edge exits
  - level layout
  - platform routes
related_docs:
  - docs/concepts/llm-spatial-authoring-discipline.md
  - docs/recipes/ldtk-authoring.md
last_verified: 2026-11-27
---

# Level connections — use edge exits

> [!IMPORTANT]
> **A side loading zone is the standard connection between rooms in one
> region. Outside the Sandbox, use a loading-zone `Door` only at a clear
> threshold between places, such as the entrance from an overworld into a
> dungeon or a building. The Sandbox (`sandbox.ldtk`) may use doors
> between rooms. Use jumpable platforms or steps to change height.
> Ladders are a rare exception.**

A room boundary divides the map for loading. A path through that boundary
continues into the next room. A player who walks east through an opening
must arrive on the west side of the next room, on the same path.

1. Draw the region on the flat map before you add connections. Give each
   path a direction and a height. Place the rooms so their edges touch
   where the paths meet.
2. Use `LoadingZone` with `activation: "EdgeExit"` where a path meets
   the room edge. At a side exit, align the floors at the same world
   height. At a vertical exit, align the openings in world coordinates
   and give the player a way back up if the route is two-way. Use platforms
   or steps that the player can jump between.
3. Check the approach and the landing on each side. Leave space for the
   player's body. Give the player a path out of the landing with the
   movement abilities available in that region.
4. If a key, lever, or encounter opens the route, put a physical barrier
   such as a `LockWall` across the passage. Keep the edge exit at the room
   boundary. This lets the player walk through the same passage after the
   barrier opens.
5. Outside the Sandbox, name the threshold for each loading-zone `Door`.
   If both sides are rooms along one continuous route, lay out that route
   and join the rooms at their edges.

Use ladders sparingly. A ladder gives the player one way to go up. Platforms
give the player a choice of where to jump and where to land. Use a ladder
only when its position and use have a specific purpose in that room.

Run `python3 tools/ambition_ldtk_tools/specs/check_flat_map.py <world.ldtk>`
after the layout changes. Check the new routes in both directions with the
region's movement abilities.
