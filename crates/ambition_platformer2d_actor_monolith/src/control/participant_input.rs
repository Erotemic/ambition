//! Translate participant seat input into body control.

use bevy::prelude::*;

use ambition_characters::brain::{tick_player_brain, BrainSnapshot};
use ambition_characters::control::ActorControl;
use ambition_characters::control::{DrivingParticipant, SlotControls};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::{BodyGroundState, BodyKinematics};

/// The frame a held movement stick was pressed in, kept while it stays held
/// (Jon, 2026-10-19: in ceiling_cross the player-relative camera "used to
/// just flip the screen once and let me continue moving in that direction").
///
/// A body-relative stick (every mode, under the player-relative camera) is
/// read in the body's frame. When gravity turns while the stick is held, the
/// same hold would point the other way in the world: held right into an
/// upward field became world-left, walked the body back out, turned gravity
/// back, and so on (30 turns in 6 s, measured). So a held stick keeps the
/// world direction it had when it was pressed; letting go, or turning the
/// stick, reads it in the body's frame again. Rollback state: the frame of a
/// hold is part of the body's input history.
///
/// A hold belongs to one seat's stick, read under one frame mode. Another
/// seat that drives the body, or a change of the mode, is a new hold, and a
/// body that no seat drives keeps none.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HeldControlFrame {
    /// The body's `down` when the hold began.
    pub down: ae::Vec2,
    /// The raw stick of the hold, to tell when it turns.
    pub raw: ae::Vec2,
    /// The seat whose stick this is.
    pub slot: ambition_characters::control::PlayerSlot,
    /// The movement frame mode the stick was read under.
    pub mode: ae::InputFrameMode,
}

/// A stick this long is held.
const HELD: f32 = 0.2;
/// A stick turned further than this from its hold (cosine) is a new hold.
const SAME_HOLD_COS: f32 = 0.7;

/// The raw stick to read this tick in the body's frame `down` under `mode`,
/// and the hold to keep: a held stick points where it pointed in the frame it
/// was pressed in ([`HeldControlFrame`]).
pub fn hold_stick_through_turns(
    hold: Option<HeldControlFrame>,
    raw: ae::Vec2,
    down: ae::Vec2,
    mode: ae::InputFrameMode,
    slot: ambition_characters::control::PlayerSlot,
) -> (Option<HeldControlFrame>, ae::Vec2) {
    if raw.length() < HELD {
        return (None, raw);
    }
    let held = hold.filter(|hold| {
        hold.slot == slot && hold.mode == mode && raw.normalize().dot(hold.raw.normalize_or_zero()) >= SAME_HOLD_COS
    });
    let Some(held) = held else {
        return (Some(HeldControlFrame { down, raw, slot, mode }), raw);
    };
    if held.down == down {
        return (Some(held), raw);
    }
    let then = ae::AccelerationFrame::new(held.down);
    let now = ae::AccelerationFrame::new(down);
    let world = then.to_world(then.resolve_input(mode, ae::ScreenAxes::from_vec(raw)).vec());
    let stick = now.raw_axis_for_resolved_input(mode, ae::LocalAxes::from_vec(now.to_local(world)));
    (Some(held), stick.vec())
}





/// Ordering seam meaning participant input has been translated to `ActorControl`.
///
/// Keep this a single-member leaf set; brain-adjacent work belongs in the parent
/// phase, while consumers may order directly after this translation.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControlledBrainTick;

