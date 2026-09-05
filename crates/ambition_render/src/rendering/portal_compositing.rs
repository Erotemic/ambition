//! Publish the drawables a portal pane may have to composite against.
//!
//! ⛔⛔ THE PORTAL PRESENTATION CRATE CANNOT SEE ORDINARY ACTORS. Its body seams
//! are `PortalSceneBody` (ONE entity — whose sprite is decomposed at the seam)
//! and `PortalAffordanceBody` (whoever operates the portals). An NPC standing
//! behind an aperture is neither, so that crate had no way to know it exists —
//! while THIS crate drew it at `WORLD_Z_DUMMY + 1.0`, above every pane. A
//! far-side actor punching through a seamless window (Jon, 2026-09-05) is
//! invisible from the side that would have to fix it.
//!
//! ⭐ THE HOST PUBLISHES THE FACT, which is the same shape as every other seam
//! that crate exposes: it does not reach into this one, and this one does not
//! learn what a pane is.

use bevy::prelude::*;

/// Publish each drawn actor sprite as a compositing candidate, in ENGINE
/// coordinates.
///
/// ⚠ DRAWN BOUNDS, NOT THE COLLISION BOX. `Sprite::custom_size` is what actually
/// paints; the collision footprint is routinely smaller, and the difference IS
/// the overhang that punches through the window. Publishing the box would build
/// a report that misses the finding it exists for.
///
/// ⚠ SPRITES WITH NO `custom_size` ARE SKIPPED rather than guessed at. A sprite
/// sized by its texture has bounds this system cannot know without the atlas,
/// and inventing one would put a confident wrong rectangle into a diagnostic
/// whose whole job is to be trusted.
pub fn publish_portal_compositing_candidates(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    drawables: Query<(Entity, &Sprite, &GlobalTransform), With<crate::rendering::primitives::FeatureVisual>>,
) {
    // ⚠ `SessionWorldRef` is a `Single`, so this system simply does not run
    // without a session world -- which is the honest behaviour: there is no
    // coordinate frame to publish engine positions in.
    let size = world.0.size;
    for (entity, sprite, transform) in &drawables {
        let Some(drawn) = sprite.custom_size else {
            continue;
        };
        let bevy_centre = transform.translation().truncate();
        // ⭐ The ONE definition of the y-flip, called rather than repeated.
        let centre =
            ambition_platformer2d_core::config::bevy_size_to_world(size, bevy_centre);
        commands
            .entity(entity)
            .insert(ambition_portal2d_presentation::PortalCompositingCandidate {
                drawn_centre: centre,
                // ⚠ The half-extent is orientation-free: a y-flip moves a CENTRE,
                // never a size.
                drawn_half: drawn * 0.5,
            });
    }
}
