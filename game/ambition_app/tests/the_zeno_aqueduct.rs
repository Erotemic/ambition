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

/// The Mobius Mine under the woods: Felix Kleinbottle talks (`math.yarn`
/// `mine_kleinbottle`); up the shaft's ladder to the woods, and back down
/// the ladder in their hole.
#[test]
fn the_mobius_mine_lies_under_the_fractal_woods() {
    let mut app = a_session_in("math_mobius_mine");
    crate::the_pirate_port::talk(&mut app, "npc_math_kleinbottle", "What do you mine?");
    go(&mut app, "math_fractal_woods", Some("math_mobius_mine_shaft"));
    go(&mut app, "math_mobius_mine", Some("math_fractal_woods_shaft"));
}

/// The Tower of Hanoi, west of the gate hall by a side exit: Brother Lucas
/// keeps the count (`math.yarn` `hanoi_lucas`), and the hall is back east.
#[test]
fn the_tower_of_hanoi_lies_west_of_the_gate_hall() {
    let mut app = a_session_in("math_tower_of_hanoi");
    crate::the_pirate_port::talk(&mut app, "npc_math_lucas", "What happens when the last disk is moved?");
    go(&mut app, "math_gate_hall", Some("math_tower_of_hanoi_east"));
    go(&mut app, "math_tower_of_hanoi", Some("math_gate_hall_west"));
}
