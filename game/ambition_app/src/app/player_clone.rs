//! Brain-driven player CLONE — a non-player entity that carries the full player
//! movement body and is driven by a `StateMachineCfg::PlayerDemo` brain through
//! the EXACT same movement integration the human player uses.
//!
//! This is the live, in-game counterpart to the headless proof in
//! `ambition_platformer2d::actors::avatar::clone_probe_tests`. It demonstrates the
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

use ambition_platformer2d::characters::brain::{Brain, BrainSnapshot, StateMachineCfg};
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::RoomGeometry;
use ambition_platformer2d::platformer::construction::SpawnOrigin;
use ambition_platformer2d::platformer::sim_id::{SimId, SimIdCounter};
use ambition_platformer2d::render::rendering::{player_presentation_for_collision, PlayerVisual};
use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;

/// Marks a brain-driven player-body clone (NOT the human player).
#[derive(Component)]
pub struct PlayerClone;

/// Toggle flag set by the dev hotkey / menu — spawn one clone next frame.
#[derive(Resource, Default)]
pub struct SpawnPlayerCloneRequest(pub bool);

/// The mechanical-edit domain a clone spawn belongs to.
///
/// ⭐ SPAWNING A BODY IS AN AUTHORING ACT, NOT A PLAYER ACTION, and that is the
/// whole reason this domain exists rather than a bit on `ControlFrame`. Both
/// roads would make the press survive a rewind; they disagree about what the
/// press MEANS. Riding the input payload would declare a developer hotkey to be
/// seat-zero gameplay input — and in a session with a real remote peer it would
/// transmit "spawn a clone" into a shared match as that seat's move. The
/// mechanical-edit road says the author changed the world, so the local baseline
/// is stood down and rebased, and a FOREIGN timeline refuses outright with the
/// proposal left pending. That is the honest answer for a dev tool.
pub fn player_clone_domain() -> ae::MechanicalDomain {
    ae::MechanicalDomain::of::<SpawnPlayerCloneRequest>("spawn_player_clone")
}

