//! Presentation camera markers shared by host, render, and app wiring.
//!
//! These are Bevy presentation vocabulary, not actor-domain state. Keeping them
//! below the actor crate lets render/host systems agree on camera identity
//! without depending on `ambition_platformer2d_actor_monolith`.

use bevy::prelude::*;

/// `RenderLayers` index the front HUD camera renders sprites from. The gameplay
/// world lives on the default layer 0, so picking a distinct layer here keeps the
/// front camera from double-drawing the world over the cube. (No sprites are placed
/// on this layer; the front camera only carries UI.)
pub const FRONT_HUD_LAYER: usize = 1;

/// `RenderLayers` index for camera-relative parallax panels — **the layer the
/// backdrop RESTS on while a session has one view.**
///
/// Portal capture cameras intentionally do not render this layer: a parallax
/// panel is placed against the eye of the camera that draws it, so rendering the
/// main view's panel into a capture rig samples the background from the wrong
/// eye. Rigs get their own copies on their own private layers instead.
///
/// ⚠ **a session with several views does not draw its backdrop here.** Every
/// main camera renders this layer, so two views sharing it would each draw both
/// views' panels; `ambition_render::rendering::view_isolation` moves each view's
/// set onto that view's own band and restores this layer — declared as the
/// panels' `ProjectionRestingLayers` — when the split collapses.
pub const PARALLAX_BACKGROUND_LAYER: usize = 2;

/// **Base of the per-LOCAL-VIEW band**, the layers a second observer's drawn
/// projections live on so one view's nameplates and label copies stay out of the
/// other view's picture.
///
/// ⚠ **this is a LEDGER entry, not a policy.** Which entity gets which layer is
/// decided by the render side (`ambition_render::rendering::view_isolation`);
/// what lives here is the reservation, because a layer collision is silent — two
/// tenants on one index simply draw into each other's cameras and no test asks.
/// The bands already spoken for: 0 the world, 1 the front HUD, 2 the
/// camera-relative parallax, 5 the portal window, 28..=30 twintrack's optical and
/// spacetime overlays, 32+slot the per-portal capture parallax (slot ≤ ~271), and
/// 512+slot the per-portal window self-layers. 1024 clears all of them with room
/// for the portal bands to grow.
///
/// ⭐ **and it grows UPWARD without a ceiling, deliberately.** `RenderLayers` is a
/// growable bitset, so `BASE + n` costs a longer mask and nothing else — the
/// renderer therefore imposes no maximum on how many local views a session may
/// have, which is the property that keeps this an implementation detail rather
/// than a limit the semantic layer inherits.
pub const LOCAL_VIEW_RENDER_LAYER_BASE: usize = 1024;

/// The layer reserved for the `ordinal`-th local view.
///
/// ⛔ **the argument is an ORDINAL AMONG THE LIVE VIEWS, never a `LocalViewId`.**
/// The id is a stable human-meaningful name a game chooses (`LocalViewId(7)` is
/// legal on its own), and binding it to a bit index would make a semantic
/// identity mean a GPU visibility mask. The caller sorts the live views by id and
/// passes the POSITION, so the mapping is dense, private to the renderer, and
/// replaceable without anything semantic noticing.
pub fn local_view_render_layer(ordinal: usize) -> usize {
    LOCAL_VIEW_RENDER_LAYER_BASE + ordinal
}

/// Marks the main gameplay camera (order 0) — the rig that draws the world for
/// one local view.
///
/// ⚠ **it is a marker, not a singleton.** A split composition has one per view,
/// and every consumer that needs to know WHICH one pairs this with the camera's
/// `ambition_sim_view::PresentsView` link rather than assuming uniqueness.
#[derive(Component)]
pub struct MainCamera;

/// Marks the front HUD/UI camera (order 9) that carries `IsDefaultUiCamera`.
#[derive(Component)]
pub struct FrontHudCamera;

/// **THE MAIN CAMERA A SINGLE-CAMERA COMPOSITION SPAWNED** — a spawn record, not
/// an answer to "where is the screen".
///
/// ⭐ **it has no production reader.** D116 M2 turned six process-global "the
/// view" singletons into components on a local view (`CameraViewport`,
/// `CameraScreenFraming`, `CameraPresentationInputs`, `CameraEaseState`,
/// `ResolvedCameraSnapshot`, `CameraViewState`); this seventh survived because
/// its one reader — the cube menu's full-screen dim-scrim — wanted a DISPLAY
/// answer rather than a view answer. That reader is gone: a full-screen scrim
/// now targets its own display-scoped UI camera (`retarget_kaleidoscope_scrim`),
/// because "the whole display" and "whichever main camera was inserted last" are
/// different questions that only coincide while a composition has exactly one
/// full-screen main camera.
///
/// ⛔⛔ **do not make it a display answer again.** Under a fixed-aspect profile a
/// main camera already carries a `Camera::viewport` (`apply_gameplay_camera_viewport`),
/// and under a split layout there are several of them — so a UI node targeted
/// here is laid out against ONE PANE, not the screen. Display-scoped UI belongs
/// on the front HUD camera (which is why the surround bars, which must cover
/// exactly what the gameplay camera does not, carry no `UiTargetCamera` at all)
/// or on a dedicated camera of its own.
///
/// ⚠ **and it is not "the" camera under several.** Both writers go through
/// [`publish_main_camera`], which publishes the FIRST and complains about a
/// second rather than letting the last writer win in silence. A composition with
/// two rigs must address them by [`MainCamera`] + the view each one presents
/// (`ambition_sim_view::PresentsView`), which is the question that still has an
/// answer when there are two.
#[derive(Resource, Clone, Copy)]
pub struct MainCameraEntity(pub Entity);

/// Publish the composition's main camera as [`MainCameraEntity`].
///
/// ⚠ **first writer wins, loudly.** Both shipped camera-spawn sites inserted the
/// resource unconditionally, so a composition that installed two of them got
/// whichever insert happened to be applied last, with nothing in the log. First
/// is no less arbitrary than last — the point is that the SECOND one is now an
/// event somebody can see, next to the `PresentsView` link that already refuses
/// to guess in the same situation.
///
/// The check is queued rather than read from an `Option<Res<_>>` because two
/// spawn systems in one `Startup` would both observe the resource absent before
/// the flush between them; a command applies in order, after every insert queued
/// before it.
pub fn publish_main_camera(commands: &mut Commands, camera: Entity) {
    commands.queue(move |world: &mut World| {
        if let Some(existing) = world.get_resource::<MainCameraEntity>() {
            if existing.0 != camera {
                tracing::error!(
                    ?camera,
                    published = ?existing.0,
                    "a second main-camera rig tried to publish itself as THE main \
                     camera. `MainCameraEntity` is a single-camera spawn record — \
                     address several rigs by `MainCamera` plus the view each one \
                     presents."
                );
            }
            return;
        }
        world.insert_resource(MainCameraEntity(camera));
    });
}
