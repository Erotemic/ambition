//! The void's fighters (Jon, 2026-10-30: "make void versions of the polygon
//! fighter characters and have them be enemies to fight in the void. They
//! should behave similarly to their smash kit"). Each is placed hostile in the
//! sandbox room, as an `EnemySpawn` that names it places it, and played
//! headlessly against the real player:
//!
//! - it stands idle while the player is out of its aggro radius;
//! - when the player comes near, it stands a beat and looks (its notice);
//! - then it comes at the player and strikes with a move of its own Smash kit
//!   (a move id of its borrowed table);
//! - the strike costs the player health, a few points and never the pool;
//! - and the player can kill it.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::brain::Brain;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const FIGHTER: &str = "void_fighter_under_test";

/// The three void fighters, and the half size each is placed with.
const VOID_FIGHTERS: [(&str, (f32, f32)); 3] = [
    ("void_pointed_polygon", (16.0, 34.0)),
    ("void_pugnacious_polygon", (16.0, 34.0)),
    ("void_projectile_polygon", (16.0, 34.0)),
];

#[derive(Debug, Clone)]
struct Fighter {
    pos: ae::Vec2,
    hp: i32,
    max_hp: i32,
    noticing: bool,
    playing: Option<String>,
}

fn fighter(sim: &mut Platformer2dSimHarness) -> Option<Fighter> {
    let world = sim.world_mut();
    let mut q = world.query::<(&FeatureId, &ae::BodyKinematics, &BodyHealth, Option<&Brain>, Option<&MovePlayback>)>();
    q.iter(world).find(|(id, ..)| id.as_str() == FIGHTER).map(|(_, kin, health, brain, playback)| Fighter {
        pos: kin.pos,
        hp: health.current(),
        max_hp: health.max(),
        noticing: brain.is_some_and(Brain::is_noticing),
        playing: playback.map(|playback| playback.spec.id.clone()),
    })
}

fn player(sim: &mut Platformer2dSimHarness) -> (ae::Vec2, i32, i32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(&ae::BodyKinematics, &BodyHealth), PrimaryPlayerOnly>();
    let (kin, health) = q.iter(world).next().expect("the player");
    (kin.pos, health.current(), health.max())
}

fn place_player(sim: &mut Platformer2dSimHarness, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q = world
        .query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

/// The sandbox with `character` placed hostile `offset` px to the player's
/// right, settled.
fn a_void_fighter(character: &str, half_size: (f32, f32), offset: f32) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("the sandbox builds");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let (p, ..) = player(&mut sim);
    // `combatant` is what the void's rooms place it with: no `brain_profile`,
    // so its own policy drives it.
    sim.spawn_enemy_character_at(
        FIGHTER,
        character,
        (p.x + offset, p.y - 10.0),
        half_size,
        CharacterBrain::Custom("combatant".to_string()),
        character,
    );
    sim
}

