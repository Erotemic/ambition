//! Presentation camera markers shared by host, render, and app wiring.
//!
//! These are Bevy presentation vocabulary, not actor-domain state. Keeping them
//! below the actor crate lets render/host systems agree on camera identity
//! without depending on `ambition_platformer2d_actor_monolith`.

use bevy::prelude::*;

/// `RenderLayers` index the front HUD camera renders sprites from. The gameplay
/// world lives on the default layer 0, so picking a distinct layer here keeps the
/// front camera from double-drawing the world over the cube. (No sprites are placed
/// on this layer; the front camera only carries UI.)
pub const FRONT_HUD_LAYER: usize = 1;

/// `RenderLayers` index for camera-relative parallax panels.
///
/// Portal capture cameras intentionally do not render this layer: the current
/// parallax implementation has one shared sprite transform per layer, synced to
/// the main camera, so rendering it into portal captures samples the background
/// from the wrong eye.
pub const PARALLAX_BACKGROUND_LAYER: usize = 2;

/// **Base of the per-LOCAL-VIEW band**, the layers a second observer's drawn
/// projections live on so one view's nameplates and label copies stay out of the
/// other view's picture.
///
/// ⚠ **this is a LEDGER entry, not a policy.** Which entity gets which layer is
/// decided by the render side (`ambition_render::rendering::view_isolation`);
/// what lives here is the reservation, because a layer collision is silent — two
/// tenants on one index simply draw into each other's cameras and no test asks.
/// The bands already spoken for: 0 the world, 1 the front HUD, 2 the
/// camera-relative parallax, 5 the portal window, 28..=30 twintrack's optical and
/// spacetime overlays, 32+slot the per-portal capture parallax (slot ≤ ~271), and
/// 512+slot the per-portal window self-layers. 1024 clears all of them with room
/// for the portal bands to grow.
///
/// ⭐ **and it grows UPWARD without a ceiling, deliberately.** `RenderLayers` is a
/// growable bitset, so `BASE + n` costs a longer mask and nothing else — the
/// renderer therefore imposes no maximum on how many local views a session may
/// have, which is the property that keeps this an implementation detail rather
/// than a limit the semantic layer inherits.
pub const LOCAL_VIEW_RENDER_LAYER_BASE: usize = 1024;

/// The layer reserved for the `ordinal`-th local view.
///
/// ⛔ **the argument is an ORDINAL AMONG THE LIVE VIEWS, never a `LocalViewId`.**
/// The id is a stable human-meaningful name a game chooses (`LocalViewId(7)` is
/// legal on its own), and binding it to a bit index would make a semantic
/// identity mean a GPU visibility mask. The caller sorts the live views by id and
/// passes the POSITION, so the mapping is dense, private to the renderer, and
/// replaceable without anything semantic noticing.
pub fn local_view_render_layer(ordinal: usize) -> usize {
    LOCAL_VIEW_RENDER_LAYER_BASE + ordinal
}

/// Marks the main gameplay camera (order 0). The cube's dim-scrim looks this up to
/// retarget itself BEHIND the cube.
#[derive(Component)]
pub struct MainCamera;

/// Marks the front HUD/UI camera (order 9) that carries `IsDefaultUiCamera`.
#[derive(Component)]
pub struct FrontHudCamera;

/// The main (order-0) camera entity, stashed at spawn so the dim-scrim can target it
/// with `UiTargetCamera` without an extra query.
#[derive(Resource, Clone, Copy)]
pub struct MainCameraEntity(pub Entity);
