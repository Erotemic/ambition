//! The gate network, played: a traveller walks to a terminal, presses it,
//! dials an address, walks into the ring when it is open, and comes out of
//! the far ring through the ordinary room transition
//! (`docs/planning/game/gate-network.md`).
//!
//! A terminal's requests are host intents, as the terminal panel writes them
//! (`write_host_intent`); everything else is the player's own input on the
//! shipped simulation. No step here puts the body in another room.

use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt as _;
use ambition_platformer2d::world::rooms::{
    gate_address_known_flag, GateNetworkState, GatePortalPhases, GateTerminalAction, GateTerminalIntent,
};

use crate::common::walk::{walk_to, Pad};
use crate::common::{a_save_that_has_seen_the_hub_intro, base, finish_passages, fixed_60hz_room_options};

const HUB: &str = "central_hub_complex";
const HUB_RING: &str = "hub_public_gate_zone";
const HUB_TERMINAL: &str = "hub_public_terminal";
const MATH: &str = "math_gate_hall";
const MATH_RING: &str = "math_gate_return_zone";
const MATH_TERMINAL: &str = "math_gate_terminal";
const RING7: &str = "math_gate_home_zone";
const RING7_TERMINAL: &str = "ring7_terminal";
const RING7_BREAKER: &str = "math_gate_switch";

fn room_with(id: &str, save: ambition_platformer2d::session::AmbitionGameSaveData) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(id).with_save(save))
        .unwrap_or_else(|e| panic!("`{id}` builds: {e}"));
    for _ in 0..30 {
        sim.step(base());
    }
    sim
}

fn room(id: &str) -> Platformer2dSimHarness {
    room_with(id, a_save_that_has_seen_the_hub_intro())
}

fn network(sim: &mut Platformer2dSimHarness) -> GateNetworkState {
    sim.world().resource::<GateNetworkState>().clone()
}

fn save(sim: &mut Platformer2dSimHarness) -> ambition_platformer2d::session::AmbitionGameSaveData {
    sim.world()
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data()
        .clone()
}

/// The box of the zone `id` in the live room.
fn zone(sim: &mut Platformer2dSimHarness, id: &str) -> ae::Aabb {
    let world = sim.world_mut();
    let definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world).expect("one live room");
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(world)
    .expect("the session keeps its room set");
    rooms
        .spec(definition)
        .loading_zones
        .iter()
        .find(|zone| zone.id == id)
        .map(|zone| zone.aabb)
        .unwrap_or_else(|| panic!("the live room has no zone `{id}`"))
}

/// The centre of the switch `id` in the live room.
fn switch_at(sim: &mut Platformer2dSimHarness, id: &str) -> ae::Vec2 {
    let world = sim.world_mut();
    world
        .query::<(
            &ambition_platformer2d::encounter::switches::SwitchFeature,
            &ambition_platformer2d::engine_core::geometry::CenteredAabb,
        )>()
        .iter(world)
        .find(|(feature, _)| feature.activation.id == id)
        .map(|(_, aabb)| aabb.center)
        .unwrap_or_else(|| panic!("the live room has no switch `{id}`"))
}

fn room_id(sim: &mut Platformer2dSimHarness) -> String {
    sim.observation().active_room.clone()
}

/// Walk to the switch `id` with the player's own input and press Interact.
fn press_switch(sim: &mut Platformer2dSimHarness, id: &str) {
    let at = switch_at(sim, id);
    walk_to(&mut Pad::new(sim), at).unwrap_or_else(|e| panic!("to the switch `{id}`: {e}"));
    sim.step(AgentAction { interact: true, interact_held: true, ..base() });
    for _ in 0..4 {
        sim.step(base());
    }
}

/// Walk to the terminal `id` and open it: the network names its ring open.
/// A press that a blow takes (the wildlife at the primeval trailhead bites)
/// is pressed again, as a player does.
fn open_terminal(sim: &mut Platformer2dSimHarness, id: &str, ring: &str) {
    for _ in 0..6 {
        press_switch(sim, id);
        if network(sim).terminal.as_deref() == Some(ring) {
            return;
        }
        for _ in 0..20 {
            sim.step(base());
        }
    }
    panic!("the press at `{id}` never opened its ring's terminal");
}

/// Send one request to the open terminal of `ring`, as its panel does, and let
/// the simulation take it.
fn ask(sim: &mut Platformer2dSimHarness, ring: &str, action: GateTerminalAction) {
    ambition_platformer2d::actors::session::host_intents::write_host_intent(
        sim.world_mut(),
        GateTerminalIntent { ring: ring.to_string(), action },
    );
    for _ in 0..3 {
        sim.step(base());
    }
}