/// Raise a requested clone spawn as a mechanical-edit PROPOSAL.
///
/// ⛔ IT KEYS ON THE REQUEST FLAG, NOT ON THE KEY PRESS, and that is deliberate:
/// `SpawnPlayerCloneRequest` was already the seam between the device read and
/// the spawn (the live tests poke it directly), so proposing from the flag keeps
/// every existing caller — hotkey, menu, test — on one road instead of teaching
/// each of them the edit protocol.
pub fn propose_player_clone_spawn(
    request: Res<SpawnPlayerCloneRequest>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
) {
    if request.0 {
        pending.propose(player_clone_domain());
    }
}

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
///
/// ⭐ **THE `Publish` HALF OF THE MECHANICAL-EDIT ROAD, NOT A SIM SYSTEM.** It
/// ran in `app.sim_schedule()` until 2026-09-18, which lost the press to every
/// rewind (`Q136`); `plugins.rs` records the mechanism at the registration. It
/// may only write once `decide_mechanical_edit_admission` has answered, which is
/// why the admission is a parameter rather than a precondition stated in prose.
pub fn spawn_requested_player_clone(
    mut commands: Commands,
    mut request: ResMut<SpawnPlayerCloneRequest>,
    // ⛔ ABSENT MEANS PUBLISH, and that is safe for the same STRUCTURAL reason
    // the other four publishers rely on: the decider is registered by
    // `install_session_bridge`, the same call that installs the GGRS session, so
    // a composition cannot hold a rollback timeline without holding the decider.
    admission: Option<Res<ae::MechanicalEditAdmission>>,
    mut pending: ResMut<ae::PendingMechanicalEdits>,
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    // Optional: the headless RL harness has no loaded character sheets. Absent →
    // the clone falls back to a tinted rectangle (movement still works).
    game_assets: Option<Res<GameAssets>>,
    // PRIMARY-only: spawn the clone relative to the camera body. Once a clone is
    // itself a PlayerEntity, a bare single() here would Err on the second spawn.
    // The clone mirrors the primary's WORN IDENTITY as well as its position, so it
    // looks like whoever the player currently is rather than like a hardcoded
    // protagonist. `Option` because a bare test/demo body may wear nothing.
    mut player_q: Query<
        (
            &ambition_platformer2d::engine_core::BodyKinematics,
            Option<&ambition_platformer2d::characters::actor::WornCharacter>,
            // The clone descends from the primary. `SimId::spawned` needs the
            // parent identity and the parent mint stream, and this site already
            // reads the primary body.
            //
            // `Option` because a bare fixture body can carry no identity. A
            // production body always carries one: `ensure_sim_id` runs before
            // `CoreSimulation`, and this system runs inside it.
            Option<&SimId>,
            Option<&mut SimIdCounter>,
        ),
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
) {
    if !request.0 {
        return;
    }
    // ⛔⛤ **THE PRESS IS NOT SPENT UNTIL A BODY EXISTS TO BE CLONED, AND IT USED
    // TO BE SPENT FIRST.** `request.0 = false` stood here, above every refusal
    // below, so a press that arrived on a frame with no resolvable primary — or
    // with a primary that carried no `SimId` yet — was consumed and the clone
    // never appeared. Refusing a sub-step is not refusing the operation: the
    // flag now survives every early return and the spawn happens on the first
    // frame that can actually perform it.
    if matches!(
        admission.as_deref(),
        Some(ae::MechanicalEditAdmission::Refuse)
    ) {
        // A foreign or unhealthy timeline may not be mutated. The proposal stays
        // pending, so the press fires when the refusal lifts.
        return;
    }
    let Ok((player_kin, worn, parent_id, parent_counter)) = player_q.single_mut() else {
        return;
    };
    // Spawn a little to the left of the player so it reads as a separate body.
    let spawn = player_kin.pos + ae::Vec2::new(-90.0, -20.0);

    // ⛔ THE IDENTITY IS MINTED HERE, WHERE THE BODY IS BUILT.
    //
    // This body carries `BodyKinematics` and `PlayerEntity`, so
    // `player_simulation_system` integrates it like the human player's body. It
    // carries no authored `FeatureId`, and it is deliberately not a
    // `PrimaryPlayer`. `ensure_sim_id` matches neither of its arms and skips it,
    // so no later system can name this body. ADR 0030 puts a dynamic entity's
    // identity at the site that knows its spawner. This site knows it.
    //
    // ⛔ Refuse when the parent has no identity. Do not invent one. ADR 0030: a
    // dynamic entity that cannot name its parent cannot be reconstructed.
    let (Some(parent_id), Some(mut parent_counter)) = (parent_id, parent_counter) else {
        warn!(
            "a player clone was requested, but the primary player carries no \
             `SimId` to descend from, so the clone would reach the simulation \
             unnameable. Refusing to spawn it rather than minting an identity \
             nothing can reconstruct."
        );
        return;
    };
    // Use the parent's own stream. A global counter couples unrelated spawners.
    let parent = parent_id.clone();
    let sequence = parent_counter.next();
    let clone_id = SimId::spawned(&parent, sequence);
    // Provenance is data, not the spelling of the id (ADR 0030). Nothing may
    // recover the parent from `clone_id`.
    let origin = SpawnOrigin::Dynamic { parent, sequence };
    let scratch = ae::BodyClusterScratch::new_with_abilities(spawn, ae::AbilitySet::sandbox_all());

    let size = scratch.kinematics.size;
    let transform =
        Transform::from_translation(ambition_platformer2d::engine_core::config::world_to_bevy(
            &world.0,
            spawn,
            ambition_platformer2d::engine_core::config::WORLD_Z_PLAYER,
        ));

    // The clone carries the IDENTICAL movement component set as the player and
    // every actor: `BodyKinematics` (shared kinematic truth) + the shared
    // `AncillaryMovementBundle` (the 18 ancillary clusters). Same bundle the
    // player's `PlayerSimulationBundle` and `ActorClusterSeed::into_components`
    // nest — the convergence the ActorBody-unwrap bought.
    let kinematics = scratch.kinematics;
    let movement =
        ambition_platformer2d::platformer::body::AncillaryMovementBundle::from_scratch(scratch);
    // The published combat footprint every body carries (§A6); kept live by
    // `integrate_home_body` like the primary's.
    let hurtbox = ae::CenteredAabb::from_center_size(kinematics.pos, kinematics.size);
    let mut clone = commands.spawn((
        kinematics,
        movement,
        hurtbox,
        Brain::StateMachine(StateMachineCfg::PlayerDemo {
            cfg: ambition_platformer2d::characters::brain::state_machine::PlayerDemoCfg::default(),
            state: Default::default(),
        }),
        ActorControl::default(),
        PlayerClone,
        // Visual + combat state the SHARED `animate_player` path reads. With these
        // (+ the textured sprite/animator below + `PlayerVisual`), the clone animates
        // through the IDENTICAL player picker — `animate_player` now iterates every
        // `PlayerVisual` body, not just the primary.
        (
            ambition_platformer2d::characters::actor::BodyAnimFacts::default(),
            ambition_platformer2d::characters::actor::BodyCombat::default(),
            ambition_platformer2d::platformer::camera_ease::PlayerBlinkCameraState::default(),
        ),
        // The clone IS a `PlayerEntity` (3c-ii): the iterating
        // `player_control_system` / `player_simulation_system` move it through the
        // EXACT shared player core, driven by its own `ActorControl`. These are the
        // remaining components those queries require (the 18 movement clusters + the
        // three visual states above complete the set). It is deliberately NOT a
        // `PrimaryPlayer` (so `is_primary` gates the world-globals off for it) and
        // NOT a `DrivingParticipant` (so the device-input `tick_controlled_brains`
        // skips it — nobody is driving it, its
        // `PlayerDemo` brain is ticked by `tick_player_clone_brains` with real
        // sim-time/dt instead).
        ambition_platformer2d::platformer::markers::PlayerEntity,
        // Every integrated body carries one explicit movement policy from
        // spawn, and every player body carries the movement→policy hand-off —
        // the unified integration phase requires both.
        ambition_platformer2d::actor::MotionModel::default(),
        ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput::default(),
        (
            ambition_platformer2d::actors::body_mode::BodyModeCapabilities::full(),
            ambition_platformer2d::combat::BodyMelee::default(),
            ambition_platformer2d::platformer::safe_position::PlayerSafetyState::default(),
        ),
        // Nested, to keep the top-level bundle tuple inside Bevy's arity limit.
        (
            transform,
            Name::new("Player Clone (brain-driven)"),
            // Inserted in the same command batch as the body, so no flush shows
            // this body without its identity.
            clone_id,
            origin,
        ),
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
        .zip(worn)
        .and_then(|(g, worn)| g.characters.sheet(worn.id()));
    if let Some(asset) = asset {
        // ⚠ The COLLISION is this site's own (a clone has no pose to read, so it
        // takes the default body); the DERIVATION from it is not.
        let (sprite, clone_anchor, animator, baseline) =
            player_presentation_for_collision(asset, collision, None, None);
        clone.insert((sprite, clone_anchor, animator, baseline, PlayerVisual));
    } else {
        clone.insert(Sprite {
            color: Color::srgba(1.0, 0.55, 0.95, 0.9),
            custom_size: Some(Vec2::new(size.x, size.y)),
            ..default()
        });
    }
    // ⭐ SPENT LAST, WHERE THE OPERATION IS COMMITTED. Everything above can
    // refuse, and the press must outlive every refusal. `commands.spawn` is
    // already queued at this point, so there is nothing left that can decline.
    request.0 = false;
    pending.take(player_clone_domain());
}

/// Tick every player clone's `PlayerDemo` brain → its `ActorControl` frame.
///
/// This is the clone's counterpart to `tick_controlled_brains` (which produces the PRIMARY's
/// `ActorControl` from device input). The clone's brain is a *timed* demo cycle, so it needs
/// real `sim_time`/`dt` in its snapshot — which is why it can't ride the unfiltered
/// `tick_controlled_brains` (that passes `dt = 0`) and the clone holds no seat. Movement itself
/// is NO LONGER here: now that the clone is a `PlayerEntity`, the iterating
/// `player_control_system` / `player_simulation_system` integrate its clusters from this
/// `ActorControl` — the same shared core the human player runs.
pub fn tick_player_clone_brains(
    // ⛔⛤ **`Res<Time>` AND A HOST-LOCAL ACCUMULATOR UNTIL 2026-09-18, WHICH
    // DESYNCED THE TIMELINE THE MOMENT A CLONE EXISTED UNDER ROLLBACK.** This
    // read `time.delta_secs()` — the app's WALL dt — and added it to a
    // `PlayerCloneClock` resource that was `init_resource`d and never registered
    // for rollback. Resimulating a frame therefore accumulated it AGAIN from a
    // value no rewind restored, so `snapshot.sim_time` differed between the
    // original run and the replay, the demo brain emitted a different frame, and
    // the clone's `BodyKinematics` diverged. Measured: `GGRS sync-test checksum
    // mismatch at frames [14, 15, ..]`, reproducible on the first press.
    //
    // ⭐ IT IS A DUPLICATE AUTHORITY, AND THE COLLAPSE IS THE FIX RATHER THAN A
    // REGISTRATION. `GameplayElapsed` is the same fact — accumulated as
    // `+= world_time.scaled_dt`, rollback-registered, advanced at the head of
    // `WorldPrep` and documented as *"before any actor brain reads the
    // snapshot"*, which is where this system reads it. Registering
    // `PlayerCloneClock` would have made the drift rewind correctly and left two
    // owners of "how long gameplay has run"; deleting it leaves one.
    world_time: Res<ambition_platformer2d::time::WorldTime>,
    elapsed: Res<ambition_platformer2d::actors::features::GameplayElapsed>,
    mut clones: Query<
        (
            &ambition_platformer2d::engine_core::BodyKinematics,
            &ambition_platformer2d::engine_core::BodyGroundState,
            &ambition_platformer2d::world::ResolvedMotionFrame,
            &mut Brain,
            &mut ActorControl,
        ),
        With<PlayerClone>,
    >,
) {
    // ⚠ THE ZERO GUARD IS KEPT AND NOW MEANS SOMETHING SHARPER: `sim_dt` is
    // `raw_dt * time_scale`, so it is zero while the game is PAUSED or in
    // hitstop. Ticking a demo cycle through a pause was never intended.
    let dt = world_time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (kin, ground, resolved_frame, mut brain, mut control) in &mut clones {
        let mut snapshot = BrainSnapshot::idle();
        snapshot.actor_pos = kin.pos;
        snapshot.actor_vel = kin.vel;
        snapshot.actor_facing = kin.facing;
        // The clone resolves and consumes its OWN body frame: its demo brain
        // interprets directions in the same frame its integration uses, so a
        // clone inside a rotated-gravity zone walks that zone's way instead of
        // the hardcoded screen frame it used to assume.
        snapshot.control_down = resolved_frame.down();
        snapshot.actor_on_ground = ground.on_ground;
        snapshot.alive = true;
        snapshot.sim_time = elapsed.0;
        snapshot.dt = dt;

        let mut frame =
            ambition_platformer2d::characters::actor::control::ActorControlFrame::neutral();
        ambition_platformer2d::actors::brain_tick::tick_brain(&mut brain, &snapshot, &mut frame);
        control.0 = frame;
    }
}

/// On a sandbox reset, despawn brain-driven clones. A clone is a transient dev
/// body (spawned with K), like the held-item / portal / summon transients the
/// sandbox's own `clear_transient_on_sandbox_reset` clears — but `PlayerClone`
/// lives in this app crate, so the despawn is app-side.
///
/// Keyed on `NewGameResetCommitted`, the same signal that engine-side clear uses, and for the
/// same reason: a reset whose room preflight refuses must leave the running session exactly as
/// it found it.
pub fn despawn_player_clones_on_reset(
    mut committed: MessageReader<
        ambition_platformer2d::actors::session::reset::NewGameResetCommitted,
    >,
    clones: Query<Entity, With<PlayerClone>>,
    mut commands: Commands,
) {
    if committed.read().count() == 0 {
        return;
    }
    for entity in &clones {
        commands.entity(entity).despawn();
    }
}

/// Keep the clone's sprite on its simulated body.
pub fn sync_player_clone_transform(
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    mut clones: Query<
        (
            &ambition_platformer2d::engine_core::BodyKinematics,
            &mut Transform,
        ),
        With<PlayerClone>,
    >,
) {
    for (kin, mut transform) in &mut clones {
        transform.translation = ambition_platformer2d::engine_core::config::world_to_bevy(
            &world.0,
            kin.pos,
            ambition_platformer2d::engine_core::config::WORLD_Z_PLAYER,
        );
    }
}
