//! Player brain — translates `PlayerInputFrame` into the abstract
//! intent fields of [`ae::ActorControlFrame`].
//!
//! The player brain is **purely a translation layer**. It does not
//! make any gameplay decisions — every decision (variable-height
//! jump, dash window, projectile charge) lives in the integration
//! stage, the same way an enemy brain decides "I want to fire" but
//! the projectile-spawner system handles the cooldown gating.
//!
//! Chunk 2 (this file's introduction) keeps the function pure and
//! standalone — no actor uses it yet. Chunk 4 wires it into the
//! per-tick player pipeline.
//!
//! Inputs the player brain *doesn't* touch (and the player tick
//! still reads off `PlayerInputFrame` directly for now):
//!   - `blink_pressed/held/released` — blink is its own subsystem
//!   - `pogo_pressed` — pogo target detection is integration-side
//!   - `projectile_pressed/held/released` — fireball charge is its
//!     own state machine; once the brain owns it, `fire` carries the
//!     resolved direction.
//!   - `fast_fall_pressed`, `fly_toggle_pressed` — niche; promote
//!     these onto the frame only when a non-player brain wants them.

use ambition_engine as ae;

use crate::input::ControlFrame;
use crate::player::components::{PlayerInputFrame, PlayerSlot};

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
    out: &mut ae::ActorControlFrame,
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
    *out = ae::ActorControlFrame::neutral();
    out.facing = snapshot.actor_facing;
}

/// Translate the player's per-tick input into the abstract frame.
///
/// Convenience wrapper that unwraps `PlayerInputFrame` and calls
/// [`tick_player_brain_from_control`]. Used by tests that already
/// have a `PlayerInputFrame` in scope and by sandbox wiring that
/// hasn't migrated to the snapshot-aware `tick_player_brain` yet.
#[allow(
    dead_code,
    reason = "test-only convenience + daytime wiring entry-point"
)]
pub fn tick_player_brain_from_input(
    input: &PlayerInputFrame,
    snapshot: &BrainSnapshot,
    out: &mut ae::ActorControlFrame,
) {
    tick_player_brain_from_control(&input.frame, snapshot, out);
}

/// Translate a raw [`ControlFrame`] into the abstract intent fields
/// of an `ActorControlFrame`. This is the core of the player brain
/// — the wrappers above add convenient input-shape adapters but
/// every translation goes through this function.
pub fn tick_player_brain_from_control(
    c: &ControlFrame,
    snapshot: &BrainSnapshot,
    out: &mut ae::ActorControlFrame,
) {
    *out = ae::ActorControlFrame::neutral();

    // Movement axis → desired velocity (player speed is fixed by the
    // integration; brain just signals direction × magnitude).
    out.desired_vel = ae::Vec2::new(c.axis_x, c.axis_y);

    // Facing: prefer the input axis; fall back to snapshot facing
    // when stick is neutral so the player doesn't snap to (0).
    out.facing = if c.axis_x.abs() > 0.01 {
        c.axis_x.signum()
    } else {
        snapshot.actor_facing
    };

    // Combat verbs.
    out.melee_pressed = c.attack_pressed;
    // Per-tilt direction for the attack — read off the input axes
    // when one is held; defaults to ZERO ("use facing") otherwise.
    out.attack_axis = ae::Vec2::new(c.axis_x, c.axis_y);

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
        out.fire = Some(ae::ActorFireRequest { dir, speed: 0.0 });
    }

    // Jump edges + sustain.
    out.jump_pressed = c.jump_pressed;
    out.jump_held = c.jump_held;
    out.jump_released = c.jump_released;

    // Drop-through: the existing ControlFrame doesn't have a
    // dedicated drop-through; the sandbox computes it from down+jump
    // on the integration side. The player brain leaves it false so
    // the existing logic still owns the decision.
    out.drop_through = false;

    // Dash, interact, shield, special.
    out.dash_pressed = c.dash_pressed;
    out.interact_pressed = c.interact_pressed;
    out.shield_held = c.shield_held;
    // No dedicated "special" input today — `blink_pressed` is the
    // closest analog (blink is the player's signature ability).
    // Promote that to `special_pressed` so a `Brain::Player` driving
    // a different actor's ActionSet can resolve special there.
    out.special_pressed = c.blink_pressed;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::ControlFrame;
    use crate::player::components::PlayerInputFrame;

    fn input_with<F: FnOnce(&mut ControlFrame)>(mut_fn: F) -> PlayerInputFrame {
        let mut c = ControlFrame::default();
        mut_fn(&mut c);
        PlayerInputFrame { frame: c }
    }

    #[test]
    fn neutral_input_yields_neutral_frame() {
        let input = PlayerInputFrame::default();
        let mut s = BrainSnapshot::idle();
        s.actor_facing = 1.0;
        let mut out = ae::ActorControlFrame::default();
        out.melee_pressed = true; // pre-poisoned
        tick_player_brain_from_input(&input, &s, &mut out);
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
        let mut out = ae::ActorControlFrame::default();
        out.melee_pressed = true; // pre-poisoned
        out.fire = Some(ae::ActorFireRequest {
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
        let mut out = ae::ActorControlFrame::default();
        tick_player_brain_from_input(&input, &s, &mut out);
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
        let mut out = ae::ActorControlFrame::default();
        tick_player_brain_from_input(&input, &s, &mut out);
        assert_eq!(out.desired_vel, ae::Vec2::new(-1.0, 0.3));
        assert_eq!(out.facing, -1.0);
    }

    #[test]
    fn jump_edges_pass_through() {
        let input = input_with(|c| {
            c.jump_pressed = true;
            c.jump_held = true;
            c.jump_released = false;
        });
        let s = BrainSnapshot::idle();
        let mut out = ae::ActorControlFrame::default();
        tick_player_brain_from_input(&input, &s, &mut out);
        assert!(out.jump_pressed);
        assert!(out.jump_held);
        assert!(!out.jump_released);
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
        let mut out = ae::ActorControlFrame::default();
        tick_player_brain_from_input(&input, &s, &mut out);
        let fire = out.fire.expect("fire request expected");
        // Aim wins over facing.
        assert!((fire.dir.y - (-1.0)).abs() < 0.001);
        // No aim → fire uses facing.
        let input2 = input_with(|c| {
            c.projectile_released = true;
        });
        let mut out2 = ae::ActorControlFrame::default();
        tick_player_brain_from_input(&input2, &s, &mut out2);
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
        let mut out = ae::ActorControlFrame::default();
        tick_player_brain_from_input(&input, &s, &mut out);
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
        let mut out = ae::ActorControlFrame::default();
        out.melee_pressed = true; // pre-poisoned
        tick_player_brain(PlayerSlot(0), &s, &mut out);
        assert!(!out.melee_pressed);
        assert_eq!(out.facing, -1.0);
    }
}
