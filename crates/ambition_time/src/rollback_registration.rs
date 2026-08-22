//! This crate's own rollback declarations; the host supplies the backend registrar.
//!
//! ⭐ they are here because the TYPES are, and a domain that registers its own
//! state is what keeps the runtime free of gameplay type paths.

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
}
