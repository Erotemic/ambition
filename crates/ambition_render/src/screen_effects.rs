//! Whole-screen post-processing effects for presentation cameras.
//!
//! This is intentionally not a sprite overlay. The pass runs after the 2D main
//! pass, samples the already-rendered view texture, and writes a fullscreen
//! filtered result back into Bevy's post-process destination. That lets shader
//! toggles distort scene UVs, split color channels, apply scanlines, and
//! modulate luminance in ways an overlay cannot.

use bevy::{
    core_pipeline::{
        fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
        schedule::Core2d,
        tonemapping::tonemapping,
        Core2dSystems,
    },
    ecs::{schedule::ScheduleConfigs, system::BoxedSystem},
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
    shader::ShaderRef,
};

use ambition_persistence::settings::{ScreenShaderSettings, UserSettings};

const SHADER_ASSET_PATH: &str = "shaders/screen_effects.wgsl";

/// Presentation plugin for camera-local screen filters.
pub struct ScreenEffectsPlugin;

impl Plugin for ScreenEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FullscreenMaterialPlugin::<ScreenEffectSettings>::default())
            .add_systems(Update, sync_screen_effect_settings_from_video_settings);
    }
}

impl FullscreenMaterial for ScreenEffectSettings {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    /// Ambition is a 2D game; the default is [`Core3d`](bevy::core_pipeline::schedule::Core3d).
    fn schedule() -> impl bevy::ecs::schedule::ScheduleLabel + Clone {
        Core2d
    }

    /// AFTER tonemapping, and inside `PostProcess` so upscaling still follows.
    ///
    /// This is the same slot the hand-written render-graph node occupied under
    /// Bevy 0.18, where the edges read
    /// `Tonemapping -> ScreenEffects -> EndMainPassPostProcessing`. The order
    /// matters to the look: these filters are authored against tonemapped,
    /// display-referred color, so running them before tonemapping would change
    /// what every strength value means.
    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system.in_set(Core2dSystems::PostProcess).after(tonemapping)
    }
}

/// GPU-facing component attached to cameras that should receive the screen
/// filter. Vec4 packing keeps the uniform layout WebGL2-friendly while exposing
/// enough independent parameters for diagnosing shader ingredients.
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct ScreenEffectSettings {
    /// x = global strength, y = elapsed seconds modulo one hour,
    /// z = film-grain frame rate, w = film-grain pixel size.
    pub control: Vec4,
    /// x = CRT strength, y = film-grain strength, z = robot-death strength,
    /// w = underwater strength. These are already multiplied by the global
    /// strength on the CPU, so zero means disabled.
    pub strengths: Vec4,
    /// x = CRT scanlines, y = CRT mask, z = CRT curvature, w = CRT bloom.
    pub crt: Vec4,
    /// x = film-grain luma bias, y = vignette strength, z = CRT chroma split,
    /// w = reserved.
    pub grain_and_vignette: Vec4,
    /// x = robot static, y = robot tear, z = robot desaturation,
    /// w = robot scanlines.
    pub robot: Vec4,
    /// x = underwater distortion, y = full-screen deep-dream strength, z/w = reserved.
    pub underwater: Vec4,
}

impl Default for ScreenEffectSettings {
    fn default() -> Self {
        Self::for_shader_settings(&ScreenShaderSettings::default(), 0.0)
    }
}

impl ScreenEffectSettings {
    pub fn for_shader_settings(shaders: &ScreenShaderSettings, elapsed_secs: f32) -> Self {
        let global = shaders.strength.clamp(0.0, 1.0);
        let enabled = shaders.any_effect_enabled() && global > 0.001;
        let active = |value: f32| {
            if enabled {
                value.clamp(0.0, 1.0) * global
            } else {
                0.0
            }
        };

        Self {
            control: Vec4::new(
                global,
                elapsed_secs.rem_euclid(3600.0),
                shaders.film_grain_fps.clamp(1.0, 60.0),
                shaders.film_grain_size.clamp(1.0, 8.0),
            ),
            strengths: Vec4::new(
                active(shaders.crt_strength),
                active(shaders.film_grain_strength),
                active(shaders.robot_death_strength),
                active(shaders.underwater_strength),
            ),
            crt: Vec4::new(
                shaders.crt_scanlines.clamp(0.0, 1.0),
                shaders.crt_mask.clamp(0.0, 1.0),
                shaders.crt_curvature.clamp(0.0, 1.0),
                shaders.crt_bloom.clamp(0.0, 1.0),
            ),
            grain_and_vignette: Vec4::new(
                shaders.film_grain_luma_bias.clamp(0.0, 1.0),
                active(shaders.vignette_strength),
                shaders.crt_chroma.clamp(0.0, 1.0),
                0.0,
            ),
            robot: Vec4::new(
                shaders.robot_static.clamp(0.0, 1.0),
                shaders.robot_tear.clamp(0.0, 1.0),
                shaders.robot_desaturate.clamp(0.0, 1.0),
                shaders.robot_scanlines.clamp(0.0, 1.0),
            ),
            underwater: Vec4::new(
                shaders.underwater_distortion.clamp(0.0, 1.0),
                active(shaders.deep_dream_strength),
                0.0,
                0.0,
            ),
        }
    }
}

fn sync_screen_effect_settings_from_video_settings(
    settings: Res<UserSettings>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    time: Res<Time>,
    mut cameras: Query<&mut ScreenEffectSettings>,
) {
    let mut shaders = settings.video.shaders.clone();
    if let Some(quality) = quality {
        shaders.strength = shaders
            .strength
            .min(quality.budget.shaders.screen_shader_scale);
    }
    let next = ScreenEffectSettings::for_shader_settings(&shaders, time.elapsed_secs());
    for mut camera_settings in &mut cameras {
        *camera_settings = next;
    }
}
