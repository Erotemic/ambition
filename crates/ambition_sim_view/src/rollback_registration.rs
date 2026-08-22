//! Rollback declaration owned by `ambition_sim_view`.
//!
//! This module names this domain's concrete rewindable state while the host
//! supplies the backend through [`RollbackRegistrar`]. It deliberately contains
//! no `bevy_ggrs` dependency and no host/composition logic.
//!
//! THE FIRST ONE THIS CRATE HAS OWNED, and it arrived with `affordances`
//! . Every view here is rebuilt in the sim tail from sim state, so
//! nothing needed declaring: a rewind restores the state and the next rebuild
//! re-derives the view. The affordance table works the same way — all four of
//! these say *"recomputed per frame"* — but it is READ during the tick rather
//! than only at the end, so the sweep has to be told it is derived rather than
//! authoritative. That is what `declare_rollback_derived_resource` says.
//!
//! the reasons below are carried VERBATIM from the monolith's registrar. A
//! declaration answers *"is this per-frame state?"*, and moving a type between
//! crates does not change that answer — only where the question is asked.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register everything the observation domain needs the sweep to know about.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar
        .declare_rollback_derived_resource::<crate::affordances::interactable_proximity::NearestInteractable>(
            OWNER,
            "derived.nearest_interactable",
            "proximity read model recomputed per frame",
        );
}
