//! ⭐ The Laden Swallow's brain is an extension module
//! (`ambition_demo_mary_o_modules::laden_swallow`, fast-iteration I4
//! `actor_think`), so an edit to when it drops compiles that crate alone.
//!
//! The test reads each swallow's control frame between the control gate and
//! the integration, over two drop periods, and compares it with the module's
//! own rule (`drops_on`, `over`): a coconut straight down exactly on its
//! placement's tick while it is over Mary-O, and nothing else. A swallow a
//! script holds drops nothing. The same rule in Rhai
//! (`rhai/laden_swallow.rhai`), loaded in place of the native `think`,
//! drops on the same ticks.

use bevy::prelude::*;

use ambition_demo_mary_o_modules::laden_swallow::{drops_on, over, DROP_DIRECTION, DROP_EVERY_TICKS};
use ambition_platformer2d::characters::control::{ActorControl, ControlHold, ControlHolds};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::platformer::schedule::{PlayerInputSet, SimScheduleExt, WorldPrepSet};

const SKY: &str = "mary_o_1_3";

/// What one swallow's gated frame held on one tick.
#[derive(Clone, Debug)]
struct Seen {
    tick: u64,
    placement: String,
    fire: Option<ae::Vec2>,
    bird: [f32; 2],
    her: [f32; 2],
}

#[derive(Resource, Default)]
struct Frames(Vec<Seen>);

fn record(
    mut frames: ResMut<Frames>,
    tick: Res<ambition_platformer2d::sim::SimTick>,
    birds: Query<(&ambition_platformer2d::combat::actor_tuning::ActorConfig, &FeatureId, &ActorControl, &ae::BodyKinematics)>,
    her: Query<&ae::BodyKinematics, With<PrimaryPlayer>>,
) {
    let Ok(her) = her.single() else {
        return;
    };
    for (config, id, control, kin) in &birds {
        if ambition_demo_mary_o::swallow::is_laden_swallow_brain(&config.brain) {
            frames.0.push(Seen {
                tick: tick.get(),
                placement: id.as_str().to_owned(),
                fire: control.0.fire.map(|request| request.dir),
                bird: [kin.pos.x, kin.pos.y],
                her: [her.pos.x, her.pos.y],
            });
        }
    }
}

/// Two drop periods in 1-3, with Mary-O held under the first swallow. She
/// cannot be hurt: a coconut that killed her would replay the room.
fn two_periods(held: bool) -> Vec<Seen> {
    two_periods_loading(held, Vec::new())
}

fn two_periods_loading(held: bool, files: Vec<std::path::PathBuf>) -> Vec<Seen> {
    let scripted = !files.is_empty();
    let mut app = if scripted {
        ambition_demo_mary_o_app::build_demo_app_loading(files)
    } else {
        ambition_demo_mary_o_app::build_demo_app()
    };
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(SKY.to_string()));
    let birds: Vec<Entity> = (0..600)
        .find_map(|_| {
            app.update();
            let world = app.world_mut();
            let seated = world.query_filtered::<(), With<PrimaryPlayer>>().iter(world).next().is_some();
            let mut q = world.query::<(Entity, &ambition_platformer2d::combat::actor_tuning::ActorConfig, &FeatureId)>();
            let mut birds: Vec<(String, Entity)> = q
                .iter(world)
                .filter(|(_, config, _)| ambition_demo_mary_o::swallow::is_laden_swallow_brain(&config.brain))
                .map(|(entity, _, id)| (id.as_str().to_owned(), entity))
                .collect();
            birds.sort();
            (seated && !birds.is_empty()).then(|| birds.into_iter().map(|(_, entity)| entity).collect())
        })
        .expect("1-3 builds its swallows and seats Mary-O");
    if scripted {
        let admitted = app.world().resource::<ambition_platformer2d::extension::AdmittedExtensions>();
        let entry = admitted
            .0
            .entries
            .iter()
            .find(|e| e.path == "mary_o::laden_swallow/think")
            .expect("the swallow's think entry is admitted");
        assert!(
            matches!(entry.runner, ambition_platformer2d::extension::EntryRunner::Loaded { .. }),
            "the think entry runs from the script"
        );
    }
    if held {
        for bird in &birds {
            app.world_mut().entity_mut(*bird).insert(ControlHolds::only(ControlHold::Sequence));
        }
    }
    let sim = app.sim_schedule();
    app.init_resource::<Frames>()
        .add_systems(sim, record.after(PlayerInputSet::ControlGate).before(WorldPrepSet::BeforeIntegrate));
    for _ in 0..DROP_EVERY_TICKS * 2 {
        let at = app
            .world()
            .get::<ae::BodyKinematics>(birds[0])
            .expect("the same swallow lives the whole run: nothing replayed the room")
            .pos;
        {
            let world = app.world_mut();
            let mut q = world.query_filtered::<(
                &mut ae::BodyKinematics,
                &mut ambition_platformer2d::characters::actor::BodyHealth,
            ), With<PrimaryPlayer>>();
            let (mut body, mut health) = q.single_mut(world).expect("her body");
            body.pos = ae::Vec2::new(at.x, at.y + 120.0);
            body.vel = ae::Vec2::ZERO;
            health.health.invulnerable.set(ambition_platformer2d::characters::actor::Invulnerability::SCRIPTED, true);
        }
        app.update();
    }
    std::mem::take(&mut app.world_mut().resource_mut::<Frames>().0)
}

