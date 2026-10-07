//! The Hall of Bosses has its own instance of each boss (Jon, 2026-10-06).
//!
//! It did not: its Mockingbird and Clockwork Warden doors led into the main
//! game's rooms (`mockingbird_arena`, `basement_boss`), so you walked through a
//! door in the hall and came out of the cove or the hub. Now those two doors
//! lead to the hall's own copies, PRACTICE rooms (`RoomMetadata::practice`)
//! whose bosses are separate placements from the main game's. (The
//! Mockingbird's copy is its shore, `hall_mockingbird_arena`, under its own
//! sky, `hall_mockingbird_sky`, where it is fought.)

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
        // The Mockingbird is fought in the sky over its shore (2026-10-06).
        ("hall_mockingbird_sky", "hall.mockingbird", "mockingbird_sky"),
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
        boss_practice_in("hall_mockingbird_sky"),
        vec![("hall.mockingbird".to_string(), true)]
    );
    assert_eq!(
        boss_practice_in("mockingbird_sky"),
        vec![("cove.mockingbird".to_string(), false)]
    );
}

// ---- The life switches ------------------------------------------------------

const WARDEN_SWITCH: &str = "hall_warden_life_switch";
const MOCKINGBIRD_SWITCH: &str = "hall_mockingbird_life_switch";

fn hall() -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options(HALL))
        .expect("the hall boots");
    for _ in 0..10 {
        sim.step(crate::common::base());
    }
    sim
}

fn boss_record(sim: &Platformer2dSimHarness, placement: &str) -> String {
    format!(
        "{:?}",
        sim.world()
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data()
            .boss(placement)
    )
}

/// What the switch `id` shows (its published view): green while ON.
fn shown(sim: &mut Platformer2dSimHarness, id: &str) -> bool {
    use ambition_platformer2d::combat::components::FeatureId;
    use ambition_platformer2d::encounter::switches::SwitchFeature;
    let world = sim.world_mut();
    let feature_id = world
        .query::<(&SwitchFeature, &FeatureId)>()
        .iter(world)
        .find(|(feature, _)| feature.activation.id == id)
        .map(|(_, feature_id)| feature_id.0.clone())
        .unwrap_or_else(|| panic!("the hall authors {id}"));
    world
        .resource::<ambition_platformer2d::sim_view::FeatureViewIndex>()
        .get(&feature_id)
        .expect("the switch publishes a view")
        .switch_on
}

/// Stand on the switch and press Interact once, the way a player does.
fn press(sim: &mut Platformer2dSimHarness, id: &str) {
    let at = {
        let world = sim.world_mut();
        world
            .query::<(
                &ambition_platformer2d::encounter::switches::SwitchFeature,
                &ambition_platformer2d::engine_core::geometry::CenteredAabb,
            )>()
            .iter(world)
            .find(|(feature, _)| feature.activation.id == id)
            .map(|(_, aabb)| (aabb.center.x, aabb.center.y))
            .unwrap_or_else(|| panic!("the hall authors {id}"))
    };
    sim.teleport_player(at);
    sim.step(crate::common::base());
    sim.step(AgentAction {
        interact: true,
        ..crate::common::base()
    });
    for _ in 0..5 {
        sim.step(crate::common::base());
    }
}

/// ⭐ A switch by a hall door shows that boss's life and sets it: green while
/// it lives; a press kills it (red), another revives it (green). The boss is
/// not loaded (it is behind the door), so a kill only marks it dead. The
/// Mockingbird's switch is the control: pressing the Warden's leaves it alone.
#[test]
fn a_hall_switch_shows_and_sets_its_bosss_life() {
    let mut sim = hall();
    assert_eq!(boss_record(&sim, "hall.clockwork_warden"), "Untouched");
    assert!(shown(&mut sim, WARDEN_SWITCH), "a living boss's switch is green");

    press(&mut sim, WARDEN_SWITCH);
    assert_eq!(boss_record(&sim, "hall.clockwork_warden"), "Cleared", "the press killed it");
    assert!(!shown(&mut sim, WARDEN_SWITCH), "a dead boss's switch is red");
    assert!(
        sim.world()
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data()
            .flag(&ambition_platformer2d::encounter::encounter_reward_looted_flag(
                "hall.clockwork_warden"
            )),
        "a switch kill pays nothing: its reward reads looted"
    );
    assert!(shown(&mut sim, MOCKINGBIRD_SWITCH), "another boss's switch is untouched");
    assert_eq!(boss_record(&sim, "hall.mockingbird"), "Untouched");

    press(&mut sim, WARDEN_SWITCH);
    assert_eq!(boss_record(&sim, "hall.clockwork_warden"), "Untouched", "the press revived it");
    assert!(shown(&mut sim, WARDEN_SWITCH), "green again");
}

