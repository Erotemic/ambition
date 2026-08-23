//! Canonical live session-world surface.
//!
//! One exact [`SessionRoot`] entity owns the live world components for a
//! gameplay activation. Consumers read and mutate those components directly
//! through [`SessionWorldRef`] and [`SessionWorldMut`]. At frontend routes the
//! root does not exist, so gameplay-world access is structurally unavailable.
//! No process-resident projection or synchronization bridge exists.

pub use ambition_platformer2d_shared_tangle::lifecycle::{
    insert_session_world_component, session_world_component, session_world_component_mut,
    session_world_entity, session_world_exists, settle_until_controlled_subject,
    settle_until_session_world, SessionRoot, SessionWorldMut, SessionWorldRef,
    SESSION_SETTLE_FRAMES,
};
pub use ambition_platformer2d_runtime::{
    ContentDiagnostic, ContentEpoch, ContentFingerprint, ContentFingerprintSchemaVersion,
    PlatformerSessionCatalogs, PlatformerSessionRequests, PlatformerSessionWorld, PreparedContent,
    PreparedContentIdentity, PreparedPlatformerSource, SnapshotSchemaFingerprint,
};
