//! Player brain — translates `PlayerInputFrame` into the abstract
//! intent fields of [`crate::actor::control::ActorControlFrame`].
//!
//! The player brain is **purely a translation layer**. It does not
//! make any gameplay decisions — every decision (variable-height
//! jump, dash window, projectile charge) lives in the integration
//! stage, the same way an enemy brain decides "I want to fire" but
//! the projectile-spawner system handles the cooldown gating.
//!
//! Every `ControlFrame` field the player simulation needs survives
//! this translation, including the player-specific verbs
//! (`pogo_pressed`, `blink_*`, `fast_fall_pressed`, `fly_toggle_pressed`,
//! `projectile_*`, `aim`). The sandbox's `engine_input_from_actor_control`
//! builds the engine's `InputState` purely from `ActorControl`; the
//! raw `ControlFrame` is no longer consulted inside the player
//! simulation phases.

use ambition_engine_core as ae;

use super::PlayerSlot;
use ambition_engine_core::ControlFrame;

use super::snapshot::BrainSnapshot;

/// Translate a single player's input into the abstract intent
/// fields of an `ActorControlFrame`.
///
/// The snapshot supplies the actor's facing; the input supplies the
/// rest. The function is deterministic given (input, snapshot), so
/// it's safe to call from tests and replay without an `App`.
///
/// `slot` is part of the signature for symmetry with future
/// `Brain::Remote(peer_id)` and so a multi-player driver can route
/// per-slot inputs without changing the seam.
pub fn tick_player_brain(
    _slot: PlayerSlot,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    // Per Chunk 4e: BrainSnapshot now carries the player's input
    // frame as an Option. When present we delegate to the
    // input-aware translator. When absent (e.g. an actor that has
    // a `Brain::Player(slot)` brain but no PlayerInputFrame in
    // scope), emit a neutral frame + facing so the integration
    // doesn't see garbage.
    if let Some(ref input) = snapshot.player_input {
        tick_player_brain_from_control(input, snapshot, out);
        return;
    }
    *out = crate::actor::control::ActorControlFrame::neutral();
    out.facing = snapshot.actor_facing;
}

