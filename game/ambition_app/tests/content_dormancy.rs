//! Ambition's hostiles sleep far from every observer, and nothing else it
//! stages does.
//!
//! Which bodies may sleep is the engine's rule
//! (`features::ecs::dormancy::wake_radius`); Ambition states the distance for
//! its own rooms (`ambition_content::dormancy`). This moves every brained body
//! that the real rooms stage far beyond that distance for one tick, then reads
//! `Dormant`. The expected answer is Ambition's authored intent, stated here
//! apart from the engine: a roaming hostile sleeps; a boss, the placed cast, an
//! encounter mob, a mount and a limb do not.
//!
//! It also proves that the rule is DECLARED for Ambition's rooms, which a
//! compile cannot catch: without it, no body sleeps and the hostile half fails.

use ambition_platformer2d::actors::features::ecs::dormancy::Dormant;
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::limb::Limb;
use ambition_platformer2d::characters::brain::Brain;
use ambition_platformer2d::combat::components::{ActorFaction, EncounterMob, FeatureId};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::mount::Mountable;
use ambition_platformer2d::platformer::markers::PlayerEntity;
use bevy::prelude::{Entity, Has, Or, With, Without};

/// Authored rooms covering every class the survey found: a room full of roaming
/// hostiles, a boss arena that also stages a mount and its two driven hands, the
/// duel exhibition, and the Hall's placed cast.
const ROOMS: [&str; 4] = [
    "basement_enemies",
    "gnu_ton_arena",
    "duel_arena",
    "hall_of_characters",
];

/// Far beyond Ambition's wake radius from any observer in these rooms.
const FAR: f32 = 10.0 * ambition_content::dormancy::AMBITION_WAKE_RADIUS;

#[derive(Default)]
struct Verdicts {
    sleepers: usize,
    wakers: usize,
    wrong: Vec<String>,
}

fn judge(room: &str, out: &mut Verdicts) {
    let mut sim = crate::common::fixed_60hz_room_sim(room);
    // A few frames for room staging, the spawn-request applier, and the
    // relation wiring (mount/limb) to materialize.
    for _ in 0..10 {
        sim.step(crate::common::base());
    }
    let world = sim.world_mut();
    // "Has a brain" is the population: an autonomous actor carries `Brain`, and
    // a boss's decisions live on `BossConfig` instead.
    let mut q = world.query_filtered::<
        (
            Entity,
            Option<&FeatureId>,
            &ActorFaction,
            Has<EncounterMob>,
            Has<Mountable>,
            Has<Limb>,
            &mut BodyKinematics,
        ),
        (Or<(With<Brain>, With<BossConfig>)>, Without<PlayerEntity>),
    >();
    let mut staged = Vec::new();
    for (entity, id, faction, is_mob, is_mount, is_limb, mut body) in q.iter_mut(world) {
        body.pos.x += FAR;
        let sleeps = *faction == ActorFaction::Enemy && !is_mob && !is_mount && !is_limb;
        let name = format!(
            "{room}/{} ({faction:?})",
            id.map(|id| id.as_str().to_string())
                .unwrap_or_else(|| "<no FeatureId>".to_string())
        );
        staged.push((entity, name, sleeps));
    }
    assert!(
        !staged.is_empty(),
        "`{room}` authors brained actors; if it stops, this test checks nothing"
    );

    sim.step(crate::common::base());
    for (entity, name, sleeps) in staged {
        let Some(dormant) = sim.world().get_entity(entity).ok().map(|e| e.contains::<Dormant>())
        else {
            out.wrong.push(format!("{name} left the world"));
            continue;
        };
        if sleeps {
            out.sleepers += 1;
        } else {
            out.wakers += 1;
        }
        if dormant != sleeps {
            out.wrong.push(format!("{name}: dormant={dormant}, expected {sleeps}"));
        }
    }
}

#[test]
fn only_a_roaming_hostile_that_ambition_stages_sleeps_far_from_every_observer() {
    let mut out = Verdicts::default();
    for room in ROOMS {
        judge(room, &mut out);
    }
    println!("{} hostiles asleep, {} others awake", out.sleepers, out.wakers);
    assert!(
        out.sleepers > 0 && out.wakers > 0,
        "the rooms must stage both roaming hostiles and bodies that never sleep, \
         or one half of this test checks nothing ({} / {})",
        out.sleepers,
        out.wakers
    );
    assert!(
        out.wrong.is_empty(),
        "these bodies, moved far from every observer, did not follow Ambition's \
         dormancy intent: {:?}",
        out.wrong
    );
}
