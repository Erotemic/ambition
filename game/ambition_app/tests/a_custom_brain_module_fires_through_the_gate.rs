//! ⭐ FAST-ITERATION I4, `actor_think`: a body whose placement authors a
//! custom brain key decides through a module, and the module's fire is in
//! the frame the control gate restricts.
//!
//! `fixtures/extension_fixture_modules` is a crate the game does not link.
//! Its `thinker` module binds the brain key `fixture_thinker` and fires
//! straight up every tick its body has a target. The test builds it for
//! `wasm32-unknown-unknown`, loads the file into the shipped composition, and
//! reads the body's control frame between the gate and the integration (the
//! frame every consumer of control reads) and the shots the body fires.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::characters::control::{ActorControl, ControlHold, ControlHolds};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::platformer::schedule::{PlayerInputSet, SimScheduleExt, WorldPrepSet};
use ambition_platformer2d::projectiles::entity::ProjectileOwner;
use bevy::prelude::*;

const THINKER_ID: &str = "custom_brain_thinker";
/// `ambition_extension_fixture_modules::THINKER_BRAIN` and `THINKER_FIRE`:
/// the app does not link the fixture crate.
const THINKER_BRAIN: &str = "fixture_thinker";
const THINKER_FIRE: Vec2 = Vec2::new(0.0, -1.0);
const FRAMES: usize = 40;

/// The thinker's fire direction in each gated frame.
#[derive(Resource, Default)]
struct GatedFires(Vec<Option<Vec2>>);

/// What a thinker did in one run.
struct Run {
    /// Its fire in each gated frame.
    fires: Vec<Option<Vec2>>,
    /// The frames on which a shot it owns flies straight up: the module's
    /// shot, and no brain's.
    up_shots: usize,
}

fn record_gated_fire(mut seen: ResMut<GatedFires>, bodies: Query<(&FeatureId, &ActorControl)>) {
    let fire = bodies.iter().find(|(id, _)| id.as_str() == THINKER_ID).and_then(|(_, control)| control.0.fire);
    seen.0.push(fire.map(|request| request.dir));
}

/// A thinker beside the player, `held` by a script or not, with the fixture
/// module loaded or not, under a GGRS sync test or not.
fn run(module: Option<&std::path::Path>, held: bool, rollback: bool) -> Run {
    let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if let Some(module) = module {
        options = options.with_extension_module_files(vec![module.to_path_buf()]);
    }
    if rollback {
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
    let player = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        q.single(world).expect("primary player").pos
    };
    sim.spawn_enemy_character_at(
        THINKER_ID,
        "Thinker",
        (player.x + 120.0, player.y),
        (14.0, 23.0),
        CharacterBrain::Custom(THINKER_BRAIN.to_string()),
        "perfect_cellular_automaton",
    );
    let thinker = {
        let world = sim.world_mut();
        world
            .query::<(Entity, &FeatureId)>()
            .iter(world)
            .find(|(_, id)| id.as_str() == THINKER_ID)
            .map(|(entity, _)| entity)
            .expect("the thinker is spawned")
    };
    if held {
        sim.world_mut().entity_mut(thinker).insert(ControlHolds::only(ControlHold::Sequence));
    }
    let sim_schedule = sim.app_mut().sim_schedule();
    sim.app_mut().init_resource::<GatedFires>().add_systems(
        sim_schedule,
        record_gated_fire.after(PlayerInputSet::ControlGate).before(WorldPrepSet::BeforeIntegrate),
    );
    let mut up_shots = 0;
    for _ in 0..FRAMES {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        up_shots += world
            .query::<(&ProjectileOwner, &BodyKinematics)>()
            .iter(world)
            .filter(|(owner, kin)| owner.0 == thinker && kin.vel.y < 0.0 && kin.vel.x.abs() < 1e-3)
            .count();
    }
    Run { fires: std::mem::take(&mut sim.world_mut().resource_mut::<GatedFires>().0), up_shots }
}

fn fires_up(fires: &[Option<Vec2>]) -> usize {
    fires.iter().filter(|fire| fire.is_some_and(|dir| dir.abs_diff_eq(THINKER_FIRE, 1e-6))).count()
}

#[test]
fn a_custom_brain_module_fires_through_the_gate_and_a_held_body_does_not() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm = ambition_platformer2d::extension::build_module_crate(&root, "ambition_extension_fixture_modules")
        .expect("the fixture crate builds for wasm32-unknown-unknown");

    // Control: no module. No brain of the game fires straight up, so a frame
    // or a shot that does is the module's.
    let unloaded = run(None, false, false);
    assert_eq!(unloaded.fires.len(), FRAMES, "the probe ran once a frame");
    assert_eq!(fires_up(&unloaded.fires), 0, "with no module, no frame fires straight up: {:?}", unloaded.fires);
    assert_eq!(unloaded.up_shots, 0, "with no module, no shot of the thinker flies straight up");

    // A fire that reached the body after the gate would be in no gated frame,
    // and the moveset would still fire it: so the held body's shots are the
    // assertion that the gate saw the fire.
    let held = run(Some(&wasm), true, false);
    assert_eq!(fires_up(&held.fires), 0, "the gate removes the module's fire from a held body: {:?}", held.fires);
    assert_eq!(
        held.up_shots, 0,
        "a body a script holds fires none of the module's shots, as it fires none of a brain's"
    );

    let free = run(Some(&wasm), false, false);
    assert!(
        fires_up(&free.fires) >= FRAMES / 2,
        "the module's fire is in the gated frame of a free body: {:?}",
        free.fires
    );
    assert!(free.up_shots > 0, "the body's moveset fires the module's fire");
}

