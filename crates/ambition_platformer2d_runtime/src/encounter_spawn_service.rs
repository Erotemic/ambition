//! The kernel's spawn server, registered where both sides are nameable.

/// Composes the actor kernel's `serve_encounter_spawn_commands`.
///
/// ⭐ THE LAST THING THE ENCOUNTER ADAPTER NAMED IN THIS CRATE. The server lives
/// here because body construction is the kernel's; its registration lived in the
/// encounter plugin only because that plugin was here too. A plugin that moves
/// into `ambition_encounter` cannot name a kernel system, so the registration
/// comes home and the runtime composes it — the same shape as
/// `EncounterRewardSyncPlugin`.
///
/// ⚠ Ordered `.after(WaveEncounterDriven)`: the request and its service are the
/// same tick, and a server that ran first would serve last tick's requests. That
/// set is encounter vocabulary and stays nameable from here after the module
/// moves.
pub struct EncounterSpawnServicePlugin;

impl bevy::prelude::Plugin for EncounterSpawnServicePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;
        use bevy::prelude::IntoScheduleConfigs as _;
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            ambition_platformer2d_actor_monolith::features::serve_encounter_spawn_commands
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::EncounterSimulation,
                )
                .after(ambition_encounter_features::WaveEncounterDriven)
                .run_if(bevy::ecs::prelude::any_with_component::<ambition_encounter::Encounter>),
        );
    }
}

/// The request and its service must be the same tick — and they live in
/// DIFFERENT CRATES, which is why this guard sits here.
///
/// ⛔ The wave director is `ambition_encounter_features`' and the server is this
/// crate's. Only the kernel can name both, so only the kernel can assert the
/// edge between them; a copy on the other side would not compile.
#[cfg(test)]
mod spawn_request_service_order {
    use bevy::ecs::schedule::{NodeId, Schedules, SystemSet as _};
    use bevy::prelude::App;

    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

    /// ⛔ `serve_encounter_spawn_commands` must be ordered AFTER
    /// [`ambition_encounter_features::WaveEncounterDriven`].
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
        // ⭐ BOTH SIDES, because the invariant now spans them. The driver is
        // scheduled by the encounter plugin and the server by the kernel's
        // `EncounterSpawnServicePlugin`, which the runtime composes beside it.
        // A guard that built only one would pass by not finding the other.
        let mut app = App::new();
        app.add_plugins(ambition_encounter_features::EncounterSimulationSchedulePlugin);
        app.add_plugins(crate::encounter_spawn_service::EncounterSpawnServicePlugin);
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules
            .get(sim)
            .expect("the plugin creates the sim schedule");
        let graph = schedule.graph();

        let driver_set = graph
            .system_sets
            .get_key(ambition_encounter_features::WaveEncounterDriven.intern())
            .expect("WaveEncounterDriven must be a registered SystemSet");
        // BY SHAPE, NEVER BY NAME: `system.name()` is a placeholder unless the
        // build graph unifies `bevy_ecs/debug` on, so a name lookup passes
        // under one `-p` and fails under another. The server is the ONE
        // system the service plugin adds, and the driver's set owns none of
        // it: count the systems added by the service plugin alone, then find
        // exactly that many systems ordered after the driver set that are not
        // members of it.
        let service_alone = {
            let mut alone = App::new();
            alone.add_plugins(crate::encounter_spawn_service::EncounterSpawnServicePlugin);
            let sim = alone.sim_schedule();
            let count = alone
                .world()
                .resource::<Schedules>()
                .get(sim)
                .expect("the plugin creates the sim schedule")
                .graph()
                .systems
                .iter()
                .count();
            assert_eq!(
                count, 1,
                "EncounterSpawnServicePlugin schedules the server and nothing else"
            );
            count
        };
        let served_after_the_driver = graph
            .systems
            .iter()
            .map(|(key, _, _)| key)
            .filter(|&key| {
                !graph
                    .hierarchy()
                    .graph()
                    .contains_edge(NodeId::Set(driver_set), NodeId::System(key))
            })
            .filter(|&key| {
                graph
                    .dependency()
                    .graph()
                    .contains_edge(NodeId::Set(driver_set), NodeId::System(key))
            })
            .count();
        assert_eq!(
            served_after_the_driver, service_alone,
            "serve_encounter_spawn_commands must run AFTER WaveEncounterDriven — \
             the wave director emits SpawnCommand and this kernel serves it, and \
             an unordered server reads the requests a tick late"
        );
    }
}
