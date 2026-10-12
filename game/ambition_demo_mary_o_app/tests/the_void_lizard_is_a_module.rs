//! ⭐ The Void Lizard's brain is an extension module
//! (`ambition_demo_mary_o_modules::void_lizard`, fast-iteration I4
//! `actor_think`), so an edit to its throws or its hop compiles that crate
//! alone and the Mary-O crate does not compile again.
//!
//! The test reads the lizard's control frame between the control gate and
//! the integration, each tick of a whole round, and compares it with the
//! module's own schedule (`throws_on`, `throw_direction`, `hops_on`): the
//! authority, not a copy of it. A lizard a script holds throws nothing,
//! because the module's throw is in the frame the gate restricts. And the
//! same round runs with the module loaded from its `.wasm` file: the road a
//! developer edits on, with no build of this app.

use bevy::prelude::*;

use ambition_demo_mary_o_modules::void_lizard::{hops_on, throw_direction, throws_on, HOP_SPEED, ROUND_TICKS, SIGHT};
use ambition_platformer2d::characters::control::{ActorControl, ControlHold, ControlHolds};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::platformer::schedule::{PlayerInputSet, SimScheduleExt, WorldPrepSet};

const CASTLE: &str = "mary_o_1_4";

/// What the lizard's gated frame held on one tick.
#[derive(Clone, Copy, Debug)]
struct Seen {
    tick: u64,
    fire: Option<ae::Vec2>,
    vel_y: f32,
    on_ground: bool,
    to_her: f32,
}

#[derive(Resource, Default)]
struct Frames(Vec<Seen>);

fn record(
    mut frames: ResMut<Frames>,
    tick: Res<ambition_platformer2d::sim::SimTick>,
    lizards: Query<(
        &ambition_platformer2d::combat::actor_tuning::ActorConfig,
        &ActorControl,
        &ae::BodyKinematics,
        &ae::BodyGroundState,
    )>,
    her: Query<&ae::BodyKinematics, With<PrimaryPlayer>>,
) {
    let Ok(her) = her.single() else {
        return;
    };
    for (config, control, kin, ground) in &lizards {
        if ambition_demo_mary_o::void_lizard::is_void_lizard_brain(&config.brain) {
            frames.0.push(Seen {
                tick: tick.get(),
                fire: control.0.fire.map(|request| request.dir),
                vel_y: kin.vel.y,
                on_ground: ground.on_ground,
                to_her: her.pos.x - kin.pos.x,
            });
        }
    }
}

/// A round and a little more in the castle, with Mary-O held in the air
/// within sight of the lizard. She cannot be hurt: an orb that killed her
/// would replay the room and make a new lizard, and the round would be about
/// two bodies.
fn a_round(held: bool, loaded: Option<&std::path::Path>) -> Vec<Seen> {
    let mut app = match loaded {
        Some(file) => ambition_demo_mary_o_app::build_demo_app_loading(vec![file.to_path_buf()]),
        None => ambition_demo_mary_o_app::build_demo_app(),
    };
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(CASTLE.to_string()));
    let lizard = (0..600)
        .find_map(|_| {
            app.update();
            let world = app.world_mut();
            let mut q = world.query::<(Entity, &ambition_platformer2d::combat::actor_tuning::ActorConfig)>();
            let found = q
                .iter(world)
                .find(|(_, config)| ambition_demo_mary_o::void_lizard::is_void_lizard_brain(&config.brain))
                .map(|(entity, _)| entity);
            let seated = world.query_filtered::<(), With<PrimaryPlayer>>().iter(world).next().is_some();
            found.filter(|_| seated)
        })
        .expect("the castle builds its Void Lizard and seats Mary-O");
    let runner = {
        let admitted = app.world().resource::<ambition_platformer2d::extension::AdmittedExtensions>();
        let entry = admitted.0.entries.iter().find(|entry| entry.path == "mary_o::void_lizard/think");
        format!("{:?}", entry.expect("the Void Lizard's entry is admitted").runner)
    };
    assert_eq!(
        runner.starts_with("Loaded"),
        loaded.is_some(),
        "the entry runs from the file exactly when a file is loaded: {runner}"
    );
    if held {
        app.world_mut().entity_mut(lizard).insert(ControlHolds::only(ControlHold::Sequence));
    }
    let sim = app.sim_schedule();
    app.init_resource::<Frames>()
        .add_systems(sim, record.after(PlayerInputSet::ControlGate).before(WorldPrepSet::BeforeIntegrate));
    for _ in 0..ROUND_TICKS + 30 {
        let at = app
            .world()
            .get::<ae::BodyKinematics>(lizard)
            .expect("the same lizard lives the whole round: nothing replayed the room")
            .pos;
        {
            let world = app.world_mut();
            let mut q = world.query_filtered::<(
                &mut ae::BodyKinematics,
                &mut ambition_platformer2d::characters::actor::BodyHealth,
            ), With<PrimaryPlayer>>();
            let (mut body, mut health) = q.single_mut(world).expect("her body");
            body.pos = ae::Vec2::new(at.x - SIGHT * 0.5, at.y - 160.0);
            body.vel = ae::Vec2::ZERO;
            health.health.invulnerable.set(ambition_platformer2d::characters::actor::Invulnerability::SCRIPTED, true);
        }
        app.update();
    }
    std::mem::take(&mut app.world_mut().resource_mut::<Frames>().0)
}

/// Every tick of `round` is the module's throw or nothing, and the round
/// has its five throws and its hop.
fn on_schedule(round: &[Seen], label: &str) {
    assert!(
        round.len() as u64 >= ROUND_TICKS,
        "{label}: the probe saw the lizard on {} ticks, less than a round",
        round.len()
    );
    assert!(round.iter().all(|seen| seen.to_her.abs() <= SIGHT), "{label}: she stayed in sight");
    let mut throws = 0;
    let mut hops = 0;
    for seen in round {
        let expected = throws_on(seen.tick).then(|| ae::Vec2::from(throw_direction(seen.tick, seen.to_her)));
        assert_eq!(seen.fire, expected, "{label}: tick {}: the gated frame is the module's throw or nothing", seen.tick);
        throws += usize::from(expected.is_some());
        if hops_on(seen.tick) && seen.on_ground {
            assert_eq!(seen.vel_y, -HOP_SPEED, "{label}: tick {}: it hops", seen.tick);
            hops += 1;
        }
    }
    assert_eq!(throws, 5, "{label}: a round has five throws");
    assert_eq!(hops, 1, "{label}: a round has one hop, from the ground");
}

#[test]
fn the_void_lizard_throws_and_hops_on_its_modules_schedule_and_not_when_held() {
    on_schedule(&a_round(false, None), "linked");

    let held = a_round(true, None);
    assert!(held.len() as u64 >= ROUND_TICKS);
    let thrown: Vec<_> = held.iter().filter(|seen| seen.fire.is_some()).collect();
    assert!(thrown.is_empty(), "the gate removes the throw of a lizard a script holds: {thrown:?}");
}

/// The road a developer edits on: the module crate built for
/// `wasm32-unknown-unknown` and loaded by the app, which does not compile.
#[test]
fn the_void_lizard_runs_from_its_module_file() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm = ambition_platformer2d::extension::build_module_crate(&root, "ambition_demo_mary_o_modules")
        .expect("the Mary-O module crate builds for wasm32-unknown-unknown");
    on_schedule(&a_round(false, Some(&wasm)), "loaded");
}
