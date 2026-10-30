//! The Bamboo Pass: out of the Ninja Village, east of the cedar forest
//! (`tools/ambition_ldtk_tools/specs/ninja_village/make_specs.py`).
//!
//! Jon (2026-10-30): "out of town zones with enemies", joined by side exits.
//! The player starts at the pass's west end, walks by keys west into the
//! forest, and back east out to the pass.

use crate::the_pirate_port::{a_session_in, go};

const PASS: &str = "ninja_bamboo_pass";
const FOREST: &str = "ninja_village_forest";

#[test]
fn the_bamboo_pass_lies_east_of_the_cedar_forest_by_its_side_exit() {
    let mut app = a_session_in(PASS);
    go(&mut app, FOREST, Some("pass_west"));
    go(&mut app, PASS, Some("forest_east"));
}
