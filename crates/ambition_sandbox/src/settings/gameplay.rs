//! Gameplay settings: difficulty, assist mode, trace auto-dump.
//!
//! These mostly read into resources consulted by feature systems
//! (encounter spawning, damage calculation, trace recorder). The
//! values are user-visible so they live in their own module rather
//! than under dev tools.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    #[default]
    Medium,
    Hard,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Medium, Self::Hard];

    pub fn label(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
        }
    }

    /// Multiplier applied to incoming player damage. Easy halves
    /// damage, hard doubles it. Mob lab and combat features consult
    /// this when applying damage to the player.
    pub fn damage_taken_multiplier(self) -> f32 {
        match self {
            Self::Easy => 0.5,
            Self::Medium => 1.0,
            Self::Hard => 2.0,
        }
    }

    /// Multiplier applied to enemy spawn counts in the mob lab. Easy
    /// reduces wave size; hard increases it.
    pub fn spawn_count_multiplier(self) -> f32 {
        match self {
            Self::Easy => 0.7,
            Self::Medium => 1.0,
            Self::Hard => 1.4,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Easy => Self::Medium,
            Self::Medium => Self::Hard,
            Self::Hard => Self::Easy,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Easy => Self::Hard,
            Self::Medium => Self::Easy,
            Self::Hard => Self::Medium,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistMode {
    #[default]
    Off,
    On,
}

impl AssistMode {
    pub const ALL: [Self; 2] = [Self::Off, Self::On];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::On => "on",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Off => Self::On,
            Self::On => Self::Off,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameplaySettings {
    pub difficulty: Difficulty,
    pub assist: AssistMode,
    /// Multiplier for outgoing player damage (projectiles, melee).
    pub player_damage_multiplier: f32,
    /// Whether the trace recorder dumps automatically on OOB / death.
    pub trace_auto_dump: bool,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            difficulty: Difficulty::default(),
            assist: AssistMode::default(),
            player_damage_multiplier: 1.0,
            trace_auto_dump: true,
        }
    }
}

impl GameplaySettings {
    pub const DAMAGE_STEP: f32 = 0.10;

    pub fn nudge_player_damage(&mut self, delta: f32) {
        self.player_damage_multiplier = (self.player_damage_multiplier + delta).clamp(0.25, 4.0);
    }

    pub fn clamp_all(&mut self) {
        self.player_damage_multiplier = self.player_damage_multiplier.clamp(0.25, 4.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_multipliers_are_distinct() {
        let easy = Difficulty::Easy.damage_taken_multiplier();
        let med = Difficulty::Medium.damage_taken_multiplier();
        let hard = Difficulty::Hard.damage_taken_multiplier();
        assert!(easy < med);
        assert!(med < hard);
    }

    #[test]
    fn damage_multiplier_clamps() {
        let mut s = GameplaySettings::default();
        for _ in 0..100 {
            s.nudge_player_damage(1.0);
        }
        assert!(s.player_damage_multiplier <= 4.0 + 1e-6);
        for _ in 0..100 {
            s.nudge_player_damage(-1.0);
        }
        assert!(s.player_damage_multiplier >= 0.25 - 1e-6);
    }

    #[test]
    fn assist_toggles() {
        let on = AssistMode::Off.toggle();
        assert_eq!(on, AssistMode::On);
        assert_eq!(on.toggle(), AssistMode::Off);
    }
}
