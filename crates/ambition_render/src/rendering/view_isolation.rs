//! Render-layer isolation for per-view presentation projections.
//!
//! [`PresentedForView`](ambition_sim_view::PresentedForView) records projection
//! ownership and [`PresentsView`](ambition_sim_view::PresentsView) records camera
//! ownership. With multiple views this module maps their stable sorted order onto a
//! private render-layer band. Single-view compositions are left unchanged. When views
//! collapse back to one, projections return to their declared resting layers.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_layers::{
    live_room_render_layer, local_view_render_layer, MainCamera, LIVE_ROOM_RENDER_LAYER_BASE,
    LIVE_ROOM_RENDER_LAYER_LAST, LOCAL_VIEW_RENDER_LAYER_BASE,
};

/// Render layers to restore when a per-view projection is no longer isolated.
///
/// Most projections need no component and return to the default world layer. Families
/// with a non-default base layer must record it here because isolation temporarily
/// replaces the projection's full mask.
#[derive(Component, Clone, Debug)]
pub struct ProjectionRestingLayers(pub RenderLayers);

/// Give each camera only its own view's projections.
///
/// `With<MainCamera>`, not `With<Camera2d>`, as in `camera_follow`: the portal
/// view-cone renderer spawns offscreen capture `Camera2d`s and the cube menu
/// spawns a `Camera3d`. A capture rig is a lens inside the simulation, not an
/// observer, so it presents no view.
///
/// This system is the only writer of `RenderLayers` on anything keyed by
/// `PresentedForView` and on its descendants. `RenderLayers` does not inherit
/// down a hierarchy in Bevy, so an unvisited child (for example a nameplate's
/// outline copy) would keep drawing into both cameras.
///
/// So a projection does not set its own layer; it declares the layer it rests
/// on ([`ProjectionRestingLayers`]), and this pass restores that. The room's
/// parallax panels need this.
pub fn isolate_per_view_projections(
    mut commands: Commands,
    views: Query<(Entity, &ambition_sim_view::LocalViewId), With<ambition_sim_view::LocalView>>,
    cameras: Query<(Entity, Option<&ambition_sim_view::PresentsView>), With<MainCamera>>,
    projections: Query<(Entity, &ambition_sim_view::PresentedForView)>,
    children: Query<&Children>,
    // Read on every entity of a projection's subtree: a root can rest on a
    // private layer while its children rest on the world layer.
    resting: Query<&ProjectionRestingLayers>,
    // One mutable query on `RenderLayers` for cameras, projections, and
    // children. Two queries split by `With`/`Without` would need to stay
    // disjoint as the population grows.
    mut layers: Query<&mut RenderLayers>,
) {
    // Sorted by the view's stable ordinal, so a view's layer is the same on
    // every frame and run. Archetype order is not, and the sort is cheap.
    let mut ordered: Vec<(ambition_sim_view::LocalViewId, Entity)> =
        views.iter().map(|(view, id)| (*id, view)).collect();
    ordered.sort();

    // With one observer there is nothing to isolate.
    let isolating = ordered.len() > 1;

    let on_hand = ambition_sim_view::ViewsOnHand::survey(ordered.iter().map(|(_, view)| *view));

    for (camera, link) in &cameras {
        // The camera's own link, through the shared binding rule. A camera that
        // names none of several views is refused there and arrives as `None`: it
        // keeps the world and gets no view's text.
        let wanted = on_hand
            .presented_by(link.copied())
            .and_then(|view| view_layer(&ordered, view, isolating));
        match layers.get_mut(camera) {
            Ok(mut current) => {
                // Keep the authored layers and rewrite only the view band. A host
                // composes its camera's layers (world, parallax, and the portal
                // window layer when enabled); this pass owns only one band.
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
        let band = view_layer(&ordered, key.0, isolating);

        // The projection and its descendants; see the system doc.
        let mut pending: Vec<Entity> = vec![root];
        while let Some(entity) = pending.pop() {
            let desired = match band {
                Some(layer) => RenderLayers::none().with(layer),
                // Isolating, but the named view is gone. No camera may draw
                // it, so the mask is empty.
                None if isolating => RenderLayers::none(),
                // Not isolating: back to this entity's resting layers. Usually
                // the world layer; a family that rests elsewhere declares it
                // with `ProjectionRestingLayers`.
                None => resting
                    .get(entity)
                    .map(|resting| resting.0.clone())
                    .unwrap_or_default(),
            };
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

/// Give each camera only the live room its view frames (view half, cut V3).
///
/// Live rooms use one coordinate space, so a camera that draws the world layer
/// draws two live rooms on top of each other. While two or more rooms are
/// live, an entity stamped into a live room (`InRoomInstance`), and its
/// descendants, draw on that room's band in place of the world layer, and each
/// main camera adds the band of the room its view frames
/// (`ResolvedCameraFrame::room`). With one live room nothing is banded, and a
/// banded entity returns to the world layer.
///
/// Only the world layer moves. An unstamped entity stays on it, and every
/// camera draws it. An entity that does not draw on the world layer (a
/// parallax panel) keeps its layers. A view's own projections belong to
/// [`isolate_per_view_projections`], so this pass does not enter a
/// `PresentedForView` subtree, and the two passes write different bands of a
/// camera's mask.
#[allow(clippy::type_complexity)]
pub fn isolate_live_rooms(
    mut commands: Commands,
    rooms: Query<
        &ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    views: Query<(Entity, &ambition_sim_view::camera_snapshot::ResolvedCameraSnapshot), With<ambition_sim_view::LocalView>>,
    cameras: Query<(Entity, Option<&ambition_sim_view::PresentsView>), With<MainCamera>>,
    stamped: Query<
        (Entity, &ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance),
        Without<ambition_sim_view::PresentedForView>,
    >,
    per_view: Query<(), With<ambition_sim_view::PresentedForView>>,
    children: Query<&Children>,
    mut layers: Query<&mut RenderLayers>,
) {
    // In instance order, so a room keeps its band while the rooms around it
    // stay live.
    let mut ordered: Vec<_> = rooms.iter().copied().collect();
    ordered.sort();
    let isolating = ordered.len() > 1;
    let band = |room| {
        isolating
            .then(|| ordered.iter().position(|live| *live == room))
            .flatten()
            .map(live_room_render_layer)
    };

    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter().map(|(view, _)| view));
    for (camera, link) in &cameras {
        let wanted = on_hand
            .presented_by(link.copied())
            .and_then(|view| views.get(view).ok())
            .and_then(|(_, resolved)| resolved.frame())
            .and_then(|frame| band(frame.room));
        match layers.get_mut(camera) {
            Ok(mut current) => {
                let base = without_room_layers(&current);
                let desired = match wanted {
                    Some(layer) => base.with(layer),
                    None => base,
                };
                if *current != desired {
                    *current = desired;
                }
            }
            Err(_) => {
                if let Some(layer) = wanted {
                    commands.entity(camera).insert(RenderLayers::default().with(layer));
                }
            }
        }
    }

    for (root, stamp) in &stamped {
        let wanted = band(stamp.0);
        let mut pending: Vec<Entity> = vec![root];
        while let Some(entity) = pending.pop() {
            if entity != root && per_view.contains(entity) {
                continue;
            }
            match layers.get_mut(entity) {
                Ok(mut current) => {
                    let desired = in_room_band(&current, wanted);
                    if *current != desired {
                        *current = desired;
                    }
                }
                Err(_) => {
                    // No mask is the world layer.
                    if let Some(layer) = wanted {
                        commands.entity(entity).try_insert(RenderLayers::none().with(layer));
                    }
                }
            }
            if let Ok(kids) = children.get(entity) {
                pending.extend(kids.iter());
            }
        }
    }
}

/// A mask with the live-room band cleared: what a camera draws when its view
/// frames no banded room.
fn without_room_layers(layers: &RenderLayers) -> RenderLayers {
    let mut base = layers.clone();
    for layer in layers.iter() {
        if (LIVE_ROOM_RENDER_LAYER_BASE..=LIVE_ROOM_RENDER_LAYER_LAST).contains(&layer) {
            base = base.without(layer);
        }
    }
    base
}

/// An entity's mask with its world layer on room band `wanted`, or back on
/// the world layer when `wanted` is `None`.
fn in_room_band(layers: &RenderLayers, wanted: Option<usize>) -> RenderLayers {
    let banded = layers.iter().any(|layer| (LIVE_ROOM_RENDER_LAYER_BASE..=LIVE_ROOM_RENDER_LAYER_LAST).contains(&layer));
    let world = if banded { without_room_layers(layers).with(0) } else { layers.clone() };
    match wanted {
        Some(layer) if world.intersects(&RenderLayers::layer(0)) => world.without(0).with(layer),
        _ => world,
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

/// A camera's authored layers with the per-view band cleared: what it renders
/// if no view claims it.
///
/// Derived, not stored. A base saved at spawn would go stale if a host added
/// a layer later (which `PlatformerPresentationPlugin`'s doc allows).
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

    /// The renderer's own rule. `check_visibility` reads each side's mask,
    /// defaults a missing one to layer 0, and draws the entity in that view when
    /// the masks intersect. So these assertions are about the picture.
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
        /// A room sprite that belongs to no view: the shared world both observers
        /// see.
        scenery: Entity,
    }

    /// One simulation, two views, two cameras, one per-view projection each.
    ///
    /// `first_presents_lower` is the only difference between runs: it swaps
    /// which view each camera names, and keeps spawn order, entity ids, and
    /// every other value.
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
            // A nameplate's outline copies are children with their own text.
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

    /// Each camera draws its own view's projections and not the other's.
    ///
    /// The shared world is checked too. Showing each camera nothing would also
    /// isolate the views; both cameras must still draw the room.
    ///
    /// The second run swaps only the two `PresentsView` links, and the cameras
    /// must swap too. An implementation keyed on camera iteration order, or on
    /// `LocalViewId` as a bit, fails that run.
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

    /// A one-view composition is left exactly as it was.
    ///
    /// Every shipped composition is single-view, so the mechanism must cost
    /// nothing there: no new component on a projection, and the camera's layers
    /// unchanged. Moving single-view labels to a private layer would look right
    /// in the main camera but drop them from every portal capture.
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

    /// A projection that rests on a private layer returns to it, not to layer 0.
    ///
    /// The room's parallax panels are on `PARALLAX_BACKGROUND_LAYER` so the portal
    /// capture cameras do not draw them (a capture rig's eye would sample the
    /// wrong background). They are per-view projections because a panel's
    /// transform and size depend on its camera.
    ///
    /// The collapse cannot be derived. While isolating, the mask is
    /// `none().with(band)` with nothing of the spawner's choice left, so a derived
    /// resting layer would send the backdrop to layer 0 and every portal capture
    /// would draw it.
    #[test]
    fn a_projection_with_private_resting_layers_returns_to_them_when_the_split_collapses() {
        let mut world = World::new();
        let lower = world.spawn((LocalView, LocalViewId(0))).id();
        let upper = world.spawn((LocalView, LocalViewId(1))).id();
        for view in [lower, upper] {
            world.spawn((MainCamera, authored_camera_layers(), PresentsView(view)));
        }
        // A backdrop panel per view, each declaring its resting layer.
        let panels = [lower, upper].map(|view| {
            world
                .spawn((
                    PresentedForView(view),
                    RenderLayers::layer(PARALLAX_BACKGROUND_LAYER),
                    ProjectionRestingLayers(RenderLayers::layer(PARALLAX_BACKGROUND_LAYER)),
                ))
                .id()
        });
        // An ordinary plate, which rests on the world layer and must not be
        // affected.
        let plate = world.spawn(PresentedForView(lower)).id();

        world
            .run_system_once(isolate_per_view_projections)
            .expect("the isolation pass reads only components the fixture spawns");

        // Non-vacuity: the split must really move the panels, or the collapse
        // below proves nothing.
        assert_ne!(
            mask(&world, panels[0]),
            mask(&world, panels[1]),
            "two views must put their backdrops on two different bands, or the \
             two cameras draw both panels and there is no split screen"
        );
        assert!(
            !mask(&world, panels[0]).intersects(&RenderLayers::layer(PARALLAX_BACKGROUND_LAYER)),
            "while isolating, a panel may NOT stay on the shared parallax layer: \
             every main camera renders it, so both views would draw both panels"
        );

        // The second view retires with its whole set, as the mirror despawns
        // it.
        world.entity_mut(upper).despawn();
        world.entity_mut(panels[1]).despawn();
        world
            .run_system_once(isolate_per_view_projections)
            .expect("the isolation pass reads only components the fixture spawns");

        assert_eq!(
            mask(&world, panels[0]),
            RenderLayers::layer(PARALLAX_BACKGROUND_LAYER),
            "one view again means the panel's OWN resting layer again — layer 0 \
             here would hand the backdrop to every portal capture camera, which \
             is the eye it was put on a private layer to stay out of"
        );
        assert_eq!(
            mask(&world, plate),
            RenderLayers::default(),
            "a projection that declares no resting layers still rests on the \
             world layer, so nothing about the ordinary case moved"
        );
    }

    /// A retired view leaves the survivor reset, not stripped: the remaining
    /// projection keeps its `RenderLayers`, set back to the default.
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

        // The second view goes away with its whole projection set, as a
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

    /// View half, cut V3: each camera draws only the live room its view
    /// frames. Two live rooms, two views (one frames #0, one #1), two cameras.
    /// A sprite in each room, the second with a child, and an unstamped
    /// sprite. Each camera draws its own room's sprite and the unstamped one,
    /// and not the other room's; the child goes with its parent. When #1 is
    /// no longer live, every entity is back on the world layer and both
    /// cameras draw all of it. The control: one live room bands nothing.
    #[test]
    fn each_camera_draws_only_the_live_room_its_view_frames() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
        let first = LiveRoomInstance::ACTIVATION;
        let second = first.next();
        let mut world = World::new();
        let roots = [first, second].map(|room| world.spawn((RoomInstanceRoot, room)).id());
        let views = [first, second].map(|room| {
            world
                .spawn((
                    LocalView,
                    LocalViewId(if room == first { 0 } else { 1 }),
                    ambition_sim_view::camera_snapshot::ResolvedCameraSnapshot(Some(
                        ambition_sim_view::camera_snapshot::ResolvedCameraFrame {
                            snapshot: Default::default(),
                            follow_world: Default::default(),
                            room,
                        },
                    )),
                ))
                .id()
        });
        let cameras = views.map(|view| world.spawn((MainCamera, authored_camera_layers(), PresentsView(view))).id());
        let in_first = world.spawn(InRoomInstance(first)).id();
        let in_second = world.spawn(InRoomInstance(second)).id();
        let child = world.spawn(ChildOf(in_second)).id();
        let shared = world.spawn_empty().id();

        world.run_system_once(isolate_live_rooms).expect("the pass runs");
        let draws = |world: &World| {
            cameras.map(|camera| [in_first, in_second, child, shared].map(|entity| camera_draws(world, camera, entity)))
        };
        assert_eq!(
            draws(&world),
            [[true, false, false, true], [false, true, true, true]],
            "a camera drew another live room, or not its own"
        );

        world.entity_mut(roots[1]).despawn();
        world.run_system_once(isolate_live_rooms).expect("the pass runs");
        assert_eq!(draws(&world), [[true; 4]; 2], "one live room again, and a camera does not draw all of it");
        assert_eq!(
            [mask(&world, cameras[0]), mask(&world, in_second), mask(&world, child)],
            [authored_camera_layers(), RenderLayers::default(), RenderLayers::default()],
            "a mask did not return to the world layer"
        );
    }
}
