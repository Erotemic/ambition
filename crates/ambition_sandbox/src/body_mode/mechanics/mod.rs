//! Sandbox-side body-mode driver (crouch + morph ball + collision-safe
//! stand-up).
//!
//! Listens to the deadzoned `axis_y` from `ControlFrame` and the
//! double-tap-down gesture (`fast_fall_pressed`) and asks the engine
//! to flip the player's body mode between `Standing`, `Crouching`, and
//! `MorphBall`. `try_change_body_mode_clusters` does the per-frame
//! collision-safe resize: if a low ceiling would clip the larger body
//! the helper rejects the transition and the player stays in the
//! smaller stance. Auto-detected `PlayerModeChanged` trace events
//! fire from the trace recorder diffing `body_mode` between snapshots,
//! so this driver does not push events itself.
//!
//! Body-mode mutations happen directly on `BodyKinematics` +
//! `PlayerBodyModeState` cluster components via
//! `try_change_body_mode_clusters` — no `ae::Player` aggregate, no
//! `engine_player_bridge` round-trip (both deleted 2026-05-28).
//!
//! Input model:
//! - Standing + Down held + grounded → Crouching.
//! - Standing/Crouching + double-tap Down + grounded → MorphBall.
//! - MorphBall + Jump pressed → try Standing (gated). If a low
//!   ceiling blocks the standing body, the morph ball stays curled.
//! - Crouching + Down released → Standing (gated).
//! - Standing/Crouching + Up/Down inside `climbable_contact` → Climbing.
//! - Climbing + Up + Jump → ladder jump boost, stay Climbing.
//! - Climbing + Jump without Up or Dash → push off, exit to Standing.
//!   Climbing + losing contact → exit to Standing automatically.
//! - Mid-action mechanics (dash, blink-aim, wall-cling/climb, ledge grab,
//!   swim) own the player shape; the driver no-ops while any of them are
//!   active.

use crate::engine_core as ae;
use bevy::prelude::*;

/// Threshold on `axis_y` for treating Down as "held" for crouch.
/// Mirrors the threshold used by ledge-grab drop and the engine's
/// drop-through gesture so the player feel stays consistent.
const CROUCH_AXIS_Y_THRESHOLD: f32 = 0.4;

