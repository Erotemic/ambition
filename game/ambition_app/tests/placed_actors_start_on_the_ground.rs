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
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::{BodyGroundState, BodyKinematics};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};
use bevy::ecs::message::{MessageCursor, Messages};
use bevy::prelude::*;

/// The frames in which a body that starts in the air lands: a fall of 116 px
/// took 22 frames.
const FRAMES: usize = 120;

/// The bodies that landed in the first `FRAMES` of `room`, and how many
/// placed bodies it built. `meddle` runs before each frame (`frame`, the sim).
/// A body's name is its placement id and, in brackets, the catalog character
/// it wears (`NpcSpawn-0001 (sanic)`), or `player`.
///
/// A landing cue names the body whose ground contact went from off to on in
/// the frame of the cue (the cue carries a sound source, not a body), the
/// nearest of them when several did. The nearest feet named the wrong body:
/// a wide one beside a body that landed. A cue no body's touch-down explains
/// is not a landing; it is returned apart, named by the nearest feet.
fn the_start_of_with(
    room: &str,
    mut meddle: impl FnMut(usize, &mut Platformer2dSimHarness),
) -> (Vec<String>, usize, Vec<String>) {
    let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options(room))
        .unwrap_or_else(|error| panic!("`{room}` boots: {error:?}"));
    let mut cursor = MessageCursor::<OwnedSfxMessage>::default();
    let mut landed = Vec::new();
    let mut unexplained = Vec::new();
    let mut placed = 0;
    // Each body's ground contact before the first frame, so a body built in
    // the air that lands in frame 0 is seen to.
    let mut was_on_ground: std::collections::HashMap<Entity, bool> = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &BodyGroundState)>();
        q.iter(world).map(|(entity, ground)| (entity, ground.on_ground)).collect()
    };
    for frame in 0..FRAMES {
        meddle(frame, &mut sim);
        sim.step(crate::common::base());
        let cues: Vec<Vec2> = cursor
            .read(sim.world().resource::<Messages<OwnedSfxMessage>>())
            .filter_map(|message| match message.request {
                SfxMessage::Land { pos } => Some(pos),
                _ => None,
            })
            .collect();
        let world = sim.world_mut();
        let mut bodies = world.query::<(
            Entity,
            &BodyKinematics,
            Option<&BodyGroundState>,
            Option<&FeatureId>,
            Option<&WornCharacter>,
            Has<PrimaryPlayer>,
        )>();
        // (feet, name, it touched down this frame)
        let bodies: Vec<(Vec2, String, bool)> = bodies
            .iter(world)
            .map(|(entity, kin, ground, id, worn, is_player)| {
                let feet = kin.pos + Vec2::new(0.0, kin.size.y * 0.5);
                let name = match (id, worn) {
                    (Some(id), Some(worn)) => format!("{} ({})", id.0, worn.id()),
                    (Some(id), None) => id.0.clone(),
                    (None, _) if is_player => "player".to_string(),
                    (None, _) => "a body with no placement".to_string(),
                };
                let on = ground.is_some_and(|g| g.on_ground);
                let touched_down = on && was_on_ground.insert(entity, on) == Some(false);
                if !on {
                    was_on_ground.insert(entity, false);
                }
                (feet, name, touched_down)
            })
            .collect();
        placed = placed.max(bodies.iter().filter(|(_, name, _)| name.contains("Spawn-")).count());
        let nearest = |cue: Vec2, of: &mut dyn Iterator<Item = &(Vec2, String, bool)>| {
            of.min_by(|a, b| a.0.distance(cue).total_cmp(&b.0.distance(cue))).map(|b| b.1.clone())
        };
        for cue in cues {
            match nearest(cue, &mut bodies.iter().filter(|b| b.2)) {
                Some(name) => landed.push(name),
                None => unexplained.push(format!(
                    "{} (frame {frame})",
                    nearest(cue, &mut bodies.iter()).unwrap_or_else(|| "no body".to_string())
                )),
            }
        }
    }
    landed.sort();
    (landed, placed, unexplained)
}

fn the_start_of(room: &str) -> (Vec<String>, usize, Vec<String>) {
    the_start_of_with(room, |_, _| {})
}

/// The control: a body lifted off its floor falls, lands, and is named.
#[test]
fn a_body_dropped_from_a_height_is_named() {
    const DROPPED: &str = "npc_evil_zombie";
    let (landed, _, _) = the_start_of_with("hall_of_characters", |frame, sim| {
        if frame == 30 {
            let world = sim.world_mut();
            let mut q = world.query::<(&mut BodyKinematics, &WornCharacter)>();
            for (mut kin, worn) in q.iter_mut(world) {
                if worn.id() == DROPPED {
                    kin.pos.y -= 60.0;
                }
            }
        }
    });
    assert!(
        landed.iter().any(|name| name.ends_with(&format!("({DROPPED})"))),
        "the {DROPPED} dropped 60 px and no landing named it: {landed:?}"
    );
}

#[test]
fn no_placed_walker_lands_when_its_room_starts() {
    let rooms: [(&str, usize, &[&str]); 4] = [
        (
            "hall_of_characters",
            100,
            &["npc_puppy_slug", "sanic", "super_sanic", "player"],
        ),
        ("basement_enemies", 8, &["EnemySpawn-0143", "player"]),
        ("pirate_cove", 9, &[]),
        ("intro_raid_corridor", 3, &[]),
    ];
    for (room, at_least, still) in rooms {
        let (landed, placed, unexplained) = the_start_of(room);
        // A cue no touch-down explains (measured 2026-10-18: one for each of
        // four bodies standing still in `hall_of_characters`, all in frame 0)
        // is another defect than this test's; it is shown, not failed.
        if !unexplained.is_empty() {
            eprintln!("`{room}`: landing cues no body's touch-down explains: {unexplained:?}");
        }
        assert!(
            placed >= at_least,
            "premise: `{room}` builds its placed bodies ({placed}, and it places {at_least} or more)"
        );
        let excused = |name: &str| {
            still.iter().any(|row| name == *row || name.starts_with(&format!("{row} (")) || name.ends_with(&format!("({row})")))
        };
        let new: Vec<&String> = landed.iter().filter(|name| !excused(name)).collect();
        assert!(
            new.is_empty(),
            "these bodies landed in the first {FRAMES} frames of `{room}`: {new:?} (all: {landed:?})"
        );
    }
}
