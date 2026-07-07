//! Thrown abilities: gravity grenade, puppy-slug gun.
//!
//! Each submodule is a self-contained player ability/weapon mechanic tied
//! to a `crate::items::Item`. Moved here from the crate root in Stage 17
//! (`crate::abilities` layer) — pure relocation, no behavior change.

pub mod gravity_grenade;
pub mod puppy_slug_gun;
