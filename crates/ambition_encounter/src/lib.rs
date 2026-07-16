//! Generic encounter orchestration: the ONE lifecycle authority, command
//! ingress, objectives, participants, timeline, and wave policy.
//!
//! An encounter is orchestration, not an actor type. Every encounter entity —
//! wave arena, boss wrap, signal-driven puzzle — carries the same generic
//! [`EncounterLifecycle`] driven by [`EncounterCommand`]s and completed/failed
//! by its [`EncounterObjective`] (E8/E9). Host crates adapt it to LDtk, ECS
//! spawning, banners, save/quest plumbing, and renderer/audio side effects.

pub mod entity;
pub mod events;
pub mod lifecycle;
pub mod music;
pub mod objective;
pub mod participants;
pub mod registry;
pub mod rewards;
pub mod spec;
pub mod timeline;
pub mod waves;

pub use entity::{Encounter, EncounterView};
pub use events::{EncounterEvent, EncounterEventMsg};
pub use lifecycle::{
    reduce_encounter_lifecycles, EncounterCommand, EncounterCommandKind, EncounterLifecycle,
    EncounterLifecycleSet, EncounterPhase,
};
pub use music::EncounterMusicRequest;
pub use objective::{objective_met, EncounterObjective, Objective};
pub use participants::{
    EncounterCleanupPolicy, EncounterParticipant, EncounterParticipants, EncounterRole, Ownership,
    SpawnedCleanup,
};
pub use registry::{EncounterRegistry, EncounterRegistryPlugin, SwitchActivation};
pub use rewards::{encounter_reward_chest_pos, encounter_reward_looted_flag};
pub use spec::{
    authored_encounter_waves, install_encounter_waves, EncounterMobSpec, EncounterSpec,
    EncounterWaveSpec, LockWallSpec,
};
pub use timeline::{
    EncounterBeat, EncounterEffect, EncounterGate, EncounterScript, EncounterTrigger,
};
pub use waves::{
    active_encounter_camera_zoom, EncounterRun, EncounterWaves, ENCOUNTER_INTER_WAVE_DELAY_SECONDS,
    WAVES_EXHAUSTED_SIGNAL,
};
