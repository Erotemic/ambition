//! Ranged abilities: beam, meteor, shockwave, vortex, volley, bomb, sentry.
//!
//! Each submodule is a self-contained player ability/weapon mechanic tied
//! to a `crate::items::Item`. Moved here from the crate root in Stage 17
//! (`crate::abilities` layer) — pure relocation, no behavior change.

pub mod beam;
pub mod bomb;
pub mod meteor;
pub mod sentry;
pub mod shockwave;
pub mod volley;
pub mod vortex;
