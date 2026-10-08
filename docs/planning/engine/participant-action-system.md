# Participant and semantic action system — current residual work

**State:** OPEN, narrow.

The original participant/input migration is largely landed. This plan owns only
remaining participant-context and provider-action architecture.

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
(`actor_monolith/src/schedule/input_systems.rs`).

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
slot as a hot one does. A refused attempt (`features/ecs/brain_effects.rs`)
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

## Menu activation policy

Pointer activation already shares `MenuTapMode` policy across current menu
call-shapes, including destructive-row guarding. Gamepad submit does not have
the same stray-touch failure mode.

Treat extending destructive double-confirm behavior to gamepads as a product
feel decision, not unfinished plumbing.

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
