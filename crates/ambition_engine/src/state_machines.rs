//! Entity-local state machine vocabulary for Ambition.
//!
//! Bevy `States` are still the right tool for app-wide modes such as loading,
//! pause, dialogue, room transitions, and cutscenes. This module is narrower:
//! it introduces component states for individual entities whose behavior changes
//! over time, and wires in `seldom_state` as the reusable state-machine runner.
//!
//! The first pass is intentionally conservative. The existing sandbox feature
//! runtime can continue to drive prototype hazards and actors while future enemy,
//! boss, chest, and breakable systems migrate one family at a time.

use bevy::prelude::*;
use seldom_state::prelude::StateMachinePlugin;

/// Registers the third-party `seldom_state` state-machine runner.
///
/// Add this plugin once in Bevy-facing crates that spawn Ambition entities with
/// `seldom_state::prelude::StateMachine` components. Engine state components are
/// defined below so game/story/sandbox crates share the same vocabulary.
#[derive(Debug, Default)]
pub struct AmbitionStateMachinePlugin;

impl Plugin for AmbitionStateMachinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(StateMachinePlugin::default());
    }
}

/// Marker for entities whose behavior is intended to be owned by an Ambition
/// entity-local state machine rather than a one-off sandbox loop.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AmbitionStateMachineActor;

/// Enemy is inactive until the player enters its room/arena or aggro region.
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[component(storage = "SparseSet")]
pub struct EnemyIdle;

/// Enemy follows a path, paces in a leash area, or otherwise performs a default
/// low-threat movement pattern.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct EnemyPatrol {
    pub speed: f32,
    pub path_id: Option<String>,
}

impl Default for EnemyPatrol {
    fn default() -> Self {
        Self {
            speed: 80.0,
            path_id: None,
        }
    }
}

/// Enemy is warning the player before committing to an attack.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct EnemyTelegraph {
    pub attack_id: String,
    pub remaining: f32,
}

/// Enemy is actively attacking. The attack shape/timing remains data-driven;
/// this state records the current scripted action and timer.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct EnemyAttack {
    pub attack_id: String,
    pub remaining: f32,
}

/// Enemy has finished an attack and is vulnerable or waiting before returning
/// to idle/patrol.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct EnemyRecover {
    pub remaining: f32,
}

/// Enemy is briefly disabled by damage, pogo, or story rules.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct EnemyStunned {
    pub remaining: f32,
}

/// Enemy is dead/removed from active behavior until respawn or room reload.
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[component(storage = "SparseSet")]
pub struct EnemyDead;

/// Boss exists but has not begun its encounter.
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[component(storage = "SparseSet")]
pub struct BossDormant;

/// Boss intro/title-card/roar state.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct BossIntro {
    pub remaining: f32,
}

/// Generic numbered boss phase. Pattern data chooses the actual attacks.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct BossPhase {
    pub phase: u8,
    pub pattern_id: String,
}

/// Boss has been defeated but may still be playing outro/reward logic.
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[component(storage = "SparseSet")]
pub struct BossDefeated;

/// Chest has not been opened.
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[component(storage = "SparseSet")]
pub struct ChestClosed;

/// Chest is playing an opening animation / reward grant window.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct ChestOpening {
    pub remaining: f32,
}

/// Chest has already granted its reward.
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[component(storage = "SparseSet")]
pub struct ChestOpened;

/// Breakable is intact and should participate in gameplay/collision if solid.
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[component(storage = "SparseSet")]
pub struct BreakableIntact;

/// Breakable is showing pre-break feedback.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct BreakableCracking {
    pub remaining: f32,
}

/// Breakable is broken and either gone or non-solid.
#[derive(Component, Clone, Debug, Default, PartialEq)]
#[component(storage = "SparseSet")]
pub struct BreakableBroken;

/// Breakable is waiting for its respawn timer.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct BreakableRespawning {
    pub remaining: f32,
}

/// Encounter (e.g. mob lab) is loaded but the player has not entered the
/// trigger yet.
///
/// The encounter family of states is designed so a single encounter
/// controller entity can carry the state machine while a global
/// `EncounterRegistry` tracks save-game persistence per encounter id.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
#[component(storage = "SparseSet")]
pub struct EncounterDormant;

/// Encounter trigger is firing: brief intro window before the first wave
/// spawns. Used for camera zoom ramp-out and any pre-encounter telegraph.
#[derive(Component, Clone, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct EncounterStarting {
    pub remaining: f32,
}

/// Encounter is in progress. Wave index and per-wave remaining mob count
/// are stored on the component so a save / debug overlay can read the
/// canonical state directly from the controller entity.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
#[component(storage = "SparseSet")]
pub struct EncounterActive {
    pub wave_index: u8,
    pub remaining_mobs: u8,
    pub total_waves: u8,
}

/// Encounter has been cleared. The controller stays in this state until
/// the player triggers a reset (e.g. a switch outside the room).
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
#[component(storage = "SparseSet")]
pub struct EncounterCleared;

/// Player died inside the encounter. Same lifetime as `EncounterCleared`:
/// stays Failed until reset, at which point a fresh attempt becomes
/// available.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
#[component(storage = "SparseSet")]
pub struct EncounterFailed;

/// A latched, player-facing on/off switch that persists across save loads.
/// The runtime sandbox uses this to drive encounter resets, but the
/// vocabulary is intentionally generic so future puzzles / doors / doors
/// can layer on top.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
#[component(storage = "SparseSet")]
pub struct SwitchOff {
    pub id: String,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
#[component(storage = "SparseSet")]
pub struct SwitchOn {
    pub id: String,
}

/// Stable list used by docs and snapshot tests to catch accidental renames of
/// the public state-machine vocabulary.
pub fn state_machine_vocabulary() -> &'static [&'static str] {
    &[
        "AmbitionStateMachineActor",
        "EnemyIdle",
        "EnemyPatrol",
        "EnemyTelegraph",
        "EnemyAttack",
        "EnemyRecover",
        "EnemyStunned",
        "EnemyDead",
        "BossDormant",
        "BossIntro",
        "BossPhase",
        "BossDefeated",
        "ChestClosed",
        "ChestOpening",
        "ChestOpened",
        "BreakableIntact",
        "BreakableCracking",
        "BreakableBroken",
        "BreakableRespawning",
        "EncounterDormant",
        "EncounterStarting",
        "EncounterActive",
        "EncounterCleared",
        "EncounterFailed",
        "SwitchOff",
        "SwitchOn",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_contains_the_core_entity_families() {
        let names = state_machine_vocabulary();
        assert!(names.iter().any(|name| name.starts_with("Enemy")));
        assert!(names.iter().any(|name| name.starts_with("Boss")));
        assert!(names.iter().any(|name| name.starts_with("Chest")));
        assert!(names.iter().any(|name| name.starts_with("Breakable")));
    }
}
