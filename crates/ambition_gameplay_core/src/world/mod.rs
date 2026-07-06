//! World / level authoring runtime: room graph + spawning, the code-first
//! room builder, the LDtk hot-reloadable project loader, the Avian2D
//! physics adapter, and LDtk-authored moving platforms.
//!
//! Long-term shape: this umbrella is the spine the future `ambition`
//! framework crate wraps. Module-internal `crate::rooms::…` paths still
//! resolve via re-exports at the crate root so this reorg is a pure
//! relocation.

pub mod ldtk_world;
pub mod overlay;
pub mod overlay_rebuild;
pub mod physics;
pub mod placements;
pub mod platforms;
pub mod ron_room;
pub mod rooms;
