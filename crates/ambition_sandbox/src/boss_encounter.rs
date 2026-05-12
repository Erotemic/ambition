//! Sandbox-side boss encounter coordinator.
//!
//! Bridges `ae::BossEncounterState` (the phase machine) with the
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

mod damage;
mod events;
mod ids;
mod profile;
mod registry;
mod rewards;
mod specs;
mod systems;

#[cfg(test)]
mod tests;

pub use damage::record_boss_damage;
pub use ids::{encounter_id_from_name, MOCKINGBIRD_ENCOUNTER_ID};
pub use profile::{default_boss_profiles, BossProfile, BossRewardProfile};
pub use registry::BossEncounterRegistry;
pub use rewards::{sync_boss_reward_chests, sync_mockingbird_treasure_chest};
pub use specs::default_boss_specs;
pub use systems::{populate_boss_encounter_registry, update_boss_encounters};
