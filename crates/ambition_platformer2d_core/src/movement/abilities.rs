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
use super::model::AxisManeuverState;
use super::ops::MovementOp;
use super::tuning::AxisSweptParams;
use crate::body_clusters::{
    BodyAbilities, BodyComboTrace, BodyDashState, BodyDodgeState, BodyFlightState, BodyGroundState,
    BodyKinematics, BodyShieldState,
};
use crate::MotionFrame;

/// Facing + input buffering: turn to face the stick (only when grounded or
/// flying), and buffer jump/dash presses for the short windows the sim phase
/// consumes them in. The intent step at the head of the control phase.
pub(super) fn apply_intent(
    kinematics: &mut BodyKinematics,
    ground: &BodyGroundState,
    flight: &BodyFlightState,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    input: InputState,
    tuning: AxisSweptParams,
) {
    let can_turn = ground.on_ground || flight.fly_enabled;
    let local_stick = input.local_axis();
    if can_turn && local_stick.x.abs() > 0.1 {
        kinematics.facing = local_stick.x.signum();
    }
    if input.jump_pressed() && abilities.abilities.jump {
        state.buffer_jump = tuning.locomotion.jump_buffer;
    }
    if input.dash_pressed() && abilities.abilities.dash {
        state.buffer_dash = tuning.abilities.dash_buffer;
    }
}

/// Flight toggle: flip fly mode; on entering, clear transient wall/dash/blink
/// maneuver state so the body cleanly enters free flight.
pub(super) fn apply_fly_toggle(
    flight: &mut BodyFlightState,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    events: &mut FrameEvents,
) {
    if input.fly_toggle_pressed() && abilities.abilities.fly && abilities.abilities.fly_toggle {
        flight.fly_enabled = !flight.fly_enabled;
        if flight.fly_enabled {
            state.fast_falling = false;
            state.wall_clinging = false;
            state.wall_climbing = false;
            state.dash_timer = 0.0;
            state.blink_grace_timer = 0.0;
            state.phased_jump.clear();
        }
        events.op_clusters(combo_trace, MovementOp::FlyToggle);
    }
}

