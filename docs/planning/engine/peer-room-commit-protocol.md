# Peer room commits without a session hold

## Status and scope

Design for MULTIPLAYER-SEAMS slice 6. This protocol is not implemented.
Keep the current peer hold and timeline rebase until the behavior gates below
pass. A full-party checkpoint restore can keep that barrier.

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

These are room-application witnesses. The peer input, delayed-preparation,
failure, and corrected-input gates below remain open. Keep the current peer
hold until they pass.

## Source findings

The current `lifecycle_commit` in `ambition_platformer2d_rollback_ggrs` applies
the room transaction outside `GgrsSchedule`. It holds the simulation from the
frame after the intent, waits for confirmed preparation verdicts, and starts a
new timeline after the commit. These steps give both peers the same baseline.
Removing only the hold is unsafe. The existing
`a_peer_commit_with_no_freeze_runs_on_two_worlds` witness detects that failure.

The room state is partly ready for a reversible commit. The core registers
`RoomGeometry`; the world registers `RoomSet`, `LiveRoomInstance`,
`LiveRoomDefinition`, and `MovingPlatformSet`. Those declarations do not prove
that room publication, actor construction, retirement, and their references
can survive a rollback across a crossing. Prove that behavior first.

`PeerVerdict` identifies an operation by its recording frame and the admitted
intent checksum, which includes its checkpoint key. Replies for a replaced intent cannot prepare or
fail its replacement, even when both use the same frame. The wire identity is
5. Guards: `verdicts_for_a_replaced_intent_do_not_prepare_or_fail_its_replacement`
and `a_peer_waits_for_the_replacement_intent_even_when_its_frame_is_unchanged`.
Content identity and preparation inputs remain open. The host keeps one current
preparation transaction. Its presentation tail can
remain after the commit. Neither a render deadline nor that tail can select
the simulation frame of the next peer commit.

Room preparation also reads the occurrence ledger when the host runs its
preflight. That read can select different inputs on peers at different frames.
The new protocol must select those inputs from identified simulation state.
An existing live destination uses its live state; moving actors there must not
force a new room-construction promise each tick.

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
`pending_and_withdrawn_restore_keys_survive_the_registered_snapshot_path`.
There is no second canonical codec for this local state.
A checkpoint restore still uses the current peer barrier.

The current construction-plan ID includes local content epochs. It is useful
for local cache and staleness checks. A peer promise needs a projection of the
same frozen plan with peer content identity in place of local epochs. Do not
send a hash of the local plan ID as proof that peers prepared the same plan.
Actor row summaries also omit execution inputs, such as staged positions.
They are diagnostics. The shared plan projection must cover complete payloads
and the frozen catalogs that constructors read.

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
