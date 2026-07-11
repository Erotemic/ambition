//! Generic encounter wave/lockdown vocabulary and headless state machine.
//!
//! This crate owns the authored data, live phase machine, registry resources,
//! music request resources, switch activation payload, and reward math. Host
//! crates adapt it to LDtk, ECS spawning, banners, save/quest plumbing, and
//! renderer/audio side effects.

pub mod entity;
pub mod events;
pub mod music;
pub mod registry;
pub mod rewards;
pub mod spec;
pub mod state;

pub use entity::{Encounter, EncounterView};
pub use events::EncounterEvent;
pub use music::EncounterMusicRequest;
pub use registry::{EncounterRegistry, SwitchActivation};
pub use rewards::{encounter_reward_chest_pos, encounter_reward_looted_flag};
pub use spec::{
    authored_encounter_waves, install_encounter_waves, EncounterMobSpec, EncounterSpec,
    EncounterWaveSpec, LockWallSpec,
};
pub use state::{
    active_encounter_camera_zoom, EncounterPhase, EncounterRun, EncounterState,
    ENCOUNTER_INTER_WAVE_DELAY_SECONDS,
};
