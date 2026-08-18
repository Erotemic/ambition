//! Player ECS systems.

use bevy::prelude::*;

use super::components::{PlayerEntity, PrimaryPlayer};
use super::events::PlayerHealRequested;
use super::movement_components::{BodyGroundState, BodyKinematics};
use crate::features::ActorPose;
use ambition_characters::actor::BodyHealth;
use ambition_characters::brain::{
    ActorControl, Brain, BrainSnapshot, ScriptedControl, SlotControls,
};
use ambition_platformer2d_core as ae;

/// Blank the control frame of every body a scripted sequence is driving.
///
/// Ordered immediately AFTER the brains write, which is the whole point. The
/// sequences that predate [`ScriptedControl`] each blanked the frame from their
/// own phase — Mary-O's death beat from `GameplayEffects` — and the brain simply
/// refilled it at the next frame's `PlayerInput` before anything read it. The
/// only position where blanking is observable is between the producer and the
/// frame's consumers, so the engine owns that position rather than asking each
/// sequence to find it.
///
/// This clears the frame the body ACTS on; it does not touch the device layer,
/// so a held button is still held and resumes on its own once the sequence
/// retires.
pub fn blank_scripted_control_frames(mut bodies: Query<&mut ActorControl, With<ScriptedControl>>) {
    for mut control in &mut bodies {
        control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
    }
}

/// Mirror authoritative player body state into the generic gameplay
/// [`ActorPose`] used by the brain/action resolver.
///
/// The player, NPCs, enemies, and bosses should all expose action origins
/// through gameplay pose data rather than presentation `Transform`s.
pub fn sync_player_actor_poses(
    mut players: Query<(&BodyKinematics, &mut ActorPose), With<PlayerEntity>>,
) {
    for (kin, mut pose) in &mut players {
        *pose = ActorPose::from_parts(kin.pos, kin.size * 0.5, kin.facing);
    }
}

/// **The set [`tick_controlled_brains`] runs in — the controlled-decision phase.**
///
/// FOUR consumers pinned this function by name: both Mary-O rows, one Sanic row,
/// and the causal movement-intent observer. It was recorded for a while as the
/// one conversion that would be STRICTER rather than equivalent, on the grounds
/// that `PlayerInputSet::Brain` already holds two things.
///
/// ⭐ that reasoning was wrong, and the correction generalises: "the target is
/// already in a multi-member set" only forces a stricter pin if you insist on
/// reusing THAT set. A NESTED single-member set is always available and is
/// exactly equivalent to the leaf pin it replaces.
///
/// It also unblocks a pin that could never have used the parent. The causal
/// observer `record_player_movement_intent` is itself a member of
/// `PlayerInputSet::Brain`, so `.after(PlayerInputSet::Brain)` would be a cycle;
/// `.after(ControlledBrainTick)` is not, because it is not in this set.
///
/// ⚠ ONE member, permanently. The parent phase is the place to add brain-adjacent
/// work; this set means "participant control has become `ActorControl`" and
/// nothing else, which is precisely what all four consumers were reaching for.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControlledBrainTick;

