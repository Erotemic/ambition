# Hosted prediction: native state, time and authority

**State:** PLANNED, source audit 2026-11-22. Runtime source:
`34b4ab0b7f4f5a950a52e6031423e76b5e75f6ac`; planning base:
`4c5853c4c697b6a036e26aed618cef72ce100f15`.
The counterexamples below follow executable source. No new runtime witness,
build, network measurement or performance result is claimed.

**Execution:** bounded follow-up under
[NET-AUTHORITY-EXPERIMENT](../queue.md#net-authority-experiment--compare-hosted-world-replication-with-ggrs).
The [experiment](hosted-world-networking-experiment.md) keeps its completed
measurements. [ISL-3](island-rollback-and-compact-peer-input.md#isl-3-refinement--history-membership-and-scoped-transfer-cuts)
owns transfers between host room histories.
[CW1-CW4](causal-world-commit.md) owns committed world operations.
This page owns the guest's supported prediction and correction contract.
Smash keeps its current single-domain GGRS composition.

## Decision

A guest predicts a declared native unit from an authoritative continuation
at a real simulation cut. That unit uses the ordinary gameplay kernels.
External observations and forecasts supply only the dependencies the
supported unit needs. Executing that branch gives the guest no authority
to settle committed-world operations.

A continuation is the native state needed to resume the selected mechanics.
Position, velocity and facing are one projection of that state. Equal
projections can have different legal next actions. A room snapshot of the
guest supplies retained speculative history; it does not turn that history
into the host's continuation.

The first implementation uses a bounded native composition, ordinary typed
values and existing Bevy ownership seams. “Prediction unit” names its
contract; it does not require a new registry, an authored language or an
automatic ECS dependency graph. Expand support through native customers.

The profile declares both its corrected continuation and the gameplay owners
permitted to influence its speculative advancement. Each influence comes
from that continuation or an admitted external-input/forecast policy.
Other gameplay producers that could influence it must be absent from the
supported composition or remain passive. Unrelated presentation can continue.
The first profile can enforce this through room/composition eligibility and
the existing observation fallback; it does not require a general scheduler.
Importing a richer checkpoint alone does not establish a supported forward
simulation while the rest of the guest room advances without that contract.

## What the existing witness establishes

[`a_guest_predicts_its_own_body.rs`](../../../game/ambition_app/tests/a_guest_predicts_its_own_body.rs)
checks local turn response, five motion floats, correction counts and final
motion after a constant-delay link drains. It proves useful run/jump and
remote-strike cases. It does not compare combat permission, charges, motion
continuation, environment phase or population/assignment changes. Its loop
runs one fixed step per App update.

Earlier projectile, effect and late-join measurements in the experiment used
a prototype guest. The production own-body receipt explicitly leaves its
shot-signature claim unwitnessed. Preserve both scopes; do not reinterpret
their receipts as general production prediction support.

| Current boundary | Source evidence |
| --- | --- |
| Observation used as correction | [`BodyRow`](../../../crates/ambition_hosted_world/src/body_row.rs) has five motion floats. [Reconciliation](../../../crates/ambition_hosted_world/src/prediction.rs) compares those floats, restores the guest's post-tick save, then overlays received motion and current HP |
| Whole gameplay stays awake | [Hosted plugin and guest gate](../../../crates/ambition_hosted_world/src/lib.rs) keep ordinary simulation running for `PredictsOwnBody`; other bodies advance locally between rows |
| Native time can be relabeled | `predict_own_tick` writes `SimTick = wanted - 1` before saving the existing native state under that new label |
| Tick used as publication identity | `reconcile_own_body` rejects `RowTick <= compared_through`, although a host rewind can revise state at the same `SimTick` |
| Received data resides on restored entities | [Room restore](../../../crates/ambition_platformer2d_runtime/src/rollback/room_snapshot.rs) can recreate native entities. [`replay_room`](../../../crates/ambition_hosted_world/src/room_rollback.rs) ignores its remapping report, then correction queries live `BodyRow` components |
| Construction uses current driver | `write_spawn_rows` captures `DrivingParticipant` once; `build_spawned_bodies` uses that seat to choose the home-body constructor |
| Environment starts from local construction | [`MovingPlatformSet`](../../../crates/ambition_platformer2d_world/src/collision.rs) owns platform continuation on the room root. None of the seven production replicated row types imports it |

## The four kinds of data a guest needs

| Meaning | Contract |
| --- | --- |
| Presentation observation | Enough to display an authoritative observation. It may be interpolated; it does not by itself authorize native simulation |
| Native continuation | Canonical state, capability presence and references sufficient to resume the declared prediction unit at its admitted cut |
| External input or forecast | A dependency the unit reads without owning its authoritative transition. Retained inputs keep their cut during replay; unreceived future inputs have a declared forecast |
| Committed outcome | A world owner's accepted result/revision. It enters dependent guest simulation at a retained cut; guest speculation cannot manufacture it |

The same native capability can supply more than one of these values.
Keep their meaning explicit without copying its transition law into networking.
A sleeping guest can consume observations without installing a predictor.
A predicting guest need not simulate every visible autonomous actor.

## HP1 — separate speculative execution from world authority

The source has a concrete authority gap. The
[visible hosted guest](../../../game/ambition_app/src/app/hosted_launch.rs)
uses `Fixed60Hz`. Only the Host branch installs the live confirmation
publisher. `PredictsOwnBody` keeps gameplay and `GameplayEffects` awake.

[World-value admission](../../../crates/ambition_platformer2d_runtime/src/world_values.rs)
holds a room's request because `SimulationIslands` is enabled. Its
`apply_confirmed_writes` then treats an absent `ConfirmedFrameBoundary`
as confirmed and applies that value to the guest's local committed-world
model. The temporary boundary inside replay does not cover forward execution.
The retractor removes pending requests, not already settled values.

This audit establishes local independent settlement on the guest. It does
not establish a write to the server or an on-disk save failure. The absent
boundary rule remains valid for an authoritative offline fixed host.

### Bounded change

The hosted composition must select two independent permissions: which
native work may run speculatively, and which world owners may commit.
Keep domain admission under its existing owner; do not scatter
`if HostedGuest` tests through gameplay systems.

For the current input-driven customer, the host receives input, executes
native gameplay and settles its world operation. The guest may retain a
provisional request/observation where its supported mechanics need it,
but cannot report a host-accepted result from a local write. A received
`HostTick` is neither source finality nor world-domain acceptance.

Do not also send each derived world request merely because the guest
predicted it: the host already executes the input that produces it.
A future direct client-command path needs its own accepted-request contract.
Import authoritative decisions through the existing domain/result seams,
with the retained read cut required by CW3 when prediction consumes them.

Local suppression or waiting for authority is not host admission or host
rejection. It must not fabricate a terminal refusal that sends an authored
callback down a different branch. The first slice can suspend that dependent
prediction until an admitted result is available; state that limited
support explicitly rather than reporting an unanswered request as false.
Retain the existing occurrence/ticket identity while waiting, and define its
disposition when prediction is canceled or re-anchored. Do not accumulate
held requests indefinitely or issue a fresh one on every speculative tick.

Begin with the existing world-value owner. Extend this permission boundary
to another committed owner only when a supported prediction customer uses
it. Cosmetic prediction can retain its own policy; suppressing duplicate
presentation during replay does not establish world authority.

### Proof and removal

Reuse `ISLAND_SOURCE` from
[`scripted_world_values.rs`](../../../game/ambition_app/tests/scripted_world_values.rs).
Its native status callback submits `add_world_count` after crossing an x
coordinate. Compose it in the own-body guest fixture, correct the predicted
crossing away, and inspect the count, reply and change mailbox. A confirmed
host crossing settles once; the authoritative offline fixed host remains
the control allowed to settle locally.

Remove the assumption that waking native gameplay makes the guest a world
committer. Do not change every offline host to an artificial unconfirmed
mode, or use the latest received tick as a substitute for an authority policy.

## HP2 — import a native continuation independent of live entities

An accepted no-launch damaging hit can reset a spent air dodge and arm
hitstop, then return without writing velocity. See
[`apply_body_hit_reaction`](../../../crates/ambition_combat/src/hit_reaction.rs)
and its [native actor-hit caller](../../../crates/ambition_platformer2d_actor/src/body/damage/actor_hit.rs).
A stationary victim can have unchanged `BodyRow` values while its freeze
and next legal dodge differ. `LifeRow` transfers current HP and death
display fields; it does not supply those combat/dodge facts.

Even a later motion mismatch does not repair the missing hit. Current
correction restores the guest's save after the host hit's tick and replays
later local input. That suffix cannot reconstruct a remote hit that was
never part of the guest's accepted history.

### Native owner contract

Define the complete continuation for the first admitted body composition.
The relevant native owners supply the state their next steps actually read:
motion model/support, action and reaction state, charges/resources, and
mechanical shape/pose where those capabilities participate. Include
capability absence when it changes the legal continuation. These examples
are evidence of distinct state, not a mandate to replicate all components.

The continuation is not limited to components attached to the body.
Retained input-interpreter/seat state can affect its next tick too, including
held-frame and buffered interaction state. Use the existing owner-scoped
seat/room slices at the same cut; do not import every global resource.

Use the existing canonical state and native restore ownership. In-process
`RoomSnapshotRows` declarations help locate that state; their local
`Entity` handles and `Any` slices are not a network format. Resolve
network references through stable occurrence identity and the existing
construction binding. Do not create a central concrete-component census.

Capture a coherent native continuation at a declared post-step cut, with
the identity/time contract in HP3. Compare the supported continuation or an
equivalent owner-defined comparison value. A digest can detect a mismatch;
it cannot supply missing state. Begin with a small typed value before adding
comparison/transfer optimizations.

Apply canonical state through its native restore/import seam, then rebuild
derived state before any consumer reads it. Keep authoritative correction
distinct from an attempted local movement; the guest cannot reject the
host's accepted position through a fresh collision attempt against its old
environment. The environment used for continuation must be admitted too.

Ordinary receive must not mutate the predicted unit's canonical state
before this admission. Today `apply_life_rows` writes health/death state
even for the predicted body, and `apply_plan_rows` can install native `Life`.
Route the supported predicted state through its coherent correction cut;
a waiting or refused correction must not have partly applied already.
Keep ordinary row adoption for sleeping/observation replicas.

### Correction input survives native restore

Retain the accepted correction as a value outside the entity population
being restored. Apply it by the resolved native occurrence after restore.
Today, a recreated entity has its registered gameplay rows but lacks the
received `BodyRow` queried by the correction closure. Receipt before
restore therefore does not establish application after restore.

Use `RoomRestore` or the existing construction result to rebind affected
transport attachments. Preserve the cheap in-place case. Do not snapshot
live transport connection internals. Exact Replicon mapping behavior after
recreation was not established by this audit; prove it through the native
link when population-changing prediction is admitted under HP5.

### First slice and proof

Use an accepted no-launch hit on a stationary body with a spent dodge and
declared nonzero hitlag. Assert unchanged motion at the host cut, then
correct native freeze and next dodge permission on the guest. Keep the
existing movement/strike fixture as a control. The profile must state its
complete supported continuation; adding only enough fields to pass this
one witness does not define that profile.

A checkpoint is the smaller first fit for the existing post-tick correction.
Replaying a raw hit request can reconsider armor or invulnerability against
different guest facts. A later accepted-outcome protocol needs its native
identity, phase placement and revision meaning, and an appropriate earlier
restore cut. That is a separate optimization with a concrete customer.

**Remove:** five-float equality as the complete prediction verdict, health
repair detached from the correction cut for predicted state, and reads of
correction payload from a population that restore can replace.

## HP3 — distinguish executed time, publication and finality

The current time jump has a precise failure of meaning. If tick 100 was
last executed and the desired input tick becomes 105, `predict_own_tick`
sets the clock to 104, labels the existing body as `after[104]`, saves the
existing room under 105, then executes one step. Ticks 101–104 did not run.
Native timers and platform cursors did not advance with the label.

The later replay requires a saved/compared entry for its start. Most skipped
ticks have no entry, and a later motion repair does not reconstruct their
missing native work.

There is also a publication problem. Host receive/rewind runs in
`PreUpdate`; row publication runs in `PostUpdate`. An App update with
no new fixed step can revise the host's provisional state at tick T and
publish a new row at T. The guest's `compared_through` rejects it because
an earlier version of T was already compared.

### One correction cut, distinct meanings

| Meaning | Required statement |
| --- | --- |
| Input sequence/stamp | The connection's admitted input order and intended simulation tick |
| Executed simulation cut | The native steps that actually ran, or the authoritative continuation adopted at that cut |
| Publication revision | Which complete host publication supersedes which, including revisions at the same simulation tick |
| Source finality | Which prefix can no longer change; observation of a host tick alone does not prove this |
| Subject and interpretation | The body/assignment occurrence, live-room binding and admitted mechanical content that give the continuation meaning |

Reuse existing identities where they distinguish these facts. A transport
sequence may serve as publication identity only if it names the complete
correction with the required ordering. Do not assume a component change
tick or `SimTick` supplies that guarantee. This audit does not allege a
transport-specific mixed-component bug.

Each accepted correction must be coherent for its declared unit. Retain
its source values before restore; do not fill absent pieces from whichever
live row is newest. A host room identity maps to a guest's local room
instance; local ordinals or an authored room name alone are not a general
cross-App occurrence binding.

A provisional host continuation is authoritative for its current revision
without being final. The guest can predict from it and later accept its
revision. That does not grant permission to commit the provisional world's
consequences.

### Admission outlives the predicted subject

The first fixed-body profile needs positive admission information even
before HP5 supports full assignment or population changes. Keep a small
host-owned status outside the driven entity's lifetime, addressed through
the existing session/connection and seat. It identifies the native subject,
room/history binding, admitted segment/baseline cut and supported profile
or interpretation revision, with an effective ready/re-anchor/unavailable
disposition. These are semantic requirements, not a new profile registry.

Invalidate the admitted segment when a relevant unsupported subject,
capability, content or population change occurs. Retain that break until a
fresh baseline is admitted. Recomputing current eligibility alone can hide
`ready → unavailable → ready` between sends; equal final population counts
or component values do not prove that the old continuation stayed valid.
A bounded segment break is enough; no event journal is required.

Continuing rows from the former body, or their silence, cannot establish
current support. Keep forwarding seat input while prediction is unavailable.
The old observation row alone cannot construct or follow a newly assigned
native body; before HP5 that transition can remain explicitly unsupported
pending a usable native baseline.

### Advance or adopt native time

For a small lead increase, run the missing supported steps through the
ordinary schedule under a bounded catch-up budget and the declared input
policy. Replayed past steps use retained admitted inputs; a gap forecast
uses held levels with no invented presses. If required history/dependencies
are absent or the gap exceeds policy, adopt a coherent authoritative
continuation. Initial prediction needs that real start cut too.

Input-clock estimation can move independently. It cannot advance native
time by relabeling a save. Keep the effective simulation step with its
native time owner; host and guest may hold different numbers of rooms
(HP4), so a raw copy of one global clock tuple is not the full contract.

A received correction has an explicit disposition: applied, waiting for a
required binding/baseline, superseded, or requiring a new baseline.
Do not mark it consumed before discovering that its replay is unavailable.
Suspend unsupported prediction or use the existing observation path while
ordinary input transmission continues. Do not wait for another motion
difference to repair a refused correction.

There is exactly one active input producer. Today `PredictsOwnBody`
disables `send_local_input`; suspending prediction must transfer sending
to the ordinary adapter without a gap or duplicate samples. Keep the
connection's admitted input sequence across this transition.

### Proof and removal

Use focused cases in the existing link fixture:

- After comparing T, revise the host at T through late input, with no new
  fixed step before send. The newer publication must be admitted.
- Force one lead increase with held movement and one native timer/charge.
  Compare executed native state as well as motion at corresponding cuts.

In the fallback assertions, let a native change invalidate the selected
profile and reverse before publication. The old segment must stay broken
until a fresh baseline is admitted; input still sends exactly once. This
checks the conservative boundary without requiring prediction of that change.

Keep stable-lead turn latency as the control. Remove direct tick relabeling
of unadvanced state and the tick-only rejection of revised publications.
No transport replacement or new timing benchmark campaign is required.

## HP4 — admit environmental trajectories and feedback state

[`MovingPlatformSet`](../../../crates/ambition_platformer2d_world/src/collision.rs)
already owns each platform's position, motion cursor, stage and carry delta.
[Platform advance](../../../crates/ambition_platformer2d_world/src/platforms/simulation.rs)
runs before integration. The ordinary
[body integrator](../../../crates/ambition_platformer2d_core/src/movement/integration.rs)
uses its collision projection and swept carry.

A host can advance a room before the guest builds it. Guest construction
starts from the authored platform state. No production row imports the
host's phase, and body correction restores only the guest's old platform
state. Matching content, body position and a tick label does not establish
matching live platform phase.

A position row is insufficient even for an autonomous sweep: the same
position can occur in either direction. A path needs its cursor. A wrapping
platform's travel delta differs from its teleport. Keep those facts with
the existing [platform owner](../../../crates/ambition_platformer2d_world/src/platforms/mod.rs).

### Two dependency laws in one native owner

An autonomous sweep/path can supply an external trajectory. Admit its
native seed/phase and effective clock inputs, then derive its ordinary
collision projection. Retain the samples or seed/updates needed to replay
the selected interval. Future motion beyond received evidence remains an
explicit forecast. A compact seed can be a fast path; no saving is measured.

Use the native evaluator and enough admitted step/phase information to
reproduce its progression. A collision consumer needs displacement over
its simulation interval, including the existing exclusion of wrap teleport
from carry. A correct end pose or a formula from one global timestamp does
not establish that interval's `last_delta`.

A replay attempt uses an identified, fixed external-input history and
forecast policy. Retained seed, policy and time inputs can replace stored
per-tick samples if they reproduce the same inputs. Do not read the latest
mutable network proxy while replaying an older interval. A newly admitted
authoritative revision can replace an old forecast and replay dependent
state; retaining evidence does not make a wrong forecast permanent.

Admit an external driver's revision independently of whether the body is
equal at the anchor. Its already-predicted suffix can depend on the replaced
trajectory. Replay or re-anchor the affected supported interval under the
declared policy; do not retain a compare-body-first early return that hides
a changed dependency. The selected unit already names the relevant driver.

A rider-triggered lift closes a feedback loop: body support changes
`LiftStage`, the lift changes its solid/carry, and the body produces new
support. For responsive prediction of that interaction, include the lift's
native continuation with the body state that closes the loop. Foreign rider
occupancy/input remains a named external input or forecast.

A passive alternative supplies the body kernel with an admitted or forecast
solid trajectory while the guest body does not advance the external lift
owner. State its contact/carry policy. Waking all bodies and platform owners
between received rows does not implement that boundary.

There is one evaluator of the surface per tick. An external trajectory and
ordinary `advance_moving_platforms` must not both advance it, or let local
occupancy mutate a supposedly external lift. A change of prediction role
takes place at an admitted cut with the native continuation it needs.

Choose these roles through the capability's native adapter and composition.
Do not scan the whole ECS for transitive dependencies each tick, duplicate
platform physics, or make every visible actor part of prediction.

### Clock meaning and first proof

[`RoomFreeze`](../../../crates/ambition_platformer2d_shared_tangle/src/lifecycle/room_pace.rs)
and the [time owner](../../../crates/ambition_platformer2d_runtime/src/sim_clock/time_control/mod.rs)
represent slowdown differently with one versus several live rooms. A host
can hold several while its guest holds one. Admit the effective room step/
hold meaning through that owner; blindly copying `RoomPace` or the host's
global clock is insufficient as a general design.

First, let an autonomous platform advance before guest entry. Import the
admitted phase, then prove body support/carry through correction and replay.
Keep the body equal at the anchor of one revised driver input and verify
that its dependent predicted suffix is repaired anyway.
Stationary geometry is the control. One non-unit room-pace case with several
rooms on the host and one on the guest distinguishes the clock meanings.
Next use the existing rider-triggered lift: a correction changes the
supporting condition, and both lift stage and body support/carry must follow
the corrected history.

Reuse the native platform tests and existing two-App fixture. Mounts have
a related feedback shape, but this packet does not require a mount, portal
or navigation campaign before the two platform laws are established.

**Remove:** unsupported local environment continuation from the correction
path, and accidental guest execution of autonomous owners outside the
supported unit. Rebuild derived collision state through the existing owner.

## HP5 — separate native provenance from control assignment

`SpawnRow.seat` currently copies the current driver once and also selects
the home-body constructor. Native possession changes
[`PossessionState`](../../../crates/ambition_platformer2d_actor/src/control/possession.rs)
and claims, then [projects `DrivingParticipant`](../../../crates/ambition_platformer2d_actor/src/control/authority.rs).
The wire carries no equivalent assignment change.

Updating the seat field alone would preserve the wrong conflation.
A possessed actor retains its native construction/capability identity.
Current human control does not turn it into a home body.
Replicating only the derived driver marker would also leave the native
possession/claim state inconsistent.

### Assignment and prediction segments

Admit the authoritative assignment through the existing control owner at a
dated cut. Bind the prediction segment to that native body/assignment
occurrence. Today `GuestPrediction` resets for seat or room changes only;
same-seat, same-room possession can mix histories of different bodies.

The first supported handoff can suspend/re-anchor prediction at the admitted
assignment. Keep connection input sequence and run-ahead state independent
of that subject change. Retain the old segment's meaning for delayed input/
correction, or give it an explicit disposition. A transition predicted
through the cut needs a retained assignment decision; it is later work.

Use existing occurrence identities and the assignment cut. Add an epoch
only where those values would otherwise alias a retained segment. The
observer follows the admitted control result under its existing view policy.

### Population and native construction

A received transport entity is not yet a constructed native occupant.
Use the existing constructor/receipt, pinned native recipe and live-room
binding. Keep initial provenance separate from current form and capability
continuation. Constructor refusal must remain refusal.

After native recreation, use the restore/construction result to rebind the
transport attachment to the actual occupant. HP2's retained correction input
then applies by stable occurrence. In-place correction remains the common
cheap path; unsupported population change can require a fresh baseline.

One ordinary remote projectile is a concrete construction customer.
Production marks `SimId + BodyKinematics` for replication, but the current
spawn row requires `CharacterForm`; an ordinary
[projectile](../../../crates/ambition_projectiles/src/entity.rs) has no native
constructor through that row. Matching a locally predicted shot's signature
does not construct a shot the guest never predicted.

Per-owner spawn counters match only while accepted spawn history matches.
Define the native proposal/adoption/refusal meaning before merging a
predicted occurrence with an authoritative one. Reuse the existing
construction and action receipt; do not add a global allocator or universal
spawn journal.

### Bounded proofs and removal

First use native possession/release inside one room. Both sleeping and
predicting guests must adopt the right driven occurrence without changing
its native recipe. Delay an old-assignment correction across the change.
The first implementation need not predict the possession transition.

For population support, first use one real unpredicted host shot to prove
construction and retirement. Add a locally predicted shot as an adoption
control when the supported profile predicts spawning. Then use an occupant
recreated by correction, followed by another authoritative update/removal.
Assert one native occupant, correct application and retirement. Exact
transport rebinding is a native-link proof, not a claim inferred from adding
a `Signature` component.

**Remove:** current driver as a construction discriminator, seat+room as
the whole prediction subject, and assumptions that receiving a row or
matching an ID proves native construction/adoption.

The implementation agent should also preserve room binding in the ordinary
spawn branch, distinguish initial recipe from current form, and carry the
constructor's real refusal. These are local completions of the same owner;
they do not justify another architectural packet.

## Delivery order and efficiency

HP1 can land independently at the current world-value seam. HP2 and HP3
form the first complete body-correction slice: one bounded native body
composition, static admitted environment, and explicit fallback for
unsupported assignment/population changes. Do not advertise arbitrary
body prediction from the no-launch-hit witness alone.
Existing local room snapshots can remain the retained-history mechanism
for this slice; reducing their footprint is not a prerequisite to defining
the native correction contract.

HP4 adds one autonomous platform, then one feedback lift. HP5 adds native
assignment and population boundaries; its construction/assignment split
also benefits the sleeping guest and can proceed independently.
The host's ISL-3 transfer refinement and CW4 finality optimization are not
prerequisites for proving these same-room guest boundaries.

Keep the efficient gameplay path explicit:

- Reuse native kernels and existing in-place restore.
- Prepare the selected capability support at composition/admission, with
  bounded changes at native membership boundaries.
- Keep passive observations and autonomous trajectories out of unrelated
  actor/AI execution.
- Retain only the correction inputs and history the supported unit needs.
- Use native deltas/seeds where their meaning is established; measure before
  adding hashes, collection swaps or a general data-structure package.

Measure forward guest CPU, snapshot/correction bytes, retained history,
replayed work and visible correction behavior for the selected profile.
Source simplification alone establishes no percentage saving.

The [input cut](participant-action-system.md#input-history-and-body-interpretation)
still supplies admitted requests. Prediction must consume that same stage.
The [construction owner](construction-and-reconstitution.md) keeps native
population, and the [capability owner](capability-and-runtime-composition.md)
keeps optional composition. None of these changes makes a network backend
mandatory for an offline game.
