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
mod object_occurrence;
mod object_refs;
mod authored_objects;
pub use authored_objects::{AuthoredObjectIndex, AuthoredLookupError};
mod object_scope;
pub use object_scope::{owned_object_state_id, ObjectScopeOwner, object_scope_room, room_scope_members, include_owned_object_state};
pub use object_refs::{LiveObjects, ObjectIndex, ObjectLookupError, object_reference, resolve_object};
mod menu_pause;
pub use menu_pause::{IslandMenuPause, LocalMenuPauseVotes, MenuPauseHold, MenuPauseRoster, MenuPauseRosterPublished, NetworkMenuPause};
mod room_pace;
pub use room_pace::{RoomFreeze, RoomHolds, RoomPace, rooms_keep_their_own_clocks};
mod room_writes;
pub use room_writes::{
    ROOM_REWIND_HORIZON_TICKS, RoomWrite, RoomWriteRetractors, register_room_write_log, register_room_write_retractor,
    retract_room_writes, take_beyond_the_horizon, take_rewound, take_rewound_superseded,
};
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
    CheckpointCapture, CheckpointCommitted, CheckpointEstablishRequested,
    CheckpointDomainApply, CheckpointRestore, CheckpointRestoreInputs, FreshRunRestore,
    LifecycleCheckpointHorizonPlugin, ResetToCheckpoint, RetainedPoseCapture,
    adopt_pinned_lifecycle_baselines,
};
pub use markers::{
    BodyCustodySettled, CustodyDurability, FeatureSimEntity, InCustodyOf, LoadingZoneVisual,
    ModeScopedEntity,
    PlayerVisual, PosedBody, PresentationOf, RoomResident, RoomScopedEntity, RoomVisual, custody_closure,
};
pub use mode_visit::{Arrival, ModeVisit};
pub use object_occurrence::{BrokenCommitted, CollectedCommitted, DefeatCommitted, ObjectOccurrence, ObjectPublished, PreparedObjectOccurrences, ensure_object_occurrence};
pub(crate) use object_occurrence::ObjectOccurrenceSequence;
pub use reset_slots::{ContentDialogueFollowupSet, ContentRoomReplayResetSet, ContentRoomResetSet};
pub use room_instance::{
    ExecutionScope, ExecutionScopes, InRoomInstance, LiveBodies,
    LiveBodyId, LiveRoomInstance,
    LiveRoomOf, LiveRooms, RoomInstanceRoot, SoleLiveRoom, TransactionRooms, activation_room_root,
    insert_live_room_component, live_room_of, live_room_root, live_room_root_for, sole_live_room_component,
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
    SessionMessageChannels, clear_channels_for_a_rewind, clear_message_at_session_activation,
    register_room_replay_message,
    keep_message_across_session_activation,
};

mod session_time;
pub use session_time::{GameplayElapsedAdvanced, SessionElapsedPlugin};

mod attempt;
pub use attempt::SpawnedThisAttempt;

pub use room_writes::{
    SimulationIsland, SimulationIslands, simulation_island, shares_simulation_island, RoomReplay, RoomReplayRefusals, begin_room_replay, end_room_replay, with_room_replay, register_room_replay_refusal, room_replay_refusal,
};
