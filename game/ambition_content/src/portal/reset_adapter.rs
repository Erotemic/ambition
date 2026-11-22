//! Ambition room-reset bridge: `RoomReplayAdmitted` → portal `ClearPortals`.
//!
//! Portal core's [`clear_portals_on_reset`](ambition_portal2d::clear_portals_on_reset)
//! consumes the portal-owned [`ClearPortals`](ambition_portal2d::ClearPortals) signal,
//! not the Ambition `RoomReplayAdmitted`. This bridge translates the Ambition
//! room-reset event into the portal signal so portal core never names the reset
//! event — the room-reset *policy* (when a room resets) stays Ambition's, while
//! portal owns the *clear-portals* mechanic.

use bevy::prelude::*;

use ambition_combat::events::RoomResetReason;
use ambition_portal2d::ClearPortals;

/// Emit a [`ClearPortals`] for a MANUAL room reset (the delete-key reset or a
/// scripted replay), but NOT for a player DEATH — so dying preserves the player's
/// gun-portal setup (and authored level portals are spared by
/// `clear_portals_on_reset` either way). Runs in `PortalSet::RoomReset` before
/// `clear_portals_on_reset`, so the clear happens the same frame the room reset
/// fires.
///
/// The clear is of the reset's room: the live room of the body the replay is
/// for, else the sole live room. A New Game (a manual checkpoint restore)
/// restarts the whole session and clears every room, as does a reset whose
/// room cannot be told.
pub fn bridge_room_reset_to_clear_portals(
    mut resets: ambition_combat::events::AdmittedReplays,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut clear: MessageWriter<ClearPortals>,
) {
    // A death (`PlayerDeath`) leaves every portal in place.
    let mut scopes: Vec<ClearPortals> = Vec::new();
    for reset in resets.read().filter(|r| r.reason == RoomResetReason::Manual) {
        let room = if reset.to_checkpoint { None } else { reset.subject.as_ref().and_then(|s| s.room).or_else(|| live.sole()) };
        let scope = ClearPortals { room };
        if !scopes.contains(&scope) {
            scopes.push(scope);
        }
    }
    for scope in scopes {
        clear.write(scope);
    }
}

#[cfg(test)]
mod tests {
    //! The bridge routes by reset reason: a MANUAL reset emits `ClearPortals`
    //! (clearing the gun pair downstream), a PLAYER-DEATH reset emits nothing.
    use super::*;
    use ambition_combat::events::RoomReplayAdmitted;

    fn clears_for(reason: RoomResetReason) -> bool {
        let mut app = App::new();
        app.add_message::<RoomReplayAdmitted>();
        app.add_message::<ClearPortals>();
        app.add_systems(Update, bridge_room_reset_to_clear_portals);
        app.world_mut()
            .write_message(RoomReplayAdmitted::because(reason));
        app.update();
        let mut reader = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ClearPortals>>();
        reader.drain().count() > 0
    }

    #[test]
    fn manual_reset_clears_but_death_does_not() {
        assert!(
            clears_for(RoomResetReason::Manual),
            "a manual reset must emit ClearPortals (clears the gun pair)"
        );
        assert!(
            !clears_for(RoomResetReason::PlayerDeath),
            "a death reset must NOT emit ClearPortals (gun portals survive a death)"
        );
    }

    /// P1: the clear names the room of the body the reset is for, so the
    /// other live room keeps its pair; a New Game names none.
    #[test]
    fn a_manual_reset_clears_the_room_of_its_subject() {
        use ambition_platformer2d_shared_tangle::lifecycle::{LiveBodyId, LiveRoomInstance, RoomInstanceRoot};
        use ambition_platformer2d_shared_tangle::sim_id::SimId;
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        let emitted = |replay: RoomReplayAdmitted| {
            let mut app = App::new();
            app.add_message::<RoomReplayAdmitted>();
            app.add_message::<ClearPortals>();
            app.add_systems(Update, bridge_room_reset_to_clear_portals);
            for room in rooms {
                app.world_mut().spawn((RoomInstanceRoot, room));
            }
            app.world_mut().write_message(replay);
            app.update();
            let mut messages = app.world_mut().resource_mut::<bevy::ecs::message::Messages<ClearPortals>>();
            messages.drain().collect::<Vec<_>>()
        };
        let mut retry = RoomReplayAdmitted::because(RoomResetReason::Manual);
        retry.subject = Some(LiveBodyId::new(SimId::placement("alice"), Some(rooms[1])));
        assert_eq!(emitted(retry), [ClearPortals::of(rooms[1])], "a retry clears its own room");
        let new_game = RoomReplayAdmitted::because(RoomResetReason::Manual).to_the_checkpoint();
        assert_eq!(emitted(new_game), [ClearPortals::EVERYWHERE], "a New Game clears every room");
    }
}
