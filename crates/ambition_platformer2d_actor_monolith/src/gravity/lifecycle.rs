//! Gravity-zone lifecycle and room-reset policy. Extracted from
//! `ambition_portal2d::lifecycle` (Stage 6 follow-up): this is a *gravity
//! mechanic*, not portal behavior, so it owns its state here and must not
//! depend on `ambition_portal2d`.

use bevy::prelude::*;


/// Reset the AMBIENT gravity to the default (down) when a new world begins, so a
/// flipped room doesn't carry over. Only the ambient is authored state; the
/// presentation `GravityField` is a per-tick mirror of the primary body's
/// resolved frame with exactly one writer (`resolve_active_gravity`), so it
/// follows on the next resolution.
///
/// ⭐ ONE FACT COVERS A DEATH, A RETRY AND A NEW GAME. A 2026-09-13 review found
/// that *"flip gravity, Reset New Game"* started the fresh run upside down,
/// because a New Game had its own commit message and nothing here read it.
/// Since 2026-09-29 a New Game is a checkpoint restore, and its admission
/// writes `RoomReplayAdmitted` like a death does, so this one reader serves all
/// three. ⚠ The SESSION edge is not here: `BaseGravity` is a member of
/// `SessionScopedResources`, so activation and retirement clear it with the rest
/// of the live-session mirrors and under that aggregate's compiler guard.
///
/// ⭐ ONLY THE ROOM REPLAYED (customer 2). Each live room has its own ambient,
/// and a replay rebuilds one player's room: the ambient of that room goes back
/// down, and another live room keeps its own. A replay with nobody in it
/// names no room, and resets the sole live room.
pub fn reset_gravity_on_room_reset(
    mut resets: MessageReader<ambition_combat::events::RoomReplayAdmitted>,
    mut base: ResMut<ambition_platformer2d_shared_tangle::gravity::BaseGravity>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    for reset in resets.read() {
        let room = reset.subject.as_ref().and_then(|subject| subject.room).or_else(|| live.sole());
        base.forget(room);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::gravity::BaseGravity;

    /// A replay admission puts gravity back down. The first update, with no
    /// message, is the control: the flip must survive it.
    #[test]
    fn a_replay_admission_puts_gravity_back_down() {
        let mut app = App::new();
        app.add_message::<ambition_combat::events::RoomReplayAdmitted>();
        app.init_resource::<BaseGravity>();
        app.add_systems(Update, reset_gravity_on_room_reset);

        let flipped = ambition_platformer2d_core::Vec2::new(0.0, -1.0);
        app.world_mut().insert_resource(BaseGravity::in_room(None, flipped));
        app.update();
        assert_eq!(
            app.world().resource::<BaseGravity>().dir_in(None),
            flipped,
            "gravity reset with no lifecycle message at all"
        );

        app.world_mut()
            .write_message(ambition_combat::events::RoomReplayAdmitted::manual());
        app.update();
        assert_eq!(
            app.world().resource::<BaseGravity>().dir_in(None),
            BaseGravity::default().dir_in(None),
            "a replay left the ambient gravity flipped, so the new world starts \
             upside down"
        );
    }

    /// A replay resets only the ambient of the room it replays. Two live
    /// rooms, both turned up; Alice's replay names the second. The first
    /// keeps its gravity. Poison: forget every room and the first is put
    /// down too.
    #[test]
    fn a_replay_puts_down_only_the_gravity_of_the_room_it_replays() {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            LiveBodyId, LiveRoomInstance, RoomInstanceRoot,
        };
        let first = LiveRoomInstance::ACTIVATION.next();
        let second = first.next();
        let up = ambition_platformer2d_core::Vec2::new(0.0, -1.0);
        let mut app = App::new();
        app.add_message::<ambition_combat::events::RoomReplayAdmitted>();
        let mut base = BaseGravity::in_room(Some(first), up);
        base.turn(Some(second), up);
        app.insert_resource(base);
        for room in [first, second] {
            app.world_mut().spawn((RoomInstanceRoot, room));
        }
        app.add_systems(Update, reset_gravity_on_room_reset);
        let alice = LiveBodyId::new(
            ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
            Some(second),
        );
        app.world_mut().write_message(
            ambition_combat::events::RoomReplayAdmitted::manual().for_subject(alice),
        );
        app.update();
        let base = app.world().resource::<BaseGravity>();
        assert_eq!(
            (base.dir_in(Some(first)), base.dir_in(Some(second))),
            (up, BaseGravity::default().dir_in(None)),
            "(the room not replayed, the room replayed)"
        );
    }
}
