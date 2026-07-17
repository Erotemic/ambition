//! Replaceable, contributor-neutral presentation for unresolved load barriers.
//!
//! The core plugin consumes [`ambition_load`] facts plus generic presentation
//! commands. Shell routes are supported by a thin adapter; room transitions and
//! future activation owners can drive the same lifecycle without fake routes.
//! The crate owns delayed reveal, honest semantic progress, optional activity
//! lifecycles, engagement, ready-hold, Continue, and scoped cleanup. It never
//! manufactures readiness or destination policy.

mod model;
mod plugin;
mod shell_adapter;

#[cfg(feature = "basic_presentation")]
mod basic_presentation;
#[cfg(feature = "basic_presentation")]
mod deterministic_activity;

pub use model::*;
pub use plugin::AmbitionLoadPresentationPlugin;
pub use shell_adapter::{AmbitionLoadShellPresentationPlugin, ShellLoadPresentationCatalog};

/// Stable identifier for the reusable neutral-input loading activity.
pub const DETERMINISTIC_LOADING_ACTIVITY_ID: &str = "ambition.loading.edge-practice";

#[cfg(feature = "basic_presentation")]
pub use basic_presentation::{BasicLoadPresentationPlugin, BasicLoadRoot};
#[cfg(feature = "basic_presentation")]
pub use deterministic_activity::DeterministicLoadingActivityPlugin;

use bevy::prelude::SystemSet;

/// Stable ordering seam for game-provided loading activities and presentations.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LoadPresentationSet {
    Observe,
    Activity,
    ActivitySignals,
    Drive,
    Input,
    Actions,
    Finalize,
    Cleanup,
    Render,
}

/// Contributor-neutral presentation composition. It can run without a shell.
pub struct MinimalLoadPresentationPlugins;

impl bevy::prelude::PluginGroup for MinimalLoadPresentationPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let builder =
            bevy::app::PluginGroupBuilder::start::<Self>().add(AmbitionLoadPresentationPlugin);
        #[cfg(feature = "basic_presentation")]
        let builder = builder
            .add(BasicLoadPresentationPlugin)
            .add(DeterministicLoadingActivityPlugin);
        builder
    }
}

/// Shell adapter plus the contributor-neutral presentation composition.
pub struct MinimalShellLoadPresentationPlugins;

impl bevy::prelude::PluginGroup for MinimalShellLoadPresentationPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let builder = bevy::app::PluginGroupBuilder::start::<Self>()
            .add(AmbitionLoadPresentationPlugin)
            .add(AmbitionLoadShellPresentationPlugin);
        #[cfg(feature = "basic_presentation")]
        let builder = builder
            .add(BasicLoadPresentationPlugin)
            .add(DeterministicLoadingActivityPlugin);
        builder
    }
}

/// Complete no-art composition for demos and early game integration.
///
/// Games may install the individual plugins instead when replacing either
/// shell or loading presentation. This group intentionally registers no routes,
/// loads, activities, or game content.
pub struct MinimalLoadShellPlugins;

impl bevy::prelude::PluginGroup for MinimalLoadShellPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let builder = bevy::app::PluginGroupBuilder::start::<Self>()
            .add(ambition_load::AmbitionLoadPlugin)
            .add(ambition_game_shell::AmbitionGameShellPlugin)
            .add(ambition_game_shell::GameplaySessionBridgePlugin)
            .add(ambition_game_shell::ShellSequencePlugin)
            .add(ambition_game_shell::ShellLauncherPlugin)
            .add(AmbitionLoadPresentationPlugin)
            .add(AmbitionLoadShellPresentationPlugin);
        #[cfg(feature = "basic_presentation")]
        let builder = builder
            .add(ambition_game_shell::BasicShellPresentationPlugin)
            .add(BasicLoadPresentationPlugin)
            .add(DeterministicLoadingActivityPlugin);
        builder
    }
}

#[cfg(test)]
mod tests;
