//! Runtime schedule vocabulary that is independent of Ambition content.
//!
//! `SandboxSet` remains the concrete app schedule for now. These labels document
//! the future crate-level concepts and give new runtime modules names that do
//! not depend on app assembly details.

use bevy::prelude::*;

/// Generic platformer runtime phases.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PlatformerRuntimeSet {
    /// Build or refresh world-derived runtime inputs before actors tick.
    WorldPrep,
    /// Translate input/control intent into actor control frames.
    ControlInput,
    /// Integrate actors, held items, projectiles, and other gameplay bodies.
    ActorSimulation,
    /// Handle room unload/load, room-scoped cleanup, and authored room respawn.
    RoomLifecycle,
    /// Resolve damage, hitboxes, combat intents, and gameplay consequences.
    Combat,
    /// Publish simulation state to presentation-facing mirrors/caches.
    PresentationSync,
}

/// Bevy run condition: returns `true` only in [`GameMode::Playing`].
///
/// Use this to gate simulation systems that must not run while paused,
/// in dialogue, in a room transition, or in a cutscene.
///
/// ```ignore
/// app.add_systems(Update, my_system.run_if(gameplay_allowed));
/// ```
pub fn gameplay_allowed(mode: Res<State<GameMode>>) -> bool {
    mode.get().allows_gameplay()
}

/// Bevy run condition: complement of [`gameplay_allowed`]. True in any mode
/// that suspends gameplay (paused, dialogue, room transition, cutscene).
///
/// Use this to gate the small set of systems that should only run while
/// gameplay is suspended, such as forcing world time to zero.
pub fn gameplay_suspended(mode: Res<State<GameMode>>) -> bool {
    !mode.get().allows_gameplay()
}

/// Coarse gameplay/session mode shared by runtime, input, host, and render.
///
/// `GameMode` is intentionally broader than per-entity behavior. It belongs
/// with the schedule vocabulary because it answers the same question as the
/// runtime sets: which groups of systems may mutate gameplay state this frame?
/// Enemy, chest, boss, and dialogue state machines can layer narrower state on
/// top of this coarse mode without teaching every mechanic how to pause itself.
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Reflect)]
pub enum GameMode {
    /// Normal gameplay: controlled actors, NPCs, enemies, hazards, room
    /// triggers, and pickups may consume gameplay inputs and advance
    /// simulation time.
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
    fn gameplay_suspended_is_complement_of_allowed() {
        for mode in [
            GameMode::Playing,
            GameMode::Paused,
            GameMode::Dialogue,
            GameMode::RoomTransition,
            GameMode::Cutscene,
        ] {
            assert_eq!(mode.allows_gameplay(), !gameplay_suspended_for_value(mode));
        }
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

    fn gameplay_suspended_for_value(mode: GameMode) -> bool {
        !mode.allows_gameplay()
    }
}
