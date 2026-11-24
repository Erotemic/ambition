# Participant and semantic action system — current residual work

**State:** OPEN, narrow.

The original participant/input migration is largely landed. This plan owns
remaining participant-context and provider-action work, plus the admitted
input and body-interpretation boundaries below.

## Landed model

The current architecture already provides:

- explicit participant/seat identity and device assignment;
- one per-seat control delivery road rather than a privileged seat-zero channel;
- participant-owned input contexts;
- per-seat menu frames and deterministic pause-menu ownership;
- `ambition_ui_nav::ListCursor`/focus/navigation primitives used by current
  menus;
- an action registry and semantic action IDs;
- provider-defined physical keyboard bindings through `ProviderBindings`;
- provider action state installed on participant entities;
- deterministic `SemanticActionPressed { id, participant }` publication;
- context filtering for provider-action presses;
- a global `ControlPrompt` for the current one-screen gameplay presentation.

These pieces should be extended rather than replaced by another input manager.

## P1 — per-seat dialogue/gameplay model — DONE

The context claim carries input ownership, and `stops_the_world` carries the
clock. Dialogue says only the first. Ownership lives in `SeatInputContexts` /
`ParticipantContexts` (`ambition_input::participant`: `DIALOGUE_CONTEXT`,
`owner()`, `gameplay_owned(slot)`), not in `GameMode`. One seat can read a
dialogue box while another keeps running. Pause, room transition and cutscene
still stop everybody. Witness:
`dialogue_claims_the_talker_while_a_pause_still_stops_everybody`
(`runtime/src/input.rs`).

`GameMode::allows_gameplay()` is still `matches!(self, Self::Playing)`. It no
longer decides per-seat gameplay routing. Do not "fix" it by deleting the mode
gate, and do not thread the world-stop flag into another global gate.

## P2 — finish provider actions at composition boundaries

A provider can now declare an action, bind it to a key, and receive a semantic
per-seat press without adding a variant to the engine's closed built-in action
enum.

**2026-11-22 source refinement at `00bf92c22142`:** the publisher is live
in host `Update/InputSet::Route`, but no production gameplay reader of
`SemanticActionPressed` was found. The capability demo explicitly leaves
the final hop to composition. This remains customer-gated work, not a
demonstrated shipped provider-action failure.

A direct host-event-to-effect bridge would not complete the contract.
[`SemanticActionPressed`](../../../crates/ambition_input/src/semantic.rs)
contains action and participant IDs; the
[recorder](../../../crates/ambition_platformer2d_runtime/src/input_stream.rs)
records `ControlFrame`, and
[`PeerInput`](../../../crates/ambition_platformer2d_rollback_ggrs/src/peer_input.rs)
carries that frame plus a publication verdict. A host reader that directly
writes `PulseRequested` could change body velocity while recorded/peer
input remains identical. Registering the resulting body/cooldown state
cannot reconstruct the missing external cause. This is a counterexample
to that implementation shortcut, not an existing bridge.

For the first concrete consumer, decide these at composition:

