//! Each kind of badnik the speedway places is a body in the game.
//!
//! A badnik is a row of the cast and a placement of the level. A row that
//! names a sheet the game does not have, or a placement that names no row,
//! gives no error at the place a person would look: the act has one enemy
//! fewer, or an enemy of the wrong size. So each kind is asked for here: that
//! it is there, that it has the height its row states, that it stands on the
//! floor, and that it walks.

use std::collections::BTreeMap;

use ambition_demo_sanic::speedway_oracle::FLOOR_TOP;
use ambition_demo_sanic_app::build_demo_app;
use ambition_platformer2d::actor::ActorFaction;
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

/// The kinds the speedway places (`BADNIK_PLACES` in its authoring script):
/// the row, and the height the row states.
const KINDS: [(&str, f32); 4] = [
    ("sanic_badnik", 28.8),
    ("sanic_badnik_liar", 44.0),
    ("sanic_badnik_noodle", 34.0),
    ("sanic_badnik_saint", 52.0),
];

/// Each enemy by the key of its placement: the body, where its feet are, and
/// its size.
fn enemies(app: &mut App) -> BTreeMap<String, (Entity, Vec2, Vec2)> {
    let world = app.world_mut();
    world
        .query::<(Entity, &ActorConfig, &ActorFaction, &ae::BodyKinematics)>()
        .iter(world)
        .filter(|(_, _, faction, _)| matches!(faction, ActorFaction::Enemy))
        .filter_map(|(entity, config, _, body)| match &config.brain {
            CharacterBrain::Custom(key) => {
                let aabb = body.aabb();
                let size = Vec2::new(aabb.max.x - aabb.min.x, aabb.max.y - aabb.min.y);
                Some((
                    key.clone(),
                    (entity, Vec2::new(body.pos.x, aabb.max.y), size),
                ))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn each_kind_of_badnik_of_the_speedway_stands_at_its_height_and_walks() {
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
        if player && enemies(&mut app).len() >= KINDS.len() {
            break;
        }
    }
    // They are dropped a little over the floor: let each land.
    for _ in 0..90 {
        app.update();
    }
    let placed = enemies(&mut app);
    for (kind, height) in KINDS {
        let (_, feet, size) = *placed.get(kind).unwrap_or_else(|| {
            panic!(
                "the speedway has no `{kind}`: it has {:?}",
                placed.keys().collect::<Vec<_>>()
            )
        });
        assert!(
            (size.y - height).abs() < 0.5,
            "`{kind}` is {:.1} high; its row states {height}",
            size.y
        );
        assert!(
            (feet.y - FLOOR_TOP).abs() < 2.0,
            "`{kind}` stands at y {:.1}; the floor is {FLOOR_TOP}",
            feet.y
        );
    }

    // A badnik far from Sanic does not think (`BADNIK_WAKE_RADIUS`), and two
    // of these are placed that far. Each is put on the flat west of the loop,
    // near him, to walk.
    for (k, (kind, _)) in KINDS.iter().enumerate() {
        let world = app.world_mut();
        let mut q = world.query::<(
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
        )>();
        let (mut clusters, mut model) = q.get_mut(world, placed[*kind].0).expect("a badnik body");
        ae::movement::transit_body(
            &mut model,
            &mut clusters.as_clusters_mut(),
            ae::Vec2::new(1480.0 + 70.0 * k as f32, FLOOR_TOP - 60.0),
            ae::movement::TransitVelocity::Zero,
        );
    }
    for _ in 0..60 {
        app.update();
    }
    let before = enemies(&mut app);
    for _ in 0..120 {
        app.update();
    }
    let after = enemies(&mut app);
    for (kind, _) in KINDS {
        let walked = (after[kind].1.x - before[kind].1.x).abs();
        assert!(walked > 20.0, "`{kind}` walked {walked:.1} in two seconds");
    }
}
