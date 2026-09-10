//! What this composition installed a technique handler for.
//!
//! ⛔⛔ **THE AUTHORITY IS `ambition_entity_catalog`'s AND THE SHELL IS HERE.**
//! `TechniqueSupport` is engine-free on purpose — authored content vocabulary
//! carries no bevy dependency — so the resource wrapper lives beside the crate
//! that owns effect EXECUTION (`strike::apply_effects`,
//! [`crate::strike::EffectExecutionSet`]) rather than in the catalog.
//!
//! ⚠ AND NOT IN THE RUNTIME, which is where it was first written. The runtime
//! composes; `architecture-boundaries.md` does not admit a direct
//! `ambition_entity_catalog` dependency there, and the workspace policy said so
//! before this crate did. Composition still owns the INSTALLING — see
//! `combat_schedule::install_technique` — and names this vocabulary through the
//! mechanic crate it already depends on.

pub use ambition_entity_catalog::{
    check_hydrates, ParamCheck, TechniqueConflict, TechniqueOffer, TechniqueParams,
    TechniqueRefusal, TechniqueSupport,
};

/// Re-exported: the shell moved to `ambition_characters::technique` because the
/// admission pass reads it at the character PREPARATION barrier, and that crate
/// cannot name this one. Effect execution still lives here; only the table's
/// definition moved down, so every `ambition_combat::technique::InstalledTechniques`
/// path keeps resolving.
pub use ambition_characters::technique::InstalledTechniques;
