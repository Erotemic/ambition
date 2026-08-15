//! **TWO VIEWS, ONE ROOM, ONE SIMULATION — AND TWO PICTURES.**
//!
//! `PresentedForView` already says which view a drawn presentation entity was
//! built for, and `PresentsView` says which view a camera shows. Both ends of
//! that seam were correct before this module existed and the pictures still were
//! not: every view's nameplates and label copies drew into EVERY camera, so a
//! second view produced two frames each containing both views' text, each label
//! ranked and faded for the wrong observer. Per-view TRANSFORMS without per-view
//! VISIBILITY is half a projection.
//!
//! # ⭐ The relationship is the identity; this is only the mechanism
//!
//! Nothing here decides which view anything belongs to — it reads the answer off
//! [`ambition_sim_view::PresentedForView`] and
//! [`ambition_sim_view::PresentsView`], through the same
//! [`ambition_sim_view::ViewsOnHand`] rule the follow camera and the viewport
//! applier use. What this module owns is the render-side ANSWER to "and how is
//! that kept true on screen", which is a `RenderLayers` band and could be a
//! camera-per-layer scheme, a render-target split or an extraction filter
//! tomorrow without a semantic type changing.
//!
//! ⛔ **so `LocalViewId` is still not a `RenderLayers` bit.** The band index is
//! the view's POSITION among the live views sorted by id, computed here, every
//! frame. A game may name its views `LocalViewId(0)` and `LocalViewId(9)` and
//! they occupy layers `BASE + 0` and `BASE + 1`; the ordinal never leaves this
//! system, and no semantic value is a mask.
//!
//! # Why `RenderLayers`, and where it comes from
//!
//! It is what this tree already isolates cameras with. The front HUD camera is
//! pinned to `FRONT_HUD_LAYER` so it cannot re-draw the world over the cube; the
//! portal view-cone renderer gives each capture rig a PRIVATE parallax layer and
//! writes `RenderLayers::none().with(private_layer)` onto the copies so they
//! reach that rig's camera and no other — the identical shape to the one below,
//! one seam over. Adopting it means a per-view projection is invisible to a
//! camera exactly when the renderer's own rule says so
//! (`check_visibility`: `view_mask.intersects(entity_mask)`), rather than when
//! some second visibility concept agrees with it.
//!
//! # ⭐ A ONE-VIEW COMPOSITION PAYS NOTHING
//!
//! With fewer than two views there is nothing to isolate FROM, so this system
//! writes nothing: no component is inserted on a projection, and no camera's
//! authored layers are touched. Every shipped composition today is single-view,
//! and it draws precisely the entities it drew before, on precisely the layers it
//! drew them on. The mechanism appears when a second view does and disappears
//! when it goes.
//!
//! ⚠ **and it disappears by RESETTING, never by stripping.** When a composition
//! collapses back to one view, an entity that was isolated keeps its
//! `RenderLayers` and has it set back to the default (layer 0). Removing the
//! component would produce the same picture today and is the shape that has
//! bitten this repository repeatedly — an absent component reads as "no value" to
//! every query, and the test asserting the absence agrees with the bug.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_layers::{
    local_view_render_layer, MainCamera, LOCAL_VIEW_RENDER_LAYER_BASE,
};

