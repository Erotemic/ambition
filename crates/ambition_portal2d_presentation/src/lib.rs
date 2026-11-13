//! Default renderer for the headless [`ambition_portal2d`] mechanic.
//!
//! Provides placed-portal visuals, shots in flight, mid-transit body pieces,
//! disorientation indicators and through-portal view windows. Each visual is
//! placed in the live room of the thing it draws ([`PortalFrames`]). Hosts
//! sync the crate-owned seams ([`PortalSceneBody`],
//! [`PortalAffordanceBody`], [`PortalBodyView`]) and may replace any visual by
//! disabling that
//! [`PortalPresentationPlugin`] flag and registering an alternative system.
//!
//! Depends only on `bevy`, `ambition_platformer2d_core`,
//! `ambition_platformer2d_shared_tangle`, and `ambition_portal2d`; it never names a host
//! crate.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;

mod camera_continuity;
mod carry;
mod clip_material;
mod effects;
mod glow;
mod shot_visuals;
mod limb_seams;
mod plugin;
#[cfg(feature = "effect_view_cones")]
mod view_cones;
mod visuals;
mod viewpoints;

pub use camera_continuity::{
    camera_roll_for_portal_transit, PortalCameraContinuityCamera, PortalCameraContinuityConfig,
    PortalCameraContinuityFocus, PortalCameraContinuityHostView, PortalCameraContinuitySelection,
    PortalCameraContinuityState, PortalCameraTransit, PortalCameraTransitMode,
    PortalCameraTransitRule, PortalObserverViews, PortalParallaxCarry,
};
pub use clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, PortalClipMaterial,
    SpriteFrameBasis,
};
/// Re-exported for hosts. A host must be able to say "run this only when a
/// portal exists" without a new dependency edge to `ambition_portal2d` just to
/// name a `run_if` condition.
// The channel types are re-exported too: `PlacedPortal::fixed` takes a
// `PortalChannel`, which takes a `PortalChannelColor`, so without them the
// struct cannot be built outside this crate.
pub use ambition_portal2d::{PlacedPortal, PortalChannel, PortalChannelColor};

/// Whether `app` simulates portals. The presentation reads the simulation's
/// state, so a host installs it only where this is true.
pub fn portals_are_simulated(app: &App) -> bool {
    app.is_plugin_added::<ambition_portal2d::PortalSimulationPlugin>()
}
pub use effects::{PortalEffectSelection, PortalVisualEffect};
pub use glow::{
    dress_held_light_lines, opening_length, sync_portal_glows, sync_portal_light_lines, HeldLightLineOf, PortalGlow, PortalGlowMaterial, PortalGlowStyle, PortalLightLine, APPEAR_S, DISSOLVE_S, ROOM_SETTLE_S,
};
pub use limb_seams::{
    eased_between, lay_light_lines_on_limb_seams, laid_on, LightLineOnLimb, LimbSeam, LimbSeams, LAY_REACH, LAY_S, LIMB_LINE_MARGIN,
    OVER_LIMB_Z,
};
pub use plugin::{PortalPresentationPlugin, PortalPresentationSet};
#[cfg(feature = "effect_view_cones")]
pub use view_cones::{
    debug_portal_view_zones, effective_portal_capture_budget, flush_portal_view_cone_debug_dump,
    handle_portal_view_cone_dump_hotkey, selected_portal_view_cone_debug_rows,
    sync_portal_view_cones, EffectivePortalCaptureBudget, PortalApertureLosQuality,
    PortalCaptureCameraMode, PortalCaptureQualityBudget, PortalConeMesh, PortalDebugOverlay,
    PortalViewConeConfig, PortalViewConeDebugDumpRequest, PortalViewConeDebugRow,
    PortalViewConeMode, PortalViewConeRule, PortalViewCones, PortalViewConeSourceClipPolicy,
    PortalViewConeVisibilityMode,
    PortalViewRig, PORTAL_WINDOW_RENDER_LAYER,
};
pub use viewpoints::{PortalViewer, PortalViewers};
pub use visuals::{
    sync_portal_disorientation_indicator, sync_portal_visuals, PortalBodyPiece, PortalDisorientIndicator,
    PortalVisual,
};
pub use carry::{
    carry_through_pairs, give_back_carried_layers, CarriedLayers, CarryQuadPool, PortalCarried,
    PortalCarryAtlas, PortalCarryBudget, PortalCarryLoad, PortalCarryCamera, PortalCarrySelection, PortalDrawnWith,
};

/// Host-observation systems that publish data into this crate's presentation
/// seams run in this set. Renderers can order presentation after it without
/// depending on a concrete host crate.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalObservationSet;

