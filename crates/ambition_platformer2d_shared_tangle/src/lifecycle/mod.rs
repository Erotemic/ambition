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
mod departure;
mod durable_horizon;
pub mod horizon;
mod markers;
mod mode_visit;
mod reset_slots;
mod room_instance;
mod round;
mod session;
mod session_messages;

pub use cleanup::despawn_scoped_entity;
pub use continuity::{
    AuthoredOccurrences, AwayFromAuthoredRoom, OccurrenceBaseline,
    OccurrenceDisposition,
    OccurrenceWhereabouts, RoomOccurrenceOutlook, capture_occurrence_baseline,
    compact_ended_mints_at_checkpoint, project_custody_onto_authored_occurrences,
    restore_occurrence_baseline,
};
pub use custody_horizon::{CustodyBaseline, capture_custody_baseline, live_custody_rows};
pub use departure::{DEPARTURE_GIVE_UP_S, Departure, DepartureState, Destination};
pub use durable_horizon::{DurableHorizonSet, DurableRestoreSet, SaveRestored};
pub use horizon::{
    CheckpointCapture, CheckpointCommitted,
    CheckpointDomainApply, CheckpointRestore, CheckpointRestoreInputs, FreshRunRestore,
    LifecycleCheckpointHorizonPlugin, ResetToCheckpoint,
    adopt_pinned_lifecycle_baselines,
};
pub use markers::{
    BodyCustodySettled, CustodyDurability, FeatureSimEntity, InCustodyOf, LoadingZoneVisual,
    ModeScopedEntity,
    PlayerVisual, PosedBody, PresentationOf, RoomResident, RoomScopedEntity, RoomVisual, custody_closure,
};
pub use mode_visit::{Arrival, ModeVisit};
pub use reset_slots::{ContentDialogueFollowupSet, ContentRoomReplayResetSet, ContentRoomResetSet};
pub use room_instance::{
    InRoomInstance, LiveBodies,
    LiveBodyId, LiveRoomInstance,
    LiveRoomOf, LiveRooms, RoomInstanceRoot, SoleLiveRoom, TransactionRooms, activation_room_root,
    insert_live_room_component, live_room_of, live_room_root_for, sole_live_room_component,
    sole_live_room_component_mut, sole_live_room_entity, spawn_live_room,
};
pub use round::{
    ActiveRoundScope, RoundScopeId, RoundScopePlugin,
    RoundScopedEntity, RoundSpawnScope,
    despawn_departed_round_entities,
};
pub use session::{
    ActiveSessionScope, CandidateSessionRoot, InitialGameplayReadiness, LiveSessionScope,
    SESSION_SETTLE_FRAMES, SessionCommands, SessionGatedSimulation, SessionRoot, SessionRootQuery,
    SessionScopeActivated, SessionScopeId, SessionScopePlugin, SessionScopeRetired,
    SessionScopeSet, SessionScopedEntity, SessionSpawnScope, SessionSpawnVisibility,
    SessionWorldMut, SessionWorldRef, SpawnSessionScopedExt, despawn_retired_session_entities, insert_session_world_component, live_session_scope,
    require_on_session_root,
    session_root_for_scope, session_root_owned_by, session_world_component, session_world_component_mut,
    session_world_component_mut_at,
    session_world_entity,
    session_world_exists, settle_until_controlled_subject, settle_until_session_world,
    simulation_authorized,
};
pub use session_messages::{
    SessionMessageChannels, clear_message_at_session_activation, keep_message_across_session_activation,
};

mod session_time;
pub use session_time::{GameplayElapsedAdvanced, SessionElapsedPlugin};

mod attempt;
pub use attempt::SpawnedThisAttempt;
