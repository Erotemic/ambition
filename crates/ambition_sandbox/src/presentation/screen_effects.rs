//! Whole-screen post-processing effects for presentation cameras.
//!
//! This is intentionally not a sprite overlay. The render node runs after the
//! 2D main pass, samples the already-rendered view texture, and writes a
//! fullscreen filtered result back into Bevy's post-process destination. That
//! lets presets distort scene UVs, split color channels, apply scanlines, and
//! modulate luminance in ways an overlay cannot.

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
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode,
            ViewNodeRunner,
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

use crate::dev_tools::{DeveloperTools, ScreenEffectPreset};

const SHADER_ASSET_PATH: &str = "shaders/screen_effects.wgsl";

/// Presentation plugin for camera-local screen filters.
pub struct ScreenEffectsPlugin;

impl Plugin for ScreenEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<ScreenEffectSettings>::default(),
            UniformComponentPlugin::<ScreenEffectSettings>::default(),
        ))
        .add_systems(Update, sync_screen_effect_settings_from_developer_tools);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(RenderStartup, init_screen_effects_pipeline);
        render_app
            .add_render_graph_node::<ViewNodeRunner<ScreenEffectsNode>>(
                Core2d,
                ScreenEffectsLabel,
            )
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
/// filter. Three vec4s keep the uniform layout WebGL2-friendly and give the
/// shader room to add POC ingredients without Rust/WGSL layout churn.
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct ScreenEffectSettings {
    /// x = preset id, y = strength, z = elapsed time seconds, w = reserved.
    pub control: Vec4,
    /// x = noise, y = vignette, z = distortion, w = scanlines.
    pub amounts: Vec4,
    /// x = chromatic split, y = rolling tear, z = desaturation, w = ripple.
    pub modulation: Vec4,
}

impl Default for ScreenEffectSettings {
    fn default() -> Self {
        Self::for_preset(ScreenEffectPreset::Off, 0.0, 0.0)
    }
}

impl ScreenEffectSettings {
    pub fn for_preset(preset: ScreenEffectPreset, strength: f32, elapsed_secs: f32) -> Self {
        let strength = if matches!(preset, ScreenEffectPreset::Off) {
            0.0
        } else {
            strength.clamp(0.0, 1.0)
        };

        let (amounts, modulation) = match preset {
            ScreenEffectPreset::Off => (Vec4::ZERO, Vec4::ZERO),
            ScreenEffectPreset::RobotDeathStatic => (
                Vec4::new(0.55, 0.50, 1.00, 0.38),
                Vec4::new(0.0075, 0.90, 0.48, 0.00),
            ),
            ScreenEffectPreset::Crt => (
                Vec4::new(0.08, 0.65, 0.45, 0.65),
                Vec4::new(0.0035, 0.05, 0.08, 0.00),
            ),
            ScreenEffectPreset::Underwater => (
                Vec4::new(0.04, 0.20, 0.95, 0.00),
                Vec4::new(0.0010, 0.00, 0.00, 1.00),
            ),
        };

        Self {
            control: Vec4::new(preset.gpu_id(), strength, elapsed_secs, 0.0),
            amounts,
            modulation,
        }
    }
}

fn sync_screen_effect_settings_from_developer_tools(
    developer: Res<DeveloperTools>,
    time: Res<Time>,
    mut cameras: Query<&mut ScreenEffectSettings>,
) {
    let next = ScreenEffectSettings::for_preset(
        developer.screen_effect_preset,
        developer.screen_effect_strength,
        time.elapsed_secs(),
    );
    for mut settings in &mut cameras {
        *settings = next;
    }
}
