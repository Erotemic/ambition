//! **Domain-owned rollback schema adapters** (Campaign 2).
//!
//! Central rollback registration knew too many gameplay-domain types: 336 of the
//! engine's 351 registrations were made by one function in `ambition_platformer2d_runtime`,
//! which made the runtime a mandatory edit point for every new domain and put
//! rollback semantics somewhere other than the domain that owns the state.
//!
//! Each module here registers ONE domain's schema. `register_engine_rollback_state`
//! calls them and stops naming their types.
//!
//! ⚠ **why these live in `ambition_platformer2d_runtime` and not in the domain crates.** The
//! registration vocabulary (`AmbitionRollbackApp`) lives here, and the domain
//! crates do not depend on this one — `ambition_projectiles` has no
//! `ambition_platformer2d_runtime` dependency and must not gain one, or the dependency
//! direction inverts. The campaign anticipates this: *"the adapter may live in
//! the domain crate or in a higher-level companion module if adding the schema
//! dependency to the primitive crate would invert dependencies."* Moving the
//! adapters into their crates needs R1's schema-vocabulary extraction first, and
//! that is its own slice.
//!
//! What this buys before that extraction: one place per domain instead of a
//! 1,600-line function, and a seam the extraction can move wholesale.
//!
//! ⭐ **the extraction is HALF DONE, and the remaining half is not ours to
//! finish** (measured 2026-08-15). A domain's rollback declaration is two
//! separable things:
//!
//! - **the semantics** — wire codec and checksum projection. Already federated:
//!   `SnapshotState` moved down to `ambition_platformer2d_core::snapshot`, and
//!   the orphan rule now puts each crate's codecs beside its types (see
//!   `ambition_platformer2d_world::snapshot_impls`, which deleted 2,688 lines
//!   from here). ⛔ so "the orphan rule forces it" is no longer why a projection
//!   would live in this crate — it is just inertia. `GatePortalPhases`'s
//!   projection was the last one authored here and has since moved down.
//! - **the installation** — `RollbackApp::rollback_resource_with_clone::<T>` and
//!   friends. ⛔ `bevy_ggrs` 0.21 offers NO non-generic path: every registration
//!   (clone, copy, and even the `Reflect` strategy) is generic over the concrete
//!   type, so SOMETHING must monomorphize it, and that something must be able to
//!   name `bevy_ggrs`. Only this crate and `ambition_app` may — every other
//!   workspace crate is `bevy_ggrs`-free, which is a boundary worth more than
//!   this seam.
//!
//! ⇒ a crate ABOVE this one owns its registration outright and needs nothing
//! from here (`ambition_content::bosses::specials::rollback` does exactly that).
//! A crate BELOW it cannot, and the deletion gate is a `bevy_ggrs` registration
//! API keyed on `ComponentId`/`TypeId` rather than a type parameter — upstream's
//! to open, not ours. ⛔ do not close it with an Ambition-owned type-erased
//! snapshot layer: that is a second rollback implementation, not a seam.

pub(super) mod actors;
pub(super) mod characters;
pub(super) mod combat;
pub(super) mod encounter;
pub(super) mod items;
pub(super) mod cutscene;
pub(super) mod lifecycle;
pub(super) mod portal;
pub(super) mod primitives;
pub(super) mod projectiles;
pub(super) mod vfx;
