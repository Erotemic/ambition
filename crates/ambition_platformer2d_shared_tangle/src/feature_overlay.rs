//! Shared read resource for transient ECS-derived world collision overlays.

use ambition_platformer2d_core as ae;
use bevy::prelude::Resource;

/// Collision/world contributions rebuilt from ECS feature state.
#[derive(Resource, Default, Clone, Debug)]
pub struct FeatureEcsWorldOverlay {
    pub blocks: Vec<ae::Block>,
    pub gate_solids: Vec<ae::Block>,
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
            portal_carves,
            removed_block_names,
            climbable_carves,
            water_regions,
        } = self;
        blocks.clear();
        gate_solids.clear();
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
    /// Retract by RESETTING, never by removing: every reader must keep reading
    /// the resource. The destructure has no `..` for the same reason as above:
    /// a new field must get a decision here.
    pub fn retract_for_room_change(&mut self) {
        let Self {
            blocks,
            gate_solids,
            portal_carves,
            removed_block_names,
            climbable_carves,
            water_regions,
        } = self;
        blocks.clear();
        gate_solids.clear();
        // The portal bridge refills this from `PortalCarves` on its next run,
        // as it does every frame.
        portal_carves.clear();
        removed_block_names.clear();
        climbable_carves.clear();
        water_regions.clear();
    }
}
