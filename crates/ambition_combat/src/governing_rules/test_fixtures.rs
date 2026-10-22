//! A session with two games' rooms live, for the witnesses of the per-room
//! rule readers here and in the crates that read rules per room.

use bevy::prelude::*;
use ambition_platformer2d_shared_tangle::lifecycle::{
    session_world_component, LiveRoomInstance, RoomInstanceRoot, SessionRoot, SessionScopeId,
};
use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};

/// The hall's live room and the stage's.
pub type TwoRooms = (LiveRoomInstance, LiveRoomInstance);

/// A session in `app` whose hall (no mode) is live, and with `stage` (mode
/// `smash`) live beside it when `two`. The hall's live room is returned; the
/// stage's is the next. Shared by the per-room rule readers' witnesses.
pub fn two_game_session(app: &mut App, two: bool) -> LiveRoomInstance {
    fn room(id: &str, mode: Option<&str>) -> RoomSpec {
        let mut room = RoomSpec::new(
            id,
            ambition_platformer2d_core::World::new(
                id,
                ambition_platformer2d_core::Vec2::new(320.0, 240.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        room.metadata.mode = mode.map(str::to_owned);
        room
    }
    app.world_mut().spawn((
        SessionRoot(SessionScopeId(1)),
        RoomSet::from_parts_or_panic(
            "hall",
            vec![room("hall", None), room("stage", Some("smash"))],
            Vec::new(),
        ),
    ));
    ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), "hall")
        .expect("the fixture set holds its room");
    let first = *ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<
        LiveRoomInstance,
    >(app.world())
    .expect("the hall is live");
    if two {
        let stage = session_world_component::<RoomSet>(app.world())
            .and_then(|rooms| rooms.definition_by_id("stage"))
            .expect("the fixture set holds the stage");
        app.world_mut().spawn((RoomInstanceRoot, first.next(), stage));
    }
    first
}