#[test]
fn the_laden_swallow_drops_on_its_modules_rule_and_not_when_held() {
    let free = two_periods(false);
    assert!(free.len() as u64 >= DROP_EVERY_TICKS * 2, "the probe saw the swallows on {} frames", free.len());
    let mut drops = 0;
    for seen in &free {
        let expected =
            (drops_on(seen.tick, &seen.placement) && over(seen.bird, seen.her)).then(|| ae::Vec2::from(DROP_DIRECTION));
        assert_eq!(
            seen.fire, expected,
            "tick {} ({}): the gated frame is the module's drop or nothing",
            seen.tick, seen.placement
        );
        drops += usize::from(expected.is_some());
    }
    assert!(drops >= 2, "the swallow over her drops once a period: {drops} drops");

    let held = two_periods(true);
    let dropped: Vec<_> = held.iter().filter(|seen| seen.fire.is_some()).collect();
    assert!(dropped.is_empty(), "the gate removes the drop of a swallow a script holds: {dropped:?}");
}

/// ⭐ THE BRAIN IN RHAI: the same rule as a script (`actor_think` in,
/// `fire` out, both described ports), loaded in place of the native code,
/// gives the same gated frame on every tick: the module's drop or nothing.
#[test]
fn the_laden_swallow_in_rhai_drops_on_the_same_rule() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ambition_demo_mary_o_modules/rhai/laden_swallow.rhai");
    let seen = two_periods_loading(false, vec![script]);
    assert!(seen.len() as u64 >= DROP_EVERY_TICKS * 2, "the probe saw the swallows on {} frames", seen.len());
    let mut drops = 0;
    for seen in &seen {
        let expected =
            (drops_on(seen.tick, &seen.placement) && over(seen.bird, seen.her)).then(|| ae::Vec2::from(DROP_DIRECTION));
        assert_eq!(seen.fire, expected, "tick {} ({}): the script's frame is the rule's", seen.tick, seen.placement);
        drops += usize::from(expected.is_some());
    }
    assert!(drops >= 2, "the scripted swallow over her drops once a period: {drops} drops");
}

/// A controller that coats its own body: the swallow's brain hook holds the
/// status `drill.coat` on its first think, and the coat counts its own ticks.
/// Both counts are records on the body.
const COAT: &str = r#"
fn declare() {
    #{
        provider: "drill",
        key: "coat",
        schemas: [
            #{ key: "coat.ticks", attachment: "body",
               fields: [
                   #{ tag: 1, name: "thinks", kind: "u32" },
                   #{ tag: 2, name: "coated", kind: "u32" },
               ] },
        ],
        entries: [
            #{ key: "think", phase: "actor_think",
               trigger: #{ port: "actor_think", selector: "mary_o_laden_swallow" },
               writes: ["coat.ticks"], requests: ["hold_status"], limit: 1 },
            #{ key: "coat", phase: "technique_execution",
               trigger: #{ port: "status_tick", selector: "drill.coat" },
               writes: ["coat.ticks"], requests: [], limit: 1 },
        ],
    }
}

