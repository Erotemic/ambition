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

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::{Block, World};
use bevy::prelude::*;

use super::systems::{charge_projectile_input, step_projectiles};
use crate::trace::GameplayTraceBuffer;
use ambition_characters::actor::BodyHealth;
use ambition_characters::control::{PlayerSlot, SeatRawFrames};
use ambition_combat::components::ActorIdentity;
use ambition_combat::events::{GameplayBanner, HitEvent, SetFlagRequested};
use ambition_input::ControlFrame;
use ambition_platformer2d_core::RoomGeometry;
use ambition_projectiles::state::PlayerProjectileState;
use ambition_vfx::vfx::DebrisBurstMessage;
use ambition_vfx::vfx::VfxMessage;

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
    // (`BodyKinematics`, `PlayerEntity`, `PrimaryPlayer`, `LocalPlayer`, the
    // cluster components, …) with no manual spawn-tuple list.
    let mut scratch = crate::avatar::primary_player_scratch(pos, ae::AbilitySet::sandbox_all());
    scratch.kinematics.facing = facing;
    scratch.ground.on_ground = true;
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        scratch,
        ambition_characters::actor::Health::new(10),
    );
    app.world_mut().spawn(bundle);
}

/// Register the player-kit motion techniques the fire policy expects (qcf /
/// qcf_grace / hcf). Mirrors `ambition_content::input_techniques`, duplicated
/// here because this crate cannot depend on the content crate.
fn register_test_motion_techniques(app: &mut App) {
    use ambition_projectiles::MotionDirection::{Down, DownLeft, DownRight, Left, Right};
    use ambition_projectiles::{MotionTechnique, MotionTechniqueAppExt};
    app.register_motion_technique(
        "qcf",
        MotionTechnique::new(vec![
            vec![Down, DownRight, Right],
            vec![Down, DownLeft, Left],
        ]),
    );
    app.register_motion_technique(
        "qcf_grace",
        MotionTechnique::new(vec![vec![Down, Right], vec![Down, Left]]),
    );
    app.register_motion_technique(
        "hcf",
        MotionTechnique {
            patterns: vec![
                vec![Right, DownRight, Down, DownLeft, Left],
                vec![Left, DownLeft, Down, DownRight, Right],
            ],
            invert_facing: true,
        },
    );
}

fn projectile_test_app(world: World, player_pos: ae::Vec2, facing: f32) -> App {
    let mut app = App::new();
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    app.insert_resource(Time::<()>::default());
    app.insert_resource(ambition_time::WorldTime::default());
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        RoomGeometry(world),
    );
    // `step_projectiles` collides against the portal-carved world; no carves in
    // these tests, so the overlay is empty (collision == raw world).
    app.init_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>();
    app.insert_resource(SeatRawFrames::default());
    app.insert_resource(ambition_persistence::settings::UserSettings::default());
    app.insert_resource(GameplayTraceBuffer::default());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    // Projectile state lives on the player; this counter only gives in-flight
    // projectile entities stable spawn order.
    app.init_resource::<ambition_projectiles::ProjectileSeqCounter>();
    // The stepper resolves each shot's visual id through the (empty here) content
    // catalog for its detonation-FX pick; init it so the `Res` param validates.
    app.init_resource::<ambition_projectiles::ProjectileVisualCatalog>();
    // The fire policy resolves gestures through the content-owned technique
    // catalog; register the player-kit patterns the charging tests exercise (the
    // production set lives in `ambition_content::input_techniques`).
    register_test_motion_techniques(&mut app);
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<HitEvent>();
    app.add_message::<ambition_combat::events::ActorStimulus>();
    app.add_message::<ambition_combat::stocks::BodyKnockedOut>();
    app.add_message::<ambition_damage::WalletShieldSpent>();
    app.add_message::<ambition_projectiles::ProjectileSpawnRequest>();
    // The unified stepper can heal the player on a parry, so the message must be
    // registered even though player projectiles never trigger it.
    app.add_message::<crate::avatar::PlayerHealRequested>();
    app.init_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>();
    app.add_plugins(ambition_characters::brain::BrainPlugin);
    app.add_systems(
        Update,
        (
            // Commit every seat's shaped raw frame into SlotControls, then let
            // the player brain translate that canonical slot (production order).
            crate::schedule::publish_seat_controls_when_nobody_else_does,
            crate::avatar::tick_controlled_brains,
            ambition_characters::brain::emit_player_projectile_tick_messages,
            // Mirror production order: the unified stepper advances existing
            // shots, THEN input fires + the delayed request materializer runs (so a
            // shot fired this frame first ticks next frame), then feature hits.
            step_projectiles,
            charge_projectile_input,
            ambition_projectiles::materialize_projectiles_for_next_tick,
            crate::features::apply_feature_hit_events,
        )
            .chain(),
    );
    spawn_player(&mut app, player_pos, facing);
    app
}

fn min_app() -> App {
    projectile_test_app(dummy_world(), ae::Vec2::new(300.0, 300.0), 1.0)
}

