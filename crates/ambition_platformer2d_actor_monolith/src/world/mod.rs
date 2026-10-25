//! Live world mechanics and authored command execution.

pub mod authored_room_commands;
pub mod authored_switch_commands;
pub mod gated_lock_walls;
/// The Avian debris adapter, its own crate since 2026-10-23.
pub use ambition_platformer2d_physics_debris as physics;
pub mod platforms;
