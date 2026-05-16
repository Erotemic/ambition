//! Sandbox-side body-mode driver (crouch + morph ball + collision-safe
//! stand-up).
//!
//! Listens to the deadzoned `axis_y` from `ControlFrame` and the
//! double-tap-down gesture (`fast_fall_pressed`) and asks the engine
//! to flip `Player::body_mode` between `Standing`, `Crouching`, and
//! `MorphBall`. `try_change_body_mode` does the per-frame
//! collision-safe resize: if a low ceiling would clip the larger body
//! the helper rejects the transition and the player stays in the
//! smaller stance. Auto-detected `PlayerModeChanged` trace events
//! fire from the trace recorder diffing `player.body_mode` between
//! snapshots, so this driver does not push events itself.
//!
//! Input model:
//! - Standing + Down held + grounded → Crouching.
//! - Standing/Crouching + double-tap Down + grounded → MorphBall.
//! - MorphBall + Jump pressed → try Standing (gated). If a low
//!   ceiling blocks the standing body, the morph ball stays curled.
//! - Crouching + Down released → Standing (gated).
//! - Standing/Crouching + Up/Down inside `climbable_contact` → Climbing.
//! - Climbing + Jump → push off, exit to Standing. Climbing + losing
//!   contact → exit to Standing automatically.
//! - Mid-action mechanics (dash, blink-aim, wall-cling/climb, ledge grab,
//!   swim) own the player shape; the driver no-ops while any of them are
//!   active.
//!
//! Runs in the progression chain after `sandbox_update` because body resize is
//! still a sandbox-side affordance. Ledge grab and swim now live in the engine
//! movement pipeline; this driver only avoids fighting their active states. The
//! size/pos delta is constrained to the body-mode swap (no horizontal
//! repositioning), so the next simulator tick treats it as a clean smaller AABB
//! and collision repair runs as usual against any new geometry. The engine
//! still gates `fast_fall_pressed` on `!on_ground`, so using the same gesture for
//! grounded morph and airborne fast-fall has no input crosstalk.

use ambition_engine as ae;
use bevy::prelude::*;

/// Threshold on `axis_y` for treating Down as "held" for crouch.
/// Mirrors the threshold used by ledge-grab drop and the engine's
/// drop-through gesture so the player feel stays consistent.
const CROUCH_AXIS_Y_THRESHOLD: f32 = 0.4;

pub fn update_body_mode(
    world: Res<crate::GameWorld>,
    controls: Res<crate::input::ControlFrame>,
    mut player_q: Query<
        (
            &mut crate::player::PlayerMovementAuthority,
            &mut crate::player::PlayerInteractionState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let Ok((mut authority, mut interaction)) = player_q.single_mut() else {
        return;
    };
    let player = &mut authority.player;

    // Mid-action mechanics own the body shape — don't fight them.
    if player.dash_timer > 0.0 || player.blink_aiming {
        return;
    }
    // Wall / ledge state owns its own posture; reverting it via crouch
    // would break the ledge-grab anchor invariant.
    if player.wall_clinging || player.wall_climbing || player.ledge_grab.is_some() {
        return;
    }
    // In-water posture: leave water swim mechanics alone.
    if player.water_contact.is_some() {
        return;
    }

    let down_held = controls.axis_y > CROUCH_AXIS_Y_THRESHOLD;
    let up_held = controls.axis_y < -CROUCH_AXIS_Y_THRESHOLD;
    let on_ground = player.on_ground;
    let mode = player.body_mode;
    let solid = |b: &ae::Block| matches!(b.kind, ae::BlockKind::Solid);
    let climbable_contact_present = player.climbable_contact.is_some();

    // Consume the double-tap-down edge regardless of branch so we
    // don't latch a stale signal across frames or gameplay states.
    let double_tap_down = std::mem::take(&mut interaction.double_tap_down_pending);

    // Climbing exits: jump pushes off, losing contact drops the mode.
    // Engine's `integrate_climb` defensive-zeros velocity if contact
    // is None mid-climb, so the visible result of a contact loss is a
    // one-frame velocity stall before this driver flips back to
    // Standing — acceptable for the first slice. Future polish can
    // grant the player a small "let-go" velocity here so falling off
    // the bottom feels natural.
    if mode == ae::BodyMode::Climbing {
        let exit_via_jump = controls.jump_pressed;
        let exit_via_lost_contact = !climbable_contact_present;
        if exit_via_jump || exit_via_lost_contact {
            let _ = ae::try_change_body_mode(player, ae::BodyMode::Standing, &world.0, solid);
            // Falling-through bottom of ladder gets the player a
            // small downward nudge so they don't hover at the bottom
            // edge waiting for gravity to take over. Pushing off via
            // jump leaves vel.y as the engine wrote it (Climbing
            // sets vel = (0, 0) on each tick when input is zero, so
            // the integrate path on the next frame will compute a
            // proper jump impulse from `jump_pressed`).
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
    // from below" gesture. This mirrors the LDtk authoring intent
    // where ladders typically begin at floor level.
    let climb_initiator = (up_held) || (down_held && !on_ground);
    if climbable_contact_present && climb_initiator && mode != ae::BodyMode::MorphBall {
        let _ = ae::try_change_body_mode(player, ae::BodyMode::Climbing, &world.0, solid);
        return;
    }

    // MorphBall has the smallest AABB. Exiting it means re-checking
    // overhead clearance; sourcing the exit input from `jump_pressed`
    // mirrors how a player would naturally try to "stand up" out of
    // the ball. Up-pressed (a tap, not held) is also accepted as the
    // unmorph gesture so keyboards that bind Up to a different
    // physical key can still escape the ball without committing to a
    // jump arc — useful for testing on layouts where Jump and Up
    // map to the same key.
    if mode == ae::BodyMode::MorphBall {
        if controls.jump_pressed || controls.up_pressed {
            let _ = ae::try_change_body_mode(player, ae::BodyMode::Standing, &world.0, solid);
        }
        return;
    }

    // Double-tap-down on the ground from Standing or Crouching curls
    // into MorphBall. The signal is `PlayerInteractionState::double_tap_down_pending`,
    // set by `input_timer_phase` via the ECS component. The engine gates
    // fast_fall on `!on_ground` already, so the same gesture firing
    // morph-ball when grounded has no input crosstalk.
    if on_ground && double_tap_down {
        let _ = ae::try_change_body_mode(player, ae::BodyMode::MorphBall, &world.0, solid);
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

    // The engine helper does the resize-with-fit check; ignore the
    // boolean result — a blocked stand-up is the desired UX (player
    // stays crouched under the ceiling) and the auto-trace diff will
    // surface a successful transition.
    let _ = ae::try_change_body_mode(player, target, &world.0, solid);
}
