//! Wait for the pipelines that draw a portal's first picture.

use bevy::asset::AssetId;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::core_2d::{AlphaMask2d, Opaque2d, Transparent2d};
use bevy::prelude::{
    App, Entity, Image, IntoScheduleConfigs, Mesh2d, Query, Res, ResMut, Resource, With,
};
use bevy::render::camera::SortedCameras;
use bevy::render::mesh::RenderMesh;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{
    BinnedPhaseItem, BinnedRenderPhase, ViewBinnedRenderPhases, ViewSortedRenderPhases,
};
use bevy::render::render_resource::{CachedRenderPipelineId, PipelineCache};
use bevy::render::renderer::render_system;
use bevy::render::sync_world::{MainEntity, MainEntityHashMap};
use bevy::render::view::{ExtractedView, RenderVisibleEntities, ViewTarget};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};
use bevy::sprite_render::{PendingMeshMaterial2dQueues, RenderMesh2dInstances};

use super::PortalViewRig;

#[derive(Default, Resource)]
struct FirstCaptures {
    targets: MainEntityHashMap<AssetId<Image>>,
    ready: MainEntityHashMap<AssetId<Image>>,
}

pub(crate) fn install(app: &mut App) {
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .init_resource::<FirstCaptures>()
            .add_systems(ExtractSchedule, extract_targets)
            .add_systems(
                Render,
                defer_first_captures
                    .in_set(RenderSystems::Render)
                    .before(render_system),
            );
    }
}

fn extract_targets(
    rigs: Extract<Query<(Entity, &RenderTarget), With<PortalViewRig>>>,
    mut captures: ResMut<FirstCaptures>,
) {
    let FirstCaptures { targets, ready } = &mut *captures;
    targets.clear();
    targets.extend(rigs.iter().filter_map(|(camera, target)| {
        let RenderTarget::Image(target) = target else {
            return None;
        };
        Some((camera.into(), target.handle.id()))
    }));
    ready.retain(|camera, image| targets.get(camera) == Some(image));
}

fn defer_first_captures(
    mut cameras: ResMut<SortedCameras>,
    views: Query<(&MainEntity, &ExtractedView, &RenderVisibleEntities, Option<&ViewTarget>)>,
    mut captures: ResMut<FirstCaptures>,
    transparent: Res<ViewSortedRenderPhases<Transparent2d>>,
    opaque: Res<ViewBinnedRenderPhases<Opaque2d>>,
    masked: Res<ViewBinnedRenderPhases<AlphaMask2d>>,
    pending: Res<PendingMeshMaterial2dQueues>,
    instances: Res<RenderMesh2dInstances>,
    meshes: Res<RenderAssets<RenderMesh>>,
    pipelines: Res<PipelineCache>,
) {
    if captures.ready.len() == captures.targets.len() {
        return;
    }
    let FirstCaptures { targets, ready } = &mut *captures;
    let is_compiled = |id| pipelines.get_render_pipeline(id).is_some();
    // Leave the view in the prepare and queue passes. Its private target
    // stays untouched until the draw passes can fill it.
    cameras.0.retain(|camera| {
        let Ok((main, view, visible, target)) = views.get(camera.entity) else {
            return true;
        };
        let Some(&image) = targets.get(main) else {
            return true;
        };
        if ready.contains_key(main) {
            return true;
        }

        let key = &view.retained_view_entity;
        let materials_pending = pending.get(key).is_some_and(|queue| !queue.current_frame.is_empty());
        let meshes_ready = visible.get::<Mesh2d>().is_none_or(|visible| {
            visible.iter_visible().all(|(_, entity)| {
                instances.get(entity).is_some_and(|mesh| meshes.get(mesh.mesh_asset_id).is_some())
            })
        });
        if target.is_none() || materials_pending || !meshes_ready {
            return false;
        }

        let prepared = opaque.get(key)
            .is_some_and(|phase| bins_ready(phase, |bin| bin.pipeline, is_compiled))
            && masked.get(key)
                .is_some_and(|phase| bins_ready(phase, |bin| bin.pipeline, is_compiled))
            && transparent.get(key)
                .is_some_and(|phase| phase.items.values().all(|draw| is_compiled(draw.pipeline)));
        if prepared {
            ready.insert(*main, image);
        }
        prepared
    });
}

fn bins_ready<Item: BinnedPhaseItem>(
    phase: &BinnedRenderPhase<Item>,
    pipeline: impl Fn(&Item::BinKey) -> CachedRenderPipelineId,
    is_compiled: impl Fn(CachedRenderPipelineId) -> bool,
) -> bool {
    // Bevy's 2D queues use individual draws or batches.
    phase.batchable_meshes.keys()
        .chain(phase.unbatchable_meshes.keys())
        .chain(phase.non_mesh_items.keys())
        .all(|(_, bin)| is_compiled(pipeline(bin)))
}