/// The same free thinker under a GGRS sync test: each rewound tick replays to
/// the same state. Its brain reads whether its body is running before the
/// body moves this tick, so that fact must come back with a rewind
/// (SHOT-END-REPLAY: it did not, and the replay chose another mode).
#[test]
fn a_custom_brain_module_replays_the_same_under_rollback() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm = ambition_platformer2d::extension::build_module_crate(&root, "ambition_extension_fixture_modules")
        .expect("the fixture crate builds for wasm32-unknown-unknown");
    let rolled = run(Some(&wasm), false, true);
    assert!(
        fires_up(&rolled.fires) >= FRAMES / 2,
        "the module's fire is in the gated frame under rollback too: {:?}",
        rolled.fires
    );
}

/// `ambition_extension_fixture_modules::Thoughts` (kept durable) and
/// `Musings` (transient), both body records of the thinker.
const THOUGHTS: ambition_platformer2d::extension::sdk::SchemaKey =
    ambition_platformer2d::extension::sdk::SchemaKey::new("fixture", "thinker.thoughts", 1);
const MUSINGS: ambition_platformer2d::extension::sdk::SchemaKey =
    ambition_platformer2d::extension::sdk::SchemaKey::new("fixture", "thinker.musings", 1);

fn thinker_entity(sim: &mut Platformer2dSimHarness) -> Option<Entity> {
    let world = sim.world_mut();
    world
        .query::<(Entity, &FeatureId)>()
        .iter(world)
        .find(|(_, id)| id.as_str() == THINKER_ID)
        .map(|(entity, _)| entity)
}

/// (thoughts, musings) on the live thinker.
fn counts(sim: &mut Platformer2dSimHarness) -> (Option<u32>, Option<u32>) {
    use ambition_platformer2d::extension::sdk::{FieldRef, Value};
    let thinker = thinker_entity(sim).expect("a live thinker");
    let records = sim.world().get::<ambition_platformer2d::extension::BodyRecords>(thinker);
    let read = |key| {
        records.and_then(|r| r.get(key)).map(|record| match record.get(FieldRef(0)) {
            Ok(Value::U32(ticks)) => *ticks,
            other => panic!("{key} holds {other:?}"),
        })
    };
    (read(&THOUGHTS), read(&MUSINGS))
}

fn spawn_thinker(sim: &mut Platformer2dSimHarness) {
    let player = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        q.single(world).expect("primary player").pos
    };
    sim.spawn_enemy_character_at(
        THINKER_ID,
        "Thinker",
        (player.x + 120.0, player.y),
        (14.0, 23.0),
        CharacterBrain::Custom(THINKER_BRAIN.to_string()),
        "perfect_cellular_automaton",
    );
}

/// A second thinker, with no durable identity: the control.
const DRIFTER_ID: &str = "custom_brain_drifter";

/// Stamp the thinker as a provider-staged occupant of its room, in the
/// command flush that builds it, as the room planner stamps what it stages.
/// The harness builds it on the out-of-plan road, which gives a body no plan
/// identity (`apply_spawn_actor_requests`); the drifter keeps that.
fn stamp_the_thinker_as_staged(
    added: On<Add, FeatureId>,
    ids: Query<&FeatureId>,
    mut commands: Commands,
) {
    if ids.get(added.entity).is_ok_and(|id| id.as_str() == THINKER_ID) {
        commands.entity(added.entity).insert(ambition_platformer2d::platformer::construction::SpawnOrigin::ProviderStaged {
            provider: "sim-harness".into(),
            room: "sandbox".into(),
            instance: THINKER_ID.into(),
        });
    }
}

/// The thinker built again under the identity it had: what a room entered
/// again does with its staged occupant.
fn build_thinker_again(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::actor::{ActorFaction, SpawnActorKind, SpawnActorRequest};
    let player = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        q.single(world).expect("primary player").pos
    };
    sim.world_mut().write_message(SpawnActorRequest {
        id: THINKER_ID.into(),
        name: "Thinker".into(),
        pos: Vec2::new(player.x + 120.0, player.y),
        half_size: Vec2::new(14.0, 23.0),
        faction: ActorFaction::Enemy,
        grudge_against: None,
        kind: SpawnActorKind::Enemy {
            brain: CharacterBrain::Custom(THINKER_BRAIN.to_string()),
            character: ambition_platformer2d::character::CharacterId::from("perfect_cellular_automaton"),
        },
    });
}