/// Dial `address` at the open terminal of `ring`; the answer's kind
/// (`"link"` or the refusal's kind).
fn dial(sim: &mut Platformer2dSimHarness, ring: &str, address: &str) -> String {
    ask(sim, ring, GateTerminalAction::Dial(address.to_string()));
    let report = network(sim).reports.get(ring).cloned().expect("the dial is answered");
    match report.outcome {
        Ok(_) => "link".to_string(),
        Err(refusal) => refusal.kind().to_string(),
    }
}

fn portal_on(sim: &mut Platformer2dSimHarness, ring: &str) -> bool {
    sim.world().resource::<GatePortalPhases>().allows_traversal(ring)
}

/// Walk into the ring `ring` and press Interact until the room changes.
/// Returns the room the traveller is in after the passage.
fn go_through(sim: &mut Platformer2dSimHarness, ring: &str) -> String {
    let from = room_id(sim);
    let opening = zone(sim, ring);
    // A ring on a plinth (the Math gate's cradle) stands on a walk line the
    // surface graph does not hold: a traveller already in the ring stays.
    // Straight at the ring first: a ring on a plinth (the Math gate's cradle)
    // stands on a walk line the surface graph does not hold, and its stairs
    // are walked by holding the stick.
    for _ in 0..600 {
        if standing_in(sim, ring) {
            break;
        }
        let toward = (opening.center().x - sim.observation().player_pos.0).signum();
        sim.step(AgentAction { move_x: toward, ..base() });
    }
    if !standing_in(sim, ring) {
        walk_to(&mut Pad::new(sim), opening.center()).unwrap_or_else(|e| panic!("into the ring `{ring}`: {e}"));
    }
    for frame in 0..240 {
        if room_id(sim) != from {
            break;
        }
        let press = frame % 10 < 2;
        sim.step(AgentAction { interact: press, interact_held: press, ..base() });
    }
    if room_id(sim) == from {
        let at = sim.observation().player_pos;
        let world = sim.world();
        eprintln!(
            "[gate] no crossing from `{from}`: conversation {:?}, phase {:?}, link {:?}, at {:?}",
            world.resource::<ambition_platformer2d::conversation::ActiveConversation>().instance(),
            world.resource::<GatePortalPhases>().phase(ring),
            world.resource::<GateNetworkState>().route_from(ring),
            at,
        );
    }
    finish_passages(sim);
    for _ in 0..10 {
        sim.step(base());
    }
    room_id(sim)
}

/// Dial from the terminal of `ring`, leave the terminal, wait for the ring to
/// open, and go through. Returns the room the traveller arrives in.
fn travel(sim: &mut Platformer2dSimHarness, terminal: &str, ring: &str, address: &str) -> String {
    open_terminal(sim, terminal, ring);
    assert_eq!(dial(sim, ring, address), "link", "`{address}` answers from `{ring}`");
    ask(sim, ring, GateTerminalAction::Close);
    for _ in 0..60 {
        if portal_on(sim, ring) {
            break;
        }
        sim.step(base());
    }
    assert!(portal_on(sim, ring), "the ring `{ring}` opens after a dial that answers");
    go_through(sim, ring)
}

fn standing_in(sim: &mut Platformer2dSimHarness, ring: &str) -> bool {
    let opening = zone(sim, ring);
    let at = sim.observation().player_pos;
    (opening.min.x..=opening.max.x).contains(&at.0)
}

/// Acceptance 1: a fresh save dials TANGENT with no knowledge, walks into the
/// public gate and comes out of Math Town's ring; the address is known now,
/// the knowledge survives a save and a load, and the Math ring's terminal
/// takes the traveller home without a known address (LEGAL is written on it).
#[test]
fn a_fresh_save_dials_tangent_reaches_math_town_and_comes_home() {
    let mut sim = room(HUB);
    let flag = gate_address_known_flag("TANGENT");
    assert!(!save(&mut sim).flag(&flag), "a fresh save knows no address");
    assert_eq!(travel(&mut sim, HUB_TERMINAL, HUB_RING, "TANGENT"), MATH);
    assert!(standing_in(&mut sim, MATH_RING), "the traveller comes out of the Math ring");
    assert!(save(&mut sim).flag(&flag), "a dial that answered taught the address");

    // The save is written and read back: the knowledge is in it.
    let file = std::env::temp_dir().join(format!("the_gate_network_{}.ron", std::process::id()));
    ambition_platformer2d::persistence::save::write_save(&file, &save(&mut sim)).expect("the save is written");
    let read = ambition_platformer2d::persistence::save::load_save(&file).data;
    let _ = std::fs::remove_file(&file);
    assert!(read.flag(&flag), "the knowledge survives a save and a load");
    let mut reloaded = room_with(MATH, read);
    assert!(save(&mut reloaded).flag(&flag));

    // Home, from the reloaded save, by the address on the Math terminal.
    assert_eq!(travel(&mut reloaded, MATH_TERMINAL, MATH_RING, "LEGAL"), HUB);
    assert!(standing_in(&mut reloaded, HUB_RING), "the traveller comes home out of the public gate");
}

