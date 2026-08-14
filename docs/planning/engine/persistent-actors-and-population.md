# Persistent actors and population — Engine 1.0 program

**State:** OPEN — shared actor-instance semantics are desired; dormant simulation and respawn policy are not settled.

## Goal

Support a world where some actor instances matter across long periods and room
unloads while other actors are ordinary spawned population.

`CharacterDefinition` is authored body composition, not a singleton actor.
Ambition may normally have one Fia and many goblins, but the engine must still
permit multiple runtime instances of either definition when a game/test/mechanic
requests them.

## Actor categories are policy, not ontologies

Useful policies may include:

- persistent named/world actor;
- session actor;
- room population;
- encounter-bound spawn;
- summon/temporary actor;
- respawning population;
- explicitly disposable/debug actor.

All should construct ordinary actor bodies and differ through provenance,
lifetime, controller/brain, disposition and persistence policy.

## Required capabilities

- stable actor-instance identity when persistence requires it;
- persistent location/residency independent of the current ECS entity;
- explicit spawn/despawn/respawn provenance;
- background/dormant representation when a room is not fully simulated;
- inventories/items and world facts that survive according to policy;
- transition back to full simulation without duplicating the actor;
- queries for agents: where is this actor, why does it exist, what is its current
  simulation fidelity?

## Candidate crate / Bevy shape

The reusable population/lifetime core should not depend on Ambition's named
characters. Whether it belongs with actor simulation, world residency or a
separate population plugin is unresolved. Prefer domain plugins with explicit
handoff messages rather than a global actor manager.

## Open design questions — deliberately unresolved

- Which actors advance behavior while dormant, and at what fidelity?
- Do persistent characters have authored schedules, agent-selected goals, or
  both?
- How are encounters reset without confusing spawned population with persistent
  actors?
- What is the default death/respawn policy for persistent characters?
- How does multiplayer affect which distant actors need full simulation?
- How is narrative/content "uniqueness" validated when duplicate runtime
  instances remain legal?
- Can an actor migrate between persistence classes during play?