/// Translate a raw [`ControlFrame`] into the abstract intent fields
/// of an `ActorControlFrame`. This is the core of the player brain
/// — the wrappers above add convenient input-shape adapters but
/// every translation goes through this function.
pub fn tick_player_brain_from_control(
    c: &ControlFrame,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    *out = crate::actor::control::ActorControlFrame::neutral();

    // Directional verbs interpret raw input in the controlled body's local
    // frame. This is the important seam for facing, attacks, crouch-like
    // edges, and future possessed actors: unqualified left/right/up/down means
    // local to the controlled body, not privileged screen/player space.
    let frame = ae::AccelerationFrame::new(snapshot.control_down);
    let resolved = frame.resolve_control(
        snapshot.movement_frame_mode,
        ambition_engine_core::ScreenAxes::new(c.axis_x, c.axis_y),
    );
    let local_axis = resolved.local_axes.vec();
    let raw_aim = ae::Vec2::new(c.aim_x, c.aim_y);
    let local_aim = if raw_aim.length() > 0.1 {
        frame
            .resolve_input(
                snapshot.aim_frame_mode,
                ambition_engine_core::ScreenAxes::new(c.aim_x, c.aim_y),
            )
            .vec()
            .normalize_or_zero()
    } else {
        ae::Vec2::ZERO
    };

    // Movement axis → desired velocity. At the ActorControl seam, unqualified
    // direction is controlled-body-local: x = local side/right, y = local
    // down/toward-feet. Downstream movement code should not re-resolve this
    // through the raw input frame.
    out.locomotion = local_axis;
    // Body-generic free-mover steering. A grounded integrator reads the
    // normalized `locomotion` stick (scaled by the body's own run capability);
    // a FLYING body (free-mover, or a hybrid with flight toggled on) steers by
    // absolute `velocity_target` instead. Deriving it here from the snapshot's
    // run capability keeps `Brain::Player` fully body-generic: a possessed flyer
    // moves at its own speed with no possession-specific plumbing. The human
    // player passes `max_run_speed == 0` (its integrator ignores this field), so
    // this is inert for the grounded avatar.
    out.velocity_target = local_axis * snapshot.max_run_speed;

    // Facing: prefer local side intent; fall back to snapshot facing when stick
    // is neutral so the actor doesn't snap to (0).
    out.facing = if local_axis.x.abs() > 0.01 {
        local_axis.x.signum()
    } else {
        snapshot.actor_facing
    };

    // Combat verbs.
    out.melee_pressed = c.attack_pressed;
    // Per-tilt direction for the attack, in the controlled body's local frame.
    // Zero still means "use facing".
    out.attack_axis = local_axis;

    // Projectile: held + released path stays in the player's
    // existing charge state machine for now. The brain just
    // surfaces "pressed" via fire on the release edge.
    if c.projectile_released {
        // Direction: preserve the controlled-body-local aim through the
        // ActorFireRequest seam. The ranged consumer converts at the spawn seam,
        // so arbitrary acceleration-frame orientation remains a consumer policy
        // instead of a hidden world-axis assumption here.
        let local_dir = if local_aim.length() > 0.1 {
            local_aim
        } else {
            ae::Vec2::new(snapshot.actor_facing, 0.0)
        };
        let dir = local_dir.normalize_or_zero();
        out.fire = Some(crate::actor::control::ActorFireRequest::controlled_body_local(dir, 0.0));
    }

    // Jump edges + sustain.
    out.jump_pressed = c.jump_pressed;
    out.jump_held = c.jump_held;
    out.jump_released = c.jump_released;

    // Drop-through: the engine derives this from `down + jump_pressed`
    // inside `engine_input_from_actor_control`. The brain leaves
    // `drop_through` at its default; the engine's gesture-detection
    // logic owns the final flag.
    out.drop_through = false;
    // Human-controlled bodies do not become passive contact hazards
    // unless a specific mode opts in.
    out.body_contact_damage_enabled = false;

    // Dash, interact, shield, special.
    out.dash_pressed = c.dash_pressed;
    out.interact_pressed = c.interact_pressed;
    out.shield_held = c.shield_held;
    // No dedicated "special" input today — `blink_pressed` is the
    // closest analog (blink is the player's signature ability).
    // Promote that to `special_pressed` so a `Brain::Player` driving
    // a different actor's ActionSet can resolve special there.
    out.special_pressed = c.blink_pressed;

    // Player-specific verbs (pogo, blink, fly_toggle, fast_fall,
    // projectile charge, aim). Promoted onto the frame so the
    // sandbox's player simulation can read `ActorControl` only and
    // drop the raw `ControlFrame` dependency. AI brains leave
    // these at their defaults.
    out.pogo_pressed = c.pogo_pressed;
    out.fast_fall_pressed = c.fast_fall_pressed;
    out.fly_toggle_pressed = c.fly_toggle_pressed;
    out.projectile_pressed = c.projectile_pressed;
    out.projectile_held = c.projectile_held;
    out.projectile_released = c.projectile_released;
    out.blink_pressed = c.blink_pressed;
    out.blink_held = c.blink_held;
    out.blink_released = c.blink_released;
    // Blink steers with the LOCOMOTION stick, but the two forms use different
    // frame policies, resolved here (the seam) into WORLD space so the movement
    // engine stays frame-agnostic. Quick blink follows the movement mode (already
    // baked into `local_axis`); precision blink follows the aim mode (screen-
    // directed by default), so a precision blink points where the stick points on
    // screen under any gravity.
    out.blink_quick_dir = frame.to_world(local_axis);
    out.blink_aim_step = frame.to_world(
        frame
            .resolve_input(
                snapshot.aim_frame_mode,
                ambition_engine_core::ScreenAxes::new(c.axis_x, c.axis_y),
            )
            .vec(),
    );
    out.aim = local_aim;
}

#[cfg(test)]
mod tests;
