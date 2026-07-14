//! Top-level game-shell routing without game-specific route names or rendering.
//!
//! A host configures separate initial and home routes. Registered experiences
//! receive scoped activation identities and report semantic completion, failure,
//! navigation, or `QuitToHome`; they never hard-code the menu that launched
//! them. The neutral sequence and launcher modules are reusable shell
//! experiences, not a universal gameplay state machine.

mod experience;
mod input;
mod id;
mod launcher;
mod plugin;
mod preparation;
mod router;
mod sequence;
mod session;

#[cfg(feature = "basic_presentation")]
mod basic_presentation;

pub use experience::{
    ExperienceAvailability, ExperienceRegistration, ShellExperienceAppExt, ShellExperienceRegistry,
};
pub use id::*;
pub use input::*;
pub use launcher::*;
pub use plugin::{AmbitionGameShellPlugin, ShellLauncherPlugin, ShellSequencePlugin};
pub use preparation::*;
pub use router::*;
pub use sequence::*;
pub use session::*;

#[cfg(feature = "basic_presentation")]
pub use basic_presentation::BasicShellPresentationPlugin;

use bevy::prelude::SystemSet;

#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AmbitionGameShellSet {
    Commands,
    Pending,
    Cleanup,
}

/// Stable scheduling seam for arbitrary programmatic shell segments.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ShellSequenceSet {
    Sync,
    Tick,
    Programmatic,
    Commands,
    Cleanup,
}

pub struct MinimalShellPlugins;

impl bevy::prelude::PluginGroup for MinimalShellPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let builder = bevy::app::PluginGroupBuilder::start::<Self>()
            .add(AmbitionGameShellPlugin)
            .add(GameplaySessionBridgePlugin)
            .add(ShellSequencePlugin)
            .add(ShellLauncherPlugin);
        #[cfg(feature = "basic_presentation")]
        let builder = builder.add(BasicShellPresentationPlugin);
        builder
    }
}

#[cfg(test)]
mod tests;
