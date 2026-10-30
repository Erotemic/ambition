//! Zeno's Aqueduct: out of the Math dimension's town, west along the
//! aqueduct (`tools/ambition_ldtk_tools/specs/math_town/make_specs.py`).
//!
//! Jon (2026-10-30): "out of town zones with enemies". The player starts on
//! the aqueduct's deck, walks by keys (`common::walk`) east along it into the
//! town's street, and back out along it to the aqueduct.

use ambition_platformer2d::engine_core as ae;
use bevy::prelude::App;

use crate::the_pirate_port::{a_session_in, go, go_to};

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

/// Jon (2026-10-30): "Make sure it's actually 64". The tower has the 64
/// golden disks, each its own prop (`hanoi_disk_01` .. `hanoi_disk_64`), each
/// on one of the three needles, stacked with no gap, and never a larger on a
/// smaller. The disks are the climb: the player walks by keys up the
/// tallest stack, a disk a step, to its top.
#[test]
fn the_tower_of_hanoi_has_sixty_four_disks_never_a_larger_on_a_smaller() {
    use ambition_platformer2d::platformer::lifecycle::session_world_component;
    use ambition_platformer2d::world::rooms::RoomSet;
    use crate::common::walk::{Keys, Press, Walker as _};

    let mut app = a_session_in("math_tower_of_hanoi");
    let live = ambition_platformer2d::world::rooms::sole_live_room_definition(app.world()).expect("a live room");
    let spec = session_world_component::<RoomSet>(app.world()).expect("a room set").spec(live).clone();
    let needles: Vec<f32> = spec.props.iter().filter(|p| p.kind == "hanoi_needle").map(|p| p.pos.x).collect();
    assert_eq!(needles.len(), 3, "the tower has three needles");
    // (size, centre, top, bottom) of each disk.
    let mut disks: Vec<(u32, f32, f32, f32)> = spec
        .props
        .iter()
        .filter_map(|p| {
            let size = p.kind.strip_prefix("hanoi_disk_")?.parse().ok()?;
            Some((size, p.pos.x, p.pos.y - p.size.y * 0.5, p.pos.y + p.size.y * 0.5))
        })
        .collect();
    let mut sizes: Vec<u32> = disks.iter().map(|d| d.0).collect();
    sizes.sort_unstable();
    assert_eq!(sizes, (1..=64).collect::<Vec<_>>(), "each of the 64 disks, once");

    let floor = disks.iter().map(|d| d.3).fold(f32::MIN, f32::max);
    disks.sort_by(|a, b| b.3.total_cmp(&a.3));
    let mut tallest = (0, floor);
    for x in &needles {
        let stack: Vec<_> = disks.iter().filter(|d| (d.1 - x).abs() < 1.0).collect();
        let mut under = (u32::MAX, floor);
        for &&(size, _, top, bottom) in &stack {
            assert!(size < under.0, "disk {size} is on disk {}, a smaller one, on the needle at x {x}", under.0);
            assert!((bottom - under.1).abs() < 0.5, "disk {size} does not lie on what is under it (x {x})");
            under = (size, top);
        }
        if stack.len() > tallest.0 {
            tallest = (stack.len(), under.1);
        }
    }
    let on_needles: usize = needles.iter().map(|x| disks.iter().filter(|d| (d.1 - x).abs() < 1.0).count()).sum();
    assert_eq!(on_needles, 64, "every disk is on a needle");

    // The tallest stack is a climb: each disk's top is a one-way line, so a
    // jump goes up through the disks over it and lands on the highest one
    // under its peak. The player walks to the floor east of the stack, then
    // jumps, moving in over its top, until it stands on the top disk.
    let (_, top) = tallest;
    let x = needles.iter().copied().find(|x| disks.iter().filter(|d| (d.1 - x).abs() < 1.0).any(|d| d.2 == top)).unwrap();
    let base = spec
        .props
        .iter()
        .filter(|p| p.kind.starts_with("hanoi_disk_") && (p.pos.x - x).abs() < 1.0)
        .map(|p| p.pos.x + p.size.x * 0.5)
        .fold(f32::MIN, f32::max);
    go_to(&mut app, ae::Vec2::new(base + 40.0, floor - 30.0), "the foot of the tallest stack");
    let body = crate::neighbor_prefetch_prepares_rooms::alice(&mut app);
    let at = |app: &App| {
        let kin = app.world().get::<ae::BodyKinematics>(body).expect("the player");
        (kin.pos.x, kin.pos.y + kin.size.y * 0.5)
    };
    let mut keys = Keys::new(&mut app);
    for frame in 0..1200 {
        let (px, feet) = at(keys.app);
        if px <= x + 40.0 && (feet - top).abs() < 1.0 {
            break;
        }
        let jump = frame % 40 < 20;
        let axis = if px > x + 40.0 { -1.0 } else { 0.0 };
        keys.step(Press { axis, jump_pressed: jump, jump_held: jump, ..Default::default() });
    }
    for _ in 0..60 {
        keys.step(Press::default());
    }
    drop(keys);
    let (px, feet) = at(&app);
    assert!((feet - top).abs() < 2.0, "the player stands at ({px}, {feet}), not on the top disk (y {top})");
}

/// The Fibonacci Meadow, east of the Fractal Woods by a side exit, floors
/// level: Farmer Fibbs talks by his counting board (`math.yarn`
/// `meadow_fibbs`), and the woods are back west.
#[test]
fn the_fibonacci_meadow_lies_east_of_the_woods() {
    let mut app = a_session_in("math_fibonacci_meadow");
    crate::the_pirate_port::talk(&mut app, "npc_math_fibbs", "How many rabbits do you have?");
    go(&mut app, "math_fractal_woods", Some("math_fibonacci_meadow_west"));
    go(&mut app, "math_fibonacci_meadow", Some("math_fractal_woods_east"));
}
