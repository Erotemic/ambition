//! Top-level game-shell routing without game-specific route names or rendering.
//!
//! A host configures separate initial and home routes. Registered experiences
//! receive scoped activation identities and report semantic completion, failure,
//! navigation, or `QuitToHome`; they never hard-code the menu that launched
//! them. The neutral sequence and launcher modules are reusable shell
//! experiences, not a universal gameplay state machine.

mod experience;
mod frontend;
mod id;
mod input;
mod launcher;
mod plugin;
mod preparation;
mod router;
mod scope;
mod sequence;
mod session;

#[cfg(feature = "basic_presentation")]
mod basic_presentation;

// ⛔ UNGATED ON PURPOSE: `plugin.rs` is not gated and consumes these, so a
// presentation-gated home broke the crate at default features.
mod audio_controls;
#[cfg(feature = "basic_presentation")]
mod pause_menu;

mod abandon;
pub use abandon::{ShellAbandonOffer, ShellAbandonRequested};

pub use experience::{
    ExperienceAvailability, ExperienceRegistration, ShellExperienceAppExt, ShellExperienceRegistry,
};
pub use frontend::*;
pub use id::*;
pub use input::*;
pub use launcher::*;
pub use plugin::{
    AmbitionGameShellPlugin, ShellFailureLog, ShellLauncherPlugin, ShellSequencePlugin,
};
pub use preparation::*;
pub use router::*;
pub use scope::{
    release_departed_experience_state, shell_experience_is_active, ExperienceScope,
    ExperienceScopeBuilder, ReleaseKind, ShellExperienceScopeAppExt, ShellExperienceScopes,
};
pub use sequence::*;
pub use session::*;

#[cfg(feature = "basic_presentation")]
pub use basic_presentation::{BasicSequenceRoot, BasicShellPresentationPlugin, BasicShellUiRoot};

#[cfg(feature = "basic_presentation")]
pub use pause_menu::{ShellPauseMenu, ShellPauseMenuPlugin, ShellPauseMenuSuppressed};

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
        let builder = builder
            .add(BasicShellPresentationPlugin)
            .add(ShellPauseMenuPlugin);
        builder
    }
}

#[cfg(test)]
mod tests;
