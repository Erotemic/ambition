//! The two roads into `gate_solids`, registered in one place.
//!
//! ⛔ THEY WERE CO-LOCATED BY ACCIDENT OF HISTORY, AND THAT MATTERED ONCE. Both
//! systems derive seal walls onto the collision overlay's `gate_solids` in
//! `WorldPrep`, after the feature overlay is rebuilt and before any collision
//! consumer reads it. One is driven by an encounter's phase; the other by an
//! authored condition. Until 2026-09-03 both were registered inside the
//! ENCOUNTER plugin, and the encounter plugin's own comment said why: the
//! authored one *"arrived from `ambition_content`, where being invisible next to
//! its sibling was part of how it went unnoticed that its data lived in Rust"*.
//!
//! ⛔ AND THE PAIR NOW SPANS TWO CRATES (2026-09-03): the encounter road left
//! the actor kernel with `ambition_encounter_features`, while the authored one
//! stayed. Only the RUNTIME can name both, so this plugin lives here — the same
//! reason `ambition_world_items`' plugin is composed here rather than
//! registered back inside the kernel.
//!
//! ⭐ SO THE ADJACENCY IS LOAD-BEARING AND THE HOST WAS NOT. Keeping it inside
//! the encounter plugin meant that plugin scheduled a system belonging to
//! neither encounters nor itself, and a carve that moved the encounter plugin
//! would have separated the two roads — re-creating exactly the condition that
//! hid the defect. A plugin named for the invariant keeps them together for a
//! reason a reader can see, and lets the encounter adapter leave with only its
//! own systems.

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::IntoScheduleConfigs;

/// Schedules every writer of `gate_solids`.
pub struct WorldGatingSchedulePlugin;

impl bevy::prelude::Plugin for WorldGatingSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (
                // Encounter phase → seal walls.
                ambition_encounter_features::contribute_encounter_lock_walls,
                // Authored condition → the same overlay field. Registered beside
                // its sibling ON PURPOSE; see the module doc.
                ambition_platformer2d_actor_monolith::world::gated_lock_walls::sync_authored_gated_lock_walls,
            )
                .after(ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlaySet)
                .before(ambition_combat::hazards::update_ecs_hazards)
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep,
                ),
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::schedule::{NodeId, Schedules, SystemKey, SystemSet as _};
    use bevy::prelude::App;

    use ambition_platformer2d_shared_tangle::schedule::{
        FeatureWorldOverlaySet, Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
    };

    /// Both writers of `gate_solids`, both after the overlay rebuild.
    ///
    /// ⛔ THE INVARIANT IS THE PAIR, not either system. Each derives seal walls
    /// onto an overlay field that `rebuild_feature_ecs_world_overlay` clears
    /// every frame, so a writer that lost its `.after(FeatureWorldOverlaySet)`
    /// edge would write into a list about to be cleared and its walls would
    /// silently stop existing — no panic, no failing test, just collision that
    /// is not there. And a writer that fell out of the plugin entirely would
    /// take its road with it, which is the accident this plugin exists to
    /// prevent.
    ///
    /// ⚠ The existing `gated_lock_walls` tests register that system into their
    /// own app, so they prove the SYSTEM and say nothing about the wiring. This
    /// is the wiring.
    #[test]
    fn both_gate_solids_writers_are_scheduled_after_the_overlay_rebuild() {
        let mut app = App::new();
        app.add_plugins(super::WorldGatingSchedulePlugin);
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules
            .get(sim)
            .expect("the plugin creates the sim schedule");
        let graph = schedule.graph();

        // BY SHAPE, NEVER BY NAME: `system.name()` is the placeholder
        // `<Enable the debug feature to see the name>` unless the build graph
        // happens to unify `bevy_ecs/debug` on, so a name-keyed lookup passes
        // under one `-p` and fails under another. The plugin schedules exactly
        // the pair and nothing else, so "every system it scheduled" IS the pair.
        let systems: Vec<SystemKey> = graph.systems.iter().map(|(key, _, _)| key).collect();
        assert_eq!(
            systems.len(),
            2,
            "WorldGatingSchedulePlugin schedules the two gate_solids writers and nothing else; \
             a writer that fell out took its road with it"
        );

        let overlay_set = graph
            .system_sets
            .get_key(FeatureWorldOverlaySet.intern())
            .expect("FeatureWorldOverlaySet must be a registered SystemSet");
        let world_prep = graph
            .system_sets
            .get_key(Platformer2dSimulationPhaseMonolith::WorldPrep.intern())
            .expect("WorldPrep must be a registered SystemSet");

        for system in systems {
            assert!(
                graph
                    .dependency()
                    .graph()
                    .contains_edge(NodeId::Set(overlay_set), NodeId::System(system)),
                "every gate_solids writer must run AFTER FeatureWorldOverlaySet — it writes \
                 gate_solids, which the overlay rebuild clears every frame"
            );
            assert!(
                graph
                    .hierarchy()
                    .graph()
                    .contains_edge(NodeId::Set(world_prep), NodeId::System(system)),
                "every gate_solids writer must be a member of WorldPrep, where collision \
                 consumers read it"
            );
        }
    }
}
