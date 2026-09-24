//! Whole-screen post-processing effects for presentation cameras.
//!
//! Not a sprite overlay. The pass runs after the 2D main pass, samples the
//! rendered view texture, and writes a filtered fullscreen result to Bevy's
//! post-process destination. So the filters can distort scene UVs, split
//! colour channels, add scanlines, and change luminance, which an overlay
//! cannot.

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

    /// After tonemapping, inside `PostProcess`, so upscaling still follows. The
    /// filters are authored against tonemapped, display-referred colour; running
    /// them earlier would change what every strength value means.
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

/// Marks a camera that may receive the screen filter.
///
/// Eligibility (this marker) and enrolment ([`ScreenEffectSettings`]) are
/// separate. Bevy's `FullscreenMaterialPlugin` uses the presence of
/// [`ScreenEffectSettings`] as the enable: without it there is no pipeline,
/// no bind group, no pass, and no `post_process_write` ping-pong.
/// [`sync_screen_effect_settings`] adds and removes the settings component,
/// and uses this marker to know which cameras to add it back to. Nothing
/// removes this marker, so removing the settings is safe.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ScreenEffectCamera;

/// Whether this configuration would draw anything at all.
///
/// One definition, two readers: the packing below zeroes every strength when
/// it is false, and [`sync_screen_effect_settings`] does not enrol the camera.
fn draws_anything(shaders: &ScreenShaderSettings) -> bool {
    shaders.any_effect_enabled() && shaders.strength.clamp(0.0, 1.0) > 0.001
}

