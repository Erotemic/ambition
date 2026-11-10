//! BOSS-CINEMATIC-COVERAGE: a boss that does not stage its own name card gets
//! the encounter's. Each boss here is woken in its real room, and the card is
//! read where the renderer reads it (`BossRenderIndex`): it comes up after the
//! wake and goes again. Wolfenstein's and the Inugami's conducted cards are
//! `wolfenstein_fight` and `inugami_fight`'s subject.

#![cfg(feature = "rl_sim")]

use std::collections::BTreeMap;

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;

/// Rooms whose bosses have no conductor that stages a card: the Clockwork
/// Warden has no conductor; the T-rex and the Flying Spaghetti Monster have
/// conductors that stage nothing.
const ROOMS: [&str; 3] = ["basement_boss", "trex_arena", "flying_spaghetti_monster_arena"];

fn untouchable_player(sim: &mut Platformer2dSimHarness) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
    }
}

/// Stand the player a little in front of the first boss, so a boss that wakes
/// to a nearby player wakes.
fn stand_by_a_boss(sim: &mut Platformer2dSimHarness) {
    let world = sim.world_mut();
    let at = world
        .query::<(&BossConfig, &ae::BodyKinematics)>()
        .iter(world)
        .map(|(_, kin)| kin.pos)
        .next()
        .expect("the room places a boss");
    sim.teleport_player((at.x - 160.0, at.y));
}

/// Each boss's card on this frame, by id.
fn cards(sim: &Platformer2dSimHarness) -> BTreeMap<String, f32> {
    sim.world()
        .get_resource::<ambition_platformer2d::sim_view::BossRenderIndex>()
        .expect("the sim publishes the boss render index")
        .iter()
        .map(|(id, view)| (id.to_string(), view.stage.card))
        .collect()
}

/// Every boss in each room shows its name card in full after it wakes, and
/// the card is gone again once its beat is over.
#[test]
fn each_boss_shows_its_name_card_after_it_wakes_and_puts_it_away() {
    for room in ROOMS {
        let opts = Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(room);
        let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the boss room builds headlessly");
        untouchable_player(&mut sim);
        stand_by_a_boss(&mut sim);
        let mut fullest: BTreeMap<String, f32> = BTreeMap::new();
        // Its beat ends 3.45 s after it wakes; a boss may wait for the player
        // to come near, so allow for that.
        for _ in 0..60 * 7 {
            sim.step(AgentAction::default());
            for (id, card) in cards(&sim) {
                let full = fullest.entry(id).or_default();
                *full = full.max(card);
            }
        }
        assert!(!fullest.is_empty(), "{room}: no boss in the render index");
        for (id, full) in &fullest {
            assert!(*full > 0.99, "{room}: {id}'s name card came up to {full}, not all the way");
        }
        for (id, card) in cards(&sim) {
            assert!(card < 0.01, "{room}: {id}'s name card is still up ({card}) seven seconds in");
        }
    }
}
