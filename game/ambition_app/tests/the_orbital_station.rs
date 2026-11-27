//! The orbital station of the Science dimension (gate address GALACTIC;
//! `game/ambition_map_assets/ambition_content/generators/orbital_station/`): its decks walked with
//! the robot's own inputs, the gates its errand opens, the tram over the open
//! spoke, and the damaged spur.

use crate::common::{base, fixed_60hz_room_options};
use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};

const DOCK: &str = "orbital_station_gate_dock";
const BAY: &str = "orbital_station_docking_bay";
const VIEW: &str = "orbital_station_observation_deck";
const HAB: &str = "orbital_station_habitation";
const SERVICE: &str = "orbital_station_service_deck";
const SPUR: &str = "orbital_station_damaged_spur";
const SPOKE: &str = "orbital_station_spoke";
const TERMINUS: &str = "orbital_station_tether_terminus";
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

/// Walk the decks in both directions. Customs has opened the freight lane.
#[test]
fn the_station_is_one_loop_of_walked_decks() {
    let mut sim = room_with(DOCK, with_flags(&["orbital_consignment_cleared"]));
    for target in [HAB, SERVICE, HAB, VIEW, BAY, TERMINUS, BAY, DOCK] {
        walk(&mut sim, target);
    }
    for target in [BAY, VIEW, HAB, DOCK] {
        walk(&mut sim, target);
    }
}

/// The open hatch connects the service deck to the spur's safe end.
#[test]
fn the_service_deck_leads_to_the_spur() {
    let mut sim = room_with(SERVICE, with_flags(&["orbital_spur_pass"]));
    walk(&mut sim, SPUR);
    walk(&mut sim, SERVICE);
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
    for _ in 0..180 {
        shut.step(AgentAction { move_x: -1.0, ..base() });
    }
    let blocked = shut.observation();
    assert_eq!(blocked.active_room, SERVICE, "the closed hatch blocks the exit");
    assert!(blocked.player_pos.0 > 272.0, "the robot stops east of the hatch");
    let mut open = room_with(SERVICE, with_flags(&["orbital_spur_pass"]));
    assert!(!stands(&mut open, "service_spur_hatch"), "{:?}", standing(&mut open));
}

/// THE DAMAGED SPUR is dangerous and has an end: its crawlers and mites are
/// hostile and alive, its gaps have live cable in them, and its breaker is a
/// lever whose press is a fact of the save.
#[test]
fn the_damaged_spur_has_foes_and_a_breaker() {
    use ambition_platformer2d::characters::actor::{BodyHealth, CharacterForm};
    let mut sim = room_with(SPUR, with_flags(&["orbital_spur_pass"]));
    let foes: Vec<String> = {
        let world = sim.world_mut();
        world
            .query::<(&CharacterForm, &BodyHealth)>()
            .iter(world)
            .filter(|(_, health)| health.alive())
            .map(|(worn, _)| worn.id().to_string())
            .filter(|id| id == "npc_dividing_mite" || id == "npc_exploding_mite")
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
    sim.teleport_player((1936.0, 700.0 - half));
    for _ in 0..90 {
        sim.step(base());
    }
    assert!(sim.observation().resets > 0, "a fall into the first gap is a reset");
}

fn tram(sim: &Platformer2dSimHarness) -> ambition_platformer2d::engine_core::Aabb {
    use ambition_platformer2d::platformer::lifecycle::sole_live_room_component;
    use ambition_platformer2d::world::collision::MovingPlatformSet;

    sole_live_room_component::<MovingPlatformSet>(sim.world())
        .expect("the spoke has moving platforms")
        .0.iter()
        .find(|platform| platform.id == "spoke_tram")
        .expect("the spoke has a tram")
        .aabb()
}

fn ride_tram(sim: &mut Platformer2dSimHarness, from: f32, to: f32) {
    use ambition_platformer2d::engine_core::{AabbExt, Vec2};

    let direction = (to - from).signum();
    crate::common::walk::walk_to(
        &mut crate::common::walk::Pad::new(sim),
        Vec2::new(from - 32.0 * direction, 576.0),
    ).expect("walk to the tram stop");
    let start_resets = sim.observation().resets;
    let mut boarded = false;
    let mut got_off = false;
    for frame in 0..1200 {
        let car = tram(sim);
        let o = sim.observation();
        let near_edge = if direction > 0.0 { car.left() } else { car.right() };
        let far_edge = if direction > 0.0 { car.right() } else { car.left() };
        let mut action = base();
        if got_off {
            if (o.player_pos.0 - to) * direction > 32.0 && o.on_ground {
                assert_eq!(o.resets, start_resets, "cross the gap without falling");
                return;
            }
            action.move_x = direction;
            action.jump_held = true;
        } else if boarded {
            if (to - far_edge) * direction <= 12.0 {
                got_off = true;
                action = AgentAction { move_x: direction, jump: true, jump_held: true, ..base() };
            }
        } else if (o.player_pos.0 - from) * direction > 48.0 && o.on_ground {
            boarded = true;
        } else if (near_edge - from) * direction <= 8.0 || !o.on_ground {
            action = AgentAction {
                move_x: if (o.player_pos.0 - car.center().x).abs() > 12.0 {
                    (car.center().x - o.player_pos.0).signum()
                } else {
                    0.0
                },
                jump: o.on_ground,
                jump_held: true,
                ..base()
            };
        }
        sim.step(action);
        assert_eq!(sim.observation().resets, start_resets, "fell from the tram at frame {frame}");
    }
    panic!("tram ride {from} to {to} stopped at {:?}", sim.observation().player_pos);
}

/// Ride the tram in both directions between the orchard and the station.
#[test]
fn the_tram_joins_the_station_to_the_orchard() {
    let mut sim = room(ORCHARD);
    walk(&mut sim, SPOKE);
    ride_tram(&mut sim, 1840.0, 1200.0);
    walk(&mut sim, VIEW);
    walk(&mut sim, SPOKE);
    ride_tram(&mut sim, 1200.0, 1840.0);

    crate::common::walk::walk_to(
        &mut crate::common::walk::Pad::new(&mut sim),
        ambition_platformer2d::engine_core::Vec2::new(2800.0, 576.0),
    ).expect("walk to the orchard ladder");
    for _ in 0..180 {
        if sim.observation().player_pos.1 < 388.0 {
            break;
        }
        sim.step(AgentAction { move_y: -1.0, ..base() });
    }
    assert!(sim.observation().player_pos.1 < 388.0, "climb above the orchard walk");
    for _ in 0..240 {
        if sim.step(AgentAction { move_x: 1.0, ..base() }).active_room == ORCHARD {
            return;
        }
    }
    panic!("walk from the ladder to the orchard: {:?}", sim.observation().player_pos);
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
    assert_eq!(sim.world().resource::<ambition_content::gate_network::GateWordsResource>().0.word(&ring.zone), Some("GALACTIC"));
    let directory = ambition_content::gate_network::lowered_directory(ambition_content::pack::shipped());
    assert!(ring.terminal.is_some() && directory.posted(&ring.zone).iter().any(|address| address == "LEGAL"), "its terminal posts the way home");
}


