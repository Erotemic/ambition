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
use ambition_input::ControlFrame;

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
    let resolved = ae::AccelerationFrame::new(snapshot.control_down).resolve_control(
        snapshot.input_frame_mode,
        c.axis_x,
        c.axis_y,
    );
    let local_axis = resolved.local_axis;

    // Movement axis → desired velocity. At the ActorControl seam, unqualified
    // direction is controlled-body-local: x = local side/right, y = local
    // down/toward-feet. Downstream movement code should not re-resolve this
    // through the raw input frame.
    out.desired_vel = local_axis;

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
        // Direction: aim stick if present, else facing horizontal.
        let aim = ae::Vec2::new(c.aim_x, c.aim_y);
        let dir = if aim.length() > 0.1 {
            aim.normalize_or_zero()
        } else {
            ae::Vec2::new(snapshot.actor_facing, 0.0)
        };
        out.fire = Some(crate::actor::control::ActorFireRequest { dir, speed: 0.0 });
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
    out.aim = ae::Vec2::new(c.aim_x, c.aim_y);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_input::ControlFrame;

    fn input_with<F: FnOnce(&mut ControlFrame)>(mut_fn: F) -> ControlFrame {
        let mut c = ControlFrame::default();
        mut_fn(&mut c);
        c
    }

    #[test]
    fn neutral_input_yields_neutral_frame() {
        let input = ControlFrame::default();
        let mut s = BrainSnapshot::idle();
        s.actor_facing = 1.0;
        let mut out = crate::actor::control::ActorControlFrame::default();
        out.melee_pressed = true; // pre-poisoned
        tick_player_brain_from_control(&input, &s, &mut out);
        assert!(!out.melee_pressed);
        assert_eq!(out.desired_vel, ae::Vec2::ZERO);
        // Facing falls back to snapshot when stick neutral.
        assert_eq!(out.facing, 1.0);
    }

    #[test]
    fn tick_player_brain_without_snapshot_input_falls_back_to_neutral() {
        // When snapshot.player_input is None, tick_player_brain
        // emits a neutral frame + facing only (no garbage from
        // uninitialized fields). Pins the safe-default path so
        // an actor with Brain::Player but no input snapshot
        // doesn't fire random actions.
        let mut s = BrainSnapshot::idle();
        s.actor_facing = -1.0;
        s.player_input = None;
        let mut out = crate::actor::control::ActorControlFrame::default();
        out.melee_pressed = true; // pre-poisoned
        out.fire = Some(crate::actor::control::ActorFireRequest {
            dir: ae::Vec2::new(1.0, 0.0),
            speed: 200.0,
        });
        tick_player_brain(PlayerSlot(0), &s, &mut out);
        assert!(!out.melee_pressed);
        assert!(out.fire.is_none());
        assert_eq!(out.facing, -1.0);
    }

    #[test]
    fn attack_pressed_routes_to_melee_intent() {
        let input = input_with(|c| {
            c.attack_pressed = true;
            c.axis_x = 0.0;
            c.axis_y = -1.0;
        });
        let mut s = BrainSnapshot::idle();
        s.actor_facing = 1.0;
        let mut out = crate::actor::control::ActorControlFrame::default();
        tick_player_brain_from_control(&input, &s, &mut out);
        assert!(out.melee_pressed);
        // Up-tilt: attack_axis carries the input direction.
        assert_eq!(out.attack_axis, ae::Vec2::new(0.0, -1.0));
    }

    #[test]
    fn movement_axis_routes_to_desired_vel_and_facing() {
        let input = input_with(|c| {
            c.axis_x = -1.0;
            c.axis_y = 0.3;
        });
        let s = BrainSnapshot::idle();
        let mut out = crate::actor::control::ActorControlFrame::default();
        tick_player_brain_from_control(&input, &s, &mut out);
        assert_eq!(out.desired_vel, ae::Vec2::new(-1.0, 0.3));
        assert_eq!(out.facing, -1.0);
    }

    #[test]
    fn screen_directed_sideways_gravity_maps_directional_verbs_to_local_frame() {
        let mut s = BrainSnapshot::idle();
        // Gravity / acceleration points screen-right, so the controlled body's
        // local-right direction is screen-up. In screen-directed mode, raw
        // screen-down should therefore mean local-left.
        s.control_down = ae::Vec2::new(1.0, 0.0);
        s.input_frame_mode = ae::InputFrameMode::Screen;
        s.actor_facing = 1.0;
        let input = input_with(|c| {
            c.axis_x = 0.0;
            c.axis_y = 1.0;
            c.attack_pressed = true;
        });
        let mut out = crate::actor::control::ActorControlFrame::default();
        tick_player_brain_from_control(&input, &s, &mut out);
        assert_eq!(out.desired_vel, ae::Vec2::new(-1.0, 0.0));
        assert_eq!(out.facing, -1.0);
        assert_eq!(out.attack_axis, ae::Vec2::new(-1.0, 0.0));

        // Raw screen-right maps to local-down under the same frame/mode, which
        // is the crouch / down-attack / morph-ball direction.
        let input = input_with(|c| {
            c.axis_x = 1.0;
            c.axis_y = 0.0;
            c.attack_pressed = true;
        });
        tick_player_brain_from_control(&input, &s, &mut out);
        assert_eq!(out.desired_vel, ae::Vec2::new(0.0, 1.0));
        assert_eq!(out.attack_axis, ae::Vec2::new(0.0, 1.0));
    }

    #[test]
    fn jump_edges_pass_through() {
        let input = input_with(|c| {
            c.jump_pressed = true;
            c.jump_held = true;
            c.jump_released = false;
        });
        let s = BrainSnapshot::idle();
        let mut out = crate::actor::control::ActorControlFrame::default();
        tick_player_brain_from_control(&input, &s, &mut out);
        assert!(out.jump_pressed);
        assert!(out.jump_held);
        assert!(!out.jump_released);
    }

    #[test]
    fn player_brain_keeps_body_contact_damage_disabled() {
        let input = input_with(|c| {
            c.axis_x = 1.0;
            c.jump_pressed = true;
            c.attack_pressed = true;
        });
        let s = BrainSnapshot::idle();
        let mut out = crate::actor::control::ActorControlFrame::default();
        out.body_contact_damage_enabled = true; // pre-poisoned
        tick_player_brain_from_control(&input, &s, &mut out);
        assert!(!out.body_contact_damage_enabled);
    }

    #[test]
    fn projectile_released_emits_fire_with_aim_or_facing() {
        // Aim stick → fire uses aim.
        let input = input_with(|c| {
            c.projectile_released = true;
            c.aim_x = 0.0;
            c.aim_y = -1.0;
        });
        let mut s = BrainSnapshot::idle();
        s.actor_facing = -1.0;
        let mut out = crate::actor::control::ActorControlFrame::default();
        tick_player_brain_from_control(&input, &s, &mut out);
        let fire = out.fire.expect("fire request expected");
        // Aim wins over facing.
        assert!((fire.dir.y - (-1.0)).abs() < 0.001);
        // No aim → fire uses facing.
        let input2 = input_with(|c| {
            c.projectile_released = true;
        });
        let mut out2 = crate::actor::control::ActorControlFrame::default();
        tick_player_brain_from_control(&input2, &s, &mut out2);
        let fire2 = out2.fire.expect("fire request expected");
        assert!((fire2.dir.x - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn shield_and_special_pass_through() {
        let input = input_with(|c| {
            c.shield_held = true;
            c.blink_pressed = true;
            c.dash_pressed = true;
            c.interact_pressed = true;
        });
        let s = BrainSnapshot::idle();
        let mut out = crate::actor::control::ActorControlFrame::default();
        tick_player_brain_from_control(&input, &s, &mut out);
        assert!(out.shield_held);
        assert!(out.special_pressed);
        assert!(out.dash_pressed);
        assert!(out.interact_pressed);
    }

    #[test]
    fn tick_player_brain_no_input_emits_neutral_with_facing() {
        // The signature-matching variant doesn't have input access
        // yet (Chunk 4 plumbing); it emits neutral + facing from the
        // snapshot. Keep this test honest to that contract.
        let mut s = BrainSnapshot::idle();
        s.actor_facing = -1.0;
        let mut out = crate::actor::control::ActorControlFrame::default();
        out.melee_pressed = true; // pre-poisoned
        tick_player_brain(PlayerSlot(0), &s, &mut out);
        assert!(!out.melee_pressed);
        assert_eq!(out.facing, -1.0);
    }
}
