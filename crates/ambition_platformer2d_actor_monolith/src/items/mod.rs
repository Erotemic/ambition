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

// ⭐ `world_item` AND `item_motion` LEFT THIS CRATE (D33, 2026-09-02) and are
// `ambition_world_items` now — the physical life of a touched collectible: where
// it is, whether it is moving, and that touching it collects it. Nothing is
// re-exported here on purpose. A re-export would keep this module as the
// discovery path for something it no longer owns, and the point of the carve is
// that a consumer names the crate that has the code.
//
// ⛔ THE SIBLING STAYED, and the split is along the collect TRIGGER rather than
// by size: `pickup` owns the PRESSED pickup (a held weapon grabbed with
// `Attack`), reaches `abilities`, `ability_cooldown`, `construction` and
// `shrine`, and holds 27 of this module's 51 references into the rest of the
// kernel. That is a much larger carve and is not this one.
