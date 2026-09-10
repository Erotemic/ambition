# Accepted control and body execution — writer map and production fixtures

**A4's hold, verbatim: "first map writers and select production fixtures."** This
page is that, and nothing else. No type has moved and no split is proposed here;
the frontier says the hold is released by the enumeration, so the enumeration is
the deliverable.

Measured 2026-09-10 against `966351e25`. Re-derive before acting:

```bash
python3 scripts/measure_state_writers.py --domain control --sites
```

## The four responsibilities, kept apart

The frontier names them in this order — *"accepted driver relation, input
projection, live body execution and custody reconciliation"* — and they are four
different questions. A merged "who writes control state" list hides the finding
in each one.

| responsibility | component | sites | outside its defining crate |
|---|---|---|---|
| accepted driver relation | `DrivingParticipant` | 8 | **8** |
| input projection | `ActorControl` | 24 | 22 |
| live body execution | `BodyKinematics` | 38 | 28 |
| custody reconciliation | `InCustodyOf`, `BodyCustodySettled` | 4 | 4 |

⚠ **"Outside its defining crate" is not the interesting column for A4 and saying
so matters.** The packet's destination is *"coherent logical actor/control modules
in the same package first"* — an internal regrouping. `DrivingParticipant` is
defined in `ambition_characters` and written nowhere in it, so every row reads as
foreign while the real owner is `control/authority.rs`. Read the per-crate rows.

## What the map says

⭐ **THE ACCEPTED RELATION ALREADY HAS ONE OWNER, AND IT IS NOT THE CRATE THAT
DEFINES THE TYPE.** Six of the eight `DrivingParticipant` writes are in the
monolith and four of those are `control/authority.rs` alone (`try_insert` and
`try_remove`, twice — vacate and reclaim). The other two are a bundle field
(`avatar/bundles.rs`) and match activation (`character_runtime/
match_activation.rs`). The remaining two are `ambition_abilities`' test support
and `ambition_demo_twintrack`, which seats its own lab twin.
⇒ There is no missing narrow claim/result value here, which is what the packet's
"New abstraction: none until the writer map shows one" was waiting to learn.

⛔⛔ **`InCustodyOf` HAS TWO WRITERS IN TWO CRATES, OVER TWO DISJOINT
POPULATIONS.** `ambition_held_items` inserts and removes it for an ITEM's holder;
`actor_monolith::body_custody::project_body_custody` inserts and removes it for a
BODY's custodian (riders, limbs, possessed actors). Four sites, no overlap, and
`possession.rs:268` already carries the rule in prose: *"do not write
`InCustodyOf` at the possession site."*
⇒ **Whether that is one fact with two writers or two facts sharing a component is
A4's question**, and it is the one place in this map where "one authority per
fact" is genuinely in doubt. Both writers are correct today; the risk is that the
shared spelling makes a third writer look reasonable.

⚠ **`BodyKinematics` IS NOT A CONTROL FACT AND ITS 38 SITES SHOULD NOT BE READ AS
ONE.** Ten of the foreign rows are `mut_param` seams in `shared_tangle` —
helpers taking `&mut BodyKinematics`, which is a borrowed road rather than a
separate one. The frontier already says *"generic motion remains at its
established body owner"*, so this row is context for A4, not work.

## Production fixtures for A4's acceptance list

**A fixture that would still pass if the seam moved is not a fixture for this
packet.** The discriminator is how it composes: a test that hand-adds the systems
it wants names the functions directly and cannot notice one falling out of the
production schedule; a test that drives `Platformer2dSimHarness` or
`build_visible_app` runs the real composition and can.

| acceptance bullet | fixture | composes | catches a seam move |
|---|---|---|---|
| human → possession → brain → human handoff | `possession_end_to_end::a_player_can_possess_drive_and_release_an_actor_end_to_end` | `Platformer2dSimHarness` | **yes** |
| action continuity | `possession_end_to_end::attack_while_possessing_starts_the_possessed_actors_melee_not_the_home` | `Platformer2dSimHarness` | **yes** |
| input reaches the driven body this frame | `possession_end_to_end::possessed_actor_reads_this_frame_slot_input` | `Platformer2dSimHarness` | **yes** |
| mount / dismount | `player_pilots_mount_end_to_end::a_player_pilots_a_mount_end_to_end` | `Platformer2dSimHarness` | **yes** |
| removal of a controlled body | `player_pilots_mount_end_to_end::a_dead_mount_rebuilds_its_riders_brain_through_the_real_schedule`, `carried_item_crosses_rooms::a_mount_dying_under_a_possession_leaves_the_player_driving` | `Platformer2dSimHarness` | **yes** |
| rollback over handoff | `rollback_provoked_actor::possession_survives_the_real_rollback_window` | `Platformer2dSimHarness` | **yes** |
| two participants | `twintrack_it::each_seat_moves_its_own_body_and_leaves_the_others_alone` | demo app harness | **yes** |
| custody across a room boundary | `carried_item_crosses_rooms` (18 tests, incl. `the_occurrence_ledger_learns_of_a_driven_body_on_the_tick_it_is_driven`, `a_custody_row_with_nobody_holding_it_is_retracted_before_a_room_can_act_on_it`) | `Platformer2dSimHarness` | **yes** |
| **competing claims** | — | — | **NO FIXTURE** |
| **no double body tick** | `boot_budget::no_system_is_registered_twice_in_one_schedule` | `build_visible_app` | **partial** |

