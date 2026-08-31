//! Player-brain translation from slot-selected [`ControlFrame`] values to
//! [`crate::actor::control::ActorControlFrame`] intent.
//!
//! Gameplay decisions remain in shared integration systems rather than a
//! player-specific simulation path. [`tick_player_brain_from_control`]
//! exhaustively destructures `ControlFrame`, forcing each new input field to be
//! considered by the human-control translation.

use ambition_platformer2d_core as ae;

use crate::control::PlayerSlot;
use ambition_platformer2d_core::ControlFrame;

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
    // Missing slot input produces neutral intent so stale input cannot survive.
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

    // No `..`: adding an input field must make this translation choose how to carry it.
    let ControlFrame {
        // ── carried below, verbatim or after interpretation ──
        axis_x: _,
        axis_y: _,
        aim_x: _,
        aim_y: _,
        jump_pressed: _,
        jump_held: _,
        jump_released: _,
        burst_pressed: _,
        interact_pressed: _,
        shield_held: _,
        grab_pressed: _,
        taunt_pressed: _,
        special_pressed: _,
        special_held: _,
        attack_pressed: _,
        attack_held: _,
        attack_released: _,
        attack_strength_hint: _,
        pogo_pressed: _,
        fast_fall_pressed: _,
        fly_toggle_pressed: _,
        blink_pressed: _,
        blink_held: _,
        blink_released: _,
        projectile_pressed: _,
        projectile_held: _,
        projectile_released: _,
        modifier_held: _,
        modifier_pressed: _,
        // ── deliberately NOT carried, each for a stated reason ──
        // The raw direction EDGES. The body reads the RESOLVED axes above and
        // `locomotion`; carrying these too would be a second answer to "which
        // way", unrotated by the body's own frame — which under arbitrary
        // gravity is a different direction.
        left_pressed: _,
        right_pressed: _,
        up_pressed: _,
        down_pressed: _,
        // Interact is an EDGE verb at the body. A sustained interact has no
        // body meaning today; the day one does, it is carried here.
        interact_held: _,
        // Shell-level, not body verbs: pause and reset belong to the session,
        // and a body that could read them could act on somebody else's menu.
        reset_pressed: _,
        start_pressed: _,
    } = c;
    out.facing = snapshot.actor_facing;

    // Directional verbs interpret raw input in the controlled body's local
    // frame. This is the important seam for facing, attacks, crouch-like
    // edges, and future possessed actors: unqualified left/right/up/down means
    // local to the controlled body, not privileged screen/player space.
    let frame = ae::AccelerationFrame::new(snapshot.control_down);
    let resolved = frame.resolve_control(
        snapshot.movement_frame_mode,
        ambition_platformer2d_core::ScreenAxes::new(c.axis_x, c.axis_y),
    );
    let local_axis = resolved.local_axes;
    let raw_aim = ae::Vec2::new(c.aim_x, c.aim_y);
    let local_aim = if raw_aim.length() > 0.1 {
        frame
            .resolve_input(
                snapshot.aim_frame_mode,
                ambition_platformer2d_core::ScreenAxes::new(c.aim_x, c.aim_y),
            )
            .vec()
            .normalize_or_zero()
    } else {
        ae::Vec2::ZERO
    };

    // Movement axis → desired velocity. At the crate::control::ActorControl seam, unqualified
    // direction is controlled-body-local: x = local side/right, y = local
    // down/toward-feet. Downstream movement code should not re-resolve this
    // through the raw input frame.
    out.locomotion = local_axis;
    // Body-generic free-mover steering. A grounded integrator reads the
    // normalized `locomotion` stick (scaled by the body's own run capability);
    // a FLYING body (free-mover, or a hybrid with flight toggled on) steers by
    // absolute `velocity_target` instead. Deriving it here from the snapshot's
    // run capability keeps the player translator fully body-generic: a possessed flyer
    // moves at its own speed with no possession-specific plumbing. The human
    // player passes `max_run_speed == 0` (its integrator ignores this field), so
    // this is inert for the grounded avatar.
    //
    // `velocity_target` is WORLD-SPACE and this wrote a LOCAL vector. Its
    // own doc says "exact world-space velocity command in px/s", and every other
    // writer agrees — `limbs.rs` sends `(home_world - pos) * gain`, and the smash
    // shadow model assigns it straight to `f.vel`. Only this one handed it the
    // body-local stick.
    out.velocity_target = ae::WorldVec2(frame.to_world(local_axis.vec()) * snapshot.max_run_speed);

    // Facing: prefer local side intent; fall back to snapshot facing when stick
    // is neutral so the actor doesn't snap to (0).
    //
    // ⛔⛔ AND ONLY WHILE THE BODY MAY ACTUALLY TURN, which is what made the
    // BACK AIR dead content for the whole cast (D252, found 2026-08-27 by
    // `moveset_takes` on the first run that recorded the resolved gesture).
    // This wrote a facing every tick, grounded or not; it reaches the movement
    // kernel as `MotionStepContext::facing_intent` and is applied there
    // unconditionally, so an airborne fighter TURNED TO FACE the back input
    // before the press was read. `attack_dir_from_axis` then folds the
    // reversal away — `forward = axis.x * facing` — and every back-air press
    // resolved as `Forward`. Fourteen fighters authored an `attack_air_back`
    // none of them could throw.
    //
    // ⭐ THE RULE WAS ALREADY WRITTEN DOWN ONE LAYER DOWN and this contradicted
    // it: `movement/abilities.rs` gates its own steering on
    // `ground.on_ground || flight.fly_enabled` — an airborne body may not turn
    // from the stick. Two authorities disagreed and the aerial one won because
    // it wrote to a field applied later. Stated here at the PRODUCER, where the
    // field's own doc already says what to do: `0.0`/unchanged means *leave the
    // actor's existing facing alone*.
    //
    // ⛔ THE FLYER IS NOT AIRBORNE IN THIS SENSE. `actor_aerial` is a
    // gravity-free FREE-MOVER (a possessed flyer, a `Floating` NPC), and it
    // steers in 2D by `velocity_target` — it has always turned freely and must
    // keep doing so, which is the same carve-out `fly_enabled` makes below.
    //
    // ⚠ THIS IS THE HUMAN ROAD ONLY. A brain that wants to face a particular
    // way still says so; the reverse aerial rush stays a GROUNDED pivot that
    // the jump resolves, and `resolve_attack_gestures` still owns folding a
    // turnaround into an aim.
    let may_turn = snapshot.actor_on_ground || snapshot.actor_aerial;
    out.facing = if may_turn && local_axis.x.abs() > 0.01 {
        local_axis.x.signum()
    } else {
        snapshot.actor_facing
    };

    // Combat verbs.
    out.melee_pressed = c.attack_pressed;
    out.melee_held = c.attack_held;
    out.melee_released = c.attack_released;
    out.melee_strength_hint = c.attack_strength_hint;
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

    out.body_contact_damage_enabled = false;

    // Burst, interact, shield, grab, special.
    out.burst_pressed = c.burst_pressed;
    out.interact_pressed = c.interact_pressed;
    out.shield_held = c.shield_held;
    // Everything else existed: the pad bound Y to Grab, the seat's `ControlFrame.grab_pressed`
    // was set, the body had `AbilitySet:grab` and a `ControlSlot:Grab`, George authored the
    // move — and this brain, the ONE seam a human's frame crosses to reach a body, never copied
    // the field.
    //
    // `crate::control::ActorControl::grab_pressed`'s own doc asserted the opposite: *"the
    // human's Grab button and a CPU's decision write this SAME field. There is
    // deliberately no `cpu_wants_grab` beside it."* The design was right and the
    // carry list simply never learned it, so the comment described an intent the
    // code did not implement — and a CPU could grab while a person could not.
    out.grab_pressed = c.grab_pressed;
    // Same carry, same reason: a verb absent here is CPU-ONLY.
    out.taunt_pressed = c.taunt_pressed;
    // No dedicated "special" input today — `blink_pressed` is the
    // Special now has its OWN dedicated input slot (`Platformer2dInputActionMonolith::Special` →
    // `ControlFrame.special_pressed`), retiring the old
    // `special_pressed = blink_pressed` alias. Blink and Special are separate
    // actions: pressing blink no longer fires a body's signature special.
    out.special_pressed = c.special_pressed;
    // The SUSTAIN beside the edge, and the charge shot is why: a held
    // neutral-B that arrived here as an edge only would release on the tick
    // after it started, every time.
    out.special_held = c.special_held;

    // Player-specific verbs (pogo, blink, fly_toggle, fast_fall,
    // projectile charge, aim). Promoted onto the frame so the
    // sandbox's player simulation can read `crate::control::ActorControl` only and
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
    // The modifier slot crosses the seam UNINTERPRETED — level and edge both. The
    // brain does not decide what sustaining it means; a body's own rules do.
    out.modifier_held = c.modifier_held;
    out.modifier_pressed = c.modifier_pressed;
    // Blink steers with the LOCOMOTION stick, but the two forms use different
    // frame policies, resolved here (the seam) into WORLD space so the movement
    // engine stays frame-agnostic. Quick blink follows the movement mode (already
    // baked into `local_axis`); precision blink follows the aim mode (screen-
    // directed by default), so a precision blink points where the stick points on
    // screen under any gravity.
    out.blink_quick_dir = ae::WorldVec2(frame.to_world(local_axis.vec()));
    out.blink_aim_step = ae::WorldVec2(
        frame.to_world(
            frame
                .resolve_input(
                    snapshot.aim_frame_mode,
                    ambition_platformer2d_core::ScreenAxes::new(c.axis_x, c.axis_y),
                )
                .vec(),
        ),
    );
    out.aim = ae::LocalAxes::from_vec(local_aim);
}

#[cfg(test)]
mod tests;
