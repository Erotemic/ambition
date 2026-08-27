//! Every actor `ambition_content` stages declares whether it sleeps.
//!
//! This is the assembled proof that it is now a decision: `ambition_content::dormancy` states a
//! stance for each, and every actor the real rooms stage carries one.
//!
//! it deliberately does NOT pin WHICH stance — the property is that the
//! choice is stated and findable, not that today's answers are frozen. Mary-O's
//! equivalent test learned that the hard way: it asserted that only the slop
//! declared dormancy, and so spent a day defending the snake's absence.
//!
//! and it also proves the pass is REGISTERED, which a compile cannot
//! catch: `declare_ambition_dormancy` could be perfectly written and never added
//! to a schedule, and everything would still build.

use ambition_platformer2d::actors::features::ecs::dormancy::DormancyPolicy;
use ambition_platformer2d::actors::features::{ActorFaction, FeatureId};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::brain::Brain;
use ambition_platformer2d::platformer::markers::PlayerEntity;
use bevy::prelude::{Or, With, Without};

/// Authored rooms covering every class the survey found: a room full of roaming
/// hostiles, a boss arena that also stages a mount and its two driven hands, the
/// duel exhibition, and the Hall's placed cast.
const ROOMS: [&str; 4] = [
    "basement_enemies",
    "gnu_ton_arena",
    "duel_arena",
    "hall_of_characters",
];

fn undeclared_in(room: &str) -> (usize, Vec<String>) {
    let mut sim = crate::common::fixed_60hz_room_sim(room);
    // A few frames for room staging, the spawn-request applier, and the
    // relation wiring (mount/limb) to materialize.
    for _ in 0..10 {
        sim.step(crate::common::base());
    }
    let world = sim.world_mut();
    // "Has a brain" is the population: an autonomous actor carries `Brain`, and a boss's decisions
    // live on `BossConfig` instead.
    let mut q = world.query_filtered::<
        (Option<&FeatureId>, &ActorFaction, Option<&DormancyPolicy>),
        (Or<(With<Brain>, With<BossConfig>)>, Without<PlayerEntity>),
    >();
    let mut total = 0usize;
    let mut undeclared = Vec::new();
    for (id, faction, policy) in q.iter(world) {
        total += 1;
        if policy.is_none() {
            undeclared.push(format!(
                "{} ({faction:?})",
                id.map(|id| id.as_str().to_string())
                    .unwrap_or_else(|| "<no FeatureId>".to_string())
            ));
        }
    }
    (total, undeclared)
}

#[test]
fn every_actor_ambition_content_stages_declares_whether_it_sleeps() {
    for room in ROOMS {
        let (total, undeclared) = undeclared_in(room);
        println!(
            "{room}: {total} brained actor(s) staged, {} undeclared",
            undeclared.len()
        );
        assert!(
            total > 0,
            "`{room}` authors brained actors; if it stops, this test checks nothing"
        );
        assert!(
            undeclared.is_empty(),
            "these actors staged in `{room}` declare no DormancyPolicy, so nobody \
             has decided whether they think while unobserved: {undeclared:?}"
        );
    }
}