/// **Give each camera only its own view's projections.**
///
/// ⚠ **`With<MainCamera>`, not `With<Camera2d>`, for the reason `camera_follow`
/// gives**: the portal view-cone renderer spawns offscreen capture `Camera2d`s
/// and the cube menu spawns a `Camera3d`. A capture rig is not an observer of the
/// simulation, it is a lens inside one, and dragging it into the per-view scheme
/// would hand it a view it does not present.
///
/// ⛔ **this system is the SINGLE WRITER of `RenderLayers` on anything keyed by
/// `PresentedForView`** (and on that entity's descendants — a nameplate's outline
/// copies are children with their own `Text2d`, and `RenderLayers` does not
/// inherit down a hierarchy in Bevy, so an unvisited child would keep drawing
/// into both cameras while its parent moved). A projection that wants a layer of
/// its own would be fighting this pass every frame, which is why the per-view key
/// and a hand-set layer must never appear on the same entity.
pub fn isolate_per_view_projections(
    mut commands: Commands,
    views: Query<(Entity, &ambition_sim_view::LocalViewId), With<ambition_sim_view::LocalView>>,
    cameras: Query<(Entity, Option<&ambition_sim_view::PresentsView>), With<MainCamera>>,
    projections: Query<(Entity, &ambition_sim_view::PresentedForView)>,
    children: Query<&Children>,
    // ⚠ ONE mutable handle on `RenderLayers` for cameras, projections and their
    // children alike. Two mutable queries split by `With`/`Without` would express
    // the same thing and would make this system's write access a pair of claims
    // that have to stay disjoint as the population widens.
    mut layers: Query<&mut RenderLayers>,
) {
    // Sorted by the view's own stable ordinal, so a view's layer is the same
    // answer on every frame and every run. Archetype iteration order is neither,
    // and this population is small enough that the sort is free.
    let mut ordered: Vec<(ambition_sim_view::LocalViewId, Entity)> =
        views.iter().map(|(view, id)| (*id, view)).collect();
    ordered.sort();

    // The whole switch: one observer has nobody to be isolated from.
    let isolating = ordered.len() > 1;

    let on_hand = ambition_sim_view::ViewsOnHand::survey(ordered.iter().map(|(_, view)| *view));

    for (camera, link) in &cameras {
        // The camera's own link, through the shared binding rule — a camera that
        // names none of several views is refused there, loudly, and lands here as
        // `None`. It keeps the world and gets no view's text, which is the honest
        // picture for a camera nobody said what to present.
        let wanted = on_hand
            .presented_by(link.copied())
            .and_then(|view| view_layer(&ordered, view, isolating));
        match layers.get_mut(camera) {
            Ok(mut current) => {
                // ⛔ the authored layers are KEPT and only the view band is
                // rewritten: a host composes its main camera's layers itself
                // (world + parallax, plus the portal window layer when that
                // feature is on) and this pass owns exactly one band of them.
                let base = without_view_layers(&current);
                let desired = match wanted {
                    Some(layer) => base.with(layer),
                    None => base,
                };
                if *current != desired {
                    *current = desired;
                }
            }
            Err(_) => {
                // A camera with no authored layers renders layer 0, so the
                // default is its base. Nothing is inserted while single-view.
                if let Some(layer) = wanted {
                    commands
                        .entity(camera)
                        .insert(RenderLayers::default().with(layer));
                }
            }
        }
    }

    for (root, key) in &projections {
        let desired = match view_layer(&ordered, key.0, isolating) {
            Some(layer) => RenderLayers::none().with(layer),
            // ⚠ isolating, but the view this copy names is GONE. No camera may
            // draw it: it belongs to nobody, and the empty mask says exactly that
            // in the renderer's own vocabulary. Despawning it is the owning
            // system's job (`mirror_static_world_labels_per_view` retracts a
            // retired view's whole set); until it runs, drawing a dead view's
            // label into an arbitrary camera is the guess this seam exists to
            // refuse.
            None if isolating => RenderLayers::none(),
            None => RenderLayers::default(),
        };

        // The projection AND its descendants — see the system doc for why the
        // children are not optional.
        let mut pending: Vec<Entity> = vec![root];
        while let Some(entity) = pending.pop() {
            match layers.get_mut(entity) {
                Ok(mut current) => {
                    if *current != desired {
                        *current = desired.clone();
                    }
                }
                Err(_) => {
                    if isolating {
                        commands.entity(entity).insert(desired.clone());
                    }
                }
            }
            if let Ok(kids) = children.get(entity) {
                pending.extend(kids.iter());
            }
        }
    }
}

/// The layer a view's projections draw on, or `None` when nothing is being
/// isolated or the named view is not live.
fn view_layer(
    ordered: &[(ambition_sim_view::LocalViewId, Entity)],
    view: Entity,
    isolating: bool,
) -> Option<usize> {
    if !isolating {
        return None;
    }
    ordered
        .iter()
        .position(|(_, candidate)| *candidate == view)
        .map(local_view_render_layer)
}

