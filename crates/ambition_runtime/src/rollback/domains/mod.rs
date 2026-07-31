//! **Domain-owned rollback schema adapters** (Campaign 2).
//!
//! Central rollback registration knew too many gameplay-domain types: 336 of the
//! engine's 351 registrations were made by one function in `ambition_runtime`,
//! which made the runtime a mandatory edit point for every new domain and put
//! rollback semantics somewhere other than the domain that owns the state.
//!
//! Each module here registers ONE domain's schema. `register_engine_rollback_state`
//! calls them and stops naming their types.
//!
//! ⚠ **why these live in `ambition_runtime` and not in the domain crates.** The
//! registration vocabulary (`AmbitionRollbackApp`) lives here, and the domain
//! crates do not depend on this one — `ambition_projectiles` has no
//! `ambition_runtime` dependency and must not gain one, or the dependency
//! direction inverts. The campaign anticipates this: *"the adapter may live in
//! the domain crate or in a higher-level companion module if adding the schema
//! dependency to the primitive crate would invert dependencies."* Moving the
//! adapters into their crates needs R1's schema-vocabulary extraction first, and
//! that is its own slice.
//!
//! What this buys before that extraction: one place per domain instead of a
//! 1,600-line function, and a seam the extraction can move wholesale.

pub(super) mod actors;
pub(super) mod characters;
pub(super) mod combat;
pub(super) mod encounter;
pub(super) mod portal;
pub(super) mod primitives;
pub(super) mod projectiles;
