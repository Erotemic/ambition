//! Debug view/art modes + player body / movement profile presets.

use crate::engine_core as ae;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Coarse player-body presets for feel testing.
///
/// These affect the movement collider only. The placeholder sprite is scaled
/// separately around the collider so temporary art can change without becoming
/// gameplay authority.
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerBodyProfile {
    Compact,
    #[default]
    ReadableDefault,
    Heavy,
}

impl PlayerBodyProfile {
    pub const ALL: [Self; 3] = [Self::Compact, Self::ReadableDefault, Self::Heavy];

    pub fn label(self) -> &'static str {
        match self {
            Self::Compact => "compact 26x42",
            Self::ReadableDefault => "default 30x48",
            Self::Heavy => "heavy 32x50",
        }
    }

    pub fn size(self) -> ae::Vec2 {
        match self {
            Self::Compact => ae::Vec2::new(26.0, 42.0),
            Self::ReadableDefault => ae::Vec2::new(30.0, 48.0),
            Self::Heavy => ae::Vec2::new(32.0, 50.0),
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(1);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(1);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// High-level movement profiles for fast chassis swaps from the dev menu.
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementProfile {
    AgileBase,
    #[default]
    SandboxFast,
    Heavy,
    Legacy,
}

impl MovementProfile {
    pub const ALL: [Self; 4] = [
        Self::AgileBase,
        Self::SandboxFast,
        Self::Heavy,
        Self::Legacy,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AgileBase => "agile base",
            Self::SandboxFast => "sandbox fast",
            Self::Heavy => "heavy",
            Self::Legacy => "legacy",
        }
    }

    pub fn tuning(self) -> ae::MovementTuning {
        let mut tuning = ae::MovementTuning::default();
        match self {
            Self::AgileBase => {
                tuning.flight_accel = 2200.0;
                tuning.flight_drag = 1600.0;
                tuning.flight_terminal_speed = 560.0;
                tuning.flight_hover_speed = 30.0;
            }
            Self::SandboxFast => {
                // Matches the Phase 1 default constants: slower base chassis,
                // but fast explicit flight for sandbox traversal.
            }
            Self::Heavy => {
                tuning.max_run_speed = 240.0;
                tuning.run_accel = 4200.0;
                tuning.air_accel = 2400.0;
                tuning.ground_friction = 7000.0;
                tuning.air_friction = 550.0;
                tuning.gravity = 2450.0;
                tuning.jump_speed = 570.0;
                tuning.double_jump_speed = 470.0;
                tuning.max_fall_speed = 1000.0;
                tuning.dash_speed = 700.0;
                tuning.dash_time = 0.115;
                tuning.dash_cooldown = 0.220;
                tuning.flight_accel = 1900.0;
                tuning.flight_drag = 1450.0;
                tuning.flight_terminal_speed = 520.0;
                tuning.flight_hover_speed = 28.0;
            }
            Self::Legacy => {
                tuning.gravity = 2250.0;
                tuning.run_accel = 7600.0;
                tuning.air_accel = 4700.0;
                tuning.ground_friction = 9200.0;
                tuning.air_friction = 860.0;
                tuning.max_run_speed = 330.0;
                tuning.max_fall_speed = 1040.0;
                tuning.jump_speed = 690.0;
                tuning.double_jump_speed = 630.0;
                tuning.wall_jump_x = 565.0;
                tuning.wall_slide_speed = 170.0;
                tuning.wall_climb_speed = 250.0;
                tuning.dash_speed = 820.0;
                tuning.dash_time = 0.105;
                tuning.dash_cooldown = 0.060;
                tuning.dash_buffer = 0.110;
                tuning.flight_accel = 900.0;
                tuning.flight_drag = 520.0;
                tuning.flight_terminal_speed = 430.0;
                tuning.flight_hover_speed = 42.0;
                tuning.pogo_speed = 810.0;
                tuning.slash_recoil = 130.0;
            }
        }
        tuning
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(1);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(1);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Named debug visualization presets for the daily "what am I inspecting?"
/// workflow. Individual booleans still exist as the Custom backing store, but
/// these modes are the primary interface exposed in the developer menu.
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugViewMode {
    /// Normal play view: no spatial overlays.
    Gameplay,
    /// Default debug-authoring view: level context plus player/feature volumes.
    Authoring,
    /// Collision view: solid/query volumes with art out of the way.
    Collision,
    /// Trigger/transition view: loading zones and camera/debug volumes.
    Triggers,
    /// Combat-feel view: actor/projectile/player combat volumes.
    Combat,
    /// Everything we can draw without opening the full inspector.
    All,
    /// Hand-edited toggle state.
    #[default]
    Custom,
}

impl DebugViewMode {
    pub const ALL: [Self; 7] = [
        Self::Gameplay,
        Self::Authoring,
        Self::Collision,
        Self::Triggers,
        Self::Combat,
        Self::All,
        Self::Custom,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Gameplay => "gameplay",
            Self::Authoring => "authoring",
            Self::Collision => "collision",
            Self::Triggers => "triggers",
            Self::Combat => "combat",
            Self::All => "all",
            Self::Custom => "custom",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub const fn recommended_art_mode(self) -> DebugArtMode {
        match self {
            Self::Gameplay | Self::Authoring | Self::Triggers | Self::Combat | Self::All => {
                DebugArtMode::Normal
            }
            Self::Collision => DebugArtMode::Hidden,
            Self::Custom => DebugArtMode::Normal,
        }
    }
}

/// How normal sprite presentation should behave while inspecting debug data.
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugArtMode {
    #[default]
    Normal,
    Placeholder,
    Hidden,
}

impl DebugArtMode {
    pub const ALL: [Self; 3] = [Self::Normal, Self::Placeholder, Self::Hidden];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Placeholder => "placeholder",
            Self::Hidden => "hidden",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}