/// The switch stores nothing: a boss that dies in a FIGHT (its record goes
/// `Cleared`, no press) turns its switch red.
#[test]
fn a_boss_killed_in_a_fight_turns_its_switch_red() {
    let mut sim = hall();
    assert!(shown(&mut sim, MOCKINGBIRD_SWITCH));
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_boss(
            "hall.mockingbird",
            ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared,
        );
    sim.step(crate::common::base());
    assert!(!shown(&mut sim, MOCKINGBIRD_SWITCH), "its switch shows the boss's death");
}

fn queue_life_press(sim: &mut Platformer2dSimHarness, target: &str) {
    use ambition_platformer2d::encounter::switches::SwitchActivationQueue;
    sim.world_mut()
        .resource_mut::<SwitchActivationQueue>()
        .0
        .push(
            ambition_platformer2d::encounter::SwitchActivation {
                id: MOCKINGBIRD_SWITCH.into(),
                action: "BossLife".into(),
                target_encounter: target.into(),
            }
            .into(),
        );
}

fn mockingbird_state(sim: &mut Platformer2dSimHarness) -> (i32, String) {
    let world = sim.world_mut();
    let mut q = world.query::<(
        &BossConfig,
        &ambition_platformer2d::boss_encounter::BossEncounter,
        &ambition_platformer2d::characters::actor::BodyHealth,
    )>();
    q.iter(world)
        .find(|(config, _, _)| config.id == "hall.mockingbird")
        .map(|(_, status, health)| (health.health.current, format!("{:?}", status.encounter_phase())))
        .expect("the hall's Mockingbird is in its sky")
}

/// "If the boss is loaded in the simulation and the boss-alive switch goes
/// red, it brings the boss health to zero and kills it immediately." The
/// press comes from the hall while the boss fights in its arena (another
/// player's press), so it is queued as the switch would queue it.
#[test]
fn a_life_switch_kills_a_loaded_boss_at_once_and_revives_it() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("hall_mockingbird_sky"),
    )
    .expect("the hall's Mockingbird sky boots");
    // Until it fights: a living, woken boss is the precondition.
    let mut woke = false;
    for _ in 0..400 {
        sim.step(crate::common::base());
        let (hp, phase) = mockingbird_state(&mut sim);
        if hp > 0 && phase.starts_with("Phase") {
            woke = true;
            break;
        }
    }
    assert!(woke, "precondition: the Mockingbird wakes and fights: {:?}", mockingbird_state(&mut sim));

    queue_life_press(&mut sim, "hall.mockingbird");
    sim.step(crate::common::base());
    let (hp, phase) = mockingbird_state(&mut sim);
    assert_eq!(hp, 0, "killed at once, not after a fight");
    assert_eq!(phase, "Death");
    assert_eq!(boss_record(&sim, "hall.mockingbird"), "Cleared");

    // The boss died where it fell. Move it well away from its spawn first, so
    // that "back at its spawn" can only be the revive's doing.
    let spawn = {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &mut ambition_platformer2d::engine_core::BodyKinematics)>();
        let (config, mut kin) = q
            .iter_mut(world)
            .find(|(config, _)| config.id == "hall.mockingbird")
            .expect("the hall's Mockingbird is in its sky");
        kin.pos = config.spawn + ambition_platformer2d::engine_core::Vec2::new(300.0, 200.0);
        config.spawn
    };
    queue_life_press(&mut sim, "hall.mockingbird");
    sim.step(crate::common::base());
    assert_eq!(boss_record(&sim, "hall.mockingbird"), "Untouched", "revived in the save");
    let at = {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &ambition_platformer2d::engine_core::BodyKinematics)>();
        q.iter(world)
            .find(|(config, _)| config.id == "hall.mockingbird")
            .map(|(_, kin)| kin.pos)
            .expect("the hall's Mockingbird is in its sky")
    };
    assert!(
        at.distance(spawn) < 40.0,
        "the revive puts the boss back at its spawn {spawn:?}, not where it fell: {at:?}"
    );
    let mut revived = false;
    for _ in 0..400 {
        sim.step(crate::common::base());
        let (hp, phase) = mockingbird_state(&mut sim);
        if hp > 0 && phase.starts_with("Phase") {
            revived = true;
            break;
        }
    }
    assert!(revived, "it fights again: {:?}", mockingbird_state(&mut sim));
}
