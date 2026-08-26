//! Controlled-body mode driver for crouch, morph ball, and climbing.
//!
//! `try_change_body_mode_clusters` owns collision-safe resizing. Mid-action
//! mechanics that own body shape suppress mode transitions. Any driven body with
//! `BodyModeCapabilities` uses this path; controller kind does not select a
//! separate simulation path.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

/// Threshold on `axis_y` for treating Down as "held" for crouch.
/// Mirrors the threshold used by ledge-grab drop and the engine's
/// drop-through gesture so the player feel stays consistent.
const CROUCH_AXIS_Y_THRESHOLD: f32 = 0.4;

pub fn update_body_mode(
    world: ambition_platformer2d_world::collision::CollisionWorld,
    // Slot-scoped gesture edges for the participant driving each body.
    mut slot_gestures: ResMut<ambition_characters::control::SlotInteractionState>,
    // Capability-gated driven bodies; no `PlayerEntity` identity filter.
    mut bodies: Query<(
        &ambition_characters::control::DrivingParticipant,
        &mut crate::actor::BodyKinematics,
        &crate::actor::BodyBaseSize,
        &mut crate::actor::BodyModeState,
        &mut crate::actor::BodyJumpState,
        &crate::actor::BodyGroundState,
        &crate::features::MotionModel,
        &ae::BodyMotionFacts,
        &crate::actor::BodyEnvironmentContact,
        &ambition_characters::control::ActorControl,
        (
            &crate::body_mode::BodyModeCapabilities,
            &crate::actor::BodyFlightState,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        ),
    )>,
) {
    // Body-mode changes test overhead/standing clearance against the composited
    // collision world so a moving platform / ECS solid blocks unmorphing the same
    // way authored geometry does. No room (minimal test app) → nothing to clear.
    let Some(collision) = world.solids() else {
        return;
    };
    for (
        driver,
        mut kinematics,
        base_size,
        mut body_mode_state,
        mut jump_state,
        ground,
        motion,
        facts,
        env_contact,
        control,
        (caps, flight, resolved_frame),
    ) in &mut bodies
    {
        // `DrivingParticipant` is the control-authority filter.
        let slot = driver.0;
        // ActorControl supplies body-local intent; gesture edges remain slot-scoped.
        let control = &control.0;

        // Mid-action mechanics own the body shape — don't fight them.
        if facts.dashing || facts.blink_aiming {
            continue;
        }
        // Wall / ledge state owns its own posture; reverting it via crouch
        // would break the ledge-grab anchor invariant.
        if facts.wall_clinging || facts.wall_climbing || facts.ledge.is_some() {
            continue;
        }
        // In-water posture: leave water swim mechanics alone.
        if env_contact.water.is_some() {
            continue;
        }

        // Crouch/climb directions use the body's resolved motion frame.
        let gravity_dir = resolved_frame.down();
        let frame = resolved_frame.basis();
        let local_axis = control.locomotion;
        let descend = local_axis.y;
        let down_held = descend > CROUCH_AXIS_Y_THRESHOLD;
        let up_held = descend < -CROUCH_AXIS_Y_THRESHOLD;
        let climb_axis = frame.to_world(local_axis.vec()).y;
        let climb_axis_held = climb_axis.abs() > CROUCH_AXIS_Y_THRESHOLD;
        let climb_axis_down = climb_axis > CROUCH_AXIS_Y_THRESHOLD;
        let jump_pressed = control.jump_pressed;
        let burst_pressed = control.burst_pressed;
        let stand_up_gesture = jump_pressed || up_held;
        // Momentum bodies publish support through their ride state. The generic
        // AABB ground cluster can remain false while a body is attached to a
        // chain or block boundary, so body-mode policy must consume the unified
        // support fact rather than privileging one movement model.
        let on_ground = ground.on_ground
            || matches!(
                motion,
                crate::features::MotionModel::SurfaceMomentum(momentum)
                    if matches!(
                        momentum.state,
                        ae::SurfaceMotion::Riding { .. }
                    )
            );
        let mode = body_mode_state.body_mode;
        let solid = |b: &ae::Block| matches!(b.kind, ae::BlockKind::Solid);
        let climbable_contact_present = env_contact.climbable.is_some();

        // Consume the slot edge once; a missing slot fails closed to `false`.
        let double_tap_down = slot_gestures
            .get_mut(slot)
            .map(|gestures| std::mem::take(&mut gestures.double_tap_down_pending))
            .unwrap_or(false);

        if !down_held {
            jump_state.ladder_drop_through_hold_lock = false;
        }

        // Climbing exits: plain jump / the burst press pushes off, losing contact drops the
        // mode. Jump+Up is handled by movement as a climb-speed boost while keeping the ladder
        // state.
        if mode == ae::BodyMode::Climbing {
            if jump_pressed && down_held {
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
            let exit_via_jump = jump_pressed && !up_held;
            let exit_via_burst = burst_pressed;
            let exit_via_lost_contact = !climbable_contact_present;
            if exit_via_jump || exit_via_burst || exit_via_lost_contact {
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
        let climb_initiator = climb_axis_held && !(climb_axis_down && on_ground && !jump_pressed);
        if caps.can_climb
            && climbable_contact_present
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
            if stand_up_gesture {
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
        // into MorphBall — only if this body can morph.
        if caps.can_morph && on_ground && double_tap_down {
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

        // Crouch only if this body can crouch; otherwise it stays Standing.
        let target = if caps.can_crouch && down_held && on_ground {
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
