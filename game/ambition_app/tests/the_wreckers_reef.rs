//! The Wreckers' Reef: out of the Pirate Port, east of the docks
//! (`tools/ambition_ldtk_tools/specs/pirate_port/make_specs.py`).
//!
//! Jon (2026-10-30): "out of town zones with enemies", joined by side exits.
//! The player starts on the reef's shore, walks by keys west onto the docks,
//! and back east along the shore to the reef.

use crate::the_pirate_port::{a_session_in, go};

const REEF: &str = "pirate_wreckers_reef";
const DOCKS: &str = "pirate_port_docks";

#[test]
fn the_reef_lies_east_of_the_docks_along_the_shore() {
    let mut app = a_session_in(REEF);
    go(&mut app, DOCKS, Some("reef_west"));
    go(&mut app, REEF, Some("docks_east"));
}
