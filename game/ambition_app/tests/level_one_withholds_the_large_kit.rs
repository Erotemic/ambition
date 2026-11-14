//! LEVEL-1-KIT: level 1 withholds everything past run, one jump and the
//! strike (each room's `withheld_verbs`, written into the player's body as a
//! ceiling contribution). The control is the same robot in the hub, whose
//! rooms withhold nothing: it has its whole kit.

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::{AbilityBase, AbilitySet, BodyAbilities};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;

use crate::common::{base, fixed_60hz_room_options};

/// The player's authored kit and its effective one, a few ticks into `room`.
fn kit_in(room: &str) -> (AbilitySet, AbilitySet) {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(room)).expect("the room boots");
    for _ in 0..5 {
        sim.step(base());
    }
    let world = sim.world_mut();
    let mut player = world.query_filtered::<(&AbilityBase, &BodyAbilities), PrimaryPlayerOnly>();
    let (authored, effective) = player.single(world).expect("one primary player");
    (authored.abilities, effective.abilities)
}

#[test]
fn level_one_withholds_the_large_kit() {
    let (authored, effective) = kit_in("intro_hobford_lane");
    assert!(
        authored.double_jump && authored.dash && authored.pogo,
        "premise: the robot's authored kit has the air jump, the dash and the pogo"
    );
    assert!(effective.jump && effective.attack && effective.move_horizontal, "level 1 keeps run, jump and the strike");
    let withheld = [
        ("double_jump", effective.double_jump),
        ("wall_jump", effective.wall_jump),
        ("dash", effective.dash),
        ("fly", effective.fly),
        ("blink", effective.blink),
        ("pogo", effective.pogo),
    ];
    let kept: Vec<_> = withheld.iter().filter(|(_, granted)| *granted).map(|(verb, _)| *verb).collect();
    assert!(kept.is_empty(), "level 1 left the robot {kept:?}");

    let (_, in_the_hub) = kit_in("central_hub_complex");
    assert!(
        in_the_hub.double_jump && in_the_hub.dash && in_the_hub.pogo,
        "the control: in the hub, which withholds nothing, the robot has its whole kit"
    );
}
