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

/// Bookkeeping for a DEPENDANT drawable, and a separate marker from
/// [`PortalSourceHidden`] for two reasons.
///
/// ⛔⛔ IT MUST NOT JOIN THE `bodies` QUERY'S FILTER. That query takes
/// `&mut Visibility` over exactly the entities carrying a body marker; a
/// dependant carrying one too would be matched by both queries and Bevy refuses
/// the system (B0001). A fourth, dependant-only marker keeps the two populations
/// disjoint by construction.
///
/// ⭐ AND THE RELEASE IS NOT THE SAME AS A BODY'S. When the portal drops a claim
/// on a BODY it asserts no value, because every body it reaches has a per-frame
/// visibility owner that will write the right answer anyway. **A dependant may
/// have no such owner** — the hit-flash overlay's update path says "Visibility
/// stays `Visible` permanently" and only moves its transform, so a dropped claim
/// with no assertion leaves it hidden forever. This marker is what makes
/// restoring safe: the resolver restores ONLY what it took away.
#[derive(Component, Debug, Clone, Copy)]
pub struct PortalDependantHidden;

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
    // ⛔⛔ A BODY'S OTHER DRAWABLES MUST FOLLOW ITS HIDE. The hit-flash
    // silhouette is a separate root mesh that MIRRORS the base sprite, and
    // `overlay_look` already blanks it when its source is `Hidden` -- but
    // `sync_hit_flash_overlays` runs BEFORE portal presentation, so on the frame
    // the portal hides a far-side body the overlay was computed from a VISIBLE
    // source and drew the whole silhouette over the pane.
    // ⚠ ORDERING CANNOT FIX IT: the portal publisher runs `.after(
    // animate_feature_sprites)`, which is itself after the hit-flash mirror in
    // the render chain, so making the mirror run later is a CYCLE.
    // ⇒ This resolver runs late and already owns the answer, so it settles the
    // dependants too. That is what `PresentationOf` was for: the drawable says
    // whose body it draws, and the body's hide reaches it without either side
    // learning about the other.
    dependants: Query<
        (
            Entity,
            &ambition_platformer2d_shared_tangle::lifecycle::PresentationOf,
            Has<PortalDependantHidden>,
            // ⭐ CAN THE COMPOSITOR SEE THIS DRAWABLE ITSELF? Its candidate
            // publisher takes `&Sprite` + `&Transform` with `Without<ChildOf>`
            // and now admits `PresentationOf` drawables — so an unparented sprite
            // dependant is classified FROM ITS OWN GEOMETRY and needs no help
            // from its owner's answer.
            Has<Sprite>,
            Has<ChildOf>,
        ),
        Without<PortalSourceHidden>,
    >,
    mut dependant_visibility: Query<
        &mut Visibility,
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::PresentationOf>,
            // ⛔ ALL THREE REASON MARKERS, or Bevy refuses the system (B0001):
            // the `bodies` query above takes `&mut Visibility` over exactly the
            // entities carrying any of them, so excluding only two leaves an
            // entity both queries could match and the two borrows conflict.
            // Missing `PortalSourceHidden` here panicked seven tests at once.
            Without<PortalTransitHidden>,
            Without<crate::far_side::PortalFarSideHidden>,
            Without<PortalSourceHidden>,
        ),
    >,
) {
    // Which bodies the portal is hiding THIS frame, so their other drawables can
    // be settled in the same pass.
    let mut hidden_bodies: bevy::platform::collections::HashSet<Entity> =
        bevy::platform::collections::HashSet::new();
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
            hidden_bodies.insert(entity);
            if !we_hid_it {
                commands.entity(entity).insert(PortalSourceHidden);
            }
        } else if we_hid_it {
            // ⛔⛔ RELEASE THE CLAIM WITHOUT ASSERTING A VALUE. Writing
            // `Inherited` here assumed that when the portal's reason ends the
            // correct state is VISIBLE -- and other owners have live hide
            // authority over the same component. Morph-ball presentation hides
            // the base `PlayerVisual` while morphed; submerged presentation hides
            // it too. ⇒ A body that stops being far-side on the same frame it
            // morphs was hidden by ITS owner and then RESURRECTED here, because
            // this branch only knew that the portal had once hidden it.
            //
            // ⭐ THE ORDERING IS WHAT MAKES DOING NOTHING CORRECT, not laziness:
            // this resolver runs late in `PortalPresentationSet`, after
            // `sync_visuals` and the other presentation writers have already put
            // THIS FRAME's value in the component. Dropping the claim leaves that
            // value standing, which is exactly what "the portal no longer has an
            // opinion" should mean. Reasserting `Hidden` while a reason STANDS is
            // still required (see above) for the same reason: those writers run
            // every frame and would otherwise win.
            // ⚠ A body nothing else writes therefore stays as it was rather than
            // springing back — correct here, because every population this
            // reaches has a per-frame owner, and a guess would be wrong for the
            // ones that do.
            commands.entity(entity).remove::<PortalSourceHidden>();
        }
    }

    // ⭐ THE DEPENDANTS, AFTER the bodies are settled. Only drawables that name
    // a body are touched, and only while that body is portal-hidden -- a
    // dependant with its own portal reason is excluded by the query, because
    // then it is a source in its own right and the loop above owns it.
    for (drawable, owner, we_hid_it, has_sprite, is_parented) in &dependants {
        // ⛔⛔ A DEPENDANT THE COMPOSITOR CAN CLASSIFY ANSWERS FOR ITSELF, and
        // copying its owner's scalar answer onto it is geometrically WRONG.
        // A tether line whose own pixels are nowhere near the pane was hidden
        // wholesale because the BODY it names overlaps one — body ownership
        // silently became compositing authority. An unparented sprite is exactly
        // the population `publish_portal_compositing_candidates` evaluates, so
        // its own bounds already decide, and a dependant that IS far-side gets
        // its own reason and never reaches this loop at all.
        // ⚠ `is_parented` is not pedantry: the publisher requires
        // `Without<ChildOf>` because it substitutes the LOCAL transform for the
        // world one. A parented sprite is not a candidate, so it still needs the
        // fallback below.
        if has_sprite && !is_parented {
            continue;
        }
        if hidden_bodies.contains(&owner.0) {
            if let Ok(mut visibility) = dependant_visibility.get_mut(drawable) {
                if *visibility != Visibility::Hidden {
                    *visibility = Visibility::Hidden;
                }
            }
            if !we_hid_it {
                commands.entity(drawable).insert(PortalDependantHidden);
            }
        } else if we_hid_it {
            // ⛔⛔ RESTORE, WHICH IS THE OPPOSITE OF THE BODY BRANCH ABOVE, AND
            // FOR THE REASON THAT BRANCH GIVES. Releasing without asserting a
            // value is correct there because "every population this reaches has a
            // per-frame owner". THIS population does not: the hit-flash overlay
            // is spawned `Visible` and its update path states that visibility
            // "stays `Visible` permanently", writing only transform and material.
            // ⇒ Dropping the claim silently would latch it hidden for the rest of
            // the session — one far-side crossing and that character never flashes
            // again. Found by a GPT review 2026-09-06.
            //
            // ⭐ `Inherited`, not `Visible`: it is the "no opinion" value, so a
            // dependant that is someone's child defers to its parent instead of
            // being resurrected over a legitimate hide. At a root it reads as
            // visible, which is what these overlays want.
            if let Ok(mut visibility) = dependant_visibility.get_mut(drawable) {
                if *visibility == Visibility::Hidden {
                    *visibility = Visibility::Inherited;
                }
            }
            commands.entity(drawable).remove::<PortalDependantHidden>();
        }
    }
}