| Boundary | Required choice |
| --- | --- |
| UI or gameplay | UI-only commands stay host events. Host-origin gameplay requests enter [IN1's admitted input cut](#in1--record-and-replay-at-the-same-admitted-input-cut) before native execution. Simulation-owned AI/domain requests keep their native owners. |
| Existing slot or distinct request | Use an ordinary semantic slot when it expresses the action. Add bounded input vocabulary only when the customer needs a distinct meaning. |
| Subject | Ordinary driving input resolves the unique body assigned to the participant at the stated simulation cut. An explicitly selected object uses the existing scoped occurrence reference and domain checks. |
| Gesture | State the needed press/hold/release/value semantics and bounded coalescing rule. A press-only event cannot express a charge-and-release operation. |
| Definition | Gameplay meaning belongs to the session's selected prepared content/extension binding. Physical key rebinding stays host-local. |
| Acceptance | Repertoire, control holds and native body/domain permission still decide the attempt. Writing an `ActorActionMessage` is not proof that it passed those cuts. |

Open provider behavior is already implemented. Pulse Forge authors a
technique on the Projectile slot and receives pressed/held/released values
through `TechniqueUsePort`
([content](../../../examples/projects/pulse_forge/content/characters.ron),
[module](../../../examples/projects/pulse_forge/portable/src/lib.rs)).
`ActionRequest::Special` and the installed extension executor already
route keyed behavior. Reuse those owners; keep `ActionRegistry` as
metadata, without a second callback registry or per-provider input plugin.
The present one `ProviderAction` specialization also avoids repeating the
input machinery for every provider.

If a new input value is required, preserve the wire owner's fixed-width
contract and apply the wire and recorded-stream owners' compatibility
rules. A changed capture stage or existing meaning requires a new recorded
meaning; an additive field whose old value correctly means neutral follows
the stream owner's existing compatibility rule. Do not
put arbitrary strings, descriptors or an unbounded action vector into each
frame. A compact binding is meaningful only under the same selected
definition on both peers. `ActionTicket.generation` names a caller
lifetime, not a content generation; phase-local extension invocation queues
are not a replay input channel.

Prove the first selected final hop through a real host: capture it, change
driver before admission, exercise the chosen target policy, and replay the
same native outcome. Preserve context refusal and body holds, with an
allowed control. Use the existing Sanic provider-technique replay as a
control for the execution road; it does not prove an arbitrary provider
binding. No new transport work is selected merely because the publisher
exists.

Controller/touch presentation still chooses a finite physical/on-screen
slot. Add binding-authoring schemas only for a real tooling customer.
Do not make `ambition_input` learn actor/body concepts to close the hop.

## P3 — decide prompt multiplicity with the multiview customer

`ControlPrompt` is one global read model describing the primary local gameplay
surface. That is reasonable for one screen, especially for one shared touch
overlay. Split views by live room exist. The banner and the prompt follow the
primary seat. The built-in vitals HUD is per participant (Q150): each view
has a HUD of the body it follows (`ViewHudFacts`), and each other seat on a
shared view has its own HUD there (`SharedViewHudFacts`). The declared HUD
readouts are still one per session.

With several independent local views/seats, one participant may need a different
prompt from another. Do not make `ControlPrompt` plural solely for naming
symmetry. Resolve the desired UI/product shape together with
[`multiplayer-and-multiview.md`](multiplayer-and-multiview.md):

- one shared display may intentionally have one prompt;
- split views may require view- or participant-indexed prompts;
- touch overlays are device/screen policy, not automatically one per participant.

## P4 — keep context vocabulary semantic

Do not add a `VEHICLE` context merely because mounted bodies have different
verbs. Menu/dialogue contexts exist because another surface owns the
participant's input. A rider still drives a gameplay body; the body's action
scheme should change the prompt/repertoire.

Add another context only when ownership/routing semantics actually differ.

Loading/retry and specialized UI contexts should migrate when their schedule and
ownership seams are clear. Avoid introducing a dependency cycle merely to make
all surfaces use the same enum immediately.

## P5 — weapon readiness is a semantic state (Q33)

Ruling Q33 (2026-10-04, [`../maintainer-decisions.md`](../maintainer-decisions.md)):
readiness is a generic semantic state that the engine publishes and
presentation shows: about `ready` versus `recharging/unavailable`, optionally
with progress. A trigger during a cooldown must not look like a successful
shot. Each weapon's or game's presentation chooses the treatment (dimmed or
disabled, a recharge bar, a cue); the engine does not hard-code one.

**Built 2026-10-08, the action road.** `ambition_combat::WeaponReadiness`
is the read model: `Ready`, `Recharging { progress }` or `NoRoom`, from the
fire-rate floor (`RangedRefire`: `remaining`, and `armed`, the length it
was last armed with, which gives the progress), the authored action's
live-shot limit (`ActionSet.ranged.max_live`) and the body's shots in flight.
The floor keeps its own length because a held item arms it with the item's
spec, not the body's action (schema 323). `progress` is `None` only for a
floor with no armed length.
`derive_weapon_readiness` (`ambition_sim_view::control_prompt`) writes it each
tick onto `BodyWeaponReadiness`, which `RangedRefire` requires. It is declared
derived (`derived.weapon_readiness`, schema 322), not snapshotted. The prompt's `ready` bit reads it, so a full weapon dims the
slot as a hot one does. A refused attempt (`body/brain_effects.rs`)
writes `RangedFireRefused { actor, readiness }` and starts no shoot pose,
sound or shot. Held-item discharge fires through the same attempt with the
item's own spec, so its refusal is published too. Witnesses: `a_press_during_the_recharge_is_refused_with_its_progress`
(the control is the same press with the floor spent) and
`readiness_says_what_the_floor_and_the_limit_decide`, and
`a_floor_armed_by_another_spec_reports_that_specs_progress` for a held
item's shot (the control is the body's own shot).

**Built 2026-10-08, the fireball road.** A charge body's press reaches
its fireball (`PlayerProjectileState.spawner`, `ProjectileSpawner`: a
cooldown and a resource meter), not the floor.
`WeaponReadiness::of_spawner` asks the spawner's questions in the order
`try_spawn` asks them: `Recharging { progress }` from the cooldown and
`cooldown_armed` (each kind has its own cooldown, schema 324), then
`NoAmmunition` when the meter cannot pay (`ResourceMeter::can_pay`, the
rule `try_spend` spends by). A refused press
(`projectile/systems.rs::try_fire_projectile`) writes `RangedFireRefused`
with that readiness. `derive_weapon_readiness` reads the weapon the press
reaches, by the predicate the emitter uses
(`action_emission::charge_stream_owns_the_press`), and inserts the read
model on a charge body that has no floor. Witnesses:
`cooldown_blocks_second_fire_in_same_window` and
`out_of_resource_blocks_fire` (the control is the first press, which fires
and is not refused), `a_spawner_says_its_cooldown_then_its_meter`, and
`a_charge_body_carries_its_fireballs_readiness` (the control is a body with
the same spawner that does not charge: it reads its floor).

## Contextual interaction decisions belong to gameplay

The [interaction opportunity plan](interaction-opportunities-and-activation.md)
extends the current Interact customer. Domain availability, target selection
and accepted gesture consumption move below `sim_view`; prompts become their
projection. Input continues to own participant routing and gesture generation.
Preserve raw Interact versus Up shortcuts, fresh-edge versus buffered
eligibility, and each domain's consume cut. A shared attempt does not give
every operation the same gesture policy. Dialogue, mount and lifecycle owners
still admit their own requests. This does not restore the deleted all-button
affordance table or put body/domain concepts into `ambition_input`.

