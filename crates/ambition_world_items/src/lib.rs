//! The physical life of a collectible in the world.
//!
//! A [`WorldItem`] is a thing you walk into and thereby gain something from —
//! a mushroom, a heart, a ring, a spark-blossom. This crate owns three facts
//! about it and nothing else: that it is SOMEWHERE ([`WorldItem::pos`] and its
//! half-extent), that it may be MOVING ([`ItemMotion`], stepped against the
//! world per axis), and that TOUCHING it collects it
//! ([`collect_world_items`]).
//!
//! ⛔ **WHAT IT DELIBERATELY DOES NOT OWN.** What a collected item MEANS is an
//! `EquipmentRow` recorded on [`WornEquipment`](ambition_characters::equipment::WornEquipment);
//! the verbs that row grants are derived elsewhere by
//! `reconcile_equipment_grants`, which is the one place a body's granted actions
//! come from and stays in the actor kernel. How the item is DRAWN is an art id —
//! an `Option<String>` this crate never resolves — that a game maps through its
//! own `WorldItemArt`. So a collectible's presence, motion and collection are
//! here; its meaning and its picture are not.
//!
//! ⭐ **WHY IT IS ITS OWN CRATE (D33, 2026-09-02).** These modules were
//! `actor_monolith::items::{world_item, item_motion}`, and the reason they could
//! not leave was one type: the collect pass named
//! `features::ecs::pickups::TouchCollectorFilter`, which is composed of nothing
//! but `PlayerEntity` and `TemporaryControl` — both already in `shared_tangle`.
//! Publishing that filter and its value twin `body_collects_on_touch` downward
//! is what freed the rest, the same inversion `ActorDecisionSet` and
//! `AudioInitSet` made before it.
//!
//! ⛔ **AND THE SIBLING STAYED BEHIND, ON PURPOSE.** A `GroundItem` — a held
//! weapon grabbed with a deliberate `Attack` press — lives in the kernel's
//! `items::pickup`, which reaches `abilities`, `ability_cooldown`,
//! `construction` and `shrine`. That file holds 27 of the `items/` module's 51
//! references into the rest of the kernel and is a different, much larger
//! carve. The split here is along the collect TRIGGER (touched vs pressed),
//! which is the line the pickup module's own `AMBITION_REVIEW(discrete_ok)`
//! note had already drawn.

pub mod item_motion;
pub mod world_item;

pub use item_motion::{
    step_item_motion, ItemEmerge, ItemMotion, ItemMotionPlan, DEFAULT_ITEM_GRAVITY,
};
pub use world_item::{
    collect_world_items, spawn_moving_world_item, spawn_world_item, WorldItem, WorldItemPayload,
};

/// Steps moving world items, then collects the ones a body is touching.
///
/// ⛔ **THE ORDER IS LOAD-BEARING AND IS WHY THIS IS ONE PLUGIN RATHER THAN TWO
/// REGISTRATIONS.** A pickup is collected where it IS this tick: step first, so
/// a fast item cannot still be collectable from a box it has already left. The
/// two systems were adjacent in `ItemPickupSimulationPlugin` for exactly this
/// reason, with the rule written between them; moving them out separately would
/// have left that ordering to be re-derived by whoever noticed it was gone.
///
/// ⚠ **BOTH ARE `GameplayGated`**, unchanged from their old home: an item must
/// not drift or be collected while gameplay is suspended.
///
/// ⭐ The host composes this beside `ItemPickupSimulationPlugin`, which is how
/// that one is already added — so no registration for this domain lands in the
/// actor kernel.
pub struct WorldItemSimulationPlugin;