/// Far from the player it stands idle; brought near, it notices, then
/// strikes the player with a move of its own kit, for a few points of health.
#[test]
fn each_void_fighter_notices_the_player_then_strikes_it_with_its_kit() {
    for (character, half_size) in VOID_FIGHTERS {
        let mut sim = a_void_fighter(character, half_size, 260.0);
        let start = fighter(&mut sim).unwrap_or_else(|| panic!("{character}: the fighter was not built"));
        // Out of its reach: the player goes far to its other side.
        let (p, ..) = player(&mut sim);
        let far = ae::Vec2::new(start.pos.x - 900.0, p.y);
        place_player(&mut sim, far);
        for _ in 0..90 {
            sim.step(AgentAction::default());
        }
        let idle = fighter(&mut sim).expect("the fighter");
        let (p_far, ..) = player(&mut sim);
        eprintln!("{character}: idle check: fighter {:?}, player {:?}", idle, p_far);

        // Near: the player stands 220 px to its left.
        place_player(&mut sim, ae::Vec2::new(idle.pos.x - 220.0, p.y));
        let (_, hp_before, hp_max) = player(&mut sim);
        let (mut noticed_at, mut first_move, mut hit_at) = (None, None, None);
        let mut moves: Vec<String> = Vec::new();
        let mut hp_drops: Vec<i32> = Vec::new();
        let mut last_hp = hp_before;
        for tick in 0..900 {
            sim.step(AgentAction::default());
            let f = fighter(&mut sim).expect("the fighter");
            let (_, hp, _) = player(&mut sim);
            if f.noticing && noticed_at.is_none() {
                noticed_at = Some(tick);
            }
            if let Some(id) = &f.playing {
                first_move.get_or_insert(tick);
                if moves.last() != Some(id) {
                    moves.push(id.clone());
                }
            }
            if hp < last_hp {
                hp_drops.push(last_hp - hp);
                hit_at.get_or_insert(tick);
            }
            last_hp = hp;
            if hp_drops.len() >= 3 {
                break;
            }
        }
        eprintln!(
            "{character}: noticed {noticed_at:?}, first move {first_move:?}, first hit {hit_at:?}, \
             moves {moves:?}, drops {hp_drops:?}, hp {hp_before}->{last_hp} of {hp_max}"
        );
        assert!(idle.playing.is_none() && !idle.noticing, "{character}: it acted on a player out of its reach: {idle:?}");
        assert!((idle.pos.x - start.pos.x).abs() < 24.0, "{character}: it moved toward a player out of its reach: {start:?} -> {idle:?}");
        let noticed = noticed_at.unwrap_or_else(|| panic!("{character}: it never noticed the player"));
        let first_move = first_move.unwrap_or_else(|| panic!("{character}: it never played a move"));
        assert!(noticed < first_move, "{character}: it struck ({first_move}) before it noticed ({noticed})");
        let own = format!("{character}_");
        assert!(moves.iter().all(|id| id.starts_with(&own)), "{character}: it played a move not of its kit: {moves:?}");
        assert!(hit_at.is_some(), "{character}: it never hurt the player: {moves:?}");
        assert!(hp_drops.iter().all(|drop| (1..=3).contains(drop)), "{character}: a hit cost more than a smash attack's 3: {hp_drops:?}");
        assert!(last_hp > 0, "{character}: three hits killed the player");
    }
}

/// The player can kill each void fighter: it walks at it and swings until it
/// falls.
#[test]
fn the_player_can_kill_each_void_fighter() {
    for (character, half_size) in VOID_FIGHTERS {
        let mut sim = a_void_fighter(character, half_size, 120.0);
        let start = fighter(&mut sim).expect("the fighter");
        let mut dead_at = None;
        for tick in 0..3600 {
            let f = fighter(&mut sim).expect("the fighter");
            if f.hp <= 0 {
                dead_at = Some(tick);
                break;
            }
            let (p, hp, _) = player(&mut sim);
            assert!(hp > 0, "{character}: the player died first, at tick {tick}: {f:?}");
            let dx = f.pos.x - p.x;
            let move_x = if dx.abs() > 28.0 { dx.signum() } else { dx.signum() * 0.2 };
            let attack = tick % 9 == 0 && dx.abs() < 80.0;
            sim.step(AgentAction { move_x, attack, attack_held: attack, ..Default::default() });
        }
        let (_, hp, hp_max) = player(&mut sim);
        eprintln!("{character}: dead at {dead_at:?}, max hp {}, player {hp} of {hp_max}", start.max_hp);
        assert!(dead_at.is_some(), "{character}: the player could not kill it in a minute: {:?}", fighter(&mut sim));
    }
}

fn entity_of_fighter(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world).find(|(_, id)| id.as_str() == FIGHTER).map(|(e, _)| e).expect("the fighter")
}

fn entity_of_player(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).expect("the player")
}

