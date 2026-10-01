//! Gameplay-core adapter for the generic quest runtime.
//!
//! Quest data, events, registry, and save mirroring live in
//! `ambition_persistence::quest`, and every consumer names that crate
//! directly. The only piece here is the room-specific producer that
//! translates the active `RoomSet` into a generic `RoomEntered` quest event —
//! it lives in this crate because `RoomSet` does.

use bevy::prelude::*;

/// Push a `RoomEntered` quest event for each room id that becomes live.
/// Idempotent: only fires the frame the id becomes live.
///
/// Every live room, not the sole one (OW1 cut 7k): with Alice and Bob in two
/// live rooms, the room each one enters is entered. A room id that is live
/// already is not entered again when a second player joins it. With no live
/// room (between sessions) the memory is kept, so the room that comes back
/// is not announced twice.
///
/// The memory of the previous room is [`LastQuestRoom`], rollback state — not a
/// `Local`, which a rewind does not touch and which therefore let a
/// resimulation skip the push (S2 in the determinism plan).
///
/// [`LastQuestRoom`]: ambition_persistence::quest::LastQuestRoom
pub fn push_room_entered_quest_events(
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut registry: ResMut<ambition_persistence::quest::QuestRegistry>,
    mut last_rooms: ResMut<ambition_persistence::quest::LastQuestRoom>,
) {
    let mut live: Vec<String> = rooms
        .live_rooms()
        .map(|(_, definition)| rooms.rooms().spec(definition).id.clone())
        .collect();
    if live.is_empty() {
        return;
    }
    live.sort();
    live.dedup();
    // Read through the immutable deref: on every frame but a change there is
    // nothing to write, and a `DerefMut` would mark the resource changed.
    if last_rooms.0 == live {
        return;
    }
    // In sorted order, so a resimulation pushes the same events in the same
    // order.
    for room in live.iter().filter(|room| !last_rooms.0.contains(room)) {
        registry.push_event(ambition_persistence::quest::QuestAdvanceEvent::RoomEntered(
            room.clone(),
        ));
    }
    last_rooms.0 = live;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_persistence::quest::{LastQuestRoom, QuestAdvanceEvent, QuestRegistry};
    use ambition_platformer2d_shared_tangle::lifecycle::{SessionRoot, SessionScopeId};
    use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};

    fn room(id: &str) -> RoomSpec {
        RoomSpec::new(
            id,
            ambition_platformer2d_core::World::new(
                id,
                ambition_platformer2d_core::Vec2::new(640.0, 480.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        )
    }

    fn app_in(room_id: &str) -> App {
        let mut app = App::new();
        app.init_resource::<QuestRegistry>()
            .init_resource::<LastQuestRoom>()
            .add_systems(Update, push_room_entered_quest_events);
        app.world_mut().spawn((
            SessionRoot(SessionScopeId(1)),
            RoomSet::from_parts_or_panic(room_id, vec![room(room_id)], Vec::new()),
        ));
        ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), room_id)
            .expect("the fixture set holds its room");
        app
    }

    fn room_entered_pushes(app: &mut App) -> usize {
        app.world()
            .resource::<QuestRegistry>()
            .pending_events()
            .iter()
            .filter(|event| matches!(event, QuestAdvanceEvent::RoomEntered(_)))
            .count()
    }

    /// The producer's memory is the RESOURCE, so restoring the resource
    /// restores the producer's behaviour. A rewind hands the world back a
    /// `LastQuestRoom` from before the room flip; the producer must then push
    /// `RoomEntered` again on resimulation. With a `Local` the memory survives
    /// the restore and the push is skipped — this test is red with the `Local`
    /// put back.
    #[test]
    fn restoring_the_last_room_makes_the_producer_announce_the_room_again() {
        let mut app = app_in("hall");
        app.update();
        assert_eq!(room_entered_pushes(&mut app), 1, "the first frame announces the room");
        app.update();
        assert_eq!(room_entered_pushes(&mut app), 1, "an unchanged room is not re-announced");
        assert_eq!(
            app.world().resource::<LastQuestRoom>().0,
            vec!["hall".to_owned()],
            "the memory is the resource"
        );

        // A rollback restores the resource to its pre-flip value.
        app.world_mut().resource_mut::<LastQuestRoom>().0 = Vec::new();
        app.update();
        assert_eq!(
            room_entered_pushes(&mut app),
            2,
            "after the memory rewinds, the resimulation pushes RoomEntered again"
        );
    }

    /// OW1 cut 7k: with two live rooms, the room each player enters is
    /// entered. Alice is in `hall`; Bob's `cellar` becomes live beside it, and
    /// it is announced; a second live room of `hall` is not `hall` entered
    /// again. When the producer read the sole live room, it did not run while
    /// two rooms were live, and `cellar` was never entered.
    #[test]
    fn a_room_that_becomes_live_beside_another_is_entered() {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            session_world_component, LiveRoomInstance, RoomInstanceRoot,
        };
        let mut app = App::new();
        app.init_resource::<QuestRegistry>()
            .init_resource::<LastQuestRoom>()
            .add_systems(Update, push_room_entered_quest_events);
        app.world_mut().spawn((
            SessionRoot(SessionScopeId(1)),
            RoomSet::from_parts_or_panic("hall", vec![room("hall"), room("cellar")], Vec::new()),
        ));
        ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), "hall")
            .expect("the fixture set holds its room");
        let definition = |app: &App, id: &str| {
            session_world_component::<RoomSet>(app.world())
                .and_then(|rooms| rooms.definition_by_id(id))
                .expect("the fixture set holds the room")
        };
        let entered = |app: &App| -> Vec<String> {
            app.world()
                .resource::<QuestRegistry>()
                .pending_events()
                .iter()
                .filter_map(|event| match event {
                    QuestAdvanceEvent::RoomEntered(room) => Some(room.clone()),
                    _ => None,
                })
                .collect()
        };
        app.update();
        let first = *ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<LiveRoomInstance>(
            app.world(),
        )
        .expect("the hall is live");
        let cellar = definition(&app, "cellar");
        app.world_mut().spawn((RoomInstanceRoot, first.next(), cellar));
        app.update();
        let hall = definition(&app, "hall");
        app.world_mut().spawn((RoomInstanceRoot, first.next().next(), hall));
        app.update();
        assert_eq!(
            (entered(&app), app.world().resource::<LastQuestRoom>().0.clone()),
            (
                vec!["hall".to_owned(), "cellar".to_owned()],
                vec!["cellar".to_owned(), "hall".to_owned()],
            ),
            "(rooms entered, the memory): each room id is entered once, as it becomes live"
        );
    }
}
