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
//! - Mid-action mechanics (dash, blink-aim, wall-cling/climb, swim)
//!   own the player shape; the driver no-ops while any of them are
//!   active.
//!
//! Runs in the progression chain after `sandbox_update` for the same
//! reason `ledge_grab` and `swim` do: it mutates `runtime.player`
//! outside the dense `movement.rs` simulator. The size/pos delta is
//! constrained to the body-mode swap (no horizontal repositioning),
//! so the next simulator tick treats it as a clean smaller AABB and
//! collision repair runs as usual against any new geometry. The
//! engine still gates `fast_fall_pressed` on `!on_ground`, so using
//! the same gesture for grounded morph and airborne fast-fall has
//! no input crosstalk.

use ambition_engine as ae;
use bevy::prelude::*;

/// Threshold on `axis_y` for treating Down as "held" for crouch.
/// Mirrors the threshold used by ledge-grab drop and the engine's
/// drop-through gesture so the player feel stays consistent.
const CROUCH_AXIS_Y_THRESHOLD: f32 = 0.4;

pub fn update_body_mode(
    world: Res<crate::GameWorld>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    controls: Res<crate::input::ControlFrame>,
) {
    let player = &runtime.player;

    // Mid-action mechanics own the body shape — don't fight them.
    if player.dash_timer > 0.0 || player.blink_aiming {
        return;
    }
    // Wall / ledge state owns its own posture; reverting it via crouch
    // would break the ledge-grab anchor invariant.
    if player.wall_clinging || player.wall_climbing {
        return;
    }
    // In-water posture: leave water swim mechanics alone.
    if player.water_contact.is_some() {
        return;
    }

    let down_held = controls.axis_y > CROUCH_AXIS_Y_THRESHOLD;
    let on_ground = player.on_ground;
    let mode = player.body_mode;
    let solid = |b: &ae::Block| matches!(b.kind, ae::BlockKind::Solid);

    // MorphBall has the smallest AABB. Exiting it means re-checking
    // overhead clearance; sourcing the exit input from `jump_pressed`
    // mirrors how a player would naturally try to "stand up" out of
    // the ball.
    if mode == ae::BodyMode::MorphBall {
        if controls.jump_pressed {
            let _ = ae::try_change_body_mode(
                &mut runtime.player,
                ae::BodyMode::Standing,
                &world.0,
                solid,
            );
        }
        return;
    }

    // Double-tap-down on the ground from Standing or Crouching curls
    // into MorphBall. The engine gates fast_fall on `!on_ground` so
    // this gesture does not race with airborne fast-fall.
    if on_ground && controls.fast_fall_pressed {
        let _ = ae::try_change_body_mode(
            &mut runtime.player,
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

    // The engine helper does the resize-with-fit check; ignore the
    // boolean result — a blocked stand-up is the desired UX (player
    // stays crouched under the ceiling) and the auto-trace diff will
    // surface a successful transition.
    let _ = ae::try_change_body_mode(&mut runtime.player, target, &world.0, solid);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::ControlFrame;
    use crate::{GameWorld, SandboxRuntime};
    use ambition_engine as ae;

    fn empty_world() -> ae::World {
        ae::World::new(
            "body_mode_test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 1000.0),
            Vec::new(),
        )
    }

    fn ceiling_world(ceiling_top: f32, ceiling_h: f32) -> ae::World {
        ae::World::new(
            "body_mode_ceiling",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 1000.0),
            vec![ae::Block::solid(
                "ceiling",
                ae::Vec2::new(0.0, ceiling_top),
                ae::Vec2::new(2000.0, ceiling_h),
            )],
        )
    }

    fn body_app(world: ae::World) -> App {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(GameWorld(world.clone()));
        let runtime = SandboxRuntime::new(
            &world,
            ae::AbilitySet::sandbox_all(),
            ae::DEFAULT_TUNING,
            crate::physics::PhysicsSandboxSettings::default(),
        );
        app.insert_resource(runtime);
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, update_body_mode);
        app
    }

    fn set_grounded_at(app: &mut App, pos: ae::Vec2) {
        let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
        runtime.player.pos = pos;
        runtime.player.vel = ae::Vec2::ZERO;
        runtime.player.on_ground = true;
        runtime.player.on_wall = false;
        runtime.player.wall_clinging = false;
        runtime.player.wall_climbing = false;
        runtime.player.dash_timer = 0.0;
        runtime.player.blink_aiming = false;
        runtime.player.water_contact = None;
    }

    fn set_axis_y(app: &mut App, axis_y: f32) {
        let mut controls = app.world_mut().resource_mut::<ControlFrame>();
        controls.axis_y = axis_y;
    }

    /// Holding Down on the ground transitions Standing → Crouching and
    /// shrinks `player.size.y`.
    #[test]
    fn down_held_grounded_enters_crouch() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        set_axis_y(&mut app, 1.0);

        app.update();

        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Crouching);
        assert!(runtime.player.size.y < runtime.player.base_size.y);
    }

    /// Releasing Down with overhead clearance returns to Standing.
    #[test]
    fn down_released_returns_to_standing_when_clear() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));

        // Crouch first.
        set_axis_y(&mut app, 1.0);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::Crouching
        );

        // Release down.
        set_axis_y(&mut app, 0.0);
        app.update();

        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
        assert_eq!(runtime.player.size, runtime.player.base_size);
    }

    /// A low ceiling above the crouched body must reject the stand-up
    /// transition — the player stays crouched.
    #[test]
    fn stand_up_blocked_under_low_ceiling() {
        // Player at pos.y = 600, base_size.y = 46. AABB convention:
        // pos = center, +Y is downward, so:
        //   * Standing y range = [600 - 23, 600 + 23] = [577, 623].
        //   * Crouching size = 46 * 0.55 = 25.3, dy = (46-25.3)/2 = 10.35,
        //     so pos.y = 610.35 and crouched y range = [597.7, 623].
        //   * Stand-up restores pos.y = 600 and standing y range = [577, 623].
        //
        // Ceiling y range [560, 590]:
        //   * Crouched [597.7, 623] vs [560, 590]: 597.7 > 590 → no overlap.
        //   * Standing [577, 623] vs [560, 590]: 577 < 590 → overlap.
        // Initial standing also overlaps; the helper doesn't reject pre-
        // existing penetration — it only checks the *target* shape, so
        // the crouch transition still succeeds and the stand-up correctly
        // fails.
        let world = ceiling_world(560.0, 30.0);
        let mut app = body_app(world);
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 600.0));

        // Crouch.
        set_axis_y(&mut app, 1.0);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::Crouching
        );

        // Release down — stand-up should be blocked.
        set_axis_y(&mut app, 0.0);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Crouching);
        assert!(runtime.player.size.y < runtime.player.base_size.y);
    }

    /// In the air, holding Down does not crouch (crouch is grounded only).
    #[test]
    fn airborne_down_does_not_crouch() {
        let mut app = body_app(empty_world());
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.player.pos = ae::Vec2::new(200.0, 200.0);
            runtime.player.on_ground = false;
            runtime.player.vel = ae::Vec2::ZERO;
        }
        set_axis_y(&mut app, 1.0);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
    }

    /// Mid-dash holding Down does not crouch — dash owns the body shape.
    #[test]
    fn dash_active_blocks_crouch() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.player.dash_timer = 0.05;
        }
        set_axis_y(&mut app, 1.0);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
    }

    /// Double-tap-down on the ground from Standing curls into MorphBall.
    /// `fast_fall_pressed` is the existing input-layer gesture for
    /// double-tap-down, so the test sets it directly.
    #[test]
    fn double_tap_down_grounded_enters_morph_ball() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        {
            let mut controls = app.world_mut().resource_mut::<ControlFrame>();
            controls.fast_fall_pressed = true;
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::MorphBall);
        // MorphBall is smaller than Standing on both axes.
        assert!(runtime.player.size.x < runtime.player.base_size.x);
        assert!(runtime.player.size.y < runtime.player.base_size.y);
    }

    /// Crouching + double-tap-down also curls into MorphBall (reachable
    /// from either entry point). Mirrors the input model in the
    /// docstring.
    #[test]
    fn double_tap_down_from_crouch_enters_morph_ball() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));

        // Crouch first.
        set_axis_y(&mut app, 1.0);
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::Crouching
        );
        // Then double-tap-down.
        {
            let mut controls = app.world_mut().resource_mut::<ControlFrame>();
            controls.fast_fall_pressed = true;
        }
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::MorphBall
        );
    }

    /// Jump-pressed inside MorphBall unmorphs to Standing when there's
    /// overhead clearance.
    #[test]
    fn jump_press_in_morph_ball_unmorphs_to_standing() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));

        // Drive into MorphBall via the gesture (covers the input path
        // and avoids juggling a second world reference inside a
        // resource borrow).
        {
            let mut controls = app.world_mut().resource_mut::<ControlFrame>();
            controls.fast_fall_pressed = true;
        }
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::MorphBall
        );

        {
            let mut controls = app.world_mut().resource_mut::<ControlFrame>();
            controls.fast_fall_pressed = false;
            controls.jump_pressed = true;
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
        assert_eq!(runtime.player.size, runtime.player.base_size);
    }

    /// Jump-pressed inside MorphBall under a low ceiling stays curled —
    /// the standing AABB doesn't fit.
    #[test]
    fn jump_press_in_morph_ball_under_low_ceiling_stays_curled() {
        // Ceiling at y in [560, 590]: standing top 577 < 590 → blocks.
        // MorphBall body: base_size 28x46 → MorphBall is (28*0.55,
        // 28*0.55) = (15.4, 15.4). On the floor at pos.y = 600, the
        // morph ball center is at 600 + (46 - 15.4)/2 = 615.3, half
        // 7.7 → top 607.6, bottom 623.0. Crouched would be 597.7→623,
        // so the morph ball clears the ceiling at 590 by an even wider
        // margin. Standing has top 577 → blocked.
        let world = ceiling_world(560.0, 30.0);
        let mut app = body_app(world);
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 600.0));

        // Morph via gesture.
        {
            let mut controls = app.world_mut().resource_mut::<ControlFrame>();
            controls.fast_fall_pressed = true;
        }
        app.update();
        assert_eq!(
            app.world().resource::<SandboxRuntime>().player.body_mode,
            ae::BodyMode::MorphBall
        );

        // Try to unmorph.
        {
            let mut controls = app.world_mut().resource_mut::<ControlFrame>();
            controls.fast_fall_pressed = false;
            controls.jump_pressed = true;
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::MorphBall);
    }

    /// Airborne double-tap-down does NOT curl (morph is grounded only).
    #[test]
    fn airborne_double_tap_down_does_not_morph() {
        let mut app = body_app(empty_world());
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.player.pos = ae::Vec2::new(200.0, 200.0);
            runtime.player.on_ground = false;
        }
        {
            let mut controls = app.world_mut().resource_mut::<ControlFrame>();
            controls.fast_fall_pressed = true;
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
    }

    /// Wall-cling state owns the player posture; do not crouch from it.
    #[test]
    fn wall_clinging_blocks_crouch() {
        let mut app = body_app(empty_world());
        set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.player.wall_clinging = true;
            runtime.player.on_wall = true;
            runtime.player.on_ground = false;
        }
        set_axis_y(&mut app, 1.0);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.body_mode, ae::BodyMode::Standing);
    }
}