## Input history and body interpretation

**State:** PLANNED from source `34b4ab0b7f4f`, 2026-11-22. These are shared
Ambition/Smash boundaries. They do not reopen the participant migration or
[ACT1/ACT2](body-action-execution.md). Input selects a request; those owners
decide action permission and elapsed execution.

### IN1 — record and replay at the same admitted input cut

The current `ControlFrame` carries several meanings: a device sample, a
tick's admitted input, body-interpreted control, and what remains after a
consumer spends an edge. The recorder and replay reader use different cuts.

| Source | Established boundary |
| --- | --- |
| [Device capture](../../../crates/ambition_platformer2d_runtime/src/input.rs), [GGRS input](../../../crates/ambition_platformer2d_rollback_ggrs/src/session.rs) | Render-rate samples enter a latch; the backend admits its fixed-tick frame before simulation interpretation |
| [`shape_seat_frame`](../../../crates/ambition_platformer2d_actor_monolith/src/control/queries.rs) | Simulation edits both `SlotControls` and `SeatRawFrames` |
| [Player schedule](../../../crates/ambition_platformer2d_runtime/src/player_schedule.rs), [recorder](../../../crates/ambition_platformer2d_runtime/src/input_stream.rs) | Capture occurs after reset consumption, direction gestures and portal shaping |
| [Harness `step_frame`](../../../crates/ambition_sim_harness/src/runtime.rs), [input driver](../../../crates/ambition_platformer2d_runtime/src/input_drive.rs) | Recorded frames return through the ordinary input seam before those transformations |

There is a concrete lost cause. Ambition's
[`apply_player_reset_input_system`](../../../game/ambition_app/src/app/sim_systems.rs)
clears `reset_pressed` in both tables, then requests a room replay. Its
[registration](../../../game/ambition_app/src/app/plugins.rs) precedes the
recorder. The recorded frame therefore lacks the request. No recorder reset
hook supplies a new initial state; `InputStream::push` only replaces a
speculative tail or restarts at an earlier tick. The current replay fixture
uses run/jump/burst, where this distinction does not arise. This is a source
finding; an accepted reset and decoded replay were not run in this audit.

**Contract:** establish one immutable per-seat input value after sample
coalescing and backend admission, before body-dependent interpretation or
consumption. Record that value and inject replay at that same cut. A GGRS
prediction and its later correction each supply the value for their tick;
the existing tail rewrite records the corrected history. A plain driver and
a hosted seat supply the same semantic stage through their existing adapters.

| Value | Owner and lifetime |
| --- | --- |
| Physical/context-filtered samples | Device/participant input; may arrive several times before a tick |
| Admitted seat input | Backend/driver input cut; immutable for that execution of the tick |
| Held-direction and gesture state | Existing deterministic body/participant owners; restored for replay |
| Resolved body control and remaining consumable edges | Current simulation execution; derived from the admitted input and native state |

A reset, interaction or movement guard can alter its working result without
erasing its cause. Reuse existing actor-control values and gesture owners.
Remove writes from simulation interpretation back into raw proposals as
their consumers move. Do not duplicate every table indefinitely or create
a durable button journal. UI-only actions retain their host/UI owner.

The artifact must name its capture stage. Existing post-shaped streams
cannot silently acquire the new pre-interpretation meaning under the same
version. They can remain forensic observations. A replayable segment also
needs its matching initial simulation/content context; this packet does not
make a mid-session input file a complete checkpoint.

**First slice and removal:** publish the admitted value for every backend
and driver at a simulation-side cut, then record it under that execution's
assigned `SimTick`. Merely moving the recorder earlier is insufficient:
latchless hosts currently publish `SlotControls` later, while early readers
can use `SeatRawFrames`. Predicted and corrected executions must both enter
this cut. Do not record at render/device capture or packet receipt, before
the tick executes. Move or split `record_input_stream` there, retain
speculative-tail replacement, and remove the claim that recording consumed
control is sufficient for ordinary re-injection. Keep a distinct
post-interpretation trace only where it answers a diagnostic question. This
is a publication/order correction, not another input service or network
protocol replacement.

**Proof:** extend the existing recorder/decoded-stream/harness fixture with
one accepted manual room replay and one real portal transit with held input.
Check the resulting lifecycle/body state and retained gesture/guard state.
Keep ordinary motion as the control. A final position alone does not prove
that the next tick will interpret the same held input.

### IN2 — resolve the input frame before applying a physical constraint

[`warp_portal_input`](../../../crates/ambition_platformer2d_runtime/src/portal_seat.rs)
maps raw axes through world-space portal normals and removes their inward
component. It does not read the frame mode or the body's resolved frame.
The [frame owner](../../../crates/ambition_geometry/src/reference_frame.rs)
gives a concrete counterexample: under `BodyRelativeStrict`, down `(1,0)`
has side `(0,-1)`. For an exit normal `(1,0)`:

| Raw stick | Resolved world direction | Current raw-axis guard |
| --- | --- | --- |
| `(0,-1)` | `(-1,0)`, inward | Dot is zero; retains the inward input |
| `(-1,0)` | `(0,1)`, tangent | Dot is negative; removes tangential input |

