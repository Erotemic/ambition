//! Generic, reusable enemy-WAVE / arena-lockdown system (data-driven, not
//! scripted) — distinct from `crate::boss_encounter`, which is one specific
//! scripted boss fight with hand-authored phases.
//!
//! An "encounter" is a sequence of mob waves with explicit lock / unlock
//! semantics: entering the trigger zone starts it, exits seal until all waves
//! are defeated, player death resets/unlocks, all-defeated → cleared + exits
//! unlock. Any number of encounters coexist via `EncounterRegistry`.
//!
//! Facade module. Authored data, registry resources, event vocabulary, music
//! request resources, reward math, and the headless state machine live in
//! `ambition_encounter`. Gameplay-core keeps the adapters that still touch LDtk,
//! ECS spawning, player/body queries, feature overlays, banners, save/quest
//! plumbing, and schedule sets.

use ambition_platformer_primitives::schedule::SimScheduleExt;
mod events;
mod lifecycle_reexports;
mod loading;
mod lock_walls;
mod music;
mod registry;
mod rewards;
mod spec;
mod switches;
mod systems;

pub use ambition_encounter::{
    active_encounter_camera_zoom, install_encounter_waves, Encounter, EncounterParticipant,
    EncounterParticipants, EncounterRole, EncounterView,
};
pub use events::{EncounterEvent, EncounterEventMsg};
#[cfg(test)]
pub(super) use lifecycle_reexports::ENCOUNTER_INTER_WAVE_DELAY_SECONDS;
pub use lifecycle_reexports::{
    EncounterCommand, EncounterCommandKind, EncounterLifecycle, EncounterLifecycleSet,
    EncounterPhase, EncounterRun, EncounterWaves, WAVES_EXHAUSTED_SIGNAL,
};
pub use loading::load_encounter_specs_from_ldtk;
pub use lock_walls::contribute_encounter_lock_walls;
pub use music::EncounterMusicRequest;
pub use registry::{EncounterRegistry, SwitchActivation};
pub use rewards::{encounter_reward_chest_pos, encounter_reward_looted_flag};
pub use spec::{EncounterMobSpec, EncounterSpec, EncounterWaveSpec, LockWallSpec};
pub use switches::{
    rebuild_encounter_switch_index, EncounterSwitchIndex, SwitchActivated, SwitchActivationQueue,
    SwitchFeature, SwitchOn,
};
pub use systems::{
    apply_encounter_cleanup, apply_wave_encounter_effects, drive_wave_encounters,
    populate_encounter_registry,
};

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
        let sim = app.sim_schedule();
        use bevy::prelude::IntoScheduleConfigs;
        app.add_systems(
            sim,
            (
                crate::world::platforms::sync_moving_platform,
                drive_wave_encounters,
                crate::features::apply_gameplay_banner_requests,
                crate::features::tick_gameplay_banner,
            )
                .chain()
                .in_set(crate::schedule::SandboxSet::EncounterSimulation),
        );
        // The wave EFFECT adapter + the ownership-driven cleanup adapter (E10)
        // react to this frame's lifecycle events, so they run after the
        // generic reducer (`EncounterLifecycleSet`, whose Progression position
        // the runtime owns). Chained: effects still read the participant
        // relations cleanup is about to prune.
        app.add_systems(
            sim,
            (apply_wave_encounter_effects, apply_encounter_cleanup)
                .chain()
                .in_set(crate::schedule::SandboxSet::Progression)
                .after(EncounterLifecycleSet),
        );
        // The lock-wall contribution runs a phase EARLIER, in WorldPrep: it
        // derives the seal walls onto the collision overlay's `gate_solids` from
        // the registry phase, right after the overlay is cleared/rebuilt and
        // before any WorldPrep collision consumer (enemy actor sweeps) — so the
        // walls are present for this frame's collision exactly as the old
        // base-resident blocks were, without mutating the authored base.
        app.add_systems(
            sim,
            contribute_encounter_lock_walls
                .after(crate::features::rebuild_feature_ecs_world_overlay)
                .before(crate::features::update_ecs_hazards)
                .in_set(crate::schedule::SandboxSet::WorldPrep),
        );
    }
}

#[cfg(test)]
mod tests;
