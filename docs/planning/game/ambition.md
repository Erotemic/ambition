# Ambition on the engine — flagship customer

Ambition is the primary product driver and deepest customer of the reusable
engine. It is not a thin demo that waits for a "finished engine" before game
production begins. The game and engine co-evolve: real Ambition world pressure
reveals missing reusable capability, and the resulting capability returns to the
game through supported engine surfaces.

Story/pillars: [`vision.md`](vision.md). Open-world build order:
[`open-world-roadmap.md`](open-world-roadmap.md). Systemic progression:
[`systemic-progression.md`](systemic-progression.md). Multiplayer:
[`multiplayer.md`](multiplayer.md).

## Structural rule

Named Ambition content and product policy live in game/provider crates. Reusable
actor, world, persistence, navigation, item, view, authoring and service semantics
belong in engine domains when another game could plausibly consume them with the
same meaning.

The same rule applies to acceptance-game wings and content integrations: they may
share the engine and even appear inside Ambition, but they do not define a second
engine ontology.

## Current flagship pressure

The most important Ambition-driven engine work is:

1. finish the controlled-character actor kernel;
2. make LDtk/kinematic world mechanics first-class;
3. establish persistent world residency and instance/item accounting;
4. make capability/item/world-state progression queryable;
5. add platformer reachability/navigation;
6. support persistent/spawned actor populations;
7. support local/remote/mixed multiplayer and adaptive multiview;
8. layer reactive character intelligence/dialogue over authoritative world facts.

The current story arcs remain desired product work, but the world-first roadmap
outranks the old assumption that a linear narrative slice is the implementation
spine.
