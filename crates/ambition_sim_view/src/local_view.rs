//! Per-observer presentation state for one local simulation view.
//!
//! View entities own observer-specific viewport, framing/reference-frame, and
//! snapshot/easing state. Display-wide presentation and diagnostic mirrors remain
//! separate resources.

use bevy::prelude::{Component, Entity};

/// Marks an entity as one local observer of the simulation.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalView;

/// Stable ordinal for diagnostics and saved layout; systems still address views
/// by querying their entities.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalViewId(pub u8);

impl LocalViewId {
    /// The view a single-view game has. A second view is `LocalViewId(1)`, and
    /// nothing about the first one changes when it appears.
    pub const FIRST: Self = Self(0);
}

/// Fractional placement of one view inside the resolved gameplay rectangle.
///
/// Layout policy writes this component; it stores only the resulting rectangle.
/// Absence/default means the full gameplay area.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ViewPlacement {
    /// Top-left corner, as a fraction of the gameplay rectangle.
    pub min: ambition_platformer2d_core::Vec2,
    /// Bottom-right corner, as a fraction of the gameplay rectangle.
    pub max: ambition_platformer2d_core::Vec2,
}

impl Default for ViewPlacement {
    fn default() -> Self {
        Self::FULL
    }
}

impl ViewPlacement {
    /// The whole gameplay rectangle — one view, no layout.
    pub const FULL: Self = Self {
        min: ambition_platformer2d_core::Vec2::ZERO,
        max: ambition_platformer2d_core::Vec2::ONE,
    };

    /// Vertical slice `index` of `of`, left to right. Degenerate counts return
    /// `FULL`; out-of-range indices clamp to the final column.
    pub fn column(index: usize, of: usize) -> Self {
        if of <= 1 {
            return Self::FULL;
        }
        let of = of as f32;
        let index = (index.min(usize::from(u8::MAX)) as f32).min(of - 1.0);
        Self {
            min: ambition_platformer2d_core::Vec2::new(index / of, 0.0),
            max: ambition_platformer2d_core::Vec2::new((index + 1.0) / of, 1.0),
        }
    }

    /// Resolve this fractional placement inside `(origin, size)` logical pixels.
    /// Output size is clamped to at least one pixel per axis.
    pub fn carve(
        self,
        origin: ambition_platformer2d_core::Vec2,
        size: ambition_platformer2d_core::Vec2,
    ) -> (
        ambition_platformer2d_core::Vec2,
        ambition_platformer2d_core::Vec2,
    ) {
        let min = self.min.min(self.max);
        let max = self.min.max(self.max);
        (
            origin + min * size,
            ((max - min) * size).max(ambition_platformer2d_core::Vec2::ONE),
        )
    }
}

/// Explicit body framed by a view. This is presentation policy, not control
/// authority, and may name an undriven or spectated body.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewSubject(pub Entity);

/// Seat followed by a view. Keep this type aligned with the identifier used there.
///
///  [`ViewSubject`] still wins where both are present, and still exists. Following one
/// named body is a real policy and not a mistake: spectator cameras, cutscenes, a portal's far
/// side.
///
///  presentation only, and it is NOT control authority. This says where a
/// camera looks; `DrivingParticipant` says who is driving. Collapsing the two
/// would make every spectator view a possession.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewParticipant(pub ambition_characters::control::PlayerSlot);

/// THE BODY THIS VIEW IS LOOKING AT, resolved once per frame from whichever
/// way the view declared it.
///
///  the two ways a view can name a subject collapse into ONE fact here. A
/// view declares a body ([`ViewSubject`]) or a seat ([`ViewParticipant`]); which
/// of those it used, and where that seat's body currently is, are questions with
/// a single answer, and answering them is not camera geometry.
///
///  the camera resolve must not search control authority itself, and it
/// did. `resolve_camera_observation` carried a `DrivingParticipant` query
/// folded into another parameter's tuple, with a comment explaining that the
/// system sits at Bevy's 16-parameter ceiling. Packing satisfies the limit
/// without reducing what the system knows about: a 600-line easing resolver that
/// can also answer *who is driving* has two jobs, and the second one grows.
///
/// `None` means the view named nothing resolvable, and frames the session's
/// default subject.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResolvedViewSubject(pub Option<Entity>);