/// Translate replayed seat input for each uniquely driven body.
/// Refuse an ambiguous seat with neutral control and remove its held frame.
/// A latched control frame must not survive loss of authority.
/// Dormancy suppresses autonomous brain work, not participant input.
#[allow(clippy::too_many_arguments)]
pub fn tick_controlled_brains(
    mut commands: Commands,
    // Read replayed seat frames. Live menu settings must not change resimulation.
    slots: Res<SlotControls>,
    drivers: Query<(bevy::prelude::Entity, &DrivingParticipant)>,
    // A body that no seat drives keeps no hold: the next seat to drive it
    // presses its own.
    undriven: Query<bevy::prelude::Entity, (With<HeldControlFrame>, Without<DrivingParticipant>)>,
    mut controlled: Query<(
        bevy::prelude::Entity,
        &BodyKinematics,
        &BodyGroundState,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        Option<&ambition_platformer2d_core::MotionModel>,
        &DrivingParticipant,
        &mut ActorControl,
        Option<&HeldControlFrame>,
    )>,
) {
    for entity in &undriven {
        commands.entity(entity).remove::<HeldControlFrame>();
    }
    for (entity, kin, ground, resolved_frame, motion_model, driver, mut control, hold) in &mut controlled
    {
        // Input interpretation uses the same resolved frame as this tick's physics.
        let control_down = resolved_frame.down();
        // Input authority is the body's driving slot.
        let slot = driver.0;
        // ⛔ AND THE SEAT'S OWN ANSWER TO WHO HOLDS IT. A body carrying
        // `DrivingParticipant(slot)` is a CLAIM on that seat, not a licence: two
        // claims are ambiguous authority, and the resolver refuses rather than
        // picking by query order. Skipping here is what makes that refusal real
        // instead of advisory.
        if crate::control::body_driving_seat(&drivers, slot) != Some(entity) {
            // ⛔⛔ **NEUTRALISED, NOT SKIPPED — AND SKIPPING WAS THE FIRST FIX,
            // WHICH THE FIXTURE CAUGHT.** `ActorControl` is LATCHED state: it
            // holds whatever was written into it last tick until something
            // writes again. So declining to write it leaves the body running on
            // the last frame it was given, and under a HELD press that is
            // indistinguishable from driving — measured, the player kept
            // travelling its full 180px with the seat refused. "Refuse" has to
            // mean the stick reaches nothing, not "carry on with what you had".
            // ⭐ `neutral()`, THE SPELLING THIS TYPE DOCUMENTS — "construction goes
            // through `ActorControlFrame::neutral`" — and the one
            // `blank_scripted_control_frames` twelve lines up already uses for
            // the same job. `default()` is equal to it and is a second name for
            // one fact.
            control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
            // And the seat's hold leaves with its authority.
            if hold.is_some() {
                commands.entity(entity).remove::<HeldControlFrame>();
            }
            continue;
        }
        let mut input = slots.get(slot);
        // ⭐ ASKED PER SEAT, beside the seat's own control frame. A frame mode is
        // the comfort preference of the human in that chair; resolving every body
        // against one machine-wide answer is what this migration removed.
        // From the seat's own replayed frame, not from an `Update`-written table.
        let control_frame_modes = input.control_frame_modes;
        // A held stick keeps the world direction it was pressed in through a
        // turn of gravity (`HeldControlFrame`).
        let (next_hold, stick) = hold_stick_through_turns(
            hold.copied(),
            ae::Vec2::new(input.axis_x, input.axis_y),
            control_down,
            control_frame_modes.movement,
            slot,
        );
        if next_hold.as_ref() != hold {
            match next_hold {
                Some(next) => {
                    commands.entity(entity).insert(next);
                }
                None => {
                    commands.entity(entity).remove::<HeldControlFrame>();
                }
            }
        }
        input.axis_x = stick.x;
        input.axis_y = stick.y;
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
            // The player brain faces by the stick, never by `face_toward`.
            actor_half_width: kin.size.x * 0.5,
            control_down,
            movement_frame_mode: control_frame_modes.movement,
            aim_frame_mode: control_frame_modes.aim,
            actor_on_ground: ground.on_ground,
            // Human input owns facing policy. Collision facts do not implicitly
            // reverse a controlled body.
            side_contact_normal: None,
            turns_at_walls: false,
            ground_ends_ahead: false,
            // This translation path does not carry an ActorMoveset; fighter attack
            // generation is therefore inactive here.
            attack_kit: Vec::new(),
            melee_reach: None,
            running: false,
            // The player brain reads input, not the Smash aerial path; grounded
            // locomotion semantics regardless of fly mode.
            actor_aerial: false,
            alive: true,
            target_pos: kin.pos,
            target_alive: true,
            // The player brain doesn't regroup on damage; full-health is inert here.
            health_fraction: 1.0,
            // A possessed body's own guards answer nothing on this road.
            guard: Default::default(),
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
            navigation: Default::default(),
            // A person learns from their own eyes whether their move hit; the
            // player brain does not read this field.
            own_move: None,
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

#[cfg(test)]
mod tests;
