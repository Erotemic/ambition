//! Whole-screen post-processing effects for presentation cameras.
//!
//! This is intentionally not a sprite overlay. The render node runs after the
//! 2D main pass, samples the already-rendered view texture, and writes a
//! fullscreen filtered result back into Bevy's post-process destination. That
//! lets shader toggles distort scene UVs, split color channels, apply scanlines,
//! and modulate luminance in ways an overlay cannot.

use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        FullscreenShader,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
        RenderApp, RenderStartup,
    },
};

use ambition_gameplay_core::persistence::settings::{ScreenShaderSettings, UserSettings};

const SHADER_ASSET_PATH: &str = "shaders/screen_effects.wgsl";

/// Presentation plugin for camera-local screen filters.
pub struct ScreenEffectsPlugin;

impl Plugin for ScreenEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<ScreenEffectSettings>::default(),
            UniformComponentPlugin::<ScreenEffectSettings>::default(),
        ))
        .add_systems(Update, sync_screen_effect_settings_from_video_settings);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(RenderStartup, init_screen_effects_pipeline);
        render_app
            .add_render_graph_node::<ViewNodeRunner<ScreenEffectsNode>>(Core2d, ScreenEffectsLabel)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    ScreenEffectsLabel,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ScreenEffectsLabel;

#[derive(Default)]
struct ScreenEffectsNode;

impl ViewNode for ScreenEffectsNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ScreenEffectSettings,
        &'static DynamicUniformIndex<ScreenEffectSettings>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, _settings, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline = world.resource::<ScreenEffectsPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<ScreenEffectSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();
        let bind_group = render_context.render_device().create_bind_group(
            "screen_effects_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                post_process.source,
                &pipeline.sampler,
                settings_binding.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("screen_effects_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_render_pipeline(render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}

#[derive(Resource)]
struct ScreenEffectsPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

fn init_screen_effects_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "screen_effects_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<ScreenEffectSettings>(true),
            ),
        ),
    );
    let sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let shader = asset_server.load(SHADER_ASSET_PATH);
    let vertex = fullscreen_shader.to_vertex_state();
    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("screen_effects_pipeline".into()),
        layout: vec![layout.clone()],
        vertex,
        fragment: Some(FragmentState {
            shader,
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });

    commands.insert_resource(ScreenEffectsPipeline {
        layout,
        sampler,
        pipeline_id,
    });
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
