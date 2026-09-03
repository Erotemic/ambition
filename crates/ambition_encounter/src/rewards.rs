//! Encounter reward-chest helpers: `encounter_reward_looted_flag` (the
//! per-encounter save-flag id that remembers a chest was opened across
//! save/load) and `encounter_reward_chest_pos` (where the `EncounterSpec`'s
//! reward chest spawns — centered on the trigger, resting on its floor). The
//! reward payload itself lives in `EncounterSpec::reward` (`spec.rs`).

use ambition_platformer2d_core as ae;

use super::EncounterSpec;

pub fn encounter_reward_looted_flag(encounter_id: &str) -> String {
    format!("encounter_{encounter_id}_reward_dropped")
}

/// Position the reward chest is spawned at, given an encounter spec.
/// Bottom edge of the chest snaps to the trigger AABB's `max.y` (the
/// lower edge in y-down world space, which the LDtk authoring puts
/// on the arena floor).
pub fn encounter_reward_chest_pos(spec: &EncounterSpec, chest_size: ae::Vec2) -> ae::Vec2 {
    use ambition_platformer2d_core::AabbExt;
    let trigger = spec.trigger_aabb();
    ae::Vec2::new(trigger.center().x, trigger.max.y - chest_size.y * 0.5)
}

#[cfg(test)]
mod rewards_tests {
    use super::*;
    use ambition_interaction::PickupKind;

    fn spec_with_trigger(min: [f32; 2], size: [f32; 2]) -> EncounterSpec {
        EncounterSpec {
            id: "test_enc".into(),
            waves: Vec::new(),
            trigger_min: min,
            trigger_size: size,
            camera_zoom: 1.0,
            lock_wall: None,
            intro_seconds: 0.0,
            music_track: String::new(),
            reward: PickupKind::Health { amount: 2 },
        }
    }

    #[test]
    fn looted_flag_is_namespaced_by_encounter_id() {
        assert_eq!(
            encounter_reward_looted_flag("goblin_encounter"),
            "encounter_goblin_encounter_reward_dropped"
        );
        // Distinct encounters get distinct save keys.
        assert_ne!(
            encounter_reward_looted_flag("a"),
            encounter_reward_looted_flag("b")
        );
    }

    #[test]
    fn chest_centers_on_the_trigger_and_rests_on_its_floor() {
        // Trigger spans (100,100)..(300,180); chest is 28x28.
        let spec = spec_with_trigger([100.0, 100.0], [200.0, 80.0]);
        let pos = encounter_reward_chest_pos(&spec, ae::Vec2::new(28.0, 28.0));
        assert_eq!(pos.x, 200.0, "centered on the trigger in x");
        // Chest center sits half its height above the trigger's bottom edge,
        // so its bottom rests on the floor (y-down world space).
        assert_eq!(pos.y, 180.0 - 14.0);
    }
}

/// The cleared encounters, as `(id, spec)` pairs — this domain's published
/// answer to *"which encounters are done and owe a reward?"*.
///
/// ⭐ A FACT THIS CRATE PUBLISHES, NOT A LIST SOMEBODY ASSEMBLES ABOUT IT.
/// The actor kernel's reward-chest sync used to be handed this list by an
/// ADAPTER living in the kernel, which read encounter phase and wave state and
/// pushed the result down. That adapter is the only reason the kernel had to
/// know how an encounter represents "completed"; with the fact published here,
/// the feature layer reads it and the adapter has nothing left to do.
///
/// ⚠ REBUILT EACH TICK rather than accumulated. A cleared encounter that is
/// reset (player death re-opens it) must LEAVE this list, and an accumulating
/// set would keep paying out for an encounter that is no longer cleared —
/// which is the bug the `clear_encounter_reward_ecs` road exists to undo.
#[derive(bevy::prelude::Resource, Default, Clone, Debug)]
pub struct ClearedEncounters(pub Vec<(String, EncounterSpec)>);

