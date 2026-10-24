//! The orbital station of the Science dimension (gate address GALACTIC;
//! `tools/ambition_ldtk_tools/specs/orbital_station/`): its decks walked with
//! the robot's own inputs, the gates its errand opens, the tram over the open
//! spoke, and the damaged spur.

use crate::common::{base, fixed_60hz_room_options, walk_through_the_door_to};
use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};

const DOCK: &str = "orbital_station_gate_dock";
const BAY: &str = "orbital_station_docking_bay";
const VIEW: &str = "orbital_station_observation_deck";
const HAB: &str = "orbital_station_habitation";
const SERVICE: &str = "orbital_station_service_deck";
const SPUR: &str = "orbital_station_damaged_spur";
const SPOKE: &str = "orbital_station_spoke";
const ORCHARD: &str = "gnu_ton_orchard";

fn room_with(id: &str, save: ambition_platformer2d::session::AmbitionGameSaveData) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(id).with_save(save))
        .unwrap_or_else(|e| panic!("`{id}` builds: {e}"));
    for _ in 0..30 {
        sim.step(base());
    }
    sim
}

fn room(id: &str) -> Platformer2dSimHarness {
    room_with(id, Default::default())
}

fn with_flags(flags: &[&str]) -> ambition_platformer2d::session::AmbitionGameSaveData {
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    for flag in flags {
        save.set_flag((*flag).to_string(), true);
    }
    save
}

/// The gate solids that stand in the live room, by name.
fn standing(sim: &mut Platformer2dSimHarness) -> Vec<String> {
    ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<ambition_platformer2d::world::FeatureEcsWorldOverlay>(
        sim.world_mut(),
    )
    .expect("the live room has a collision overlay")
    .gate_solids
    .iter()
    .map(|solid| solid.name.clone())
    .collect()
}

fn stands(sim: &mut Platformer2dSimHarness, gate: &str) -> bool {
    standing(sim).iter().any(|name| name.contains(gate))
}

/// Walk from the live room to `target` with the robot's own inputs.
fn walk(sim: &mut Platformer2dSimHarness, target: &str) {
    let from = sim.observation().active_room.clone();
    if let Err(why) = crate::common::walk::walk_through(&mut crate::common::walk::Pad::new(sim), target) {
        panic!("the robot did not walk from `{from}` to `{target}`: {why}");
    }
    for _ in 0..20 {
        sim.step(base());
    }
    assert_eq!(sim.observation().active_room, target, "the walk from `{from}` ended elsewhere");
}

/// ⭐ THE STATION IS WALKED. From the gate deck the robot walks up to the
/// habitation ring, along to the observation deck, down to the docking bay,
/// by the cargo lift to the service deck and back to the habitation ring:
/// a loop, by doors and with its own inputs. The way home is the gate deck.
#[test]
fn the_station_is_one_loop_of_walked_doors() {
    let mut sim = room(DOCK);
    for target in [HAB, VIEW, BAY, SERVICE, HAB, DOCK] {
        walk(&mut sim, target);
    }
}

/// The spoke leaves the service deck for the gravity works, and the spur's
/// door is behind its hatch. Each door has its way back.
#[test]
fn the_service_deck_leads_to_the_spoke_and_the_spur() {
    let mut sim = room_with(SERVICE, with_flags(&["orbital_spur_pass"]));
    for (to, back) in [(SPOKE, SERVICE), (SPUR, SERVICE)] {
        assert_eq!(walk_through_the_door_to(&mut sim, to), to);
        assert_eq!(walk_through_the_door_to(&mut sim, back), back, "`{to}` has its door back");
    }
    assert_eq!(walk_through_the_door_to(&mut sim, SPOKE), SPOKE);
    assert_eq!(walk_through_the_door_to(&mut sim, ORCHARD), ORCHARD, "the spoke ends at the gravity works");
}

/// THE FREIGHT LANE. The quarantine pen's gates (one on the gate deck, one
/// in the bay) stand until customs clears consignment 0447; cleared, the
/// lane is a short way from the gate deck to the bay.
#[test]
fn the_freight_lane_opens_when_customs_clears_the_crate() {
    let mut held = room(DOCK);
    assert!(stands(&mut held, "dock_quarantine_gate"), "{:?}", standing(&mut held));
    let mut held_bay = room(BAY);
    assert!(stands(&mut held_bay, "bay_quarantine_gate"), "{:?}", standing(&mut held_bay));

    let mut cleared = room_with(DOCK, with_flags(&["orbital_consignment_cleared"]));
    assert!(!stands(&mut cleared, "dock_quarantine_gate"), "{:?}", standing(&mut cleared));
    walk(&mut cleared, BAY);
    assert!(!stands(&mut cleared, "bay_quarantine_gate"), "{:?}", standing(&mut cleared));
    walk(&mut cleared, DOCK);
}

