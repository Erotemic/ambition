//! The Wreckers' Reef: out of the Pirate Port, east of the docks
//! (`tools/ambition_ldtk_tools/specs/pirate_port/make_specs.py`).
//!
//! Jon (2026-10-30): "out of town zones with enemies", joined by side exits.
//! The player starts on the reef's shore, walks by keys west onto the docks,
//! and back east along the shore to the reef.

use crate::the_pirate_port::{a_session_in, go, talk};

const REEF: &str = "pirate_wreckers_reef";
const DOCKS: &str = "pirate_port_docks";

#[test]
fn the_reef_lies_east_of_the_docks_along_the_shore() {
    let mut app = a_session_in(REEF);
    go(&mut app, DOCKS, Some("reef_west"));
    go(&mut app, REEF, Some("docks_east"));
}

/// Nell Brine beachcombs the reef's shore (`pirate_port.yarn`).
#[test]
fn the_beachcomber_sells_authentic_salvage() {
    let mut app = a_session_in(REEF);
    crate::the_pirate_port::talk(&mut app, "npc_port_brine", "What do you sell?");
}

/// The Lighthouse Point, east of the reef by a side exit at its east rocks:
/// Ezra Lamplugh talks by his cottage, and the reef is back west.
#[test]
fn the_lighthouse_point_lies_east_of_the_reef() {
    let mut app = a_session_in("pirate_lighthouse_point");
    talk(&mut app, "npc_port_lamplugh", "Why do the wreckers want your light out?");
    go(&mut app, "pirate_wreckers_reef", Some("point_west"));
    go(&mut app, "pirate_lighthouse_point", Some("reef_east"));
}

/// The sail loft, in at its door in the net loft on the waterfront: Tamsin
/// Bolt sews flags to order (`pirate_port.yarn` `pirate_port_bolt`).
#[test]
fn the_sail_loft_sews_flags_to_order() {
    let mut app = a_session_in("pirate_port_waterfront");
    go(&mut app, "pirate_sail_loft", Some("waterfront_loft_door"));
    talk(&mut app, "npc_port_bolt", "What are the flags on the wall?");
    go(&mut app, "pirate_port_waterfront", Some("loft_door"));
}

/// The cellars under customs, down the old well against the plaza's west
/// wall by a vertical exit: Fennimore Slipp, import consultant, talks by his
/// desk (`pirate_port.yarn` `pirate_port_slipp`); the ladder goes up the
/// shaft to the plaza, and back down. (Up first, as the Mobius Mine's test:
/// the walker takes the well's ladder down from it, not from the floor.)
#[test]
fn the_cellars_under_customs_lie_down_the_plazas_well() {
    let mut app = a_session_in("pirate_customs_cellars");
    talk(&mut app, "npc_port_slipp", "What is all this?");
    go(&mut app, "pirate_port_gate_plaza", Some("cellars_shaft"));
    go(&mut app, "pirate_customs_cellars", Some("plaza_well"));
}

/// East of the cellars through the knocked-through arch: the smugglers'
/// tunnel under the waterfront. Its drain's grate is shut from the street
/// until the lever under it is pulled; then up the drain to the waterfront
/// and back down: the port's loop under it. The plank wall at the tunnel's
/// east end is false (the hoard behind it).
#[test]
fn the_smugglers_tunnel_loops_the_cellars_to_the_waterfront() {
    use crate::the_note_travels_from_alice_to_bob::tap;
    use ambition_platformer2d::engine_core as ae;
    use bevy::prelude::KeyCode;

    let lever = |app: &bevy::prelude::App| {
        app.world()
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data()
            .switch("pirate_port_drain_lever")
    };
    let mut app = a_session_in("pirate_customs_cellars");
    go(&mut app, "pirate_smugglers_tunnel", Some("cellars_east"));
    go(&mut app, "pirate_smugglers_hoard", Some("secret_tunnel_east"));
    go(&mut app, "pirate_smugglers_tunnel", Some("secret_hoard_west"));
    assert!(!lever(&app), "control: the drain's lever is not pulled");
    crate::the_pirate_port::go_to(&mut app, ae::Vec2::new(1812.0, 560.0), "the drain's lever");
    tap(&mut app, KeyCode::KeyF);
    for _ in 0..30 {
        app.update();
    }
    assert!(lever(&app), "the drain's lever is pulled");
    go(&mut app, "pirate_port_waterfront", Some("tunnel_drain"));
    go(&mut app, "pirate_smugglers_tunnel", Some("waterfront_drain"));
    go(&mut app, "pirate_customs_cellars", Some("tunnel_west"));
}