/// A camera's authored layers with the per-view band cleared — what it would
/// render if no view had claimed it.
///
/// ⭐ **derived, not remembered.** Stashing the base at spawn would be a second
/// copy of a value the entity already holds, and it would go stale the moment a
/// host added a layer afterwards (which `PlatformerPresentationPlugin`'s doc
/// invites it to do).
fn without_view_layers(layers: &RenderLayers) -> RenderLayers {
    let mut base = layers.clone();
    for layer in layers.iter() {
        if layer >= LOCAL_VIEW_RENDER_LAYER_BASE {
            base = base.without(layer);
        }
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER;
    use ambition_sim_view::{LocalView, LocalViewId, PresentedForView, PresentsView};
    use bevy::ecs::system::RunSystemOnce as _;

    /// What a host composes onto its main camera before any of this runs.
    fn authored_camera_layers() -> RenderLayers {
        RenderLayers::layer(0).with(PARALLAX_BACKGROUND_LAYER)
    }

    fn mask(world: &World, entity: Entity) -> RenderLayers {
        world
            .entity(entity)
            .get::<RenderLayers>()
            .cloned()
            .unwrap_or_default()
    }

    /// **The renderer's own rule, not a proxy for it.** `check_visibility` reads
    /// each side's mask, defaults a missing one to layer 0, and draws the entity
    /// in that view when the two intersect. Asking the same question here is what
    /// makes these assertions about the picture rather than about a bookkeeping
    /// component this test invented.
    fn camera_draws(world: &World, camera: Entity, entity: Entity) -> bool {
        mask(world, camera).intersects(&mask(world, entity))
    }

    struct TwoViews {
        world: World,
        /// In spawn order. `cameras[i]` presents `presented[i]`.
        cameras: [Entity; 2],
        presented: [Entity; 2],
        /// Plate and outline child, keyed to `presented[i]`.
        plates: [Entity; 2],
        outlines: [Entity; 2],
        /// A room sprite belonging to no view — the shared world both observers
        /// are looking at.
        scenery: Entity,
    }

    /// One simulation, two views, two cameras, one per-view projection each.
    ///
    /// `first_presents_lower` is the ONLY thing that differs between runs: it
    /// swaps which view each camera names while leaving spawn order, entity ids
    /// and every other value untouched.
    fn two_views(first_presents_lower: bool) -> TwoViews {
        let mut world = World::new();
        let lower = world.spawn((LocalView, LocalViewId(0))).id();
        let upper = world.spawn((LocalView, LocalViewId(1))).id();

        let presented = if first_presents_lower {
            [lower, upper]
        } else {
            [upper, lower]
        };
        let cameras = presented.map(|view| {
            world
                .spawn((MainCamera, authored_camera_layers(), PresentsView(view)))
                .id()
        });

        let mut outlines = [Entity::PLACEHOLDER; 2];
        let plates = [0usize, 1].map(|slot| {
            let plate = world.spawn(PresentedForView(presented[slot])).id();
            // A nameplate's outline copies are CHILDREN carrying their own text.
            let outline = world.spawn(ChildOf(plate)).id();
            outlines[slot] = outline;
            plate
        });
        let scenery = world.spawn_empty().id();

        TwoViews {
            world,
            cameras,
            presented,
            plates,
            outlines,
            scenery,
        }
    }

    /// **⛔⛔ EACH CAMERA DRAWS ITS OWN VIEW'S PROJECTIONS AND NOT THE OTHER'S.**
    ///
    /// This is the acceptance D116 M2 still owed. Both views' transforms were
    /// already per-view correct and every drawn copy still reached every camera,
    /// so a two-view session produced two frames each carrying both views' text —
    /// each label placed for an observer that was not the one looking at it.
    ///
    /// ⭐ **the shared world is asserted too, in the same breath.** "Isolate the
    /// views" is trivially satisfiable by showing each camera nothing; what makes
    /// the pictures right is that both cameras still draw the room. A mechanism
    /// that isolated the WORLD as well would pass every negative assertion here.
    ///
    /// ⚠ **the falsifier is inside the test.** The second run swaps only the two
    /// `PresentsView` links — same spawn order, same entities, same components —
    /// and the two cameras must swap with them. An implementation that keys off
    /// camera iteration order, or off `LocalViewId` as a bit, passes the first run
    /// and fails this one.
    #[test]
    fn each_camera_draws_only_the_projections_of_the_view_it_names() {
        for first_presents_lower in [true, false] {
            let mut fixture = two_views(first_presents_lower);
            fixture
                .world
                .run_system_once(isolate_per_view_projections)
                .expect("the isolation pass reads only components the fixture spawns");
            let world = &fixture.world;

            for own in [0usize, 1] {
                let other = 1 - own;
                let camera = fixture.cameras[own];
                assert!(
                    camera_draws(world, camera, fixture.plates[own]),
                    "camera {camera:?} presents view {:?} and must draw that view's \
                     own plate; presenting a view whose projections it cannot see \
                     is an empty picture, not an isolated one",
                    fixture.presented[own]
                );
                assert!(
                    camera_draws(world, camera, fixture.outlines[own]),
                    "a plate's outline children must follow the plate: `RenderLayers` \
                     does not inherit, so leaving them behind draws every outline in \
                     both cameras while the text moves"
                );
                assert!(
                    !camera_draws(world, camera, fixture.plates[other]),
                    "camera {camera:?} presents view {:?} and must NOT draw view \
                     {:?}'s plate — that copy was placed and faded for a different \
                     observer",
                    fixture.presented[own],
                    fixture.presented[other]
                );
                assert!(
                    !camera_draws(world, camera, fixture.outlines[other]),
                    "the other view's outline copies leak exactly like its text does"
                );
                assert!(
                    camera_draws(world, camera, fixture.scenery),
                    "both observers are looking at ONE room: isolating the per-view \
                     projections must not take the world away from either camera"
                );
            }
        }
    }

    /// **⭐ A ONE-VIEW COMPOSITION IS LEFT EXACTLY AS IT WAS.**
    ///
    /// Every composition that ships today is single-view, so the mechanism must
    /// cost them nothing: no component appears on a projection that did not have
    /// one, and the camera's authored layers come out byte-identical. A pass that
    /// moved single-view labels onto a private layer would still look correct in
    /// the main camera and would silently drop them out of every portal capture.
    #[test]
    fn a_single_view_composition_is_untouched() {
        let mut world = World::new();
        let view = world.spawn((LocalView, LocalViewId(0))).id();
        let camera = world
            .spawn((MainCamera, authored_camera_layers(), PresentsView(view)))
            .id();
        let plate = world.spawn(PresentedForView(view)).id();
        let outline = world.spawn(ChildOf(plate)).id();

        world
            .run_system_once(isolate_per_view_projections)
            .expect("the isolation pass reads only components the fixture spawns");

        assert!(
            !world.entity(plate).contains::<RenderLayers>(),
            "a single-view projection must not acquire a visibility mask it has \
             nothing to be hidden from"
        );
        assert!(
            !world.entity(outline).contains::<RenderLayers>(),
            "nor may its children"
        );
        assert_eq!(
            mask(&world, camera),
            authored_camera_layers(),
            "the host composed these layers; a single view gives the pass nothing \
             to add and nothing to take away"
        );
    }

    /// **⚠ A RETIRED VIEW LEAVES THE SURVIVOR RESET, NOT STRIPPED.**
    ///
    /// The projection that outlives the second view keeps its `RenderLayers` and
    /// has it set back to the default. Expressing "no longer isolated" by REMOVING
    /// the component would draw the same picture and is the shape this repository
    /// has been bitten by: absence reads as "no value" to every query that
    /// requires the component, and the assertion that it is gone agrees with the
    /// bug.
    #[test]
    fn collapsing_to_one_view_resets_the_layer_rather_than_removing_it() {
        let mut fixture = two_views(true);
        fixture
            .world
            .run_system_once(isolate_per_view_projections)
            .expect("the isolation pass reads only components the fixture spawns");
        assert!(
            fixture
                .world
                .entity(fixture.plates[0])
                .contains::<RenderLayers>(),
            "the two-view phase must actually have isolated something, or the \
             collapse below proves nothing"
        );

        // The second view goes away with its whole projection set, exactly as a
        // retirement despawns it.
        fixture.world.entity_mut(fixture.presented[1]).despawn();
        fixture.world.entity_mut(fixture.plates[1]).despawn();
        fixture
            .world
            .run_system_once(isolate_per_view_projections)
            .expect("the isolation pass reads only components the fixture spawns");

        let survivor = fixture.plates[0];
        assert!(
            fixture.world.entity(survivor).contains::<RenderLayers>(),
            "the surviving projection must be RESET, never stripped"
        );
        assert_eq!(
            mask(&fixture.world, survivor),
            RenderLayers::default(),
            "one view again means the world layer again"
        );
        assert_eq!(
            mask(&fixture.world, fixture.cameras[0]),
            authored_camera_layers(),
            "and the camera is back to exactly the layers its host composed"
        );
        assert!(
            camera_draws(&fixture.world, fixture.cameras[0], survivor),
            "the survivor is drawn by the remaining camera"
        );
    }
}
