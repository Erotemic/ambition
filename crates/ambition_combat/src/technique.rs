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
    check_hydrates, NestedReferences, ParamCheck, TechniqueConflict, TechniqueDelivery,
    TechniqueOffer, TechniqueParams, TechniqueRefusal, TechniqueSupport,
};

/// The techniques this composition installed handlers for.
///
/// ⭐ A KEY IN HERE MEANS SOMETHING INSTALLED ANSWERS IT, because the only way in
/// is the statement that adds the handler system. That is the property a
/// metadata registry cannot have, and the reason its predecessor —
/// `ParamSchemaRegistry`, which had zero production callers — could not tell a
/// misspelled effect key from a real one.
///
/// ⛔⛔ **IT STAYS HERE, AND A REVIEW HAD TO SAY SO.** It was briefly moved down
/// into `ambition_characters` so the preparation barrier — which lives there and
/// cannot depend on this crate — could read it. That was DEPENDENCY CONVENIENCE
/// WEARING OWNERSHIP'S CLOTHES, the same category of error as the earlier
/// `actor_spawn` mistake: what this table records is *which native handlers the
/// selected application composition installed*, which is composition/runtime
/// state and is not a fact about a character definition. Moving a resource down
/// the graph because the desired consumer cannot see upward does not move who
/// owns the fact.
///
/// ⇒ The check reaches it the other way instead, and needs no new abstraction:
/// [`TechniqueSupport`] is a plain data type in `ambition_entity_catalog`, which
/// `ambition_characters` already depends on, so composition passes the table
/// INTO a fallible preparation/publication function as an ordinary argument.
/// Neither dependency reverses.
#[derive(bevy::prelude::Resource, Default)]
pub struct InstalledTechniques(pub TechniqueSupport);
