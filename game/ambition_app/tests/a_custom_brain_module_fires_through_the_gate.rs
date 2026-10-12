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
/// module loaded or not.
fn run(module: Option<&std::path::Path>, held: bool) -> Run {
    let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if let Some(module) = module {
        options = options.with_extension_module_files(vec![module.to_path_buf()]);
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
    let unloaded = run(None, false);
    assert_eq!(unloaded.fires.len(), FRAMES, "the probe ran once a frame");
    assert_eq!(fires_up(&unloaded.fires), 0, "with no module, no frame fires straight up: {:?}", unloaded.fires);
    assert_eq!(unloaded.up_shots, 0, "with no module, no shot of the thinker flies straight up");

    // A fire that reached the body after the gate would be in no gated frame,
    // and the moveset would still fire it: so the held body's shots are the
    // assertion that the gate saw the fire.
    let held = run(Some(&wasm), true);
    assert_eq!(fires_up(&held.fires), 0, "the gate removes the module's fire from a held body: {:?}", held.fires);
    assert_eq!(
        held.up_shots, 0,
        "a body a script holds fires none of the module's shots, as it fires none of a brain's"
    );

    let free = run(Some(&wasm), false);
    assert!(
        fires_up(&free.fires) >= FRAMES / 2,
        "the module's fire is in the gated frame of a free body: {:?}",
        free.fires
    );
    assert!(free.up_shots > 0, "the body's moveset fires the module's fire");
}