This is a coordinate proof, not a captured gameplay result. Current guard
fixtures use ordinary X-axis input. The shaped axes also feed ordinary
attack direction in [the player translator](../../../crates/ambition_characters/src/brain/player.rs).
A movement protection can thus change another verb's meaning.

**Contract:** resolve admitted input for the uniquely driven body under the
frame policy carried by that input and the body's native motion frame.
Apply a declared held-direction continuation and emergence constraint to
the locomotion channel in explicit local/world coordinates. Existing
`ActorControlFrame` separates locomotion, attack axis and world velocity
target. Keep action aim and interaction gestures with their own declared
policy. Do not infer that all verbs share movement's continuation.

Constrain a command the selected motion law can realize. For an axis-swept
grounded body, local Y can mean descend/crouch rather than a freely commanded
travel velocity. A world-space projection must not invent jump/crouch input
or treat that body as a two-axis flyer. Keep expressibility with the native
movement/control adapter; the old world-X-only check is not its definition.

[`HeldControlFrame`](../../../crates/ambition_platformer2d_actor_monolith/src/control/participant_input.rs)
already keeps held direction through gravity turns. `PortalInputWarp` keeps
a second raw anchor and portal normals. Make these operations compose once:
the continuation retains its reference mapping, and an accepted transit
supplies the map under which that held movement continues. Do not transform
raw axes, then implicitly transform the result again in the body translator.
Keep release, redirection, driver change and mode change as explicit end cuts.

Retain the accepted map or equivalent bounded linear value. Reading the
current global portal convention against old normals is not the map of the
accepted transit, especially under the planned per-connection law. Stick
magnitude is dimensionless; portal scale must not multiply it. Do not bake
body-dependent direction at device capture or read an eased render camera
from deterministic simulation. The existing bounded frame helpers suffice.

**Remove:** raw-axis/world-normal tests, shared input mutation used to guard
only movement, and redundant gravity/portal continuation state when the
selected representation replaces it. Preserve the default world-fixed
behavior and existing held-through-gravity comfort policy.

**Proof:** use the inward/tangent pair above, ordinary gravity as control,
one held stick through a portal and gravity turn, and a directional attack
at that boundary. The expected aim policy must be explicit. Extend current
release/redirection and two-seat cases only where their boundary changes.
This does not reopen the portal geometry solver.

