//! Video / display-related settings.
//!
//! Display mode is the established axis; flashes and colorblind mode are
//! new and read by VFX/HUD systems where wired. The structs are
//! serializable so persistence (`crate::persistence::settings::persistence`) can
//! load/save them.

use serde::{Deserialize, Serialize};

use crate::host::windowing::DisplayModeKind;

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
    pub const ALL: [Self; 4] = [
        Self::Centered,
        Self::Combat,
        Self::Forward,
        Self::MobileSafe,
    ];

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

/// Whole-screen shader/post-process controls exposed under Video > Shaders.
///
/// The screen shader stack is controlled by independent strengths. A value of
/// `0.0` disables that ingredient; `strength` is the global multiplier and
/// therefore acts as the master off switch when it is zero. Secondary knobs
/// stay non-zero by default so enabling an effect strength immediately shows a
/// useful tuned version while still letting the Shaders page diagnose each
/// ingredient independently.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScreenShaderSettings {
    /// Legacy master switch from the first proof-of-concept. Kept only so old
    /// `settings.ron` files can be migrated during clamping, then omitted when
    /// settings are saved again.
    #[serde(default, skip_serializing)]
    pub enabled: bool,
    /// Legacy CRT toggle from the first proof-of-concept.
    #[serde(default, skip_serializing)]
    pub crt: bool,
    /// Legacy film-grain toggle from the first proof-of-concept.
    #[serde(default, skip_serializing)]
    pub film_grain: bool,
    /// Legacy robot-death toggle from the first proof-of-concept.
    #[serde(default, skip_serializing)]
    pub robot_death_static: bool,
    /// Legacy underwater toggle from the first proof-of-concept.
    #[serde(default, skip_serializing)]
    pub underwater_ripple: bool,
    /// Legacy vignette toggle from the first proof-of-concept.
    #[serde(default, skip_serializing)]
    pub vignette: bool,

    /// Global strength for the shader stack, in 0..=1. This is the master off
    /// switch when it reaches zero.
    #[serde(default = "default_shader_strength")]
    pub strength: f32,

    /// Lottes-inspired CRT treatment: curvature, beam scanlines, RGB mask, and
    /// subtle local glow.
    #[serde(default)]
    pub crt_strength: f32,
    #[serde(default = "default_crt_scanlines")]
    pub crt_scanlines: f32,
    #[serde(default = "default_crt_mask")]
    pub crt_mask: f32,
    #[serde(default = "default_crt_curvature")]
    pub crt_curvature: f32,
    #[serde(default = "default_crt_bloom")]
    pub crt_bloom: f32,
    #[serde(default = "default_crt_chroma")]
    pub crt_chroma: f32,

    /// Pixel/frame-anchored film grain. Grain size is measured in output
    /// pixels per grain cell; FPS controls how often the random seed changes.
    #[serde(default)]
    pub film_grain_strength: f32,
    #[serde(default = "default_grain_size")]
    pub film_grain_size: f32,
    #[serde(default = "default_grain_fps")]
    pub film_grain_fps: f32,
    #[serde(default = "default_grain_luma_bias")]
    pub film_grain_luma_bias: f32,

    /// Robot-death static/glitch treatment.
    #[serde(default)]
    pub robot_death_strength: f32,
    #[serde(default = "default_robot_static")]
    pub robot_static: f32,
    #[serde(default = "default_robot_tear")]
    pub robot_tear: f32,
    #[serde(default = "default_robot_desaturate")]
    pub robot_desaturate: f32,
    #[serde(default = "default_robot_scanlines")]
    pub robot_scanlines: f32,

    /// Underwater/heat-haze style ripple displacement and tint.
    #[serde(default)]
    pub underwater_strength: f32,
    #[serde(default = "default_underwater_distortion")]
    pub underwater_distortion: f32,

    /// Full-screen version of the puppy-slug deep-dream shader. This is a
    /// debug/reference view for validating the look independently of the
    /// per-sprite atlas/material path.
    #[serde(default)]
    pub deep_dream_strength: f32,

    /// Shared edge darkening layered after the other effects.
    #[serde(default)]
    pub vignette_strength: f32,
}

