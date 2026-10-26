//! Rollback declarations for state owned by this crate.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register this domain's rollback state.
pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {    // the heal shrine, for the same reason as the portal gun pickup
    // . It carries `SimId`, `SpawnOrigin` and
    // `TransactionId`, had no anchor, and so those registrations were inert on
    // it. Its own component is waived as authored geometry — the heal reads it
    // and never writes it — but the anchor is not about the shrine's data; it is
    // about whether GGRS reproduces the ENTITY on a resimulated timeline.
    registrar.require_rollback::<crate::HealShrine>(OWNER, "entity:heal_shrine");

}
