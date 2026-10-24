//! The durable save horizon's vocabulary: the slots a domain's save adapters
//! join, and the latch that says the loaded file is applied.
//!
//! The session installs and orders them (`session::durable_horizon` in the
//! actor kernel); a domain such as the item horizon only names them. They live
//! here, below both, so a domain that plugs into the save does not depend on
//! the session that runs it.

use bevy::prelude::{Resource, SystemSet};

/// Whether the loaded save has been applied to this world.
///
/// This is the single durable-restore latch across inventory and occurrence
/// domains. It gates behavior and is therefore rollback state, not a cache.
#[derive(Resource, Default, Clone)]
pub struct SaveRestored(pub bool);

/// Where a domain's durable adapters run in the simulation. A domain installs
/// its own systems here; the session orders the slots.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DurableHorizonSet {
    /// Other domains' live → save mirrors. Each domain chains its own members:
    /// they all take the save resource mutably.
    DomainMirror,
    /// The session's own live → save mirrors, after every domain's.
    SessionMirror,
}

/// Where a loaded file is applied, in the simulation schedule. The session
/// orders these slots.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DurableRestoreSet {
    /// The lifecycle/occurrence baseline, which the domains' restores build on.
    Lifecycle,
    /// Each domain applies its half of the file and adopts its own baselines.
    Domains,
    /// The file is fully applied; a checkpoint resume may now be requested.
    Complete,
}