impl bevy::prelude::Plugin for WorldItemSimulationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled;
        use ambition_platformer2d_shared_tangle::schedule::{
            GameplayGated, Platformer2dSimulationPhaseMonolith, SimScheduleExt as _, WorldItemSet,
        };
        use bevy::prelude::IntoScheduleConfigs;

        let sim = app.sim_schedule();

        // ⛔ THE PHASE IS THE POINT, NOT JUST THE CHAIN. `GameplayGated` is the
        // MODE gate and nothing else — its own doc comment says it is
        // deliberately not nested in `GameplaySimulationRoot`. Registering only
        // against it left these systems outside session authorization and
        // outside the phase order, so they could move an item on a tick the
        // simulation never advanced.
        //
        // ⚠ CONFIGURED HERE, not inherited from the kernel's pickup plugin:
        // depending on that plugin to configure our sets first would make this
        // correct only for one plugin insertion order.
        app.configure_sets(
            sim,
            (WorldItemSet::Motion, WorldItemSet::PreCollect, WorldItemSet::Collect)
                .chain()
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)
                .in_set(GameplayGated),
        );
        // Collection reads custody, so it must not run before custody settles;
        // the same edge `ItemPickupSet::CoreHeldItems` carries for the held half.
        app.configure_sets(sim, WorldItemSet::Motion.after(BodyCustodySettled));

        app.add_systems(sim, item_motion::step_item_motion.in_set(WorldItemSet::Motion));
        app.add_systems(sim, world_item::collect_world_items.in_set(WorldItemSet::Collect));
    }
}

