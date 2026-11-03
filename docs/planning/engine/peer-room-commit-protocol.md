# Peer room commits without a session hold

## Status and scope

Design for MULTIPLAYER-SEAMS slice 6. This protocol is not implemented.
Keep the current peer hold and timeline rebase until the behavior gates below
pass. A full-party checkpoint restore can keep that barrier.

This change removes the lifecycle hold from an ordinary door crossing in the
current GGRS host. It does not provide independent rollback histories for
separate regions. The requirements for those histories remain in
[architecture reassessment](architecture-reassessment-and-composable-multiplayer.md).

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

`PeerVerdict` currently identifies an operation by its recording frame. The
host keeps one current preparation transaction. Its presentation tail can
remain after the commit. Neither a render deadline nor that tail can select
the simulation frame of the next peer commit.

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
