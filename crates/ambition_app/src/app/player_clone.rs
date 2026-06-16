//! Brain-driven player CLONE — a non-player entity that carries the full player
//! movement body and is driven by a `StateMachineCfg::PlayerDemo` brain through
//! the EXACT same movement integration the human player uses.
//!
//! This is the live, in-game counterpart to the headless proof in
//! `ambition_sandbox::player::clone_probe_tests`. It demonstrates the
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

use ambition_sandbox::brain::{ActorControl, Brain, BrainSnapshot, StateMachineCfg};
use ambition_sandbox::dev::dev_tools::EditableMovementTuning;
use ambition_sandbox::engine_core as ae;
use ambition_sandbox::engine_core::movement::InputState;
use ambition_sandbox::GameWorld;

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
    world: Res<GameWorld>,
    player_q: Query<
        &ambition_sandbox::player::BodyKinematics,
        With<ambition_sandbox::player::PlayerEntity>,
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
        ae::PlayerClusterScratch::new_with_abilities(spawn, ae::AbilitySet::sandbox_all());

    let size = scratch.kinematics.size;
    let transform = Transform::from_translation(ambition_sandbox::config::world_to_bevy(
        &world.0,
        spawn,
        ambition_sandbox::config::WORLD_Z_PLAYER,
    ));

    // 18 cluster components are over Bevy's tuple-bundle arity, so nest them.
    let clusters_a = (
        scratch.abilities,
        scratch.kinematics,
        scratch.base_size,
        scratch.ground,
        scratch.wall,
        scratch.jump,
        scratch.dash,
        scratch.flight,
        scratch.blink,
        scratch.ledge,
        scratch.dodge,
    );
    let clusters_b = (
        scratch.shield,
        scratch.body_mode,
        scratch.env_contact,
        scratch.mana,
        scratch.offense,
        scratch.action_buffer,
        scratch.lifetime,
        scratch.combo_trace,
    );
    commands.spawn((
        clusters_a,
        clusters_b,
        Brain::StateMachine(StateMachineCfg::PlayerDemo {
            cfg: ambition_sandbox::brain::state_machine::PlayerDemoCfg::default(),
            state: Default::default(),
        }),
        ActorControl::default(),
        PlayerClone,
        Sprite {
            color: Color::srgba(1.0, 0.55, 0.95, 0.9),
            custom_size: Some(Vec2::new(size.x, size.y)),
            ..default()
        },
        transform,
        Name::new("Player Clone (brain-driven)"),
    ));
}

/// Build the engine `InputState` from a clone's brain-emitted control frame.
/// Mirrors `engine_input_from_actor_control` for the player path: `desired_vel`
/// is the normalized stick AXIS. No hitstun on a clone.
fn input_from_actor_control(
    f: &ambition_sandbox::actor::control::ActorControlFrame,
    dt: f32,
) -> InputState {
    InputState {
        axis_x: f.desired_vel.x,
        axis_y: f.desired_vel.y,
        jump_pressed: f.jump_pressed,
        jump_held: f.jump_held,
        jump_released: f.jump_released,
        dash_pressed: f.dash_pressed,
        fly_toggle_pressed: f.fly_toggle_pressed,
        fast_fall_pressed: f.fast_fall_pressed,
        attack_pressed: f.melee_pressed,
        pogo_pressed: f.pogo_pressed,
        interact_pressed: f.interact_pressed,
        shield_held: f.shield_held,
        control_dt: dt,
        ..InputState::default()
    }
}

/// Drive every player clone: tick its brain → `ActorControl`, then run the
/// shared player movement core on its clusters. The SAME
/// `update_player_with_tuning_clusters` the human player uses — no clone-
/// specific integration.
pub fn drive_player_clones(
    time: Res<Time>,
    world: Res<GameWorld>,
    editable_tuning: Res<EditableMovementTuning>,
    gravity_field: Option<Res<ambition_sandbox::physics::GravityField>>,
    platform_set: Res<ambition_sandbox::MovingPlatformSet>,
    overlay: Res<ambition_sandbox::features::FeatureEcsWorldOverlay>,
    mut clock: ResMut<PlayerCloneClock>,
    mut clones: Query<
        (ae::PlayerClusterQueryData, &mut Brain, &mut ActorControl),
        With<PlayerClone>,
    >,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    clock.0 += dt;
    let mut tuning = editable_tuning.as_engine();
    let gdir = ambition_sandbox::physics::gravity_dir_or_default(gravity_field.as_deref());
    ambition_sandbox::physics::apply_gravity_dir(&mut tuning, gdir);
    let control_world =
        ambition_sandbox::features::world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);

    for (mut cluster_item, mut brain, mut control) in &mut clones {
        let mut clusters = cluster_item.as_clusters_mut();
        let mut snapshot = BrainSnapshot::idle();
        snapshot.actor_pos = clusters.kinematics.pos;
        snapshot.actor_vel = clusters.kinematics.vel;
        snapshot.actor_facing = clusters.kinematics.facing;
        snapshot.actor_on_ground = clusters.ground.on_ground;
        snapshot.alive = true;
        snapshot.sim_time = clock.0;
        snapshot.dt = dt;

        let mut frame = ambition_sandbox::actor::control::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);
        control.0 = frame;

        let input = input_from_actor_control(&frame, dt);
        ae::update_player_with_tuning_clusters(&control_world, &mut clusters, input, dt, tuning);
    }
}

/// Keep the clone's sprite on its simulated body.
pub fn sync_player_clone_transform(
    world: Res<GameWorld>,
    mut clones: Query<
        (&ambition_sandbox::player::BodyKinematics, &mut Transform),
        With<PlayerClone>,
    >,
) {
    for (kin, mut transform) in &mut clones {
        transform.translation = ambition_sandbox::config::world_to_bevy(
            &world.0,
            kin.pos,
            ambition_sandbox::config::WORLD_Z_PLAYER,
        );
    }
}
