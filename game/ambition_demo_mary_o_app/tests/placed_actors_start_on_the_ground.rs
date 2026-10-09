//! A placed actor starts on the ground under it.
//!
//! Jon, 2026-10-08: the level started with a burst of landing sounds, because
//! each actor was placed above its floor and fell to it.
//!
//! Measured before the rule (`SpawnGrounding`): the 17 enemies of 1-1 were
//! placed 4 to 36 px above their floor, and each one landed in the first
//! second.

use bevy::ecs::message::{MessageCursor, Messages};
use bevy::prelude::*;

use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};

/// The frames in which a body that starts in the air lands: a fall of 36 px
/// takes less than half a second.
const FRAMES: usize = 120;

/// The landing cues of each frame, and how far each placed enemy fell from
/// the place it was first seen.
fn the_first_frames() -> (usize, Vec<(String, f32)>) {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    let mut cursor = MessageCursor::<OwnedSfxMessage>::default();
    let mut landings = 0;
    let mut feet = std::collections::BTreeMap::<String, (f32, f32)>::new();
    for _ in 0..FRAMES {
        app.update();
        let messages = app.world().resource::<Messages<OwnedSfxMessage>>();
        landings += cursor
            .read(messages)
            .filter(|message| matches!(message.request, SfxMessage::Land { .. }))
            .count();
        let mut bodies = app
            .world_mut()
            .query_filtered::<(&FeatureId, &ae::BodyKinematics), Without<PrimaryPlayer>>();
        for (id, kin) in bodies.iter(app.world()) {
            let now = kin.pos.y + kin.size.y * 0.5;
            let entry = feet.entry(id.0.clone()).or_insert((now, now));
            entry.1 = entry.1.max(now);
        }
    }
    let fell = feet.into_iter().map(|(id, (first, lowest))| (id, lowest - first)).collect();
    (landings, fell)
}

#[test]
fn no_enemy_of_1_1_falls_when_the_level_starts() {
    let authored = ambition_demo_mary_o::level_1_1().enemy_spawns.len();
    let (_, fell) = the_first_frames();
    assert!(
        fell.len() >= authored && authored > 0,
        "premise: the {authored} placed enemies of 1-1 are built ({} bodies seen)",
        fell.len()
    );
    let fallers: Vec<_> = fell.iter().filter(|(_, drop)| *drop > 0.5).collect();
    assert!(
        fallers.is_empty(),
        "{} of {} placed bodies fell when the level started: {fallers:?}",
        fallers.len(),
        fell.len()
    );
}

/// Each enemy of 1-1 stands with its feet on a surface from its first frame,
/// at the size its character authors (Jon, 2026-10-08: the Solid Snake and the
/// AI Slop are two times as large, "make sure feet are on the ground").
#[test]
fn each_enemy_of_1_1_starts_with_its_feet_on_a_surface() {
    use ae::AabbExt;

    let room = ambition_demo_mary_o::level_1_1();
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    // The frames in which the session is published and its room is built.
    let mut bodies = Vec::new();
    for _ in 0..10 {
        app.update();
        let mut query = app
            .world_mut()
            .query_filtered::<(&FeatureId, &ae::BodyKinematics), Without<PrimaryPlayer>>();
        bodies = query.iter(app.world()).map(|(id, kin)| (id.0.clone(), kin.aabb())).collect();
        if !bodies.is_empty() {
            break;
        }
    }
    assert!(
        bodies.len() >= room.enemy_spawns.len() && !bodies.is_empty(),
        "premise: the placed enemies of 1-1 are built ({} bodies)",
        bodies.len()
    );
    let in_the_air: Vec<_> = bodies
        .iter()
        .filter(|(_, body)| {
            !room.world.blocks.iter().any(|block| {
                ae::collision_semantics::is_support_surface(block.kind)
                    && block.aabb.left() < body.right()
                    && block.aabb.right() > body.left()
                    && (block.aabb.top() - body.bottom()).abs() < 0.5
            })
        })
        .collect();
    assert!(in_the_air.is_empty(), "these bodies have no surface under their feet: {in_the_air:?}");
    let in_a_wall: Vec<_> = bodies
        .iter()
        .filter(|(_, body)| {
            room.world.blocks.iter().any(|block| {
                ae::collision_semantics::is_full_collision_surface(block.kind)
                    && body.strict_intersects(block.aabb)
            })
        })
        .collect();
    assert!(in_a_wall.is_empty(), "these bodies start in a solid block: {in_a_wall:?}");
}

#[test]
fn the_level_starts_with_no_landing_sound() {
    let (landings, _) = the_first_frames();
    assert_eq!(landings, 0, "landing cues in the first {FRAMES} frames of 1-1");
}
