//! Shared planar mesh preparation and 2D draw bindings.

use std::marker::PhantomData;

use ambition_platformer2d_shared_tangle::camera_layers::PlanarMeshSource;
use bevy::camera::primitives::{Aabb, MeshAabb};
use bevy::camera::visibility::{RenderLayers, VisibilitySystems};
use bevy::prelude::{
    App, Asset, Assets, Camera, Camera2d, Commands, Component, Entity, FromWorld, Handle,
    IntoScheduleConfigs, Mesh, Mesh2d, PostUpdate, Query, Rectangle, RemovedComponents, Res,
    ResMut, Resource, SystemSet, Transform, Visibility, With, World,
};
use bevy::render::render_asset::RenderAssets;
use bevy::render::sync_world::MainEntityHashMap;
use bevy::render::{Render, RenderApp};
use bevy::sprite_render::{
    AlphaMode2d, Material2d, MeshMaterial2d, PreparedMaterial2d, RenderMaterial2dInstances,
    RenderMesh2dInstances,
};

/// The geometry and material selected for one planar surface.
/// A surface has one selected material. Remove the old typed component
/// when a change selects a different material type.
#[derive(Component)]
#[require(PlanarMeshSource, Transform, Visibility)]
pub struct PreparedPlanarMesh<M: Asset> {
    pub mesh: Handle<Mesh>,
    pub material: Handle<M>,
}

/// One unit quad that planar effects can share.
#[derive(Resource)]
pub struct PlanarUnitQuad {
    mesh: Handle<Mesh>,
    bounds: Aabb,
}

impl FromWorld for PlanarUnitQuad {
    fn from_world(world: &mut World) -> Self {
        let mesh = Mesh::from(Rectangle::default());
        let bounds = mesh.compute_aabb().expect("a unit quad has bounds");
        Self { mesh: world.resource_mut::<Assets<Mesh>>().add(mesh), bounds }
    }
}

impl PlanarUnitQuad {
    /// Supply the bounds before a camera binds the mesh.
    pub fn prepare<M: Asset>(&self, material: Handle<M>) -> (PreparedPlanarMesh<M>, Aabb) {
        (PreparedPlanarMesh { mesh: self.mesh.clone(), material }, self.bounds)
    }
}

#[derive(Resource)]
struct Installed<M: Material2d>(PhantomData<M>);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum PlanarMesh2d {
    Retire,
    Bind,
}

#[derive(Resource, Default)]
pub(super) struct TransparentMeshDepths2d(pub MainEntityHashMap<f32>);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum MeshPaint2d {
    Clear,
    Collect,
}

pub(super) fn clear_mesh_depths(mut depths: ResMut<TransparentMeshDepths2d>) {
    depths.0.clear();
}

fn collect_mesh_depths<M: Material2d>(
    depths: Option<ResMut<TransparentMeshDepths2d>>,
    meshes: Option<Res<RenderMesh2dInstances>>,
    instances: Option<Res<RenderMaterial2dInstances<M>>>,
    materials: Option<Res<RenderAssets<PreparedMaterial2d<M>>>>,
) {
    let (Some(mut depths), Some(meshes), Some(instances), Some(materials)) = (depths, meshes, instances, materials) else {
        return;
    };
    for (entity, id) in instances.iter() {
        let Some(material) = materials.get(*id).filter(|material| material.properties.alpha_mode == AlphaMode2d::Blend) else {
            continue;
        };
        if let Some(mesh) = meshes.get(entity) {
            depths.0.insert(*entity, mesh.transforms.world_from_local.translation.z + material.properties.depth_bias);
        }
    }
}

/// Install the 2D bindings for one material type.
pub fn install_planar_mesh_2d<M: Material2d>(app: &mut App) {
    if app.world().contains_resource::<Installed<M>>() {
        return;
    }
    app.insert_resource(Installed(PhantomData::<M>))
        .configure_sets(
            PostUpdate,
            (PlanarMesh2d::Retire, PlanarMesh2d::Bind)
                .chain()
                .after(bevy::camera::CameraUpdateSystems)
                .after(super::view_isolation::isolate_live_rooms)
                .before(VisibilitySystems::CalculateBounds),
        )
        .add_systems(
            PostUpdate,
            (
                retire_bindings::<M>.in_set(PlanarMesh2d::Retire),
                bind_meshes::<M>.in_set(PlanarMesh2d::Bind),
            ),
        );
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render.add_systems(Render, collect_mesh_depths::<M>.in_set(MeshPaint2d::Collect));
    }
}

fn retire_bindings<M: Material2d>(
    mut commands: Commands,
    mut removed: RemovedComponents<PreparedPlanarMesh<M>>,
    replaced: Query<(), With<MeshMaterial2d<M>>>,
    live: Query<(), With<PreparedPlanarMesh<M>>>,
) {
    // Remove the old bindings before a change of material type can bind again.
    for entity in removed.read() {
        if replaced.contains(entity) && !live.contains(entity) {
            commands
                .entity(entity)
                .try_remove::<(Mesh2d, MeshMaterial2d<M>)>();
        }
    }
}

fn bind_meshes<M: Material2d>(
    mut commands: Commands,
    cameras: Query<(&Camera, Option<&RenderLayers>), With<Camera2d>>,
    sources: Query<(
        Entity,
        &PreparedPlanarMesh<M>,
        Option<&RenderLayers>,
        Option<&Mesh2d>,
        Option<&MeshMaterial2d<M>>,
    )>,
) {
    if sources.is_empty() {
        return;
    }
    let masks: Vec<_> = cameras
        .iter()
        .filter(|(camera, _)| camera.is_active)
        .map(|(_, layers)| layers.unwrap_or_default())
        .collect();
    for (entity, source, layers, mesh, material) in &sources {
        let wanted = masks
            .iter()
            .any(|mask| mask.intersects(layers.unwrap_or_default()));
        if wanted {
            if mesh.is_none_or(|mesh| mesh.0 != source.mesh)
                || material.is_none_or(|material| material.0 != source.material)
            {
                commands.entity(entity).try_insert((
                    Mesh2d(source.mesh.clone()),
                    MeshMaterial2d(source.material.clone()),
                ));
            }
        } else if mesh.is_some() || material.is_some() {
            commands
                .entity(entity)
                .try_remove::<(Mesh2d, MeshMaterial2d<M>)>();
        }
    }
}
