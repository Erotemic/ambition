//! World / level authoring runtime: room graph + spawning, the code-first
//! room builder, the Avian2D debris adapter (re-exported from its own crate),
//! and LDtk-authored moving platforms.
//!
//! Name the owning crate — or, from a game, the facade's `ambition_platformer2d:ldtk_map`.
//!
//! Long-term shape: this umbrella is the spine the future `ambition_platformer2d`
//! framework crate wraps. Module-internal `crate::rooms::…` paths still
//! resolve via re-exports at the crate root so this reorg is a pure
//! relocation.

pub mod authored_room_commands;
pub mod authored_switch_commands;
pub mod gated_lock_walls;
/// The Avian debris adapter, its own crate since 2026-10-23.
pub use ambition_platformer2d_physics_debris as physics;
pub mod platforms;
pub mod rooms;
