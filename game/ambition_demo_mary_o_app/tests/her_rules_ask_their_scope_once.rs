//! HER RULES ASK THEIR SCOPE ONCE A RUN: each Mary-O rule is in her
//! `RulesScopeSet`, whose one condition (`in_rules_scope`) is asked once a
//! schedule run. A copy of the condition on each system was asked once a
//! system, ~45 times a tick (findings F34, P18).

use ambition_demo_mary_o::MARY_O_MODE;
use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::combat::scoped_rules::RulesScope;
use ambition_platformer2d::platformer::schedule::SimScheduleExt;
use ambition_platformer2d::runtime::RulesScopeSet;
use bevy::ecs::schedule::graph::Direction;
use bevy::ecs::schedule::{NodeId, Schedules, SystemSet};

#[test]
fn her_rules_are_members_of_her_scope_set() {
    let mut app = build_demo_app();
    let sim = app.sim_schedule();
    for _ in 0..30 {
        app.update();
    }
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(sim).expect("the sim schedule").graph();
    // Her scope is her mode where rooms of other games are composed, and
    // every room where she is the one game.
    let members: usize = [RulesScope::Mode(MARY_O_MODE), RulesScope::EveryRoom]
        .into_iter()
        .filter_map(|scope| graph.system_sets.get_key(RulesScopeSet(scope).intern()))
        .map(|key| {
            graph
                .hierarchy()
                .graph()
                .neighbors_directed(NodeId::Set(key), Direction::Outgoing)
                .filter(|node| matches!(node, NodeId::System(_)))
                .count()
        })
        .sum();
    // 38 measured 2026-10-17.
    assert!(members >= 38, "only {members} of her rules are in her scope set: the others ask the scope once each");
}
