//! Readiness of the GPU pass that finishes a cached body image.

use super::ImpostorUnpremultiply;
use bevy::core_pipeline::core_2d::Transparent2d;
use bevy::prelude::*;
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::render::camera::ExtractedCamera;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::render_resource::PipelineCache;
use bevy::render::render_phase::ViewSortedRenderPhases;
use bevy::render::sync_world::MainEntity;
use bevy::render::view::{ExtractedView, ViewTarget};
use bevy::sprite_render::SpecializedMaterial2dPipelineCache;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[derive(Resource, Default)]
pub(super) struct GpuImpostors;

#[derive(Component, Clone, ExtractComponent, Debug)]
pub(super) struct PassReady {
    source: Entity,
    finished: Arc<AtomicBool>,
}

impl PassReady {
    pub(super) fn new(source: Entity) -> Self { Self { source, finished: Arc::default() } }
    pub(super) fn is_ready(&self) -> bool { self.finished.load(Ordering::Acquire) }
    pub(super) fn acknowledge(&self) { self.finished.store(true,Ordering::Release); }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<GpuImpostors>().add_plugins(ExtractComponentPlugin::<PassReady>::default());
    app.sub_app_mut(RenderApp).add_systems(Render, finish_ready_passes.after(RenderSystems::Render));
}

fn finish_ready_passes(
    quads: Query<(&MainEntity, &PassReady)>,
    views: Query<(&MainEntity, &ExtractedView), (With<ExtractedCamera>, With<ViewTarget>)>,
    sources: Res<ViewSortedRenderPhases<Transparent2d>>,
    specialized: Res<SpecializedMaterial2dPipelineCache<ImpostorUnpremultiply>>,
    pipelines: Res<PipelineCache>,
) {
    for (quad, ready) in &quads {
        if ready.is_ready() { continue; }
        let Some((_, source)) = views.iter().find(|(view, _)| view.id() == ready.source) else { continue; };
        let source_ready = sources.get(&source.retained_view_entity).is_some_and(|phase|
            phase.items.values().all(|item| pipelines.get_render_pipeline(item.pipeline).is_some()));
        if !source_ready { continue; }
        let drawn = views.iter().any(|(view, _)| specialized.get(view).and_then(|cache| cache.get(quad))
            .is_some_and(|pipeline| pipelines.get_render_pipeline(*pipeline).is_some()));
        if drawn { ready.acknowledge(); }
    }
}
