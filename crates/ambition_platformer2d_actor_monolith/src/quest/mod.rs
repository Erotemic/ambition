//! Gameplay-core adapter for the generic quest runtime.
//!
//! Quest data, events, registry, and save mirroring live in
//! `ambition_persistence::quest`, and every consumer names that crate
//! directly. The only piece here is the room-specific producer that
//! translates the active `RoomSet` into a generic `RoomEntered` quest event —
//! it lives in this crate because `RoomSet` does.

use bevy::prelude::*;

/// Push a `RoomEntered` quest event whenever the active room changes.
/// Idempotent: only fires the frame the room id flips.
///
/// The memory of the previous room is [`LastQuestRoom`], rollback state — not a
/// `Local`, which a rewind does not touch and which therefore let a
/// resimulation skip the push (S2 in the determinism plan).
///
/// [`LastQuestRoom`]: ambition_persistence::quest::LastQuestRoom
pub fn push_room_entered_quest_events(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ambition_platformer2d_world::rooms::RoomSet>,
    mut registry: ResMut<ambition_persistence::quest::QuestRegistry>,
    mut last_room: ResMut<ambition_persistence::quest::LastQuestRoom>,
) {
    let current = room_set.active_spec().id.clone();
    // Read through the immutable deref: on every frame but the flip there is
    // nothing to write, and a `DerefMut` would mark the resource changed.
    if last_room.0.as_deref() == Some(current.as_str()) {
        return;
    }
    last_room.0 = Some(current.clone());
    registry.push_event(ambition_persistence::quest::QuestAdvanceEvent::RoomEntered(
        current,
    ));
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
            RoomSet::from_parts(room_id, vec![room(room_id)], Vec::new()),
        ));
        app
    }

    fn room_entered_pushes(app: &mut App) -> usize {
        app.world()
            .resource::<QuestRegistry>()
            .pending_events
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
            app.world().resource::<LastQuestRoom>().0.as_deref(),
            Some("hall"),
            "the memory is the resource"
        );

        // A rollback restores the resource to its pre-flip value.
        app.world_mut().resource_mut::<LastQuestRoom>().0 = None;
        app.update();
        assert_eq!(
            room_entered_pushes(&mut app),
            2,
            "after the memory rewinds, the resimulation pushes RoomEntered again"
        );
    }
}
