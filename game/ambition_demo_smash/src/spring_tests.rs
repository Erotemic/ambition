//! The plate, on the road a game runs it: the module
//! `ambition_demo_smash_modules::spring` drops a module entity
//! (`ambition.world.spawn_module_entity`) and ticks it
//! (`ambition.world.module_entity_tick`), and a throw is
//! `ambition.world.launch_toucher`.
//!
//! The plate's clocks and uses are a module record, so these tests read them
//! as what the plate does: when it throws, and how often.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::smash_spring::{PlaceSpringParams, PLACE_SPRING};
use ambition_platformer2d::extension::sdk::phases::{MODULE_ENTITY_TICK, TECHNIQUE_EXECUTION};
use ambition_platformer2d::extension::{ExtensionAppExt as _, ExtensionHostPlugin, ExtensionSet};

const DT: f32 = 1.0 / 60.0;

/// Where the plate comes from: linked into the game, or its file built for
/// `wasm32-unknown-unknown` and loaded.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Road {
    Linked,
    Loaded,
}

fn app() -> App {
    app_on(Road::Linked)
}

fn app_on(road: Road) -> App {
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_message::<ambition_platformer2d::vfx::vfx::VfxInRoom>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_plugins(ExtensionHostPlugin::new(Update))
        .init_resource::<ambition_platformer2d::time::SimTick>()
        .insert_resource(ambition_platformer2d::time::WorldTime::new(0.0, DT));
    {
        let mut time = app.world_mut().resource_mut::<ambition_platformer2d::time::WorldTime>();
        time.set_sim_dt(DT);
        time.set_wall_dt(DT);
    }
    ambition_platformer2d::combat::extension::special::install_for_specials(&mut app);
    ambition_platformer2d::abilities::extension::install_for_technique_execution(&mut app);
    ambition_platformer2d::abilities::extension::install_module_entities(&mut app);
    // A plate dropped this tick is ticked this tick, as in the game.
    app.configure_sets(
        Update,
        ExtensionSet::Collect(MODULE_ENTITY_TICK).after(ExtensionSet::Lower(TECHNIQUE_EXECUTION)),
    );
    match road {
        Road::Linked => {
            app.add_extension_module(ambition_demo_smash_modules::spring::module());
        }
        Road::Loaded => {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let file = ambition_platformer2d::extension::build_module_crate(&root, "ambition_demo_smash_modules")
                .expect("the Smash module crate builds for wasm32-unknown-unknown");
            let (backend, mut modules) =
                ambition_platformer2d::extension::WasmModules::load(&std::fs::read(file).unwrap())
                    .expect("the module file loads");
            // The file holds every Smash module; this fixture offers the
            // ports of the spring only.
            let key = ambition_demo_smash_modules::spring::module().key;
            modules.retain(|module| module.key == key);
            app.add_loaded_extension_modules("ambition_demo_smash_modules.wasm", backend, modules, false);
        }
    }
    app.finish();
    app
}

/// A fighter's half size: a plate 6 high reaches 32 above its middle.
const HALF: ae::Vec2 = ae::Vec2::new(14.0, 26.0);

/// A fighter that can drop a plate (it names what it spawns) and be thrown.
fn body(app: &mut App, seat: usize, at: ae::Vec2) -> Entity {
    use ambition_platformer2d::platformer::sim_id::{SimId, SimIdCounter};
    app.world_mut()
        .spawn((
            ae::BodyKinematics { pos: at, facing: 1.0, size: HALF * 2.0, ..Default::default() },
            ae::CenteredAabb::new(at, HALF),
            ambition_platformer2d::actor::ActorFaction::Player,
            ambition_platformer2d::platformer::lifecycle::FeatureSimEntity,
            SimId::placement(&format!("seat{seat}")),
            SimIdCounter::default(),
        ))
        .id()
}

/// Put `who` at `at`, at rest: the plate reads the published box.
fn put(app: &mut App, who: Entity, at: ae::Vec2, vel: ae::Vec2) {
    let world = app.world_mut();
    let mut kin = world.get_mut::<ae::BodyKinematics>(who).unwrap();
    kin.pos = at;
    kin.vel = vel;
    *world.get_mut::<ae::CenteredAabb>(who).unwrap() = ae::CenteredAabb::new(at, HALF);
}

