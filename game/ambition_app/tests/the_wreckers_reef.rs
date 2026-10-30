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
