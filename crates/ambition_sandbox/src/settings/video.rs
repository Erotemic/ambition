//! Video / display-related settings.
//!
//! Display mode is the established axis; flashes and colorblind mode are
//! new and read by VFX/HUD systems where wired. The structs are
//! serializable so persistence (`crate::settings::persistence`) can
//! load/save them.

use serde::{Deserialize, Serialize};

use crate::windowing::DisplayModeKind;

/// Whether full-screen flash effects are shown at full strength,
/// reduced, or disabled. Read by camera flash and VFX systems.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlashIntensity {
    #[default]
    On,
    Reduced,
    Off,
}

impl FlashIntensity {
    pub const ALL: [Self; 3] = [Self::On, Self::Reduced, Self::Off];

    pub fn label(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Reduced => "reduced",
            Self::Off => "off",
        }
    }

    /// Multiplier applied to flash alpha / camera flash decay. `1.0` is
    /// full strength; `0.0` disables the effect.
    pub fn multiplier(self) -> f32 {
        match self {
            Self::On => 1.0,
            Self::Reduced => 0.45,
            Self::Off => 0.0,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::On => Self::Reduced,
            Self::Reduced => Self::Off,
            Self::Off => Self::On,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::On => Self::Off,
            Self::Reduced => Self::On,
            Self::Off => Self::Reduced,
        }
    }
}

/// Colorblind accessibility mode. The full palette remap is future work;
/// for now the setting is a resource so HUD/debug can show it and
/// future render systems can consult it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorblindMode {
    #[default]
    Off,
    Protanopia,
    Deuteranopia,
    Tritanopia,
    HighContrast,
}

impl ColorblindMode {
    pub const ALL: [Self; 5] = [
        Self::Off,
        Self::Protanopia,
        Self::Deuteranopia,
        Self::Tritanopia,
        Self::HighContrast,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Protanopia => "protanopia",
            Self::Deuteranopia => "deuteranopia",
            Self::Tritanopia => "tritanopia",
            Self::HighContrast => "high contrast",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|m| m == &self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|m| m == &self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Gameplay camera viewport preset.
///
/// The camera now targets a fixed world-space gameplay rectangle instead of
/// letting larger desktop windows reveal more of the level. Encounter zooms
/// multiply this base viewport; debug overview remains a separate developer
/// override.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraZoomPreset {
    Tight,
    #[default]
    Combat,
    Arena,
    Cinematic,
    Debug,
}

impl CameraZoomPreset {
    pub const ALL: [Self; 5] = [
        Self::Tight,
        Self::Combat,
        Self::Arena,
        Self::Cinematic,
        Self::Debug,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Tight => "tight 640x360",
            Self::Combat => "combat 800x450",
            Self::Arena => "arena 960x540",
            Self::Cinematic => "cinematic 1120x630",
            Self::Debug => "debug 1600x900",
        }
    }

    /// Base gameplay viewport in world units before encounter/debug multipliers.
    pub fn base_view(self) -> (f32, f32) {
        match self {
            Self::Tight => (640.0, 360.0),
            Self::Combat => (800.0, 450.0),
            Self::Arena => (960.0, 540.0),
            Self::Cinematic => (1120.0, 630.0),
            Self::Debug => (1600.0, 900.0),
        }
    }

    /// Relative scale versus the combat default. Kept for HUD/tests and for
    /// callers that still treat this as a zoom-like setting.
    pub fn scale(self) -> f32 {
        let (_, h) = self.base_view();
        h / 450.0
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


/// How the fixed gameplay viewport maps onto non-16:9 windows.
///
/// `FitDesign` is the default: the full authored design rectangle remains
/// visible, and wider/taller windows may reveal a modest margin. `FixedHeight`
/// mirrors the usual Unity orthographic policy (stable vertical world height;
/// width follows aspect ratio). `FixedWidth` is useful for checking narrow /
/// portrait/mobile framing where horizontal information must stay bounded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraAspectPolicy {
    #[default]
    FitDesign,
    FixedHeight,
    FixedWidth,
}

impl CameraAspectPolicy {
    pub const ALL: [Self; 3] = [Self::FitDesign, Self::FixedHeight, Self::FixedWidth];

    pub fn label(self) -> &'static str {
        match self {
            Self::FitDesign => "fit design",
            Self::FixedHeight => "fixed height",
            Self::FixedWidth => "fixed width",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Bias applied to the camera target inside the selected viewport.
///
/// This is deliberately presentation-only: it does not change collision,
/// enemy activation, or room logic. It lets a combat game show slightly more
/// space in front/above the player while preserving the same authored viewport
/// size across desktop and mobile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraFramingPreset {
    Centered,
    #[default]
    Combat,
    Forward,
    MobileSafe,
}

impl CameraFramingPreset {
    pub const ALL: [Self; 4] = [Self::Centered, Self::Combat, Self::Forward, Self::MobileSafe];

    pub fn label(self) -> &'static str {
        match self {
            Self::Centered => "centered",
            Self::Combat => "combat bias",
            Self::Forward => "forward bias",
            Self::MobileSafe => "mobile safe",
        }
    }