/// RESOLVE EVERY VIEW'S SUBJECT, before anything frames one.
///
///  an explicitly NAMED body wins over a seat. Both are legitimate and they
/// answer different questions — *watch this thing* and *watch whoever drives
/// this seat* — so a view carrying both is stating a deliberate override of its
/// own default, which is what [`ViewSubject`] is for.
///
///  who drives a seat is asked through `control::body_driving_seat`, not
/// answered here. Presentation is not the layer that decides what a second
/// holder of one slot means, and a private copy of that loop is how this system
/// came to take the first one silently while calling it an error in a comment.
pub fn resolve_view_subjects(
    mut views: bevy::prelude::Query<
        (
            &mut ResolvedViewSubject,
            Option<&ViewSubject>,
            Option<&ViewParticipant>,
        ),
        bevy::prelude::With<LocalView>,
    >,
    drivers: bevy::prelude::Query<(Entity, &ambition_characters::control::DrivingParticipant)>,
) {
    use bevy::prelude::DetectChangesMut as _;
    for (mut resolved, subject, participant) in &mut views {
        let next = subject.map(|subject| subject.0).or_else(|| {
            participant.and_then(|participant| {
                ambition_platformer2d_actor_monolith::control::body_driving_seat(
                    &drivers,
                    participant.0,
                )
            })
        });
        //  `set_if_neq`, because a write every frame is a CHANGE every
        // frame. A view whose subject has not moved must not look to a reader
        // keyed on `is_changed()` as though it had.
        resolved.set_if_neq(ResolvedViewSubject(next));
    }
}

/// The view, for a caller that knows there is exactly one.
///
///  the name is the disclaimer: this asserts the single-view assumption out
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

/// Composition-time camera-to-view binding.
///
/// A camera that explicitly presents a local view carries this component; the binding
/// is authored when the camera is spawned rather than inferred each frame.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentsView(pub Entity);

/// Local view that owns this view-dependent presentation projection.
///
/// Authoritative simulation/content entities remain singular; each view may own a
/// separate presentation entity with its own transform/layout. This component records
/// the semantic relationship only; renderer isolation is implemented elsewhere.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentedForView(pub Entity);

/// Resolve unkeyed camera/projection ownership against the current local views.
///
/// An explicit view always wins. With exactly one view, an unkeyed camera or projection
/// belongs to that view. With several views, unkeyed ownership is refused; with no
/// views, there is nothing to resolve.
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
        match self.resolve(link.map(|PresentsView(view)| view)) {
            Ok(view) => view,
            Err(Ambiguous) => {
                bevy::log::error_once!(
                    "several local views exist and a camera names none of them; \
                     refusing to guess which one it presents. Bind `PresentsView` \
                     where the camera is spawned."
                );
                None
            }
        }
    }

    /// The view a DRAWN world-space presentation entity belongs to — the same
    /// rule, asked at the other end of the seam.
    pub fn drawn_for(&self, key: Option<PresentedForView>) -> Option<Entity> {
        match self.resolve(key.map(|PresentedForView(view)| view)) {
            Ok(view) => view,
            Err(Ambiguous) => {
                bevy::log::error_once!(
                    "several local views exist and a world-space presentation entity \
                     names none of them; refusing to draw it for an arbitrary one. \
                     Each view needs its own copy, keyed by `PresentedForView`."
                );
                None
            }
        }
    }

    /// The rule itself, with the complaint left to the caller so the two
    /// `error_once!` sites stay distinct — one shared site would silence
    /// whichever kind of ambiguity happened to occur second.
    fn resolve(&self, named: Option<Entity>) -> Result<Option<Entity>, Ambiguous> {
        if let Some(view) = named {
            return Ok(Some(view));
        }
        // No views at all is quiet, and it is checked BEFORE ambiguity: a
        // composition with nothing to present has nothing to be ambiguous about.
        let Some(only) = self.first else {
            return Ok(None);
        };
        if self.several {
            return Err(Ambiguous);
        }
        Ok(Some(only))
    }
}

/// Several views exist and nothing named one, so the rule has no answer that is
/// not a guess.
struct Ambiguous;

