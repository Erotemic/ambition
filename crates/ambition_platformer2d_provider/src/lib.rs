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
pub mod presentation;
pub mod session_contents;
pub mod world_source;

pub use authoring::PlatformerExperienceAuthoring;
pub mod catalogs;
pub use catalogs::{
    AuthoredCatalogFragments, PlatformerAuthoredCatalogRegistry,
    PlatformerAuthoringRegistrationError,
};
pub use world_source::{IntoPreparedSource, WorldRefusal};
pub use composition::ShellComposition;
pub use session_contents::{SessionContents, SessionContentsCatalog, SessionContentsFn};
pub use lifecycle::{
    install_direct_session_root, prepare_platformer_content,
    prepare_platformer_content_for_app,
    prepare_world_replacement_candidate, PlatformerPreparationReport, PlatformerPreparationSet,
    FirstRoomArtContributor, PlatformerSessionBuilder, PreparedPlatformerSession,
    PreparedPlatformerSessions, SessionBuildResult,
};