/// ⭐⭐ THE PHASE THIS CRATE'S SYSTEMS RUN IN, ASSERTED — NOT THEIR EXISTENCE.
///
/// ⛔ THE GUARD SHAPE IS THE WHOLE POINT AND THE OBVIOUS ONE MISSES. The defect
/// this crate shipped with (`69641a83f`) had BOTH systems present and running
/// every frame: the carve moved the `add_systems` line and the ordering between
/// the two systems, and left behind the `configure_sets` that said they were
/// `in_set(PlayerSimulation)` and `after(BodyCustodySettled)`. It compiled, it
/// ran, a 548-test suite and a workspace gate were green on it. Anything that
/// asked "is the system scheduled" would have passed. So these ask for the
/// HIERARCHY EDGE — the fact that was actually lost.
///
/// ⚠ AND THE SETS ARE CONFIGURED BY THIS CRATE'S OWN PLUGIN, which is what lets
/// this test exist at all. A carved crate that registered into sets some other
/// plugin configures would assert its ordering VACUOUSLY here: the edges would
/// be missing and the test would be measuring plugin insertion order.
#[cfg(test)]
mod simulation_phase_tests {
    use super::WorldItemSimulationPlugin;
    use ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled;
    use ambition_platformer2d_shared_tangle::schedule::{
        GameplayGated, Platformer2dSimulationPhaseMonolith, SimScheduleExt as _, WorldItemSet,
    };
    use bevy::app::App;
    use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemSet};

    /// ⭐ THE PLUGIN ALONE, on a bare `App`. Composing the kernel beside it
    /// would let the kernel's own `configure_sets` supply an edge this crate
    /// failed to declare, and the test would pass on the defect it exists to
    /// catch.
    fn carved_plugin_only() -> App {
        let mut app = App::new();
        app.add_plugins(WorldItemSimulationPlugin);
        app
    }

    /// How many systems are DIRECT members of `set`.
    ///
    /// ⛔⛔ COUNTED, BECAUSE BEVY 0.19 WILL NOT TELL US THEIR NAMES. Without
    /// `bevy_ecs`'s `debug` feature every system reports
    /// `"<Enable the debug feature to see the name>"`, and this crate takes
    /// `bevy` with `default-features = false`. A name-based lookup would work
    /// only when some OTHER crate in the build turned that feature on — the
    /// test would pass under `--workspace` and fail under
    /// `-p ambition_world_items`, which is precisely the composition-dependent
    /// guard D33 warns about. So identity comes from the SHAPE instead, and it
    /// is exact rather than approximate: this plugin adds two systems and no
    /// others are present, so "each set holds exactly one" pins which system is
    /// where with nothing left over.
    fn direct_system_members<S: SystemSet + Copy + std::fmt::Debug>(
        graph: &ScheduleGraph,
        set: S,
    ) -> usize {
        let set_node = set_key(graph, set);
        graph
            .systems
            .iter()
            .filter(|(key, _, _)| {
                graph
                    .hierarchy()
                    .graph()
                    .contains_edge(set_node, NodeId::System(*key))
            })
            .count()
    }

    fn set_key<S: SystemSet + Copy + std::fmt::Debug>(graph: &ScheduleGraph, set: S) -> NodeId {
        NodeId::Set(
            graph
                .system_sets
                .get_key(set.intern())
                .unwrap_or_else(|| panic!("{set:?} must be a registered SystemSet")),
        )
    }

    fn with_graph(f: impl FnOnce(&ScheduleGraph)) {
        let mut app = carved_plugin_only();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        f(schedules.get(sim).expect("the sim schedule exists").graph());
    }

    #[test]
    fn the_carved_systems_are_direct_members_of_this_crates_sets() {
        with_graph(|graph| {
            assert_eq!(
                graph.systems.iter().count(),
                2,
                "this plugin adds exactly the motion step and the touch-collect \
                 pass; the counts below identify which is which only while that \
                 is true",
            );
            assert_eq!(
                direct_system_members(graph, WorldItemSet::Motion),
                1,
                "the motion step must be a direct member of {:?}",
                WorldItemSet::Motion,
            );
            assert_eq!(
                direct_system_members(graph, WorldItemSet::Collect),
                1,
                "the touch-collect pass must be a direct member of {:?}",
                WorldItemSet::Collect,
            );
            // ⭐ AND THE MIDDLE SET IS DELIBERATELY EMPTY HERE. `PreCollect` is
            // vocabulary this crate OWNS and does not use: it is the seam a game
            // hooks to refuse a pickup before the engine claims it (Mary-O's
            // weaker-form rule is the customer). A system appearing here would
            // mean this crate had quietly taken the game's slot.
            assert_eq!(
                direct_system_members(graph, WorldItemSet::PreCollect),
                0,
                "{:?} is the GAME's hook and this crate must leave it empty",
                WorldItemSet::PreCollect,
            );
        });
    }

    #[test]
    fn every_set_this_crate_owns_is_inside_the_player_simulation_phase() {
        // ⛔ THIS IS THE ASSERTION THE CARVE WOULD HAVE FAILED. Losing it meant
        // the systems sat outside session authorization and outside the phase
        // order, free to move an item on a tick the simulation never advanced.
        with_graph(|graph| {
            let phase = set_key(
                graph,
                Platformer2dSimulationPhaseMonolith::PlayerSimulation,
            );
            for set in [
                WorldItemSet::Motion,
                WorldItemSet::PreCollect,
                WorldItemSet::Collect,
            ] {
                assert!(
                    graph
                        .hierarchy()
                        .graph()
                        .contains_edge(phase, set_key(graph, set)),
                    "{set:?} must be inside PlayerSimulation — a set outside the phase runs \
                     on a tick the simulation did not advance",
                );
                // ⚠ AND THE MODE GATE IS NOT A SUBSTITUTE FOR THE PHASE.
                // `GameplayGated` is deliberately NOT nested in the simulation
                // root, so registering against it alone is exactly the defect.
                // Both memberships, or neither is worth asserting.
                assert!(
                    graph
                        .hierarchy()
                        .graph()
                        .contains_edge(set_key(graph, GameplayGated), set_key(graph, set)),
                    "{set:?} must stay gated on gameplay being live",
                );
            }
        });
    }

    #[test]
    fn motion_waits_for_custody_and_the_three_sets_stay_chained() {
        with_graph(|graph| {
            assert!(
                graph.dependency().graph().contains_edge(
                    set_key(graph, BodyCustodySettled),
                    set_key(graph, WorldItemSet::Motion),
                ),
                "collection reads custody, so item motion must not run before custody settles \
                 — the edge `ItemPickupSet::CoreHeldItems` carries for the held half",
            );
            // Step first, so a fast item cannot still be collectable from a box
            // it has already left; the refusal hook sits between them.
            for pair in [
                (WorldItemSet::Motion, WorldItemSet::PreCollect),
                (WorldItemSet::PreCollect, WorldItemSet::Collect),
            ] {
                assert!(
                    graph
                        .dependency()
                        .graph()
                        .contains_edge(set_key(graph, pair.0), set_key(graph, pair.1)),
                    "the edge {:?} -> {:?} must be explicit",
                    pair.0,
                    pair.1,
                );
            }
        });
    }
}
