//! The Searchlight Moor: out of town in the Evil dimension, west of the gate
//! forest (`tools/ambition_ldtk_tools/specs/evil_moor/make_specs.py`).
//!
//! Jon (2026-10-30): "out of town zones with enemies", joined by side exits.
//! The player starts at the moor's east end, walks by keys east into the gate
//! forest, and back out west to the moor.

use crate::the_pirate_port::{a_session_in, go, talk};

const MOOR: &str = "evil_searchlight_moor";
const FOREST: &str = "evil_gate_forest";

#[test]
fn the_moor_lies_west_of_the_gate_forest_by_its_side_exit() {
    let mut app = a_session_in(MOOR);
    go(&mut app, FOREST, Some("moor_east"));
    go(&mut app, MOOR, Some("forest_west"));
}

/// The Crooked Lantern: from the moor's far west end through the inn's door,
/// a word with Tobias and with Gudrun, and back out.
#[test]
fn the_crooked_lantern_is_lit_at_the_end_of_the_moor() {
    let mut app = a_session_in("evil_crooked_lantern");
    talk(&mut app, "npc_evil_tobias", "How do you know?");
    talk(&mut app, "npc_evil_gudrun", "Why keep an inn out here?");
    go(&mut app, MOOR, Some("inn_door"));
    go(&mut app, "evil_crooked_lantern", Some("moor_inn_door"));
}
