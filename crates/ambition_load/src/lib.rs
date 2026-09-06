//! Headless, contributor-neutral loading coordination.
//!
//! Bevy and subsystem-specific loaders perform work. This crate records what a
//! game is waiting for, keeps activation barriers honest, and makes cancellation,
//! supersession, streaming, prefetch promotion, progress evidence, and commit
//! authorization composable. It never renders a loading screen and never owns a
//! game-specific destination.

mod coordinator;
mod id;
mod model;
mod plugin;

pub use coordinator::LoadCoordinator;
pub use id::{LoadBarrierId, LoadId, LoadWorkId};
pub use model::*;
pub use plugin::{AmbitionLoadPlugin, LoadCommand, LoadCommitRejection, LoadEvent};

use bevy::prelude::SystemSet;

/// Stable ordering seam for load contributors and command application.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AmbitionLoadSet {
    /// Subsystem adapters discover work and emit [`LoadCommand`] messages here.
    Contributors,
    /// The coordinator applies commands and publishes [`LoadEvent`] messages.
    Commands,
}

#[cfg(test)]
mod tests;
