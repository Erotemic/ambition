//! Domain-owned rollback declarations; the host supplies the backend registrar.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.rollback_resource_canonical::<crate::time_control::RequestedClockScale>(
        OWNER,
        "resource.requested_clock_scale",
    );
    registrar.rollback_resource_canonical::<crate::time_control::RegimePolicy>(
        OWNER,
        "resource.clock_regime_policy",
    );
    registrar.clear_message_on_rollback::<crate::time_control::ClockResetRequest>(
        OWNER,
        "message.clock_reset_request",
    );
    registrar.clear_message_on_rollback::<crate::time_control::ClockScaleRequest>(
        OWNER,
        "message.clock_scale_request",
    );
    // ⛔⛔ THESE FOUR WERE DECLARED BY THE RUNTIME, under `ENGINE`, while this
    // function already existed three lines up. The same crate's rollback state
    // was owned in TWO places — the split the federation exists to end, and the
    // sharpest instance of it, because the domain-owned half was RIGHT HERE.
    //
    // ⭐ THE STABLE NAMES ARE UNCHANGED. They are identities on the wire, not
    // addresses, so moving the declaration is not a schema change; only the
    // OWNER string moves, from the composition to the crate that defines the
    // types.
    registrar.rollback_resource_canonical::<crate::SimTick>(OWNER, "resource.sim_tick");
    registrar.rollback_resource_canonical::<crate::WorldTime>(OWNER, "resource.world_time");
    registrar.rollback_resource_canonical::<crate::ClockState>(OWNER, "resource.clock_state");
    registrar
        .rollback_component_canonical::<crate::ProperTimeScale>(OWNER, "actor.proper_time_scale");
}
