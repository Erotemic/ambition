//! Traversal abilities: blink, dive, flyline, grapple, possession, mark/recall, teleport.

pub mod blink;
pub mod dive;
/// The authored flyline a MOVE fires: a wire, a winch, and a pendulum.
/// ⛔ NOT beside [`teleport`] in behaviour — it picks no destination and moves
/// nothing; the kernel's `integrate_wire_clusters` owns every pixel of it.
pub mod flyline;
pub mod grapple;
pub mod mark_recall;
pub mod possession;
/// The authored teleport a MOVE fires, with the ledge assist a recovery needs.
/// Beside [`blink`] because they share `blink_target`, the one teleport rule.
pub mod teleport;
pub mod trapdoor;
