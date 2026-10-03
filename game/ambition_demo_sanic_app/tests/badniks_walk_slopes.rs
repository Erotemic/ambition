//! A badnik walks the ground Sanic runs on: over the speedway's hills, and back
//! from the pit lip instead of off it.
//!
//! On the axis kernel a badnik collided only with the flat solids under the
//! hills and walked through every one of them. It now rides the surface solver
//! (`slope_factor: 0`, a walker, not a ball) and its profile turns where
//! `ground_ends_ahead` says its ground runs out.

use ambition_demo_sanic::{PIT_LEFT_X, FLOOR_TOP};
use ambition_demo_sanic_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

fn boot() -> App {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..600 {
        app.update();
        let player = {
            let mut q = app.world_mut().query_filtered::<(), With<PrimaryPlayer>>();
            q.iter(app.world()).next().is_some()
        };
        if player && !badniks(&mut app).is_empty() {
            return app;
        }
    }
    panic!("the speedway never spawned Sanic and his badniks");
}

fn badniks(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    world
        .query::<(Entity, &ambition_platformer2d::combat::actor_tuning::ActorConfig)>()
        .iter(world)
        .filter(|(_, config)| {
            matches!(
                &config.brain,
                ambition_platformer2d::entity_catalog::placements::CharacterBrain::Custom(key)
                    if key == ambition_demo_sanic::badnik::BADNIK_BRAIN_KEY
            )
        })
        .map(|(entity, _)| entity)
        .collect()
}

/// Drop one badnik at `at` (through the discrete-transit authority, so its
/// surface state is not left describing the old place) and record its path.
fn walk_from(at: Vec2, frames: usize) -> Vec<Vec2> {
    let mut app = boot();
    let badnik = badniks(&mut app)[0];
    {
        let world = app.world_mut();
        let mut q = world.query::<(
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
        )>();
        let (mut clusters, mut model) = q.get_mut(world, badnik).expect("a badnik body");
        let mut clusters = clusters.as_clusters_mut();
        ae::movement::transit_body(
            &mut model,
            &mut clusters,
            at,
            ae::movement::TransitVelocity::Zero,
        );
    }
    (0..frames)
        .map(|_| {
            app.update();
            app.world()
                .get::<ae::BodyKinematics>(badnik)
                .expect("the badnik is still alive")
                .pos
        })
        .collect()
}

#[test]
fn a_badnik_walks_over_the_first_hill() {
    // Above the first hill's rising flank (x 350..900, 90 high).
    let path = walk_from(Vec2::new(560.0, 500.0), 900);
    let crest = path.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    let furthest = path.iter().map(|p| p.x).fold(f32::MIN, f32::max);
    assert!(
        crest < FLOOR_TOP - 80.0,
        "it rose with the hill (highest centre y {crest:.0}; the floor is {FLOOR_TOP})"
    );
    assert!(
        furthest > 900.0,
        "and walked on over it, not stuck on the flank (furthest x {furthest:.0})"
    );
    let dropped_into_the_hill = path
        .iter()
        .filter(|p| p.x > 600.0 && p.x < 800.0)
        .any(|p| p.y > FLOOR_TOP - 20.0);
    assert!(
        !dropped_into_the_hill,
        "it never walked the flat floor INSIDE the hill"
    );
}

#[test]
fn a_badnik_turns_back_at_the_pit_lip() {
    let path = walk_from(Vec2::new(PIT_LEFT_X - 100.0, 600.0), 400);
    let furthest = path.iter().map(|p| p.x).fold(f32::MIN, f32::max);
    let last = *path.last().unwrap();
    assert!(
        furthest > PIT_LEFT_X - 80.0,
        "it reached the lip before turning (furthest x {furthest:.0}), so the \
         turn was the ledge's and not a wall's"
    );
    assert!(
        furthest < PIT_LEFT_X && last.x < furthest - 100.0,
        "it turned at the lip and walked back (furthest {furthest:.0}, now {:.0})",
        last.x
    );
}

/// The badnik a test's marker gate is open for, if any.
#[derive(Resource, Default)]
struct MarkerPass(Option<Entity>);

/// A small gate solid far from the pit. Its name sorts before [`BRIDGE`], so
/// in the composed walls it comes just before the bridge.
const MARKER: &str = "gate:a_marker";
/// A gate solid that bridges the pit. It is never open for a badnik.
const BRIDGE: &str = "gate:b_bridge";