/// **The evade, on the ground and in the air.** A buffered dash spent by a body
/// that owns the dodge ability becomes a roll when its feet are down and an air
/// dodge when they are not (the dodge ability claims the dash buffer before
/// `apply_dash` would).
///
/// The two share the buffer, the ability gate and the i-frame *idea*, and
/// nothing else — see [`AxisManeuverState::air_dodge_timer`] for why they do not
/// share a timer. Their commitments differ in kind:
///
/// ```text
///                  gate                travel            spent against
/// ground roll      on_ground           facing/stick x     a cooldown clock
/// air dodge        airborne, unspent   full 2D stick      this trip through the air
/// ```
///
/// ⚠ **an air dodge with `air_dodge_time <= 0.0` is a body that has none.** The
/// tuning defaults are `#[serde(default)]`, so every authored body baked before
/// the maneuver existed keeps exactly the movement it had; a body opts in by
/// authoring a window.
pub(super) fn apply_dodge(
    kinematics: &mut BodyKinematics,
    dodge: &mut BodyDodgeState,
    state: &mut AxisManeuverState,
    ground: &BodyGroundState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    if state.buffer_dash <= 0.0 || !abilities.abilities.dodge {
        return;
    }
    let local_stick = input.local_axis();
    if ground.on_ground {
        if dodge.cooldown > 0.0 {
            return;
        }
        let dir = if local_stick.x.abs() > 0.1 {
            local_stick.x.signum()
        } else {
            kinematics.facing
        };
        let descend = kinematics.vel.dot(frame.down()).min(0.0);
        kinematics.vel =
            frame.side() * (dir * tuning.abilities.dodge_roll_speed) + frame.down() * descend;
        state.dodge_roll_timer = tuning.abilities.dodge_roll_time;
        state.phased_jump.clear();
        dodge.cooldown = tuning.abilities.dodge_roll_cooldown;
        state.buffer_dash = 0.0;
        events.op_clusters(combo_trace, MovementOp::DodgeRoll);
        return;
    }
    // ── airborne ────────────────────────────────────────────────────────────
    // ⛔ the budget is checked BEFORE the buffer is consumed, so a body that has
    // already dodged this airtime leaves the dash buffer standing rather than
    // silently eating it — the buffered input goes on to mean what it would have
    // meant without the dodge ability at all.
    if dodge.air_dodge_spent
        || tuning.abilities.air_dodge_time <= 0.0
        || state.air_dodge_timer > 0.0
        || state.air_dodge_endlag_timer > 0.0
    {
        return;
    }
    // The stick aims the evade in the body's own frame: sideways, up, down or
    // any diagonal. A neutral stick dodges in place — a real option, not a
    // degenerate one, because the invulnerability is the point and standing
    // still keeps the body where its drift left it.
    let aim = if local_stick.length_squared() > 0.01 {
        local_stick.normalize()
    } else {
        bevy_math::Vec2::ZERO
    };
    // ⚠ **local `y` points toward the FEET**, the same convention
    // `wants_drop_through` reads — so the stick's y composes with `down()`, not
    // against it. Negating here would have aimed every "dodge down through the
    // stage" upward, which is the exact input a recovering body uses.
    kinematics.vel =
        (frame.side() * aim.x + frame.down() * aim.y) * tuning.abilities.air_dodge_speed;
    state.air_dodge_timer = tuning.abilities.air_dodge_time;
    state.air_dodge_endlag_timer = 0.0;
    state.fast_falling = false;
    state.phased_jump.clear();
    dodge.air_dodge_spent = true;
    state.buffer_dash = 0.0;
    events.op_clusters(combo_trace, MovementOp::AirDodge);
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
    state: &AxisManeuverState,
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
        state.dash_timer > 0.0,
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
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    if !input.jump_released() {
        return;
    }
    cut_ascent_now(kinematics, state, abilities, frame, tuning);
}

/// The variable-jump CUT itself, with the release edge already established by
/// the caller. Split out because a jump-squat swallows the release edge — the
/// button can come up mid-crouch, when there is no ascent to shorten — so the
/// squat's takeoff replays the cut at the instant the body actually leaves the
/// ground. ⛔ that is a deferred release, not a second "short hop" mechanic:
/// whichever [`super::tuning::AxisJumpLaw`] the body authored still decides
/// what shortening means.
pub(super) fn cut_ascent_now(
    kinematics: &mut BodyKinematics,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    if !abilities.abilities.variable_jump {
        return;
    }
    match tuning.locomotion.jump_law {
        super::tuning::AxisJumpLaw::VelocityCut => {
            let ascend_speed = -kinematics.vel.dot(frame.down());
            if ascend_speed > 120.0 {
                let along_down = kinematics.vel.dot(frame.down());
                kinematics.vel += frame.down() * (along_down * 0.54 - along_down);
            }
        }
        super::tuning::AxisJumpLaw::PhasedGravity(_) => {
            // Do not rewrite velocity. The next integration tick observes the
            // latched release and applies the stronger gravity phase in the
            // body's current resolved frame.
            state.phased_jump.cancel_hold();
        }
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
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    if state.buffer_dash > 0.0
        && abilities.abilities.dash
        && dash.charges_available > 0
        && dash.cooldown <= 0.0
    {
        let fallback = bevy_math::Vec2::new(kinematics.facing, 0.0);
        let aim = input.local_axis().normalize_or(fallback);
        kinematics.vel = frame.to_world(aim) * tuning.abilities.dash_speed;
        state.dash_timer = tuning.abilities.dash_time;
        state.phased_jump.clear();
        dash.cooldown = tuning.abilities.dash_cooldown;
        state.buffer_dash = 0.0;
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