pub fn update_body_mode(
    world: Res<crate::GameWorld>,
    gravity_field: Option<Res<crate::physics::GravityField>>,
    mut player_q: Query<
        (
            &mut crate::player::BodyKinematics,
            &crate::player::PlayerBaseSize,
            &mut crate::player::PlayerBodyModeState,
            &mut crate::player::PlayerJumpState,
            &crate::player::PlayerGroundState,
            &crate::player::PlayerWallState,
            &crate::player::PlayerDashState,
            &crate::player::PlayerBlinkState,
            &crate::player::PlayerLedgeState,
            &crate::player::PlayerEnvironmentContact,
            &mut crate::player::PlayerInteractionState,
            &crate::player::PlayerInputFrame,
            &crate::player::PlayerFlightState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let Ok((
        mut kinematics,
        base_size,
        mut body_mode_state,
        mut jump_state,
        ground,
        wall,
        dash,
        blink,
        ledge,
        env_contact,
        mut interaction,
        input,
        flight,
    )) = player_q.single_mut()
    else {
        return;
    };
    let controls = &input.frame;

    // Mid-action mechanics own the body shape — don't fight them.
    if dash.timer > 0.0 || blink.aiming {
        return;
    }
    // Wall / ledge state owns its own posture; reverting it via crouch
    // would break the ledge-grab anchor invariant.
    if wall.wall_clinging || wall.wall_climbing || ledge.grab.is_some() {
        return;
    }
    // In-water posture: leave water swim mechanics alone.
    if env_contact.water.is_some() {
        return;
    }

    // Gravity-relative "descend" gate: crouch is "press toward your feet", which
    // flips to screen-up under inverted gravity.
    let gravity_dir = gravity_field
        .as_deref()
        .map_or(ae::Vec2::new(0.0, 1.0), |g| g.dir);
    let descend = ae::movement::gravity_descend(controls.axis_y, gravity_dir);
    let down_held = descend > CROUCH_AXIS_Y_THRESHOLD;
    let up_held = descend < -CROUCH_AXIS_Y_THRESHOLD;
    let on_ground = ground.on_ground;
    let mode = body_mode_state.body_mode;
    let solid = |b: &ae::Block| matches!(b.kind, ae::BlockKind::Solid);
    let climbable_contact_present = env_contact.climbable.is_some();

    // Consume the double-tap-down edge regardless of branch so we
    // don't latch a stale signal across frames or gameplay states.
    let double_tap_down = std::mem::take(&mut interaction.double_tap_down_pending);

    if !down_held {
        jump_state.ladder_drop_through_hold_lock = false;
    }

    // Climbing exits: plain jump / dash pushes off, losing contact
    // drops the mode. Jump+Up is handled by movement as a climb-speed
    // boost while keeping the ladder state.
    // Engine's `integrate_climb` defensive-zeros velocity if contact
    // is None mid-climb, so the visible result of a contact loss is a
    // one-frame velocity stall before this driver flips back to
    // Standing — acceptable for the first slice.
    if mode == ae::BodyMode::Climbing {
        if controls.jump_pressed && down_held {
            jump_state.ladder_drop_through_timer = ae::movement::ONE_WAY_DROP_THROUGH_GRACE;
            let _ = ae::try_change_body_mode_clusters(
                &mut kinematics,
                base_size,
                &mut body_mode_state,
                ae::BodyMode::Standing,
                &world.0,
                solid,
            );
            return;
        }
        let exit_via_jump = controls.jump_pressed && !up_held;
        let exit_via_dash = controls.dash_pressed;
        let exit_via_lost_contact = !climbable_contact_present;
        if exit_via_jump || exit_via_dash || exit_via_lost_contact {
            let _ = ae::try_change_body_mode_clusters(
                &mut kinematics,
                base_size,
                &mut body_mode_state,
                ae::BodyMode::Standing,
                &world.0,
                solid,
            );
            return;
        }
        // Otherwise stay Climbing — engine drives motion through
        // integrate_climb. No body-mode change this frame.
        return;
    }

    // Climbing entry: holding Up or Down inside a climbable contact
    // engages the ladder. Down is gated to NOT trigger climbing while
    // grounded (so a Down-press on a floor stays a crouch). Up, by
    // contrast, can engage from grounded as a "step onto the ladder
    // from below" gesture.
    // While flying, holding Up is "fly up", not "grab the ladder" — flight
    // suppresses ladder auto-climb so you can fly past / over a ladder without
    // snapping onto it. (Land or disable flight to climb.)
    let climb_initiator = up_held || (down_held && !on_ground && !controls.jump_pressed);
    if climbable_contact_present
        && climb_initiator
        && !flight.fly_enabled
        && jump_state.ladder_drop_through_timer <= 0.0
        && !jump_state.ladder_drop_through_hold_lock
        && mode != ae::BodyMode::MorphBall
    {
        let _ = ae::try_change_body_mode_clusters(
            &mut kinematics,
            base_size,
            &mut body_mode_state,
            ae::BodyMode::Climbing,
            &world.0,
            solid,
        );
        return;
    }

    // MorphBall has the smallest AABB. Exiting it means re-checking
    // overhead clearance; sourcing the exit input from `jump_pressed`
    // mirrors how a player would naturally try to "stand up" out of
    // the ball. Up-pressed (a tap, not held) is also accepted as the
    // unmorph gesture so keyboards that bind Up to a different
    // physical key can still escape the ball without committing to a
    // jump arc.
    if mode == ae::BodyMode::MorphBall {
        if controls.jump_pressed || controls.up_pressed {
            let _ = ae::try_change_body_mode_clusters(
                &mut kinematics,
                base_size,
                &mut body_mode_state,
                ae::BodyMode::Standing,
                &world.0,
                solid,
            );
        }
        return;
    }

    // Double-tap-down on the ground from Standing or Crouching curls
    // into MorphBall.
    if on_ground && double_tap_down {
        let _ = ae::try_change_body_mode_clusters(
            &mut kinematics,
            base_size,
            &mut body_mode_state,
            ae::BodyMode::MorphBall,
            &world.0,
            solid,
        );
        return;
    }

    let target = if down_held && on_ground {
        ae::BodyMode::Crouching
    } else {
        ae::BodyMode::Standing
    };

    if mode == target {
        return;
    }

    let _ = ae::try_change_body_mode_clusters(
        &mut kinematics,
        base_size,
        &mut body_mode_state,
        target,
        &world.0,
        solid,
    );
}

#[cfg(test)]
mod tests;
