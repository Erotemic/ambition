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

### Engine acceptance today

A product milestone is judged by playing it. This table names which criteria
already have engine acceptance, so a session knows which ones it would exercise
first.

| acceptance criterion | engine acceptance |
|---|---|
| explore multiple interconnected regions | ✔ `leaving_a_room_and_returning_rebuilds_what_entering_it_built` |
| acquire materially different traversal/interaction capabilities | ◐ The player can acquire capabilities (the dive, the blink and the portal gun are authored pickups). The world can gate on them: `body.can(verb)` and `body.fits(height)` are published conditions, and a `gated_by` line on a `LockWall` may read them (`a_wall_may_be_gated_on_what_the_body_can_do`, `a_wall_may_be_gated_on_the_body_being_small_enough_to_pass`). The gate is per actor (`a_body_gate_is_open_only_for_the_bodies_that_satisfy_it`). `interact` is an ability verb too. No shipped level authors a body gate; the authored gates read a quest flag. Measure with `scripts/authored_route_gates.py`. |
| move/hold/equip/drop persistent objects | ✔ `an_object_in_your_hands_survives_a_replay_and_is_not_re_authored` |
| alter world mechanisms, return to the changed state | ✔ `a_lever_left_on_is_on_when_you_come_back` (the save is a switch's one authority) |
| encounter persistent and spawned actors | ✔ the encounter suite, `a_spawn_request_on_the_bus_becomes_a_body` |
| save/reload without losing instance/location truth | ✔ `loading_a_save_builds_the_room_a_re_entry_builds`, `a_relocated_occurrence_is_suppressed_by_a_load_and_by_a_re_entry_alike` |
| navigate enough that tooling can reason about routes | ▢ the navigation frontier |
| separate from another participant into another room | ◐ simulation and most of the view are built; see A3 in [`multiplayer.md`](multiplayer.md) |

Open content questions for the maintainer, in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md): Q55
(should authored worlds use all five route-gate families?) and Q58 (does the body
gate family ask what a body can do or what it is doing?). The milestone wording
"traversal/interaction" can mean "the player can acquire" or "the world gates on";
Q55 decides which the world must exercise.

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
