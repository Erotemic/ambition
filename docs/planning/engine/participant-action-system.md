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

The remaining responsibilities are deliberately outside `ambition_input`:

- composition maps a participant press to the body/domain request that should
  consume it;
- controller/touch presentation chooses which finite physical/on-screen slot a
  provider action occupies;
- authoring schemas for bindings are added only when a tooling customer needs
  them.

Do not make `ambition_input` learn actor/body concepts to close the final hop.

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
