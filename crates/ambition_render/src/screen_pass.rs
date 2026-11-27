//! Camera post-processing with a separate pipeline and bindings for each effect.

use std::{collections::HashMap, marker::PhantomData};

use bevy::{
    core_pipeline::{schedule::Core2d, tonemapping::tonemapping, Core2dSystems, FullscreenShader},
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            encase::internal::WriteInto,
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            BufferId, CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState,
            Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, TextureFormat, TextureSampleType, TextureView, TextureViewId, VertexState,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::ViewTarget,
        Render, RenderApp, RenderStartup, RenderSystems,
    },
    shader::Shader,
};

#[cfg(feature = "card_view")]
use bevy::core_pipeline::schedule::{Core3d, Core3dSystems};

pub(crate) mod fields;

pub(crate) trait ScreenPass:
    Component + ExtractComponent + Clone + ShaderType + WriteInto
{
    const SHADER: &'static str;
    const STAGE: ScreenPassStage;
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ScreenPassStage {
    Spirit,
    Void,
    Filter,
}

pub(crate) struct ScreenPassPlugin<T>(PhantomData<fn() -> T>);

impl<T> Default for ScreenPassPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: ScreenPass> Plugin for ScreenPassPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<T>::default(),
            UniformComponentPlugin::<T>::default(),
        ));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        use ScreenPassStage::{Filter, Spirit, Void};
        render_app
            .configure_sets(
                Core2d,
                (Spirit, Void, Filter)
                    .chain()
                    .in_set(Core2dSystems::PostProcess)
                    .after(tonemapping),
            )
            .add_systems(RenderStartup, init_pipeline::<T>)
            .add_systems(
                Render,
                prepare_passes::<T>.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(Core2d, draw_pass::<T>.in_set(T::STAGE));
        #[cfg(feature = "card_view")]
        render_app
            .configure_sets(
                Core3d,
                (Spirit, Void, Filter)
                    .chain()
                    .in_set(Core3dSystems::PostProcess)
                    .after(tonemapping),
            )
            .add_systems(Core3d, draw_pass::<T>.in_set(T::STAGE));
    }
}

#[derive(Resource)]
struct PassPipeline<T> {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    shader: Handle<Shader>,
    vertex: VertexState,
    formats: HashMap<TextureFormat, CachedRenderPipelineId>,
    effect: PhantomData<fn() -> T>,
}

fn init_pipeline<T: ScreenPass>(
    mut commands: Commands,
    device: Res<RenderDevice>,
    assets: Res<AssetServer>,
    fullscreen: Res<FullscreenShader>,
) {
    commands.insert_resource(PassPipeline::<T> {
        layout: BindGroupLayoutDescriptor::new(
            T::SHADER,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<T>(true),
                ),
            ),
        ),
        sampler: device.create_sampler(&SamplerDescriptor::default()),
        shader: assets.load(T::SHADER),
        vertex: fullscreen.to_vertex_state(),
        formats: HashMap::new(),
        effect: PhantomData,
    });
}

/// Bevy 0.19 shares one pipeline ID component among all fullscreen materials.
/// Keep this record typed so one effect can only change its own pass.
#[derive(Component)]
struct PreparedPass<T> {
    pipeline: CachedRenderPipelineId,
    buffer: BufferId,
    bindings: [(TextureViewId, BindGroup); 2],
    effect: PhantomData<fn() -> T>,
}

fn prepare_passes<T: ScreenPass>(
    mut commands: Commands,
    mut pipeline: ResMut<PassPipeline<T>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    uniforms: Res<ComponentUniforms<T>>,
    cameras: Query<
        (Entity, &ViewTarget, Option<&T>, Option<&PreparedPass<T>>),
        Or<(With<T>, With<PreparedPass<T>>)>,
    >,
) {
    let buffer = uniforms.uniforms().buffer();
    let binding = uniforms.uniforms().binding();
    for (camera, target, effect, previous) in &cameras {
        let (Some(_), Some(buffer), Some(binding)) = (effect, buffer, binding.as_ref()) else {
            if previous.is_some() {
                commands.entity(camera).remove::<PreparedPass<T>>();
            }
            continue;
        };
        let views = [target.main_texture_view(), target.main_texture_other_view()];
        if previous.is_some_and(|pass| {
            pass.buffer == buffer.id()
                && views
                    .iter()
                    .all(|view| pass.bindings.iter().any(|entry| entry.0 == view.id()))
        }) {
            continue;
        }
        let format = target.main_texture_format();
        let id = if let Some(id) = pipeline.formats.get(&format) {
            *id
        } else {
            let id = cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(T::SHADER.into()),
                layout: vec![pipeline.layout.clone()],
                vertex: pipeline.vertex.clone(),
                fragment: Some(FragmentState {
                    shader: pipeline.shader.clone(),
                    targets: vec![Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                    ..default()
                }),
                ..default()
            });
            pipeline.formats.insert(format, id);
            id
        };
        let bind = |view: &TextureView| {
            let group = device.create_bind_group(
                T::SHADER,
                &cache.get_bind_group_layout(&pipeline.layout),
                &BindGroupEntries::sequential((view, &pipeline.sampler, binding.clone())),
            );
            (view.id(), group)
        };
        commands.entity(camera).insert(PreparedPass::<T> {
            pipeline: id,
            buffer: buffer.id(),
            bindings: views.map(bind),
            effect: PhantomData,
        });
    }
}

fn draw_pass<T: ScreenPass>(
    view: ViewQuery<(&ViewTarget, &T, &DynamicUniformIndex<T>, &PreparedPass<T>)>,
    cache: Res<PipelineCache>,
    mut context: RenderContext,
) {
    let (target, _, uniform, pass) = view.into_inner();
    let Some(pipeline) = cache.get_render_pipeline(pass.pipeline) else {
        return;
    };
    let source = target.main_texture_view().id();
    let Some((_, binding)) = pass.bindings.iter().find(|entry| entry.0 == source) else {
        return;
    };
    let write = target.post_process_write();
    let mut render_pass = context
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some(T::SHADER),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: write.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            ..default()
        });
    render_pass.set_pipeline(pipeline);
    render_pass.set_bind_group(0, binding, &[uniform.index()]);
    render_pass.draw(0..3, 0..1);
}

/// The view of `camera` as a part of its target: its top left and its size,
/// each from 0 to 1. The whole target when the camera states no part.
pub(crate) fn view_in_target(camera: &Camera) -> Vec4 {
    match (camera.logical_viewport_rect(), camera.logical_target_size()) {
        (Some(rect), Some(target))
            if target.x > 0.0 && target.y > 0.0 && rect.width() > 0.0 && rect.height() > 0.0 =>
        {
            Vec4::new(
                rect.min.x / target.x,
                rect.min.y / target.y,
                rect.width() / target.x,
                rect.height() / target.y,
            )
        }
        _ => WHOLE_TARGET,
    }
}

/// A view that is its whole target.
pub(crate) const WHOLE_TARGET: Vec4 = Vec4::new(0.0, 0.0, 1.0, 1.0);
