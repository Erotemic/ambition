//! Composable movement-ability functions — the limbs of the shared spine.
//!
//! Each `apply_<verb>` is a self-contained step the integration calls in a fixed
//! order. Splitting the movement monolith into these named units is the first
//! move toward the "shared physics spine + composable ability limbs" architecture
//! (see `docs/planning/engine/unified-actors.md`): an ability
//! reads + writes ONLY its own cluster fields, so it can later become an opt-in
//! component+system an actor carries or not — and an actor (enemy, NPC, boss,
//! player) is then a different *instance* of one system, differing only in which
//! ability components + tuning it holds.

use super::events::FrameEvents;
use super::input::InputState;
use super::ops::MovementOp;
use super::tuning::AxisSweptParams;
use crate::body_clusters::{
    BodyAbilities, BodyActionBuffer, BodyBlinkState, BodyComboTrace, BodyDashState, BodyDodgeState,
    BodyFlightState, BodyGroundState, BodyKinematics, BodyShieldState, BodyWallState,
};
use crate::MotionFrame;

/// Facing + input buffering: turn to face the stick (only when grounded or
/// flying), and buffer jump/dash presses for the short windows the sim phase
/// consumes them in. The intent step at the head of the control phase.
pub(super) fn apply_intent(
    kinematics: &mut BodyKinematics,
    ground: &BodyGroundState,
    flight: &BodyFlightState,
    action_buffer: &mut BodyActionBuffer,
    abilities: &BodyAbilities,
    input: InputState,
    tuning: AxisSweptParams,
) {
    let can_turn = ground.on_ground || flight.fly_enabled;
    let local_stick = input.local_axis();
    if can_turn && local_stick.x.abs() > 0.1 {
        kinematics.facing = local_stick.x.signum();
    }
    if input.jump_pressed && abilities.abilities.jump {
        action_buffer.jump = tuning.locomotion.jump_buffer;
    }
    if input.dash_pressed && abilities.abilities.dash {
        action_buffer.dash = tuning.abilities.dash_buffer;
    }
}

/// Flight toggle: flip fly mode; on entering, clear transient ground/wall/dash/
/// blink state so the body cleanly enters free flight.
pub(super) fn apply_fly_toggle(
    flight: &mut BodyFlightState,
    wall: &mut BodyWallState,
    dash: &mut BodyDashState,
    blink: &mut BodyBlinkState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
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
    dodge: &mut BodyDodgeState,
    action_buffer: &mut BodyActionBuffer,
    ground: &BodyGroundState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    if action_buffer.dash > 0.0
        && abilities.abilities.dodge
        && ground.on_ground
        && dodge.cooldown <= 0.0
    {
        let local_stick = input.local_axis();
        let dir = if local_stick.x.abs() > 0.1 {
            local_stick.x.signum()
        } else {
            kinematics.facing
        };
        let descend = kinematics.vel.dot(frame.down()).min(0.0);
        kinematics.vel =
            frame.side() * (dir * tuning.abilities.dodge_roll_speed) + frame.down() * descend;
        dodge.roll_timer = tuning.abilities.dodge_roll_time;
        dodge.cooldown = tuning.abilities.dodge_roll_cooldown;
        action_buffer.dash = 0.0;
        events.op_clusters(combo_trace, MovementOp::DodgeRoll);
    }
}

/// The ONE shield-activation rule, shared by the player body and every actor body
/// (roadmap S6b convergence / invariant I3 — the body owns the gate, the
/// controller only attempts). Given the controller's held-shield attempt and the
/// body's gates — does it have the shield ability, and is it mid-dash (you can't
/// raise a guard while dashing) — it resolves the raised state and refreshes the
/// parry window on the *rising edge*. Returns `true` iff a FRESH guard was raised
/// this tick (the edge that opens a parry window / emits a `ShieldUp` op), so the
/// caller can fire its own side effect. Pure + frame-agnostic.
///
/// The player's [`apply_shield`] and the actor resolver in `update_ecs_actors`
/// both call this, so "raise the guard" is one implementation, not two.
pub fn resolve_shield(
    active: &mut bool,
    parry_window_timer: &mut f32,
    ability_enabled: bool,
    dash_active: bool,
    shield_held: bool,
    parry_window_time: f32,
) -> bool {
    if !ability_enabled {
        *active = false;
        *parry_window_timer = 0.0;
        return false;
    }
    let want = shield_held && !dash_active;
    let fresh = want && !*active;
    if fresh {
        *parry_window_timer = parry_window_time;
    }
    *active = want;
    fresh
}

