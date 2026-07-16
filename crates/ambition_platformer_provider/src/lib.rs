//! The platformer experience-provider layer.
//!
//! A provider is a content crate that registers one playable experience with a
//! shell host. This crate owns everything BETWEEN the host's shell/session/load
//! protocol (`ambition_game_shell`) and provider-independent runtime state
//! (`ambition_runtime`'s [`PlatformerSessionWorld`]):
//!
//! - [`PlatformerExperienceAuthoring`] + [`AuthoredCatalogFragments`] — the
//!   authored identity a provider declares: experience/route ids, starting
//!   character, audio expectations, loading presentation.
//! - The ONE preparation/activation lifecycle. A provider supplies a single
//!   source system producing its exact [`PlatformerSessionWorld`]; this crate
//!   validates the authored catalogs, publishes the typed
//!   `PreparedSessionIdentity`, and on activation moves the prepared world onto
//!   the live session root.
//!
//! The lifecycle, concretely:
//!
//! ```text
//! host selects an experience (shell route + provider load transaction)
//!         ↓
//! the provider's source system builds the exact PlatformerSessionWorld
//!         ↓
//! shared preparation validates catalogs against the authored fragments
//! and publishes a PreparedSessionIdentity (owner: PreparedPlatformerSessions)
//!         ↓
//! shared activation takes the prepared world by that exact identity and
//! constructs the live session-scoped simulation world
//! ```
//!
//! Hosts register providers EXPLICITLY — a manifest dependency plus the
//! provider's plugin in the host's plugin tuple (the documented two-line
//! exception in `docs/planning/architecture.md`). There is no provider
//! discovery mechanism, and none should be added.
//!
//! [`PlatformerSessionWorld`]: ambition_runtime::PlatformerSessionWorld

pub mod authoring;
pub mod lifecycle;

pub use authoring::{
    AuthoredCatalogFragments, PlatformerAuthoredCatalogRegistry, PlatformerExperienceAuthoring,
};
pub use lifecycle::{
    PlatformerPreparationReport, PlatformerPreparationSet, PlatformerSessionBuilder,
    PreparedPlatformerSessions, SessionBuildResult,
};
