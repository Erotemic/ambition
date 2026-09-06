//! Experience-provider boundary between shell loading and platformer runtime state.
//!
//! Providers declare authored identity/catalog fragments and supply a prepared
//! source for matching load transactions. Shared preparation validates and owns
//! each transaction's prepared session; activation moves that exact prepared
//! world onto the live session root. Hosts register providers explicitly; there
//! is no provider discovery mechanism.
//!
//! [`PreparedPlatformerSource`]: ambition_platformer2d_runtime::PreparedPlatformerSource

pub mod authoring;
pub mod composition;
pub mod lifecycle;

pub use authoring::{
    AuthoredCatalogFragments, PlatformerAuthoredCatalogRegistry,
    PlatformerAuthoringRegistrationError, PlatformerExperienceAuthoring,
};
pub use composition::ShellComposition;
pub use lifecycle::{
    prepare_platformer_content, prepare_platformer_content_for_app,
    prepare_world_replacement_candidate, PlatformerPreparationReport, PlatformerPreparationSet,
    FirstRoomArtContributor, PlatformerSessionBuilder, PreparedPlatformerSession,
    PreparedPlatformerSessions, SessionBuildResult,
};
