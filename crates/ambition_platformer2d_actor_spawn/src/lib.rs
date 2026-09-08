//! Actor spawn/construction capability extracted from the platformer actor monolith.
//!
//! ⚠ ONE CONCERN: turning prepared characters and authored spawn facts into
//! complete simulated bodies — spawn requests, the spawn/materialization
//! routines, the body/brain BUILDERS (data in, components out), spawn-time NPC
//! policy, and the actor bundles a body is built from.
//!
//! ⛔ NOTHING HERE TOUCHES A LIVE ENTITY. The per-tick actor view
//! (`ActorMut`, `ActorClusterQueryData`), combat timing, provocation of a body
//! already in the world, the fighter-ladder projection over live brains and the
//! dismounted-rider rebuild all belong to the actor kernel
//! (`ambition_platformer2d_actor_monolith`), which consumes this crate from
//! above and is never named by it. A live system may call a builder here; it may
//! not find a query, a schedule membership or a timing constant here.
//! `scripts/tests/test_actor_spawn_boundary.py` holds that line.

pub mod actor_bundles;
pub mod actor_spawn;
pub mod character_body;

pub use actor_spawn::*;
pub use character_body::{grant_prepared_character_body, KitOwnership, ProjectedCharacterKit};
