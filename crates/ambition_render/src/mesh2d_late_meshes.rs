//! A 2D mesh whose GPU mesh arrives late is specialized when it arrives.
//!
//! ⛔ THE BEVY 0.19 GAP THIS CLOSES. `specialize_material2d_meshes`
//! (`bevy_sprite_render`, `mesh2d/material.rs`) builds a `Mesh2d` entity's
//! draw pipeline only when the entity is newly visible, changed, or on the
//! retry list. An entity whose MATERIAL is not ready goes on the retry list;
//! an entity whose MESH is not ready does not (`let Some(mesh) = ... else {
//! continue }`). So a mesh that the per-frame GPU upload budget
//! (`render_asset_budget.rs`) holds back on the frame its entity first
//! becomes visible is never drawn, until something else changes the entity
//! or the view.
//!
//! Measured 2026-10-17 in `evil_gate_forest`: at a 2-4 MiB budget one whole
//! 512-unit chunk of the terrain skin (`terrain_skin.rs`) was missing in
//! every run, and stayed missing; at the shipped 16 MiB it was missing on
//! some loads (Jon: "some ground and wall textures are not rendering
//! sometimes ... It's not consistent").
//!
//! The fix: remember each `Mesh2d` instance whose mesh is not prepared, and
//! on the frame its mesh is prepared, mark it changed
//! (`DirtySpecializations::changed_renderables`), so the specialize step
//! builds its pipeline then. Every `Mesh2d` user gains it, not one system.
//! Retire it when Bevy retries a missing mesh as it retries a missing
//! material.

use bevy::prelude::*;
use bevy::render::camera::DirtySpecializations;
use bevy::render::mesh::RenderMesh;
use bevy::render::render_asset::{prepare_assets, RenderAssets};
use bevy::render::sync_world::MainEntityHashSet;
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::sprite_render::RenderMesh2dInstances;

/// Installs the late-mesh respecialization in the render world. A no-op with
/// no render world (headless): nothing is specialized there.
pub struct Mesh2dLateMeshPlugin;

impl Plugin for Mesh2dLateMeshPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            respecialize_meshes_that_arrived
                .after(prepare_assets::<RenderMesh>)
                .before(RenderSystems::Specialize),
        );
    }
}

/// The `Mesh2d` instances whose mesh was not prepared when last seen.
#[derive(Default)]
struct AwaitingMesh(MainEntityHashSet);

fn respecialize_meshes_that_arrived(
    instances: Res<RenderMesh2dInstances>,
    meshes: Res<RenderAssets<RenderMesh>>,
    mut dirty: ResMut<DirtySpecializations>,
    mut awaiting: Local<AwaitingMesh>,
) {
    // An instance that left (despawned, or no longer a 2D mesh) waits for
    // nothing.
    awaiting.0.retain(|entity| instances.contains_key(entity));
    for (entity, instance) in instances.iter() {
        if meshes.get(instance.mesh_asset_id).is_none() {
            awaiting.0.insert(*entity);
        } else if awaiting.0.remove(entity) {
            dirty.changed_renderables.insert(*entity);
        }
    }
}
