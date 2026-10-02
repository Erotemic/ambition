//! A live room's transient collision contributions, rebuilt from ECS feature
//! state, on that room's own root.

use ambition_platformer2d_core as ae;
use bevy::ecs::system::SystemParam;
use bevy::prelude::{Component, Mut, Query, With};

use crate::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};

/// Collision/world contributions rebuilt from ECS feature state, for ONE live
/// room: a component on its `RoomInstanceRoot`, beside its geometry and
/// platforms (OW1 cut 3c). A second live room has its own.
/// The bodies that pass one gate solid (Q54: a body/capability gate is
/// evaluated per actor). The solid stays in `gate_solids`, so it is solid for
/// every reader that names no body; a body that names itself and is listed
/// here passes through it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GatePass {
    /// The `Block::name` of the gate solid.
    pub block: String,
    /// The bodies the gate is open for, in entity order.
    pub bodies: Vec<bevy::prelude::Entity>,
}

#[derive(Component, Default, Clone, Debug)]
pub struct FeatureEcsWorldOverlay {
    pub blocks: Vec<ae::Block>,
    pub gate_solids: Vec<ae::Block>,
    /// Per-actor openings of `gate_solids`; see [`GatePass`].
    pub gate_passes: Vec<GatePass>,
    pub portal_carves: Vec<ae::Aabb>,
    pub removed_block_names: Vec<String>,
    pub climbable_carves: Vec<ae::Aabb>,
    pub water_regions: Vec<ae::WaterRegion>,
}

impl FeatureEcsWorldOverlay {
    /// Clear the fields the engine's per-frame rebuild owns, so contributors
    /// re-extend onto a clean slate.
    ///
    /// ⭐⭐ THE DESTRUCTURE HAS NO `..`, AND THAT IS THE WHOLE POINT. This overlay
    /// has TWO owners over DISJOINT field sets: `rebuild_feature_ecs_world_overlay`
    /// clears five of six, and `portal_carves` is cleared and refilled every frame
    /// by the portal bridge from the portal-owned `PortalCarves` — so a frame with
    /// no transiting body re-seals the host wall. Both halves are correct and
    /// single-authority today.
    ///
    /// ⛔ WHAT WAS NOT ENFORCED IS THE SPLIT ITSELF. It lived as five hand-written
    /// `.clear()` calls plus a comment, so a SEVENTH field could be added and
    /// silently belong to neither owner — never cleared, accumulating across
    /// frames, and visible only as geometry that will not go away. Adding one now
    /// fails to compile (E0027) and lands its author here, where the question is
    /// "engine-owned or contributor-owned?".
    ///
    /// ⇒ The point is not to forbid a field. It is to make "who clears this?" a
    /// decision somebody wrote down rather than a default nobody noticed.
    pub fn clear_engine_contributions(&mut self) {
        let Self {
            blocks,
            gate_solids,
            gate_passes,
            portal_carves,
            removed_block_names,
            climbable_carves,
            water_regions,
        } = self;
        blocks.clear();
        gate_solids.clear();
        gate_passes.clear();
        removed_block_names.clear();
        climbable_carves.clear();
        water_regions.clear();
        // ⛔ NOT OURS. The portal bridge (`bridge_portal_carves`) clears and
        // refills this every frame; clearing it here would race that and blink the
        // aperture depending on system order.
        let _ = portal_carves;
    }

    /// Retract every contribution, from both owners, when the active room
    /// changes.
    ///
    /// The overlay is a statement about ONE room: each block, carve and water
    /// region is geometry of the room its contributors last saw. A room commit
    /// replaces the room geometry at once, but the next rebuild comes only on
    /// the next tick. Until then `CollisionWorld::solids()` would compose the
    /// new room with the walls of the room just left, and it cannot tell.
    /// An empty overlay states nothing about any room, which is true until
    /// the contributors rebuild it for the new one.
    ///
    /// The room publication calls this on the root of the live room it
    /// replaces, in the same step that writes that root's new geometry.
    ///
    /// Retract by RESETTING, never by removing: every reader must keep reading
    /// the component. The destructure has no `..` for the same reason as above:
    /// a new field must get a decision here.
    pub fn retract_for_room_change(&mut self) {
        let Self {
            blocks,
            gate_solids,
            gate_passes,
            portal_carves,
            removed_block_names,
            climbable_carves,
            water_regions,
        } = self;
        blocks.clear();
        gate_solids.clear();
        gate_passes.clear();
        // The portal bridge refills this from `PortalCarves` on its next run,
        // as it does every frame.
        portal_carves.clear();
        removed_block_names.clear();
        climbable_carves.clear();
        water_regions.clear();
    }
}

/// Every live room's overlay, for the systems that write contributions.
///
/// A contributor names the room its geometry is in: the `InRoomInstance` of
/// the entity that contributes it. An unstamped contributor writes the sole
/// live room's overlay, and nothing while there are two.
#[derive(SystemParam)]
pub struct RoomOverlays<'w, 's> {
    rooms: Query<
        'w,
        's,
        (&'static LiveRoomInstance, &'static mut FeatureEcsWorldOverlay),
        With<RoomInstanceRoot>,
    >,
}

impl RoomOverlays<'_, '_> {
    /// Every live room's overlay, for a rebuild that clears them all.
    pub fn each(&mut self) -> impl Iterator<Item = Mut<'_, FeatureEcsWorldOverlay>> {
        self.rooms.iter_mut().map(|(_, overlay)| overlay)
    }

    /// The overlay of the live room `room` names.
    pub fn for_room(&mut self, room: Option<&InRoomInstance>) -> Option<Mut<'_, FeatureEcsWorldOverlay>> {
        match room {
            Some(room) => self
                .rooms
                .iter_mut()
                .find(|(live, _)| **live == room.0)
                .map(|(_, overlay)| overlay),
            None => self.sole(),
        }
    }

    /// The sole live room's overlay. ⚠ The one-live-room write, the same debt
    /// as `SoleLiveRoom`: a contributor that says no room.
    pub fn sole(&mut self) -> Option<Mut<'_, FeatureEcsWorldOverlay>> {
        self.rooms.single_mut().ok().map(|(_, overlay)| overlay)
    }
}
