//! Composable movement-ability functions — the limbs of the shared spine.
//!
//! Each `apply_<verb>` is a self-contained step the integration calls in a fixed
//! order. Splitting the movement monolith into these named units is the first
//! move toward the "shared physics spine + composable ability limbs" architecture
//! (see `docs/planning/non-player-centric-actor-unification.md`): an ability
//! reads + writes ONLY its own cluster fields, so it can later become an opt-in
//! component+system an actor carries or not — and an actor (enemy, NPC, boss,
//! player) is then a different *instance* of one system, differing only in which
//! ability components + tuning it holds.

use super::events::FrameEvents;
use super::input::InputState;
use super::ops::MovementOp;
use super::tuning::MovementTuning;
use crate::player_clusters::{
    BodyKinematics, PlayerAbilities, PlayerActionBuffer, PlayerBlinkState, PlayerComboTrace,
    PlayerDashState, PlayerDodgeState, PlayerFlightState, PlayerGroundState, PlayerShieldState,
    PlayerWallState,
};

/// Facing + input buffering: turn to face the stick (only when grounded or
/// flying), and buffer jump/dash presses for the short windows the sim phase
/// consumes them in. The intent step at the head of the control phase.
pub(super) fn apply_intent(
    kinematics: &mut BodyKinematics,
    ground: &PlayerGroundState,
    flight: &PlayerFlightState,
    action_buffer: &mut PlayerActionBuffer,
    abilities: &PlayerAbilities,
    input: InputState,
    tuning: MovementTuning,
) {
    let can_turn = ground.on_ground || flight.fly_enabled;
    if can_turn && input.axis_x.abs() > 0.1 {
        kinematics.facing = input.axis_x.signum();
    }
    if input.jump_pressed && abilities.abilities.jump {
        action_buffer.jump = tuning.jump_buffer;
    }
    if input.dash_pressed && abilities.abilities.dash {
        action_buffer.dash = tuning.dash_buffer;
    }
}

/// Flight toggle: flip fly mode; on entering, clear transient ground/wall/dash/
/// blink state so the body cleanly enters free flight.
pub(super) fn apply_fly_toggle(
    flight: &mut PlayerFlightState,
    wall: &mut PlayerWallState,
    dash: &mut PlayerDashState,
    blink: &mut PlayerBlinkState,
    abilities: &PlayerAbilities,
    combo_trace: &mut PlayerComboTrace,
    input: InputState,
    events: &mut FrameEvents,
) {
    if input.fly_toggle_pressed && abilities.abilities.fly {
        flight.fly_enabled = !flight.fly_enabled;
        if flight.fly_enabled {
            flight.fast_falling = false;
            wall.wall_clinging = false;
            wall.wall_climbing = false;
            dash.timer = 0.0;
            blink.grace_timer = 0.0;
        }
        events.op_clusters(combo_trace, MovementOp::FlyToggle);
    }
}

/// Dodge roll: consume a buffered dash on the ground into an i-frame roll (the
/// dodge ability claims the dash buffer before `apply_dash` would).
pub(super) fn apply_dodge(
    kinematics: &mut BodyKinematics,
    dodge: &mut PlayerDodgeState,
    action_buffer: &mut PlayerActionBuffer,
    ground: &PlayerGroundState,
    abilities: &PlayerAbilities,
    combo_trace: &mut PlayerComboTrace,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if action_buffer.dash > 0.0
        && abilities.abilities.dodge
        && ground.on_ground
        && dodge.cooldown <= 0.0
    {
        let dir = if input.axis_x.abs() > 0.1 {
            input.axis_x.signum()
        } else {
            kinematics.facing
        };
        kinematics.vel.x = dir * tuning.dodge_roll_speed;
        kinematics.vel.y = kinematics.vel.y.min(0.0);
        dodge.roll_timer = tuning.dodge_roll_time;
        dodge.cooldown = tuning.dodge_roll_cooldown;
        action_buffer.dash = 0.0;
        events.op_clusters(combo_trace, MovementOp::DodgeRoll);
    }
}

/// Shield / parry hold. Can't raise while dashing; opens a parry window on the
/// rising edge.
pub(super) fn apply_shield(
    shield: &mut PlayerShieldState,
    dash: &PlayerDashState,
    abilities: &PlayerAbilities,
    combo_trace: &mut PlayerComboTrace,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !abilities.abilities.shield {
        shield.active = false;
        shield.parry_window_timer = 0.0;
    } else {
        let can_shield = dash.timer <= 0.0;
        let want_shield = input.shield_held && can_shield;
        if want_shield && !shield.active {
            shield.parry_window_timer = tuning.parry_window_time;
            events.op_clusters(combo_trace, MovementOp::ShieldUp);
        }
        shield.active = want_shield;
    }
}

/// Variable jump height: cut the rising jump short on an early button release.
pub(super) fn apply_jump_release(
    kinematics: &mut BodyKinematics,
    abilities: &PlayerAbilities,
    input: InputState,
) {
    if abilities.abilities.variable_jump
        && input.jump_released
        && kinematics.vel.y < -120.0
    {
        kinematics.vel.y *= 0.54;
    }
}

/// Dash: a buffered, charge-gated burst that REPLACES the velocity vector and
/// opens a timed window during which the integrator skips normal physics (see
/// `integrate_velocity_clusters`'s `dash.timer > 0` branch). Picks Dash vs
/// DoubleDash by the charge count before decrement. No-op unless the actor has
/// the dash ability + a buffered press + a free charge + the cooldown clear, so
/// an actor without dash (no buffered press / `abilities.dash == false`) pays
/// only the gate check.
///
/// Order: runs in the CONTROL phase after the input buffer is populated and
/// after dodge (which consumes the same buffer on the ground first).
pub(super) fn apply_dash(
    kinematics: &mut BodyKinematics,
    dash: &mut PlayerDashState,
    action_buffer: &mut PlayerActionBuffer,
    abilities: &PlayerAbilities,
    combo_trace: &mut PlayerComboTrace,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if action_buffer.dash > 0.0
        && abilities.abilities.dash
        && dash.charges_available > 0
        && dash.cooldown <= 0.0
    {
        let fallback = bevy_math::Vec2::new(kinematics.facing, 0.0);
        let aim = bevy_math::Vec2::new(input.axis_x, input.axis_y).normalize_or(fallback);
        kinematics.vel = aim * tuning.dash_speed;
        dash.timer = tuning.dash_time;
        dash.cooldown = tuning.dash_cooldown;
        action_buffer.dash = 0.0;
        let before = dash.charges_available;
        dash.charges_available = dash.charges_available.saturating_sub(1);
        let op = if before >= 2 {
            MovementOp::DoubleDash
        } else {
            MovementOp::Dash
        };
        events.op_clusters(combo_trace, op);
    }
}
