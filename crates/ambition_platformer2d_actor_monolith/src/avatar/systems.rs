//! Player ECS systems.

use bevy::prelude::*;

use super::events::PlayerHealRequested;
use ambition_characters::actor::BodyHealth;
use ambition_characters::brain::{tick_player_brain, BrainSnapshot};
use ambition_characters::control::ActorControl;
use ambition_characters::control::ScriptedControl;
use ambition_characters::control::{DrivingParticipant, SlotControls};
use ambition_combat::components::ActorPose;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::{BodyGroundState, BodyKinematics};
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

/// Blank scripted bodies after brain production and before control consumers.
///
/// Device state is untouched, so held input resumes when scripted control ends.
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

/// Ordering seam meaning participant input has been translated to `ActorControl`.
///
/// Keep this a single-member leaf set; brain-adjacent work belongs in the parent
/// phase, while consumers may order directly after this translation.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControlledBrainTick;

/// Translate `SlotControls` into `ActorControl` for any participant-driven body.
///
/// `DrivingParticipant` selects the slot; body motion policy supplies movement
/// scale. Vacated or autonomous bodies are skipped. Dormancy does not suppress
/// human-driven bodies because it sleeps AI brain work, not body integration.
pub fn tick_controlled_brains(
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    slots: Res<SlotControls>,
    mut controlled: Query<(
        &BodyKinematics,
        &BodyGroundState,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        Option<&ambition_platformer2d_core::MotionModel>,
        &DrivingParticipant,
        &mut ActorControl,
    )>,
) {
    let control_frame_modes = user_settings
        .as_deref()
        .map_or(ae::ControlFrameModes::default(), |s| {
            s.gameplay.control_frame_modes()
        });

    for (kin, ground, resolved_frame, motion_model, driver, mut control) in &mut controlled {
        // Input interpretation uses the same resolved frame as this tick's physics.
        let control_down = resolved_frame.down();
        // Input authority is the body's driving slot.
        let slot = driver.0;
        let input = slots.get(slot);
        // Same slot frame plus same body snapshot produces the same control frame.
        let snapshot = BrainSnapshot {
            // A possessed body's brain drives a body a person is steering; it is
            // never in a capture on this road, and saying so beats inheriting a
            // default nobody chose.
            captured: false,
            captured_for: 0.0,
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
            // This translation path does not carry an ActorMoveset; fighter attack
            // generation is therefore inactive here.
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
            // Free-mover velocity targets use the body's own commanded top speed.
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
            // Player brain does not consult this snapshot field; air-jump
            // acceptance remains body-side movement state.
            air_jumps_remaining: 0,
        };
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        // the player-input translator, called DIRECTLY rather than reached
        // through an enum arm. It was only ever a `Brain` variant because the
        // seat was; with the seat named on the body there is nothing left to
        // dispatch on.
        tick_player_brain(slot, &snapshot, &mut frame);
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
/// Refills every DRIVEN body's mana — the bodies actually spending it on charge
/// attacks and held abilities — so possessing an actor regenerates that actor's
/// meter, not the vacated home avatar's, and a couch's second seat regenerates
/// at all.
///
/// ⛔⛔ IT REFILLED ONE `ControlledSubject`, which is one entity by construction.
/// Seat one spent mana on a gauntlet it could never get back — a slow leak
/// rather than a dead verb, which is why it outlived the verbs' own fix.
pub fn regen_player_mana(
    time: Res<ambition_time::WorldTime>,
    driven: crate::items::pickup::DrivenBodies,
    mut manas: Query<&mut ambition_platformer2d_core::BodyMana>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // ⚠ THE FALLBACK IS THE STARTUP FRAME and nothing else.
    let mut subjects = driven.entities();
    if subjects.is_empty() {
        subjects.extend(primary.single().ok());
    }
    for subject in subjects {
        if let Ok(mut mana) = manas.get_mut(subject) {
            mana.meter.refill(MANA_REGEN_PER_SEC * dt);
        }
    }
}

#[cfg(test)]
mod tests;
