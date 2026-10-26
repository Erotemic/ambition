//! Host viewpoints shared by portal presentation passes.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

/// One viewpoint: an eye in one live room. The host publishes one for each
/// live room that a local view frames ([`PortalViewers`]).
#[derive(Clone, Debug, Default)]
pub struct PortalViewer {
    /// Whether a controlled-character eye is available this frame.
    pub present: bool,
    /// The observer this eye is for: the entity the host records that
    /// observer's camera sample by ([`crate::PortalObserverViews`]). The windows of
    /// the eye's room are clipped to that camera and captured for it. `None`
    /// has no camera sample: a window is then clipped to its room.
    pub observer: Option<Entity>,
    /// The controlled character's eye position (body center), world space.
    pub eye: Vec2,
    /// The way the observer faces, in world space (unit), or zero for an eye
    /// that faces no way. Apertures don't open to observers that turn away:
    /// a window opens for an aperture ahead, above or below, never for one
    /// wholly behind.
    pub forward: Vec2,
    /// The live room the eye is in. Near and far are relative to an eye, so a
    /// window or a far-side composite is made only in this room; `None` is a
    /// room that cannot be told.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    /// The character's body half-size: line-of-sight is tested from all four
    /// body corners, so partial cover yields a partial (smoothly blended)
    /// window instead of a binary popping one.
    pub half_size: Vec2,
    /// The pair whose opening the observer cuts from behind
    /// ([`ambition_portal2d::PortalBackside`]), if any. It is on the back of
    /// that pair, and the pair's windows stay shut for it until it is wholly
    /// past the plane, as the pair will not take it until then.
    pub on_back_of: Option<ambition_portal2d::PortalChannel>,
    /// The portal the observer's body has crossed into and still straddles
    /// (the sim's crossing, `PortalTransit` once crossed), if any. That
    /// portal's window takes over from its partner's at once: where the
    /// observer just was is drawn by it, on the frame the body is moved.
    pub crossed_into: Option<ambition_portal2d::PortalChannel>,
    /// Solid AABBs for the line-of-sight test — a portal whose aperture is
    /// occluded from `eye` renders no window. The host syncs these from its
    /// collision world (only the blocks that block sight).
    pub occluders: Vec<ae::Aabb>,
}

/// Host viewpoints in publication order, with at most one eye per live room.
/// Window rendering and far-side compositing read the same eye for a room.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortalViewers {
    viewers: Vec<PortalViewer>,
}

impl PortalViewers {
    /// One viewpoint: the whole seam of a session with one observer.
    pub fn one(viewer: PortalViewer) -> Self {
        let mut viewers = Self::default();
        viewers.publish(viewer);
        viewers
    }

    /// Remove each viewpoint. The host calls this first in a frame.
    pub fn clear(&mut self) {
        self.viewers.clear();
    }

    /// Add a viewpoint. A second eye in a room that has one is refused: the
    /// first observer of a room, in the order the host publishes, is the eye
    /// of that room.
    pub fn publish(&mut self, viewer: PortalViewer) -> bool {
        if viewer.room.is_some() && self.viewers.iter().any(|held| held.room == viewer.room) {
            return false;
        }
        self.viewers.push(viewer);
        true
    }

    /// The eye of live room `room`. A room that cannot be told has no eye.
    pub fn in_room(&self, room: ambition_portal2d::PortalRoom) -> Option<&PortalViewer> {
        let room = room?;
        self.viewers.iter().find(|viewer| viewer.room == Some(room))
    }

    /// Each viewpoint, in the order the host published them.
    pub fn iter(&self) -> impl Iterator<Item = &PortalViewer> {
        self.viewers.iter()
    }

    /// The first viewpoint: the one a diagnostic of one eye describes.
    pub fn first(&self) -> Option<&PortalViewer> {
        self.viewers.first()
    }
}