/// ⭐ RUNTIME-AUTHORING P5 STEP 2: a body record whose schema the durable
/// world save keeps outlives the live body. The thinker's `thoughts` count
/// goes on when the body is built again under its identity (its room entered
/// again) and when a session loads the save, through the file; `musings`
/// beside it, transient, starts again each time. A New Game takes the kept
/// record away with the rest of the world.
#[test]
fn a_kept_body_record_outlives_its_body_and_comes_back_with_the_save() {
    use ambition_platformer2d::persistence::save::{load_save, write_save};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm = ambition_platformer2d::extension::build_module_crate(&root, "ambition_extension_fixture_modules")
        .expect("the fixture crate builds for wasm32-unknown-unknown");
    let options = || {
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_extension_module_files(vec![wasm.clone()])
    };
    let save_of = |sim: &Platformer2dSimHarness| {
        sim.world()
            .resource::<ambition_platformer2d::session::AmbitionGameSave>()
            .data()
            .clone()
    };

    let mut sim = Platformer2dSimHarness::new_with_options(options()).expect("the sandbox builds");
    sim.app_mut().add_observer(stamp_the_thinker_as_staged);
    spawn_thinker(&mut sim);
    let player = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        q.single(world).expect("primary player").pos
    };
    sim.spawn_enemy_character_at(
        DRIFTER_ID,
        "Drifter",
        (player.x - 120.0, player.y),
        (14.0, 23.0),
        CharacterBrain::Custom(THINKER_BRAIN.to_string()),
        "perfect_cellular_automaton",
    );
    sim.step_n(AgentAction::default(), 30);
    let (thoughts, musings) = counts(&mut sim);
    let first = thoughts.expect("the premise: the thinker thought");
    assert!(first > 0 && musings == Some(first), "the premise: both counts agree on a fresh body: {thoughts:?} {musings:?}");
    let thinker = thinker_entity(&mut sim).unwrap();
    let identity = sim
        .world()
        .get::<ambition_platformer2d::platformer::sim_id::SimId>(thinker)
        .expect("the thinker has an identity")
        .as_str()
        .to_owned();
    let rows: Vec<(String, String)> = save_of(&sim)
        .body_module_records()
        .iter()
        .map(|row| (row.body.clone(), row.key.clone()))
        .collect();
    assert_eq!(
        rows,
        [(identity.clone(), "fixture::thinker.thoughts@1".to_owned())],
        "the save keeps the durable record, under the staged body's identity, and not the transient one; \
         the drifter, which has no durable identity, keeps nothing"
    );

    // The body leaves the world, and the save keeps its row.
    sim.world_mut().entity_mut(thinker).despawn();
    sim.step_n(AgentAction::default(), 5);
    assert!(thinker_entity(&mut sim).is_none(), "the premise: the thinker is gone");
    assert_eq!(save_of(&sim).body_module_records().len(), 1, "a body that is not live keeps its row");
    let gone_at = first + 1;

    build_thinker_again(&mut sim);
    sim.step_n(AgentAction::default(), 3);
    let (thoughts, musings) = counts(&mut sim);
    assert!(
        thoughts.is_some_and(|t| t > gone_at) && musings.is_some_and(|m| m < first),
        "built again: the kept count goes on from {gone_at}, the transient one starts again: {thoughts:?} {musings:?}"
    );
    sim.step_n(AgentAction::default(), 10);
    let (before_save, _) = counts(&mut sim);
    let before_save = before_save.unwrap();

    let save = save_of(&sim);
    let dir = root.join(format!("target/extension-modules/durable_body_record_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("sandbox_save.ron");
    write_save(&file, &save).expect("the save is written");
    let loaded = load_save(&file);
    assert_eq!(loaded.data, save, "the file holds the save the game kept");

    let mut sim = Platformer2dSimHarness::new_with_options(options().with_save(loaded.data))
        .expect("the sandbox builds from the save");
    sim.app_mut().add_observer(stamp_the_thinker_as_staged);
    spawn_thinker(&mut sim);
    sim.step_n(AgentAction::default(), 5);
    let (thoughts, musings) = counts(&mut sim);
    assert!(
        thoughts.is_some_and(|t| t > before_save) && musings.is_some_and(|m| m <= 6),
        "after the load the kept count goes on from {before_save}, the transient one starts again: {thoughts:?} {musings:?}"
    );

    sim.world_mut()
        .write_message(ambition_platformer2d::session::reset::NewGameRequested);
    sim.step_n(AgentAction::default(), 240);
    let (thoughts, musings) = counts(&mut sim);
    assert!(
        thoughts.is_some() && thoughts == musings,
        "a New Game starts the kept record from nothing, as the transient one starts: {thoughts:?} {musings:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