/// Republish [`ClearedEncounters`] from live encounter state.
///
/// Runs inside `EncounterLifecycleSet`, immediately after the reducer that
/// settles this tick's phases, so a consumer ordering `.after(EncounterLifecycleSet)`
/// sees a list that agrees with the phases it can observe.
pub fn publish_cleared_encounters(
    mut cleared: bevy::prelude::ResMut<ClearedEncounters>,
    encounters: bevy::prelude::Query<(
        &crate::entity::Encounter,
        &crate::lifecycle::EncounterLifecycle,
        Option<&crate::waves::EncounterWaves>,
    )>,
) {
    cleared.0.clear();
    cleared.0.extend(
        encounters
            .iter()
            .filter(|(_, lifecycle, waves)| {
                matches!(lifecycle.phase, crate::lifecycle::EncounterPhase::Completed)
                    && waves.is_some()
            })
            .filter_map(|(encounter, _, waves)| {
                waves.map(|waves| (encounter.id.clone(), waves.spec.clone()))
            }),
    );
}

#[cfg(test)]
mod published_fact_guard {
    use bevy::ecs::schedule::{NodeId, Schedules, SystemSet as _};
    use bevy::prelude::App;

    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

    /// Every system this plugin schedules must be inside a set it MEANT.
    ///
    /// ⛔ Consumers order `.after(EncounterLifecycleSet)` to read
    /// [`super::ClearedEncounters`], and `.after(SwitchActivationDrained)` to
    /// read `ResolvedSwitchActivations`. A system that fell out of its set would
    /// still run and still fill its resource, and consumers would read LAST
    /// tick's answer — a reward chest one tick late, an encounter re-armed a
    /// frame after the press. Nothing would be red.
    ///
    /// ⚠ ASSERTED BY COUNTING MEMBERS, NOT BY NAMING SYSTEMS. `system.name()`
    /// renders `"<Enable the debug feature to see the name>"` in this crate's
    /// test build — bevy only carries real names with its `debug` feature, which
    /// the actor kernel happens to enable and this crate does not. A name-keyed
    /// lookup passes vacuously here.
    ///
    /// ⭐ THIS GUARD ALREADY EARNED ITS KEEP: it went red the moment the switch
    /// drain was added to this plugin, with the message telling its author to
    /// revisit rather than relax it. The counts below are the revisit — the
    /// drain belongs to a DIFFERENT set on purpose, and saying so is stricter
    /// than the total it replaced.
    #[test]
    fn every_system_this_plugin_schedules_is_inside_the_set_it_meant() {
        let mut app = App::new();
        app.add_plugins(crate::registry::EncounterRegistryPlugin);
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules.get(sim).expect("the plugin creates the sim schedule");
        let graph = schedule.graph();

        let members_of = |set: bevy::ecs::schedule::InternedSystemSet| -> usize {
            let Some(set_key) = graph.system_sets.get_key(set) else {
                panic!("the set must be registered");
            };
            graph
                .systems
                .iter()
                .filter(|(system_key, _, _)| {
                    graph
                        .hierarchy()
                        .graph()
                        .contains_edge(NodeId::Set(set_key), NodeId::System(*system_key))
                })
                .count()
        };

        let scheduled = graph.systems.iter().count();
        let lifecycle = members_of(crate::EncounterLifecycleSet.intern());
        let drained = members_of(crate::switches::SwitchActivationDrained.intern());

        assert_eq!(
            lifecycle, 2,
            "the lifecycle reducer and the cleared-list publisher must BOTH be \
             inside EncounterLifecycleSet — consumers order after it to read a \
             cleared list that agrees with the phases the same tick settled"
        );
        assert_eq!(
            drained, 1,
            "the switch drain must be inside SwitchActivationDrained — it is the \
             ONE drain, and consumers order after that set to react to this \
             tick's presses"
        );
        assert_eq!(
            scheduled,
            lifecycle + drained,
            "every system this plugin schedules must be in one of those sets; a \
             system in neither runs at an arbitrary point and its consumers read \
             stale facts with nothing red"
        );
    }
}
