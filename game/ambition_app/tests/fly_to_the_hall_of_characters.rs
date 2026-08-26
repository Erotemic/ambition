//! FLY TO A NAMED DOOR, PRESS INTERACT, LAND IN THAT ROOM.
//!
//! the hall of characters door and press interact and check if it loads into the
//! new room?"*
//!
//! this names its DESTINATION, which the sibling door tests do not. They
//! take whichever `Door` zone is nearest and assert the room merely CHANGED —
//! true of a door that leads somewhere wrong, and true of a door that leads back
//! where you started. This one asks for `hall_of_characters` by name, walks to
//! that specific zone, and asserts she arrives THERE.
//!
//! and she FLIES, which is a different road through the movement kernel.
//! A walked approach is carried by the grounded integrator; flight is free
//! motion with drag and no ground contact, so the two reach a zone by different
//! code. `walking_into_a_loading_zone` covers the walk; this covers the flight,
//! and between them a door has been entered by both ways a body can arrive.
//!
//! the flight is a real toggle press through `AgentAction::fly_toggle`, not a
//! component poked onto the body — a body given `fly_enabled` by hand would
//! prove the transition works for a state no player can reach.

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::world::rooms::{LoadingZone, LoadingZoneActivation};
use bevy::prelude::With;

use crate::common::{base, fixed_60hz_sim};

/// The room the hub's own authored door says it leads to.
const DESTINATION: &str = "hall_of_characters";

/// Frames she may spend getting there. A liveness backstop: the assertions are
/// about WHICH room, never about when.
const FLIGHT_CAP: usize = 900;

fn active_room(sim: &mut Platformer2dSimHarness) -> String {
    sim.observation().active_room.clone()
}

fn body_pos(sim: &mut Platformer2dSimHarness) -> ambition_platformer2d::engine_core::Vec2 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ambition_platformer2d::platformer::body::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    q.single(world).expect("a controlled body").pos
}

/// The authored `Door` zone that leads to `DESTINATION`, asked of the room
/// graph rather than of the zone.
///
/// a `LoadingZone` does not carry its destination — the edge does, keyed
/// by zone id — so the honest way to ask "where does this door go" is the
/// production road itself: stand a probe rect inside the zone and let
/// `transition_for_player` answer. A test that matched on the zone's NAME would
/// be inventing a second mapping that could disagree with the real one.
fn the_named_door(sim: &mut Platformer2dSimHarness) -> Option<(LoadingZone, Vec<String>)> {
    let world = sim.world_mut();
    let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let room_set = query.iter(world).next()?;
    let mut offered = Vec::new();
    let mut found = None;
    for zone in room_set.active_loading_zones() {
        if zone.activation != LoadingZoneActivation::Door {
            continue;
        }
        // A probe the size of the zone, sitting in it, already "pressing".
        let probe = zone.aabb;
        let Some(transition) = room_set.transition_for_player(
            probe,
            ambition_platformer2d::engine_core::Vec2::ZERO,
            true,
        ) else {
            continue;
        };
        let target = room_set.rooms[transition.target_room].id.clone();
        offered.push(target.clone());
        if target == DESTINATION && found.is_none() {
            found = Some(zone.clone());
        }
    }
    found.map(|zone| (zone, offered))
}

#[test]
fn flying_to_the_hall_of_characters_door_and_pressing_interact_loads_the_hall() {
    let mut sim = fixed_60hz_sim();
    for _ in 0..10 {
        sim.step(base());
    }
    let from = active_room(&mut sim);

    // LOUD, never a quiet `return`: a test that skips itself when it cannot
    // find its subject reports green for the one reason it exists.
    let (door, offered) = the_named_door(&mut sim).unwrap_or_else(|| {
        panic!("'{from}' authors no Door zone leading to '{DESTINATION}'")
    });
    assert!(
        offered.contains(&DESTINATION.to_string()),
        "the hub's doors lead to {offered:?}, none of them '{DESTINATION}'",
    );

    // Take off. A real toggle press, then fly toward the door in both axes.
    sim.step(AgentAction {
        fly_toggle: true,
        ..base()
    });

    let target = door.aabb.center();
    let mut closest = f32::MAX;
    for _ in 0..FLIGHT_CAP {
        let now = active_room(&mut sim);
        if now != from {
            assert_eq!(
                now, DESTINATION,
                "she pressed interact in the `{}` door of '{from}' and the room \
                 became '{now}' instead of '{DESTINATION}' — the door led \
                 somewhere its own `target_room` does not name",
                door.name,
            );
            return;
        }
        let here = body_pos(&mut sim);
        closest = closest.min((target - here).length());
        let to = target - here;
        // Interact only inside the target zone; holding interact during transit can
        // activate unrelated doors along the route.
        let inside = here.x >= door.aabb.min.x
            && here.x <= door.aabb.max.x
            && here.y >= door.aabb.min.y
            && here.y <= door.aabb.max.y;
        sim.step(AgentAction {
            move_x: to.x.signum() * (to.x.abs() > 2.0) as i32 as f32,
            move_y: to.y.signum() * (to.y.abs() > 2.0) as i32 as f32,
            right_pressed: to.x > 2.0,
            left_pressed: to.x < -2.0,
            interact: inside,
            interact_held: inside,
            ..base()
        });
    }

    panic!(
        "flew at the `{}` door of '{from}' for {FLIGHT_CAP} frames holding \
         interact and never left. She ended at {:?}, the door is {:?}, and she \
         got within {closest:.1}px of its centre. ⚠ a closest distance that \
         never shrank means the FLIGHT never started — check the `fly_toggle` \
         press, not the door.",
        door.name,
        body_pos(&mut sim),
        door.aabb,
    );
}
