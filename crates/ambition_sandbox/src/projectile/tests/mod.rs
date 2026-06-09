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
use crate::features::{ActorHealth, ActorIdentity, GameplayBanner, HitEvent, SetFlagRequested};
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
    // (`BodyKinematics`, `PlayerEntity`, `PrimaryPlayer`,
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
    // needed. In-flight projectiles are ECS entities (Phase 3c-ii);
    // their spawn-id source is this global counter.
    app.init_resource::<crate::projectile::ProjectileSeqCounter>();
    // Buffered-message channels the system writes into. The brain
    // plugin owns the `ActorActionMessage` channel; install it so
    // `tick_player_brains` → `emit_player_projectile_tick_messages` →
    // `update_projectiles` form the same chain that production uses
    // for the player projectile path.
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<HitEvent>();
    app.add_message::<crate::features::ActorStimulus>();
    app.add_message::<crate::projectile::SpawnProjectile>();
    app.add_plugins(crate::brain::BrainPlugin);
    app.add_systems(
        Update,
        (
            crate::player::sync_local_player_input_frame,
            crate::player::tick_player_brains,
            crate::brain::emit_player_projectile_tick_messages,
            update_projectiles,
            // Phase 3b: update_projectiles emits SpawnProjectile; the
            // player-pool consumer pushes the body into the firing player's
            // PlayerProjectileState.bodies (after the step, like production).
            super::systems::apply_player_spawn_projectile_messages,
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

/// The entity id of the (single) primary player spawned by `min_app`.
pub(in crate::projectile) fn primary_player_entity(app: &mut App) -> Entity {
    let world = app.world_mut();
    let mut q = world
        .try_query_filtered::<Entity, With<crate::player::PlayerEntity>>()
        .unwrap();
    q.iter(world)
        .next()
        .expect("min_app spawned exactly one player")
}

/// Collect the in-flight player projectile bodies, sorted by spawn
/// sequence (oldest first) — the same order the old `state.bodies` Vec
/// presented. Recomposes a [`crate::projectile::ProjectileBody`] from the
/// entity's split `BodyKinematics` + `ProjectileGameplay` so the tests can
/// keep asserting on `.body.kin` / `.body.game` exactly as before.
pub(in crate::projectile) fn projectile_bodies(
    app: &mut App,
) -> Vec<crate::projectile::ProjectileBody> {
    use crate::projectile::{ProjectileGameplay, ProjectileSeq};
    let world = app.world_mut();
    let mut q = world
        .try_query::<(
            &crate::player::BodyKinematics,
            &ProjectileGameplay,
            &ProjectileSeq,
        )>()
        .unwrap();
    let mut rows: Vec<(ProjectileSeq, crate::projectile::ProjectileBody)> = q
        .iter(world)
        .map(|(kin, game, seq)| {
            (
                *seq,
                crate::projectile::ProjectileBody::from_parts(*kin, *game),
            )
        })
        .collect();
    rows.sort_by_key(|(seq, _)| *seq);
    rows.into_iter().map(|(_, body)| body).collect()
}

/// Directly spawn an in-flight player projectile entity owned by the
/// primary player — the entity-era equivalent of the old
/// `state.bodies.push(InFlightProjectile { .. })` test setup. Assigns the
/// next monotonic `ProjectileSeq` so injected bodies keep a stable order.
pub(in crate::projectile) fn spawn_player_projectile(
    app: &mut App,
    body: crate::projectile::ProjectileBody,
    owner_id: &str,
) {
    let owner = primary_player_entity(app);
    let seq = {
        let mut counter = app
            .world_mut()
            .get_resource_or_insert_with(crate::projectile::ProjectileSeqCounter::default);
        counter.next()
    };
    app.world_mut().spawn((
        body.kin,
        body.game,
        crate::projectile::ProjectileOwner(owner),
        seq,
        crate::projectile::ProjectileOwnerId(owner_id.to_string()),
        crate::projectile::PlayerProjectile,
        Name::new("Player projectile (test)"),
    ));
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