/// Through-portal composite z: over every drawable of the room's plane, and
/// under the atmosphere in front of it.
///
/// A window replaces everything under its coverage with the world the viewer
/// will see after crossing, so nothing of the near room may draw over it: not
/// a door, a sign, the scenery, a body behind the portal, nor that body's
/// nameplate and bars. The play plane reaches `WORLD_Z_FX + 8` (speech plates)
/// and `WORLD_Z_PLAYER + 18` (nameplates); the front motes start at
/// `WORLD_Z_FX + 10`, the foreground parallax at 45, the daylight veil at 60.
///
/// That hides no near-side body. The window mesh is the wedge from the
/// aperture face back into the host surface (`ambition_portal2d::view`), so a
/// body on the viewer's side is outside it whatever its z. The part of a body
/// past the face is under it, and the carry draws that part out of the exit,
/// where the capture sees it. Overlapping windows keep their viewer-dependent
/// order inside `z .. z + z_proximity_span` (`view_cones::mesh::pane_z`).
pub const PORTAL_WINDOW_Z: f32 = ae::config::WORLD_Z_FX + 9.0;
/// The lower edge of the exit-slice depth band, which ends at
/// [`PORTAL_EXIT_COPY_TOP`]. Carried families keep their source depth order
/// inside it.
pub const PORTAL_EXIT_COPY_Z: f32 = 9.4;
/// The top of the exit-slice band. A drawable drawn at or under it is the
/// room's ground (blocks, a portal's frame) or a carried copy: the carry never
/// carries it.
pub const PORTAL_EXIT_COPY_TOP: f32 = 9.5;
/// The z of a portal's frame, which is its line of light (`glow`) and its
/// label: under the exit slice and under the window.
///
/// A body is drawn over the light on each side of a seam. On the near side
/// the body is at its own z, far above. On the far side the body is the exit
/// slice, which is over the far portal's line in the window's capture. The
/// window covers the half of the near line that is behind its face, and the
/// capture shows the far portal's line in that place, so the seam is one line
/// of two halves.
pub const PORTAL_FRAME_Z: f32 = 9.3;

// Compile-time checks of the z stack that the doc comments above describe, so
// editing one number cannot reorder it. `band_tests` places the band against
// the rest of the world.
const _: () = assert!(
    PORTAL_EXIT_COPY_Z < PORTAL_EXIT_COPY_TOP && PORTAL_EXIT_COPY_TOP < PORTAL_WINDOW_Z,
    "the exit-side body slice draws BELOW the through-portal composite"
);
const _: () = assert!(
    PORTAL_FRAME_Z + 0.05 < PORTAL_EXIT_COPY_Z,
    "the frame (its line, and its label 0.05 above) draws BELOW the exit-side \
     body slice, so a body is over the light on the far side of a seam too"
);

/// The host-world half of the render transform: the size of one live room.
/// Engine coordinates are top-left-origin y-down; Bevy's 2D camera is centered
/// y-up; [`Self::to_render`] is the one adapter between them (delegating to
/// `ambition_platformer2d_core::config::world_size_to_bevy` so the math is
/// defined exactly once). Each live room has its own frame: read it with
/// [`PortalFrames`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

/// The live room of a portal, a body or a visual, and that room's render frame.
///
/// One rule for every visual in this crate: a visual is placed by the frame of
/// the live room of the thing it draws, and is stamped into that room
/// ([`PortalPlacement::stamp`]), so a view draws only the portal visuals of
/// the room it frames. The room is `LiveRooms::of`, the rule the portal
/// mechanic pairs portals by (`ambition_portal2d::rooms`). A thing whose room
/// cannot be told (with two live rooms, an unstamped entity) is not drawn.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PortalFrames<'w, 's> {
    roots: Query<
        'w,
        's,
        (
            &'static ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
            &'static ae::RoomGeometry,
        ),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    live: ambition_portal2d::PortalRooms<'w, 's>,
}

impl PortalFrames<'_, '_> {
    /// Where `entity` is drawn: its live room and that room's frame.
    pub fn of(&self, entity: Entity) -> Option<PortalPlacement> {
        self.in_room(self.room_of(entity))
    }

    /// The placement in live room `room`, when that room is live.
    pub fn in_room(&self, room: ambition_portal2d::PortalRoom) -> Option<PortalPlacement> {
        let room = room?;
        let (_, geometry) = self.roots.iter().find(|(live, _)| **live == room)?;
        Some(PortalPlacement {
            room,
            frame: PortalWorldFrame { size: geometry.0.size },
        })
    }

    /// The render band of live room `room` while two or more rooms are live
    /// (`None` with one): what a capture adds to see that room's world.
    pub fn band(&self, room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance) -> Option<usize> {
        ambition_platformer2d_shared_tangle::camera_layers::live_room_band(
            self.roots.iter().map(|(live, _)| *live),
            room,
        )
    }

    /// The live room of `entity`, by the rule [`Self::of`] uses.
    pub fn room_of(&self, entity: Entity) -> ambition_portal2d::PortalRoom {
        self.live.of(entity)
    }

