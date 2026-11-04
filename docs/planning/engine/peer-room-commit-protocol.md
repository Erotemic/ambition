# Peer room commits without a session hold

## Status and scope

The ordinary-travel simulation path is implemented (`1b1baf65a`). Gates 1–3
pass in the focused default-feature checks. Delayed peers apply on the same
input frame, keep one generation, and replay through an application frame.
Rendered combat and view isolation pass in both participant orders. Broader
feature and tooling checks remain open. Full-party checkpoint restores and
explicit rebuilds retain their barrier.

This change removes the lifecycle hold from an ordinary door crossing in the
current GGRS host. It does not provide independent rollback histories for
separate regions. The requirements for those histories remain in
[architecture reassessment](architecture-reassessment-and-composable-multiplayer.md).

## Replay witnesses

`79826e053` adds four single-host witnesses. They apply the shared room
transaction inside the rollback schedule without a session rebase. They cover
a new destination, source retention, an existing destination, and moving-platform
retirement. All four pass in the default and combined-feature app binaries.
Guard: `rollback_room_transition::a_room_publication_can_be_replayed_without_rebasing_the_session`
and its three companion cases.

The witnesses found a missing collision overlay after root restoration.
`RoomInstanceRoot` now requires that buffer. Normal contribution systems fill
it before actor targeting. Do not snapshot derived collision arrays or add a
second room constructor to repair this case.

The delayed-peer tests now add frame-input agreement, both crossing orders,
preparation failure, changed occurrences, and retained-plan lifetime. Guard:
`ordinary_peer_crossings_apply_on_shared_input_frames_without_holding_or_rebasing`.
They require a real rollback across an application frame and matching confirmed
state. Gate 4's focused default witnesses now pass (`b2e6ad940`). A participant
moves and kills an actor in another live room while preparation waits. A late
attack removes an observed speculative crossing and suppresses its arrival
reset; the control releases one reset. Guards:
`a_peer_keeps_moving_and_fighting_in_a_live_destination_while_another_peer_prepares`
and `corrected_combat_removes_a_speculative_crossing_and_its_arrival_effect`.

Death now withdraws the exact body's crossing on the current rollback frame.
Ordinary application rejects dead or out-of-play members before publication;
explicit restores can still recover them. Schema 423 records this change.
Guards: `a_death_withdraws_its_crossing_on_the_current_rollback_frame` and
`ordinary_travel_refuses_dead_or_out_of_play_bodies_before_publication`.
Preparation no longer waits for a committed cover (`dbc8f0cf2`). Both hosts
move the committed payload out of the preparation slot. `LoadPresentations`
holds each owner's foreground, activity, result, and input scope. Each cover
has its own clock and view. Session retirement clears its derived loads.
Delayed peers commit both crossings while both receipts remain held, with
one arrival reset per crossing. Guard:
`an_old_cover_does_not_block_the_next_peers_preparation_or_commit`.
The visible host also prepares and commits another participant's crossing
while the first destination remains covered. Guard:
`a_second_crossing_prepares_while_the_first_destinations_frame_is_not_presentable`.
GPU captures pass for both participants and separate rooms. Guard:
`another_participants_load_draws_beside_a_cover_that_is_still_settling`.

Gate 5's rendered combat and timing witness passes (`dc303e611`) in both participant orders, with damage and one arrival reset.
Guards: `another_rooms_loading_does_not_block_gamepad_combat` and
`another_rooms_loading_keeps_combat_visible`.
[Measurements](../../../dev/ambition_dev_measurements/summaries/2026-11-04-independent-room-loading-combat.md)
separate raw-event response, commit cost, preparation, and cover retirement.

The provoked-source fixture now stages hostile policy before its rebase
(`72419da35`); its four module arms pass. Guard:
`a_provoked_wounded_body_survives_the_real_rollback_window`.
Keep the Hall generated. Target-derived standing can change when no foe exists.