fn params() -> PlaceSpringParams {
    PlaceSpringParams {
        vfx: "test_plate".to_string(),
        // Up is negative y, as everywhere in this codebase.
        launch: (0.0, -900.0),
        half_extents: (22.0, 6.0),
        lifetime_s: 8.0,
        uses: 3,
        offset: (0.0, 18.0),
        // Not the shipped 0.3 and 0.25, so a plate that kept a Rust delay
        // fails `the_plate_keeps_the_delays_its_move_authored`.
        arm_s: 0.28,
        rearm_s: 0.2,
    }
}

fn drop_plate_with(app: &mut App, actor: Entity, params: &PlaceSpringParams) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(PLACE_SPRING.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(params).expect("spring params serialize"),
    };
    app.world_mut().write_message(ActorActionMessage { actor, request, move_instance: None });
    app.update();
}

fn drop_plate(app: &mut App, actor: Entity) {
    drop_plate_with(app, actor, &params());
}

fn plates(app: &mut App) -> Vec<ae::Vec2> {
    app.world_mut()
        .query::<&ambition_platformer2d::abilities::module_entity::ModuleEntity>()
        .iter(app.world())
        .map(|plate| plate.pos)
        .collect()
}

fn thrown(app: &App, who: Entity) -> bool {
    app.world().get::<ae::BodyKinematics>(who).unwrap().vel.y < -800.0
}

/// Hold `who` on the plate at rest for `ticks` ticks; the ticks (from 1) on
/// which the plate threw it.
fn throws_while_held(app: &mut App, who: Entity, at: ae::Vec2, ticks: usize) -> Vec<usize> {
    let mut throws = Vec::new();
    for tick in 1..=ticks {
        put(app, who, at, ae::Vec2::ZERO);
        app.update();
        if thrown(app, who) {
            throws.push(tick);
        }
    }
    throws
}

#[test]
fn the_plate_lands_where_the_move_asked_and_throws_who_steps_on_it() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, -200.0));
    drop_plate(&mut app, engineer);
    // At the dropper's position plus the offset, `(0, -200) + (0, 18)`.
    assert_eq!(plates(&mut app), vec![ae::Vec2::new(0.0, -182.0)]);
    // Walk him off it: a dropper is inside his own plate by construction.
    put(&mut app, engineer, ae::Vec2::new(600.0, -200.0), ae::Vec2::ZERO);
    let walker = body(&mut app, 0, ae::Vec2::new(0.0, -182.0));
    for _ in 0..20 {
        app.update();
    }
    let vel = app.world().get::<ae::BodyKinematics>(walker).unwrap().vel;
    assert!(vel.y < -800.0, "the plate did not throw him: {vel:?}");
}

/// It throws anybody, so it is stage, not kit. A plate that served only its
/// dropper would be a second recovery.
#[test]
fn the_plate_throws_the_fighter_who_dropped_it_too() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, 0.0));
    drop_plate(&mut app, engineer);
    app.update();
    assert!(!thrown(&app, engineer), "the plate threw its dropper while it was arming");
    for _ in 0..20 {
        app.update();
    }
    assert!(thrown(&app, engineer), "his own plate refused him, so it is kit and not stage");
}

/// The delays are the move's (`arm_s` 0.28, `rearm_s` 0.2), not Rust's: held
/// on the plate, a body is thrown first when the arming has run out, and then
/// each time the re-arm has; never on consecutive ticks.
#[test]
fn the_plate_keeps_the_delays_its_move_authored() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, -200.0));
    drop_plate(&mut app, engineer);
    put(&mut app, engineer, ae::Vec2::new(600.0, -200.0), ae::Vec2::ZERO);
    let loiterer = body(&mut app, 0, ae::Vec2::new(0.0, -182.0));
    let throws = throws_while_held(&mut app, loiterer, ae::Vec2::new(0.0, -182.0), 60);
    // The clocks count down in f32 steps of 1/60 s. 0.28 s takes 17 steps,
    // and the drop tick is the first, so the first throw is on the 16th tick
    // after it. 0.2 s leaves 1.1e-8 s after 12 steps, so a throw comes each
    // 13th tick. Rust's old 0.3 and 0.25 would give 18 and 16.
    assert_eq!(throws, vec![16, 29, 42], "the throws of one continuous stand");
}

