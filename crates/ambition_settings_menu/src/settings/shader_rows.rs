//! The single source of truth for the 20 shader-settings rows.
//!
//! `build.rs` and `apply.rs` used to each hand-declare the same
//! (id, label, field, step, description) shape once per row — one to describe
//! the row, one to mutate it — and nothing forced the two declarations to
//! agree. This table exists once; `build.rs` reads it to compose the rows and
//! `apply.rs` reads it to mutate them.

use super::SettingsOptionId;
use ambition_persistence::settings::video::ScreenShaderSettings;

/// How a shader row's slider is stepped and clamped.
#[derive(Clone, Copy)]
pub(super) enum ShaderStep {
    /// A `[0.0, 1.0]` slider stepped by a fixed magnitude
    /// (`ScreenShaderSettings::UNIT_STEP` or `FINE_STEP`), percent-labelled.
    Unit(f32),
    /// A slider with its own range and step, labelled by `value_label`.
    Range {
        min: f32,
        max: f32,
        step: f32,
        value_label: fn(f32) -> String,
    },
}

pub(super) struct ShaderRow {
    pub id: SettingsOptionId,
    pub label: &'static str,
    pub description: &'static str,
    pub step: ShaderStep,
    pub get: fn(&ScreenShaderSettings) -> f32,
    pub set: fn(&mut ScreenShaderSettings, f32),
}

fn grain_size_label(v: f32) -> String {
    format!("{v:.0}px")
}

fn grain_fps_label(v: f32) -> String {
    format!("{v:.0} fps")
}

macro_rules! unit_row {
    ($id:expr, $field:ident, $step:expr, $label:literal, $desc:literal) => {
        ShaderRow {
            id: $id,
            label: $label,
            description: $desc,
            step: ShaderStep::Unit($step),
            get: |s: &ScreenShaderSettings| s.$field,
            set: |s: &mut ScreenShaderSettings, v: f32| s.$field = v,
        }
    };
}

/// The 20 shader rows, in the order they appear on screen. `shader_rows_cover_every_shader_id`
/// (in `tests.rs`) checks this list against every `SettingsOptionId::Shader*` variant.
pub(super) const SHADER_ROWS: &[ShaderRow] = &[
    unit_row!(
        SettingsOptionId::ShaderStrength,
        strength,
        ScreenShaderSettings::UNIT_STEP,
        "Shader Strength",
        "Global multiplier for the whole shader stack (0% = off)."
    ),
    unit_row!(
        SettingsOptionId::ShaderCrtStrength,
        crt_strength,
        ScreenShaderSettings::UNIT_STEP,
        "CRT Strength",
        "Overall CRT treatment strength."
    ),
    unit_row!(
        SettingsOptionId::ShaderCrtScanlines,
        crt_scanlines,
        ScreenShaderSettings::FINE_STEP,
        "CRT Scanlines",
        "CRT beam scanline darkening."
    ),
    unit_row!(
        SettingsOptionId::ShaderCrtMask,
        crt_mask,
        ScreenShaderSettings::FINE_STEP,
        "CRT Phosphor Mask",
        "CRT RGB phosphor mask intensity."
    ),
    unit_row!(
        SettingsOptionId::ShaderCrtCurvature,
        crt_curvature,
        ScreenShaderSettings::FINE_STEP,
        "CRT Curvature",
        "CRT screen-curvature warp amount."
    ),
    unit_row!(
        SettingsOptionId::ShaderCrtBloom,
        crt_bloom,
        ScreenShaderSettings::FINE_STEP,
        "CRT Bloom",
        "CRT local glow / bloom."
    ),
    unit_row!(
        SettingsOptionId::ShaderCrtChroma,
        crt_chroma,
        ScreenShaderSettings::FINE_STEP,
        "CRT Chroma Split",
        "CRT chromatic-aberration split."
    ),
    unit_row!(
        SettingsOptionId::ShaderFilmGrainStrength,
        film_grain_strength,
        ScreenShaderSettings::FINE_STEP,
        "Film Grain Strength",
        "Film-grain noise strength."
    ),
    ShaderRow {
        id: SettingsOptionId::ShaderFilmGrainSize,
        label: "Film Grain Size",
        description: "Output pixels per film-grain cell.",
        step: ShaderStep::Range {
            min: 1.0,
            max: 8.0,
            step: ScreenShaderSettings::GRAIN_SIZE_STEP,
            value_label: grain_size_label,
        },
        get: |s: &ScreenShaderSettings| s.film_grain_size,
        set: |s: &mut ScreenShaderSettings, v: f32| s.film_grain_size = v,
    },
    ShaderRow {
        id: SettingsOptionId::ShaderFilmGrainFps,
        label: "Film Grain Rate",
        description: "How often the film-grain seed changes.",
        step: ShaderStep::Range {
            min: 1.0,
            max: 60.0,
            step: ScreenShaderSettings::GRAIN_FPS_STEP,
            value_label: grain_fps_label,
        },
        get: |s: &ScreenShaderSettings| s.film_grain_fps,
        set: |s: &mut ScreenShaderSettings, v: f32| s.film_grain_fps = v,
    },
    unit_row!(
        SettingsOptionId::ShaderFilmGrainLumaBias,
        film_grain_luma_bias,
        ScreenShaderSettings::FINE_STEP,
        "Film Grain Luma Bias",
        "Bias film grain toward darker / brighter areas."
    ),
    unit_row!(
        SettingsOptionId::ShaderRobotDeathStrength,
        robot_death_strength,
        ScreenShaderSettings::UNIT_STEP,
        "Robot Death Strength",
        "Robot-death static / glitch strength."
    ),
    unit_row!(
        SettingsOptionId::ShaderRobotStatic,
        robot_static,
        ScreenShaderSettings::FINE_STEP,
        "Robot Static",
        "Robot-death static noise amount."
    ),
    unit_row!(
        SettingsOptionId::ShaderRobotTear,
        robot_tear,
        ScreenShaderSettings::FINE_STEP,
        "Robot Tear",
        "Robot-death horizontal tearing."
    ),
    unit_row!(
        SettingsOptionId::ShaderRobotDesaturate,
        robot_desaturate,
        ScreenShaderSettings::FINE_STEP,
        "Robot Desaturate",
        "Robot-death desaturation."
    ),
    unit_row!(
        SettingsOptionId::ShaderRobotScanlines,
        robot_scanlines,
        ScreenShaderSettings::FINE_STEP,
        "Robot Scanlines",
        "Robot-death scanline overlay."
    ),
    unit_row!(
        SettingsOptionId::ShaderUnderwaterStrength,
        underwater_strength,
        ScreenShaderSettings::UNIT_STEP,
        "Underwater Strength",
        "Underwater ripple / tint strength."
    ),
    unit_row!(
        SettingsOptionId::ShaderUnderwaterDistortion,
        underwater_distortion,
        ScreenShaderSettings::FINE_STEP,
        "Underwater Distortion",
        "Underwater displacement amount."
    ),
    unit_row!(
        SettingsOptionId::ShaderDeepDreamStrength,
        deep_dream_strength,
        ScreenShaderSettings::UNIT_STEP,
        "Deep Dream Strength",
        "Full-screen deep-dream reference view strength."
    ),
    unit_row!(
        SettingsOptionId::ShaderVignetteStrength,
        vignette_strength,
        ScreenShaderSettings::FINE_STEP,
        "Vignette Strength",
        "Edge-darkening vignette strength."
    ),
];
