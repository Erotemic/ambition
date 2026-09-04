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

### Which of these the ENGINE already pins, measured 2026-09-03

A product milestone is judged by playing it, not by grepping — but five of the
eight have engine acceptance today, and naming which does two things: it stops a
reader assuming none of it is real, and it isolates the three that a session
would actually be the first to exercise.

| acceptance criterion | engine acceptance at HEAD |
|---|---|
| explore multiple interconnected regions | ✔ `leaving_a_room_and_returning_rebuilds_what_entering_it_built` |
| acquire materially different traversal capabilities | ◐ **the ENGINE pins it since 2026-09-04; no shipped level authors it** — `a_wall_may_be_gated_on_what_the_body_can_do` and `a_wall_may_be_gated_on_the_body_being_small_enough_to_pass` (`gated_lock_walls/tests.rs`), see below |
| move/hold/equip/drop persistent objects | ✔ `an_object_in_your_hands_survives_a_replay_and_is_not_re_authored` (both retention legs) |
| alter world mechanisms, return to the changed state | ✔ `switches_restore_their_on_state_from_the_save`, and the reconstitution census compares switch position across every lifecycle path |
| encounter persistent and spawned actors | ✔ the encounter suite, plus `a_spawn_request_on_the_bus_becomes_a_body` |
| save/reload without losing instance/location truth | ✔ `loading_a_save_builds_the_room_a_re_entry_builds` and `a_relocated_occurrence_is_suppressed_by_a_load_and_by_a_re_entry_alike` |
| navigate enough that tooling can reason about routes | ▢ open — the navigation frontier |
| separate from another participant into another room | ▢ gated — A3 in [`multiplayer.md`](multiplayer.md), needs several resident rooms |

⭐⭐ **THE SECOND ROW MOVED THE DAY AFTER THIS TABLE WAS MEASURED, and it
moved for the engine only — which is the distinction the row now carries.**

✔ **THE MECHANISM EXISTS.** `body.can(verb)` and `body.fits(height)` are
published conditions as of 2026-09-04, and a `gated_by` is an authored condition
LINE, so a wall may read `body.can wall_climb` or `body.fits 32` directly.
Verified 2026-09-04 by running them, not by reading: the four
`a_wall_may_be_gated_on_*` arms pass, including the capability and body-size
ones. ⇒ *"Why can I go there now?"* — this page's North Star question — **has a
mechanism behind it.**

⛔ **AND NO SHIPPED LEVEL USES IT**, which is why this row is `◐` and not `✔`.
`scripts/authored_route_gates.py` measures the whole authored corpus of route
gates at **three walls**, two of them gated, both on the same story flag; and
five of ten published conditions — `body.can`, `body.fits`, `custody.is_held`,
`encounter.cleared`, `world.switch_on` — are authored nowhere at all. ⇒ The
product criterion is judged by PLAYING, and a player cannot yet meet a door that
opens because they learned to climb.
⚠ **That is a content question with a filed answer pending** (question 55), and
this page should not pre-empt it: the honest reading is *"the vocabulary is not
unused because authors chose flags, it is unused because the world has almost no
gates at all"*.

⚠ **What this page said before, kept because the correction is only legible
beside it:** *"nothing gates a route on a body capability … a capability changes
what a body CAN DO and never what the world will LET IT PAST."* True when
measured on 2026-09-03 and false one day later. ⇒ A dated engine-acceptance
table goes stale at the speed of the engine, and this row was the fastest-moving
one on the page.

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