impl Default for ScreenShaderSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            crt: false,
            film_grain: false,
            robot_death_static: false,
            underwater_ripple: false,
            vignette: false,
            strength: default_shader_strength(),
            crt_strength: 0.0,
            crt_scanlines: default_crt_scanlines(),
            crt_mask: default_crt_mask(),
            crt_curvature: default_crt_curvature(),
            crt_bloom: default_crt_bloom(),
            crt_chroma: default_crt_chroma(),
            film_grain_strength: 0.0,
            film_grain_size: default_grain_size(),
            film_grain_fps: default_grain_fps(),
            film_grain_luma_bias: default_grain_luma_bias(),
            robot_death_strength: 0.0,
            robot_static: default_robot_static(),
            robot_tear: default_robot_tear(),
            robot_desaturate: default_robot_desaturate(),
            robot_scanlines: default_robot_scanlines(),
            underwater_strength: 0.0,
            underwater_distortion: default_underwater_distortion(),
            deep_dream_strength: 0.0,
            vignette_strength: 0.0,
        }
    }
}

impl ScreenShaderSettings {
    pub const UNIT_STEP: f32 = 0.10;
    pub const FINE_STEP: f32 = 0.05;
    pub const GRAIN_SIZE_STEP: f32 = 1.0;
    pub const GRAIN_FPS_STEP: f32 = 6.0;

    pub fn any_effect_enabled(&self) -> bool {
        self.crt_strength > 0.001
            || self.film_grain_strength > 0.001
            || self.robot_death_strength > 0.001
            || self.underwater_strength > 0.001
            || self.deep_dream_strength > 0.001
            || self.vignette_strength > 0.001
    }

    pub fn strength_percent(&self) -> u8 {
        Self::percent(self.strength)
    }

    pub fn percent(value: f32) -> u8 {
        (value.clamp(0.0, 1.0) * 100.0).round() as u8
    }

    pub fn nudge_unit(value: &mut f32, delta: f32) {
        *value = (*value + delta).clamp(0.0, 1.0);
    }

    pub fn nudge_range(value: &mut f32, delta: f32, min: f32, max: f32) {
        *value = (*value + delta).clamp(min, max);
    }

    pub fn nudge_strength(&mut self, delta: f32) {
        Self::nudge_unit(&mut self.strength, delta);
    }

    /// Clamp hand-edited settings and migrate legacy boolean shader toggles
    /// into the new independent-strength model.
    pub fn clamp_all(&mut self) {
        let had_new_effect_strength = self.any_effect_enabled();
        if self.enabled && !had_new_effect_strength {
            if self.crt {
                self.crt_strength = default_migrated_effect_strength();
            }
            if self.film_grain {
                self.film_grain_strength = default_migrated_grain_strength();
            }
            if self.robot_death_static {
                self.robot_death_strength = default_migrated_effect_strength();
            }
            if self.underwater_ripple {
                self.underwater_strength = default_migrated_effect_strength();
            }
            if self.vignette {
                self.vignette_strength = default_migrated_vignette_strength();
            }
        }

        self.strength = self.strength.clamp(0.0, 1.0);
        self.crt_strength = self.crt_strength.clamp(0.0, 1.0);
        self.crt_scanlines = self.crt_scanlines.clamp(0.0, 1.0);
        self.crt_mask = self.crt_mask.clamp(0.0, 1.0);
        self.crt_curvature = self.crt_curvature.clamp(0.0, 1.0);
        self.crt_bloom = self.crt_bloom.clamp(0.0, 1.0);
        self.crt_chroma = self.crt_chroma.clamp(0.0, 1.0);
        self.film_grain_strength = self.film_grain_strength.clamp(0.0, 1.0);
        self.film_grain_size = self.film_grain_size.clamp(1.0, 8.0);
        self.film_grain_fps = self.film_grain_fps.clamp(1.0, 60.0);
        self.film_grain_luma_bias = self.film_grain_luma_bias.clamp(0.0, 1.0);
        self.robot_death_strength = self.robot_death_strength.clamp(0.0, 1.0);
        self.robot_static = self.robot_static.clamp(0.0, 1.0);
        self.robot_tear = self.robot_tear.clamp(0.0, 1.0);
        self.robot_desaturate = self.robot_desaturate.clamp(0.0, 1.0);
        self.robot_scanlines = self.robot_scanlines.clamp(0.0, 1.0);
        self.underwater_strength = self.underwater_strength.clamp(0.0, 1.0);
        self.underwater_distortion = self.underwater_distortion.clamp(0.0, 1.0);
        self.deep_dream_strength = self.deep_dream_strength.clamp(0.0, 1.0);
        self.vignette_strength = self.vignette_strength.clamp(0.0, 1.0);
    }
}

