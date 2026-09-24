//! Where a block of text goes, for every overlay that shows one.
//!
//! `ResolvedGameplayPresentation::reading_rect()` gives the geometry: the safe
//! display minus everything a reader must not sit behind (thumb-sticks, the
//! action cluster, corner system controls). This module is the bevy_ui side,
//! shared by every panel.
//!
//! This module sets only the root's position and size. Padding, flex
//! direction, and child stacking stay with each panel: those are presentation
//! choices, while "never under a live button" is not.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation;

/// Place `node` in the reading rect, leaving everything else about it alone.
///
/// With no resolver the node is untouched, not zeroed. A composition without
/// the layout resolver (demos that skip `HostGameplayPresentationPlugin`,
/// and one of `capture_scene`'s app builders) must still show its dialogue.
pub fn place_in_reading_rect(node: &mut Node, presentation: Option<&ResolvedGameplayPresentation>) {
    let Some(presentation) = presentation else {
        return;
    };
    let rect = presentation.reading_rect();
    let display = presentation.display_rect;
    node.position_type = PositionType::Absolute;
    node.left = Val::Px(rect.min.x - display.min.x);
    node.top = Val::Px(rect.min.y - display.min.y);
    node.width = Val::Px(rect.size().x);
    node.height = Val::Px(rect.size().y);
    // The authored box used `right`/`bottom` to span the screen. If left set,
    // they conflict with the explicit width and height, and bevy_ui favours
    // the insets (full screen again).
    node.right = Val::Auto;
    node.bottom = Val::Auto;
}

/// Keep every root marked `M` in the reading rect as the layout moves.
pub fn fit_to_reading_rect<M: Component>(
    presentation: Option<Res<ResolvedGameplayPresentation>>,
    mut roots: Query<&mut Node, With<M>>,
) {
    let Some(presentation) = presentation else {
        return;
    };
    if !presentation.is_changed() {
        return;
    }
    for mut node in &mut roots {
        // Build the candidate from the live node and compare, instead of writing
        // every frame: `Node` change detection drives layout, and rewriting an
        // identical box would re-layout the whole text subtree.
        let mut next = node.clone();
        place_in_reading_rect(&mut next, Some(&presentation));
        if *node != next {
            *node = next;
        }
    }
}
