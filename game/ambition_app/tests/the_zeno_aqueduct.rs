//! Zeno's Aqueduct: out of the Math dimension's town, west along the
//! aqueduct (`tools/ambition_ldtk_tools/specs/math_town/make_specs.py`).
//!
//! Jon (2026-10-30): "out of town zones with enemies". The player starts on
//! the aqueduct's deck, walks by keys (`common::walk`) east along it into the
//! town's street, and back out along it to the aqueduct.

use crate::the_pirate_port::{a_session_in, go};

const ZENO: &str = "math_zeno_aqueduct";
const STREET: &str = "math_town_street";

#[test]
fn the_aqueduct_runs_from_town_out_to_zenos_reservoir_and_back() {
    let mut app = a_session_in(ZENO);
    go(&mut app, STREET, Some("math_zeno_aqueduct_east"));
    go(&mut app, ZENO, Some("math_town_street_aqueduct_west"));
}

/// The Fractal Woods: out of town east of the summit. The player starts at
/// the woods' west end, walks by keys west onto the summit, and back east.
#[test]
fn the_fractal_woods_lie_east_of_the_summit() {
    let mut app = a_session_in("math_fractal_woods");
    go(&mut app, STREET, Some("math_fractal_woods_west"));
    go(&mut app, "math_fractal_woods", Some("math_town_street_summit_east"));
}

/// Ivo keeps the Fractal Woods (`math.yarn` `woods_ivo`).
#[test]
fn the_groundskeeper_of_the_fractal_woods_talks() {
    let mut app = a_session_in("math_fractal_woods");
    crate::the_pirate_port::talk(&mut app, "npc_math_ivo", "What do you do here?");
}

/// Zeno, halfway along the aqueduct's deck, talks (`math.yarn`
/// `zeno_halfway`): from the deck's town end, past the mite and the owl.
#[test]
fn zeno_is_halfway_to_the_reservoir() {
    let mut app = a_session_in(ZENO);
    crate::the_pirate_port::talk(&mut app, "npc_math_zeno", "How far have you got?");
}
