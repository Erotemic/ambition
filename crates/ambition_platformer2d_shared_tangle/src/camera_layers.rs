//! Shared presentation-camera markers and render-layer reservations.

use bevy::prelude::*;

/// Render layer reserved for the front HUD camera; gameplay remains on layer 0.
pub const FRONT_HUD_LAYER: usize = 1;

/// Resting render layer for camera-relative parallax in a single-view session.
///
/// Portal captures and multi-view compositions use isolated copies/layers so a
/// camera never draws another view's camera-relative backdrop.
pub const PARALLAX_BACKGROUND_LAYER: usize = 2;

/// Base of the render-layer band reserved for isolated local-view projections.
///
/// Lower ranges are reserved for world/HUD/parallax/portal/overlay layers. The
/// band grows upward with the live-view ordinal; `RenderLayers` imposes no fixed
/// semantic limit on the number of local views.
pub const LOCAL_VIEW_RENDER_LAYER_BASE: usize = 1024;

/// Render layer for a live-view ordinal, not a semantic `LocalViewId`.
///
/// Callers sort views by id and pass the dense ordinal so render-layer allocation
/// remains private to presentation.
pub fn local_view_render_layer(ordinal: usize) -> usize {
    LOCAL_VIEW_RENDER_LAYER_BASE + ordinal
}

/// Marks a gameplay camera for one local view; this marker is not a singleton.
/// Pair it with `ambition_sim_view::PresentsView` when view identity is required.
#[derive(Component)]
pub struct MainCamera;

/// Marks the front HUD/UI camera (order 9) that carries `IsDefaultUiCamera`.
#[derive(Component)]
pub struct FrontHudCamera;

/// Spawn record for single-camera compositions.
///
/// Multi-view code must address cameras through [`MainCamera`] plus
/// `ambition_sim_view::PresentsView`; display-scoped UI should target its own UI camera.
#[derive(Resource, Clone, Copy)]
pub struct MainCameraEntity(pub Entity);

/// Publish [`MainCameraEntity`] for a single-camera composition.
///
/// The first writer wins and a conflicting second writer is reported. The check
/// is queued so multiple startup writers are serialized against the actual world
/// resource rather than each observing it absent before command application.
pub fn publish_main_camera(commands: &mut Commands, camera: Entity) {
    commands.queue(move |world: &mut World| {
        if let Some(existing) = world.get_resource::<MainCameraEntity>() {
            if existing.0 != camera {
                tracing::error!(
                    ?camera,
                    published = ?existing.0,
                    "a second main-camera rig tried to publish itself as THE main \
                     camera. `MainCameraEntity` is a single-camera spawn record — \
                     address several rigs by `MainCamera` plus the view each one \
                     presents."
                );
            }
            return;
        }
        world.insert_resource(MainCameraEntity(camera));
    });
}
