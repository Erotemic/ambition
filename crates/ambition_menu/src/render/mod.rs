//! Renderers for a [`crate::MenuPageModel`].
//!
//! The same backend-agnostic page model can be drawn by multiple presentations.
//! This crate ships the flat, tabbed [`bevy_ui`] renderer; the bevy_lunex 3D
//! OoT-style cube renderer is the optional `ambition_menu_kaleidoscope`
//! extension crate (E1e) — a host installs it to draw the same model as a cube.

pub mod bevy_ui;

/// Host systems that place menu surfaces before camera projection updates.
#[derive(bevy::prelude::SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MenuSurfaceLayout;

pub use ambition_platformer2d_shared_tangle::camera_layers::menu_scene_render_layer;

/// A presentation entity whose lifetime follows its menu without transform inheritance.
#[derive(bevy::prelude::Component)]
#[relationship(relationship_target = MenuPresentations)]
pub struct MenuPresentationOf(pub bevy::prelude::Entity);

/// Presentation roots and cameras owned by one menu.
#[derive(bevy::prelude::Component)]
#[relationship_target(relationship = MenuPresentationOf, linked_spawn)]
pub struct MenuPresentations(Vec<bevy::prelude::Entity>);