/// THE SPUR'S HATCH stands until customs gives the pass.
#[test]
fn the_spur_hatch_opens_with_the_pass() {
    let mut shut = room(SERVICE);
    assert!(stands(&mut shut, "service_spur_hatch"), "{:?}", standing(&mut shut));
    let mut open = room_with(SERVICE, with_flags(&["orbital_spur_pass"]));
    assert!(!stands(&mut open, "service_spur_hatch"), "{:?}", standing(&mut open));
}

/// THE DAMAGED SPUR is dangerous and has an end: its crawlers and mites are
/// hostile and alive, its gaps have live cable in them, and its breaker is a
/// lever whose press is a fact of the save.
#[test]
fn the_damaged_spur_has_foes_and_a_breaker() {
    use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
    let mut sim = room_with(SPUR, with_flags(&["orbital_spur_pass"]));
    let foes: Vec<String> = {
        let world = sim.world_mut();
        world
            .query::<(&WornCharacter, &BodyHealth)>()
            .iter(world)
            .filter(|(_, health)| health.alive())
            .map(|(worn, _)| worn.id().to_string())
            .filter(|id| id == "npc_searchlight_crawler" || id == "npc_exploding_mite")
            .collect()
    };
    assert_eq!(foes.len(), 4, "two crawlers and two mites in the spur: {foes:?}");
    let breaker = {
        let world = sim.world_mut();
        world
            .query::<&ambition_platformer2d::encounter::switches::SwitchFeature>()
            .iter(world)
            .any(|feature| feature.activation.id == "orbital_spur_breaker")
    };
    assert!(breaker, "the spur's breaker is a lever");
    // A fall into a gap is a fall onto live cable: the spur starts again.
    let half = sim.observation().player_size.1 * 0.5;
    sim.teleport_player((624.0, 700.0 - half));
    for _ in 0..90 {
        sim.step(base());
    }
    assert!(sim.observation().resets > 0, "a fall into the first gap is a reset");
}

/// THE TRAM carries a robot that stands on it over the open part of the
/// spoke, and the robot steps off on the works' deck.
#[test]
fn the_tram_carries_the_robot_over_the_open_spoke() {
    let mut sim = room(SPOKE);
    // The car starts at the station's end and runs east: drop onto it.
    let half = sim.observation().player_size.1 * 0.5;
    sim.teleport_player((1340.0, 576.0 - 80.0 - half));
    let mut rode = false;
    for _ in 0..900 {
        let o = sim.observation();
        if o.player_pos.0 > 1900.0 && o.on_ground {
            break;
        }
        rode |= o.on_ground && (1200.0..1840.0).contains(&o.player_pos.0);
        // Ride until the car is at the works' end, then step off.
        let action = if o.player_pos.0 > 1670.0 { AgentAction { move_x: 1.0, ..base() } } else { base() };
        sim.step(action);
    }
    let o = sim.observation();
    assert!(rode && o.player_pos.0 > 1900.0 && o.on_ground, "rode {rode}, at {:?}", o.player_pos);
    assert_eq!(o.resets, 0, "the robot did not fall into the gap");
}

/// The arrival of the gate network's address GALACTIC is on the gate deck:
/// it is the zone of the dock's ring, whose terminal stands beside it. The
/// way home through it is dialed (`the_gate_network`).
#[test]
fn the_gate_deck_receives_the_arrival_and_goes_home() {
    let mut sim = room(DOCK);
    let arrival = {
        let world = sim.world_mut();
        let stamp = world
            .query_filtered::<&ambition_platformer2d::platformer::lifecycle::InRoomInstance, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .ok()
            .map(|stamp| stamp.0);
        let definition = ambition_platformer2d::world::rooms::live_room_definition_in(world, stamp).expect("a live room");
        let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let rooms = rooms.iter(world).next().expect("a room set");
        rooms.spec(definition).loading_zones.iter().any(|zone| zone.id == "orbital_station_gate_arrival")
    };
    assert!(arrival, "the gate deck has the arrival `orbital_station_gate_arrival`");
    let catalog = sim.world().resource::<ambition_platformer2d::world::rooms::GateNetworkCatalog>();
    let ring = catalog.ring("orbital_station_gate_arrival").expect("the arrival is a ring of the gate network");
    assert_eq!(ring.address.as_deref(), Some("GALACTIC"));
    assert!(ring.terminal.is_some() && ring.posted.iter().any(|address| address == "LEGAL"), "its terminal posts the way home");
}


