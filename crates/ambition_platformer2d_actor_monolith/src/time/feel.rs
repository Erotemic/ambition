//! `Platformer2dFeelTuningMonolith` moved DOWN to `ambition_combat::feel` on
//! 2026-08-21 (D33): every `crate::` path inside it already resolved into that
//! crate, and four crates BELOW this one were citing its fields in comments and
//! restating its constants because they could not name the type.
//!
//! Re-exported so `crate::time::feel::…` paths keep working, the same shape as
//! `features::RespawnPolicy` and `features::ActorSurfaceState`.

pub use ambition_combat::feel::Platformer2dFeelTuningMonolith;