Broader feature and tooling checks remain open. The full feature job must pass
its 40 GB disk preflight before it can run.

## Source findings

Ordinary peer travel applies in `GgrsSchedule`, before gameplay. It reads that
frame's verdicts, not the latest confirmed host summary. Every handle must state
the same admission, captured preparation, content identities, and frozen plan.
The exact retained plan uses the shared room application. A missing or changed
promise is a protocol error. A matching failure or disconnected required handle
ends the operation on the shared input timeline.

`PendingLifecycleCommit::applied` retains the admission, application frame,
preparation checksum, and arrival room in clone snapshots. The peer sends a
presentation receipt through the existing confirmed-effect journal. Replay
replaces speculative batches. Confirmed delivery retires only the matching
loading presentation. Shared publication does not write host loading state.
Schema 422 includes the applied receipt.

Checkpoint restores and explicit room rebuilds retain the confirmed hold and
rebase. Ordinary travel does not enter that barrier. Both use the same room
constructor and transaction.

`PeerVerdict` identifies an operation by its recording frame, admitted intent
checksum, and captured preparation checksum. The intent includes its checkpoint key. Replies for a replaced intent cannot prepare or
fail its replacement, even when both use the same frame. The wire identity is
8. Guards: `verdicts_for_a_replaced_intent_do_not_prepare_or_fail_its_replacement`
and `a_peer_waits_for_the_replacement_intent_even_when_its_frame_is_unchanged`.
An older capture cannot prepare or fail its replacement. Guards:
`verdicts_for_an_older_capture_do_not_prepare_or_fail_the_current_capture` and
`the_same_intent_on_a_corrected_branch_needs_its_own_preparation`.
Replies also carry both content identities and the frozen plan digest. Agreement
requires stated content and equal plan data on every handle. The commit checks
its local plan against that agreement. A mismatch publishes `Failed` through
the input stream. Wire identity 8 carries 168 bytes, including the captured preparation checksum. Guards:
`preparation_requires_the_same_content_and_frozen_plan` and
`peers_refuse_different_room_plans_under_the_same_content_identity`.
The host retains each offered plan and its exact captured preparation outside
that transaction. Commit checks the current content binding, session, source
room, and captured preparation. Publication also checks the consumed occurrence
and mint inputs before it builds a destination.
The loading transaction's presentation tail can remain after the commit.
Neither a render deadline nor that tail can select
the simulation frame of the next peer commit.

Occurrence and mint continuity are now captured on the admitting simulation
frame, after lifecycle producers and durable mirrors. The session root retains
`CapturedRoomPreparation` through registered clone snapshots. Host preflight
requires that exact admission and includes the captured value in cache reuse.
A missing pinned restore records an error. Schema 413 includes this state.
Guards: `room_preparation::tests` and
`lifecycle_snapshot_tests::room_preparation_inputs_survive_the_registered_snapshot_path`.

Publication and prefetch reuse now check the same consumed continuity inputs
(`69929efb2`). The plan records queried authored identities, including suppressed
records, all destination reinstatements, and the mint recipes it used. Plan
identity and reuse use the same canonical encoding. A relevant change refuses
publication before room retirement. Unrelated occurrences permit reuse.
An existing live destination uses its live state and does not validate an unused
construction plan. Schema 420 records the change. Guards:
`continuity_rechecks_authored_and_suppressed_records_without_other_room_dependencies`,
`continuity_rechecks_foreign_arrivals_and_their_positions`,
`continuity_rechecks_the_recipe_of_a_reinstated_runtime_mint`,
`a_changed_destination_continuity_refuses_before_retiring_any_live_room`, and
`a_live_destination_uses_live_occurrences_instead_of_the_unused_construction_plan`.
The delayed-peer witness now covers a real destination pickup consumed after
admission capture (`438be9428`). Both peers refuse the stale plan and keep the
source room and both bodies. Guard:
`peers_refuse_a_destination_occurrence_changed_after_preparation_capture`.
This witness uses the ordinary shared timeline without a rebase. The live-room
combat witness also changes its destination population before arrival.

