//! Generic, reusable enemy-WAVE / arena-lockdown system (data-driven, not
//! scripted) — distinct from `ambition_boss_encounter`, which is one specific
//! scripted boss fight with hand-authored phases.
//!
//! An "encounter" is a sequence of mob waves with explicit lock / unlock
//! semantics: entering the trigger zone starts it, exits seal until all waves
//! are defeated, player death resets/unlocks, all-defeated → cleared + exits
//! unlock. Any number of encounters coexist via `EncounterRegistry`.
//!
//! ADAPTER module — it re-exports nothing it does not define. Authored data,
//! registry resources, event vocabulary, music request resources, reward math,
//! and the headless state machine live in `ambition_encounter`, and every
//! consumer names that crate directly (`ambition_platformer2d::encounter` is
//! the same crate under the facade's short name). Gameplay-core keeps only the
//! adapters that still touch LDtk, ECS spawning, player/body queries, feature
//! overlays, banners, save/quest plumbing, switch state, and schedule sets.

use ambition_encounter::{Encounter, EncounterLifecycleSet};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
mod loading;
mod lock_walls;
mod switch_index;
mod systems;

pub use loading::load_encounter_specs_from_rooms;
pub use lock_walls::contribute_encounter_lock_walls;
pub use switch_index::rebuild_encounter_switch_index;
pub use systems::{
    apply_encounter_cleanup, apply_wave_encounter_effects, drive_wave_encounters,
    populate_encounter_registry, WaveEncounterDriven,
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
                // Platform pictures are reconciled by a render family from `MovingPlatformSet`
                // now, in `Update`, not by a sim-schedule system in the actor monolith — see
                // `world::platforms`. IT RUNS ONLY WHERE THERE ARE ENCOUNTERS (, ). A
                // composition with no encounter content therefore PANICKED on boot: every test
                // in `ambition_platformer2d_host`'s `demo_shell_smoke` was red, and had been,
                // because the run's gate is `-p ambition_app` and never built that crate's
                // tests.
                //
                // the alternative was `Option<Res<..>>` on six params, and it
                // is worse: an absent resource would then read as "skip this
                // encounter" INSIDE a game that has encounters, which is the
                // silent-disable this repo has a standing rule against. A run
                // condition says the honest thing — *no encounters, no encounter
                // driver* — and leaves the panic in place for a world that has
                // them and is missing its authorities.
                drive_wave_encounters
                    .in_set(WaveEncounterDriven)
                    .run_if(bevy::ecs::prelude::any_with_component::<Encounter>),
                // The SERVER for the domain's spawn requests, ordered after the
                // driver that emits them so a request and its service are the
                // same tick. It lives in `features` — body construction is the
                // kernel's — and is registered here only until the adapter
                // leaves; nothing about it names the encounter adapter.
                crate::features::serve_encounter_spawn_commands
                    .after(WaveEncounterDriven)
                    .run_if(bevy::ecs::prelude::any_with_component::<Encounter>),
                ambition_combat::banner::apply_gameplay_banner_requests,
                ambition_combat::banner::tick_gameplay_banner,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::EncounterSimulation),
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
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::Progression)
                .after(EncounterLifecycleSet),
        );
        // ⭐ THE TWO `gate_solids` ROADS MOVED OUT (2026-09-03), together, to
        // `world::gating::WorldGatingSchedulePlugin`. They were registered here
        // — including `sync_authored_gated_lock_walls`, which is NOT an
        // encounter system — because being visible beside its sibling is
        // load-bearing: the authored one arrived from `ambition_content`, and
        // being invisible next to this one is how it went unnoticed that its
        // data lived in Rust. Keeping them here meant this plugin scheduled a
        // system belonging to neither encounters nor itself, and a carve that
        // moved this plugin would have split the pair. A plugin named for the
        // invariant holds them now.
    }
}

#[cfg(test)]
mod tests;

/// The request and its service must be the same tick.
#[cfg(test)]
mod spawn_request_service_order {
    use bevy::ecs::schedule::{NodeId, Schedules, SystemSet as _};
    use bevy::prelude::App;

    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

    /// ⛔ `serve_encounter_spawn_commands` must be ordered AFTER
    /// [`super::WaveEncounterDriven`].
    ///
    /// The domain's wave director emits `EncounterEvent::SpawnCommand` onto the
    /// bus; this kernel serves it. If the server were unordered relative to the
    /// driver it would read the requests a tick late — a wave whose mobs arrive
    /// one frame after the wave started — and on an executor that happened to
    /// run it first, every existing test would still pass, because the tests in
    /// this module assert the EVENT is emitted, not that a body was built.
    ///
    /// ⚠ That gap is why this guard is an ordering EDGE and not a smoke test:
    /// nothing else pins the seam between the request and its service.
    #[test]
    fn the_spawn_server_runs_after_the_wave_driver() {
        let mut app = App::new();
        app.add_plugins(super::EncounterSimulationSchedulePlugin);
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules.get(sim).expect("the plugin creates the sim schedule");
        let graph = schedule.graph();

        let driver_set = graph
            .system_sets
            .get_key(super::WaveEncounterDriven.intern())
            .expect("WaveEncounterDriven must be a registered SystemSet");
        let server = {
            let mut found = None;
            for (key, system, _) in graph.systems.iter() {
                let name = format!("{}", system.name());
                if name.rsplit("::").next() == Some("serve_encounter_spawn_commands") {
                    found = Some(key);
                }
            }
            found.expect("serve_encounter_spawn_commands must be scheduled")
        };

        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(driver_set), NodeId::System(server)),
            "serve_encounter_spawn_commands must run AFTER WaveEncounterDriven — \
             the wave director emits SpawnCommand and this kernel serves it, and \
             an unordered server reads the requests a tick late"
        );
    }
}
