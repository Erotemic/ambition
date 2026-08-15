//! **A LOCAL VIEW: one observer of the simulation, on this machine.**
//!
//! One simulation may publish N observer views — split screen, a spectator, a
//! replay, an inspection window. Today Ambition has exactly one, and the point
//! of this module is that ONE is a count rather than an assumption: the single
//! view is the one-entry case, not a separate architecture that a second view
//! would have to replace.
//!
//! ⛔ **`FeatureViewIndex` is not this.** It is a per-FEATURE render read-model
//! that happens to share the word "view". Local-view identity did not exist at
//! HEAD; nothing needed deleting to make room for it.
//!
//! # What belongs on a view
//!
//! Facts that answer *for this observer*: its viewport rectangle, its framing
//! and safe-area policy, the reference frame it presents in, the snapshot it
//! resolved, and the easing state that snapshot integrates. Those are components
//! on the view entity, so asking them requires naming WHICH view — which is
//! exactly the question a process-global resource cannot be asked.
//!
//! # What does not
//!
//! - `ResolvedGameplayPresentation` is a DISPLAY resolve: one physical screen,
//!   its safe-area insets and control occupancy. It becomes per-view when layout
//!   splits, which is a later phase.
//! - `CameraViewState` is a render-side diagnostic mirror for the debug overlay
//!   and nameplates.
//! - `CameraShakeState`'s per-view semantics are an open feel question (does
//!   shake belong to the view or to the world?), and guessing it here would
//!   answer a design question by refactor.

use bevy::prelude::{Component, Entity};

/// **Marks an entity as one local observer of the simulation.**
///
/// The identity is the ENTITY. There is no side table mapping ids to views and
/// no registry to keep in sync — a view is spawned, queried and despawned like
/// anything else in the world.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalView;

/// A stable, human-meaningful ordinal for a local view.
///
/// ⚠ **not an index into anything.** It exists so a log line, a debug overlay
/// or a saved layout can name a view without holding an `Entity` (which is
/// recycled, and whose bits are not stable across runs). Systems address views
/// by querying, never by looking up an ordinal.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalViewId(pub u8);

impl LocalViewId {
    /// The view a single-view game has. A second view is `LocalViewId(1)`, and
    /// nothing about the first one changes when it appears.
    pub const FIRST: Self = Self(0);
}

/// **The view, for a caller that knows there is exactly one.**
///
/// ⚠ the name is the disclaimer: this asserts the single-view assumption out
/// loud instead of burying it in a `single()` call. Fixtures and diagnostics use
/// it; a system that presents a view must query, because the second view is the
/// whole point of this module existing.
///
/// Panics if there is not exactly one, which is what a caller making this
/// assumption wants — a silent `None` here would be a fixture quietly measuring
/// nothing.
pub fn the_only_view(world: &mut bevy::prelude::World) -> Entity {
    let mut views = world.query_filtered::<Entity, bevy::prelude::With<LocalView>>();
    let found: Vec<Entity> = views.iter(world).collect();
    match found.as_slice() {
        [one] => *one,
        other => panic!(
            "expected exactly one local view, found {}: this helper is only for \
             callers that know the session has a single view",
            other.len()
        ),
    }
}

/// **WHICH VIEW THIS CAMERA PRESENTS** — a camera→view link, stated on the
/// camera.
///
/// ⭐ **the first thing M2 demands, and the reason is a `Single`.**
/// `camera_follow` read the view as `Single<…, With<LocalView>>` and the main
/// camera as a query over `With<MainCamera>`, pairing them by the coincidence
/// that there is one of each. That is not a pairing, it is an assumption of
/// uniqueness at BOTH ends — and the whole point of this module is that the
/// second view is coming. With the link, a camera says what it presents and a
/// second camera can say something different; without it, adding a view silently
/// makes a `Single` panic and adding a camera silently makes two cameras fight
/// over one snapshot.
///
/// ⚠ **bound where the camera is SPAWNED, not resolved per frame.** The binding
/// is a composition decision (which rig presents which view), so it is data on
/// the entity rather than a lookup a draw system repeats.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentsView(pub Entity);

/// **THE CAMERA→VIEW BINDING RULE, WRITTEN ONCE.**
///
/// Three places have to answer *"which view is this camera for"*: the follow
/// camera (`camera_follow`), the physical viewport applier
/// (`apply_gameplay_camera_viewport`), and the draw-side lookup
/// ([`crate::camera_snapshot::PresentedViewState`]). Spelling the rule three
/// times is three chances to disagree, and every disagreement would be SILENT —
/// each site would still resolve *some* view and draw *something*.
///
/// The rule:
///
/// - a camera that NAMES a view presents that one;
/// - a camera that names none, in a composition with exactly ONE view, presents
///   that view — this is every fixture in the tree and every shipped host today,
///   and taking the only view is the honest reading of a single-view
///   composition;
/// - a camera that names none while SEVERAL views exist is refused, loudly.
///   Picking one would be arbitrary, and arbitrary is exactly the process-global
///   "the gameplay view" that D116 M2 deleted.
/// - no views at all is quiet: a headless or pre-composition host has nothing to
///   present and nothing to complain about.
#[derive(Clone, Copy, Debug, Default)]
pub struct ViewsOnHand {
    first: Option<Entity>,
    several: bool,
}

