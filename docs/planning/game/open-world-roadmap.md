# Ambition open-world roadmap — world first, story over reality

**State:** OPEN — this is the flagship product direction, not a linear quest checklist.

## North star

Build a 2D platforming world with RPG-scale systemic depth before relying on a
large authored story structure to make it feel alive.

The controlled robot should be able to roam a substantial connected world with
the real movement/capability vocabulary, acquire items and abilities, change
world mechanisms, encounter persistent and spawned actors, leave meaningful
state behind, save/reload and continue coherently.

When that world feels real, authored story and reactive character dialogue can
inhabit it.

## Build order

### W1 — connected world skeleton

A substantial region graph with alternate routes, verticality, portals and
room/region transitions. The goal is not map acreage; it is enough topology to
stress residency, traversal and returning to changed places.

### W2 — embodied traversal vocabulary

Put the flagship robot into that world with the intended movement/body
capabilities and possession mechanics. World gates should principally test
physical capability/property/tool facts.

### W3 — items and mechanisms

Persistent objects, ephemeral spawned pickups, equipment, keys/tools, moving
platforms, powered/opened/repaired world mechanisms and explicit item custody.

### W4 — persistent population

Named persistent characters plus ordinary spawned mobs, encounter populations
and actors that can exist coherently when their room is not currently visible.

### W5 — systemic intelligence

Reachability/navigation, actor goals, observations and interaction. Dialogue can
react to world facts without becoming authoritative over them.

### W6 — authored narrative layers

Bring the Fia arc, Alice/Bob, factions, quests and larger story structure into a
world whose state already has independent meaning. Use explicit story gates when
sequencing really matters; do not make them the default explanation for why the
world is traversable.

## Product acceptance

A convincing pre-story milestone is a session where the robot can:

- explore multiple interconnected regions;
- acquire materially different traversal/interaction capabilities;
- move/hold/equip/drop persistent objects;
- alter world mechanisms and return later to the changed state;
- encounter persistent and spawned actors;
- save/reload without losing instance/location/accounting truth;
- navigate enough of the world that AI and agent tooling can reason about routes;
- optionally separate from another participant into a different room once the
  multiplayer architecture is ready.

## Open design questions — deliberately unresolved

- What initial region is large enough to stress open-world systems without
  becoming a content-production sink?
- How much fast travel should exist, and what systemic requirements unlock it?
- How punitive should death/item loss be?
- How dense should persistent named population be relative to spawned mobs?
- How much background simulation is needed for the world to feel coherent?
- Which early theorem/capability set best proves embodied progression?
- When does authored story become useful enough to layer in without turning back
  into a linear gate chain?