fn think() {
    let ticks = this.state.ticks;
    ticks.thinks += 1;
    this.state.ticks = ticks;
    if ticks.thinks == 1 {
        this.requests.push(request("hold_status", #{ kind: "drill.coat", held: true }));
    }
}

fn coat() {
    let ticks = this.state.ticks;
    ticks.coated += 1;
    this.state.ticks = ticks;
}
"#;

/// ⭐ A BEHAVIOUR ON A BODY OUTLIVES ITS CONTROLLER: a swallow's brain hook
/// (`actor_think`, a controller) puts a status on its body. When a player
/// takes the swallow, the hook stops, as a controller yields to the body's
/// driver, and the status goes on ticking, every tick, because it is the
/// body's. When the player lets go, the hook decides again.
#[test]
fn a_status_a_brain_puts_on_its_body_outlives_a_change_of_driver() {
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d::combat::extension::status::BodyStatuses;
    use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
    use ambition_platformer2d::extension::BodyRecords;

    const TICKS: SchemaKey = SchemaKey::new("drill", "coat.ticks", 1);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../target/extension-modules/script_test_{}_coat", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("coat.rhai"), COAT).unwrap();
    let mut app = ambition_demo_mary_o_app::build_demo_app_loading(vec![dir.join("coat.rhai")]);
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(SKY.to_string()));
    let bird = (0..600)
        .find_map(|_| {
            app.update();
            let world = app.world_mut();
            let mut q = world.query::<(Entity, &ambition_platformer2d::combat::actor_tuning::ActorConfig, &FeatureId)>();
            q.iter(world)
                .filter(|(_, config, _)| ambition_demo_mary_o::swallow::is_laden_swallow_brain(&config.brain))
                .min_by(|a, b| a.2.as_str().cmp(b.2.as_str()))
                .map(|(entity, ..)| entity)
        })
        .expect("1-3 builds its swallows");
    let counts = |app: &App| -> (u32, u32, bool) {
        let world = app.world();
        let count = |tag| match world.get::<BodyRecords>(bird).and_then(|r| r.get(&TICKS)).map(|r| r.get(FieldRef(tag)).cloned()) {
            Some(Ok(Value::U32(n))) => n,
            _ => 0,
        };
        let coated = world.get::<BodyStatuses>(bird).is_some_and(|held| held.holds("drill.coat"));
        (count(0), count(1), coated)
    };
    // A hostile asleep under her rooms' dormancy rule decides nothing, module
    // brain or native (OBS3), so the player stays within the wake radius of
    // the bird while it is meant to think.
    let step = |app: &mut App| {
        let world = app.world_mut();
        let at = world.get::<ae::BodyKinematics>(bird).expect("the bird").pos;
        let mut her = world.query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>();
        for mut kin in her.iter_mut(world) {
            kin.pos = at + ae::Vec2::new(0.0, 240.0);
            kin.vel = ae::Vec2::ZERO;
        }
        app.update();
    };
    for _ in 0..30 {
        step(&mut app);
    }
    let (thinks, coated, holds) = counts(&app);
    assert!(thinks >= 20 && coated >= 20 && holds, "the brain thinks and coats its body: {thinks} thinks, {coated} coated, held {holds}");

    app.world_mut().entity_mut(bird).insert(DrivingParticipant(PlayerSlot(1)));
    step(&mut app);
    let (driven_thinks, driven_coated, _) = counts(&app);
    for _ in 0..30 {
        step(&mut app);
    }
    let (after_thinks, after_coated, still) = counts(&app);
    assert_eq!(after_thinks, driven_thinks, "a driven body's brain hook does not decide");
    assert!(still, "the coat stays on the body");
    assert_eq!(after_coated - driven_coated, 30, "the coat ticks every tick a player drives the body");

    app.world_mut().entity_mut(bird).remove::<DrivingParticipant>();
    for _ in 0..10 {
        step(&mut app);
    }
    let (back_thinks, _, _) = counts(&app);
    assert!(back_thinks > after_thinks, "the brain decides again when the player lets go");
    let _ = std::fs::remove_dir_all(&dir);
}
