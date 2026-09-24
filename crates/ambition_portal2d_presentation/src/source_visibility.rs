//! One authority over a portal-presented body's own `Visibility`.
//!
//! Two systems in `PortalPresentationSet` need to hide the source body:
//!
//! * [`crate::visuals::sync_portal_body_pieces`], when it draws clipped transit
//!   pieces; and
//! * [`crate::far_side::composite_far_side_bodies`], for a far-covered body.
//!
//! If both write `Visibility`, the later one wins. For example, a body that was
//! far-covered on frame N and enters `PortalTransit` on frame N+1 would draw
//! the whole sprite over its own slices. An ordering edge would still leave two
//! writers. So each system adds a reason (a marker component), and this
//! resolver is the only writer. A new reason is a new marker.
//!
//! The resolver reverses only its own hides. `PlayerVisual` bodies have other
//! visibility writers (for example submerged presentation in `ambition_render`).
//! [`PortalSourceHidden`] records that the hide was ours, so a body with no
//! portal reason is left alone.

use bevy::prelude::*;

/// A reason: portal transit presentation has replaced this body with clipped
/// pieces, so the whole sprite must not also draw.
#[derive(Component, Debug, Clone, Copy)]
pub struct PortalTransitHidden;

/// Bookkeeping, not a reason: this module applied the current hide, so the
/// resolver restores only what it took away.
#[derive(Component, Debug, Clone, Copy)]
pub struct PortalSourceHidden;

/// Bookkeeping for a dependant drawable. It is separate from
/// [`PortalSourceHidden`] for two reasons.
///
/// It must not match the `bodies` query's filter. That query takes
/// `&mut Visibility` over entities with a body marker, so a dependant with one
/// too would be matched by both queries (B0001).
///
/// The release is different. A body always has a per-frame visibility owner,
/// so releasing a body's claim asserts no value. A dependant may have none: the
/// hit-flash overlay stays `Visible` and only moves its transform. So the
/// resolver restores what it hid, and this marker records that it did.
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
    // A body's other drawables must follow its hide. The hit-flash silhouette is a
    // separate root mesh that mirrors the base sprite, but
    // `sync_hit_flash_overlays` runs before portal presentation, so on the frame
    // the portal hides a far-side body the overlay would still draw. Running the
    // mirror later would create a cycle (the portal publisher runs
    // `.after(animate_feature_sprites)`, which is after the mirror). So this
    // resolver also settles dependants, found through `PresentationOf`.
    dependants: Query<
        (
            Entity,
            &ambition_platformer2d_shared_tangle::lifecycle::PresentationOf,
            Has<PortalDependantHidden>,
            // An unparented sprite dependant is a compositing candidate itself
            // (the publisher admits `PresentationOf` drawables), so it is
            // classified from its own geometry.
            Has<Sprite>,
            // Or declares its frame: a `Mesh2d` overlay with `DeclaredFrame` is
            // composited like a sprite, so it also answers for itself.
            Has<ambition_sprite_fx::DeclaredFrame>,
            Has<ChildOf>,
        ),
        Without<PortalSourceHidden>,
    >,
    mut dependant_visibility: Query<
        &mut Visibility,
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::PresentationOf>,
            // Exclude all three reason markers, or Bevy refuses the system
            // (B0001): the `bodies` query takes `&mut Visibility` over entities
            // with any of them.
            Without<PortalTransitHidden>,
            Without<crate::far_side::PortalFarSideHidden>,
            Without<PortalSourceHidden>,
        ),
    >,
) {
    // Bodies the portal hides this frame, so their other drawables can be
    // settled in the same pass.
    let mut hidden_bodies: bevy::platform::collections::HashSet<Entity> =
        bevy::platform::collections::HashSet::new();
    for (entity, mut visibility, far_side, transit, we_hid_it) in &mut bodies {
        // Any reason hides. Reasons are not ranked, and a body can hold both
        // during the handoff frame.
        let hide = far_side || transit;
        if hide {
            // Reassert every frame, not only on the transition. `sync_visuals`
            // writes `Visible`/`Hidden` for every `FeatureVisual` each frame and
            // runs before portal presentation, so a one-time write would be undone
            // on the next frame.
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            hidden_bodies.insert(entity);
            if !we_hid_it {
                commands.entity(entity).insert(PortalSourceHidden);
            }
        } else if we_hid_it {
            // Release the claim without asserting a value. Other owners hide this
            // body too (morph-ball and submerged presentation). This resolver runs
            // late in `PortalPresentationSet`, after `sync_visuals` and the other
            // writers have set this frame's value, so leaving it keeps their
            // answer. Every body population this reaches has a per-frame owner.
            commands.entity(entity).remove::<PortalSourceHidden>();
        }
    }

    // Dependants, after the bodies. Only drawables that name a body are
    // touched, and only while that body is portal-hidden. A dependant with its
    // own portal reason is a source itself and the loop above owns it.
    for (drawable, owner, we_hid_it, has_sprite, has_declared, is_parented) in &dependants {
        // A dependant that the compositor can classify answers for itself.
        // Copying the owner's answer would hide, for example, a tether line whose
        // own pixels are far from the pane. The publisher requires
        // `Without<ChildOf>` (it uses the local transform as the world one), so a
        // parented sprite is not a candidate and still needs the fallback below.
        if (has_sprite || has_declared) && !is_parented {
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
            // Restore, unlike the body branch. This population has no per-frame
            // owner: the hit-flash overlay is spawned `Visible` and never writes
            // visibility again, so a silent release would leave it hidden for the
            // session. Use `Inherited`, not `Visible`: a dependant that is a child
            // defers to its parent, and at a root it reads as visible.
            if let Ok(mut visibility) = dependant_visibility.get_mut(drawable) {
                if *visibility == Visibility::Hidden {
                    *visibility = Visibility::Inherited;
                }
            }
            commands.entity(drawable).remove::<PortalDependantHidden>();
        }
    }
}
