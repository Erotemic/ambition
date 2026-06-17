//! Two self-contained, plugin-owning mechanics — NOT "all game mechanics".
//! Despite the name this is really just [`combat`] (the bulk: the generic
//! combat/feature kit — hazards, pickups, chests, breakables, switches,
//! targeting, hitboxes, path-motion, collision overlay) plus a small
//! [`gravity`] zone mechanic. Player abilities, items, and movement live
//! elsewhere in the crate. Each resident owns its own components, systems,
//! resources, and plugin registration (the Stage 12 `src/mechanics/` layout):
//! [`gravity`] extracted out of `crate::portal` (Stage 6 follow-up),
//! [`combat`] extracted out of `content/features` (Stage 20 / A2).

pub mod combat;
pub mod gravity;
