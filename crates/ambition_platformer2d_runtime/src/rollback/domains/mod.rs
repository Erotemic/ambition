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
//!   friends. `bevy_ggrs` 0.21 offers no non-generic path: every registration
//!   (clone, copy, and even the `Reflect` strategy) is generic over the concrete
//!   type, so SOMETHING must monomorphize it, and that something must be able to
//!   name `bevy_ggrs`. Only this crate and `ambition_app` may — every other
//!   workspace crate is `bevy_ggrs`-free, which is a boundary worth more than
//!   this seam.
//!
//! ⛔⛔ **that second bullet was read as a BLOCKER, and it is not one** (falsified
//! 2026-08-15, `ambition_platformer2d_world::rooms::GatePortalPhases`). "The
//! registration API is generic over `T`" says only that a monomorphizing call
//! site must exist somewhere it can name `bevy_ggrs`. It does NOT say the LIST of
//! `T`s must live in that crate's source — the call site can be a trait method
//! the domain invokes. `AmbitionRollbackApp` was already that shape, one crate
//! too high up.
//!
//! ⇒ **the real gate is where the VOCABULARY lives, and that gate is now open.**
//! `ambition_platformer2d_core::snapshot::RollbackRegistrar` is the
//! backend-neutral trait (floor, no `bevy_ggrs`, no `bevy_app`);
//! `super::registrar::GgrsRollbackRegistrar` is the runtime-owned `App` wrapper
//! that implements it — a wrapper because the orphan rule forbids implementing a
//! floor trait for foreign `App`. A domain below the runtime now writes
//! `register_*_rollback_state(&mut impl RollbackRegistrar)` in its own crate; the
//! composition hands it a registrar. ⭐ **so each module here is now inertia with
//! a known cure**, not an architectural floor, and the remaining work per domain
//! is: widen the floor trait with the methods that domain uses, move its
//! `register` body down, delete the module.
//!
//! ⚠ a crate ABOVE this one still owns its registration outright and needs
//! nothing from here (`ambition_content::bosses::specials::rollback`). ⛔ and do
//! not close the remaining gap with an Ambition-owned type-erased snapshot layer:
//! that is a second rollback implementation, not a seam. The registrar trait is
//! deliberately NOT object-safe for the same reason.

pub(super) mod actors;
pub(super) mod characters;
pub(super) mod combat;
pub(super) mod cutscene;
pub(super) mod encounter;
pub(super) mod items;
/// ⛔ NOT called by `register_engine_rollback_state` — see the module doc.
pub(crate) mod ldtk;
pub(super) mod lifecycle;
pub(super) mod portal;
pub(super) mod primitives;
pub(super) mod projectiles;
pub(super) mod vfx;
