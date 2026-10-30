//! Castle Godwin's variant enemies, played headlessly against the real player
//! (Jon, 2026-10-30: "notice, space, telegraph, attack"): the Crawling
//! Conscript, a Risen Conscript with no legs, is built low and wide on the
//! floor, and it lunges and bites; the Fascist Swordsman thrusts and the
//! Fascist Clubman brings his club down. Each is placed in the sandbox beside
//! the player as its catalog row builds it, and fights as the castle's
//! placements make it fight (`brain: "combatant"`).

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const FOE: &str = "castle_variant_under_test";

fn player(sim: &mut Platformer2dSimHarness) -> (ae::BodyKinematics, i32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(&ae::BodyKinematics, &BodyHealth), PrimaryPlayerOnly>();
    let (kin, health) = q.iter(world).next().expect("the player");
    (kin.clone(), health.current())
}

#[derive(Debug, Default)]
struct Fight {
    /// The foe's body as it was built, after it settled.
    size: ae::Vec2,
    grounded: bool,
    /// The moves it played, by id, in order (each once per run of it).
    moves: Vec<String>,
    hp_before: i32,
    hp_after: i32,
}

/// The sandbox, with `character` placed `gap` px to the player's right as a
/// castle placement places it; the player stands still for `seconds`.
fn fight(character: &str, gap: f32, seconds: usize) -> Fight {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("the sandbox builds");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let (p, hp_before) = player(&mut sim);
    sim.spawn_enemy_character_at(
        FOE,
        character,
        (p.pos.x + gap, p.pos.y - 10.0),
        (16.0, 16.0),
        CharacterBrain::Custom("combatant".to_string()),
        character,
    );
    let mut out = Fight { hp_before, hp_after: hp_before, ..Default::default() };
    for frame in 0..60 * seconds {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let mut q = world.query::<(&FeatureId, &ae::BodyKinematics, &ae::BodyGroundState, Option<&MovePlayback>)>();
        let (_, kin, ground, playback) = q.iter(world).find(|(id, ..)| id.as_str() == FOE).expect("the foe was built");
        if frame == 40 {
            out.size = kin.size;
            out.grounded = ground.on_ground;
        }
        let playing = playback.map(|playback| playback.spec.id.to_string());
        if let Some(id) = playing {
            if out.moves.last() != Some(&id) {
                out.moves.push(id);
            }
        }
        out.hp_after = out.hp_after.min(player(&mut sim).1);
    }
    out
}

/// The Crawling Conscript is low and wide, on the floor: its body is its
/// body as it lies, not a standing man's. It comes for the player, lunges
/// and bites, and the bite hurts.
#[test]
fn the_crawling_conscript_lies_low_and_lunges() {
    let f = fight("npc_evil_zombie_crawler", 110.0, 12);
    assert!(f.grounded, "it is not on the floor: {f:?}");
    assert!(f.size.x > f.size.y * 1.4, "it is not low and wide: {f:?}");
    assert!(f.size.y < 40.0, "it stands as tall as a man: {f:?}");
    assert!(!f.moves.is_empty(), "it never attacked: {f:?}");
    assert!(f.hp_after < f.hp_before, "its lunge never landed ({} -> {}): {f:?}", f.hp_before, f.hp_after);
}

/// The Fascist Swordsman and the Fascist Clubman are built as men of the
/// enforcer's height, and each one's own swing lands on the player.
#[test]
fn the_armed_fascists_strike_with_their_weapons() {
    for character in ["npc_fascist_swordsman", "npc_fascist_clubman"] {
        let f = fight(character, 140.0, 10);
        assert!(f.grounded, "{character} is not on the floor: {f:?}");
        assert!(f.size.y > f.size.x * 1.5, "{character} is not built standing: {f:?}");
        assert!(!f.moves.is_empty(), "{character} never attacked: {f:?}");
        assert!(f.hp_after < f.hp_before, "{character}'s swing never landed ({} -> {}): {f:?}", f.hp_before, f.hp_after);
    }
}
