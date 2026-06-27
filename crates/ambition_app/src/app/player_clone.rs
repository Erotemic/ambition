//! Brain-driven player CLONE — a non-player entity that carries the full player
//! movement body and is driven by a `StateMachineCfg::PlayerDemo` brain through
//! the EXACT same movement integration the human player uses.
//!
//! This is the live, in-game counterpart to the headless proof in
//! `ambition_gameplay_core::player::clone_probe_tests`. It demonstrates the
//! universal-brain seam: the clone runs / jumps / dashes / flies entirely from
//! brain-emitted `ActorControlFrame` verbs.
//!
//! Design note (see the player-clone probe in
//! `docs/journals/content-authoring-pain-points.md`): the PRIMARY player keeps
//! its full, entangled tick — it owns the global concerns (world clock, moving-
//! platform advance, camera, sandbox reset) that a clone must NOT touch. The
//! clone gets this focused driver that reuses the shared per-entity movement
//! core (`update_player_with_tuning_clusters`) without those globals. The deeper
//! refactor — decoupling those globals so ONE loop drives every player-bodied
//! entity — is the documented follow-up.

use bevy::prelude::*;

use ambition_gameplay_core::assets::game_assets::GameAssets;
use ambition_characters::brain::{ActorControl, Brain, BrainSnapshot, StateMachineCfg};
use ambition_gameplay_core::character_sprites::{
    build_character_sprite_with_render_size, feet_anchor_for_render_size,
    player_placeholder_render_size, CharacterAnimator,
};
use ambition_engine_core as ae;
use ambition_gameplay_core::RoomGeometry;
use ambition_render::rendering::{PlayerSpriteBaseline, PlayerVisual};

/// Marks a brain-driven player-body clone (NOT the human player).
#[derive(Component)]
pub struct PlayerClone;

/// Monotonic clock for the clone brains (the `PlayerDemo` cycle timing).
#[derive(Resource, Default)]
pub struct PlayerCloneClock(pub f32);

/// Toggle flag set by the dev hotkey / menu — spawn one clone next frame.
#[derive(Resource, Default)]
pub struct SpawnPlayerCloneRequest(pub bool);

/// `\`-style dev hotkey: press `K` to spawn a brain-driven player clone at the
/// player's position. Cheap to gate behind a key so it never appears unbidden.
pub fn request_player_clone_on_key(
    // Optional: the headless RL harness has no keyboard resource. Absent → no-op
    // (tests poke `SpawnPlayerCloneRequest` directly).
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut request: ResMut<SpawnPlayerCloneRequest>,
) {
    if keys.is_some_and(|k| k.just_pressed(KeyCode::KeyK)) {
        request.0 = true;
    }
}

/// Spawn a player-body clone at the player's current position when requested.
/// The clone carries all 18 movement clusters (full ability set) + a
/// `PlayerDemo` brain + an `ActorControl` + a placeholder sprite.
pub fn spawn_requested_player_clone(
    mut commands: Commands,
    mut request: ResMut<SpawnPlayerCloneRequest>,
    world: Res<RoomGeometry>,
    // Optional: the headless RL harness has no loaded character sheets. Absent →
    // the clone falls back to a tinted rectangle (movement still works).
    game_assets: Option<Res<GameAssets>>,
    // PRIMARY-only: spawn the clone relative to the camera body. Once a clone is
    // itself a PlayerEntity, a bare single() here would Err on the second spawn.
    player_q: Query<
        &ambition_gameplay_core::actor::BodyKinematics,
        ambition_gameplay_core::actor::PrimaryPlayerOnly,
    >,
) {
    if !request.0 {
        return;
    }
    request.0 = false;
    let Ok(player_kin) = player_q.single() else {
        return;
    };
    // Spawn a little to the left of the player so it reads as a separate body.
    let spawn = player_kin.pos + ae::Vec2::new(-90.0, -20.0);
    let scratch =
        ae::BodyClusterScratch::new_with_abilities(spawn, ae::AbilitySet::sandbox_all());

    let size = scratch.kinematics.size;
    let transform = Transform::from_translation(ambition_gameplay_core::config::world_to_bevy(
        &world.0,
        spawn,
        ambition_gameplay_core::config::WORLD_Z_PLAYER,
    ));

    // The clone carries the IDENTICAL movement component set as the player and
    // every actor: `BodyKinematics` (shared kinematic truth) + the shared
    // `AncillaryMovementBundle` (the 18 ancillary clusters). Same bundle the
    // player's `PlayerSimulationBundle` and `ActorClusterSeed::into_components`
    // nest — the convergence the ActorBody-unwrap bought.
    let kinematics = scratch.kinematics;
    let movement = ambition_gameplay_core::actor::AncillaryMovementBundle::from_scratch(scratch);
    let mut clone = commands.spawn((
        kinematics,
        movement,
        Brain::StateMachine(StateMachineCfg::PlayerDemo {
            cfg: ambition_characters::brain::state_machine::PlayerDemoCfg::default(),
            state: Default::default(),
        }),
        ActorControl::default(),
        PlayerClone,
        // Visual + combat state the SHARED `animate_player` path reads. With these
        // (+ the textured sprite/animator below + `PlayerVisual`), the clone animates
        // through the IDENTICAL player picker — `animate_player` now iterates every
        // `PlayerVisual` body, not just the primary.
        (
            ambition_gameplay_core::player::PlayerAnimState::default(),
            ambition_gameplay_core::actor::BodyCombat::default(),
            ambition_gameplay_core::player::PlayerBlinkCameraState::default(),
        ),
        // The clone IS a `PlayerEntity` (3c-ii): the iterating
        // `player_control_system` / `player_simulation_system` move it through the
        // EXACT shared player core, driven by its own `ActorControl`. These are the
        // remaining components those queries require (the 18 movement clusters + the
        // three visual states above complete the set). It is deliberately NOT a
        // `PrimaryPlayer` (so `is_primary` gates the world-globals off for it) and
        // NOT a `PlayerSlot` (so the device-input `tick_player_brains` skips it — its
        // `PlayerDemo` brain is ticked by `tick_player_clone_brains` with real
        // sim-time/dt instead).
        ambition_gameplay_core::actor::PlayerEntity,
        (
            ambition_gameplay_core::player::PlayerInteractionState::default(),
            ambition_gameplay_core::player::ActivePlayerAttack::default(),
            ambition_gameplay_core::player::PlayerSafetyState::default(),
            ambition_gameplay_core::player::PlayerInputFrame::default(),
        ),
        transform,
        Name::new("Player Clone (brain-driven)"),
    ));

    // Real textured player sprite + animator, mirroring `scene_setup`'s primary
    // visual, so the clone looks like the player instead of a placeholder box.
    // Falls back to a tinted rectangle if the character sheet didn't load.
    let collision = Vec2::new(
        ae::DEFAULT_PLAYER_BODY_WIDTH,
        ae::DEFAULT_PLAYER_BODY_HEIGHT,
    );
    let asset = game_assets
        .as_ref()
        .and_then(|g| g.characters.player.as_ref().or(g.characters.robot.as_ref()));
    if let Some(asset) = asset {
        let render = player_placeholder_render_size(&asset.spec, collision);
        clone.insert((
            build_character_sprite_with_render_size(asset, render),
            feet_anchor_for_render_size(&asset.spec, collision, render),
            CharacterAnimator::new(&asset.spec),
            PlayerSpriteBaseline {
                standing_render: render,
                standing_collision: collision,
            },
            PlayerVisual,
        ));
    } else {
        clone.insert(Sprite {
            color: Color::srgba(1.0, 0.55, 0.95, 0.9),
            custom_size: Some(Vec2::new(size.x, size.y)),
            ..default()
        });
    }
}

