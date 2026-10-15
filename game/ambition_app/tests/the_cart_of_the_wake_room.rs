//! The cart of the wake room is a platform, and its ramp can be walked.
//!
//! The cart is not a prop on a platform: its bed IS a platform that stands
//! still, and the player wakes on it. The back end of the cart is let down as
//! a ramp: a line the player walks (`walkable_lines`), from the end of the bed
//! to the floor. This walks it down and up, with no tick in the air.
//!
//! The numbers are those of the room (`intro.ldtk`, `intro_wake_room`).

use crate::common::{base, fixed_60hz_room_options};
use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;

const ROOM: &str = "intro_wake_room";
/// The surface of the bed of the cart, and its two ends.
const BED_TOP: f32 = 294.0;
const BED: (f32, f32) = (48.0, 224.0);
/// The floor of the room, and where the ramp comes down to it.
const FLOOR: f32 = 352.0;
const RAMP_FOOT: f32 = 322.0;

/// Where the middle of the player is, and where its feet are.
fn player(sim: &mut Platformer2dSimHarness) -> (f32, f32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    let kin = q.iter(world).next().expect("the room has a player");
    (kin.pos.x, kin.pos.y + kin.size.y * 0.5)
}

#[test]
fn the_player_wakes_in_the_cart_and_walks_its_ramp_down_and_up() {
    // The cutscene of the room holds the player: this save has seen it.
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen".to_string(), true);
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM).with_save(save))
        .expect("the wake room builds");
    for _ in 0..40 {
        sim.step(base());
    }
    let (x, feet) = player(&mut sim);
    assert!(BED.0 < x && x < BED.1, "the player wakes over the bed of the cart, not at x {x}");
    assert!((feet - BED_TOP).abs() < 1.0, "the player stands on the bed ({BED_TOP}), not at {feet}");

    // Down the ramp to the floor. No step of the way is a fall.
    let mut biggest_drop = 0.0_f32;
    let mut before = feet;
    for _ in 0..600 {
        sim.step(AgentAction { move_x: 1.0, ..base() });
        let (x, feet) = player(&mut sim);
        if x < RAMP_FOOT {
            biggest_drop = biggest_drop.max(feet - before);
        }
        before = feet;
        if x > RAMP_FOOT + 30.0 {
            break;
        }
    }
    let (x, feet) = player(&mut sim);
    assert!(x > RAMP_FOOT, "the player walks off the end of the ramp; it stopped at x {x}");
    assert!((feet - FLOOR).abs() < 1.0, "past the ramp the player is on the floor ({FLOOR}), not at {feet}");
    assert!(
        biggest_drop < 9.0,
        "the way down the ramp is a slope, and one tick took the feet down {biggest_drop}: the player fell off the bed"
    );

    // And up it again, with no jump.
    for _ in 0..900 {
        sim.step(AgentAction { move_x: -1.0, ..base() });
        let (x, _) = player(&mut sim);
        if x < BED.1 - 40.0 {
            break;
        }
    }
    let (x, feet) = player(&mut sim);
    assert!(x < BED.1, "the player comes back over the bed; it stopped at x {x}");
    assert!(
        (feet - BED_TOP).abs() < 1.0,
        "the player walks up the ramp onto the bed ({BED_TOP}) with no jump; its feet are at {feet} \
         (at {FLOOR} it went under the ramp along the floor)"
    );
}
