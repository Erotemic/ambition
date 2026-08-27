//! Actor-sim item adapters.
//!
//! The reusable item catalog, shop primitives, and inventory UI state live in
//! `ambition_items` (E8). The pickup/throw/projectile steppers stay here because
//! they mutate actor bodies, gravity, portals, abilities, and hit events.


pub mod conditions;
pub mod match_spawn;
pub mod narrative;
pub mod persist;
pub mod pickup;
pub mod world_item;

pub mod item_motion;
pub use item_motion::{
    step_item_motion, ItemEmerge, ItemMotion, ItemMotionPlan, DEFAULT_ITEM_GRAVITY,
};
pub use world_item::{
    collect_world_items, spawn_moving_world_item, spawn_world_item, WorldItem, WorldItemPayload,
};
