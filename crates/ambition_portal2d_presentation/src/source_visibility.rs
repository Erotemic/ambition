//! ONE authority over a portal-presented body's own `Visibility`.
//!
//! ⛔⛔ THE DEFECT THIS EXISTS FOR, 2026-09-05. Two systems in
//! `PortalPresentationSet` each wrote the source body's `Visibility` directly,
//! with no order between them:
//!
//! * [`crate::visuals::sync_portal_body_pieces`] set `Inherited`
//!   UNCONDITIONALLY at the top of every run, then `Hidden` again if it drew
//!   clipped transit pieces; and
//! * [`crate::far_side::composite_far_side_bodies`] set `Hidden` for a
//!   far-covered body and `Inherited` when it stopped being one.
//!
//! ⇒ Whichever ran later won. A body that was far-covered on frame N and enters
//! `PortalTransit` on frame N+1 is exactly the collision: the transit splitter
//! hides the source because the pieces ARE the body now, while the far-side
//! compositor sees a transiting body, concludes the hide is no longer its
//! responsibility, and gives the whole sprite back — drawing the unsplit body on
//! top of its own slices for that frame. The far-side hide was ALSO clobbered
//! every ordinary frame by the unconditional `Inherited`.
//!
//! ⭐ THE FIX IS NOT AN ORDERING EDGE. Ordering the two systems would leave two
//! writers of one fact and make the correct picture depend on remembering which
//! way. Instead each system now states a REASON — a marker component — and this
//! resolver is the only thing that writes `Visibility`. Adding a third reason
//! later is a marker, not a new negotiation between existing writers.
//!
//! ⚠ IT ONLY REVERSES ITS OWN HIDES, and that is load-bearing rather than
//! tidiness: `PlayerVisual` bodies have other legitimate visibility writers (a
//! submerged body dims/hides through `ambition_render`'s own pass). Restoring
//! `Inherited` for a body this module never hid would silently overrule them.
//! [`PortalSourceHidden`] records that the hide was ours, so a body with no
//! portal reason is left completely alone.

use bevy::prelude::*;

/// A reason: portal TRANSIT presentation has replaced this body with clipped
/// pieces, so the whole sprite must not also draw.
#[derive(Component, Debug, Clone, Copy)]
pub struct PortalTransitHidden;

/// Bookkeeping: this module applied the current hide. NOT a reason — the record
/// that lets the resolver restore only what it took away.
#[derive(Component, Debug, Clone, Copy)]
pub struct PortalSourceHidden;

/// Resolve every hide reason into the one `Visibility` write.
///
/// Runs after every system that states a reason; the ordering edges are declared
/// in [`crate::plugin`], and `Update`'s default settings put an `ApplyDeferred`
/// on those edges, so a reason inserted through `Commands` this frame is visible
/// here on the same frame rather than one late.
pub fn resolve_portal_source_visibility(
    mut commands: Commands,
    mut bodies: Query<
        (
            Entity,
            &mut Visibility,
            Has<crate::far_side::PortalFarSideHidden>,
            Has<PortalTransitHidden>,
            Has<PortalSourceHidden>,
        ),
        Or<(
            With<crate::far_side::PortalFarSideHidden>,
            With<PortalTransitHidden>,
            With<PortalSourceHidden>,
        )>,
    >,
) {
    for (entity, mut visibility, far_side, transit, we_hid_it) in &mut bodies {
        // ⭐ ANY reason hides. The reasons are deliberately not ranked: "the
        // pieces are the body" and "the far side draws it" are both complete
        // arguments for not drawing the whole sprite, and a body can hold both
        // during the handoff frame that motivated this module.
        let hide = far_side || transit;
        if hide {
            // ⛔⛔ REASSERTED EVERY FRAME, NOT ONLY ON THE TRANSITION. The first
            // version wrote `Hidden` only when the marker was absent, which is
            // correct ONLY if nothing else writes this component. `sync_visuals`
            // does: it sets `*visibility = if view.visible { Visible } else {
            // Hidden }` unconditionally for EVERY `FeatureVisual`, every frame,
            // and runs before portal presentation.
            // ⇒ A stationary far-side NPC was hidden on frame N and VISIBLE
            // again on frame N+1 -- the exact punch-through this compositor
            // exists to remove, restored one frame later. Found by a GPT review
            // 2026-09-05; the crate's own tests could not see it because they
            // step a single frame and do not compose `sync_visuals`.
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            if !we_hid_it {
                commands.entity(entity).insert(PortalSourceHidden);
            }
        } else if we_hid_it {
            *visibility = Visibility::Inherited;
            commands.entity(entity).remove::<PortalSourceHidden>();
        }
    }
}
