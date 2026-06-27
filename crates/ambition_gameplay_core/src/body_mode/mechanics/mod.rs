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
//! `BodyModeState` cluster components via
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

use ambition_engine_core as ae;
use bevy::prelude::*;

/// Threshold on `axis_y` for treating Down as "held" for crouch.
/// Mirrors the threshold used by ledge-grab drop and the engine's
/// drop-through gesture so the player feel stays consistent.
const CROUCH_AXIS_Y_THRESHOLD: f32 = 0.4;

pub fn update_body_mode(
    world: crate::features::CollisionWorld,
    gravity_field: Option<Res<crate::physics::GravityField>>,
    // Optional: headless / unit-test apps may omit the settings resource. Absent →
    // Hybrid (the historical behavior).
    user_settings: Option<Res<crate::persistence::settings::UserSettings>>,
    mut player_q: Query<
        (
            &mut crate::actor::BodyKinematics,
            &crate::player::BodyBaseSize,
            &mut crate::player::BodyModeState,
            &mut crate::player::BodyJumpState,
            &crate::player::BodyGroundState,
            &crate::player::BodyWallState,
            &crate::player::BodyDashState,
            &crate::player::BodyBlinkState,
            &crate::player::BodyLedgeState,
            &crate::player::BodyEnvironmentContact,
            &mut crate::player::PlayerInteractionState,
            &crate::player::PlayerInputFrame,
            &crate::player::BodyFlightState,
        ),
        // Per-body: every player body (primary + brain-driven clone) computes its
        // OWN crouch/morph/climb posture from its own input. Iterating keeps each
        // body's body-mode independent — the clone runs the same shared system.
        With<crate::actor::PlayerEntity>,
    >,
) {
    // Body-mode changes test overhead/standing clearance against the composited
    // collision world so a moving platform / ECS solid blocks unmorphing the same
    // way authored geometry does. No room (minimal test app) → nothing to clear.
    let Some(collision) = world.solids() else {
        return;
    };
    for (
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
    ) in &mut player_q
    {
        let controls = &input.frame;

        // Mid-action mechanics own the body shape — don't fight them.
        if dash.timer > 0.0 || blink.aiming {
            continue;
        }
        // Wall / ledge state owns its own posture; reverting it via crouch
        // would break the ledge-grab anchor invariant.
        if wall.wall_clinging || wall.wall_climbing || ledge.grab.is_some() {
            continue;
        }
        // In-water posture: leave water swim mechanics alone.
        if env_contact.water.is_some() {
            continue;
        }

        // "Descend" gate: crouch is "press toward the controlled body's feet".
        // Gravity- AND input-mode-relative via the resolved local stick `y`, so it honors
        // the Screen/Hybrid setting exactly like the engine movement core (under
        // Hybrid this is the old `gravity_descend(axis_y)`).
        let gravity_dir = crate::physics::gravity_dir_or_default(gravity_field.as_deref());
        let movement_mode = user_settings
            .as_deref()
            .map_or(ae::InputFrameMode::DEFAULT_MOVEMENT, |s| s.gameplay.movement_frame_mode);
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let resolved = frame.resolve_control(movement_mode, controls.axis_x, controls.axis_y);
        let local_axis = resolved.local_axis;
        let descend = local_axis.y;
        let down_held = descend > CROUCH_AXIS_Y_THRESHOLD;
        let up_held = descend < -CROUCH_AXIS_Y_THRESHOLD;
        let climb_axis = frame.to_world(local_axis).y;
        let climb_axis_held = climb_axis.abs() > CROUCH_AXIS_Y_THRESHOLD;
        let climb_axis_down = climb_axis > CROUCH_AXIS_Y_THRESHOLD;
        let local_up_pressed = resolved.local_up_pressed(controls.raw_direction_edges());
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
                    &*collision,
                    gravity_dir,
                    solid,
                );
                continue;
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
                    &*collision,
                    gravity_dir,
                    solid,
                );
                continue;
            }
            // Otherwise stay Climbing — engine drives motion through
            // integrate_climb. No body-mode change this frame.
            continue;
        }

        // Climbing entry: resolve input to the controlled body's local frame,
        // then project that local intent onto the climbable's authored axis. The
        // engine's current climbables are vertical world-space spans, so the
        // authored climb axis is world Y for now. A downward climb input is gated
        // to NOT trigger climbing while grounded (so a floor-down press stays a
        // crouch); an upward climb input can engage from grounded as a "step onto
        // the ladder from below" gesture.
        // While flying, holding a climb direction is "fly", not "grab the ladder"
        // — flight suppresses ladder auto-climb so you can fly past / over a
        // ladder without snapping onto it. (Land or disable flight to climb.)
        let climb_initiator =
            climb_axis_held && !(climb_axis_down && on_ground && !controls.jump_pressed);
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
                &*collision,
                gravity_dir,
                solid,
            );
            continue;
        }

        // MorphBall has the smallest AABB. Exiting it means re-checking
        // overhead clearance; sourcing the exit input from `jump_pressed`
        // mirrors how a player would naturally try to "stand up" out of
        // the ball. Up-pressed (a tap, not held) is also accepted as the
        // unmorph gesture so keyboards that bind Up to a different
        // physical key can still escape the ball without committing to a
        // jump arc.
        if mode == ae::BodyMode::MorphBall {
            if controls.jump_pressed || local_up_pressed {
                let _ = ae::try_change_body_mode_clusters(
                    &mut kinematics,
                    base_size,
                    &mut body_mode_state,
                    ae::BodyMode::Standing,
                    &*collision,
                    gravity_dir,
                    solid,
                );
            }
            continue;
        }

        // Double-tap-down on the ground from Standing or Crouching curls
        // into MorphBall.
        if on_ground && double_tap_down {
            let _ = ae::try_change_body_mode_clusters(
                &mut kinematics,
                base_size,
                &mut body_mode_state,
                ae::BodyMode::MorphBall,
                &*collision,
                gravity_dir,
                solid,
            );
            continue;
        }

        let target = if down_held && on_ground {
            ae::BodyMode::Crouching
        } else {
            ae::BodyMode::Standing
        };

        if mode == target {
            continue;
        }

        let _ = ae::try_change_body_mode_clusters(
            &mut kinematics,
            base_size,
            &mut body_mode_state,
            target,
            &*collision,
            gravity_dir,
            solid,
        );
    }
}

#[cfg(test)]
mod tests;
