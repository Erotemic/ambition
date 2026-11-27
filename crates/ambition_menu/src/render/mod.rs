//! Renderers for a [`crate::MenuPageModel`].
//!
//! The same backend-agnostic page model can be drawn by multiple presentations.
//! This crate ships the flat, tabbed [`bevy_ui`] renderer; the bevy_lunex 3D
//! OoT-style cube renderer is the optional `ambition_menu_kaleidoscope`
//! extension crate (E1e) — a host installs it to draw the same model as a cube.

pub mod bevy_ui;

/// Match existing page nodes to the next model. Keep text when other nodes move or disappear.
#[doc(hidden)]
pub fn match_page_nodes<Action: PartialEq>(
    previous: &[Option<&crate::MenuNode<Action>>],
    next: &[crate::MenuNode<Action>],
) -> Vec<Option<usize>> {
    use crate::MenuNode;
    use std::collections::{HashMap, VecDeque};

    let mut matches = vec![None; next.len()];
    let mut used = vec![false; previous.len()];
    for (index, (before, after)) in previous.iter().zip(next).enumerate() {
        if *before == Some(after) {
            matches[index] = Some(index);
            used[index] = true;
        }
    }
    let mut lines: HashMap<&str, VecDeque<usize>> = HashMap::new();
    for (index, node) in previous.iter().enumerate() {
        if let Some(MenuNode::Text { text, .. }) = node.filter(|_| !used[index]) {
            lines.entry(text).or_default().push_back(index);
        }
    }
    for (index, node) in next.iter().enumerate() {
        if matches[index].is_none() {
            if let MenuNode::Text { text, .. } = node {
                if let Some(before) = lines.get_mut(text.as_str()).and_then(VecDeque::pop_front) {
                    matches[index] = Some(before);
                    used[before] = true;
                }
            }
        }
    }
    for (index, matched) in matches.iter_mut().enumerate() {
        if matched.is_none() && previous.get(index).is_some_and(Option::is_some) && !used[index] {
            *matched = Some(index);
        }
    }
    matches
}

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
