//! Presentation camera markers shared by host, render, and app wiring.
//!
//! These are Bevy presentation vocabulary, not actor-domain state. Keeping them
//! below the actor crate lets render/host systems agree on camera identity
//! without depending on `ambition_actors`.

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