/// Tick every player clone's `PlayerDemo` brain → its `ActorControl` frame.
///
/// This is the clone's counterpart to `tick_player_brains` (which produces the
/// PRIMARY's `ActorControl` from device input). The clone's brain is a *timed*
/// demo cycle, so it needs real `sim_time`/`dt` in its snapshot — which is why it
/// can't ride the unfiltered `tick_player_brains` (that passes `dt = 0`) and the
/// clone carries no `PlayerSlot`. Movement itself is NO LONGER here: now that the
/// clone is a `PlayerEntity`, the iterating `player_control_system` /
/// `player_simulation_system` integrate its clusters from this `ActorControl` —
/// the same shared core the human player runs. Runs in `PlayerInput`, before the
/// control phase consumes the frame.
pub fn tick_player_clone_brains(
    time: Res<Time>,
    mut clock: ResMut<PlayerCloneClock>,
    mut clones: Query<
        (
            &ambition_gameplay_core::actor::BodyKinematics,
            &ambition_gameplay_core::actor::BodyGroundState,
            &mut Brain,
            &mut ActorControl,
        ),
        With<PlayerClone>,
    >,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    clock.0 += dt;
    for (kin, ground, mut brain, mut control) in &mut clones {
        let mut snapshot = BrainSnapshot::idle();
        snapshot.actor_pos = kin.pos;
        snapshot.actor_vel = kin.vel;
        snapshot.actor_facing = kin.facing;
        snapshot.actor_on_ground = ground.on_ground;
        snapshot.alive = true;
        snapshot.sim_time = clock.0;
        snapshot.dt = dt;

        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);
        control.0 = frame;
    }
}

/// On a sandbox reset, despawn brain-driven clones. A clone is a transient dev
/// body (spawned with K), like the held-item / portal / summon transients the
/// sandbox's own `clear_transient_on_sandbox_reset` clears — but `PlayerClone`
/// lives in this app crate, so the despawn is app-side. Runs before
/// `process_sandbox_reset_request` consumes the request flag, so a reset returns
/// to a clean single-primary world instead of leaving orphaned clones wandering.
pub fn despawn_player_clones_on_reset(
    request: Res<ambition_gameplay_core::session::reset::SandboxResetRequested>,
    clones: Query<Entity, With<PlayerClone>>,
    mut commands: Commands,
) {
    if !request.request {
        return;
    }
    for entity in &clones {
        commands.entity(entity).despawn();
    }
}

/// Keep the clone's sprite on its simulated body.
pub fn sync_player_clone_transform(
    world: Res<RoomGeometry>,
    mut clones: Query<
        (
            &ambition_gameplay_core::actor::BodyKinematics,
            &mut Transform,
        ),
        With<PlayerClone>,
    >,
) {
    for (kin, mut transform) in &mut clones {
        transform.translation = ambition_gameplay_core::config::world_to_bevy(
            &world.0,
            kin.pos,
            ambition_gameplay_core::config::WORLD_Z_PLAYER,
        );
    }
}
