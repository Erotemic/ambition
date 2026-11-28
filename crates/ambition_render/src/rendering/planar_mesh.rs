//! Shared planar mesh preparation and 2D draw bindings.

use std::marker::PhantomData;

use bevy::camera::visibility::{RenderLayers, VisibilitySystems};
use bevy::prelude::{
    App, Asset, Camera, Camera2d, Commands, Component, Entity, Handle, IntoScheduleConfigs, Mesh,
    Mesh2d, PostUpdate, Query, RemovedComponents, Resource, SystemSet, Transform, Visibility, With,
};
use bevy::sprite_render::{Material2d, MeshMaterial2d};

/// A source that supplies its mesh before either renderer draws it.
#[derive(Component, Default)]
pub(super) struct PlanarMeshSource;

/// The geometry and material selected for one planar surface.
/// A surface has one selected material. Remove the old typed component
/// when a change selects a different material type.
#[derive(Component)]
#[require(PlanarMeshSource, Transform, Visibility)]
pub struct PreparedPlanarMesh<M: Asset> {
    pub mesh: Handle<Mesh>,
    pub material: Handle<M>,
}

#[derive(Resource)]
struct Installed<M: Material2d>(PhantomData<M>);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum PlanarMesh2d {
    Retire,
    Bind,
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