fn default_shader_strength() -> f32 {
    0.0
}

fn default_crt_scanlines() -> f32 {
    0.70
}

fn default_crt_mask() -> f32 {
    0.72
}

fn default_crt_curvature() -> f32 {
    0.90
}

fn default_crt_bloom() -> f32 {
    0.12
}

fn default_crt_chroma() -> f32 {
    // Lowered from 0.45 after Android playtesting: the higher default
    // made the screen feel rainbow-fringed before the user had a
    // chance to tune it down. 10% reads as "subtle chroma" and still
    // makes the effect visible when CRT strength is bumped.
    0.10
}

fn default_grain_size() -> f32 {
    1.0
}

fn default_grain_fps() -> f32 {
    24.0
}

fn default_grain_luma_bias() -> f32 {
    0.35
}

fn default_robot_static() -> f32 {
    0.55
}

fn default_robot_tear() -> f32 {
    0.90
}

fn default_robot_desaturate() -> f32 {
    0.48
}

fn default_robot_scanlines() -> f32 {
    0.38
}

fn default_underwater_distortion() -> f32 {
    0.95
}

fn default_migrated_effect_strength() -> f32 {
    0.75
}

fn default_migrated_grain_strength() -> f32 {
    0.22
}

fn default_migrated_vignette_strength() -> f32 {
    0.55
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Whether the FPS / frame-time overlay is shown. ON by default
    /// on every platform — useful for diagnosing perf issues across
    /// browser and desktop. Toggle via the Video page or `F3`. The
    /// overlay is wired by `crate::dev::fps_overlay::FpsOverlayPlugin`,
    /// which mirrors this flag into `FpsOverlayState::visible`.
    #[serde(default = "default_show_fps")]
    pub show_fps: bool,
    #[serde(default)]
    pub shaders: ScreenShaderSettings,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            display_mode: SerializableDisplayMode::default(),
            camera_zoom: CameraZoomPreset::default(),
            camera_aspect: CameraAspectPolicy::default(),
            camera_framing: CameraFramingPreset::default(),
            flashes: FlashIntensity::default(),
            colorblind: ColorblindMode::default(),
            show_fps: default_show_fps(),
            shaders: ScreenShaderSettings::default(),
        }
    }
}

impl VideoSettings {
    pub fn clamp_all(&mut self) {
        self.shaders.clamp_all();
    }
}

/// Default for `VideoSettings::show_fps`. Kept as a free function so
/// `serde(default = "...")` can reference it for round-tripping older
/// `settings.ron` files that pre-date this field.
fn default_show_fps() -> bool {
    true
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
    fn screen_shader_strength_stays_clamped() {
        let mut shaders = ScreenShaderSettings::default();
        shaders.nudge_strength(10.0);
        assert_eq!(shaders.strength, 1.0);
        shaders.nudge_strength(-10.0);
        assert_eq!(shaders.strength, 0.0);
    }

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
