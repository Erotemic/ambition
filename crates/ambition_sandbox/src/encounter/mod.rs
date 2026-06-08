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
pub use loading::load_encounter_specs_from_ldtk;
#[cfg(test)]
use lock_walls::sync_lock_walls;
pub use music::{BossEncounterMusicRequest, EncounterMusicRequest};
pub use registry::{EncounterRegistry, SwitchActivation};
pub use rewards::{encounter_reward_chest_pos, encounter_reward_looted_flag};
pub use spec::{EncounterMobSpec, EncounterSpec, EncounterWaveSpec, LockWallSpec};
#[cfg(test)]
pub(super) use state::ENCOUNTER_INTER_WAVE_DELAY_SECONDS;
pub use state::{EncounterPhase, EncounterRun, EncounterState};
pub use switches::{rebuild_encounter_switch_index, EncounterSwitchIndex, SwitchActivationQueue};
pub use systems::{populate_encounter_registry, update_encounters_from_world};

/// Module-local Bevy plugin: schedules the `EncounterSimulation`
/// simulation set — moving-platform sweep + encounter tick +
/// gameplay-banner queue drain.
///
/// Carved out of
/// `app/plugins.rs::register_encounter_simulation_systems` per
/// OVERNIGHT-TODO #6. Three different domains (platforms, encounter,
/// features) participate; encounter is the central one (it owns
/// `update_encounters_from_world`), so this plugin lives here.
pub struct EncounterSimulationSchedulePlugin;

impl bevy::prelude::Plugin for EncounterSimulationSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        app.add_systems(
            Update,
            (
                crate::world::platforms::sync_moving_platform,
                update_encounters_from_world,
                crate::features::apply_gameplay_banner_requests,
                crate::features::tick_gameplay_banner,
            )
                .chain()
                .in_set(crate::app::SandboxSet::EncounterSimulation),
        );
    }
}

#[cfg(test)]
mod tests;
