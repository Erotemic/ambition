//! Shared presentation-camera markers and render-layer reservations.

use bevy::prelude::*;

/// Render layer reserved for the front HUD camera; gameplay remains on layer 0.
pub const FRONT_HUD_LAYER: usize = 1;

/// Resting render layer for camera-relative parallax in a single-view session.
///
/// Portal captures and multi-view compositions use isolated copies/layers so a
/// camera never draws another view's camera-relative backdrop.
pub const PARALLAX_BACKGROUND_LAYER: usize = 2;

/// Render layer of the private cameras that composite a part-drawn body into
/// its impostor texture (`ambition_render::rendering::actors::rigged`). Only
/// those cameras draw it, so no view sees a body's loose parts; each body's
/// parts stand in a cell of their own far below any world.
pub const RIGGED_IMPOSTOR_LAYER: usize = 6;

/// Render layer of an entity stamped into a room that is no longer live, while
/// two or more rooms are live. No camera draws it. A room that is not live has
/// no view, and the live rooms share one coordinate space, so on the world
/// layer every camera would draw it over its own room (the particles of a
/// replayed room, which outlive the room by their lifetime).
pub const RETIRED_ROOM_RENDER_LAYER: usize = 7;

/// Base of the render-layer band reserved for isolated local-view projections.
///
/// Lower ranges are reserved for world/HUD/parallax/portal/overlay layers. The
/// band grows upward with the live-view ordinal; `RenderLayers` imposes no fixed
/// semantic limit on the number of local views.
pub const LOCAL_VIEW_RENDER_LAYER_BASE: usize = 1024;

/// Base of the render-layer band for live rooms. While two or more rooms are
/// live, an entity stamped into a live room draws on its room's layer instead
/// of the world layer, and a camera draws the room its view frames. The band
/// is below the portal window band (512) and above the portal capture band
/// (32 + slot).
pub const LIVE_ROOM_RENDER_LAYER_BASE: usize = 256;

/// The highest live-room render layer. More live rooms than the band holds
/// share its last layer.
pub const LIVE_ROOM_RENDER_LAYER_LAST: usize = 511;

/// Render layer for a live-room ordinal (the room's place among the live
/// rooms in instance order), not a `LiveRoomInstance`.
pub fn live_room_render_layer(ordinal: usize) -> usize {
    (LIVE_ROOM_RENDER_LAYER_BASE + ordinal).min(LIVE_ROOM_RENDER_LAYER_LAST)
}

/// The render band of live room `room` among the `live` rooms: its ordinal in
/// instance order. `None` while fewer than two rooms are live (nothing is
/// banded) or when `room` is not live. The room-band pass puts a stamped
/// entity's world layer on this band, and a camera that must see that room's
/// world adds it, so both read this one rule.
pub fn live_room_band(
    live: impl IntoIterator<Item = crate::lifecycle::LiveRoomInstance>,
    room: crate::lifecycle::LiveRoomInstance,
) -> Option<usize> {
    let mut ordered: Vec<_> = live.into_iter().collect();
    ordered.sort();
    ordered.dedup();
    if ordered.len() < 2 {
        return None;
    }
    ordered.iter().position(|live| *live == room).map(live_room_render_layer)
}

/// Render layer for a live-view ordinal, not a semantic `LocalViewId`.
///
/// Callers sort views by id and pass the dense ordinal so render-layer allocation
/// remains private to presentation.
pub fn local_view_render_layer(ordinal: usize) -> usize {
    LOCAL_VIEW_RENDER_LAYER_BASE + ordinal
}

/// Marks a gameplay camera for one local view; this marker is not a singleton.
/// Pair it with `ambition_sim_view::PresentsView` when view identity is required.
/// A gameplay camera blends in the world's compositing space
/// (`ambition_render::rendering::world_compositing`, required where the
/// renderer is composed).
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
