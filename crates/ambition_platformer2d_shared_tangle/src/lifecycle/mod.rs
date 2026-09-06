//! Lifecycle vocabulary for reusable platformer entities.
//!
//! Scope markers define teardown boundaries (room, mode, round, session);
//! residency describes which room currently owns an entity;
//! [`AuthoredOccurrences`] records durable whereabouts needed for reconstruction.
//! New spawn sites should prefer the scoped spawn helpers over inserting marker
//! components directly. Entities with no scope marker survive all scope sweeps.

mod cleanup;
mod continuity;
mod custody_horizon;
pub mod horizon;
mod markers;
mod round;
mod session;
mod spawn_ext;

pub use cleanup::despawn_scoped_entity;
pub use continuity::{
    capture_occurrence_baseline, project_custody_onto_authored_occurrences,
    restore_occurrence_baseline, AuthoredOccurrences, OccurrenceBaseline, OccurrenceDisposition,
    OccurrenceWhereabouts, RoomOccurrenceOutlook,
};
pub use custody_horizon::{capture_custody_baseline, live_custody_rows, CustodyBaseline};
pub use horizon::{
    CheckpointCapture, CheckpointCommitted, CheckpointRestore, LifecycleCheckpointHorizonPlugin,
    ResetToCheckpoint,
};
pub use markers::{
    BodyCustodySettled, FeatureSimEntity, InCustodyOf, LoadingZoneVisual, ModeScopedEntity,
    PlayerVisual, PosedBody, RoomResident, RoomScopedEntity, RoomVisual,
};
pub use round::{
    despawn_departed_round_entities, ActiveRoundScope, RoundScopeId, RoundScopePlugin,
    RoundScopedEntity, RoundSpawnScope,
};
pub use session::{
    despawn_retired_session_entities, insert_session_world_component, live_session_scope,
    session_world_component, session_world_component_mut, session_world_entity,
    session_world_exists, settle_until_controlled_subject, settle_until_session_world,
    simulation_authorized, ActiveSessionScope, InitialGameplayReadiness, LiveSessionScope,
    SessionCommands, SessionGatedSimulation,
    SessionRoot, SessionScopeActivated, SessionScopeId, SessionScopePlugin, SessionScopeRetired,
    SessionScopeSet,
    SessionScopedEntity, SessionSpawnScope, SessionWorldMut, SessionWorldRef,
    SpawnSessionScopedExt, SESSION_SETTLE_FRAMES,
};
pub use spawn_ext::SpawnScopedExt;
