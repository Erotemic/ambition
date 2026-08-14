# Platformer navigation and reachability — Engine 1.0 program

**State:** OPEN — problem and customers are clear; graph representation and dynamic-path semantics are not.

## Goal

Let the engine answer **whether and how a particular body can get somewhere** in
a 2D platformer world.

This is not a generic 3D navmesh project. Reachability depends on authored
geometry, jumps/drops, ledges, ladders, portals, moving platforms, gates, room
transitions and the body's current capabilities/properties.

## Consumers

- NPC pursuit and autonomous movement;
- Ambition open-world population and persistent actors;
- boss/fighter reasoning where long-range traversal matters;
- LDtk authoring validation and LLM spatial reasoning;
- capability-gated world design;
- multi-room route planning and soft exploration hints.

## Required query shape

Eventually an agent or AI should be able to ask:

```text
Can body B reach target T now?
Which traversal transitions make that possible?
Which capability blocks the route?
What routes change if gate G opens?
Can this NPC patrol between these regions?
Where would a moving platform create a new route?
```

## Architecture direction

Prefer a derived traversal/reachability representation over hand-authored
waypoint lore. The representation should reference authoritative world geometry
and capability requirements rather than duplicate them.

Separate **route existence/planning** from **low-level movement execution**. A
brain may choose a route while the ordinary body movement kernel still performs
jumps, climbs or portal traversal.

## Candidate crate / Bevy ecosystem value

This is one of the strongest candidates for an eventually independent Bevy
plugin/crate because platformer navigation is a general gap and can plausibly be
specified without Ambition content. A mature crate should accept world/traversal
adapters rather than import Ambition room or character catalogs.

Do not publish until the Ambition implementation survives at least moving
platforms, portals and capability-dependent routes.

## Open design questions — deliberately unresolved

- Graph of discrete traversal opportunities, sampled reachability field, or a
  hybrid?
- How are continuous jump arcs represented without exploding graph size?
- How are moving-platform/time-dependent routes represented and costed?
- How frequently does dynamic world change invalidate navigation data?
- Are portals ordinary graph edges or a separate routing layer?
- How should navigation span unloaded rooms?
- Does background AI plan exact routes or only region-level intentions?
- How is route risk/danger represented without mixing game-specific utility into
  the navigation core?
