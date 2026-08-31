//! Canonical live session-world surface.
//!
//! One exact [`SessionRoot`] entity owns the live world components for a
//! gameplay activation. Consumers read and mutate those components directly
//! through [`SessionWorldRef`] and [`SessionWorldMut`]. At frontend routes the
//! root does not exist, so gameplay-world access is structurally unavailable.
//! No process-resident projection or synchronization bridge exists.

/// ⭐ THE DURABLE STATE A SESSION RESUMES FROM is a SESSION concept, not a
/// crate-shaped `persistence::` path. The sim harness can boot a session with a
/// save file already loaded — the shape the shipped binary has, where
/// `load_save_at_startup` puts the bytes in the world before anything is built —
/// and it should be able to say so without naming an implementation crate. This
/// is the SDK gap `sim-harness-names-only-the-public-sdk` predicts by name.
pub use ambition_persistence::save::AmbitionGameSave;
pub use ambition_persistence::save_data::AmbitionGameSaveData;
pub use ambition_platformer2d_runtime::{
    ContentDiagnostic, ContentEpoch, ContentFingerprint, ContentFingerprintSchemaVersion,
    PlatformerSessionCatalogs, PlatformerSessionRequests, PlatformerSessionWorld, PreparedContent,
    PreparedContentIdentity, PreparedPlatformerSource, SnapshotSchemaFingerprint,
};
pub use ambition_platformer2d_shared_tangle::lifecycle::{
    insert_session_world_component, session_world_component, session_world_component_mut,
    session_world_entity, session_world_exists, settle_until_controlled_subject,
    settle_until_session_world, SessionRoot, SessionWorldMut, SessionWorldRef,
    SESSION_SETTLE_FRAMES,
};
