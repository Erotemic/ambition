//! Top-level game-shell routing without game-specific route names or rendering.
//!
//! A host configures separate initial and home routes. Registered experiences
//! receive scoped activation identities and report semantic completion, failure,
//! navigation, or `QuitToHome`; they never hard-code the menu that launched
//! them. The neutral sequence and launcher modules are reusable shell
//! experiences, not a universal gameplay state machine.

/// Run condition: a `T` message waits in its channel. It peeks, as
/// `ambition_platformer2d_core::a_message_waits` does and for the same reason:
/// Bevy's `on_message` takes the message even while a set the system is in is
/// off. This crate sits below that one, so it keeps its own copy.
pub(crate) fn a_message_waits<T: bevy::ecs::message::Message>(
    messages: Option<bevy::prelude::Res<bevy::ecs::message::Messages<T>>>,
) -> bool {
    messages.is_some_and(|messages| !messages.is_empty())
}

mod experience;
mod frontend;
mod id;
mod input;
mod launcher;
mod plugin;
mod preparation;
mod route_seating;
mod router;
mod scope;
mod sequence;
mod session;

#[cfg(feature = "basic_presentation")]
mod basic_presentation;

// Not gated: the ungated `plugin.rs` uses it.
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
pub use route_seating::{
    project_route_seating, route_seating_owner, RouteSeating, RouteSeatingAppExt,
    RouteSeatingCatalog, SeatCount,
};
pub use router::*;
pub use scope::{
    release_departed_experience_state, shell_experience_is_active, shell_scope_is_on_screen_or_routing,
    ExperienceScope,
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
