# Ambition open-world roadmap — world first, story over reality

⚠ **THE TRACKED CRITERION IS NARROWER THAN THE STATED ONE (noticed 2026-09-05).**
The milestone list says *"acquire materially different traversal**/interaction**
capabilities"*; the status row drops `/interaction` and has only ever been
assessed on traversal. That is not a wording nit: **`interact` IS an ability
verb** (`AbilitySet::interact`, `platformer2d_core/src/abilities.rs:157`), so
`body.can(interact)` is a publishable gate today and an interaction capability
could be acquired and gated exactly as a traversal one is.

⇒ Either the criterion means both halves and this row is under-tracking one, or
it means traversal alone and the list above should say so. ⓘ It also bears on
[decision #58](../awaiting-maintainer-decision.md), which asks whether the body
gate family is about CAPABILITY or about STATE — the answer changes what
"acquire" can mean for the interaction half too.

⭐⭐ **MEASURED 2026-09-06, AND IT MOVES THE QUESTION: NO AUTHORED GATE NAMES A
BODY CAPABILITY AT ALL — neither half.** Across every shipped `.ldtk`, the entire
authored condition surface is **six `gated_by` values, all of them the same quest
flag** (`bob_field_survey_received`). Zero `body.can`, zero `body.fits`, zero of
any other condition field. `sync_authored_gated_lock_walls` reads exactly that
field, and all six ARE `LockWall` entities — three in `intro.ldtk:alice_relay`,
three in `intro.ldtk:gate_stack_lower`. ⇒ **the gated-lock-wall machinery is LIVE
with six authored customers**; what is absent is not the mechanism but any gate
whose condition is a body capability rather than a quest flag.

⇒ **So the row is not under-tracking interaction relative to traversal; it is
tracking a criterion the authored world does not yet exercise in EITHER half.**
The capability vocabulary is real and publishable — `body.can` resolves,
`AbilitySet::interact` is a field (`platformer2d_core/src/abilities.rs:157`,
verified), and the gated-lock-wall machinery is built and tested — but no author
has written a gate against it. ⚠ That is a CONTENT gap, not an engine one, and it
is the kind that reads as an engine gap from a status row.

⚠ **What this does NOT settle**: whether the milestone means "the player can
ACQUIRE materially different capabilities" (they can — the dive, the blink and the
portal gun are all authored pickups) or "the WORLD gates on them" (nothing does).
Both readings survive the measurement, and the wording is the maintainer's to
pick — but the choice is now between two things that have been counted rather than
two that have not.

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
| acquire materially different traversal capabilities ⚠ (the criterion above says traversal**/interaction**; this row has only ever tracked the traversal half — see the note below the table) | ◐ **the ENGINE pins it since 2026-09-04; no shipped level authors it** — `a_wall_may_be_gated_on_what_the_body_can_do` and `a_wall_may_be_gated_on_the_body_being_small_enough_to_pass` (`gated_lock_walls/tests.rs`), see below. Since 2026-10-01 the gate is per actor (Q54, `GATE-PER-ACTOR`): the wall is open for the body that can and solid for the body beside it that cannot (`a_body_gate_is_open_only_for_the_bodies_that_satisfy_it`) |
| move/hold/equip/drop persistent objects | ✔ `an_object_in_your_hands_survives_a_replay_and_is_not_re_authored` (both retention legs) |
| alter world mechanisms, return to the changed state | ✔ `a_lever_left_on_is_on_when_you_come_back` (2026-09-29): Interact on `switch_lab`'s lever, out to the hub and back, and the save and the rebuilt switch both read on. The save is a switch's one authority since `561055516`. ⚠ The lever did nothing until that day: its authored action `ToggleFlag` had no reader. |
| encounter persistent and spawned actors | ✔ the encounter suite, plus `a_spawn_request_on_the_bus_becomes_a_body` |
| save/reload without losing instance/location truth | ✔ `loading_a_save_builds_the_room_a_re_entry_builds` and `a_relocated_occurrence_is_suppressed_by_a_load_and_by_a_re_entry_alike` |
| navigate enough that tooling can reason about routes | ▢ open — the navigation frontier |
| separate from another participant into another room | ◐ **the simulation half since 2026-09-30** (OW1 cuts 6–7; A3 in [`multiplayer.md`](multiplayer.md)): `a_door_crossed_by_one_player_leaves_the_other_players_room_live`, and each room's fights, switches and items run in their own room. Since 2026-10-01 each seat crosses its own doors (`the_second_player_goes_through_a_door_of_his_own_room`), and one player's door load or conversation does not stop the other player's room (`a_door_one_player_takes_does_not_stop_the_other_players_room`, `a_conversation_in_one_room_does_not_stop_the_other_players_room`). Since 2026-10-01 each player also has a view of their own room while they are apart: a second view opens and closes with the separation (`a_second_view_opens_while_the_players_are_in_two_rooms_and_closes_when_they_meet`), frames its own player (`each_view_frames_its_own_player_while_two_rooms_are_live`), and its camera draws only that room (view half V1–V5 in [`open-world-runtime-and-residency.md`](../engine/open-world-runtime-and-residency.md)). Fx (V2f), the visuals that ride a body (V2g), gravity zones and shrines (V2h), the blink ring (V2i), the LDtk level (V4b) and parallax (V4c) are each room's own too. ⚠ Open: health bars, debug overlays and the world labels read the sole live room, and the HUD, banner and music are session-wide |

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
✔ **Re-run 2026-09-05 and every figure below still holds** — same 5 of 10
unauthored, 24 total authored uses (2 route gates + 22 dialogue lines). Two of
those numbers were also reached independently that day from the other direction:
a hand count of `<<if boss_cleared(..)>>` gates found THREE, and
`items/wallet_conditions.rs` documents `can_afford` being called TEN times. The
census and the two spot checks agree, which is worth more than any of them alone.
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