⛔⛔ **COMPETING CLAIMS HAVE NO FIXTURE AT ALL, AND THE BEHAVIOUR IS NOT A PANIC —
IT IS SILENCE.** `control::body_driving_seat` (`control/queries.rs:45`) resolves a
seat's body and, when two entities hold `DrivingParticipant(slot)`, logs an
`error!` and returns `None`: *"refusing ambiguous authority, so this seat drives
nothing until one of them vacates."* Measured by `git grep`: **no test anywhere
names `body_driving_seat`.** It has four production readers —
`abilities/traversal/possession.rs:56`, `control/input_systems.rs:237`,
`control/queries.rs:224` and `ambition_sim_view::local_view.rs:145` — so an
ambiguous seat makes the player's input reach nothing, on every one of those
roads, with a log line as the only symptom.
⇒ This is the acceptance bullet A4 is least able to check, and any regrouping of
`control/authority.rs` moves the code that maintains the invariant this refusal
exists for. **Write that fixture before the move, not after.**

⛔⛔ **AND THE RETRACTION SWEEP IS NARROWER THAN THE REFUSAL IT EXISTS FOR — SO
AN AMBIGUOUS SEAT CAN HAVE NO ROAD BACK.**

`control::authority::project_driving_participant` is the one system that retracts
a stale second holder. It is bounded twice, and neither bound matches
[`body_driving_seat`]'s:

1. **It returns early unless `PossessionState.home` is set**, and `home` is set
   only between a possession starting and the release branch clearing it. The
   function's own comment states the consequence: *"A session that never
   possesses — a versus match whose seat-0 fighter legitimately holds PRIMARY —
   never reaches here at all."*
2. **Both of its branches act on `PlayerSlot::PRIMARY` only.** The release
   branch's sweep is `seat.0 == PlayerSlot::PRIMARY`; nothing anywhere retracts a
   duplicate claim on any other seat.

`body_driving_seat` refuses ambiguity for **any** slot. So the refusal is
general and the repair is not: a second claim on a non-primary seat has no
retraction road at all, and a second claim on PRIMARY has one only during a
possession.

⇒ **A claim nobody sweeps costs that seat its body until something unrelated
removes the component**, and the only symptom is a log line. That is the strongest
reason this packet needs the invariant pinned before `control/authority.rs` is
regrouped: the code that maintains it is the code being moved.

⚠ **MEASURED VS REASONED, because the tempting citation does not hold.** Bound (2)
is measured — read the two branches. What is NOT established is a reachable
production double-claim on a non-primary seat: `ambition_demo_twintrack`'s
`game/ambition_demo_twintrack/src/participants.rs:316` does insert `DrivingParticipant` outside any possession
window, but on `LAB_TWIN_SLOT`, which is `PlayerSlot(1)` — one holder, no
ambiguity. It demonstrates the *unswept insertion*, not the *duplicate*. Whether
any composition can produce two holders of one non-primary seat is open, and
naming twintrack as a competing claim would be wrong.

⭐ **AND THIS IS WHY THE RECOVERY ARM OF THE FIXTURE IS LOAD-BEARING RATHER THAN
CEREMONIAL.** Since nothing sweeps, recovery can only come from the claim itself
going away — so asserting that driving RESUMES on withdrawal pins that the
refusal is a per-tick resolution and not a latch. That is the only property that
makes the ambiguous state survivable. A reader who sees three arms and no reason
will delete the third as redundant.

⚠ **"No double body tick" is covered only at the SCHEDULE level.**
`no_system_is_registered_twice_in_one_schedule` catches the same system
registered twice; it cannot see one BODY ticked by two different systems, which
is the failure a control/execution regrouping would actually produce.

## The instrument, and what it cannot see

`scripts/measure_state_writers.py` prints its own blind spots on every run:
rollback codecs / `Reflect` / `serde` restores write without naming the type; a
`mut_param` row does not enumerate its callers; whether a given `ResMut` mutates
is not decided by a text scan; macro-generated writes are invisible. The upper
bound is a SEAL — make the field private and let `rustc` enumerate what stops
compiling.

⛔ **THE TYPE LIST IS THE MEASUREMENT'S SCOPE, AND A WIDE ONE IS A WORSE ANSWER.**
The first version of the control list carried `PlayerSlot`, `ActorControlFrame`,
`ActionSet` and `BodyBaseSize` and reported 132 foreign sites. `PlayerSlot(0)` is
a VALUE that appears wherever a seat is named; `ActorControlFrame` is the inner
value `ActorControl` wraps; `ActionSet` is an ability roster rather than input
projection. The report was describing "code that mentions control vocabulary".
One component per responsibility: 132 → 63.