Checkpoint restores now carry their key in the pending and withdrawn intents
(`ce283f8f0`).
Admission takes the slot and key together. A refusal spends no sequence.
Preparation reads the key directly; it does not infer a restore from equal
intent values. Transaction reuse, authorization, failure, cancellation, and
retirement compare the admission. Guards:
`restore_admission_is_atomic_and_refusal_spends_no_key` and
`cancelling_a_restore_preserves_an_equal_intent_from_another_admission`.

Clone snapshots retain the local scope in the key. The shared checksum uses
its peer-stable projection. Schema 411 records this change. Guard:
`restore_peer_checksums_compare_admissions_without_local_scope_values` and
`pending_withdrawn_and_applied_restore_keys_survive_the_registered_snapshot_path`.
There is no second canonical codec for this local state.
A checkpoint restore still uses the current peer barrier.

Construction-plan identities bind the complete resolved rows and relations,
constructor and placement schemas, retained character and catalog services,
consumed continuity inputs, and resolved moving-platform starts. Character publications
share immutable data versions; a later edit cannot change a retained plan.
Guards: `peer_plan_identity_binds_retained_services_without_local_history`,
`a_retained_cast_keeps_its_definitions_after_a_new_publication`, and
`payload_identity_binds_constructor_schemas_with_equal_rows`.

`RoomConstructionPlan::peer_identity` requires both stated content identities
and excludes local activation counters. Local plan IDs add those counters for
cache use. Peer replies carry this identity and the captured preparation
checksum. Do not send the local plan ID as
proof of peer preparation.

Ordinary travel now preserves current and requested session time scale
(`199313527`). An explicit rebuild can reset the clock only when no unrelated
rooms remain live. Both hosts use the shared finalizer. Schema 421 records the
change. Guard:
`travel_keeps_the_shared_clock_scale_and_a_full_rebuild_resets_it`.

## Retained plan lifetime

`PromisedRoomPlans` owns each offered plan and its captured inputs. The peer
uses that exact plan after the loading receipt retires. A later readiness pass
cannot replace a promise for the same admission and preparation. Session reset
releases the old timeline's promises.

Each published local input records the frame that can reference its plan,
including input delay. After a real save, the lower bound of retained snapshots
is at least the smaller of the saved frame and the confirmed frame. The saved
frame is required for sparse saving. The checksum-only witness does not move
this bound. A lockstep peer has no saved history.

Release a promise only when it is not the selected pending operation and that
bound has passed its last input frame. No render deadline or cache policy can
release it. Guards:
`a_promise_survives_replay_and_sparse_saves_until_its_last_input_is_unreachable`,
`a_pending_promise_is_immutable_and_outlives_the_saved_floor`, and
`a_peer_keeps_its_promised_room_plan_after_the_load_receipt_retires`.

## Operation identity and preparation

The simulation admits an intent and assigns its operation key. Use the session
timeline identity, recording frame, and deterministic admission order. Include
the stable subject identity, source room instance, destination definition,
arrival, and operation kind in the operation's canonical identity. Use the
existing passage admission order; do not use Bevy query order or a local load
sequence. A replacement intent on another rollback branch must not accept the
old branch's verdict.

Each preparation offer binds that operation to the same admitted content
identity and immutable construction plan. A local content epoch is not a peer
content identity. A mismatched content identity fails preparation. It must not
cause either peer to build a different destination under the same operation.

Bind mutable construction prerequisites to identified simulation state. If
those facts change before a new room is built, validate or select them again on
the shared timeline. Local preparation time must not select an occurrence's
position, custody, or presence. Cover this with a peer witness that changes an
occurrence while the other peer prepares the destination.