/// **Translate participant control into `ActorControl`, for ANY controlled body.**
///
/// The INPUT AUTHORITY is [`SlotControls`], keyed by the `Brain::Player(slot)`
/// the body itself carries — never `PlayerInputFrame` and never an entity
/// marker. A body is controlled because it holds a participant's brain, which is
/// equally true of the home avatar and of an actor somebody possessed.
/// `PlayerInputFrame` is now only a compatibility mirror for player-flavoured
/// ability/UI systems (held item, heal shrine, portal gun) written by
/// `sync_local_player_input_frame`.
///
/// ⭐⭐ **THE `With<PlayerEntity>` FILTER IS GONE, AND THAT IS THE POINT.** It
/// was here because a possessed actor otherwise had TWO producers writing its
/// `ActorControl` in one tick — this one and `tick_actor_brains` — and the
/// filter picked a winner by identity. Measured 2026-08-14, what the possessed
/// body was paying `tick_actor_brains` for is: a crowd observation, an enemy
/// brain snapshot, a perception policy, a world view built over the collision
/// world, a believed-target derivation, and a MUTATION of its
/// `PerceptionMemory` — none of which `tick_player_brain_from_control` reads.
/// It reads six facts, and all six are here. So the arbitration is no longer by
/// identity: `tick_actor_brains` now leaves a player-brained body alone, and a
/// human piloting a body no longer constructs AI perception to move a stick.
///
/// ⇒ the one fact that WAS actor-specific is the movement scale, and it does not
/// need actor configuration to state it. `velocity_target` is an absolute
/// world-space command, so the translation needs the body's own top speed;
/// [`MotionModel::commanded_top_speed`] is that number, on the one movement-policy
/// component every movable body already carries. A body with no movement policy
/// commands no speed, which is what the home avatar did explicitly before.
///
/// The query requires `&mut Brain`, so a vacated home avatar (its player brain
/// transferred away by `possession`) carries no `Brain` and is skipped — it stays
/// inert with a neutral `ActorControl`. A body whose brain is not a participant's
/// is skipped too: its `ActorControl` belongs to an AI producer.
///
/// ⚠ **one filter is deliberately NOT inherited from the actor tick:
/// `Without<Dormant>`.** Dormancy sleeps a BRAIN — *"only the brain sleeps: the
/// body still integrates"* — and a participant is not an AI to be optimised away.
/// A human pressing right on a body that has gone dormant must move it.
pub fn tick_controlled_brains(
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    slots: Res<SlotControls>,
    mut controlled: Query<(
        &BodyKinematics,
        &BodyGroundState,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        Option<&ambition_platformer2d_core::MotionModel>,
        &mut Brain,
        &mut ActorControl,
    )>,
) {
    let control_frame_modes = user_settings
        .as_deref()
        .map_or(ae::ControlFrameModes::default(), |s| {
            s.gameplay.control_frame_modes()
        });

    for (kin, ground, resolved_frame, motion_model, mut brain, mut control) in &mut controlled {
        // The body's OWN per-tick resolved frame (ADR 0024): the same value
        // this tick's integration moves the body under, so controller
        // interpretation and physics can never disagree at a zone boundary.
        let control_down = resolved_frame.down();
        // INPUT AUTHORITY: this body's OWN slot frame, keyed by the brain it
        // carries — the SAME `Brain::Player(slot)` → `SlotControls` path a
        // possessed actor reads. A body whose brain isn't a player brain is
        // skipped (its `ActorControl` is owned by an AI tick, not this one).
        let Some(slot) = brain.player_slot() else {
            continue;
        };
        let input = slots.get(slot);
        // Build the snapshot from the player's cluster components plus
        // the per-tick slot frame. The input is what makes
        // Brain::Player's translation deterministic: same input +
        // same body snapshot → same ActorControlFrame.
        let snapshot = BrainSnapshot {
            // A possessed body's brain drives a body a person is steering; it is
            // never in a capture on this road, and saying so beats inheriting a
            // default nobody chose.
            captured: false,
            holding_captive: false,
            pummels_landed: 0,
            // The avatar's own body; the fighter brain is not on this path,
            // but the field is the snapshot's and every builder fills it.
            subject: None,
            actor_pos: kin.pos,
            actor_vel: kin.vel,
            actor_facing: kin.facing,
            control_down,
            movement_frame_mode: control_frame_modes.movement,
            aim_frame_mode: control_frame_modes.aim,
            actor_on_ground: ground.on_ground,
            // Human input owns facing policy. Collision facts do not implicitly
            // reverse a controlled body.
            side_contact_normal: None,
            turns_at_walls: false,
            // FB4b §13.2: the fighter brain's attack kit. EMPTY here, and that is a
            // recorded gap rather than a default: `ActorMut` does not carry the
            // body's `ActorMoveset`, so filling this needs the moveset threaded into
            // the actor query. A fighter with an empty kit plays MOVEMENT ONLY —
            // `generate_options` produces no attacks — which is honest degradation
            // and not a silent wrong answer. See the S7 row in the 72h queue.
            attack_kit: Vec::new(),
            // The player brain reads input, not the Smash aerial path; grounded
            // locomotion semantics regardless of fly mode.
            actor_aerial: false,
            alive: true,
            target_pos: kin.pos,
            target_alive: true,
            // The player brain doesn't regroup on damage; full-health is inert here.
            health_fraction: 1.0,
            sim_time: 0.0,
            dt: 0.0,
            // **The body's own top speed, from its own movement policy.** The
            // grounded integrators scale the normalized `locomotion` stick
            // themselves and ignore this; a FREE-MOVER is steered by the absolute
            // `velocity_target` the translator derives from it, which is how a
            // possessed flyer moves at ITS speed with no possession-specific
            // plumbing. Absent policy ⇒ 0.0, the value the home avatar stated
            // explicitly when this system only ever saw home avatars.
            max_run_speed: motion_model.map_or(0.0, |model| model.commanded_top_speed()),
            // The player brain does not predict; it translates a stick. Nothing
            // on this path reads a movement law, and claiming one would be a
            // fact the avatar path never resolved.
            movement_tuning: None,
            // Same reason as the law above: the player brain translates a stick
            // and predicts nothing, so it never asks the kernel a question that
            // would need the kit.
            abilities: None,
            attack_cooldown_remaining: 0.0,
            attack_windup_remaining: 0.0,
            attack_active_remaining: 0.0,
            attack_recover_remaining: 0.0,
            stun_remaining: 0.0,
            // BossPattern-only inputs — inert for the player body.
            boss_encounter_phase: None,
            world_size: ae::Vec2::ZERO,
            front_wall_clearance: None,
            player_input: Some(input),
            // Player brain doesn't consult these fields; leave them
            // None so the snapshot builder doesn't pay for queries
            // the brain ignores.
            crowding: None,
            terrain: None,
            // Player brain reads its own air-jump state via the
            // PlayerInputFrame / engine path, not via the snapshot.
            air_jumps_remaining: 0,
        };
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);
        control.0 = frame;
    }
}