/// Spawn one local view, with the components a camera resolve needs.
///
///  and it carries no `Name`, deliberately. A `Name` would be a nice debug
/// label and it is REGISTERED FOR ROLLBACK (`entity.name`) — and the rollback
/// coverage contract derives its swept population from *"an entity carrying even
/// one type the rollback knows about is an entity the rollback participates
/// in"*. So labelling this presentation entity enlisted the whole view in the
/// sim sweep, and its ease state was immediately reported as an unrewound desync
/// risk. `LocalViewId` is the identity; the label is not worth the enlistment.
///
///  call this at plugin BUILD time, not from a startup system. Every
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

/// ONE VIEW AND THE CAMERA THAT PRESENTS IT, as
/// [`compose_local_views`] hands them back.
///
/// The `id` is repeated here on purpose: the caller asked for views BY id, and
/// making it read the id back off the entity to know which row is which would
/// reintroduce exactly the "whichever one the query yielded" pairing this seam
/// exists to delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundLocalView {
    /// The identity the composition asked for.
    pub id: LocalViewId,
    /// The view entity — freshly spawned, or the one already carrying this id.
    pub view: Entity,
    /// The camera spawned for it, carrying [`PresentsView`] pointing at `view`.
    pub camera: Entity,
}

/// Ensure the requested local views exist and spawn one bound camera for each.
///
/// Existing views are adopted by id; missing views are spawned with the standard local
/// view facts. Ids are sorted/deduplicated for stable ordering. Camera bundles and
/// [`ViewPlacement`] remain composition policy supplied by the caller. Compose views at
/// plugin build time so startup readers see the complete view set.
pub fn compose_local_views<C, F>(
    world: &mut bevy::prelude::World,
    ids: impl IntoIterator<Item = LocalViewId>,
    mut camera_for: F,
) -> Vec<BoundLocalView>
where
    C: bevy::prelude::Bundle,
    F: FnMut(LocalViewId) -> C,
{
    //  ascending id, never query order. The returned order is what a
    // caller lays out left-to-right and what a log line names, so deriving it
    // from archetype iteration would make the LEFT pane a property of spawn
    // history. Deduped too: two rows for one id would spawn two cameras onto one
    // view, which is the "two cameras fight over one snapshot" case
    // `PresentsView` exists to make impossible.
    let mut wanted: Vec<LocalViewId> = ids.into_iter().collect();
    wanted.sort_unstable();
    let before = wanted.len();
    wanted.dedup();
    if wanted.len() != before {
        bevy::log::error_once!(
            "compose_local_views was asked for the same LocalViewId twice; each id \
             names ONE view, so the duplicates were dropped."
        );
    }

    let mut existing = existing_views_by_id(world);

    let mut bound = Vec::with_capacity(wanted.len());
    for id in wanted.iter().copied() {
        let view = match existing.binary_search_by_key(&id, |(existing_id, _)| *existing_id) {
            Ok(at) => existing[at].1,
            Err(_) => spawn_local_view(world, id, crate::camera_snapshot::local_view_facts()),
        };
        let camera = world.spawn((camera_for(id), PresentsView(view))).id();
        bound.push(BoundLocalView { id, view, camera });
    }

    // A view the composition did not ask for is still a view, and its presence
    // is what makes every unlinked camera in the app refuse. Saying so beats
    // despawning it — this helper does not own another plugin's view.
    existing.retain(|(existing_id, _)| wanted.binary_search(existing_id).is_err());
    if let Some((stray, _)) = existing.first() {
        bevy::log::error_once!(
            "compose_local_views left {} local view(s) it was not asked about (first: \
             {stray:?}); every camera that names no view now refuses to present one. \
             Ask for every view the composition has.",
            existing.len()
        );
    }

    bound
}

