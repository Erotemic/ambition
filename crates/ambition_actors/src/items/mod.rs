//! Actor-sim item adapters.
//!
//! The reusable item catalog, shop primitives, and inventory UI state live in
//! `ambition_items` (E8). The pickup/throw/projectile steppers stay here because
//! they mutate actor bodies, gravity, portals, abilities, and hit events.

pub use ambition_items::*;

pub mod persist;
pub mod pickup;
pub mod world_item;

pub use world_item::{spawn_world_item, WorldItem, WorldItemPayload};
