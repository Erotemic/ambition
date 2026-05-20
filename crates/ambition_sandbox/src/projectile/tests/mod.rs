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

use ambition_engine as ae;
use ambition_engine::{Block, World};
use bevy::prelude::*;

use super::state::PlayerProjectileState;
use super::systems::update_projectiles;
use crate::audio::SfxMessage;
use crate::features::{
    ActorHealth, ActorIdentity, DamageEvent, GameplayBanner, GameplayEffect, PogoBounceEvent,
};
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
    let size = ae::Vec2::new(20.0, 30.0);
    let body = crate::player::PlayerBody {
        pos,
        vel: ae::Vec2::ZERO,
        size,
        base_size: size,
        facing,
        on_ground: true,
        fly_enabled: false,
        dash_charges_available: 0,
        air_jumps_available: 0,
        mana_current: 0.0,
        body_mode: ae::BodyMode::Standing,
        invincible: false,
        dodge_rolling: false,
        shielding: false,
        parrying: false,
    };
    // Projectile spawn queries `PrimaryPlayerOnly` + `PlayerInputFrame`;
    // the test fixture must mark the spawned player as primary AND
    // local AND carry `PlayerInputFrame` or the system will see an
    // empty query and silently skip fire-press handling.
    // `sync_local_player_input_frame` (added to the test schedule)
    // mirrors `Res<ControlFrame>` into the component each tick so
    // existing tests that mutate the resource continue to drive the
    // projectile system unchanged.
    app.world_mut().spawn((
        crate::player::PlayerEntity,
        crate::player::PrimaryPlayer,
        crate::player::LocalPlayer,
        crate::player::PlayerInputFrame::default(),
        body,
    ));
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
    app.insert_resource(PlayerProjectileState::default());
    // Buffered-message channels the system writes into.
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<GameplayEffect>();
    app.add_message::<DamageEvent>();
    app.add_message::<PogoBounceEvent>();
    app.add_systems(
        Update,
        (
            crate::player::sync_local_player_input_frame,
            update_projectiles,
            crate::features::apply_feature_damage_events,
        )
            .chain(),
    );
    spawn_player(&mut app, ae::Vec2::new(300.0, 300.0), 1.0);
    app
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
