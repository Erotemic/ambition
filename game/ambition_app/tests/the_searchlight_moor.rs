//! The Searchlight Moor: out of town in the Evil dimension, west of the gate
//! forest (`tools/ambition_ldtk_tools/specs/evil_moor/make_specs.py`).
//!
//! Jon (2026-10-30): "out of town zones with enemies", joined by side exits.
//! The player starts at the moor's east end, walks by keys east into the gate
//! forest, and back out west to the moor.

use crate::the_pirate_port::{a_session_in, go};

const MOOR: &str = "evil_searchlight_moor";
const FOREST: &str = "evil_gate_forest";

#[test]
fn the_moor_lies_west_of_the_gate_forest_by_its_side_exit() {
    let mut app = a_session_in(MOOR);
    go(&mut app, FOREST, Some("moor_east"));
    go(&mut app, MOOR, Some("forest_west"));
}
