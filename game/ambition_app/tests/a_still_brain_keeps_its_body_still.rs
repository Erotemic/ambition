#![cfg(feature = "rl_sim")]
//! Q85 (2026-10-04): every Hall actor stays where it is placed.
//!
//! The generator writes `brain_override: "stand_still"` on every Hall spawn,
//! and a body that moves in spite of that is a defect in how its motion model
//! obeys its driver. The crawler is the case with teeth: its pace is a policy
//! fact, so only the driver's command can keep it still.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core::{BodyKinematics, Vec2};
use bevy::prelude::*;

/// Every worn body in the room that no seat drives, by entity, with the
/// character it wears and where it is.
fn actors(sim: &mut ambition_app::Platformer2dSimHarness) -> Vec<(Entity, String, Vec2)> {
    let world = sim.world_mut();
    let mut query = world.query_filtered::<
        (Entity, &WornCharacter, &BodyKinematics),
        Without<ambition_platformer2d::characters::control::DrivingParticipant>,
    >();
    query
        .iter(world)
        .map(|(entity, worn, kin)| (entity, worn.id().to_string(), kin.pos))
        .collect()
}

/// ⭐ THE HALL STANDS STILL: every spawned actor is where it settled, three
/// seconds later. Not only the slug: the rule is one population policy.
#[test]
fn every_hall_actor_stays_where_it_settled() {
    let mut sim = fixed_60hz_room_sim("hall_of_characters");
    // Settle: a body placed above its floor lands first.
    sim.step_n(base(), 120);
    let settled = actors(&mut sim);
    assert!(
        settled.iter().any(|(_, id, _)| id == "npc_puppy_slug"),
        "premise: the Hall has no Puppy Slug, the body this test was written for"
    );
    sim.step_n(base(), 180);
    let now: std::collections::HashMap<Entity, Vec2> =
        actors(&mut sim).into_iter().map(|(entity, _, pos)| (entity, pos)).collect();
    let moved: Vec<(String, f32)> = settled
        .iter()
        .filter_map(|(entity, id, at)| {
            let distance = (now.get(entity)? - *at).length();
            (distance > 1.0).then(|| (id.clone(), distance))
        })
        .collect();
    assert!(
        moved.is_empty(),
        "{} of {} Hall actors moved under `stand_still` in three seconds: {moved:?}",
        moved.len(),
        settled.len()
    );
}

/// The control: a Puppy Slug whose brain patrols still crawls. Its room is
/// `vertical_shaft`, which places six with the authored `puppy_slug` brain.
#[test]
fn a_puppy_slug_whose_brain_moves_it_still_crawls() {
    let mut sim = fixed_60hz_room_sim("vertical_shaft");
    sim.step_n(base(), 60);
    let slugs: Vec<(Entity, Vec2)> = actors(&mut sim)
        .into_iter()
        .filter(|(_, id, _)| id == "npc_puppy_slug")
        .map(|(entity, _, pos)| (entity, pos))
        .collect();
    assert!(!slugs.is_empty(), "premise: vertical_shaft places no Puppy Slug");
    sim.step_n(base(), 180);
    let now: std::collections::HashMap<Entity, Vec2> =
        actors(&mut sim).into_iter().map(|(entity, _, pos)| (entity, pos)).collect();
    let crawled: Vec<f32> = slugs
        .iter()
        .filter_map(|(entity, at)| Some((now.get(entity)? - *at).length()))
        .collect();
    assert!(
        crawled.iter().all(|distance| *distance > 20.0),
        "a patrolling Puppy Slug stopped crawling: moved {crawled:?} px in three seconds"
    );
}
