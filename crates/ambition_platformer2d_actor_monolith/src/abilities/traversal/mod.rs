//! Control-authority and authored-world traversal that shares the `abilities/`
//! directory name and nothing else. See the parent module for why these stayed.

/// The authored flyline a MOVE fires: a wire, a winch, and a pendulum.
/// ⛔ NOT beside [`teleport`] in behaviour — it picks no destination and moves
/// nothing; the kernel's `integrate_wire_clusters` owns every pixel of it.
pub mod flyline;
pub mod possession;
/// The authored teleport a MOVE fires, with the ledge assist a recovery needs.
/// ⚠ Its `blink_target` sibling moved to `ambition_abilities::traversal::blink`
/// in the carve; the one teleport rule is still shared, now across a crate line.
pub mod teleport;
pub mod trapdoor;
