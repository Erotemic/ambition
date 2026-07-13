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
    world: ambition_world::collision::CollisionWorld,
    // Slot gestures (double-tap-down → morph) keyed by the controlling slot. The
    // body reads ITS controller's gesture, never a privileged home avatar's.
    mut slot_gestures: ResMut<crate::control::SlotInteractionState>,
    // Every CONTROLLED body (carrying `Brain::Player(slot)`) that has body-mode
    // capability + posture clusters. Not `With<PlayerEntity>`: a possessed actor with
    // the capability body-modes through the same system; a vacated home body has no
    // `Brain` so it never matches. Presence of `BodyModeCapabilities` gates it —
    // a body without the kit is skipped entirely.
    mut bodies: Query<(
        &ambition_characters::brain::Brain,
        &mut crate::actor::BodyKinematics,
        &crate::actor::BodyBaseSize,
        &mut crate::actor::BodyModeState,
        &mut crate::actor::BodyJumpState,
        &crate::actor::BodyGroundState,
        &crate::features::MotionModel,
        &crate::actor::BodyWallState,
        &crate::actor::BodyDashState,
        &crate::actor::BodyBlinkState,
        &crate::actor::BodyLedgeState,
        &crate::actor::BodyEnvironmentContact,
        &ambition_characters::brain::ActorControl,
        (
            &crate::body_mode::BodyModeCapabilities,
            &crate::actor::BodyFlightState,
            &crate::physics::ResolvedMotionFrame,
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
        brain,
        mut kinematics,
        base_size,
        mut body_mode_state,
        mut jump_state,
        ground,
        motion,
        wall,
        dash,
        blink,
        ledge,
        env_contact,
        control,
        (caps, flight, resolved_frame),
    ) in &mut bodies
    {
        // Only bodies a controller is DRIVING act — the entity carrying
        // `Brain::Player(slot)`. An AI-brained body (or a vacated home body) is
        // skipped; body mode is a controlled-body concern here.
        let Some(slot) = brain.player_slot() else {
            continue;
        };
        // Intent comes from the body's own `ActorControl` (already gravity/mode
        // resolved by the brain), and the double-tap-down morph gesture from the
        // controller's slot — never raw `ControlFrame` or a `PlayerInputFrame`.
        let control = &control.0;

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
        // The brain already resolved raw device axes into `locomotion` (local frame,
        // gravity- and input-mode-relative), so we consume THAT directly — no second
        // resolve, no `PlayerInputFrame`. `up_held` replaces the old raw up-edge for
        // the unmorph gesture: a held-up (or jump) stands the body up.
        // The body's OWN per-tick resolved frame (ADR 0024) — never a global
        // field: a possessed body inside a rotated-gravity zone crouches and
        // climbs in ITS frame.
        let gravity_dir = resolved_frame.down();
        let frame = resolved_frame.basis();
        let local_axis = control.locomotion;
        let descend = local_axis.y;
        let down_held = descend > CROUCH_AXIS_Y_THRESHOLD;
        let up_held = descend < -CROUCH_AXIS_Y_THRESHOLD;
        let climb_axis = frame.to_world(local_axis).y;
        let climb_axis_held = climb_axis.abs() > CROUCH_AXIS_Y_THRESHOLD;
        let climb_axis_down = climb_axis > CROUCH_AXIS_Y_THRESHOLD;
        let jump_pressed = control.jump_pressed;
        let dash_pressed = control.dash_pressed;
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

        // Consume the double-tap-down edge (from the controller's slot) regardless of
        // branch so we don't latch a stale signal across frames or gameplay states.
        let double_tap_down =
            std::mem::take(&mut slot_gestures.get_mut(slot).double_tap_down_pending);

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
            let exit_via_dash = dash_pressed;
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