impl ScreenEffectSettings {
    pub fn for_shader_settings(shaders: &ScreenShaderSettings, elapsed_secs: f32) -> Self {
        let global = shaders.strength.clamp(0.0, 1.0);
        let enabled = draws_anything(shaders);
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

/// Enrol every eligible camera in the filter while it draws something, and
/// take them back out when it does not.
///
/// The default configuration has every effect off. Removing
/// [`ScreenEffectSettings`] is Bevy's way to say a camera does not need the
/// material, so no fullscreen pass (read and write of the whole view texture)
/// runs for a filter that would return the source pixel.
///
/// Removal is safe only because eligibility is on [`ScreenEffectCamera`],
/// which nothing removes, so this system can always find the cameras to
/// enrol again.
fn sync_screen_effect_settings_from_video_settings(
    mut commands: Commands,
    settings: Res<UserSettings>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    time: Res<Time>,
    mut cameras: Query<(Entity, Option<&mut ScreenEffectSettings>), With<ScreenEffectCamera>>,
) {
    let mut shaders = settings.video.shaders.clone();
    if let Some(quality) = quality {
        shaders.strength = shaders
            .strength
            .min(quality.budget.shaders.screen_shader_scale);
    }
    // The quality clamp is part of the question: the Potato tier scales
    // `screen_shader_scale` to 0.0, and a camera whose effects are all
    // scaled away needs no pass.
    if !draws_anything(&shaders) {
        for (camera, settings) in &cameras {
            if settings.is_some() {
                commands.entity(camera).remove::<ScreenEffectSettings>();
            }
        }
        return;
    }

    let next = ScreenEffectSettings::for_shader_settings(&shaders, time.elapsed_secs());
    for (camera, current) in &mut cameras {
        match current {
            Some(mut current) => *current = next,
            None => {
                commands.entity(camera).insert(next);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A camera is enrolled in the filter only while something is turned up.
    ///
    /// Both directions matter. Bevy reads the presence of
    /// [`ScreenEffectSettings`], so absence saves the pass, and a remove-only
    /// system would lose the filter for the rest of the run. So the arms are
    /// off, on, off: the last is the retraction, and the middle proves it is
    /// recoverable.
    #[test]
    fn a_camera_is_enrolled_only_while_an_effect_is_turned_up() {
        fn enrolled(app: &mut App, camera: Entity) -> bool {
            app.world()
                .entity(camera)
                .contains::<ScreenEffectSettings>()
        }

        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<UserSettings>();
        app.add_systems(Update, sync_screen_effect_settings_from_video_settings);
        let camera = app.world_mut().spawn(ScreenEffectCamera).id();

        // Off is the shipped default: every strength is zero.
        app.update();
        assert!(
            !enrolled(&mut app, camera),
            "the default settings turn no effect up, so the camera must not carry \
             the material component that makes Bevy run a fullscreen pass for it"
        );

        // On. Both gates: `strength` is the master slider and defaults to 0.0,
        // so raising `crt_strength` alone still draws nothing.
        {
            let mut settings = app.world_mut().resource_mut::<UserSettings>();
            settings.video.shaders.strength = 1.0;
            settings.video.shaders.crt_strength = 1.0;
        }
        app.update();
        assert!(
            enrolled(&mut app, camera),
            "a camera eligible for the filter must be enrolled once an effect is \
             turned up"
        );

        // Off again: the retraction. A remove-only or insert-only
        // implementation fails one of these arms.
        app.world_mut()
            .resource_mut::<UserSettings>()
            .video
            .shaders
            .crt_strength = 0.0;
        app.update();
        assert!(
            !enrolled(&mut app, camera),
            "turning the last effect back down must take the camera out of the \
             filter again, not leave a no-op pass running for the rest of the run"
        );
    }

    /// A camera without the marker is never enrolled, however loud the settings.
    ///
    /// Without this, the test above would pass for a system that enrols every
    /// camera (HUD, cube, portal captures), which is a much more expensive bug.
    #[test]
    fn an_unmarked_camera_is_never_enrolled() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<UserSettings>();
        app.add_systems(Update, sync_screen_effect_settings_from_video_settings);
        let other = app.world_mut().spawn_empty().id();
        {
            let mut settings = app.world_mut().resource_mut::<UserSettings>();
            settings.video.shaders.strength = 1.0;
            settings.video.shaders.crt_strength = 1.0;
        }
        // Premise guard: without it this checks an absence the settings alone
        // would cause, and passes for any implementation.
        let marked = app.world_mut().spawn(ScreenEffectCamera).id();
        app.update();
        assert!(
            app.world()
                .entity(marked)
                .contains::<ScreenEffectSettings>(),
            "premise: these settings must enrol a marked camera, or the assertion \
             below is about nothing"
        );
        assert!(
            !app.world().entity(other).contains::<ScreenEffectSettings>(),
            "only a `ScreenEffectCamera` may be enrolled"
        );
    }

    /// The quality budget is part of the question, not a later adjustment.
    ///
    /// The Potato tier scales screen shaders to zero, and a camera whose effects
    /// are all scaled away needs no pass. Reading the settings alone would run a
    /// no-op shader on the weakest hardware.
    #[test]
    fn a_tier_that_scales_shaders_away_leaves_the_camera_out() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<UserSettings>();
        app.add_systems(Update, sync_screen_effect_settings_from_video_settings);
        let camera = app.world_mut().spawn(ScreenEffectCamera).id();
        {
            let mut settings = app.world_mut().resource_mut::<UserSettings>();
            settings.video.shaders.strength = 1.0;
            settings.video.shaders.crt_strength = 1.0;
        }
        // Premise guard: `strength` defaults to 0.0, so without raising the
        // settings this would credit the settings' absence to the tier.
        app.update();
        assert!(
            app.world()
                .entity(camera)
                .contains::<ScreenEffectSettings>(),
            "premise: these settings must enrol the camera before a tier can be \
             shown to take it back out"
        );

        let mut potato = crate::quality::ResolvedVisualQuality::default();
        potato.budget.shaders.screen_shader_scale = 0.0;
        app.insert_resource(potato);
        app.update();
        assert!(
            !app.world()
                .entity(camera)
                .contains::<ScreenEffectSettings>(),
            "the tier scaled every effect to nothing, so the pass must not run"
        );
    }
}