The host prepares and retains the exact plan before it sends `Prepared` in
the ordinary input stream. That promise lasts while any retained rollback
frame or pending operation can reference the plan. Cache eviction and content
reload must respect that lifetime. A content reload cannot replace a promised
plan in place. Do not run preparation or read local asset readiness during
simulation replay.

Readiness is sent once per peer, even when that peer owns several input
handles. The session topology defines which peer promises are required.
Disconnected peers use the session's explicit failure policy. They do not
silently disappear from the required set.

## Simulation event and commit frame

For each simulated frame, read the operation-specific preparation verdicts
from that frame's inputs. Do not read the host's latest confirmed summary to
choose the commit frame. Prediction can repeat a prior verdict for the same
operation; it cannot create a promise for another operation.

When all required peers say `Prepared` for the same operation and content,
apply the crossing at a fixed point in that frame's simulation schedule,
before systems that consume the new room state. This is the commit frame.
Both first execution and replay use the same rule and application path. A
predicted commit is speculative: later corrected input can remove it. The
local plan must already be retained, including when replay reaches that frame.
A missing promised plan is a protocol error, not a reason to wait locally.

Before commit, a matching `Failed` verdict ends the operation on the shared
input timeline. Failure takes precedence over readiness in the same frame.
A withdrawn intent cannot commit. An obsolete verdict cannot authorize its
replacement. After an operation ends, later verdicts for it have no effect.

Record the applied operation and frame in rollback state. The same operation
cannot transfer a body twice on one branch. Replaying the frame after a restore
must reproduce the same transfer and room identities. Do not increment the
session timeline generation or reset the shared simulation clock for a door.

## State and effect ownership

The shared room application owns the transfer. Preserve the body's stable
identity and use the destination's ordinary construction path. There is no
separate player, enemy, local, or peer room constructor.

Rollback must restore the live room population, body membership and custody,
room identity allocator, destination actors and features, and all mutable
mechanical state. It must restore the source when retirement was speculative.
An immutable construction plan can supply derived state. Rebuild that state
before a simulation system can read it. Do not add a second world snapshot
format beside the existing domain registration seam.

Emit external effects through the existing confirmed-effect path. Replaying
the transfer must not repeat a sound, durable write, or other external effect.
Presentation observes the committed operation and room state. Its cover,
settle deadline, and GPU work do not authorize simulation. A slow destination
view may remain covered while unrelated bodies and rooms continue to advance.

Separate preparation lifetime from the local presentation receipt. A cover
that is still settling must not prevent a later admitted peer operation from
being prepared or applied. Retire simulation state and retained plans only
after the rollback horizon no longer needs them.

## Implementation and behavior gates

1. Exercise the shared room application inside a rollback schedule. Restore
   from after a crossing to before it, then replay through it. Cover a new
   destination, an already live destination, source retirement, actors,
   moving platforms, collision geometry, and stable body references. Compare
   mechanical state and check for duplicate or missing residents.
2. Add operation and content identity to preparation inputs. Pin the new wire
   identity. Exercise a replaced intent, stale readiness, content mismatch,
   failure, and two local handles on one peer. Retain immutable plans through
   rollback and release them after the safe horizon.
3. Use that input event to apply ordinary crossings inside the simulation.
   Replace the hold and rebase for this operation only after the first two
   gates pass. Keep the full-party restore barrier.
4. Drive two peers with delayed and corrected inputs. Make the uninvolved
   participant move and fight in another room. Require unchanged timeline
   generation, matching confirmed checksums, and continued mechanical progress
   during preparation and commit. Cover delayed and failed preparation, both
   crossing orders, and rollback before and through the commit frame.
5. Render the two-controller main-game crossing. A delayed destination covers
   only its participant. The other view continues to move and fight. Confirmed
   arrival effects occur once. Report input latency and commit cost separately
   from preparation and presentation time.

These gates replace the ordinary peer door barrier. They do not establish
regional authority migration, independent histories, or durable cross-region
transactions. Those remain requirements of the wider multiplayer design.