    /// Each live room.
    pub fn live_rooms(&self) -> impl Iterator<Item = ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance> + '_ {
        self.roots.iter().map(|(live, _)| *live)
    }

    /// The placed portals grouped by live room, as the mechanic pairs them: a
    /// portal's partner is in its own room.
    pub fn portals_by_room<'a>(
        &self,
        portals: impl IntoIterator<Item = (Entity, &'a PlacedPortal)>,
    ) -> ambition_portal2d::PortalsByRoom {
        ambition_portal2d::PortalsByRoom::collect(portals, &self.live)
    }
}

/// Give a test app one live room of `size`: the activation room, whose frame
/// every portal visual of these tests is placed by.
#[cfg(test)]
pub(crate) fn one_live_room(app: &mut App, size: Vec2) {
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new("portal test room", size, size * 0.5, Vec::new())),
    );
}

/// A live room and its render frame: where one portal visual is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalPlacement {
    pub room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    pub frame: PortalWorldFrame,
}

impl PortalPlacement {
    /// The stamp that puts a spawned visual in this room's render band.
    pub fn stamp(&self) -> ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance {
        ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(self.room)
    }
}

/// Host seam: marks a visual entity whose sprite the mid-transit body-piece
/// decomposition draws (in Ambition, the player's sprite entity, and the
/// sprite of each other body while it is in transit). The entity must also
/// carry a [`PortalBodyView`] plus `Sprite` + `Visibility`;
/// [`PortalTransitView`] is read when present.
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

/// Host seam: the body-pose facts portal presentation places visuals against.
///
/// Published by the host onto every entity this crate must draw for — the
/// [`PortalSceneBody`] and the [`PortalAffordanceBody`]. Plain `Copy` data, so
/// presentation never reads a live host body component and the two sides never
/// have to agree on which type owns a pose. This is the same host-publishes-
/// facts shape as [`PortalCameraContinuityHostView`], applied to bodies.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalBodyView {
    /// Body centre in engine world coordinates (top-left origin, y-down).
    pub pos: Vec2,
    /// The body domain before its physical material transform.
    pub canonical_size: Vec2,
    /// The accepted physical scale and orientation.
    pub material: ae::frame::BodyMaterial,
    /// World-axis bounds for controls and labels.
    pub size: Vec2,
    /// Facing sign: `>= 0.0` faces +x. Only the sign is read.
    pub facing: f32,
    /// The simulation body whose portal policy applies to this visual.
    /// It can differ from the visual entity.
    pub body: Entity,
}

/// Host seam: this visual's body is straddling a portal plane right now.
///
/// Published by the host onto the visual of each body in transit, and taken
/// off when the transit ends. The body's own transit state is the portal
/// simulation's (`ambition_portal2d::PortalTransit`), and it can be on another
/// entity than the one that is drawn (an NPC's body and its sprite are two
/// entities). So presentation reads this fact and not that component, and
/// each body that crosses is cut the same way: a player, an NPC, a dog.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalTransitView {
    /// Channel of the portal whose plane the body straddles: the entry before
    /// its centre crosses, the exit after.
    pub straddling: PortalChannel,
}

/// The visual is a host's picture that a limb of it carries into the pair of
/// its [`PortalTransitView`] (a giant's arm, drawn by the giant): what went
/// through is the part past the plane, under the opening, and no deeper than
/// this (world units, the limb's reach); the rest of the picture is where it
/// is (`docs/planning/engine/portals-a-body-in-two-places.md`). A body has
/// none: all of it past the plane went through.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PortalHostReach(pub f32);

#[cfg(test)]
mod band_tests {

    /// The window, with the room its viewer-dependent order takes, is over
    /// the play plane and under the atmosphere in front of it.
    #[cfg(feature = "effect_view_cones")]
    #[test]
    fn the_window_is_over_the_play_plane_and_under_the_atmosphere() {
        use ambition_platformer2d_core::config::{WORLD_Z_FX, WORLD_Z_PLAYER};
        let band_top = crate::PORTAL_WINDOW_Z + crate::PortalViewConeConfig::default().z_proximity_span;
        for (what, z) in [
            ("a speech plate (WORLD_Z_FX + 8)", WORLD_Z_FX + 8.0),
            ("a nameplate (WORLD_Z_PLAYER + 18)", WORLD_Z_PLAYER + 18.0),
        ] {
            assert!(crate::PORTAL_WINDOW_Z > z, "{what} at {z} draws over a portal window at {}", crate::PORTAL_WINDOW_Z);
        }
        assert!(
            band_top < WORLD_Z_FX + 10.0,
            "the windows reach {band_top}, over the front motes (WORLD_Z_FX + 10 and up)"
        );
    }

    /// The frame and the exit slice stay under the cast: a body is drawn over
    /// a portal's line on both sides of a seam.
    #[test]
    fn the_frame_and_the_exit_slice_stay_under_the_cast() {
        assert!(crate::PORTAL_EXIT_COPY_TOP <= ambition_platformer2d_core::config::WORLD_Z_DUMMY);
        assert!(crate::PORTAL_FRAME_Z < crate::PORTAL_EXIT_COPY_Z);
    }
}
