//! The state of a two-state room: pure, balanced or corrupt.
//!
//! A room with the `clean_corrupted` look shows one architecture in two
//! states, and a front between them. This module is the simulation half of
//! where that front is: a fact in the save, and the authored verb that
//! changes it. The presentation half (`presentation::room_look`) reads the
//! fact and moves the front across the room.
//!
//! The fact is two ordinary world flags for each room, so it is saved, rolled
//! back and mirrored to quests as every flag is:
//!
//! | `look.<room>.pure` | `look.<room>.corrupt` | state |
//! |---|---|---|
//! | off | off | balanced (the default: a room nobody touched) |
//! | on | off | pure |
//! | off | on | corrupt |
//!
//! `look.cycle(room)` is the verb: balanced, corrupt, pure, balanced. A
//! `Switch` authors it in `on_activate`.

use ambition_persistence::save::AmbitionGameSave;
use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, CommandDescriptor, CommandId, CommandOutcome, ParamKind, ParamSpec, PublishCommand,
};
use bevy::prelude::{App, Plugin, World};

/// The authored-command domain of room looks.
pub const DOMAIN: &str = "look";

/// The state of a two-state room.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoomLookState {
    /// The whole room is clean.
    Pure,
    /// The front goes through the room.
    #[default]
    Balanced,
    /// The whole room is corrupted.
    Corrupt,
}

impl RoomLookState {
    /// The state the two flags of a room say. Both flags on is not a state a
    /// writer here makes; it reads as corrupt, the later of the two.
    pub fn from_flags(pure: bool, corrupt: bool) -> Self {
        match (pure, corrupt) {
            (_, true) => Self::Corrupt,
            (true, false) => Self::Pure,
            (false, false) => Self::Balanced,
        }
    }

    /// The state a press of the switch asks for next.
    pub fn next(self) -> Self {
        match self {
            Self::Balanced => Self::Corrupt,
            Self::Corrupt => Self::Pure,
            Self::Pure => Self::Balanced,
        }
    }

    /// How corrupted the room is: -1.0 (pure), 0.0 or 1.0 (corrupt).
    pub fn level(self) -> f32 {
        match self {
            Self::Pure => -1.0,
            Self::Balanced => 0.0,
            Self::Corrupt => 1.0,
        }
    }

    /// The state of `room` in `save`.
    pub fn of(save: &AmbitionGameSave, room: &str) -> Self {
        let data = save.data();
        Self::from_flags(data.flag(&pure_flag(room)), data.flag(&corrupt_flag(room)))
    }
}

/// The world flag that is on while `room` is pure.
pub fn pure_flag(room: &str) -> String {
    format!("look.{room}.pure")
}

/// The world flag that is on while `room` is corrupt.
pub fn corrupt_flag(room: &str) -> String {
    format!("look.{room}.corrupt")
}

const ROOM: ParamSpec = ParamSpec {
    name: "room",
    kind: ParamKind::Name,
    summary: "the id of the room whose state changes",
};

/// `look.cycle(room)`: the next state of a two-state room.
pub fn cycle_descriptor() -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new(DOMAIN, "cycle"),
        summary: "move a two-state room to its next state: balanced, corrupt, pure",
        params: &[ROOM],
    }
}

/// `look.cycle`: see [`cycle_descriptor`].
///
/// It writes the world-fact domain's own request (`SetFlagRequested`), as
/// `world.set_flag` does, and for the same reason: that request is cleared on
/// a rollback, applied in `GameplayEffects`, and mirrored to quests. A write
/// to the save from here would be a second road for a flag.
pub fn cycle(world: &mut World, args: &[AuthoredArg]) -> CommandOutcome {
    let Some(room) = args[0].as_name() else {
        return CommandOutcome::refused("`room` must be a name");
    };
    let Some(save) = world.get_resource::<AmbitionGameSave>() else {
        return CommandOutcome::refused(
            "no save layer is installed in this composition, so the room has no state",
        );
    };
    if !world.contains_resource::<
        bevy::ecs::message::Messages<ambition_combat::events::SetFlagRequested>,
    >() {
        return CommandOutcome::refused(
            "no world-fact channel is installed in this composition, so nothing \
             would ever apply the state",
        );
    }
    let next = RoomLookState::of(save, room).next();
    let (pure, corrupt) = (pure_flag(room), corrupt_flag(room));
    world.write_message(ambition_combat::events::SetFlagRequested {
        id: pure,
        on: next == RoomLookState::Pure,
    });
    world.write_message(ambition_combat::events::SetFlagRequested {
        id: corrupt,
        on: next == RoomLookState::Corrupt,
    });
    CommandOutcome::Done
}

/// Publishes the room-look domain's verb. It is a simulation plugin: a
/// headless composition prepares the same authored `on_activate` lines.
pub struct RoomLookStatePlugin;

impl Plugin for RoomLookStatePlugin {
    fn build(&self, app: &mut App) {
        app.publish_command(cycle_descriptor(), cycle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_room_nobody_touched_is_balanced_and_three_presses_come_back_to_it() {
        let start = RoomLookState::from_flags(false, false);
        assert_eq!(start, RoomLookState::Balanced);
        assert_eq!(start.next(), RoomLookState::Corrupt);
        assert_eq!(start.next().next(), RoomLookState::Pure);
        assert_eq!(start.next().next().next(), start);
    }

    #[test]
    fn each_state_is_one_pair_of_flags() {
        for state in [RoomLookState::Pure, RoomLookState::Balanced, RoomLookState::Corrupt] {
            let pair = (state == RoomLookState::Pure, state == RoomLookState::Corrupt);
            assert_eq!(RoomLookState::from_flags(pair.0, pair.1), state);
        }
    }
}