/// Read-only view of the primary player's `PlayerProjectileState`.
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
        .try_query_filtered::<Entity, With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>>()
        .unwrap();
    q.iter(world)
        .next()
        .expect("min_app spawned exactly one player")
}

/// Collect the in-flight player projectile bodies, sorted by spawn
/// sequence (oldest first) — the same order the old `state.bodies` Vec
/// presented. Recomposes a [`ambition_projectiles::ProjectileBody`] from the
/// entity's split `BodyKinematics` + `ProjectileGameplay` so the tests can
/// keep asserting on `.body.kin` / `.body.game` exactly as before.
pub(in crate::projectile) fn projectile_bodies(
    app: &mut App,
) -> Vec<ambition_projectiles::ProjectileBody> {
    use ambition_projectiles::{ProjectileGameplay, ProjectileSeq};
    let world = app.world_mut();
    let mut q = world
        .try_query::<(
            &ambition_platformer2d_core::BodyKinematics,
            &ProjectileGameplay,
            &ProjectileSeq,
        )>()
        .unwrap();
    let mut rows: Vec<(ProjectileSeq, ambition_projectiles::ProjectileBody)> = q
        .iter(world)
        .map(|(kin, game, seq)| {
            (
                *seq,
                ambition_projectiles::ProjectileBody::from_parts(*kin, *game),
            )
        })
        .collect();
    rows.sort_by_key(|(seq, _)| *seq);
    rows.into_iter().map(|(_, body)| body).collect()
}

/// Collect the in-flight player projectile *kinds*, sorted by spawn sequence
/// (oldest first). The named kind rides as its own `ProjectileKind` component
/// (the engine body is generic), so kind assertions read it here rather than
/// off `ProjectileBody`. `None` for any kind-less shot.
pub(in crate::projectile) fn projectile_kinds(
    app: &mut App,
) -> Vec<Option<ambition_projectiles::ProjectileKind>> {
    use ambition_projectiles::{ProjectileKind, ProjectileSeq};
    let world = app.world_mut();
    let mut q = world
        .try_query::<(&ProjectileSeq, Option<&ProjectileKind>)>()
        .unwrap();
    let mut rows: Vec<(ProjectileSeq, Option<ProjectileKind>)> = q
        .iter(world)
        .map(|(seq, kind)| (*seq, kind.copied()))
        .collect();
    rows.sort_by_key(|(seq, _)| *seq);
    rows.into_iter().map(|(_, kind)| kind).collect()
}

/// Directly spawn an in-flight player projectile entity owned by the
/// primary player — the entity-era equivalent of the old
/// `state.bodies.push(InFlightProjectile { .. })` test setup. Assigns the
/// next monotonic `ProjectileSeq` so injected bodies keep a stable order.
pub(in crate::projectile) fn spawn_player_projectile(
    app: &mut App,
    body: ambition_projectiles::ProjectileBody,
) {
    let owner = primary_player_entity(app);
    let seq = {
        let mut counter = app
            .world_mut()
            .get_resource_or_insert_with(ambition_projectiles::ProjectileSeqCounter::default);
        counter.next()
    };
    app.world_mut().spawn((
        body.kin,
        body.game,
        ambition_projectiles::ProjectileOwner(owner),
        seq,
        ambition_projectiles::LiveProjectile,
        Name::new("Player projectile (test)"),
    ));
}

fn advance_time(app: &mut App, dt_seconds: f32) {
    let mut time = app.world_mut().resource_mut::<Time<()>>();
    time.advance_by(std::time::Duration::from_secs_f32(dt_seconds));
    // `step_projectiles` reads `Res<WorldTime>`, not `Res<Time>`,
    // so the test harness must mirror the production pipeline's
    // `refresh_world_time` step. Tests run at `time_scale = 1.0`,
    // so `sim_dt == wall_dt`.
    let mut world_time = app.world_mut().resource_mut::<ambition_time::WorldTime>();
    world_time.raw_dt = dt_seconds;
    world_time.scaled_dt = dt_seconds;
}

/// It stopped being the input bus when every seat gained a raw row of its own; it is a mirror
/// of what seat zero received, published at the end.
fn shape_primary(app: &mut App, edit: impl FnOnce(&mut ControlFrame)) {
    app.world_mut()
        .resource_mut::<SeatRawFrames>()
        .shape(PlayerSlot::PRIMARY, edit);
}

fn tap_projectile(app: &mut App) {
    // Press frame: just_pressed=true, held=true (Bevy's button
    // semantics — pressed state lasts as long as held), released=false.
    // The system enters the press branch and starts charging.
    shape_primary(app, |frame| {
        frame.projectile_pressed = true;
        frame.projectile_held = true;
        frame.projectile_released = false;
    });
    advance_time(app, 0.016);
    app.update();
    // Release frame: just_pressed=false, held=false, released=true.
    shape_primary(app, |frame| {
        frame.projectile_pressed = false;
        frame.projectile_held = false;
        frame.projectile_released = true;
    });
    advance_time(app, 0.016);
    app.update();
    // Reset the edge for the next test step.
    shape_primary(app, |frame| {
        frame.projectile_released = false;
    });
}
