//! Apply developer gravity requests to the primary body's room.

use ambition_platformer2d_shared_tangle::gravity::{AmbientGravityRequest, BaseGravity};
use bevy::prelude::*;

/// Apply every [`AmbientGravityRequest`] written since the last tick to
/// [`BaseGravity`]. Runs in the simulation schedule, before the resolver that
/// copies the ambient into each body's frame, so a request made this frame is
/// felt this tick. Two requests in one tick cycle twice, in write order.
///
/// The developer turns the primary seat's room, which is the room the
/// developer looks at ([`ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom`]).
pub fn apply_ambient_gravity_requests(
    mut requests: MessageReader<AmbientGravityRequest>,
    mut base: ResMut<BaseGravity>,
    developer: ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom,
) {
    let room = developer.get();
    for request in requests.read() {
        match request {
            AmbientGravityRequest::Cycle => base.cycle(room),
        }
    }
}

#[cfg(test)]
mod ambient_request_tests {
    use super::*;
    use bevy::app::App;

    /// The request road, end to end: a request written outside the sim
    /// schedule is applied by the sim's applier, and only there. Poison:
    /// make `apply_ambient_gravity_requests` ignore `Cycle` and the direction
    /// stays down.
    #[test]
    fn a_cycle_request_is_applied_by_the_sim_and_steps_one_cardinal() {
        let mut app = App::new();
        app.add_message::<AmbientGravityRequest>();
        app.init_resource::<BaseGravity>();
        app.add_systems(bevy::app::Update, apply_ambient_gravity_requests);
        app.world_mut().write_message(AmbientGravityRequest::Cycle);
        app.update();
        assert_eq!(
            app.world().resource::<BaseGravity>().direction_label(None),
            "Left",
            "down -> left after one request"
        );
        app.world_mut().write_message(AmbientGravityRequest::Cycle);
        app.world_mut().write_message(AmbientGravityRequest::Cycle);
        app.update();
        assert_eq!(
            app.world().resource::<BaseGravity>().direction_label(None),
            "Right",
            "two requests in one tick cycle twice, in write order"
        );
        app.update();
        assert_eq!(
            app.world().resource::<BaseGravity>().direction_label(None),
            "Right",
            "a tick with no request changes nothing"
        );
    }

    /// The developer turns the room the primary body is in, and no other.
    /// Two live rooms; the primary body stands in the second. Poison: cycle
    /// the sole live room (`None` here) and the second room stays down.
    #[test]
    fn a_cycle_request_turns_the_room_of_the_primary_body() {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            InRoomInstance, LiveRoomInstance, RoomInstanceRoot,
        };
        let mut app = App::new();
        app.add_message::<AmbientGravityRequest>();
        app.init_resource::<BaseGravity>();
        app.add_systems(bevy::app::Update, apply_ambient_gravity_requests);
        let first = LiveRoomInstance::ACTIVATION.next();
        let second = first.next();
        for room in [first, second] {
            app.world_mut().spawn((RoomInstanceRoot, room));
        }
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::body::PrimaryBody,
            InRoomInstance(second),
        ));
        app.world_mut().write_message(AmbientGravityRequest::Cycle);
        app.update();
        let base = app.world().resource::<BaseGravity>();
        assert_eq!(
            (
                base.direction_label(Some(first)),
                base.direction_label(Some(second))
            ),
            ("Down", "Left"),
            "(the first room, the primary body's room) after one request"
        );
    }
}
