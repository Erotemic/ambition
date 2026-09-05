//! Default renderer for the headless [`ambition_portal2d`] mechanic.
//!
//! Provides placed-portal visuals, mid-transit body pieces, disorientation
//! indicators, through-portal view windows, and a sequestered compatibility
//! module for Ambition's portal-gun sprites. Hosts sync the
//! crate-owned seams ([`PortalWorldFrame`], [`PortalSceneBody`],
//! [`PortalAffordanceBody`], [`PortalBodyView`], [`PortalGunArt`],
//! [`PortalAimHint`]) and may replace any visual by disabling that
//! [`PortalPresentationPlugin`] flag and registering an alternative system.
//!
//! Depends only on `bevy`, `ambition_platformer2d_core`,
//! `ambition_platformer2d_shared_tangle`, and `ambition_portal2d`; it never names a host
//! crate.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;

mod camera_continuity;
mod compositing;
mod clip_material;
mod effects;
mod gun_visuals;
mod plugin;
#[cfg(feature = "effect_view_cones")]
mod view_cones;
mod visuals;

pub use camera_continuity::{
    camera_roll_for_portal_transit, PortalCameraContinuityCamera, PortalCameraContinuityConfig,
    PortalCameraContinuityFocus, PortalCameraContinuityHostView, PortalCameraContinuitySelection,
    PortalCameraContinuityState, PortalCameraTransitMode,
};
pub use clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, PortalClipMaterial,
    SpriteFrameBasis, CLIP_PLANE_OFF,
};
/// ⭐ RE-EXPORTED FOR HOSTS, not for convenience. A host composing portal
/// presentation must be able to say "run this only when a portal exists", and
/// every public system in this crate already queries the type — so the alternative
/// was a NEW DEPENDENCY EDGE from the renderer to `ambition_portal2d` purely to
/// name a `run_if` condition. One re-export costs nothing; an edge is priced by
/// the compile ratchet forever.
pub use ambition_portal2d::PlacedPortal;
pub use compositing::{
    current_z_policy_is_correct_for, pane_relation, piece_clip_edges, uncovered_remainder,
    PaneRelation,
    UncoveredPiece, UncoveredPieces,
};
pub use effects::{PortalEffectSelection, PortalVisualEffect};
pub use gun_visuals::{sync_portal_mode_indicator, PortalModeIndicator};
pub use plugin::{PortalPresentationPlugin, PortalPresentationSet};
#[cfg(feature = "effect_view_cones")]
pub use view_cones::{
    debug_portal_view_zones, effective_portal_capture_budget, flush_portal_view_cone_debug_dump,
    handle_portal_view_cone_dump_hotkey, selected_portal_view_cone_debug_rows,
    sync_portal_view_cones, EffectivePortalCaptureBudget, PortalApertureLosQuality,
    PortalCaptureCameraMode, PortalCaptureQualityBudget, PortalConeMesh, PortalDebugOverlay,
    PortalViewConeConfig, PortalViewConeDebugDumpRequest, PortalViewConeDebugRow,
    PortalViewConeMode, PortalViewConeSourceClipPolicy, PortalViewConeVisibilityMode,
    PortalViewRig, PortalViewer, PORTAL_WINDOW_RENDER_LAYER,
};
pub use visuals::{
    sync_portal_body_pieces, sync_portal_disorientation_indicator, sync_portal_visuals,
    PortalBodyPiece, PortalDisorientIndicator, PortalVisual,
};

/// Host-observation systems that publish data into this crate's presentation
/// seams run in this set. Renderers can order presentation after it without
/// depending on a concrete host crate.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalObservationSet;

/// Through-portal composite z. The captured far-side image draws above the
/// exit body copy but below actors and the portal rim, so near-side actors still
/// occlude the aperture and the rim remains intact. Transiting body pieces stay
/// on world layers and are captured by disjoint wormhole views; doorway pairs
/// clip direct slices outside the thin slab. Overlapping panes use front-side
/// dominance with hysteresis (`view_cones::mesh::pane_z`) rather than radial
/// distance.
pub const PORTAL_WINDOW_Z: f32 = 9.5;
/// The exit-side body slice z (just below [`PORTAL_WINDOW_Z`]).
pub const PORTAL_EXIT_COPY_Z: f32 = 9.4;
/// Portal rim/core/label overlay z: above the window and exit slice, below
/// actors. The thin rim therefore stays intact while near-side bodies can still
/// occlude the whole portal.
pub const PORTAL_RIM_OVERLAY_Z: f32 = 10.0;

/// The host-world half of the render transform: the world's size, copied from
/// the host each frame. Engine coordinates are top-left-origin y-down; Bevy's
/// 2D camera is centered y-up; [`Self::to_render`] is the one adapter between
/// them (delegating to `ambition_platformer2d_core::config::world_size_to_bevy` so
/// the math is defined exactly once).
///
/// Host seam: keep `size` synced (e.g. from Ambition's `RoomGeometry`). A zero
/// size just centers everything on the camera origin for a frame — wrong but
/// harmless until the first sync runs.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PortalWorldFrame {
    /// World size in engine units (the world's bottom-right corner).
    pub size: Vec2,
}

