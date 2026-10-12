//! ⭐ The Laden Swallow's brain is an extension module
//! (`ambition_demo_mary_o_modules::laden_swallow`, fast-iteration I4
//! `actor_think`), so an edit to when it drops compiles that crate alone.
//!
//! The test reads each swallow's control frame between the control gate and
//! the integration, over two drop periods, and compares it with the module's
//! own rule (`drops_on`, `over`): a coconut straight down exactly on its
//! placement's tick while it is over Mary-O, and nothing else. A swallow a
//! script holds drops nothing.

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
    let mut app = ambition_demo_mary_o_app::build_demo_app();
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
