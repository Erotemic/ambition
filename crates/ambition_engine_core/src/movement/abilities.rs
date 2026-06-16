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
    BodyKinematics, PlayerAbilities, PlayerActionBuffer, PlayerComboTrace, PlayerDashState,
};

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
