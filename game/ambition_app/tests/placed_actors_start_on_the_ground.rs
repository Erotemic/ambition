//! A placed actor starts on the ground under it (Jon, 2026-10-08).
//!
//! Each actor was built at the place its placement states, which is usually
//! some pixels above its floor or in it. So each one fell or was pushed out
//! when its room started, and the room started with a burst of landing
//! sounds.
//!
//! A placement now states where its body starts (`SpawnGrounding`). The
//! default (`Auto`) puts a body that walks on the ground at its placement.
//!
//! Measured before the rule, in the first 120 frames of `basement_enemies`:
//! 8 landing cues. Five of its eight enemies were placed 6 to 15 px in a
//! platform; two of those fell through it to the floor, 116 px.

use ambition_app::{AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};
use bevy::ecs::message::{MessageCursor, Messages};
use bevy::prelude::*;

/// The frames in which a body that starts in the air lands: a fall of 116 px
/// took 22 frames.
const FRAMES: usize = 120;

/// The bodies that landed in the first frames of `room` (the placement id of
/// each, or `player`), and the number of placed bodies the room built.
fn the_start_of(room: &str) -> (Vec<String>, usize) {
    let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options(room))
        .unwrap_or_else(|error| panic!("`{room}` boots: {error:?}"));
    let mut cursor = MessageCursor::<OwnedSfxMessage>::default();
    let mut landed = Vec::new();
    let mut placed = 0;
    for _ in 0..FRAMES {
        sim.step(crate::common::base());
        let cues: Vec<Vec2> = cursor
            .read(sim.world().resource::<Messages<OwnedSfxMessage>>())
            .filter_map(|message| match message.request {
                SfxMessage::Land { pos } => Some(pos),
                _ => None,
            })
            .collect();
        let world = sim.world_mut();
        let mut bodies = world.query::<(&BodyKinematics, Option<&FeatureId>, Has<PrimaryPlayer>)>();
        let bodies: Vec<(Vec2, String)> = bodies
            .iter(world)
            .map(|(kin, id, is_player)| {
                let feet = kin.pos + Vec2::new(0.0, kin.size.y * 0.5);
                let name = match id {
                    Some(id) => id.0.clone(),
                    None if is_player => "player".to_string(),
                    None => "a body with no placement".to_string(),
                };
                (feet, name)
            })
            .collect();
        placed = placed.max(bodies.iter().filter(|(_, name)| name.contains("Spawn-")).count());
        for cue in cues {
            // A landing cue is written at the feet of the body that landed.
            let (_, name) = bodies
                .iter()
                .min_by(|a, b| a.0.distance(cue).total_cmp(&b.0.distance(cue)))
                .expect("a landing cue has a body");
            landed.push(name.clone());
        }
    }
    landed.sort();
    (landed, placed)
}

/// The rooms of the shipped game that place the most actors.
///
/// ⚠ The bodies in `still` land on the first tick for a different reason, and
/// they are not the goal. Each one is where it starts, and its movement model
/// reports its first ground contact as a landing: a crawler (the puppy slug),
/// a momentum body (Sanic), and the player (built 1 px above its floor in
/// `basement_enemies`). A body that leaves this list is an improvement.
#[test]
fn no_placed_walker_lands_when_its_room_starts() {
    let rooms: [(&str, usize, &[&str]); 4] = [
        (
            "hall_of_characters",
            100,
            &["NpcSpawn-109803", "NpcSpawn-109893", "NpcSpawn-109894", "player"],
        ),
        ("basement_enemies", 8, &["EnemySpawn-0143", "player"]),
        ("pirate_cove", 9, &[]),
        ("intro_raid_corridor", 3, &[]),
    ];
    for (room, at_least, still) in rooms {
        let (landed, placed) = the_start_of(room);
        assert!(
            placed >= at_least,
            "premise: `{room}` builds its placed bodies ({placed}, and it places {at_least} or more)"
        );
        let new: Vec<&String> = landed.iter().filter(|name| !still.contains(&name.as_str())).collect();
        assert!(
            new.is_empty(),
            "these bodies landed in the first {FRAMES} frames of `{room}`: {new:?} (all: {landed:?})"
        );
    }
}