/// Apply heal messages to the authoritative `BodyHealth` ECS component.
///
/// A heal targets either a specific player entity (`heal.target ==
/// Some(entity)`) or the primary player as a fallback (`None`). The
/// fallback path keeps existing call sites — cutscene heals, dev-tool
/// heals — working with no change. Per-player producers like pickup
/// collection should set the target explicitly so a non-primary
/// player who walked into the heart actually gets healed.
pub fn apply_player_heal_requests(
    mut heals: MessageReader<PlayerHealRequested>,
    mut players: Query<&mut BodyHealth, With<PlayerEntity>>,
    primary_q: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let primary = primary_q.single().ok();
    for heal in heals.read() {
        if heal.amount <= 0 {
            continue;
        }
        let target = heal.target.or(primary);
        let Some(target) = target else {
            // No player entity yet (startup or headless): drop the
            // heal silently so the queue still drains.
            continue;
        };
        if let Ok(mut health) = players.get_mut(target) {
            health.heal(heal.amount);
        }
    }
}

/// Mana regenerated per second (clamped to the meter max).
const MANA_REGEN_PER_SEC: f32 = 14.0;

/// Mana slowly regenerates so it's a genuine spendable resource. Uses
/// `ResourceMeter::refill` (clamped) rather than the meter's own `regen_rate`
/// field so we don't change `BodyMana::default` (and any test that relies on
/// it). Scaled by sim dt, so bullet-time / pause slow it with the world.
///
/// Refills the *controlled subject's* mana — the body actually spending it on
/// charge attacks / the fireball — so possessing an actor regenerates that
/// actor's meter, not the vacated home avatar's. (Moved from the render HUD
/// module, E4: a sim mutator never lives in presentation.)
pub fn regen_player_mana(
    time: Res<ambition_time::WorldTime>,
    controlled: Option<Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
    mut manas: Query<&mut crate::actor::BodyMana>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let Some(subject) = controlled
        .as_deref()
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok())
    else {
        return;
    };
    if let Ok(mut mana) = manas.get_mut(subject) {
        mana.meter.refill(MANA_REGEN_PER_SEC * dt);
    }
}

#[cfg(test)]
mod tests;