impl ViewsOnHand {
    /// Survey the local views. Takes an iterator rather than a `Query` so the
    /// caller keeps its query free for the per-camera `get_mut` that follows.
    pub fn survey(views: impl IntoIterator<Item = Entity>) -> Self {
        let mut views = views.into_iter();
        let first = views.next();
        Self {
            first,
            several: views.next().is_some(),
        }
    }

    /// The view a camera with this link presents, or `None` — silently when
    /// there is no view at all, loudly when the link is missing and the answer
    /// would have to be a guess.
    pub fn presented_by(&self, link: Option<PresentsView>) -> Option<Entity> {
        match link {
            Some(PresentsView(view)) => Some(view),
            None => {
                let only = self.first?;
                if self.several {
                    bevy::log::error_once!(
                        "several local views exist and a camera names none of them; \
                         refusing to guess which one it presents. Bind `PresentsView` \
                         where the camera is spawned."
                    );
                    return None;
                }
                Some(only)
            }
        }
    }
}

/// Spawn one local view, with the components a camera resolve needs.
///
/// ⛔ **and it carries no `Name`, deliberately.** A `Name` would be a nice debug
/// label and it is REGISTERED FOR ROLLBACK (`entity.name`) — and the rollback
/// coverage contract derives its swept population from *"an entity carrying even
/// one type the rollback knows about is an entity the rollback participates
/// in"*. So labelling this presentation entity enlisted the whole view in the
/// sim sweep, and its ease state was immediately reported as an unrewound desync
/// risk. `LocalViewId` is the identity; the label is not worth the enlistment.
///
/// ⛔⛔ **call this at plugin BUILD time, not from a startup system.** Every
/// reader would otherwise have to tolerate a frame with no view, which in
/// practice means `single()` + `else { return }` — the shape that has produced
/// four production defects in this repository, because a system that silently
/// does nothing is indistinguishable from one that ran. A view that exists
/// before any schedule runs cannot produce that frame.
pub fn spawn_local_view(
    world: &mut bevy::prelude::World,
    id: LocalViewId,
    bundle: impl bevy::prelude::Bundle,
) -> Entity {
    world.spawn((LocalView, id, bundle)).id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_snapshot::{
        CameraObservationPlugin, CameraPresentationInputs, CameraReferenceFrame,
        CameraScreenFraming, CameraViewport, ResolvedCameraSnapshot,
    };
    use ambition_platformer2d_shared_tangle::camera_ease::CameraEaseState;
    use bevy::prelude::*;

    /// **⛔⛔ THE VIEW EXISTS BEFORE ANY SCHEDULE RUNS, CARRYING EVERYTHING THE
    /// RESOLVE REQUIRES.**
    ///
    /// Both halves can only fail silently, which is why they are pinned:
    ///
    /// - spawned from a startup system instead of plugin build, there would be
    ///   one frame with no view, and every publisher and the resolve itself
    ///   would write to nobody — indistinguishable from having run;
    /// - missing ONE component, the view simply does not match the resolve's
    ///   query. Not a compile error, not a panic. An empty iterator, a camera
    ///   frozen at the origin, and nothing in the log.
    ///
    /// The assertion is made without running a single update, because that is
    /// the claim: `App::new()` plus the plugin is already enough.
    #[test]
    fn the_plugin_spawns_one_complete_view_at_build_time() {
        let mut app = App::new();
        app.add_plugins(CameraObservationPlugin);

        let view = the_only_view(app.world_mut());
        let view = app.world().entity(view);
        assert!(view.contains::<LocalViewId>(), "the view has no identity");
        // Every component the resolve's query requires. Adding one to that query
        // without adding it here is exactly the silent exclusion above.
        assert!(view.contains::<CameraViewport>(), "no viewport");
        assert!(view.contains::<CameraScreenFraming>(), "no screen framing");
        assert!(
            view.contains::<CameraPresentationInputs>(),
            "no presentation inputs"
        );
        assert!(
            view.contains::<CameraReferenceFrame>(),
            "no reference-frame policy — the view could not be told how to present"
        );
        assert!(view.contains::<CameraEaseState>(), "no ease state");
        assert!(
            view.contains::<ResolvedCameraSnapshot>(),
            "nowhere to publish the resolved snapshot"
        );
    }

    /// **The frame policy is per-view state that a game can select.**
    ///
    /// D118 landed the whole subject-relative mechanism and left the selection
    /// deliberately unbuilt, because a policy that belongs to a VIEW must not
    /// become a process-global mode. This is that policy having a home: writing
    /// the component is the selection.
    #[test]
    fn a_views_reference_frame_is_a_component_a_game_can_write() {
        let mut app = App::new();
        app.add_plugins(CameraObservationPlugin);
        let view = the_only_view(app.world_mut());

        assert_eq!(
            *app.world()
                .entity(view)
                .get::<CameraReferenceFrame>()
                .unwrap(),
            CameraReferenceFrame::WorldFixed,
            "a view that states nothing must present the way every room does today"
        );

        *app.world_mut()
            .entity_mut(view)
            .get_mut::<CameraReferenceFrame>()
            .unwrap() = CameraReferenceFrame::SubjectFrame;
        assert_eq!(
            *app.world()
                .entity(view)
                .get::<CameraReferenceFrame>()
                .unwrap(),
            CameraReferenceFrame::SubjectFrame
        );
    }
}
