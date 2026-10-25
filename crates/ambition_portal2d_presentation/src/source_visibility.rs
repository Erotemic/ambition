//! One authority over a portal-presented body's own `Visibility`.
//!
//! [`crate::far_side::composite_far_side_bodies`] hides a far-covered body: it
//! adds a reason (a marker component), and this resolver is the only writer
//! of the body's `Visibility`. A new reason is a new marker. (What a portal
//! pair carries is taken off the world's layers, not hidden: `crate::carry`.)
//!
//! The resolver reverses only its own hides. `PlayerVisual` bodies have other
//! visibility writers (for example submerged presentation in `ambition_render`).
//! [`PortalSourceHidden`] records that the hide was ours, so a body with no
//! portal reason is left alone.

use bevy::prelude::*;

/// Bookkeeping, not a reason: this module applied the current hide, so the
/// resolver restores only what it took away.
///
/// ⛔ IT SAYS THE PORTAL HID THE BODY, NOT THAT NOTHING ELSE DOES. Another
/// owner (death, a morph, submerged presentation) can hide the body while it
/// carries this marker, and a reader that took the marker as "visible but for
/// the portal" drew far-side pieces of a dead body. So the marker keeps the
/// change tick of the resolver's own `Hidden` write: a later write to
/// `Visibility` is another owner's answer ([`Self::portal_alone_hides`]).
#[derive(Component, Debug, Clone, Copy)]
pub struct PortalSourceHidden {
    /// The change tick of the resolver's last `Hidden` write. `None`: the body
    /// was already `Hidden` when the portal claimed it, so another owner hid
    /// it.
    pub wrote: Option<bevy::ecs::change_detection::Tick>,
}

impl PortalSourceHidden {
    /// The drawable is `Hidden` by the portal alone: the portal hid it, and
    /// nothing wrote its `Visibility` since. Then the clipped pieces are the
    /// body, and it is still a compositing candidate. `false` for a drawable
    /// the portal did not hide, or one another owner wrote after it.
    pub fn portal_alone_hides(marker: Option<&Self>, visibility: &Ref<Visibility>) -> bool {
        matches!(**visibility, Visibility::Hidden)
            && marker.is_some_and(|marker| marker.wrote == Some(visibility.last_changed()))
    }
}

/// Bookkeeping for a dependant drawable. It is separate from
/// [`PortalSourceHidden`] for two reasons.
///
/// It must not match the `bodies` query's filter. That query takes
/// `&mut Visibility` over entities with a body marker, so a dependant with one
/// too would be matched by both queries (B0001).
///
/// The release is different. A body usually has a per-frame visibility owner,
/// so releasing a body's claim restores only a hide nothing wrote over (a
/// shot's visual has no such owner). A dependant may have none: the
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
            Option<&mut PortalSourceHidden>,
        ),
        Or<(With<crate::far_side::PortalFarSideHidden>, With<PortalSourceHidden>)>,
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
            // Exclude both markers, or Bevy refuses the system (B0001): the
            // `bodies` query takes `&mut Visibility` over entities with either.
            Without<crate::far_side::PortalFarSideHidden>,
            Without<PortalSourceHidden>,
        ),
    >,
) {
    // Bodies the portal hides this frame, so their other drawables can be
    // settled in the same pass.
    let mut hidden_bodies: bevy::platform::collections::HashSet<Entity> =
        bevy::platform::collections::HashSet::new();
    for (entity, mut visibility, far_side, mut ours) in &mut bodies {
        let hide = far_side;
        if hide {
            // Reassert every frame, not only on the transition. `sync_visuals`
            // writes `Visible`/`Hidden` for every `FeatureVisual` each frame and
            // runs before portal presentation, so a one-time write would be undone
            // on the next frame.
            //
            // Keep the tick of our own write only. A body another owner left
            // `Hidden` this frame is not ours to vouch for.
            let wrote = (*visibility != Visibility::Hidden).then(|| {
                *visibility = Visibility::Hidden;
                visibility.last_changed()
            });
            hidden_bodies.insert(entity);
            match ours.as_deref_mut() {
                Some(marker) => {
                    if wrote.is_some() {
                        marker.wrote = wrote;
                    }
                }
                None => {
                    commands.entity(entity).insert(PortalSourceHidden { wrote });
                }
            }
        } else if let Some(marker) = ours {
            // Release the claim. Other owners hide this body too (morph-ball and
            // submerged presentation). This resolver runs late in
            // `PortalPresentationSet`, after `sync_visuals` and the other writers
            // have set this frame's value, so a value written since our hide is
            // their answer and is kept. A hide nothing wrote over is restored: a
            // shot's visual has no per-frame owner, and stayed hidden for the
            // rest of its flight after it crossed (measured with the seam lab,
            // every pair).
            if *visibility == Visibility::Hidden && marker.wrote == Some(visibility.last_changed()) {
                *visibility = Visibility::Inherited;
            }
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
