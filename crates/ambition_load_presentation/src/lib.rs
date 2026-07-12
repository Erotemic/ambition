//! Replaceable, shell-integrated presentation for unresolved load barriers.
//!
//! The crate consumes [`ambition_load`] facts and [`ambition_game_shell`]
//! pending routes. It owns delayed reveal, honest semantic progress, optional
//! arbitrary activity lifecycles, engagement, ready-hold, Continue, and scoped
//! cleanup. It never manufactures readiness and contains no game-specific
//! activity branches.

mod model;
mod plugin;

#[cfg(feature = "basic_presentation")]
mod basic_presentation;

pub use model::*;
pub use plugin::AmbitionLoadPresentationPlugin;

#[cfg(feature = "basic_presentation")]
pub use basic_presentation::BasicLoadPresentationPlugin;

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

pub struct MinimalLoadPresentationPlugins;

impl bevy::prelude::PluginGroup for MinimalLoadPresentationPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let builder =
            bevy::app::PluginGroupBuilder::start::<Self>().add(AmbitionLoadPresentationPlugin);
        #[cfg(feature = "basic_presentation")]
        let builder = builder.add(BasicLoadPresentationPlugin);
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
            .add(ambition_game_shell::ShellSequencePlugin)
            .add(ambition_game_shell::ShellLauncherPlugin)
            .add(AmbitionLoadPresentationPlugin);
        #[cfg(feature = "basic_presentation")]
        let builder = builder
            .add(ambition_game_shell::BasicShellPresentationPlugin)
            .add(BasicLoadPresentationPlugin);
        builder
    }
}

#[cfg(test)]
mod tests;
