//! The Bamboo Pass: out of the Ninja Village, east of the cedar forest
//! (`tools/ambition_ldtk_tools/specs/ninja_village/make_specs.py`).
//!
//! Jon (2026-10-30): "out of town zones with enemies", joined by side exits.
//! The player starts at the pass's west end, walks by keys west into the
//! forest, and back east out to the pass. In the forest, Moriyama the
//! charcoal burner talks at his kiln.

use crate::the_pirate_port::{a_session_in, go, talk};

const PASS: &str = "ninja_bamboo_pass";
const FOREST: &str = "ninja_village_forest";

#[test]
fn the_bamboo_pass_lies_east_of_the_cedar_forest_by_its_side_exit() {
    let mut app = a_session_in(PASS);
    go(&mut app, FOREST, Some("pass_west"));
    go(&mut app, PASS, Some("forest_east"));
}

#[test]
fn moriyama_burns_charcoal_in_the_cedar_forest() {
    let mut app = a_session_in(FOREST);
    talk(&mut app, "npc_village_moriyama", "What are you burning?");
}

/// Matsuda referees the pass by his tally board, before the first bout.
#[test]
fn the_pass_referee_keeps_the_count() {
    let mut app = a_session_in(PASS);
    talk(&mut app, "npc_village_matsuda", "What is the board for?");
}
