//! World / level authoring runtime: room graph + spawning, the code-first
//! room builder, the Avian2D physics adapter, and LDtk-authored moving
//! platforms.
//!
//! ⚠ **the LDtk backend is NOT here and no longer has a path through here.**
//! W3 moved it to `ambition_platformer2d_ldtk`; the `ldtk_world` blanket
//! re-export that kept the historical path alive was deleted once its last
//! consumer repointed. Name the owning crate — or, from a game, the facade's
//! `ambition_platformer2d::ldtk_map`.
//!
//! Long-term shape: this umbrella is the spine the future `ambition_platformer2d`
//! framework crate wraps. Module-internal `crate::rooms::…` paths still
//! resolve via re-exports at the crate root so this reorg is a pure
//! relocation.

pub mod authored_switch_commands;
pub mod gated_lock_walls;
pub mod overlay;
pub mod physics;
pub mod placements;
pub mod platforms;
pub mod rooms;
