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
