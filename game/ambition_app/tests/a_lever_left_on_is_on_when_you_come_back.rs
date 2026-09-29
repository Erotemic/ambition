//! A mechanism the player changes stays changed when they leave and return.
//!
//! `switch_lab` (`sandbox.ldtk`) authors one lever for exactly this, prompt
//! "Toggle persistence", action `ToggleFlag`, and one two-way door to the hub.
//! Until 2026-09-29 no engine road read `ToggleFlag`, so the lever did nothing
//! when pressed. The open-world roadmap's acceptance row "alter world
//! mechanisms, return to the changed state" had no test that pressed a switch,
//! left the room and came back.

use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};

use crate::common::{
    a_save_that_has_seen_the_hub_intro, base, fixed_60hz_room_options, walk_through_the_door_to,
};

const ROOM: &str = "switch_lab";
const HUB: &str = "central_hub_complex";
const LEVER: &str = "switch_lab_persist_switch";

/// The lever as the save holds it and as the rebuilt switch shows it.
fn lever(sim: &mut Platformer2dSimHarness) -> (bool, bool) {
    use ambition_platformer2d::combat::components::FeatureId;
    use ambition_platformer2d::encounter::switches::SwitchFeature;
    let world = sim.world_mut();
    let saved = world
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data()
        .switch(LEVER);
    let feature_id = world
        .query::<(&SwitchFeature, &FeatureId)>()
        .iter(world)
        .find(|(feature, _)| feature.activation.id == LEVER)
        .map(|(_, id)| id.0.clone())
        .expect("switch_lab authors the lever");
    let shown = world
        .resource::<ambition_platformer2d::sim_view::FeatureViewIndex>()
        .get(&feature_id)
        .expect("the lever publishes a view")
        .switch_on;
    (saved, shown)
}

/// Stand on the lever and press Interact once, the way a player does.
fn press_the_lever(sim: &mut Platformer2dSimHarness) {
    let at = {
        let world = sim.world_mut();
        world
            .query::<(
                &ambition_platformer2d::encounter::switches::SwitchFeature,
                &ambition_platformer2d::engine_core::geometry::CenteredAabb,
            )>()
            .iter(world)
            .find(|(feature, _)| feature.activation.id == LEVER)
            .map(|(_, aabb)| (aabb.center.x, aabb.center.y))
            .expect("the lever has a body in the room")
    };
    sim.teleport_player(at);
    sim.step(base());
    sim.step(AgentAction {
        interact: true,
        ..base()
    });
    for _ in 0..5 {
        sim.step(base());
    }
}

/// Press the lever, leave through the door, come back: it is still on, in the
/// save and on the switch the returning room built. The reading before the
/// press is the control: the room starts with the lever off.
#[test]
fn a_lever_left_on_is_on_when_you_come_back() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .expect("switch_lab boots");
    for _ in 0..10 {
        sim.step(base());
    }
    assert_eq!(lever(&mut sim), (false, false), "the lever starts off");

    press_the_lever(&mut sim);
    assert_eq!(
        lever(&mut sim),
        (true, true),
        "one press turns the lever on, and the switch shows it"
    );

    assert_eq!(walk_through_the_door_to(&mut sim, HUB), HUB);
    for _ in 0..10 {
        sim.step(base());
    }
    assert_eq!(walk_through_the_door_to(&mut sim, ROOM), ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    assert_eq!(
        lever(&mut sim),
        (true, true),
        "the lever the player left on was off when they came back"
    );

    // And it is a lever, not a latch: a second press turns it off.
    press_the_lever(&mut sim);
    assert_eq!(lever(&mut sim), (false, false), "a second press turns it off");
}
