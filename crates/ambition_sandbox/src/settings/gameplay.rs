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
    ///
    /// _Status_: defined but not yet wired into the encounter spawn
    /// path. mob_lab waves are authored with specific positions /
    /// brain types so a naive `count *= multiplier` would either
    /// drop authored mobs or duplicate them on top of each other.
    /// Future wiring options: scale per-wave delay, or randomly
    /// drop/clone mobs during encounter compose. Tracked under
    /// "More enemy varieties" / mob_lab tier-A items.
    #[allow(dead_code)]
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
    /// Whether the large debug HUD text panel is visible.
    ///
    /// Desktop defaults to visible to preserve the sandbox-first dev posture.
    /// Android defaults to hidden so the phone viewport starts usable.
    #[serde(default = "default_debug_hud_visible")]
    pub debug_hud_visible: bool,
    /// Whether the dedicated quest objective panel is visible.
    #[serde(default = "default_quest_hud_visible")]
    pub quest_hud_visible: bool,
    /// Whether the trace recorder dumps automatically on OOB / death.
    pub trace_auto_dump: bool,
}

fn default_debug_hud_visible() -> bool {
    !cfg!(target_os = "android")
}

fn default_quest_hud_visible() -> bool {
    true
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            difficulty: Difficulty::default(),
            assist: AssistMode::default(),
            player_damage_multiplier: 1.0,
            debug_hud_visible: default_debug_hud_visible(),
            quest_hud_visible: default_quest_hud_visible(),
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

    #[test]
    fn hud_settings_have_expected_defaults() {
        let settings = GameplaySettings::default();
        assert_eq!(settings.debug_hud_visible, !cfg!(target_os = "android"));
        assert!(settings.quest_hud_visible);
    }

    #[test]
    fn difficulty_next_prev_round_trip() {
        for d in Difficulty::ALL {
            assert_eq!(d.next().prev(), d);
        }
    }

    #[test]
    fn difficulty_labels_are_unique_and_non_empty() {
        let labels: Vec<&str> = Difficulty::ALL.iter().map(|d| d.label()).collect();
        for label in &labels {
            assert!(!label.is_empty());
        }
        // ALL is small enough to do an O(n²) uniqueness check.
        for (i, a) in labels.iter().enumerate() {
            for b in &labels[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn gameplay_settings_default_values_are_sensible() {
        let s = GameplaySettings::default();
        assert_eq!(s.difficulty, Difficulty::Medium);
        assert_eq!(s.assist, AssistMode::Off);
        assert!(
            (s.player_damage_multiplier - 1.0).abs() < 1e-6,
            "default damage multiplier should be 1.0 (no scaling)"
        );
        assert!(s.trace_auto_dump);
    }

    #[test]
    fn clamp_all_pushes_out_of_range_values_into_window() {
        let mut s = GameplaySettings::default();
        s.player_damage_multiplier = 100.0;
        s.clamp_all();
        assert!(s.player_damage_multiplier <= 4.0 + 1e-6);
        s.player_damage_multiplier = -10.0;
        s.clamp_all();
        assert!(s.player_damage_multiplier >= 0.25 - 1e-6);
    }
}