#[test]
fn the_plate_is_taken_away_when_its_uses_run_out() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, 0.0));
    drop_plate(&mut app, engineer);
    let throws = throws_while_held(&mut app, engineer, ae::Vec2::new(0.0, 0.0), 60);
    assert_eq!(throws.len(), 3, "a three-use plate threw {throws:?}");
    assert!(plates(&mut app).is_empty(), "the plate outlived its three uses");
}

/// The launch replaces the arriving velocity: an additive plate would throw a
/// fast-falling body less far than a walking one.
#[test]
fn the_launch_replaces_the_speed_you_arrived_with() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, -200.0));
    drop_plate(&mut app, engineer);
    put(&mut app, engineer, ae::Vec2::new(600.0, -200.0), ae::Vec2::ZERO);
    let faller = body(&mut app, 0, ae::Vec2::new(0.0, -182.0));
    let mut throw = None;
    for _ in 0..20 {
        put(&mut app, faller, ae::Vec2::new(0.0, -182.0), ae::Vec2::new(0.0, 700.0));
        app.update();
        let vel = app.world().get::<ae::BodyKinematics>(faller).unwrap().vel;
        if vel.y < 0.0 {
            throw = Some(vel);
            break;
        }
    }
    assert_eq!(throw, Some(ae::Vec2::new(0.0, -900.0)), "the plate added to his speed or never threw");
}

#[test]
fn the_plate_is_taken_away_when_its_clock_runs_out() {
    let mut app = app();
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, -400.0));
    drop_plate(&mut app, engineer);
    for _ in 0..(8.0 * 60.0) as usize + 4 {
        app.update();
    }
    assert!(plates(&mut app).is_empty(), "the plate outlived its clock");
}

/// Two fighters on one plate: one use throws one of them, and the same one in
/// either spawn order, or two peers resimulating the tick throw different
/// fighters.
#[test]
fn two_fighters_on_one_plate_launch_the_same_one_in_either_spawn_order() {
    let launched_seat = |reversed: bool| -> usize {
        let mut app = app();
        let seats: [usize; 2] = if reversed { [1, 0] } else { [0, 1] };
        let bodies: Vec<(usize, Entity)> =
            seats.iter().map(|&s| (s, body(&mut app, s, ae::Vec2::new(0.0, -200.0)))).collect();
        let mut one_use = params();
        one_use.uses = 1;
        drop_plate_with(&mut app, bodies[0].1, &one_use);
        for _ in 0..20 {
            app.update();
        }
        let moved: Vec<usize> = bodies.iter().filter(|(_, e)| thrown(&app, *e)).map(|(s, _)| *s).collect();
        assert_eq!(moved.len(), 1, "a one-use plate threw {moved:?} (reversed={reversed})");
        moved[0]
    };
    assert_eq!(launched_seat(false), launched_seat(true), "the throw depends on the spawn order");
}

/// The effect cues the app wrote since `cursor` last read.
fn cues(
    app: &App,
    cursor: &mut bevy::ecs::message::MessageCursor<ambition_platformer2d::vfx::vfx::VfxInRoom>,
) -> usize {
    let messages = app.world().resource::<Messages<ambition_platformer2d::vfx::vfx::VfxInRoom>>();
    cursor
        .read(messages)
        .filter(|m| matches!(m.vfx, ambition_platformer2d::vfx::vfx::VfxMessage::Effect { .. }))
        .count()
}

