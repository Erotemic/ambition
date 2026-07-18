//! The platformer experience-provider layer.
//!
//! A provider is a content crate that registers one playable experience with a
//! shell host. This crate owns everything BETWEEN the host's shell/session/load
//! protocol (`ambition_game_shell`) and provider-independent runtime state
//! (`ambition_runtime`'s [`PreparedPlatformerSource`]):
//!
//! - [`PlatformerExperienceAuthoring`] + [`AuthoredCatalogFragments`] — the
//!   authored identity a provider declares: experience/route ids, starting
//!   character, audio expectations, loading presentation.
//! - The ONE preparation/activation lifecycle. A provider supplies a single
//!   source system producing its authored [`PreparedPlatformerSource`] value; this
//!   crate gives each matching load transaction an owned copy, validates the authored catalogs, publishes the typed
//!   `PreparedSessionIdentity`, and on activation moves the prepared world onto
//!   the live session root.
//!
//! The lifecycle, concretely:
//!
//! ```text
//! host selects an experience (shell route + provider load transaction)
//!         ↓
//! the provider's source system builds the authored PreparedPlatformerSource
//! for the current matching request batch
//!         ↓
//! shared preparation gives each transaction an owned copy and validates it
//! against the authored fragments
//! and publishes a PreparedSessionIdentity (owner: PreparedPlatformerSessions)
//!         ↓
//! shared activation takes the prepared world by that exact identity and
//! constructs the live session-scoped simulation world
//! ```
//!
//! Hosts register providers EXPLICITLY — a manifest dependency plus the
//! provider's plugin in the host's plugin tuple (the documented two-line
//! exception in `docs/planning/engine/architecture.md`). There is no provider
//! discovery mechanism, and none should be added.
//!
//! [`PreparedPlatformerSource`]: ambition_runtime::PreparedPlatformerSource

pub mod authoring;
pub mod lifecycle;

pub use authoring::{
    AuthoredCatalogFragments, PlatformerAuthoredCatalogRegistry,
    PlatformerAuthoringRegistrationError, PlatformerExperienceAuthoring,
};
pub use lifecycle::{
    prepare_platformer_content, prepare_platformer_content_for_app,
    prepare_world_replacement_candidate, PlatformerPreparationReport, PlatformerPreparationSet,
    PlatformerSessionBuilder, PreparedPlatformerSession, PreparedPlatformerSessions,
    SessionBuildResult,
};