/// The two gate solids, put into the room's overlay every frame after the
/// overlay rebuild, with a pass through the marker for the badnik that
/// [`MarkerPass`] names.
fn gate_the_pit(
    pass: Res<MarkerPass>,
    mut overlays: Query<&mut ambition_platformer2d::world::FeatureEcsWorldOverlay>,
) {
    for mut overlay in &mut overlays {
        overlay.gate_solids.push(ae::Block::solid(
            MARKER,
            Vec2::new(PIT_LEFT_X - 1200.0, 100.0),
            Vec2::splat(16.0),
        ));
        overlay.gate_solids.push(ae::Block::solid(
            BRIDGE,
            Vec2::new(PIT_LEFT_X, FLOOR_TOP),
            Vec2::new(ambition_demo_sanic::PIT_RIGHT_X - PIT_LEFT_X, 32.0),
        ));
        if let Some(badnik) = pass.0 {
            overlay.gate_passes.push(ambition_platformer2d::world::GatePass {
                block: MARKER.to_string(),
                bodies: vec![badnik],
            });
        }
    }
}

/// The path of a badnik put down on the middle of the bridge, with the marker
/// gate open for it or not.
fn walk_the_bridge(marker_open_for_the_badnik: bool) -> Vec<Vec2> {
    use ambition_platformer2d::sim::{
        FeatureWorldOverlayContributions, Platformer2dSimulationPhaseMonolith, SimScheduleExt,
    };
    let mut app = boot();
    let badnik = badniks(&mut app)[0];
    app.insert_resource(MarkerPass(marker_open_for_the_badnik.then_some(badnik)));
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        gate_the_pit
            .in_set(FeatureWorldOverlayContributions)
            .in_set(Platformer2dSimulationPhaseMonolith::WorldPrep),
    );
    {
        let world = app.world_mut();
        let mut q = world.query::<(
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
        )>();
        let (mut clusters, mut model) = q.get_mut(world, badnik).expect("a badnik body");
        let mut clusters = clusters.as_clusters_mut();
        ae::movement::transit_body(
            &mut model,
            &mut clusters,
            Vec2::new((PIT_LEFT_X + ambition_demo_sanic::PIT_RIGHT_X) / 2.0, 600.0),
            ae::movement::TransitVelocity::Zero,
        );
    }
    (0..400)
        .map(|_| {
            app.update();
            app.world()
                .get::<ae::BodyKinematics>(badnik)
                .map(|kin| kin.pos)
                .unwrap_or(Vec2::splat(f32::NAN))
        })
        .collect()
}

/// GATE-PER-ACTOR (Q54): A BADNIK PLANS ON THE WALLS ITS OWN BODY MEETS.
///
/// The body rides the walls without the gates open for it, so the block it
/// rides is an index into those walls. Here a marker gate, open for the badnik,
/// comes just before the bridge it rides. If the brain asked the shared walls
/// whether its ground ends ahead, that index would name the marker and not the
/// bridge. A badnik with the marker open must pace the bridge exactly as one
/// with it closed.
#[test]
fn a_gate_open_for_a_badnik_does_not_change_the_ground_it_plans_on() {
    let closed = walk_the_bridge(false);
    let start = (PIT_LEFT_X + ambition_demo_sanic::PIT_RIGHT_X) / 2.0;
    let left = closed.iter().map(|p| p.x).fold(f32::MAX, f32::min);
    let right = closed.iter().map(|p| p.x).fold(f32::MIN, f32::max);
    let lowest = closed.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert!(
        left >= PIT_LEFT_X
            && right <= ambition_demo_sanic::PIT_RIGHT_X
            && left < start - 40.0
            && right > start + 40.0
            && lowest < FLOOR_TOP,
        "control: the badnik paces the bridge, turning at both ends \
         (x {left:.0}..{right:.0}, lowest centre y {lowest:.0})"
    );

    let open = walk_the_bridge(true);
    let first_difference = closed.iter().zip(&open).position(|(c, o)| c != o);
    assert_eq!(
        first_difference, None,
        "a gate open for the badnik elsewhere changed the bridge it plans on"
    );
}
