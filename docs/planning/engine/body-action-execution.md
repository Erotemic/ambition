# Body action execution: permission, time and recovery

**State:** PLANNED, 2026-11-22. Source audit at
`34b4ab0b7f4f5a950a52e6031423e76b5e75f6ac`. No build, match, visual
capture or performance measurement was run for this review.
**Selection:** ACT1/ACT2 are shared correctness work under
[ARCHITECTURE-COMPOSITION](../queue.md#architecture-composition--link-owned-facts-and-preserve-causal-boundaries).
ACT3 is a later floor-recovery customer under
[Smash's execution order](../demos/smash-parity-inventory.md#17-current-implementation-order).
It does not raise Smash above the main Ambition work.
**Related owners:** [combat](combat-model.md),
[hit reception](combat-reception-and-injury.md),
[effects and clocks](actor-effects-and-modifier-composition.md),
[movement](unified-movement-kernel.md), and
[pose continuity](../demos/smash-expressive-fighter-poses.md#clip1--carry-the-beat-through-the-existing-clip-request).

## Decision

An action request, permission to start, an action in progress and its picture
are different facts. Keep their owners. Make the boundaries between them
explicit.

The common contract has three parts:

| Part | Owner and rule | Consumers |
| --- | --- | --- |
| Current permission | Derive it from the body's current reaction, guard, movement and relation facts at the cut that accepts an action | Movement input, ordinary move admission, boss or scripted starts |
| Execution interval | Resolve the part of this simulation step that the body may spend on an action; keep timer expiry on its declared clock | Movement, move playback, charge and flow, recovery |
| Accepted episode | The domain that starts a reaction or maneuver owns its occurrence, elapsed time, completion and interruption | Simulation, brain observations and pose requests |

These are small typed values and domain operations. They are not one global
body-state enum, an action graph, a property bag or another scheduler. A body
can be hurt, carried and executing a permitted move at the same time. Each
owner must still answer its own question.

This matters to Ambition now. Its ordinary move starts cross the same
reaction gates; Scout uses the same landing and recoil presentation; local
time effects use the same body and move clocks. Smash adds a strong customer
for optional floor recovery. Neither game needs to inherit the other's
stocks, percent formula, blast bounds, camera or complete move vocabulary.

## Source evidence

The following are source findings. The proposed tests below have not been run.

| Source and symbol | What the current path establishes |
| --- | --- |
| [`apply_post_hit_input_gates`](../../../crates/ambition_combat/src/attack_support.rs) | Reaction, guard and landing locks filter a local movement `InputState` |
| [`trigger_moveset_moves`](../../../crates/ambition_combat/src/moveset/mod.rs) | Ordinary and buffered starts read raw actor control plus selected movement gates, but do not read `BodyCombat` |
| [Actor integration](../../../crates/ambition_platformer2d_actor_monolith/src/body/actors/update.rs), [movement adapter](../../../crates/ambition_platformer2d_actor_monolith/src/body/movement.rs) | The adapter filters movement input, then republishes the actor control frame; that frame is not the filtered input |
| [Combat schedule](../../../crates/ambition_platformer2d_runtime/src/combat_schedule.rs) | Move triggers run before `resolve_aerial_landings` charges landing lag |
| [Hit reaction](../../../crates/ambition_combat/src/hit_reaction.rs) and [move teardown callers](../../../crates/ambition_combat/src/moveset/mod.rs) | Ordinary hit reaction does not retire the victim's active `MovePlayback`; capture, guard, clank, landing and leaving play have their own teardown paths |
| [`advance_move_playback`](../../../crates/ambition_combat/src/moveset/mod.rs), [body step](../../../crates/ambition_characters/src/actor/body_step.rs) | Move time uses proper time and room freeze; body hitlag separately stops movement. The move query has no body-hitlag input |
| [`decay_reaction_timers`](../../../crates/ambition_characters/src/actor/body.rs), [reaction schedule adapter](../../../crates/ambition_platformer2d_actor_monolith/src/body/reactions.rs) | Hitstop expires in unscaled step time. The current thaw calculation subtracts that duration from proper time |
| [Single-room clock policy](../../../crates/ambition_platformer2d_runtime/src/sim_clock/time_control/mod.rs) | A primary driven body's hitstop can stop the world independently of the room's `world_freezes_on_hit` rule |
| [`tick_knockdown`, `resolve_getup`](../../../crates/ambition_platformer2d_core/src/movement/knockdown.rs) | A tech or getup clears prone/tumble state, grants invulnerability and assigns velocity; it has no separate action duration |
| [Movement facts](../../../crates/ambition_platformer2d_core/src/movement/facts.rs) | The visible getup kind is present only while `getup_invuln_timer` is positive |

A useful existing visual test has a deliberate limit:
[`smash_reaction_poses`](../../../game/ambition_app/tests/smash_reaction_poses.rs)
manually cancels the move and seeds reaction state before it checks the pose.
It proves routing after that setup. It does not prove that a real strike
interrupts the move. Preserve the routing test and add the missing crossing.

## ACT1 — Accept and interrupt actions at the owning cut

### Failure

A body can have a hard reaction or landing lock that prevents movement input
while an ordinary or buffered move still starts. A move already in progress
also lacks the ordinary hit-reaction teardown that its reaction picture
implies. Masking one controller's input cannot repair either boundary.

The landing case has an additional ordering risk. A new move can be considered
in Trigger before Prepare charges lag for the aerial that just landed. Merely
adding a read of `landing_lag_timer` to Trigger would leave this contact-tick
gap.

### Implementation

1. Put a small typed body-action permission operation beside the existing
   reaction/input policy. Read current `BodyCombat`, guard state, movement
   facts and relevant relations. Keep the policy distinctions visible:
   hard recoil, soft hitstun, landing, shield stun/drop/break, held reactions,
   helplessness and maneuver commitment do not have identical permissions.
   Do not reduce them to one `busy` Boolean.
2. Use that operation at each start or spend cut. Movement still receives its
   permitted axes and verbs. Ordinary moves, buffered moves, cancels and
   native boss or scripted starts must ask the same body-level question.
   Boss or automatic starts retain their explicit execution policy; they need
   not pay human-input affordability or require a button press.
3. Settle a just-observed landing and its old move before a new ground move is
   admitted. Keep autocancel and edge-cancel policy in their current owners.
   Preserve legal jump, tech, out-of-shield and soft-stun exceptions. Do not
   apply a cached permission computed before a later contact changed the body.
4. Authorize a locomotion escape before it spends motion or clears a state.
   Tumble currently clears on an attack press before attack acceptance, and
   jump/burst cancellation is considered after movement. A refused action must
   not gain those accepted-action side effects. Distinguish locomotion allowed
   beside a continuing move from a locomotion escape that cancels it; do not
   forbid every jump that has no cancel entry.
5. Complete the narrow [CR1 outcome](combat-reception-and-injury.md#cr1--finalize-one-native-hit-outcome)
   with the actual reaction decision needed by interruption. Accepted damage
   alone is insufficient: armor can accept damage without interrupting, and
   a zero-damage reaction can interrupt. A guarded hit has its own consequence.
   The interrupt-relevant result must be unconditional gameplay data: current
   `BodyReactionOutcome` is diagnostics and becomes `()` without
   `causal`. Do not make gameplay depend on that feature or import all
   diagnostic fields.
6. When the resolved reaction interrupts a move, retire that occurrence through
   `cancel_move_playback`. Remove its live windows and stop future timeline
   events, flow and owned motion before their next execution cut. Do not undo
   already settled contacts or delete an independently owned projectile that
   the move has already released.
7. Preserve `ActorControl` as intent and `BodyActionBuffer` as the pending
   press. Refusal does not spend the buffer or resources. Acceptance spends
   them once. Buffer expiry remains explicit and must not be inferred from a
   missing animation.

Do not read `connected_hit` to decide whether the victim was interrupted.
That fact belongs to the attacker's move. Preserve the victim's interrupted
occurrence at reception, so a deferred result cannot cancel a replacement
move. If reception settles inline, do the teardown there; do not add a second
persistent reaction journal solely for this operation.

The pure permission value must not add a combat dependency to the movement
kernel. The body adapter supplies the resolved input or restriction. Queries
may share a helper without storing a second authoritative permission component.

### ACT1 first slice done: a hit retires the move it interrupted (2026-11-23)

Steps 5 and 6 above. `hit_reaction::reaction_interrupts` decides, from the
facts `apply_body_hit_reaction` reads, whether an accepted hit interrupts:
a launch or an installed status does, while armor that absorbs the hit, a gust
and a damage-only tick do not. Both reactions share one armor test
(`armor_absorbs_hit`). Each receiver asks this before the reaction runs and
captures the victim's playing move use. The home receiver reads it in its own
body query; the actor receiver reads it from the move query its system already
holds. The receiver passes that use to `BodyHitReporter::connect`, which carries
it as `ResolvedBodyHit::interrupted_move` (gameplay data in every build, not
the `causal` outcome). `mark_move_playback_resolved_hits` retires that playback
through `cancel_move_playback(.., Interrupted)` only when the instance still
matches, so a verdict read after the victim started another move cancels
nothing. A lethal hit and a boss hit report no interruption: death and the boss
conductor own their own teardown. Already settled contacts and released
projectiles are untouched. Guards: `a_reaction_interrupts_only_when_it_launches_or_holds`
(rule), `an_interrupting_reaction_ends_the_victims_move_and_only_that_use`
(occurrence), `a_launching_hit_ends_its_victims_move_and_armor_keeps_it`
(actor receiver, armor control).

**Ordinary move starts ask the body's locks (2026-11-23).**
`attack_support::body_action_lock_timer` is the one answer for "do this body's
locks take its actions away": `BodyCombat::hard_lock_timer` (recoil, landing
lag, sleep, the held statuses, the ledge trump) beside the guard's break,
shieldstun and drop lag. The movement input gate and `trigger_moveset_moves`
both read it. The trigger refuses before resolution and before the buffer is
spent, so a press still inside its window starts once when the lock ends.
Hitstun alone does not refuse, which keeps Ambition's fight-back rule; a
stricter game will need a declared policy here rather than a second gate.
Guard: `a_press_during_a_hard_lock_waits_in_its_buffer_and_hitstun_alone_does_not_refuse`.

Landing lag is one of those locks, so most of step 3's contact-tick gap is
closed: on the landing tick the aerial is still playing and refuses a
replacement, `resolve_aerial_landings` then charges the lag, and from the next
tick the lock refuses. What remains is an aerial whose own cancel window
permits a move on its landing tick; that start still precedes the landing.

**A ranged use retains its selected source (2026-11-23).** At
acceptance `start_move` binds `MovePlayback::ranged_source`: the weapon the
move brandishes for itself (`MoveSpec::equips`, resolved through the held-item
table at that moment), else the body's own ranged action. The emitter stamps it
on the `Ranged` `MoveEventMessage` beside `move_instance`, and
`dispatch_move_events` builds the shot from it. The late brandish and
`ActionSet` lookup are gone, so a weapon replaced during windup cannot fire under
the old admission, and a move admitted with no source fires nothing. Live aim,
worn modifiers, `shot_damage` and the charge ladder still resolve at the fire
beat as before. The source is in the playback checksum (schema 534). Guard:
`a_ranged_move_fires_the_source_it_was_admitted_with` (A admitted, B swapped
in during windup, A's numbers leave). This proves source retention, not that
the prospective brandish supplied admission and spend. The
[implementation follow-through](#act1-follow-through--resolve-the-request-before-spending)
closes that distinction and the final-event payload. Also not yet done:
interrupting the use when
its admitted source's live dependency (a dropped brandish, a removed form) is
gone; today the bound shot still fires.

**Boss starts ask the same locks (2026-11-23).** `trigger_boss_attack_moves`
refuses a start while `body_action_lock_timer` (or the bare hard lock for a
body with no guard) runs; the `BossAttackIntent` stands, so the move starts on
the first unlocked tick. No button, buffer or affordability is asked of a boss.
Scripted and brain starts reach the move runtime through `ActorControl` and so
through `trigger_moveset_moves`; outside test fixtures, no other production
road constructs a `MovePlayback`. Guard:
`a_locked_boss_starts_its_intended_move_only_when_the_lock_ends`.

**The attack kit shares affordability and fallback selection (2026-11-23).**
`moveset::MoveAdmission` holds what a start is decided on (the body's locks,
recovery charges, held fact, refire floor, live-shot room and meters).
`affords` is the four affordances both trigger roads ask, and `offer` returns,
borrowed and without spending, the affordable selection: the pressed move,
its authored `when_refused` variant, or nothing. `MoveAdmissionReads` (a
`SystemParam` beside it) builds one for a body from the reads its movement
clusters do not carry. The actor adapter's `attack_kit_of` takes it, so the
candidate names the selected affordable variant, with its frame data and
recovery route. Contextual cancel eligibility still needs the requested
identity, as the follow-through below establishes. A refused affordability
offer uses the new
`ActionLegality::Refused`, which the fighter's options already exclude by
reading only `Now`. To stay under Bevy's parameter limit, `tick_actor_brains`
folded the mount carrier read into its read-only `bodies` query. Guards:
`an_offer_names_the_move_the_trigger_would_start_and_spends_nothing` and
`a_locked_bodys_kit_offers_its_moves_as_refused`.

**The observed use names its move (2026-11-23).** `OwnMoveFact.spec` is the
playing use's own `Arc<MoveSpec>` handle, cloned from `MovePlayback.spec` by
the actor adapter, and `OwnMoveFact::move_id` compares it with an offered
candidate's `move_id`. A fallback, another buffered move or a
posture-dependent selection now reads as the move it is, not only as a new
`instance`. The fact is no longer `Copy`; the fighter borrows it. No reader
changes behavior yet: REC1 is the customer that follows the started move or
replans when it differs. The projection is the playback's handle itself, so
the offer test above is the identity crossing.

**An attack acts out of a tumble only through its accepted move (2026-11-23).**
Step 4's attack half. `tick_knockdown` no longer ends a tumble on the bare
attack press; the trigger calls the kernel's `MotionModel::act_out_of_tumble`
at both start cuts, after the move is accepted, so a locked, unaffordable or
unbound press leaves the tumble and the knockdown it lands in. The operation
refuses while the launch still holds the body helpless. The trigger runs
after the kernel's step, which decremented that window, so the acceptance
can act out one tick earlier than the kernel's own read did. The jump
act-out stays the kernel's own verb. Guard:
`only_an_accepted_move_acts_out_of_a_tumble`.

Still open: that cancel-window remainder of step 3; the jump half of step 4
(a tumble ends on a jump press the kernel then cannot spend, and a move's
jump/burst cancel is considered after movement).

### An offer and an accepted action must name the same move

The deeper source review found a second admission consumer. The actor
adapter's `attack_kit_of` calls `legality_of`, which checks only playback and
cancel windows. It publishes the primary move's geometry and recovery route
as `ActionLegality::Now`. Actual admission also checks resource meters,
recovery charges, held permission and weapon readiness. It can select the
one-hop `when_refused` variant. Goblin's charged dive and uncharged fallback
are an existing customer. A forecast of the primary move is not a forecast
of that accepted variant.

Expose a non-spending action offer from the existing combat admission owner.
Give it the same semantic request and current body facts as acceptance. It
returns the resolved prepared move, or a typed refusal, using the same body,
cancel, affordability and fallback rules. A body-level permission is one
input to this operation; it is not the complete offer. Keep the existing
one-hop fallback limit and do not invent fallback on every kind of refusal.

Use the resolved move's frame data and recovery contract in the existing
attack-kit projection. Keep static geometry prepared; read changing
availability at the current observation cut. Borrow prepared values or
handles instead of cloning a move program for each AI candidate. The offer
does not reserve resources, spend a buffer, turn the body or promise that a
later request will succeed. Acceptance rechecks current facts and commits
its existing costs and side effects once.

`OwnMoveFact` currently contains only occurrence and contact. Add compact
prepared move identity to that existing observation. REC1 must observe the
move and control relation that actually started. A new occurrence alone
cannot distinguish a fallback, another buffered move or a posture-dependent
selection. A matching definition also does not prove exact press causality
when identical requests coexist. Reuse accepted-request identity where it
already exists; otherwise follow the actual supported action and replan.
Do not add a durable button-receipt journal for this customer.

This projection is knowledge of the body's own capabilities. It does not
expose an opponent's hidden state or bypass the controller's normal input
path. The actor adapter composes the combat-owned operation; the character
brain does not gain a dependency on combat execution or a copy of its gates.

**Witness:** use the authored charged/uncharged fallback through the normal
request path. The offered move's identity and recovery data must agree with
the move accepted under unchanged facts. Change the bank or body restriction
between observation and acceptance; require a valid new result and no spend
from the old offer. Reuse the existing fallback/admission tests and add only
the missing projection crossing.

### A committed emission retains its admission basis

**Planning refinement, runtime source `6d100f9d9e0d`, 2026-11-22.**
[`MovePlayback`](../../../crates/ambition_combat/src/moveset/mod.rs)
already retains an `Arc<MoveSpec>` for the whole use; `MoveCharge` also
retains the selected charge policy. Preserve those native owners. They do
not yet capture every dependency of a later `Ranged` event.

At move acceptance, the trigger/cancel roads check the current
`ActionSet.ranged` readiness and live-shot limit, and `start_move` arms its
refire floor. Later, `dispatch_move_events` resolves the then-current ranged
source or the move's brandished weapon. It sends
[`RangedCommitment::CommittedMove`](../../../crates/ambition_characters/src/brain/action_set/mod.rs)
with a move occurrence, but no selected weapon binding. The
[projectile consumer](../../../crates/ambition_platformer2d_actor_monolith/src/body/brain_effects.rs)
therefore skips the new source's readiness checks. A form/equipment change
can retain the move while replacing that source. The old occurrence answers
whose shot this is; it does not prove that the selected weapon was admitted.

A bounded witness uses A with no live-shot limit, B with a limit of one,
and one existing body-owned weapon shot. Accept a ranged move under A,
replace the effective source with B during windup, then reach its fire
event. Current dispatch can emit B under A's admission. Replacing it with
no ranged source instead skips the promised event. This is a static source
counterexample, not a reproduced play session.

For the first affected ranged-move slice, retain the selected emission
source and its admission basis with the accepted use. Resolve a move's
prospective `equips` brandish when establishing that basis: capturing only
the body's pre-start default weapon would break the gun-sword behavior the
dispatcher deliberately supports. Keep the prepared mechanical source or
bounded native binding available for the use and its rollback lifetime.
Do not retain an entire actor snapshot or clone its complete repertoire.

Keep the binding times explicit:

| Fact | Binding rule |
| --- | --- |
| Move program, charge policy and occurrence | Retain from accepted start, as the native playback already does |
| Emission source and readiness already spent | Bind to the accepted use, including its authored brandish; an unrelated replacement cannot inherit that admission |
| Aim and physical muzzle placement | Keep the existing live/captured-aim precedence and current body/attachment geometry |
| Declared per-fire equipment modifiers and move `shot_damage` | Preserve their existing trigger-resolution order; changing admission binding does not freeze all modifiers |
| Later projectile birth and required live custody | Keep native geometry/admission and lifetime checks; a commitment is no immunity to interruption or missing physical dependencies |

At a form, equipment or hand transition, the action owner must keep the
admitted binding, explicitly admit a replacement basis, or interrupt that
use through its native teardown before another dependent event executes.
The first slice can retain the admitted source where its live dependencies
remain valid and use the existing interruption policy otherwise. A product
that wants a continuing action to adopt the new source must state and admit
that transition. Equal move IDs or payload values are not that decision.
Scope the commitment to the gates actually discharged; do not turn it into
permission to bypass all later checks.

Settled move costs and recovery/refire spends stay with their accepted
operation. Rebuilding a component neither refunds them nor pays them again.
Any payment for an explicitly new commitment goes through its own native
acceptance. A future resource-plan change cannot reinterpret an old paid
role through its new binding. A retained operation that still needs a future
payment must have a compatible live-bank binding, or take its declared
rebind/refusal/interruption path; keeping the old immutable plan alone does
not keep a removed live resource available.

Do not group all unfinished work as a paid action. Native
[`BufferedShot`](../../../crates/ambition_projectiles/src/state.rs) is an
unspent retry: `Firing::fire` rechecks cooldown and resource admission when
it fires. `StoredMoveCharge` preserves elapsed hold seconds and deliberately
clamps them to the selected resume policy. Keep those native continuation
rules and independently released projectile lifetimes. A pending input
charge has a separate
[binding-withdrawal rule](participant-action-system.md#same-body-binding-replacement-is-a-local-handoff).

**Delivery and witness:** bind one accepted ranged emission at the current
start/dispatch boundary and remove its implicit latest-weapon substitution.
Exercise the A-to-B windup change and a source removal through the native
repertoire/form path. Observe the old admitted source, an explicitly admitted
replacement, or an owned interruption; never B under A's admission or an
unexplained missing shot. Include one authored brandish and the existing
live-aim/modifier behavior as controls. Restore across the composition cut
with the same operation binding. This is a small ACT1 continuation correction;
it can land before the broader linking program and needs no new transaction
framework or roster-wide test campaign.

### ACT1 follow-through — resolve the request before spending

**Open residuals from static review of `1bef25200e00`, 2026-11-23.** The
[implementation review](implementation-review-2026-11-23.md#combat--resolve-the-attempt-and-retain-the-emitted-use)
gives the production traces. Keep the landed source retention, move identity
and interruption work. Complete the native boundaries below.

**Selection and admission.** `trigger_moveset_moves` and
`MoveAdmissionReads::admission` compute live-shot room from `ActionSet.ranged`
before selecting a candidate. `start_move` then resolves its `equips` weapon,
but spends the refire interval passed from the old body source. The shipped
Admiral's pistol uses 1.1 seconds and its drawn gun-sword explicitly uses
0.0 seconds; retaining the gun-sword does not make the pistol's spend correct.

The offer has a second ordering gap. Production checks a cancel window
against requested move X before choosing an affordable `when_refused` Y.
`attack_kit_of` chooses Y first and checks that window against Y. A window
that permits only X can execute Y while the forecast says it is blocked.

Resolve one native attempt with distinct requested binding/cancel names,
selected move, and selected ranged source. Preserve production ordering:
contextual permission for the request, then candidate-specific affordability
and the bounded fallback. Resolve each candidate's prospective brandish
before its weapon gates. Use the same source for live-shot admission, refire
spend, forecast frame data and accepted playback. Keep the body's existing
refire state and policy; no per-weapon cooldown map is implied.

The offer borrows prepared values and spends nothing. Actual acceptance
resolves against its current mutable facts before teardown or payment. Do
not treat a previous offer as permission. Route the touched grab projection
through this owner too; its separate `capture_candidate` path currently
bypasses `MoveAdmission`. Remove the projection's independent cancel/fallback
ordering and candidate-independent weapon Boolean. Do not fix only the
fallback-ID call while leaving another copy of the selection rule.

**Emission after completion.** A `Ranged` event at `duration_s` is valid.
`advance_move_playback` emits it, then removes the finished playback before
the chained dispatcher runs. The event keeps its source and occurrence,
but dispatch still rereads `shot_damage`, charge fraction and captured start
aim from the current playback. An occurrence check cannot recover a removed
value. Stamp those bounded use facts in the ranged event while its emitter
has them. Keep the order: source plus move damage override, current worn
modifiers, then charge scaling. Keep live fire input ahead of captured aim,
and current muzzle/body geometry. Remove the three late playback lookups;
do not extend playback lifetime or retain a whole actor to serve an event.

**Delivery:** finish shared selection/admission first; the emitting-use
payload can land independently. Extend existing native crossings with a
prospective brandish whose cadence/limit differs from the body source, an
X-only cancel to affordable fallback Y, and a valid endpoint shot carrying
damage, charge and captured aim. The reverse Y-only window must still refuse
the X request. Keep the existing windup-swap and live-modifier controls.
These are distinct facts at native phases, not one permanent universal action
object. No new executor, actor compiler or broad test campaign is required.

### Remove

Remove independent start predicates after their consumers use the common
policy. Remove retrospective cancel decisions after their authority moves
before the physical spend. Keep distinct reasons for refusal. Remove any
manually seeded interruption from the new production witness, not from a focused pose-routing
fixture that intentionally starts after the reaction. Do not add another
player/CPU split or a guard in every move handler.

### Acceptance

Use one real hit during an active move, then one press during hard reaction,
with the normal combat schedule. Require the old move's future windows to
stop and the new move to remain unspent while forbidden. A buffered press
that is still live starts exactly once when permitted; an expired press
starts nothing. Preserve separately authored hold/repeat behavior.

Use armor as a non-interrupting control. Include a landing-tick request that
must wait for its authored lag, and an autocancel control that need not wait.
The Ambition home-body/PCA fixture in
[`a_hit_on_the_player_freezes_the_match`](../../../game/ambition_app/tests/a_hit_on_the_player_freezes_the_match.rs)
is a real-hit starting point. Preserve Ambition's permitted attack after hard
recoil ends if its softer hitstun policy allows it. Smash's stricter policy
must use the same operation with its own rules.

Keep one rollback witness over interruption and subsequent acceptance where
the existing harness supports it. Do not create a roster-wide test campaign.

## ACT2 — Resolve body action time once per step

### Failure

Per-body hitlag currently stops travel without supplying the same interval to
move playback. A frozen attack can advance windows, charge or flow while the
body is still. The single-room primary-body clock shortcut can hide this in a
human match and leave a different result in a CPU-only match.

The timer calculation also mixes units. A hitstop remainder expressed in
unscaled seconds cannot be subtracted directly from a proper-time interval.
Paralysis uses proper seconds and needs its own conversion. Adding more
`if timer > 0` checks after each owner has already decremented its timer
would produce different answers at expiry.

Ordinary actor and home-body integration also receive world/room intervals
without reading `ProperTimeScale`, while move and reaction clocks do read
it. This is a second current customer of the same budget: a body under a
local time effect must move and act on the selected rate.

### Implementation

Extend the existing reaction-step operation to return the step's execution
budget before it mutates expiry timers. The budget distinguishes:

| Interval | Required meaning |
| --- | --- |
| Unscaled step interval | The existing simulation-step snapshot used to expire hitstop; no new wall-clock read |
| Body proper interval | World/room policy and body time scale applied once |
| Action interval | The remaining interval after the applicable hitstop or held-reaction portion |
| Presentation sample | The owner's elapsed time or phase used to draw this episode |

Resolve holds in their declared units. With only hitstop, an unscaled step
`u`, hitstop remainder `h`, and constant proper-time rate `r`, the
available proper time is `r * max(0, u - h)`. It is not
`max(0, r * u - h)`. If another hold uses proper seconds, convert it to
the common interval before taking the union. A zero proper-time rate needs an
explicit zero-action case, not division by zero. Preserve the existing
tick-granular contact model; this is not a new continuous event solver.

Use the resulting budget for movement, move timeline/charge/flow,
`BodyPoseClock` and, when present, floor recovery. The physical pose clock
in `advance_body_pose_clocks` currently advances without local hitlag. It
feeds authored hurt profiles, semantic rigs and landmarks, so fixing the
rendered picture alone leaves gameplay sampling on a different beat.
Replace the raw world intervals at both body
integration entry points. Keep their current ordering and one-time event
consumption. State the activation cut for a new hold created after the
budget was sampled. In the current schedule, a hit resolved after move
Prepare affects subsequent execution; do not undo movement or contacts
already settled. An earlier effective hold must be observed at the next
applicable execution cut through the owner. A stale entry budget cannot
override it, and consumers must not each invent their own remaining interval.

Hitstop expiry must keep advancing on its existing unscaled
clock, or a freeze would prevent its own end. Keep control sampling and SDI
on the clocks their rules require. Do not freeze every timer because one
action is held: statuses, invulnerability, input buffering and leases each
retain an explicit owner policy. A saddle or other external pose owner can
suspend ordinary displacement while a permitted move keeps running. That
relation is not, by itself, a hold on the body's action clock.

Make room/global hitstop come from the existing resolved-impact and room
policy. Remove the direct primary-body-hitstop shortcut that bypasses it.
Preserve authored Ambition world freeze and intentional observer blink or
transform slow motion. A human seat must not silently change Smash's
per-body freeze into a world freeze.

A small ephemeral per-step value can bridge ordered systems. Rebuild it at the
same cut after rollback; it is not another independently restored clock or
event log. Render updates sample accumulated owner time/phase. They must
not add this fixed-step budget once per rendered frame; that would change
playback at 60 versus 144 Hz. For a body with no local hold, action time is
the existing proper interval. No dynamic clock graph, map lookup per timer or callback framework
is needed.

### A paused action still has a live state

Setting playback delta to zero is not the complete fix. The current
`advance_move_playback` can still emit sustain effects, traverse instantaneous
flow nodes and fire an unfired event at time zero. A blanket early return
would also skip maintenance of live strike and defense state. Use these
separate rules inside the existing update:

| Operation | Rule during a body action hold |
| --- | --- |
| Input observation | Keep the selected aim, release, DI, mash and buffering rules; observation does not itself accept an action |
| Existing strike, defense and held contribution | Keep the current state present, attached to its owner and reconstructed after rollback; preserve its hit memory |
| Timeline, charge and move-owned cadence | Spend only the released action interval, according to that operation's clock |
| Contact-enabled move flow | Retain the occurrence's contact latch; take the continuation at the next eligible action update |
| Native contact already accepted | Settle it at its current receiver cut; do not defer ordinary damage or reaction until the attacker's flow resumes |

A charge hold is different from a body action hold. It can spend positive
action time on charge while its timeline remains fixed. Do not decide flow
eligibility from the charge-adjusted timeline delta alone. Preserve existing
input/release semantics without awarding charge for held body time.

Use an explicit endpoint convention at exact hold expiry. The body may
accept an action once permission has returned at that cut, and may emit its
instantaneous entry once. It gets no elapsed action time from the preceding
fully held interval. An already-live held action must not replay entry work.
Room/mode suspension remains a separate permission; a zero delta is not
enough to infer either an accepted entry or an ended action.

### Input-dependent continuation is distinct from an action hold

The `00bf92c22142` [IN5 follow-up](participant-action-system.md#in5--withdraw-input-dependent-continuations-at-an-admitted-cut)
finds a pending `PlayerProjectileState` charge that can miss release during
capture or after possession moves its driver. It remains pending under
neutral input. IN5 proposes cancellation without firing when that charge
loses eligible supply or its accepted driving assignment. That policy is
bounded to this input-dependent continuation; committed `MovePlayback`
still follows its body's native interrupt/completion rules.

Keep input ownership/continuity and the action interval separate. A body
action hold can preserve the controller and selected aim/release
observation while allowing no charge progression. Input withdrawal can end
the pending charge even if simulation time remains positive. A gated zero
`ActorControl`, zero `dt`, or absent ephemeral tick message cannot decide
which case occurred. Consume IN5's admitted interruption and the native
assignment transition at the appropriate owner cut. Do not freeze all
input observation to repair time, or cancel every accepted move to repair
a lost release. IN5 owns source/latch/wire meaning; ACT2 owns time.

### Sustain lifetime and cadence need separate facts

The Officer counter and capture reach use sustain as a level signal:
their window remains present. Apple rain uses repeated invocations to spend
a projectile cadence. The extension host currently gives these handlers
world time times room pace, without body action time. Suppressing sustain
on a held body can withdraw a live stance or trigger an idle reset; keeping
it with the old delta can produce shots while the move is paused.

Retain the sustain occurrence and its live contribution. Carry the selected
action or timeline interval to a move-owned cadence through the existing
technique invocation. A refresh signal does not authorize another elapsed
step. Keep maintenance work distinct from a new imperative attempt, such as
a capture acquisition, and apply that native owner's selected eligibility
at the attempt cut. Do not globally replace `ScopePace` or `Invocation::dt`.
The self-ending time-dilation module deliberately uses world/room time and
can select scale zero. A shark lease, a detached projectile and a target's
status also retain their declared clocks and lifetimes. Use the existing
[effect ownership contract](actor-effects-and-modifier-composition.md#activation-clocks-and-read-cuts).

### Input precision does not own a floor episode

The home-body adapter writes its responsive input interval into
`InputState.control_dt`. The kernel uses that override in `tick_knockdown`,
which spends tumble, prone, getup invulnerability and tech timers together.
Ordinary actor input normally leaves the override zero. Passing action time
to motion alone does not repair this difference.

At the existing kernel seam, separate input observation/leniency from
maneuver progression. Existing tumble/prone progression must use the
selected action interval in ACT2; do not defer that repair until ACT3.
ACT3's duration and pose phase use the same contract. Tech
press lifetime and lockout keep an explicit selected policy. Preserve
precision blink and other deliberate input-clock behavior; do not remove
`control_dt` or move every input timer onto action time.

The kernel also clamps a positive movement interval to 1/30 second. This
packet must state the supported normal-step/rate range. It does not establish
arbitrary high-rate equivalence or solve skipped combat windows. A bounded
substep or event-crossing policy needs its own customer; spawning all skipped
hitboxes at the final pose would not reproduce contacts along the path.

### First slices done (2026-11-23)

A body's action time is resolved once per step. `advance_body_reactions`
reads the reaction timers before they decay and writes `BodyActionStep`,
the share of the step's proper interval the body's action may spend:
`BodyCombat::released_action_share`, which is `released_action_dt` over
the proper interval. `released_action_dt` spends hitstop in unscaled
seconds and paralysis in proper seconds:
`max(0, min(proper - r*h, proper - p))`. Stun and recoil lock thaw on it,
replacing `proper - max(h, p)`; the two agree at `r == 1`.

`BodyActionStep::share` is what each reader multiplies its own proper
interval by. A body held in place reads zero. A body built after the
reaction cut falls back to its post-decay hitlag. The readers are:

- both body integration roads (actor and home), as travel;
- `advance_move_playback`, for timeline, charge and flow;
- `advance_body_pose_clocks`, for the physical pose clock.

On the step a freeze expires, travel, the move and the stun therefore all
spend the same released remainder. N frames of hitlag now give N frozen
ticks. Before this, travel and the move took the whole expiry step while
the stun took none of it.

While the share is zero, a move fires no event and runs no flow node; a
fresh move's entry event fires once on the release step. The live strike
volume and its hit memory are untouched, and sustain keeps emitting as a
level signal. Before this, an attacker froze for the hitlag it earned on
contact while its swing kept running, so the swing ended early relative
to its body.

`BodyActionStep` and OBS3's `ModuleObservation` are declared
rollback-derived components.

The clock ladder no longer has a rung for the observed body's hitlag. A
world freeze on a connect comes only from the room's `world_freezes_on_hit`
policy, armed as `ImpactHitstop` for every seat, CPU or human. A human
seat in a platform-fighter room now keeps Smash's per-body freeze, as a
CPU-only match does. In Ambition rooms a connect still stops the world,
for the room's capped hitlag. A guard deflect and a finishing blow, which
set body hitstop without a resolved connect, now freeze only their bodies.
They also did so in a CPU-only match.

Still open:

- Sustain cadence.
- Floor progression on action time (`control_dt`).
- Ordinary actor and home integration read the room interval without the
  body's `ProperTimeScale`. The share is applied on top of that interval,
  so a dilated body's travel still runs on the room's rate.
- The CPU-only and human-seat Smash acceptance. This VM cannot seat a
  Smash round, so that acceptance has to run elsewhere.

### Remove

Remove separate reconstruction of the held fraction in move, movement and
recovery readers. Remove the primary-hitstop-to-world-clock special case.
Retain the existing time authority, room pace, effect clock declarations and
native-module clock work. This completes a concrete part of the older
[proper-time audit](expressive-move-capabilities.md#status-resources-and-forms);
it does not create another time service.

### Acceptance

Use a real local-hitlag hit in a CPU-only Smash match, where global observer
freeze cannot mask the result. Compare body travel, move time, charge/flow
and the physical pose sample across the hold, then the start of hitstun.
The uninvolved fighter or stage
object must continue. Repeat the same controlled sequence with a human seat;
the gameplay clocks must agree.

Exercise one expiry step at a non-identity proper-time scale and one
room/global freeze selected by Ambition. Check that hitstop still expires,
stun spends only the released interval, and authored world freeze survives.
A prior test with only manually seeded timers or global freeze cannot prove
this local crossing.

At a touched sustain boundary, keep one live defense/reach through the hold
and require a move-owned cadence and latched flow to resume once without
extra progress. Retain the scale-zero self-expiry control. Exercise the home
adapter with a nonzero input-clock override for one floor episode; a core
fixture with `control_dt == 0` does not cover it. Reuse existing fixtures
for these distinct crossings instead of adding a clock matrix campaign.

Report changed intervals and event counts. No frame-time improvement or binary
size reduction is claimed by this plan.

## ACT3 — Give floor recovery its own episode

### Failure and scope

A successful tech currently clears tumble, sets invulnerability and one
velocity, then returns input with only the evade edge removed. A getup clears
prone immediately. The remaining movement step can accept a jump or change
the assigned velocity through ordinary steering and friction. The pose
selector then uses invulnerability as the duration of the recovery.

These are separate problems: action commitment, motion ownership and defense
are represented by one short-lived side effect. Adding art cannot establish
the missing maneuver. The intended duration and cancel policy must be authored;
this review does not import a reference game's frame table.

### Implementation

Extend the existing private movement maneuver state with a floor-recovery
episode. Keep it optional for a body with a floor game. Record the accepted
kind, occurrence, body-relative direction, elapsed action time and resolved
timing policy. The policy states:

- the action duration and permitted exit/cancel windows;
- its motion rule, including braking and response to support loss;
- its invulnerability window;
- the semantic pose phase, projected from the same episode.

Use existing movement/sweep authority for the motion. A roll is not a
teleport or an impulse that ordinary locomotion can immediately replace.
Compose its motion with the body's scale/frame policy; keep existing portal
accepted-path and contact rules. Do not force every action onto a root-motion
curve or every movement model into the axis-swept floor game.

A tech jump, getup attack, ordinary stand and roll may have different rules.
A getup attack enters the normal move owner at its permitted cut. Current
`MovementOp::GetupAttack` has no combat consumer; the FX adapter ignores it,
and the raw attack can independently select an ordinary move. The comment
that combat answers this operation does not establish a handoff. Publish a
movement-selected request with the episode's authority, then let normal
combat admission accept it. Its own recovery lock must permit that declared
exit. Spend the originating press once; it must not also start a second
ordinary attack. The movement kernel must not gain its own strike runtime.
A resolved interrupting reaction,
capture, leaving support and leaving play terminate or transfer the episode
under their explicit policy. Damage through armor or a flinchless contact
does not end it merely because health changed. Do not clear it merely because invulnerability ended.

Publish kind, occurrence and phase to observation/presentation. Keep defense
as a separate projection. The
[clip request](../demos/smash-expressive-fighter-poses.md#clip1--carry-the-beat-through-the-existing-clip-request)
uses that phase; art length does not decide when control returns.
Keep only the episode's necessary state in rollback, with stable occurrence
identity and deterministic interruption.

### Remove

Replace getup/tech-as-an-unowned-velocity-write and
getup-invulnerability-as-an-animation-lifetime. Remove the shared constant
as a proxy for all four meanings once the authored policy is in use.
Keep existing hit, tech-contact and prone-orientation facts.

### Acceptance and selection

Choose one fighter's floor tech and getup roll. Through ordinary input, show
the accepted motion and permission window, an independently shorter or longer
invulnerability window, and one interruption. Check the last recovery step
and first permitted action. Draw the corresponding clip from the same
episode. Include the wall/ceiling distinction when that route is touched.

ACT1 and ACT2 must establish admission and time before this packet promises
a timed maneuver. This packet remains below current flagship work. Its shared
kernel design makes later Ambition adoption possible; that possibility alone
does not make a Smash-only floor polish task a new top priority.

## Delivery and cost

| Packet | Can begin after | First customer | Cost boundary |
| --- | --- | --- | --- |
| ACT1 | Reaction interruption: narrow CR1 follow-on. Emission binding: existing admission/dispatch. No CR2 dependency | Ambition hard reaction/landing, Smash buffered moves, and equipment change during windup | Fixed-size permission projection at current acceptance cuts; bounded source binding with the affected use |
| ACT2 | Existing time/room owners; can proceed beside ACT1 | Smash local hitlag, Ambition reaction/time effects | One step budget; identity path for an unheld body |
| ACT3 | ACT1/ACT2 | One Smash tech and getup roll | One active episode; no per-frame allocation or roster scan |

Use existing body, combat and animation tools. Add only the trace fields
needed to see acceptance, occurrence, clock, interruption and completion.
Tests should protect the repaired crossings; visual and controller review
decide feel. A selected row, a spawned effect or a first boarding frame is
not proof that the associated action completed.

[CLIP3](../demos/smash-expressive-fighter-poses.md#clip3--preserve-source-motion-under-ruleset-retiming)
is a separate preparation correction. ACT2 determines how much selected
action time may pass. CLIP3 determines which source motion beat that time
denotes. The physical pose clock, visual fallback and prepared motion map
must retain their own meanings; none replaces the others.