/// A plate announces itself when it arrives and when it throws: it draws
/// nothing of its own, so an unannounced plate is an ambush. A plate whose
/// move authors no cue is silent, so the cues are the authored field's.
#[test]
fn a_plate_with_an_authored_cue_announces_both_its_arrival_and_its_launch() {
    let mut placed = app();
    let mut seen = Default::default();
    let dropper = body(&mut placed, 0, ae::Vec2::new(0.0, -200.0));
    let mut one_use = params();
    one_use.uses = 1;
    drop_plate_with(&mut placed, dropper, &one_use);
    assert_eq!(cues(&placed, &mut seen), 1, "the plate arrived silently");
    put(&mut placed, dropper, ae::Vec2::new(600.0, -200.0), ae::Vec2::ZERO);
    let _victim = body(&mut placed, 1, ae::Vec2::new(0.0, -182.0));
    let mut fired = 0;
    for _ in 0..24 {
        placed.update();
        fired += cues(&placed, &mut seen);
    }
    assert_eq!(fired, 1, "the plate threw silently");

    let mut quiet = app();
    let mut seen = Default::default();
    let hand = body(&mut quiet, 0, ae::Vec2::new(0.0, -200.0));
    let mut silent = params();
    silent.vfx = String::new();
    drop_plate_with(&mut quiet, hand, &silent);
    assert_eq!(cues(&quiet, &mut seen), 0, "a plate that authored no cue announced itself anyway");
}

/// A plate belongs to the match that made it, as a bomb or a mine does: the
/// world spawns it, and this game stamps it with the running match, so the
/// match's end sweeps it.
#[test]
fn a_plate_belongs_to_the_match_that_dropped_it() {
    use ambition_platformer2d::versus_match::{ActiveMatch, MatchScoped};
    let mut app = app();
    let active = ActiveMatch::for_test(2);
    app.insert_resource(active.clone());
    app.add_systems(Update, crate::stamp_smash_module_entities.after(ExtensionSet::Lower(TECHNIQUE_EXECUTION)));
    let engineer = body(&mut app, 1, ae::Vec2::new(0.0, -200.0));
    drop_plate(&mut app, engineer);
    app.update();
    let stamps: Vec<Option<MatchScoped>> = app
        .world_mut()
        .query::<(&ambition_platformer2d::abilities::module_entity::ModuleEntity, Option<&MatchScoped>)>()
        .iter(app.world())
        .map(|(_, stamp)| stamp.cloned())
        .collect();
    assert_eq!(stamps, vec![Some(MatchScoped(active.instance()))], "the plate is not the match's");
}

/// The plate from the module file throws as the linked one does, and its
/// entries run from the file.
#[test]
fn the_plate_from_its_module_file_throws_as_the_linked_one_does() {
    let stand = |road: Road| {
        let mut app = app_on(road);
        let runner = {
            let admitted = app.world().resource::<ambition_platformer2d::extension::AdmittedExtensions>();
            let entry = admitted.0.entries.iter().find(|entry| entry.descriptor.key.as_ref() == PLACE_SPRING);
            format!("{:?}", entry.expect("the drop entry is admitted").runner)
        };
        assert_eq!(runner.starts_with("Loaded"), road == Road::Loaded, "{road:?}: {runner}");
        let engineer = body(&mut app, 1, ae::Vec2::new(0.0, -200.0));
        drop_plate(&mut app, engineer);
        put(&mut app, engineer, ae::Vec2::new(600.0, -200.0), ae::Vec2::ZERO);
        let loiterer = body(&mut app, 0, ae::Vec2::new(0.0, -182.0));
        throws_while_held(&mut app, loiterer, ae::Vec2::new(0.0, -182.0), 60)
    };
    let linked = stand(Road::Linked);
    assert_eq!(linked, vec![16, 29, 42], "the linked plate's throws");
    assert_eq!(stand(Road::Loaded), linked, "the loaded plate throws as the linked one does");
}

/// Authoring a silent plate is refused at the seam: a default cue would be
/// the invisible-ambush state the field exists to end.
#[test]
#[should_panic(expected = "announces nothing")]
fn a_plate_authored_with_no_cue_is_refused() {
    let mut silent = params();
    silent.vfx = String::new();
    let _ = ambition_platformer2d::entity_catalog::smash_spring::author_place_spring(
        ambition_entity_catalog::authoring::hitless_special("silent_plate", "special", 0.0, 0.10),
        0.05,
        silent,
    );
}