**First slice done (2026-11-23): the guards act on resolved control.**
`warp_portal_input` no longer rewrites the seat's raw stick. It runs after
`ControlledBrainTick` and guards the driven body's `ActorControl`: the
locomotion goes to world space through the body's `ResolvedMotionFrame`, is
warped and stripped there, and comes back. The aim policy is explicit: the
held-stick warp continues locomotion, the flyer's `velocity_target`, the
facing request and an attack aimed by the movement stick; a C-stick attack
is intended to keep its own aim. The emergence strip does not write attack
aim. The current source still infers the aim source from equal vectors and
passes posture Y through its travel projection; the follow-through below
corrects both. `ActorControl` is latched, so the guard
acts only on a frame the translator wrote since its last run (`is_changed`).
Guard: `the_emergence_guard_reads_the_bodys_resolved_frame` (the
inward/tangent pair above for a flyer, and a walker's local Y); the
raw-axis fixtures now read the body's locomotion, and the plain-gravity
strip also checks that the attack keeps its side.

The warp retains the accepted transit's map: `PortalInputWarp.map` is the
orthogonal part of `PortalTransfer::map`, without its scale, in place of the
two normals the convention was read against again each tick.

The gravity-turn composition is proved (2026-11-23): a transit that arrives
in turned gravity maps a held stick once. The translator keeps the hold's
world direction through the turn (`HeldControlFrame`), and the warp maps
that world direction by the accepted transit; neither reads the other's
result. Guard: `a_held_stick_through_a_transit_and_a_gravity_turn_is_mapped_once`
(`portal_seat`, `--features portal`): seat one holds right under the
body-relative mode, and one tick brings a same-wall warp and upside-down
gravity together. Travel and the movement-aimed attack go world left; the
warp in ordinary gravity and the turn alone are its controls. No source
change was needed.

#### IN2 follow-through — retain direction meaning through the guard

**Static review of `1bef25200e00`, 2026-11-23.**
[`tick_player_brain`](../../../crates/ambition_characters/src/brain/player.rs)
knows whether it selected captured aim-stick direction or the movement
axis. `warp_portal_input` later infers that choice from
`attack_axis == locomotion`. A C-stick attack and the movement stick can
both resolve to `(1,0)`; a held reflection then reverses that independent
attack. A different C-stick vector survives. Equality cannot carry the
selected source.

Retain the actual translator choice in the resolved control value, or keep
the guard at a boundary that still owns it. Use a small semantic distinction
between movement-derived and independent aim. Preserve the existing fallback
when an aim-stick qualifier has no valid direction. Do not reinterpret the
raw device again downstream or change wire identity merely to add a derived
control fact. Remove the vector-equality inference.

The same guard converts the walker's whole `(x,y)` into world travel, then
restores its old Y. Restoring Y cannot undo its contribution to the new X.
With down `(r,r)`, side `(r,-r)`, `r = sqrt(1/2)`, exit normal `(-1,0)`
and pure posture input `(0,1)`, the emergence projection produces local
X `-0.5`. The guard returns `(-0.5,1)`: crouch/descend invents side travel.
This is a static valid-frame counterexample, not a measured room symptom.

Project only travel that the current motor can express. Keep posture/descend
outside that physical vector; a flyer still has two travel axes. Apply the
selected attack-aim policy to its own vector, without using the walker's
posture-preserving projection as an aim rule. Reuse the native motor/frame
adapter; no curved-world or gravity-field program is required.

Extend the existing translator-to-guard crossing with a real C-stick press
equal to the movement vector, plus the oblique-frame posture example.
Keep movement-derived aim, free flight and ordinary gravity as controls.
The accepted-map retention remains correct. The
portal-plus-gravity-turn composition is proved above.

**Follow-through landed (2026-11-23, schema 536).** The player translator
records its choice as `ActorControlFrame::attack_axis_from_aim_stick`. It
is true only when an aim-stick qualifier produced a valid vector; the
fallback to the movement axis leaves it false. The control codec encodes
it with every other field. `warp_portal_input` reads that fact; the
vector-equality inference is gone. A movement-aimed attack goes through
the held map whole, as a direction with no posture rule, and a C-stick
attack is untouched.

The travel projection takes only the travel the motor can express: both
axes for a flyer, the side axis `(x, 0)` for a walker. A walker's local Y
is kept as posture outside the guard. A temporary probe against the real
guard gave `(0, 1)` for the oblique pure-posture case (formerly
`(-0.5, 1)`). It also kept a C-stick `(1, 0)` attack while a reflection
turned the equal movement vector to `(-1, 0)`, and the movement-aimed
control still followed the warp. The probe was deleted (bug fixes add no
test).

### Tentative curved-world held direction

[CV4](curved-surfaces-and-local-gravity.md#held-input-needs-an-explicit-continuation-rule)
adds an unselected product decision, not a change to IN2's current comfort
contract. `hold_stick_through_turns` interprets the current held stick in
the retained press-time frame. An unchanged stick keeps its original world
direction; small analog changes still steer in that retained frame. On a
continuously turning planet, that differs from interpreting the command in
the current local frame, even when body-relative mode is selected.

If that scene is selected, state which continuation policy it wants.
Preserve the existing discrete-gravity-flip customer; do not infer policy
from angular change per tick. A policy transition ends/rebases the hold at
the admitted input/control boundary, with the existing history and driver
ownership. Keep camera easing and aim policy separate. Future acceptance
must include a held full circuit, release/repress at another orientation
and the existing flip-zone behavior. A fixed-camera human-only proof is
sufficient; new device routing or a generic input framework is not needed.

### IN3 — coalesce a qualified press as one value

The smaller correction is in
[`ControlFrame::merge_sample`](../../../crates/ambition_platformer2d_core/src/control_frame.rs).
Strength merges by `Smash > Tilt > Auto`, source presence uses OR, and aim
direction comes from the latest aim-stick press. Smash-left followed by
Tilt-right within one tick therefore produces Smash-right, which neither
press requested. A later held modifier can also raise an earlier hint.

Keep the bounded one-attack-opportunity policy, but select a complete winning
press: strength, source policy and any edge-bound direction. A C-stick press
carries its captured vector; an ordinary attack can retain its existing
policy of reading the movement level at interpretation. Do not freeze every
attack's direction as an incidental buffering change. Priority may remain,
provided it chooses that whole press. Latest held levels are a separate
value; a neutral sample must not erase an earlier qualified edge.
Preserve the existing same-sample C-stick/button arbitration. There is no
need for an unbounded event queue.

Verify two distinct qualified presses within one tick and the existing
neutral/recentered case. Coordinate any changed encoded fields with the
wire identity and stream-version owners. IN1's semantic stage change is not
covered merely by a field-layout check.

**Done 2026-11-23.** `ControlFrame::qualified_press` chooses the tick's
attack press whole from one sample: its strength hint, whether the aim
stick aimed it and that stick's flicked direction. The stronger hint wins
and a tie goes to the newer press, so Smash-left then Tilt-right stays
Smash-left. Only a sample that carries the press qualifies one, so a later
held modifier or a neutral sample cannot raise or erase it.
`AttackStrengthHint::merge` became `rank`, the ordering the choice uses.
No encoded field changed. Guard:
`two_qualified_presses_in_one_tick_keep_one_press_whole`, beside the
existing recentered-flick case.

## Input continuity across routing and control changes

**Source follow-up:** `00bf92c22142`, 2026-11-22. IN4/IN5 distinguish
physical observation, eligibility to deliver input, and continuation of
work already accepted by a body. The findings below are source-derived;
no new gameplay, rollback or timing witness was run.

A routing change may stop delivery. It cannot by itself establish that a
finger released or pressed a control. A real neutral sample, a sample
masked by a UI owner, an invalid sample during map replacement and the
end of a driving assignment have different meanings. Preserve the
existing participant, context, latch and body-control owners.

### IN4 — observe source transitions before routing

**State (2026-11-23):** the capture and rebind crossings are done. The
runtime carrier is `SeatSourceEdges` (it was `SeatBurstTriggerState`): it
follows valid physical samples while gameplay is captured, and discards
their gameplay edges (the latch suppression is unchanged). A rebind's
zeroed sample (`Rebound` present at `InputSet::Route`) and an unfocused
window that suppresses input observe nothing: they mark the history for
resync, and the next valid sample seeds it without a press, as
`swallow_the_rebinds_own_edges` does for buttons. The carrier stays in the
runtime: the sampler sees `Rebound` on the reset frame itself, so no
priming operation had to cross into `ambition_input`. A temporary probe
through `populate_seat_control_frames` (burst trigger held at 1.0) measured,
old sampler / fix: held through a pause, on return: press / none; release
and repress inside the pause: press / none; held across a rebind: press /
none; a real repress after return: press / press. It was removed. The
local producer retirement cut below is done too (2026-11-23):
`seat_input_participants_for_roster` neutralizes the retired seat's own
`SeatRawFrames` row and resets its latch beside the despawn (a probe with a
pending jump press read neutral in both after the sweep; the old sweep
wrote neither). IN4 is complete.

#### Native failures and their controls

[`populate_seat_control_frames`](../../../crates/ambition_platformer2d_runtime/src/input.rs)
resets `GameplayEdgeState` to Released whenever gameplay is unavailable.
That state tracks burst trigger, C-stick attack and tap-jump, despite its
runtime carrier being named `SeatBurstTriggerState`. When the same
continuously held analog value is read after capture, hysteresis reports
a new press. Ordinary buttons use Leafwing's `just_pressed` and do not
need a new press simply to resume a held level.

There is a second path through
[`rebuild_maps_from_recipes`](../../../crates/ambition_input/src/bindings.rs).
It resets `ActionState` and marks `Rebound`. The next PreUpdate repairs
button edges, but the analog tracker can read the intervening zero sample
as a release, then the still-held control as a new press. The runtime
sampler cannot currently consume the priming transition: `Rebound` is
removed before its next Update, and the rebind owner cannot query the
runtime-owned carrier.

#### Small ownership correction

Observe valid physical samples and advance source-edge history before
filtering gameplay delivery. Discard masked action edges at the routing
boundary; keep menu-edge latching and gameplay latch suppression.
Merely freezing the tracker while captured is insufficient: a release and
press entirely inside the captured interval must not leak into gameplay
on return.

Rebinding is different from a valid neutral observation. Treat the reset
sample as invalid and seed the trackers from the first valid sample under
the new map without manufacturing edges. Preserve the existing documented
policy for a real press coinciding with that resynchronization. If a
platform supplies no valid physical observations while unfocused, use
explicit resynchronization on return rather than claiming a release was
observed.

Keep the pure `GameplayEdgeState` in `ambition_input`. Move its
participant carrier there, or provide the narrow priming operation and
transition lifetime the native sampler needs. Do not make the input owner
depend on the runtime. The source-validity transition must reach the
sampler before it expires. This is a fixed-size per-participant fold; it
needs no new device registry or wire field.

**First proof:** extend the existing focus/context fixture through reopening
while an analog control remains held. Compare held-without-press, an actual
release/repress, and an unaffected seat. Extend
`a_key_held_across_a_rebind_is_not_a_new_press` with an analog input while
an unrelated binding changes. Keep its ordinary-button control. These
native crossings are more informative than another pure hysteresis test.

**Remove:** context-induced Released resets and invalid reset samples as
source history. Keep per-pad filters, the current thresholds and intended
held-level resumption.

#### Local producer retirement is an ownership cut

The same source audit found that local roster retirement despawns an
`InputParticipant`, while `populate_seat_control_frames` only rewrites
remaining rows and `commit_seat_raw_frames` republishes every persistent
`SeatRawFrames` row. A retired local producer's last edge can therefore
be reintroduced after a latch drain. No accidental join or attack was
reproduced by this audit.

When touching that retirement cut, retire or neutralize that producer's
own raw row and reset its pending latch through `SlotControlLatches::reset`.
The latch reset alone does not clear `SeatRawFrames`. Input-participant
absence alone is not authority to clear a remote, harness or authored
producer's row. Prove retirement with a pending edge and more than one
render-commit/tick-drain cycle; retain a fresh source's real Start admission.
This does not require a second source-routing table.

### IN5 — withdraw input-dependent continuations at an admitted cut

IN4 corrects local source recognition. Native consumers also need to
distinguish input interruption from an ordinary neutral sample. The current
`ControlFrame` has no availability/interruption fact; `menu_only` erases
the distinction. A latch's `device_seen` is sticky producer-ever-seen
state, and the peer publication verdict has a different purpose.

#### Two native consumers establish the need

**Possession gesture.**
[`possession_trigger_system`](../../../crates/ambition_platformer2d_actor_monolith/src/control/possession.rs)
derives a Down+Interact rising combination from filtered held values and
`prev_down_interact`. With opt-in focus filtering, a neutral tick while
`GameMode::Playing` clears that history. Restoring focus with the same
continuous hold then releases the possession. A whole-world pause skips
this system; do not claim it takes the same path.

Keep a genuine new combination distinguishable from resumed held levels.
Preserve the 0.35 direction threshold, body-frame interpretation, and the
ability to press either part of the combination last. Existing direction
and Interact edges are candidates, but their thresholds/semantics must be
checked. Do not
incidentally change the gesture to Interact-only or reconstruct a fresh
press from masked neutral.

**Pending fireball charge.**
[`charge_projectile_input`](../../../crates/ambition_projectile_combat/src/systems.rs)
sets `PlayerProjectileState.charging` on press, advances it on held,
and takes it on release. Neutral/no-message input leaves it present.
The [emitter](../../../crates/ambition_combat/src/action_emission.rs) uses
already-translated/gated `ActorControl`; a release while input is captured
can no longer reach the charge.

A second native sequence uses the shipped charging home body: start
charging, possess another body while holding fire, release fire on that
body, then return home with neutral input. Handoff changes driving
assignment and control frames but retains the home's projectile state.
The original charge can remain pending without a future release.

#### Proposed bounded policy and admission value

For this pending fireball charge, loss of eligible input supply or its
driving assignment **cancels without firing**. The charge stays on its
original body until that owner withdraws it; it is not transferred to the
newly driven body. A later valid press starts a new charge. This is the
proposed first policy, not a claim that a cancellation receipt already exists.

Keep these cases separate:

| Transition | Continuation rule |
| --- | --- |
| UI/focus capture or end/change of the driving supply | Withdraw the affected input-dependent charge without producing a shot; requalify native gesture history. |
| Body action hold while input ownership remains | Keep the active state and selected input observation; spend only ACT2's action interval. |
| Accepted `MovePlayback` after possession changes | Keep the committed move on its original body under its existing interrupt/completion rules. |
| Real fire release under eligible control | Apply normal fire admission once. |

Do not infer supply loss from an already-gated zero `ActorControl` or
from zero `dt`. A combat/action hold can neutralize execution while the
input owner remains. Likewise, missing an ephemeral message is not proof
of a physical release. If a later product chooses pause-suspension for this
charge, it must keep observing/reconciling release and continuity; neutral
masking alone cannot implement that policy.

Carry the smallest bounded supply/interruption fact through the existing
admitted per-seat input representation. It needs to preserve both the
selected current eligibility and an interruption since the previous
consume cut when that interruption ends before the next tick. A latest
Boolean alone would lose capture-and-resume between two simulation steps.
Use the existing latch's suppression/merge/consume operations; do not
create a host-only cancellation queue outside recorded input.

The fold keeps latest eligibility, OR-retains interruption through
suppression and merging, and clears the consumed interruption while keeping
levels. Ordinary direct semantic/harness frames, including existing
`..Default::default()` constructions, remain eligible; an explicitly absent
or withdrawn producer is different from a neutral frame. Define that
default in the representation and every adapter. Old recordings cannot
retroactively reveal interruptions they omitted. Apply the existing stream
compatibility rule alongside IN1's changed capture-stage meaning, and bump
the peer wire identity when its field layout changes.

At admission, withdraw the old input-dependent continuation before
accepting a genuinely new eligible edge that survived the capture
boundary. Suppression must discard earlier pending gameplay edges while
retaining the interruption obligation. Prediction/hold-last-input keeps
levels but must not repeatedly invent the interruption edge. Coordinate
codec, stream and fixed-width peer changes with IN1; old streams must not
be described as carrying provenance they never recorded.

The accepted driving relation supplies body/participant ownership, but a
query after replacement only identifies the new driver.
`PlayerProjectileState.charging` currently retains no initiating driver.
Withdraw the old pending charge at the existing relation removal/replacement
cut, or retain the bounded prior ownership needed by its native owner.
Preserve that necessary continuation through rollback; do not equate
"currently undriven" with "just lost its driver" or cancel autonomous fire
because it has no participant. Do not query the host's current focus or
menu state from replayed possession or charge execution. Reuse the existing
relation; no global device epoch, new action bus or universal cancellation
registry is required.

#### Same-body binding replacement is a local handoff

**Planning refinement, runtime source `6d100f9d9e0d`, 2026-11-22.** A form
or loadout can change the semantic receiver while the participant, driven
body and eligible input supply stay the same. The per-seat interruption
fact above cannot by itself express this body-local transition.

The shipped Fireball and Eye Laser rows share the `projectile` exclusive
slot in the [item catalog](../../../game/ambition_content/assets/data/items.ron).
An admitted [equip](../../../crates/ambition_platformer2d_items/src/narrative.rs)
can replace that slot while a native Fireball charge is pending.
[`gate_body_control`](../../../crates/ambition_platformer2d_actor_monolith/src/control/acceptance.rs)
then derives the new scheme, and
[`resolve_control_slots`](../../../crates/ambition_characters/src/action_scheme.rs)
routes held/release to Eye Laser while clearing the raw projectile fields.
The native charge capability/state remain installed. Their consumer sees
neutral input, which does not take `PlayerProjectileState.charging`.
Release under Laser and return to Fireball with neutral input can therefore
leave the original charge pending. This trace does not rely on opening the
equipment menu; its UI capture may independently cancel under IN5.

At the native old/new composition cut, settle the input-dependent work whose
binding is withdrawn before routing solely through the replacement. For the
pending native Fireball charge, extend IN5's proposed first policy: cancel
without firing. An unrelated equipment change with the same effective
Projectile binding preserves the charge. Compare the selected source,
executor and continuation meaning, not a whole-character revision or only
the new scheme after the old answer has been discarded.

If a product deliberately retains that charge, it must retain a compatible
route that can receive its real release and selected aim. Keeping the state
while all controls go only to the new binding cannot implement retention.
Settle through the existing native owner at the transition, or retain the
bounded prior binding it needs. Do not synthesize a physical release, flush
unrelated controls, or withdraw the participant's whole input supply.
Removing a technique from the current route is not itself an owner teardown.

Incoming activation keeps its own policy. Eye Laser's
[`burn_tick_of`](../../../game/ambition_content_modules/src/eye_laser.rs)
deliberately starts from `pressed || held`; preserve that level-start rule.
A press-started replacement gets no fabricated press. No universal
release-before-reuse rule follows from this handoff.

Keep committed moves, buffered-shot admission, refire and banked progress
under the [action owner's continuation rules](body-action-execution.md#a-committed-emission-retains-its-admission-basis).
Canceling the pending input charge is not removal/reset of its whole
controller component or resource pool. A released projectile has its own
lifetime. The composition change already enters through simulation, so
record/restore the affected native state under its body/room owner; this
handoff needs no additional peer-wire epoch or host-only cancellation queue.

**Witness:** begin charge, replace Projectile with Eye Laser through an
admitted same-body equip, release under Laser, and restore Fireball with
neutral input. The old pending charge ends at replacement without a shot
or stale aim state; a later real press works. Keep an unrelated equipment
change as the charge-preserving control and Eye Laser held activation as
the incoming-policy control. Extend the native loadout/charge crossing,
including restoration across its admitted cut, rather than add a pure
edge recognizer test. This local transition can land without waiting for
IN1's changed per-seat wire representation.

#### First proof and deletion

Through native input and possession, keep Down+Interact held across focus
capture and resume; possession remains until a real new combination.
An unrelated seat's capture has no effect. Exercise capture/resume between
two tick consumes so a latch implementation cannot erase the interruption.
Also capture, resume and receive a genuine new eligible fire press before
the same consume: cancel the old charge, then accept the new one. The next
held/predicted input must not repeat the interruption and cancel it again.
Deliver a late corrected input and require the same native outcome. A
sync-test replay alone does not prove the hold-last prediction rule.

Extend `a_charged_tap_fires_once` with release during capture and the
charging-home possession sequence. Assert cancellation on departure before
returning home, no shot or stuck charge/aim state, and a fresh press that
works afterward. Keep autonomous fire as an allowed control. Keep
`releasing_a_seat_keeps_the_active_move_on_its_original_body` in
[the possession fixture](../../../game/ambition_app/tests/possession_end_to_end.rs)
as the control against blanket action cancellation. Reuse its replay road
for the admitted interruption; a pure charge helper cannot prove routing.

Delete filtered-neutral history as the authority for a fresh possession
gesture and the pending charge's dependence on a release that its former
controller can no longer deliver. Keep body-relative held-direction rules,
mount steering's deliberate verb subset, and the same-tick Start that
claims a controller and joins. Do not globally flush those controls.

Human gesture observation clocks remain distinct from action progression.
The current motion buffer ages during live neutral input; global pause
holds its simulation clock. Up/down tap windows are described in real
seconds but their producer is GameplayGated. State each changed gesture's
clock/interruption policy; this audit does not select a universal
cross-pause gesture reset or another time service.

## Menu activation policy

Pointer activation already shares `MenuTapMode` policy across current menu
call-shapes, including destructive-row guarding. Gamepad submit does not have
the same stray-touch failure mode.

Treat extending destructive double-confirm behavior to gamepads as a product
feel decision, not unfinished plumbing.

## Controller routing debug view — OPEN

Jon requested a PipeWire-style connection view on 2026-11-03. Defer it until
the current menu-region and gate fixes are complete.

Provide a developer menu that shows connected controllers, input participants,
and the connections between them. Include the keyboard, shared sources,
claimable channels, retained claims for disconnected pads, local and remote
ownership, and each participant's current body and active input context.
Show an unconnected device or participant without a connection. State why a
Start press will join or pause. Keep the view live through connection changes,
a join, a leave, and a new gameplay session.

Read the existing device order, source plan, claims, participant contexts and
body assignments. Do not create another routing table. The first version is
read-only; manual reassignment is separate work. Check the real host with a
primary controller, an unused controller, a disconnected claim and two joined
participants. The graph must agree with which body and menu receive input.

## Test requirement

Input/menu tests are feature-sensitive. A plain per-crate `cargo test` may omit
substantial participant or presentation test modules. Verification for a changed
slice must name the feature composition that actually includes the code being
changed, plus the relevant real host when ownership depends on shell composition.

## Exit

The architecture is complete enough to leave active planning when:

1. participant identity, context and body assignment remain separate concepts;
2. per-seat dialogue/gameplay ownership has an explicit model;
3. a provider action can travel from authored/composed binding through a
   participant semantic edge to a domain request without a core action-enum edit;
4. controller/touch presentation has a deliberate finite-slot policy;
5. multi-view prompt ownership is explicit when a product actually uses several
   independent local views.

## Accepted actions and installed authored programs

A11/A12 in the [frontier](actor-monolith-work-frontier.md) validate technique keys,
parameters and bounded flow graphs before content activation. The participant
surface proposes actions; body/control owns acceptance; move execution owns the
resulting occurrence. Neither the authoring API nor an agent response bypasses
that chain.

Keep action identity distinct from effect/technique identity and from a live move
occurrence. Per-occurrence contact latches cannot establish that a later authored
beat made a fresh contact. Add scoped beat facts only when a real multi-beat
customer requires them, with replay and cancellation tests.
