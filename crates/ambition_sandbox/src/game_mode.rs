//! Coarse sandbox/gameplay mode state.
//!
//! `GameMode` is intentionally broader than per-entity behavior. Use it to
//! decide which groups of systems are allowed to mutate gameplay state. Enemy,
//! chest, boss, and dialogue state machines can be added later on top of this
//! coarse mode instead of teaching every mechanic how to pause itself.

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Reflect)]
pub enum GameMode {
    /// Normal gameplay: player, enemies, hazards, room triggers, and pickups
    /// may consume gameplay inputs and advance simulation time.
    #[default]
    Playing,
    /// Simulation is stopped, but pause/menu input and developer tools remain
    /// responsive. Gameplay actions are deliberately not converted into an
    /// engine `InputState` while this mode is active.
    Paused,
    /// Reserved for NPC conversations and other text-driven interactions.
    Dialogue,
    /// Reserved for scripted room loads or door/edge transition presentation.
    RoomTransition,
    /// Reserved for future cutscenes or scripted set pieces.
    Cutscene,
}

impl GameMode {
    pub fn allows_gameplay(self) -> bool {
        matches!(self, Self::Playing)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Dialogue => "dialogue",
            Self::RoomTransition => "room-transition",
            Self::Cutscene => "cutscene",
        }
    }
}