impl PortalWorldFrame {
    /// Engine world position → Bevy render translation at layer `z`.
    pub fn to_render(&self, p: Vec2, z: f32) -> Vec3 {
        ae::config::world_size_to_bevy(self.size, p, z)
    }
}

/// Host seam: marks the visual entity whose sprite the mid-transit body-piece
/// decomposition draws (in Ambition, the player's sprite entity). The entity
/// must also carry a [`PortalBodyView`] plus `Sprite` + `Visibility`;
/// `PortalTransit` / `ActorRoll` are read when present.
#[derive(Component)]
pub struct PortalSceneBody;

/// Host seam: marks the body whose portal AFFORDANCES draw — the held gun and
/// the disorientation indicator.
///
/// Separate from [`PortalSceneBody`] because the two answer different
/// questions: the scene body is *whose sprite gets decomposed at the seam*,
/// the affordance body is *who is operating the portals*. In Ambition they are
/// the same entity; a spectator viewpoint watching another body is exactly the
/// case where they diverge, and nothing here assumes they don't.
///
/// The host decides who that is (and re-tags when control moves). This crate
/// deliberately does not know what a "player" is — a portal carrier can be any
/// body, script, or emitter the host chooses.
#[derive(Component)]
pub struct PortalAffordanceBody;

/// Host seam: a drawable this crate must be able to COMPOSITE against a pane.
///
/// ⛔⛔ WITHOUT THIS THE CRATE CANNOT SEE THE BUG IT HAS. Its only body seams are
/// [`PortalSceneBody`] (ONE entity — whose sprite is decomposed at the seam) and
/// [`PortalAffordanceBody`] (whoever operates the portals). An ordinary NPC
/// standing behind an aperture is neither, so portal presentation had no way to
/// know it exists — while the renderer happily drew it at
/// `WORLD_Z_DUMMY + 1.0`, above every pane. The far-side actor punching through
/// a seamless window (Jon, 2026-09-05) is invisible from inside this crate.
///
/// ⭐ THE POPULATION WIDENS, NOT THE VOCABULARY. A tagged entity carries the
/// same [`PortalBodyView`] the other two seams already publish; nothing new is
/// invented for it. The host decides who is drawable — this crate still does not
/// know what a "player" or an "NPC" is.
///
/// ⚠ TAGGING IS OPTIONAL AND ITS ABSENCE IS SILENT, which is the honest cost: a
/// host that tags nothing gets today's behaviour and an empty compositing
/// report. The diagnostics print the CANDIDATE COUNT for that reason — a report
/// that says "0 overlapping drawables" beside a screenshot showing one is
/// telling you the host has not tagged them, not that the geometry is fine.
///
/// ⛔⛔ IT CARRIES ITS OWN DRAWN BOUNDS AND DOES NOT REUSE [`PortalBodyView`],
/// which was the tempting shortcut and would have made the report LIE.
/// `PortalBodyView::size` is the COLLISION box — its own doc says so, "crouch /
/// morph compaction included" — and the question here is which PIXELS a pane
/// should cover. A sprite is routinely taller and wider than the box that
/// collides, so a report built on collision bounds would miss exactly the
/// overhanging part of the sprite that punches through the window, which is the
/// part of the screenshot Jon circled.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalCompositingCandidate {
    /// World-space centre of the drawn sprite (engine coordinates).
    pub drawn_centre: Vec2,
    /// Half-extent of the drawn sprite, INCLUDING whatever the collision box
    /// does not cover.
    pub drawn_half: Vec2,
}

/// Host seam: the body-pose facts portal presentation places visuals against.
///
/// Published by the host onto every entity this crate must draw for — the
/// [`PortalSceneBody`] and the [`PortalAffordanceBody`]. Plain `Copy` data, so
/// presentation never reads a live host body component and the two sides never
/// have to agree on which type owns a pose. This is the same host-publishes-
/// facts shape as [`PortalCameraContinuityHostView`], applied to bodies.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PortalBodyView {
    /// Body centre in engine world coordinates (top-left origin, y-down).
    pub pos: Vec2,
    /// Current collision-box size (crouch / morph compaction included).
    pub size: Vec2,
    /// Facing sign: `>= 0.0` faces +x. Only the sign is read.
    pub facing: f32,
}

/// Host seam: the loaded portal-gun art (blue / orange mode sprites). The
/// crate defines the resource; the HOST loads it — asset paths are content.
/// Absent resource → the held gun doesn't draw and the ground pickup falls
/// back to a marker quad.
#[derive(Resource)]
pub struct PortalGunArt {
    pub blue: Handle<Image>,
    pub orange: Handle<Image>,
}

/// Host seam: content-agnostic aim hint for the held-gun presentation — the
/// resolved world-space direction the barrel should point (the same aim the
/// host's input adapter resolves for `FirePortalGun`). The host writes it each
/// frame; [`sync_portal_mode_indicator`] reads it, so portal presentation
/// never imports a host input type. Zero / unset aim falls back to facing.
///
/// Initialised by [`PortalPresentationPlugin`] (it is render-only state, so it
/// lives here rather than in the headless mechanic's plugin).
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PortalAimHint {
    /// Resolved aim direction (need not be normalized; zero falls back to facing).
    pub aim: Vec2,
}
