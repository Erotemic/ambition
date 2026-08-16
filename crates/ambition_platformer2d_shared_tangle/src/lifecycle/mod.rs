//! Lifecycle vocabulary for entities spawned by reusable platformer systems.
//!
//! The public API is the helper verb (`spawn_room_scoped`, `spawn_mode_scoped`,
//! and `SessionSpawnScope`/`RoundSpawnScope`'s `apply_to`) rather than the marker
//! component convention. Marker components remain public because existing
//! cleanup queries and tests need to name them, but new spawn sites should
//! prefer [`SpawnScopedExt`].
//!
//! **The scopes nest: round ⊂ session, and room and mode cut across both.** Each
//! one names a boundary and owns the sweep that culls at it —
//! [`RoomScopedEntity`] (room unload / sandbox reset), [`ModeScopedEntity`]
//! (`despawn_departed_mode_entities`), [`RoundScopedEntity`]
//! (`despawn_departed_round_entities`), [`SessionScopedEntity`]
//! (`despawn_retired_session_entities`).
//!
//! ⭐ **a scope is a LIFETIME; where an entity lives right now is RESIDENCY, and
//! they are not the same question.** An object in a body's custody is scoped to
//! a room and resident in nobody's room — see [`InCustodyOf`] and the
//! [`RoomResident`] roster a room CHANGE retires.
//!
//! ⭐ **and there is a THIRD question, which is what a rebuild asks: WHERE is
//! the occurrence this authored record minted last time?** That is a
//! WHEREABOUTS, it is durable room state rather than a component on anything,
//! and it lives in [`AuthoredOccurrences`]. A scope says when an occurrence
//! dies, residency says whose sweep sees it, and a whereabouts says whether
//! reconstruction owes the world a new one — and, if it does, WHERE.
//!
//! ⚠ **the whereabouts ledger owns exactly one of three horizons** (current /
//! checkpoint baseline / durable save); its module header states which and what
//! the other two would need.
//!
//! ⛔ **there is no marker for "persistent", and that is the design.** Every
//! sweep culls on the PRESENCE of its own marker, so an entity carrying none
//! already survives all four boundaries; a `PersistentEntity` tag beside a
//! `RoomScopedEntity` would have been a claim the room sweep silently overrules.
//! Spelling it was not free — the `markers` module records what the two
//! unenforced spellings cost.

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
    CheckpointCapture, CheckpointCommitted, CheckpointRestore, ResetToCheckpoint,
};
pub use markers::{
    FeatureSimEntity, InCustodyOf, LoadingZoneVisual, ModeScopedEntity, PlayerVisual, RoomResident,
    RoomScopedEntity, RoomVisual,
};
pub use round::{
    despawn_departed_round_entities, ActiveRoundScope, RoundScopeId, RoundScopePlugin,
    RoundScopedEntity, RoundSpawnScope,
};
pub use session::{
    despawn_retired_session_entities, insert_session_world_component, session_world_component,
    session_world_component_mut, session_world_entity, session_world_exists,
    settle_until_controlled_subject, settle_until_session_world, simulation_authorized,
    ActiveSessionScope, InitialGameplayReadiness, SessionCommands, SessionGatedSimulation,
    SessionRoot, SessionScopeId, SessionScopePlugin, SessionScopeRetired, SessionScopeSet,
    SessionScopedEntity, SessionSpawnScope, SessionWorldMut, SessionWorldRef,
    SpawnSessionScopedExt, SESSION_SETTLE_FRAMES,
};
pub use spawn_ext::SpawnScopedExt;
