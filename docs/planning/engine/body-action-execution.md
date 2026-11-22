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

Use the resulting budget for movement, move timeline/charge/flow and, when
present, floor recovery. Replace the raw world intervals at both body
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
and the start of hitstun across the hold. The uninvolved fighter or stage
object must continue. Repeat the same controlled sequence with a human seat;
the gameplay clocks must agree.

Exercise one expiry step at a non-identity proper-time scale and one
room/global freeze selected by Ambition. Check that hitstop still expires,
stun spends only the released interval, and authored world freeze survives.
A prior test with only manually seeded timers or global freeze cannot prove
this local crossing.

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
A getup attack enters the normal move owner at its permitted cut. A resolved interrupting reaction,
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
| ACT1 | Narrow actual-reaction follow-on to CR1; no CR2 dependency | Ambition hard reaction/landing and Smash buffered moves | Fixed-size permission projection at current acceptance cuts |
| ACT2 | Existing time/room owners; can proceed beside ACT1 | Smash local hitlag, Ambition reaction/time effects | One step budget; identity path for an unheld body |
| ACT3 | ACT1/ACT2 | One Smash tech and getup roll | One active episode; no per-frame allocation or roster scan |

Use existing body, combat and animation tools. Add only the trace fields
needed to see acceptance, occurrence, clock, interruption and completion.
Tests should protect the repaired crossings; visual and controller review
decide feel. A selected row, a spawned effect or a first boarding frame is
not proof that the associated action completed.