/// Shield / parry hold. Can't raise while dashing; opens a parry window on the
/// rising edge. Thin player-side wrapper over the shared [`resolve_shield`] rule.
pub(super) fn apply_shield(
    shield: &mut BodyShieldState,
    dash: &BodyDashState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    let fresh = resolve_shield(
        &mut shield.active,
        &mut shield.parry_window_timer,
        abilities.abilities.shield,
        dash.timer > 0.0,
        input.shield_held,
        tuning.abilities.parry_window_time,
    );
    if fresh {
        events.op_clusters(combo_trace, MovementOp::ShieldUp);
    }
}

/// Variable jump height: cut the rising jump short on an early button release.
pub(super) fn apply_jump_release(
    kinematics: &mut BodyKinematics,
    abilities: &BodyAbilities,
    input: InputState,
    frame: MotionFrame,
) {
    let ascend_speed = -kinematics.vel.dot(frame.down());
    if abilities.abilities.variable_jump && input.jump_released && ascend_speed > 120.0 {
        let along_down = kinematics.vel.dot(frame.down());
        kinematics.vel += frame.down() * (along_down * 0.54 - along_down);
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
    dash: &mut BodyDashState,
    action_buffer: &mut BodyActionBuffer,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    if action_buffer.dash > 0.0
        && abilities.abilities.dash
        && dash.charges_available > 0
        && dash.cooldown <= 0.0
    {
        let fallback = bevy_math::Vec2::new(kinematics.facing, 0.0);
        let aim = input.local_axis().normalize_or(fallback);
        kinematics.vel = frame.to_world(aim) * tuning.abilities.dash_speed;
        dash.timer = tuning.abilities.dash_time;
        dash.cooldown = tuning.abilities.dash_cooldown;
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

#[cfg(test)]
mod resolve_shield_tests {
    use super::resolve_shield;

    /// The shared rule's contract (the one both the player wrapper and the actor
    /// resolver depend on): ability-gated, rising-edge parry, dash-blocked, sustain.
    #[test]
    fn resolve_shield_is_the_one_rule() {
        // Disabled ability forces the guard down and clears the parry window.
        let (mut active, mut parry) = (true, 0.5);
        let fresh = resolve_shield(&mut active, &mut parry, false, false, true, 0.2);
        assert!(!active && parry == 0.0 && !fresh, "no ability → no guard");

        // Rising edge: a held shield with the ability raises a FRESH guard and opens
        // the parry window.
        let (mut active, mut parry) = (false, 0.0);
        let fresh = resolve_shield(&mut active, &mut parry, true, false, true, 0.2);
        assert!(
            active && parry == 0.2 && fresh,
            "rising edge opens a fresh parry"
        );

        // Held across a second tick: still raised, but NOT a fresh edge (no re-arm).
        let fresh = resolve_shield(&mut active, &mut parry, true, false, true, 0.2);
        assert!(active && !fresh, "sustained hold is not a fresh parry");

        // Can't raise while dashing — the gate that binds the player AND the actor.
        let (mut active, mut parry) = (false, 0.0);
        let fresh = resolve_shield(&mut active, &mut parry, true, true, true, 0.2);
        assert!(!active && !fresh, "dashing blocks the guard");

        // Release drops the guard (sustain re-evaluated every tick).
        let (mut active, mut parry) = (true, 0.2);
        resolve_shield(&mut active, &mut parry, true, false, false, 0.2);
        assert!(!active, "releasing the button drops the guard");
    }
}
