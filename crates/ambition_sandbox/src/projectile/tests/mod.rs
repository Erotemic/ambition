//! Projectile system tests, split by topic.
//!
//! - [`charging`] — input / charge / motion-buffer (QCF) recognition,
//!   cooldown gating, resource exhaustion.
//! - [`collision`] — hit detection against ECS actors, floor / one-way
//!   platform / wall bounce + expire behavior.
//!
//! Shared fixtures (`dummy_world`, `spawn_player`, `min_app`,
//! `advance_time`, `tap_projectile`) live here so each submodule can
//! reach them via `super::`.

use crate::engine_core as ae;
use crate::engine_core::{Block, World};
use bevy::prelude::*;

use super::state::PlayerProjectileState;
use super::systems::update_projectiles;
use crate::audio::SfxMessage;
use crate::features::{ActorHealth, ActorIdentity, GameplayBanner, GameplayEffect, HitEvent};
use crate::input::ControlFrame;
use crate::presentation::fx::VfxMessage;
use crate::trace::GameplayTraceBuffer;
use crate::world::physics::DebrisBurstMessage;
use crate::GameWorld;

mod charging;
mod collision;

fn dummy_world() -> World {
    World::new(
        "test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![Block::solid(
            "right wall",
            ae::Vec2::new(800.0, 100.0),
            ae::Vec2::new(40.0, 400.0),
        )],
    )
}

fn spawn_player(app: &mut App, pos: ae::Vec2, facing: f32) {
    // Spawn via `PlayerSimulationBundle` so the entity carries every
    // component the projectile system + visuals path queries
    // (`PlayerKinematics`, `PlayerEntity`, `PrimaryPlayer`,
    // `LocalPlayer`, `PlayerInputFrame`, the 17 other cluster
    // components, …) with no manual spawn-tuple list.
    let mut scratch = crate::player::primary_player_scratch(pos, ae::AbilitySet::sandbox_all());
    scratch.kinematics.facing = facing;
    scratch.ground.on_ground = true;
    let bundle =
        crate::player::PlayerSimulationBundle::from_scratch(scratch, crate::actor::Health::new(10));
    app.world_mut().spawn(bundle);
}

fn min_app() -> App {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.insert_resource(crate::WorldTime::default());
    app.insert_resource(GameWorld(dummy_world()));
    app.insert_resource(ControlFrame::default());
    app.insert_resource(crate::persistence::settings::UserSettings::default());
    app.insert_resource(GameplayTraceBuffer::default());
    app.insert_resource(GameplayBanner::default());
    // PlayerProjectileState is now a per-player Component attached
    // by `PlayerSimulationBundle::from_scratch` — no resource init
    // needed.
    // Buffered-message channels the system writes into. The brain
    // plugin owns the `ActorActionMessage` channel; install it so
    // `tick_player_brains` → `emit_player_projectile_tick_messages` →
    // `update_projectiles` form the same chain that production uses
    // for the player projectile path.
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<GameplayEffect>();
    app.add_message::<HitEvent>();
    app.add_plugins(crate::brain::BrainPlugin);
    app.add_systems(
        Update,
        (
            crate::player::sync_local_player_input_frame,
            crate::player::tick_player_brains,
            crate::brain::emit_player_projectile_tick_messages,
            update_projectiles,
            crate::features::apply_feature_hit_events,
        )
            .chain(),
    );
    spawn_player(&mut app, ae::Vec2::new(300.0, 300.0), 1.0);
    app
}

/// Read-only view of the primary player's `PlayerProjectileState`.
/// Tests previously read it as a `Res<PlayerProjectileState>`; the
/// per-player migration moved it onto the player entity, so this
/// helper hides the resource-vs-component difference at the test
/// boundary.
pub(in crate::projectile) fn projectile_state_ref(app: &App) -> &PlayerProjectileState {
    let world = app.world();
    let mut q = world.try_query::<&PlayerProjectileState>().unwrap();
    q.iter(world)
        .next()
        .expect("min_app spawned exactly one player with PlayerProjectileState")
}

/// Mutable handle to the primary player's `PlayerProjectileState`.
pub(in crate::projectile) fn projectile_state_mut(
    app: &mut App,
) -> bevy::prelude::Mut<'_, PlayerProjectileState> {
    let world = app.world_mut();
    let entity = {
        let mut q = world
            .try_query::<(bevy::prelude::Entity, &PlayerProjectileState)>()
            .unwrap();
        q.iter(world)
            .next()
            .expect("min_app spawned exactly one player with PlayerProjectileState")
            .0
    };
    world
        .get_mut::<PlayerProjectileState>(entity)
        .expect("entity has PlayerProjectileState")
}

fn advance_time(app: &mut App, dt_seconds: f32) {
    let mut time = app.world_mut().resource_mut::<Time<()>>();
    time.advance_by(std::time::Duration::from_secs_f32(dt_seconds));
    // `update_projectiles` reads `Res<WorldTime>`, not `Res<Time>`,
    // so the test harness must mirror the production pipeline's
    // `refresh_world_time` step. Tests run at `time_scale = 1.0`,
    // so `sim_dt == wall_dt`.
    let mut world_time = app.world_mut().resource_mut::<crate::WorldTime>();
    world_time.raw_dt = dt_seconds;
    world_time.scaled_dt = dt_seconds;
}

/// Helper: press the projectile button (no motion) and immediately
/// release it on the same press-release pair. Equivalent to a
/// "tap" in the new charge model — fires a tier-0 fireball.
fn tap_projectile(app: &mut App) {
    // Press frame: just_pressed=true, held=true (Bevy's button
    // semantics — pressed state lasts as long as held), released=false.
    // The system enters the press branch and starts charging.
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = true;
        frame.projectile_held = true;
        frame.projectile_released = false;
    }
    advance_time(app, 0.016);
    app.update();
    // Release frame: just_pressed=false, held=false, released=true.
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = false;
        frame.projectile_held = false;
        frame.projectile_released = true;
    }
    advance_time(app, 0.016);
    app.update();
    // Reset the edge for the next test step.
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_released = false;
    }
}
