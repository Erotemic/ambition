//! The Hall of Bosses has its own instance of each boss (Jon, 2026-10-06).
//!
//! It did not: its Mockingbird and Clockwork Warden doors led into the main
//! game's rooms (`mockingbird_arena`, `basement_boss`), so you walked through a
//! door in the hall and came out of the cove or the hub. Now those two doors
//! lead to the hall's own copies, PRACTICE rooms (`RoomMetadata::practice`)
//! whose bosses are separate placements from the main game's.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::boss_encounter::BossConfig;
use std::collections::BTreeSet;

const HALL: &str = "hall_of_bosses";

fn shipped_rooms() -> ambition_platformer2d::world::rooms::RoomSet {
    let world_manifest = ambition_content::worlds::world_manifest();
    let project = ambition_platformer2d::ldtk_map::LdtkProject::load_default_for_dev(&world_manifest)
        .expect("the shipped LDtk project loads");
    project
        .to_room_set(&world_manifest, &ambition_app::composed_ldtk_vocabulary())
        .expect("it lowers to rooms")
}

/// ⭐ Every door out of the hall leads to a room whose way out leads back to
/// the hall: walking through a hall door never puts you somewhere else.
#[test]
fn every_hall_door_leads_to_a_room_that_returns_to_the_hall() {
    let rooms = shipped_rooms();
    let links = rooms.canonical_links();
    let into_hall: BTreeSet<&str> = links
        .iter()
        .filter(|link| link.to_room == HALL)
        .map(|link| link.from_room.as_str())
        .collect();
    let doors: Vec<_> = links.iter().filter(|link| link.from_room == HALL).collect();
    assert!(doors.len() >= 9, "precondition: the hall's doors are read ({})", doors.len());
    let one_way: Vec<String> = doors
        .iter()
        .filter(|door| !into_hall.contains(door.to_room.as_str()))
        .map(|door| format!("{} -> {}", door.from_zone, door.to_room))
        .collect();
    assert!(
        one_way.is_empty(),
        "these hall doors lead into a room with no way back to the hall, so the player \
         comes out somewhere else: {one_way:?}"
    );
}

/// The hall's Mockingbird and Warden are practice copies at their own
/// placements; the main game's are neither.
#[test]
fn the_halls_mockingbird_and_warden_are_practice_copies_at_their_own_placements() {
    let rooms = shipped_rooms();
    let room = |id: &str| {
        rooms
            .rooms
            .iter()
            .find(|room| room.id == id)
            .unwrap_or_else(|| panic!("room {id} is authored"))
    };
    for (hall_room, placement, game_room) in [
        ("hall_mockingbird_arena", "hall.mockingbird", "mockingbird_arena"),
        ("hall_warden_arena", "hall.clockwork_warden", "basement_boss"),
    ] {
        let hall = room(hall_room);
        let game = room(game_room);
        assert!(hall.metadata.practice, "{hall_room} is a practice room");
        assert!(!game.metadata.practice, "{game_room} is the story's room");
        let hall_ids: Vec<&str> = hall.boss_spawns.iter().map(|boss| boss.id.as_str()).collect();
        assert_eq!(hall_ids, vec![placement], "{hall_room}'s boss is its own placement");
        assert!(
            game.boss_spawns.iter().all(|boss| boss.id != placement),
            "{game_room}'s boss is another placement"
        );
    }
}

fn boss_practice_in(room: &str) -> Vec<(String, bool)> {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(room),
    )
    .expect("the sim builds in the room");
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }
    let world = sim.world_mut();
    let mut q = world.query::<&BossConfig>();
    q.iter(world).map(|config| (config.id.clone(), config.practice)).collect()
}

/// In the running game the hall's copy is spawned as a practice boss, and
/// the main game's Mockingbird is not (the control: the same archetype, the
/// same arena geometry, another room).
#[test]
fn the_halls_mockingbird_spawns_as_a_practice_boss() {
    assert_eq!(
        boss_practice_in("hall_mockingbird_arena"),
        vec![("hall.mockingbird".to_string(), true)]
    );
    assert_eq!(
        boss_practice_in("mockingbird_arena"),
        vec![("cove.mockingbird".to_string(), false)]
    );
}
