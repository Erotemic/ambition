//! Gameplay settings: difficulty, assist mode, trace auto-dump.
//!
//! These mostly read into resources consulted by feature systems
//! (encounter spawning, damage calculation, trace recorder). The
//! values are user-visible so they live in their own module rather
//! than under dev tools.

use serde::{Deserialize, Serialize};

pub use ambition_engine_core::InputFrameMode;

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

    /// Multiplier applied to enemy spawn counts in the goblin encounter. Easy
    /// reduces wave size; hard increases it.
    ///
    /// _Status_: defined but not yet wired into the encounter spawn
    /// path. goblin_encounter waves are authored with specific positions /
    /// brain types so a naive `count *= multiplier` would either
    /// drop authored mobs or duplicate them on top of each other.
    /// Future wiring options: scale per-wave delay, or randomly
    /// drop/clone mobs during encounter compose. Tracked under
    /// "More enemy varieties" / goblin_encounter tier-A items.
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
    /// Damage assist (accessibility): `On` halves damage the controlled body
    /// TAKES (`incoming_player_damage_multiplier`). That is its whole effect —
    /// owner decision 2026-07-19 ("honest rename"): the UI says "Damage
    /// assist — take half damage"; aim/traversal assists, if ever built, get
    /// their own settings.
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
    /// Suppress ALL gameplay AND menu input while the OS window is not focused.
    /// Default OFF (the user is unsure it's needed); a guard against
    /// foreground/background input bleed from another running game. When ON, the
    /// input population systems clear their frames whenever `Window.focused` is
    /// false (see `app::input_systems`).
    #[serde(default)]
    pub pause_input_when_unfocused: bool,
    /// Whether passing through a portal on a same-wall turn-around reverses the
    /// controlled body's facing direction. Default OFF; mirrored into
    /// `PortalTuning::reorient_facing` by the content portal adapter so it can be
    /// toggled live from the gameplay settings page.
    #[serde(default)]
    pub portal_reverses_facing: bool,
    /// How raw LOCOMOTION input maps onto the controlled body's local frame.
    /// `BodyRelativeAssist` follows the body frame except when upside-down, where
    /// the mapping accommodates screen orientation. `ScreenRelative` (the default)
    /// presses a screen direction to move in that screen direction through the
    /// controlled body's local frame. Flows into `MovementTuning::movement_frame_mode`.
    #[serde(default = "default_movement_frame_mode")]
    pub movement_frame_mode: InputFrameMode,
    /// How raw PRECISION-AIM input (blink steer, ranged/held-item aim) maps onto
    /// the controlled body's local frame. Independent of [`Self::movement_frame_mode`]
    /// because aiming a teleport/shot at a screen point is a different gesture than
    /// locomotion — it defaults to screen-directed ([`InputFrameMode::ScreenRelative`]) so
    /// precision aiming points where the stick points on screen at any gravity.
    #[serde(default = "default_aim_frame_mode")]
    pub aim_frame_mode: InputFrameMode,
}

/// Locomotion frame-mode default (see [`GameplaySettings::movement_frame_mode`]),
/// resolved from the engine's single source of truth.
fn default_movement_frame_mode() -> InputFrameMode {
    ambition_engine_core::ControlFrameModes::default().movement
}

/// Precision aiming defaults to screen-directed (see [`GameplaySettings::aim_frame_mode`]).
fn default_aim_frame_mode() -> InputFrameMode {
    ambition_engine_core::ControlFrameModes::default().aim
}

fn default_debug_hud_visible() -> bool {
    !cfg!(target_os = "android")
}

fn default_quest_hud_visible() -> bool {
    false
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
            pause_input_when_unfocused: false,
            portal_reverses_facing: false,
            movement_frame_mode: default_movement_frame_mode(),
            aim_frame_mode: default_aim_frame_mode(),
        }
    }
}

impl GameplaySettings {
    pub const DAMAGE_STEP: f32 = 0.10;

    /// The frame modes surfaced to the user — the two Jon asked for. The engine's
    /// third mode (`InputFrameMode::BodyRelativeStrict`, fully body-relative with no
    /// accommodation) stays dev-only and is reachable through the F3 tuning editor.
    /// Shared by both the locomotion and the precision-aim cycle.
    pub const FRAME_MODES: [InputFrameMode; 2] = [
        InputFrameMode::BodyRelativeAssist,
        InputFrameMode::ScreenRelative,
    ];

    /// Short, user-facing label for a frame mode.
    pub fn frame_mode_label(mode: InputFrameMode) -> &'static str {
        match mode {
            InputFrameMode::BodyRelativeAssist => "body-relative assist",
            InputFrameMode::ScreenRelative => "screen-directed",
            InputFrameMode::BodyRelativeStrict => "body-relative strict",
        }
    }

    /// The pair of control-authority frame policies these settings express, for
    /// the gameplay verbs that resolve input by source ([`ae::ControlFrameModes`]).
    pub fn control_frame_modes(&self) -> ambition_engine_core::ControlFrameModes {
        ambition_engine_core::ControlFrameModes {
            movement: self.movement_frame_mode,
            aim: self.aim_frame_mode,
        }
    }

    /// Position of `mode` within [`Self::FRAME_MODES`] and the surfaced count, for
    /// the cycle UI. Falls back to index 0 if the live mode is the dev-only one.
    fn frame_mode_index(mode: InputFrameMode) -> (usize, usize) {
        let i = Self::FRAME_MODES
            .iter()
            .position(|&m| m == mode)
            .unwrap_or(0);
        (i, Self::FRAME_MODES.len())
    }

    fn cycle_frame_mode(mode: InputFrameMode, dir: i32) -> InputFrameMode {
        let modes = Self::FRAME_MODES;
        let n = modes.len() as i32;
        let cur = modes.iter().position(|&m| m == mode).unwrap_or(0) as i32;
        let step = if dir < 0 { -1 } else { 1 };
        modes[(((cur + step) % n + n) % n) as usize]
    }

    pub fn movement_frame_mode_index(&self) -> (usize, usize) {
        Self::frame_mode_index(self.movement_frame_mode)
    }

    /// Cycle the locomotion frame mode across the surfaced set; `dir < 0` goes
    /// back, otherwise forward (confirm advances like next).
    pub fn cycle_movement_frame_mode(&mut self, dir: i32) {
        self.movement_frame_mode = Self::cycle_frame_mode(self.movement_frame_mode, dir);
    }

    pub fn aim_frame_mode_index(&self) -> (usize, usize) {
        Self::frame_mode_index(self.aim_frame_mode)
    }

    /// Cycle the precision-aim frame mode across the surfaced set.
    pub fn cycle_aim_frame_mode(&mut self, dir: i32) {
        self.aim_frame_mode = Self::cycle_frame_mode(self.aim_frame_mode, dir);
    }

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
        assert!(!settings.quest_hud_visible);
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
