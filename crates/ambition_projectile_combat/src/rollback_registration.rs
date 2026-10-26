//! Rollback declaration for the allegiance fixed at shot launch.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    const OWNER: &str = env!("CARGO_PKG_NAME");
    // The shot's launch-time combat allegiance is intentionally owned here, not
    // by `ambition_projectiles`: the reusable projectile model is forbidden to
    // depend on the character/combat vocabulary from which this stamp is built.
    // The old central runtime adapter reached the type through
    // `runtime::projectile_schedule`; domain-owned registration instead follows
    // the concrete type to the actor-side projectile integration that owns it.
    registrar.rollback_component_canonical::<crate::ProjectileAllegiance>(
        OWNER,
        "projectile.allegiance",
    );
}
