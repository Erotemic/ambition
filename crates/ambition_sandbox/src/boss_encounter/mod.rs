//! Sandbox-side boss encounter coordinator.
//!
//! Bridges `crate::boss_encounter::BossEncounterState` (the phase machine) with the
//! existing `BossRuntime` (the in-arena physical actor) and the
//! adaptive music + cutscene + save-state systems.
//!
//! `boss_encounter.rs` is intentionally a facade: type ownership,
//! registration, runtime update systems, rewards, event publication, and
//! damage routing live in child modules. This keeps future boss work from
//! piling new behavior into the module entry point.
//!
//! Each `BossSpawn` LDtk entity in the active room maps to one encounter id
//! (defaulting to the boss `name`). When the player enters the room the
//! encounter goes Dormant -> Intro and the cutscene queue is asked to play
//! `boss_intro_<id>`. From that point the engine state machine drives
//! transitions; this module mirrors them onto the seldom_state `BossPhase`
//! component, the audio request, and the save resource.

pub mod attack_geometry;
pub mod behavior;
mod damage;
mod events;
mod ids;
mod profile;
mod registry;
mod rewards;
mod specs;
pub(crate) mod sprites;
mod systems;

mod roster;
#[cfg(test)]
mod tests;

#[allow(unused_imports)] // Future callers of `record_boss_damage` will name the outcome type.
pub use damage::BossDamageOutcome;
pub use damage::{force_boss_death, record_boss_damage};
pub use ids::encounter_id_from_name;
// `MOCKINGBIRD_ENCOUNTER_ID` is no longer re-exported — the dialog
// redirect that read it moved to the data-driven
// `assets/data/dialogue/registry.ron` `BossCleared("mockingbird")`
// rule. Internal tests reference it via `super::ids::MOCKINGBIRD_ENCOUNTER_ID`.
pub use ambition_actor::boss_encounter::{
    BossEncounterEvent, BossEncounterPhase, BossEncounterSpec, BossEncounterState,
};
pub use behavior::{install_boss_profiles, BossProfileRegistry};
pub use profile::{default_boss_profiles, BossProfile, BossRewardProfile};
pub use registry::BossEncounterRegistry;
pub use roster::BossSpecRoster;
pub use specs::default_boss_specs;
pub use systems::{
    boss_phase_transition_feedback, populate_boss_encounter_registry, update_boss_encounters,
};