/// Acceptance 2: each public address reaches its arrival from a fresh save
/// with nothing known, and the arrival's terminal takes the traveller home by
/// the address written on it. ASCENT starts the gravity trail, not GNU-ton's
/// arena; CITADEL lands in the forest outside Castle Godwin, not its throne.
#[test]
fn each_destination_is_reached_from_the_hub_and_its_terminal_leads_home() {
    for (address, room_id, ring, terminal) in [
        ("GENESIS", "primeval_trailhead", "primeval_trailhead_gate", "primeval_gate_terminal"),
        ("CITADEL", "evil_gate_forest", "evil_gate_return_zone", "evil_gate_terminal"),
        ("ASCENT", "gnu_ton_orchard", "orchard_gate", "orchard_gate_terminal"),
        ("GALACTIC", "orbital_station_gate_dock", "orbital_station_gate_arrival", "orbital_station_gate_terminal"),
        ("SAILING", "pirate_port_gate_plaza", "pirate_port_gate_arrival", "pirate_port_gate_terminal"),
        ("SILENT", "ninja_village_gate_shrine", "ninja_village_gate_arrival", "ninja_village_gate_terminal"),
        ("ALDENTE", "noodly_narthex", "noodly_narthex_gate", "noodly_gate_terminal"),
    ] {
        let mut sim = room(HUB);
        assert_eq!(travel(&mut sim, HUB_TERMINAL, HUB_RING, address), room_id, "`{address}`");
        assert!(standing_in(&mut sim, ring), "`{address}` comes out of `{ring}`");
        assert_eq!(travel(&mut sim, terminal, ring, "LEGAL"), HUB, "home from `{address}`");
        assert!(standing_in(&mut sim, HUB_RING), "home out of the public gate from `{address}`");
    }
}

/// The hidden short address NEST reaches the rookery's old survey ring, part
/// way up the primeval trail, though it is in no directory.
#[test]
fn a_hidden_address_is_dialed_like_any_other() {
    let mut sim = room(HUB);
    assert_eq!(travel(&mut sim, HUB_TERMINAL, HUB_RING, "nest"), "primeval_rookery");
    assert!(standing_in(&mut sim, "primeval_rookery_gate"));
}

/// Acceptance 3: each refusal has its own kind and changes nothing: no link,
/// no knowledge. Repeated symbols match exactly; case does not matter.
#[test]
fn each_failed_dial_says_why_and_opens_nothing() {
    let mut sim = room(HUB);
    open_terminal(&mut sim, HUB_TERMINAL, HUB_RING);
    for (address, kind) in [
        ("", "empty"),
        ("TANGENTO", "illegal"),
        ("TANGENTT", "unknown"),
        ("TANGEN", "unknown"),
        ("SAND", "unknown"),
        ("TESLA", "restricted"),
        ("LEGAL", "same_facility"),
        ("SENTINEL", "restricted"),
    ] {
        assert_eq!(dial(&mut sim, HUB_RING, address), kind, "`{address}`");
        assert!(network(&mut sim).links.is_empty(), "`{address}` opened a link");
    }
    assert!(!save(&mut sim).flag(&gate_address_known_flag("SILENT")), "a refused dial teaches nothing");
    assert_eq!(dial(&mut sim, HUB_RING, "tangent"), "link", "case does not matter");
}

/// Cancel is safe and a redial works: leaving a terminal opens nothing, a
/// hangup shuts the ring, and a second dial replaces the first.
#[test]
fn a_terminal_can_be_left_hung_up_and_dialed_again() {
    let mut sim = room(HUB);
    open_terminal(&mut sim, HUB_TERMINAL, HUB_RING);
    ask(&mut sim, HUB_RING, GateTerminalAction::Close);
    assert!(network(&mut sim).terminal.is_none() && network(&mut sim).links.is_empty());
    open_terminal(&mut sim, HUB_TERMINAL, HUB_RING);
    assert_eq!(dial(&mut sim, HUB_RING, "SAND"), "unknown");
    assert_eq!(dial(&mut sim, HUB_RING, "TANGENT"), "link");
    ask(&mut sim, HUB_RING, GateTerminalAction::Hangup);
    assert!(network(&mut sim).links.is_empty(), "a hangup shuts the link");
    for _ in 0..60 {
        sim.step(base());
    }
    assert!(!portal_on(&mut sim, HUB_RING), "the ring is shut after a hangup");
    assert_eq!(dial(&mut sim, HUB_RING, "CITADEL"), "link");
    assert_eq!(dial(&mut sim, HUB_RING, "TANGENT"), "link", "a redial replaces the ring's own link");
    let links = network(&mut sim).links;
    assert_eq!(links.len(), 1);
    assert_eq!(links[HUB_RING].far, MATH_RING);
}