/// The live views as `(id, entity)`, sorted and therefore binary-searchable.
///
///  sorted by `(id, entity)` rather than by id alone so the result is a total
/// order even in the broken case where two views share an id — a tie broken by
/// archetype order would make this function's output depend on spawn history.
fn existing_views_by_id(world: &mut bevy::prelude::World) -> Vec<(LocalViewId, Entity)> {
    let mut views =
        world.query_filtered::<(Entity, &LocalViewId), bevy::prelude::With<LocalView>>();
    let mut rows: Vec<(LocalViewId, Entity)> = views
        .iter(world)
        .map(|(entity, id)| (*id, entity))
        .collect();
    rows.sort_unstable();
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_snapshot::{
        CameraObservationPlugin, CameraPresentationInputs, CameraReferenceFrame,
        CameraScreenFraming, CameraViewport, ResolvedCameraSnapshot,
    };
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_shared_tangle::camera_ease::CameraEaseState;
    use bevy::prelude::*;

    /// A LAYOUT THAT COVERS THE DISPLAY EXACTLY ONCE.
    ///
    ///  the columns must TILE, and both halves of that are checked: no gap (a strip of
    /// unpainted display between two panes) and no overlap (two observers drawing into the same
    /// pixels, which looks healthy in every single-view assertion).
    #[test]
    fn columns_tile_the_gameplay_rectangle_with_no_gap_and_no_overlap() {
        let origin = ae::Vec2::new(40.0, 10.0);
        let size = ae::Vec2::new(1920.0, 1080.0);
        let carved: Vec<(ae::Vec2, ae::Vec2)> = (0..3)
            .map(|column| ViewPlacement::column(column, 3).carve(origin, size))
            .collect();

        assert_eq!(carved[0].0, origin, "the first column starts at the origin");
        for pane in &carved {
            assert_eq!(pane.1.y, size.y, "a column is full height");
        }
        for pair in carved.windows(2) {
            assert_eq!(
                pair[0].0.x + pair[0].1.x,
                pair[1].0.x,
                "columns must meet exactly: {pair:?}",
            );
        }
        let last = carved.last().expect("three columns");
        assert_eq!(
            last.0.x + last.1.x,
            origin.x + size.x,
            "the last column must reach the right edge",
        );
    }

    /// A COUNT OF ONE IS NOT A LAYOUT, and neither is a count of zero.
    ///
    ///  the zero case is the one that matters: `index / 0.0` is NaN, and a NaN
    /// viewport does not draw a wrong picture — it draws nothing, silently, which
    /// is this repository's most expensive failure shape.
    #[test]
    fn a_degenerate_column_count_keeps_the_whole_rectangle() {
        let origin = ae::Vec2::ZERO;
        let size = ae::Vec2::new(800.0, 600.0);
        for of in [0, 1] {
            assert_eq!(
                ViewPlacement::column(0, of).carve(origin, size),
                (origin, size),
                "a layout of {of} view(s) must leave the rectangle alone",
            );
        }
        // And an index past the end lands on the LAST column rather than off the
        // screen: a caller that has miscounted gets a legible wrong picture.
        assert_eq!(
            ViewPlacement::column(9, 2).carve(origin, size),
            ViewPlacement::column(1, 2).carve(origin, size),
        );
    }

    /// A REVERSED OR EMPTY PLACEMENT STILL PRODUCES A DRAWABLE RECTANGLE.
    ///
    ///  a zero-width viewport is a division by zero in every orthographic-scale
    /// consumer downstream. One pixel is visibly wrong; zero is invisible.
    #[test]
    fn a_reversed_placement_is_normalized_and_never_collapses_to_nothing() {
        let size = ae::Vec2::new(400.0, 400.0);
        let reversed = ViewPlacement {
            min: ae::Vec2::new(0.75, 1.0),
            max: ae::Vec2::new(0.25, 0.0),
        };
        assert_eq!(
            reversed.carve(ae::Vec2::ZERO, size),
            ViewPlacement {
                min: ae::Vec2::new(0.25, 0.0),
                max: ae::Vec2::new(0.75, 1.0),
            }
            .carve(ae::Vec2::ZERO, size),
        );

        let empty = ViewPlacement {
            min: ae::Vec2::splat(0.5),
            max: ae::Vec2::splat(0.5),
        };
        assert_eq!(empty.carve(ae::Vec2::ZERO, size).1, ae::Vec2::ONE);
    }

    ///  THE VIEW EXISTS BEFORE ANY SCHEDULE RUNS, CARRYING EVERYTHING THE
    /// RESOLVE REQUIRES.
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
        // ⚠ PRESENCE, and only presence — which is the right assertion and is
        // now visibly narrower than it used to read. Since 2026-09-04 the
        // component carries `Option<ResolvedCameraFrame>`: it exists from spawn
        // so a reader never meets a view whose state is missing, and it says
        // `None` until the resolver frames it. "The place to publish exists" and
        // "a frame has been published" are different facts and this checks the
        // first.
        assert!(
            view.contains::<ResolvedCameraSnapshot>(),
            "nowhere to publish the resolved snapshot"
        );
        assert!(
            view.contains::<ResolvedViewSubject>(),
            "nowhere to publish who this view is watching — the camera resolve \
             requires it, so a view without one is silently excluded and reads \
             as a camera frozen at the origin"
        );
    }

    /// The component set an entity actually has, sorted — the exhaustive form of
    /// "the same facts", so a fact added to `local_view_facts` cannot reach one
    /// spawn path and miss the other without this failing.
    fn component_set(world: &World, entity: Entity) -> Vec<bevy::ecs::component::ComponentId> {
        let mut ids: Vec<_> = world.entity(entity).archetype().components().to_vec();
        ids.sort_unstable();
        ids
    }

    /// TWO VIEWS COME UP, EACH BOUND TO ITS OWN CAMERA.
    ///
    /// This is the claim the whole helper exists for, and every part of it can
    /// fail silently: two cameras both naming view 0 draws one view twice and
    /// looks like a working split until you read a nameplate; one camera left
    /// unlinked draws nothing and logs a refusal nobody watches; two views
    /// sharing a `LocalViewId` makes the render side's ordinal ambiguous.
    ///
    ///  and view 0 must be the PLUGIN'S view, not a third one. The plugin
    /// spawned `LocalViewId::FIRST` at build time; a helper that spawned its own
    /// id-0 view would leave three views in a two-view composition, and the
    /// extra one would silently make every unlinked camera in the app refuse.
    #[test]
    fn two_views_come_up_each_bound_to_its_own_camera() {
        let mut app = App::new();
        app.add_plugins(CameraObservationPlugin);
        let plugins_view = the_only_view(app.world_mut());

        let bound = compose_local_views(
            app.world_mut(),
            [LocalViewId(1), LocalViewId::FIRST],
            |_id| (ambition_platformer2d_shared_tangle::camera_layers::MainCamera,),
        );

        // Asked for out of order on purpose: the result is ascending by id, not
        // in call order and not in archetype order.
        assert_eq!(
            bound.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![LocalViewId::FIRST, LocalViewId(1)],
            "the composition's views must come back in ascending LocalViewId order"
        );
        assert_eq!(
            bound[0].view, plugins_view,
            "id 0 must ADOPT the view the plugin already spawned, not duplicate it"
        );
        assert_ne!(
            bound[0].view, bound[1].view,
            "two views that are one entity are one view"
        );
        assert_ne!(
            bound[0].camera, bound[1].camera,
            "each view needs its OWN camera; one camera cannot hold two transforms"
        );

        let views: Vec<Entity> = {
            let mut q = app.world_mut().query_filtered::<Entity, With<LocalView>>();
            q.iter(app.world()).collect()
        };
        assert_eq!(
            views.len(),
            2,
            "a two-view composition must end with exactly two views"
        );

        for row in &bound {
            assert_eq!(
                app.world().entity(row.camera).get::<PresentsView>(),
                Some(&PresentsView(row.view)),
                "camera for {:?} does not name its own view",
                row.id
            );
            assert_eq!(
                *app.world().entity(row.view).get::<LocalViewId>().unwrap(),
                row.id,
                "the view carries an id the caller did not ask for"
            );
        }

        //  the "N times what the single-view path produces" claim, asked
        // exhaustively rather than by re-listing the components by hand.
        assert_eq!(
            component_set(app.world(), bound[0].view),
            component_set(app.world(), bound[1].view),
            "the second view does not carry the same facts as the engine's own \
             single view — a fact present on one and missing on the other is not a \
             compile error, it is a view that silently drops out of the resolve"
        );

        // And the refusal rule is untouched: with two views, a camera naming
        // none still gets no answer.
        let on_hand = ViewsOnHand::survey(views.iter().copied());
        assert_eq!(
            on_hand.presented_by(None),
            None,
            "with several views an unlinked camera must still be refused"
        );
    }

    /// A COMPOSITION THAT ASKS FOR ONE VIEW GETS TODAY'S RESULT.
    ///
    /// The refusal rule is deliberately unchanged, so the thing that must not
    /// move is what an UNLINKED camera resolves to — that is what both shipped
    /// camera-spawn sites do, and it is the only observable difference a
    /// single-view host could suffer from this helper existing.
    #[test]
    fn asking_for_one_view_leaves_the_single_view_composition_alone() {
        let mut app = App::new();
        app.add_plugins(CameraObservationPlugin);
        let plugins_view = the_only_view(app.world_mut());
        let before = component_set(app.world(), plugins_view);

        let bound = compose_local_views(app.world_mut(), [LocalViewId::FIRST], |_id| {
            (ambition_platformer2d_shared_tangle::camera_layers::MainCamera,)
        });

        assert_eq!(bound.len(), 1);
        assert_eq!(
            bound[0].view, plugins_view,
            "asking for one view must adopt the only view, not add a second"
        );
        assert_eq!(
            the_only_view(app.world_mut()),
            plugins_view,
            "the composition still has exactly one view"
        );
        assert_eq!(
            component_set(app.world(), plugins_view),
            before,
            "adopting a view must not add to or remove from it"
        );

        // The single-view fallback — what `spawn_main_camera` and the app's
        // `scene_setup` rely on to bind at all.
        let on_hand = ViewsOnHand::survey([plugins_view]);
        assert_eq!(
            on_hand.presented_by(None),
            Some(plugins_view),
            "a camera naming no view must still take the only view"
        );
    }

    /// The frame policy is per-view state that a game can select.
    ///
    /// This is that policy having a home: writing the component is the selection.
    ///
    ///  the Gameplay-menu option now drives it too (`camera_reference_frame`
    /// → [`crate::camera_snapshot::CameraObservationPlugin`]), and this test is
    /// still the one that matters: the setting writes this COMPONENT, so the
    /// direct-write path stays the contract and nothing here has to be removed
    /// when views become indexed. See
    /// `the_gameplay_setting_selects_the_views_reference_frame` for the wiring.
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

    /// The player's setting reaches the view — through the real plugin, not a
    /// hand-run system.
    ///
    ///  the component stays the selection (the test above pins that a game can
    /// still write it directly); this pins that the shipped Gameplay-menu option
    /// is wired to it, which is the half a "capability with zero adopters" would
    /// silently fail. Absent the settings resource nothing is forced, so the
    /// direct-write path keeps working.
    #[test]
    fn the_gameplay_setting_selects_the_views_reference_frame() {
        let mut app = App::new();
        app.add_plugins(CameraObservationPlugin);
        let view = the_only_view(app.world_mut());

        let mut settings = ambition_persistence::settings::UserSettings::default();
        settings.gameplay.camera_reference_frame = CameraReferenceFrame::SubjectFrame;
        app.insert_resource(settings);
        app.update();

        assert_eq!(
            *app.world()
                .entity(view)
                .get::<CameraReferenceFrame>()
                .unwrap(),
            CameraReferenceFrame::SubjectFrame,
            "choosing player-relative in the Gameplay menu did not reach the view"
        );

        app.world_mut()
            .resource_mut::<ambition_persistence::settings::UserSettings>()
            .gameplay
            .camera_reference_frame = CameraReferenceFrame::WorldFixed;
        app.update();
        assert_eq!(
            *app.world()
                .entity(view)
                .get::<CameraReferenceFrame>()
                .unwrap(),
            CameraReferenceFrame::WorldFixed,
            "switching back must return the view to world-fixed"
        );
    }

    ///  the write is conditional, and that is load-bearing. `is_changed()`
    /// ticks do not rewind, so a system that wrote the component every frame
    /// would report a fresh change on every replayed rollback tick.
    #[test]
    fn a_settled_reference_frame_is_not_rewritten_every_frame() {
        let mut app = App::new();
        app.add_plugins(CameraObservationPlugin);
        let view = the_only_view(app.world_mut());
        app.insert_resource(ambition_persistence::settings::UserSettings::default());

        app.update();
        let settled = app
            .world()
            .entity(view)
            .get_ref::<CameraReferenceFrame>()
            .unwrap()
            .last_changed();
        app.update();
        let after = app
            .world()
            .entity(view)
            .get_ref::<CameraReferenceFrame>()
            .unwrap()
            .last_changed();

        assert_eq!(
            settled, after,
            "the reference frame was rewritten on a frame where the setting did not \
             change, so every rollback resimulation would see it as freshly changed"
        );
    }
}
