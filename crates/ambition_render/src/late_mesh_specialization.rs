//! Request specialization when the GPU receives a delayed mesh.
//!
//! Bevy 0.19 retries specialization for a missing material. If the mesh is
//! missing, it does not retry until the entity or view changes. The upload
//! budget can delay a mesh past the first frame that shows the entity.

use bevy::asset::AssetId;
#[cfg(feature = "card_view")]
use bevy::pbr::RenderMeshInstances;
use bevy::prelude::{App, IntoScheduleConfigs, Local, Mesh, Plugin, Res, ResMut};
use bevy::render::camera::DirtySpecializations;
use bevy::render::mesh::RenderMesh;
use bevy::render::render_asset::RenderAssets;
use bevy::render::sync_world::{MainEntity, MainEntityHashSet};
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::sprite_render::RenderMesh2dInstances;

pub struct LateMeshSpecializationPlugin;

impl Plugin for LateMeshSpecializationPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.add_systems(
                Render,
                respecialize_meshes_that_arrived
                    .after(RenderSystems::PrepareMeshes)
                    .before(RenderSystems::Specialize),
            );
        }
    }
}

fn respecialize_meshes_that_arrived(
    plane: Res<RenderMesh2dInstances>,
    #[cfg(feature = "card_view")] spatial: Option<Res<RenderMeshInstances>>,
    meshes: Res<RenderAssets<RenderMesh>>,
    mut dirty: ResMut<DirtySpecializations>,
    mut plane_waiting: Local<MainEntityHashSet>,
    #[cfg(feature = "card_view")] mut spatial_waiting: Local<MainEntityHashSet>,
) {
    plane_waiting.retain(|entity| plane.contains_key(entity));
    retry_uploaded_meshes(
        &mut plane_waiting,
        plane.iter().map(|(&entity, instance)| (entity, instance.mesh_asset_id)),
        &meshes,
        &mut dirty,
    );

    #[cfg(feature = "card_view")]
    if let Some(spatial) = spatial.as_deref() {
        spatial_waiting.retain(|&entity| spatial.mesh_asset_id(entity).is_some());
        match spatial {
            RenderMeshInstances::CpuBuilding(instances) => retry_uploaded_meshes(
                &mut spatial_waiting,
                instances.iter().map(|(&entity, instance)| (entity, instance.mesh_asset_id())),
                &meshes,
                &mut dirty,
            ),
            RenderMeshInstances::GpuBuilding(instances) => retry_uploaded_meshes(
                &mut spatial_waiting,
                instances.iter().map(|(&entity, instance)| (entity, instance.mesh_asset_id())),
                &meshes,
                &mut dirty,
            ),
        }
    }
}

fn retry_uploaded_meshes(
    waiting: &mut MainEntityHashSet,
    instances: impl Iterator<Item = (MainEntity, AssetId<Mesh>)>,
    meshes: &RenderAssets<RenderMesh>,
    dirty: &mut DirtySpecializations,
) {
    for (entity, mesh) in instances {
        if meshes.get(mesh).is_none() {
            waiting.insert(entity);
        } else if waiting.remove(&entity) {
            dirty.changed_renderables.insert(entity);
        }
    }
}
