//! Swim mechanic: post-`sandbox_update` buoyancy + swim-controls layer.
//!
//! Reads `FeatureRuntime::water_volumes` (built from
//! `RoomObjectKind::WaterVolume`) and adjusts the player's velocity
//! and gravity contribution while they're submerged. Always slows
//! the player down (so an un-upgraded player splashes through water
//! sluggishly); the active swim impulse only fires when the
//! `swim` ability flag is on.
//!
//! Like ledge grab, this is intentionally a separate sandbox system
//! layered on top of `movement.rs` rather than weaving the new
//! mechanic into the dense simulator.

use ambition_engine::AabbExt;
use bevy::prelude::*;

pub fn update_swim(
    mut runtime: ResMut<crate::SandboxRuntime>,
    controls: Res<crate::input::ControlFrame>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let player_aabb = runtime.player.aabb();

    let Some(volume) = runtime
        .features
        .water_volumes
        .iter()
        .find(|v| v.aabb.strict_intersects(player_aabb))
        .cloned()
    else {
        return;
    };

    // Buoyancy drag: linear damping per tick. Always applies.
    let drag = volume.spec.drag.clamp(0.0, 1.0);
    runtime.player.vel.x *= 1.0 - drag;
    runtime.player.vel.y *= 1.0 - drag;
    // Cap fall speed.
    if runtime.player.vel.y > volume.spec.max_fall_speed {
        runtime.player.vel.y = volume.spec.max_fall_speed;
    }
    // Active swim impulse — gated on the ability flag.
    if runtime.player.abilities.swim && controls.axis_y < -0.4 {
        runtime.player.vel.y =
            runtime.player.vel.y.min(0.0) - volume.spec.swim_up_impulse * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::WaterVolumeRuntime;
    use crate::input::ControlFrame;
    use crate::{GameWorld, SandboxRuntime};
    use ambition_engine as ae;

    fn empty_world() -> ae::World {
        ae::World::new(
            "swim_test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 1000.0),
            Vec::new(),
        )
    }

    /// Build a minimal Bevy app with the swim system installed and a
    /// player parked inside a single water volume. `swim_ability`
    /// toggles the ability flag; `axis_y` controls the up/down stick.
    fn swim_app(swim_ability: bool, axis_y: f32) -> App {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(GameWorld(empty_world()));
        let world = empty_world();
        let mut abilities = ae::AbilitySet::sandbox_all();
        abilities.swim = swim_ability;
        let mut runtime = SandboxRuntime::new(
            &world,
            abilities,
            ae::DEFAULT_TUNING,
            crate::physics::PhysicsSandboxSettings::default(),
        );
        // Park the player at a known position with a known falling
        // velocity so drag/cap behavior is observable.
        runtime.player.pos = ae::Vec2::new(500.0, 500.0);
        runtime.player.vel = ae::Vec2::new(40.0, 600.0);
        // Inject a water volume that fully contains the player AABB.
        let player_aabb = runtime.player.aabb();
        let center = player_aabb.center();
        let big = ae::Vec2::new(400.0, 400.0);
        runtime.features.water_volumes.push(WaterVolumeRuntime {
            id: "water".into(),
            aabb: ae::Aabb::new(center, big),
            spec: ae::WaterVolumeSpec::default(),
        });
        app.insert_resource(runtime);
        app.insert_resource(ControlFrame {
            axis_y,
            ..ControlFrame::default()
        });
        app.add_systems(Update, update_swim);
        app
    }

    fn advance_time(app: &mut App, dt: f32) {
        let mut time = app.world_mut().resource_mut::<Time<()>>();
        time.advance_by(std::time::Duration::from_secs_f32(dt));
    }

    /// Ability off + neutral stick: passive drag must still slow the
    /// player and the fall-speed cap must clamp vertical velocity. The
    /// active swim impulse must NOT fire.
    #[test]
    fn swim_off_applies_passive_drag_and_fall_cap_only() {
        let mut app = swim_app(false, 0.0);
        advance_time(&mut app, 0.016);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        let spec = ae::WaterVolumeSpec::default();
        // Drag reduces |vx| toward zero (was 40.0).
        assert!(runtime.player.vel.x.abs() < 40.0);
        // Vertical speed is clamped to max_fall_speed because the
        // pre-clamp value (600 * (1-drag)) is well above the cap.
        assert!(
            (runtime.player.vel.y - spec.max_fall_speed).abs() < 1e-3,
            "expected vel.y == max_fall_speed ({}); got {}",
            spec.max_fall_speed,
            runtime.player.vel.y
        );
    }

    /// Ability off + holding Up: the active swim impulse is gated on
    /// the ability, so vertical velocity must equal the fall-speed cap
    /// (passive path) — never the post-impulse value.
    #[test]
    fn swim_off_does_not_apply_upward_impulse_even_with_up_held() {
        let mut app = swim_app(false, -1.0);
        advance_time(&mut app, 0.016);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        let spec = ae::WaterVolumeSpec::default();
        assert!(
            (runtime.player.vel.y - spec.max_fall_speed).abs() < 1e-3,
            "ability-off must not apply swim impulse; got vel.y={}",
            runtime.player.vel.y
        );
    }

    /// Ability on + holding Up: the active impulse must drive vertical
    /// velocity strictly upward (negative in screen coords).
    #[test]
    fn swim_on_applies_upward_impulse_when_up_held() {
        let mut app = swim_app(true, -1.0);
        advance_time(&mut app, 0.016);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        // Active impulse path: vel.y = vel.y.min(0.0) - swim_up_impulse * dt
        // The 600.0 falling vel is dropped to 0 by .min(0.0), then the
        // impulse subtracts. Result must be strongly negative (upward).
        assert!(
            runtime.player.vel.y < 0.0,
            "expected upward vel.y after impulse; got {}",
            runtime.player.vel.y
        );
    }

    /// Outside any water volume, the swim system is a complete no-op:
    /// vel and pos are unchanged.
    #[test]
    fn swim_no_op_when_player_outside_water_volume() {
        let mut app = swim_app(true, -1.0);
        // Drop the water volume so the system can't find any overlap.
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.features.water_volumes.clear();
        }
        let before_vel;
        let before_pos;
        {
            let runtime = app.world().resource::<SandboxRuntime>();
            before_vel = runtime.player.vel;
            before_pos = runtime.player.pos;
        }
        advance_time(&mut app, 0.016);
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert_eq!(runtime.player.vel, before_vel);
        assert_eq!(runtime.player.pos, before_pos);
    }
}
