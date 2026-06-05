//! Camera layering for the OoT cube-menu overlay (#31).
//!
//! Three render passes stack on the window during the pause menu:
//!
//! * order 0  — [`MainCamera`] (`Camera2d`): the gameplay world (sprites on the
//!   default `RenderLayers::layer(0)`), plus the cube's dim-scrim (explicitly
//!   retargeted to it via `UiTargetCamera`).
//! * order 8  — the cube-menu `Camera3d` (`ambition_inventory_ui::cube::CubePauseCamera`).
//! * order 9  — [`FrontHudCamera`] (`Camera2d`, `clear_color: None`): the DEFAULT UI
//!   camera, so the HUD / FPS / debug / control overlays draw IN FRONT of the cube.
//!
//! The front camera is pinned to [`FRONT_HUD_LAYER`] — a layer the gameplay sprites
//! are NOT on — so it never re-renders the world on top of the cube. bevy_ui resolves
//! each node's camera by `IsDefaultUiCamera` / `UiTargetCamera`, independent of the
//! camera's sprite `RenderLayers`, so UI still renders to the front camera.

use bevy::prelude::*;

/// `RenderLayers` index the front HUD camera renders sprites from. The gameplay
/// world lives on the default layer 0, so picking a distinct layer here keeps the
/// front camera from double-drawing the world over the cube. (No sprites are placed
/// on this layer; the front camera only carries UI.)
pub const FRONT_HUD_LAYER: usize = 1;

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