    /// Return a world-space offset (Ambition coordinates: +Y is downward) from
    /// the player center to the camera target.
    pub fn target_offset(self, view_w: f32, view_h: f32, facing: f32) -> (f32, f32) {
        let facing = if facing < 0.0 { -1.0 } else { 1.0 };
        match self {
            Self::Centered => (0.0, 0.0),
            // Combat default avoids horizontal look-ahead: a quick tap/turn
            // should not move the camera. Keep only a small upward bias for
            // jumps, flying enemies, and combat reads. Negative Y means "move
            // the camera upward" in Ambition's +Y-down coordinate frame.
            Self::Combat => (0.0, -view_h * 0.05),
            // Explicit opt-in look-ahead preset. Kept intentionally small so
            // facing flips do not read as camera jerks.
            Self::Forward => (view_w * 0.04 * facing, -view_h * 0.02),
            // Thumb controls tend to occlude the bottom corners; keep the
            // player lower on the physical screen by biasing the camera target
            // upward. Avoid horizontal bias here too; thumb motion should not
            // make the camera twitch left/right.
            Self::MobileSafe => (0.0, -view_h * 0.12),
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct VideoSettings {
    #[serde(default)]
    pub display_mode: SerializableDisplayMode,
    #[serde(default)]
    pub camera_zoom: CameraZoomPreset,
    #[serde(default)]
    pub camera_aspect: CameraAspectPolicy,
    #[serde(default)]
    pub camera_framing: CameraFramingPreset,
    #[serde(default)]
    pub flashes: FlashIntensity,
    #[serde(default)]
    pub colorblind: ColorblindMode,
}

/// Serializable mirror of `DisplayModeKind`. We keep `DisplayModeKind`
/// in the windowing module (it's tied to Bevy's `WindowMode`); this
/// type lets us serialize without reaching into windowing's enum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerializableDisplayMode {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}

impl From<DisplayModeKind> for SerializableDisplayMode {
    fn from(value: DisplayModeKind) -> Self {
        match value {
            DisplayModeKind::Windowed => Self::Windowed,
            DisplayModeKind::Borderless => Self::Borderless,
            DisplayModeKind::Fullscreen => Self::Fullscreen,
        }
    }
}

impl From<SerializableDisplayMode> for DisplayModeKind {
    fn from(value: SerializableDisplayMode) -> Self {
        match value {
            SerializableDisplayMode::Windowed => Self::Windowed,
            SerializableDisplayMode::Borderless => Self::Borderless,
            SerializableDisplayMode::Fullscreen => Self::Fullscreen,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_intensity_cycles() {
        let order = [
            FlashIntensity::On,
            FlashIntensity::Reduced,
            FlashIntensity::Off,
            FlashIntensity::On,
        ];
        let mut current = order[0];
        for expected in &order[1..] {
            current = current.next();
            assert_eq!(current, *expected);
        }
    }

    #[test]
    fn colorblind_mode_cycles_through_all() {
        let mut visited = std::collections::HashSet::new();
        let mut cur = ColorblindMode::Off;
        for _ in 0..ColorblindMode::ALL.len() {
            visited.insert(cur);
            cur = cur.next();
        }
        assert_eq!(visited.len(), ColorblindMode::ALL.len());
    }

    #[test]
    fn flash_multiplier_clamps() {
        assert_eq!(FlashIntensity::On.multiplier(), 1.0);
        assert_eq!(FlashIntensity::Off.multiplier(), 0.0);
        assert!(FlashIntensity::Reduced.multiplier() > 0.0);
        assert!(FlashIntensity::Reduced.multiplier() < 1.0);
    }

    #[test]
    fn flash_intensity_cycles_through_all() {
        let mut visited: Vec<FlashIntensity> = Vec::new();
        let mut cur = FlashIntensity::On;
        for _ in 0..FlashIntensity::ALL.len() {
            if !visited.contains(&cur) {
                visited.push(cur);
            }
            cur = cur.next();
        }
        assert_eq!(visited.len(), FlashIntensity::ALL.len());
    }

    #[test]
    fn camera_zoom_preset_scales_are_positive_finite() {
        for preset in CameraZoomPreset::ALL {
            let scale = preset.scale();
            assert!(scale > 0.0 && scale.is_finite());
        }
    }

    #[test]
    fn camera_zoom_preset_cycles_through_all() {
        let mut visited: Vec<CameraZoomPreset> = Vec::new();
        let mut cur = CameraZoomPreset::Combat;
        for _ in 0..CameraZoomPreset::ALL.len() {
            if !visited.contains(&cur) {
                visited.push(cur);
            }
            cur = cur.next();
        }
        assert_eq!(visited.len(), CameraZoomPreset::ALL.len());
    }

    #[test]
    fn camera_aspect_policy_cycles_through_all() {
        let mut visited: Vec<CameraAspectPolicy> = Vec::new();
        let mut cur = CameraAspectPolicy::FitDesign;
        for _ in 0..CameraAspectPolicy::ALL.len() {
            if !visited.contains(&cur) {
                visited.push(cur);
            }
            cur = cur.next();
        }
        assert_eq!(visited.len(), CameraAspectPolicy::ALL.len());
    }

    #[test]
    fn combat_framing_biases_up_without_horizontal_tap_lookahead() {
        let (dx, dy) = CameraFramingPreset::Combat.target_offset(800.0, 450.0, 1.0);
        assert_eq!(dx, 0.0);
        assert!(dy < 0.0);
        let (dx_left, _) = CameraFramingPreset::Combat.target_offset(800.0, 450.0, -1.0);
        assert_eq!(dx_left, 0.0);
    }

    #[test]
    fn flash_intensity_prev_next_round_trip() {
        let f = FlashIntensity::Reduced;
        assert_eq!(f.next().prev(), f);
    }
}