fn facing_of_fighter(sim: &mut Platformer2dSimHarness) -> f32 {
    let world = sim.world_mut();
    let mut q = world.query::<(&FeatureId, &ae::BodyKinematics)>();
    q.iter(world).find(|(id, _)| id.as_str() == FIGHTER).map(|(_, kin)| kin.facing).expect("the fighter")
}

/// Struck by a foe it cannot see, from far outside its aggro radius and its
/// sight, a void fighter turns to the foe and comes for it: a hit is a
/// notice of its own.
#[test]
fn a_void_fighter_struck_by_a_foe_it_cannot_see_turns_to_it_and_comes() {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    let mut sim = a_void_fighter("void_pugnacious_polygon", (16.0, 34.0), 260.0);
    let start = fighter(&mut sim).expect("the fighter");
    // Behind it, 900 px off: out of its radius and out of its sight.
    let behind = -facing_of_fighter(&mut sim).signum();
    let (p, ..) = player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(start.pos.x + behind * 900.0, p.y));
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }
    let before = fighter(&mut sim).expect("the fighter");
    assert!(before.playing.is_none() && !before.noticing, "premise: it is idle: {before:?}");
    let (shooter, target) = (entity_of_player(&mut sim), entity_of_fighter(&mut sim));
    sim.world_mut().write_message(HitEvent {
        pieces: Vec::new(),
        volume: ae::Aabb::new(before.pos, ae::Vec2::splat(20.0)).into(),
        damage: 1,
        source: HitSource::Projectile,
        attacker: Some(shooter),
        room: None,
        target: HitTarget::Body(target),
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
        strike_sfx: None,
        attacker_move_instance: None,
    });
    let (mut turned, mut came) = (false, 0.0f32);
    for _ in 0..240 {
        sim.step(AgentAction::default());
        let f = fighter(&mut sim).expect("the fighter");
        turned |= facing_of_fighter(&mut sim).signum() == behind;
        came = came.max((f.pos.x - before.pos.x) * behind);
    }
    let after = fighter(&mut sim).expect("the fighter");
    assert!(after.hp < before.hp, "premise: the shot hit it: {before:?} -> {after:?}");
    assert!(turned, "it never turned to the foe that struck it");
    assert!(came > 80.0, "it did not come for the foe that struck it: {came} px");
}

/// In a long fight every blow a void fighter lands costs the player 1 to 3
/// health, whatever the blow: a strike, a throw, a mine, a shot.
#[test]
fn every_blow_a_void_fighter_lands_costs_the_player_a_few_points() {
    use ambition_platformer2d::combat::hitbox::ResolvedBodyHit;
    use bevy::ecs::message::Messages;
    for (character, half_size) in VOID_FIGHTERS {
        let mut sim = a_void_fighter(character, half_size, 200.0);
        let me = entity_of_player(&mut sim);
        let mut cursor = sim.world().resource::<Messages<ResolvedBodyHit>>().get_cursor();
        let mut blows: Vec<(i32, String)> = Vec::new();
        for _ in 0..3600 {
            sim.step(AgentAction::default());
            let playing = fighter(&mut sim).and_then(|f| f.playing).unwrap_or_default();
            let messages = sim.world().resource::<Messages<ResolvedBodyHit>>();
            for hit in cursor.read(messages) {
                if hit.victim == me && hit.damage > 0 {
                    blows.push((hit.damage, playing.clone()));
                }
            }
            // The player is the target, not the subject: it never dies.
            let world = sim.world_mut();
            let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
            let mut health = q.single_mut(world).expect("the player");
            health.health.current = health.health.max;
        }
        eprintln!("{character}: {} blows: {blows:?}", blows.len());
        assert!(!blows.is_empty(), "{character}: it never landed a blow");
        assert!(blows.iter().all(|(damage, _)| (1..=3).contains(damage)), "{character}: {blows:?}");
    }
}