/// Acceptance 5: an opening ring takes nobody; the traveller crosses once,
/// by one confirmed transition, and the link closes after.
#[test]
fn an_opening_ring_takes_nobody_and_an_open_one_takes_the_traveller_once() {
    let mut sim = room(HUB);
    open_terminal(&mut sim, HUB_TERMINAL, HUB_RING);
    let opening = zone(&mut sim, HUB_RING);
    // Dial while standing at the terminal, then step into the ring at once.
    assert_eq!(dial(&mut sim, HUB_RING, "TANGENT"), "link");
    ask(&mut sim, HUB_RING, GateTerminalAction::Close);
    assert!(!portal_on(&mut sim, HUB_RING), "the ring is still opening");
    sim.teleport_player((opening.center().x, opening.max.y - 30.0));
    for _ in 0..3 {
        sim.step(AgentAction { interact: true, interact_held: true, ..base() });
        sim.step(base());
    }
    assert_eq!(room_id(&mut sim), HUB, "an opening ring took the traveller");
    let mut rooms = vec![room_id(&mut sim)];
    for frame in 0..240 {
        let press = frame % 10 < 2;
        sim.step(AgentAction { interact: press, interact_held: press, ..base() });
        let now = room_id(&mut sim);
        if rooms.last() != Some(&now) {
            rooms.push(now);
        }
    }
    finish_passages(&mut sim);
    assert_eq!(rooms, [HUB, MATH], "one crossing, to the far ring's room");
    for _ in 0..120 {
        sim.step(base());
    }
    assert!(network(&mut sim).links.is_empty(), "the link closes after its traveller");
}

/// Acceptance 4: a source ring answers by its own rules. Ring 7 of the gate
/// stack has no power until its breaker is thrown, dials only the Math line,
/// and never dials its own facility; a ring in a link is busy for another.
#[test]
fn ring_seven_answers_by_its_power_its_reach_and_its_facility() {
    let mut sim = room("gate_stack_lower");
    open_terminal(&mut sim, RING7_TERMINAL, RING7);
    assert_eq!(dial(&mut sim, RING7, "TANGENT"), "offline", "no power yet");
    ask(&mut sim, RING7, GateTerminalAction::Close);
    press_switch(&mut sim, RING7_BREAKER);
    open_terminal(&mut sim, RING7_TERMINAL, RING7);
    assert_eq!(dial(&mut sim, RING7, "GENESIS"), "restricted", "ring 7 reaches the Math line only");
    assert_eq!(dial(&mut sim, RING7, "LEGAL"), "same_facility");
    assert_eq!(dial(&mut sim, RING7, "TANGENT"), "link");
    // The Math ring is in ring 7's link: the public gate finds it busy.
    let mut state = network(&mut sim);
    state.terminal = Some(HUB_RING.to_string());
    let catalog = sim.world().resource::<ambition_platformer2d::world::rooms::GateNetworkCatalog>().clone();
    let facts = ambition_platformer2d::world::rooms::GateFacts {
        switch_on: &|_: &str| true,
        flag: &|_: &str| false,
        zone_exists: &|_: &str, _: &str| true,
    };
    let busy = ambition_platformer2d::world::rooms::resolve_dial(&catalog, &state, HUB_RING, "TANGENT", &facts);
    assert_eq!(busy.map(|ring| ring.zone.clone()).unwrap_err().kind(), "busy");
}

/// Acceptance 7: the fully composed visible app builds its Update schedule
/// with no cycle and runs a frame of the Ambition route with the network's
/// systems in it. A cycle (as a door-visual change once made, which no unit
/// test saw) panics at the first update of the schedule.
#[test]
fn the_visible_app_builds_its_update_schedule_with_the_gate_network() {
    let mut app = ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();
    app.world_mut().write_message(ambition_platformer2d::game_shell::ShellCommand::ReplaceWith {
        route: ambition_platformer2d::game_shell::ShellRouteId::new("ambition_gameplay"),
        request: None,
    });
    crate::common::step_until_route_is_active(&mut app, "ambition_gameplay");
    for _ in 0..5 {
        app.update();
    }
    let mut names = Vec::new();
    app.world_mut().resource_scope::<bevy::ecs::schedule::Schedules, _>(|world, mut schedules| {
        let update = schedules.get_mut(bevy::prelude::Update).expect("an Update schedule");
        update.initialize(world).unwrap_or_else(|e| panic!("the visible Update schedule does not build: {e}"));
        names.extend(update.systems().expect("the Update schedule is built").map(|(_, system)| system.name().to_string()));
    });
    for wanted in ["drive_gate_terminal", "draw_gate_terminal", "declare_gate_terminal_context"] {
        assert!(names.iter().any(|name| name.contains(wanted)), "the visible app has no `{wanted}`");
    }
}
