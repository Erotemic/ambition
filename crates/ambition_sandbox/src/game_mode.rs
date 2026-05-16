//! Coarse sandbox/gameplay mode state.
//!
//! `GameMode` is intentionally broader than per-entity behavior. Use it to
//! decide which groups of systems are allowed to mutate gameplay state. Enemy,
//! chest, boss, and dialogue state machines can be added later on top of this
//! coarse mode instead of teaching every mechanic how to pause itself.

use bevy::prelude::*;

/// Bevy run condition: returns `true` only in `GameMode::Playing`.
/// Use this to gate simulation systems that must not run while paused,
/// in dialogue, in a room transition, or in a cutscene.
///
/// ```
/// app.add_systems(Update, my_system.run_if(gameplay_allowed));
/// ```
pub fn gameplay_allowed(mode: Res<State<GameMode>>) -> bool {
    mode.get().allows_gameplay()
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_gameplay_only_in_playing() {
        assert!(GameMode::Playing.allows_gameplay());
        assert!(!GameMode::Paused.allows_gameplay());
        assert!(!GameMode::Dialogue.allows_gameplay());
        assert!(!GameMode::RoomTransition.allows_gameplay());
        assert!(!GameMode::Cutscene.allows_gameplay());
    }

    #[test]
    fn default_is_playing() {
        assert_eq!(GameMode::default(), GameMode::Playing);
    }

    #[test]
    fn labels_are_unique_and_non_empty() {
        let labels = [
            GameMode::Playing.label(),
            GameMode::Paused.label(),
            GameMode::Dialogue.label(),
            GameMode::RoomTransition.label(),
            GameMode::Cutscene.label(),
        ];
        for label in labels {
            assert!(!label.is_empty());
        }
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len(), "labels must be unique");
    }
}
