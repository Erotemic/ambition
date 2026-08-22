//! Where a block of text goes, for every overlay that shows one.
//!
//! `ResolvedGameplayPresentation::reading_rect()` answers the geometry: the safe
//! display carved back from everything a reader must not sit behind — the
//! thumb-sticks, the action cluster, the corner system controls. This module is
//! the bevy_ui side of that answer, and it exists because there is more than one
//! panel.
//!
//! the split of responsibility is the point. This module sets only the
//! root's POSITION and SIZE. Padding, flex direction, and how children stack
//! stay with each panel, because "the dialogue justifies its children to the
//! start and the cutscene spreads them apart" is a presentation decision and
//! "neither may sit under a live button" is not.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation;

/// Place `node` in the reading rect, leaving everything else about it alone.
///
/// no resolver means the node is UNTOUCHED, not zeroed. A composition without the layout
/// resolver — every demo that skips `HostGameplayPresentationPlugin`, and one of
/// `capture_scene`'s two app builders — must still show its dialogue.
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
    // The authored box used `right`/`bottom` to span the screen. Left set, they
    // fight the explicit width/height and bevy_ui resolves the conflict in
    // favour of the insets — which is the full screen again, silently.
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
        // build the candidate from the LIVE node and compare, rather than
        // writing unconditionally: `Node` drives bevy_ui's layout pass through
        // change detection, and rewriting an identical box every frame would
        // relayout the whole text subtree for nothing.
        let mut next = node.clone();
        place_in_reading_rect(&mut next, Some(&presentation));
        if *node != next {
            *node = next;
        }
    }
}
