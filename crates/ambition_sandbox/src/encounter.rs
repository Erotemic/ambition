//! Reusable encounter / wave system.
//!
//! An "encounter" is a scripted sequence of mob waves with explicit
//! lock / unlock semantics: entering the trigger zone starts the
//! sequence, exits are sealed until all waves are defeated, the
//! player dies → reset / unlock, all enemies defeated → cleared and
//! exits unlock.
//!
//! This module is now a facade. Encounter data types live in the
//! child modules below, and the Bevy/system implementation is split
//! by responsibility so this entry point stays easy to scan.

mod events;
mod loading;
mod lock_walls;
mod music;
mod registry;
mod rewards;
mod spec;
mod state;
mod switches;
mod systems;

pub use events::EncounterEvent;
pub use loading::{load_encounter_specs_from_ldtk, mob_lab_wave_specs};
#[cfg(test)]
use lock_walls::sync_lock_walls;
pub use music::EncounterMusicRequest;
pub use registry::{EncounterController, EncounterRegistry, SwitchActivation};
pub use rewards::{
    clear_encounter_reward, encounter_reward_chest_pos, encounter_reward_looted_flag,
    sync_encounter_reward_chests,
};
pub use spec::{EncounterMobSpec, EncounterSpec, EncounterWaveSpec, LockWallSpec};
pub use state::{EncounterPhase, EncounterRun, EncounterState, ENCOUNTER_INTER_WAVE_DELAY_SECONDS};
pub use switches::{
    encounter_armed_by_switch, rebuild_encounter_switch_index, switch_id_for_encounter,
    EncounterSwitchIndex, SwitchActivationQueue,
};
pub use systems::{
    populate_encounter_registry, sync_encounter_controller_states, update_encounters_from_world,
};

#[cfg(test)]
mod tests;
