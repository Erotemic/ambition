//! Shared read resource for transient ECS-derived world collision overlays.

use ambition_engine_core as ae;
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
