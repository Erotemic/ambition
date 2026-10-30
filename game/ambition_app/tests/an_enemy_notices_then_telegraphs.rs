//! An enemy reads as Hollow Knight's do (Jon, 2026-10-30: "notice, space,
//! telegraph, attack"): the raptor sees the player and stands a beat drawn in
//! its `notice` row; then, when it swings, its `windup` row is held over the
//! swing's startup, before its attack row plays the strike.
//!
//! The rows are pinned from the brain's notice beat and the swing's phase
//! (`body::pin_telegraph_rows`); a sheet without them draws as before.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::combat::moveset::MeleeSwingQuery;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::sprite_sheet::character::PinnedRow;
use bevy::prelude::World;

const RAPTOR: &str = "noticing_raptor";

fn player_pos(world: &mut World) -> ae::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

/// The row the raptor is pinned to this tick, and whether it is swinging.
fn raptor(world: &mut World) -> Option<(Option<String>, bool)> {
    let mut q = world.query::<(&FeatureId, Option<&PinnedRow>, MeleeSwingQuery)>();
    q.iter(world).find(|(id, ..)| id.as_str() == RAPTOR).map(|(_, pin, swing)| {
        (pin.filter(|pin| pin.is_pinned()).and_then(|pin| pin.rows.first().cloned()), swing.swing().is_some())
    })
}

#[test]
fn the_raptor_notices_the_player_then_winds_up_before_it_strikes() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_character_at(
        RAPTOR,
        "Raptor Stalker",
        (p.x + 150.0, p.y),
        (24.0, 40.0),
        CharacterBrain::Custom("raptor".to_string()),
        "npc_raptor_stalker",
    );
    let mut first_notice = None;
    let mut first_windup = None;
    let mut first_swing = None;
    for tick in 0..600 {
        sim.step(AgentAction::default());
        let Some((row, swinging)) = raptor(sim.world_mut()) else { continue };
        match row.as_deref() {
            Some("notice") if first_notice.is_none() => first_notice = Some(tick),
            Some("windup") if first_windup.is_none() => first_windup = Some(tick),
            _ => {}
        }
        if swinging && first_swing.is_none() {
            first_swing = Some(tick);
        }
        if first_windup.is_some() {
            break;
        }
    }
    let notice = first_notice.expect("the raptor never stood and noticed the player");
    let windup = first_windup.expect("the raptor never held its windup row");
    let swing = first_swing.expect("the raptor never swung");
    assert!(notice < windup, "it notices ({notice}) before it winds up ({windup})");
    assert_eq!(swing, windup, "the windup row is held from the swing's first tick");
}

/// The pachycephalosaurus charges: it stands still through its windup (its
/// windup row held, the telegraph), then rushes at the player head down with
/// its strike live, far faster than it walks.
#[test]
fn the_pachycephalosaurus_stands_through_its_windup_then_charges() {
    const PACHY: &str = "charging_pachy";
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_character_at(
        PACHY,
        "Pachycephalosaurus",
        (p.x + 150.0, p.y),
        (22.0, 36.0),
        CharacterBrain::Custom("pachycephalosaurus".to_string()),
        "pachycephalosaurus",
    );
    let mut windup_speeds = Vec::new();
    let mut strike_speeds = Vec::new();
    for _ in 0..900 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let mut q = world.query::<(&FeatureId, &BodyKinematics, Option<&PinnedRow>, MeleeSwingQuery)>();
        let Some((_, kin, pin, swing)) = q.iter(world).find(|(id, ..)| id.as_str() == PACHY) else { continue };
        let Some(swing) = swing.swing() else {
            if !strike_speeds.is_empty() {
                break;
            }
            continue;
        };
        match swing.phase() {
            Some(ambition_platformer2d::combat::AttackPhase::Startup) => {
                assert!(
                    pin.is_some_and(|pin| pin.rows.first().map(String::as_str) == Some("windup")),
                    "its windup row is held over the startup"
                );
                windup_speeds.push(kin.vel.x.abs());
            }
            Some(ambition_platformer2d::combat::AttackPhase::Active) => strike_speeds.push(kin.vel.x.abs()),
            _ => {}
        }
    }
    assert!(!windup_speeds.is_empty(), "it never wound up a charge");
    let still = windup_speeds.iter().cloned().fold(0.0, f32::max);
    let rush = strike_speeds.iter().cloned().fold(0.0, f32::max);
    assert!(still < 40.0, "it stands still through its windup: {still}");
    assert!(rush > 400.0, "it rushes over its strike: {rush}");
}

/// The raptor pounces: over its strike it leaves the ground at the player,
/// claws first (its attack row is its pounce), and comes down after.
#[test]
fn the_raptor_pounces_off_the_ground_at_its_strike() {
    const POUNCER: &str = "pouncing_raptor";
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_character_at(
        POUNCER,
        "Raptor Stalker",
        (p.x + 150.0, p.y),
        (24.0, 40.0),
        CharacterBrain::Custom("raptor".to_string()),
        "npc_raptor_stalker",
    );
    let mut rise: f32 = 0.0;
    let mut struck = false;
    for _ in 0..900 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let mut q = world.query::<(&FeatureId, &BodyKinematics, MeleeSwingQuery)>();
        let Some((_, kin, swing)) = q.iter(world).find(|(id, ..)| id.as_str() == POUNCER) else { continue };
        if let Some(ambition_platformer2d::combat::AttackPhase::Active) = swing.swing().and_then(|swing| swing.phase()) {
            struck = true;
            rise = rise.max(-kin.vel.y);
        } else if struck {
            break;
        }
    }
    assert!(struck, "the raptor never struck");
    assert!(rise > 250.0, "it leaps up over its strike: {rise}");
}

/// The dilophosaurus telegraphs its spit: before each volley it holds its
/// windup row (coiled, jaws wide) for the volley's windup.
#[test]
fn the_dilophosaurus_coils_before_it_spits() {
    const SPITTER: &str = "spitting_dilophosaurus";
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_character_at(
        SPITTER,
        "Dilophosaurus",
        (p.x + 200.0, p.y),
        (22.0, 37.0),
        CharacterBrain::Custom("dilophosaurus".to_string()),
        "dilophosaurus",
    );
    let mut windup_ticks = 0;
    for _ in 0..600 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let mut q = world.query::<(&FeatureId, Option<&PinnedRow>)>();
        let Some((_, pin)) = q.iter(world).find(|(id, _)| id.as_str() == SPITTER) else { continue };
        if pin.is_some_and(|pin| pin.is_pinned() && pin.rows.first().map(String::as_str) == Some("windup")) {
            windup_ticks += 1;
        }
    }
    // 0.6 s of windup is 36 ticks a volley.
    assert!(windup_ticks >= 30, "it held its windup row before a spit: {windup_ticks} ticks");
}

/// A flyer telegraphs its dive: lined up over the player, the parrot hangs
/// still for its tell (its windup row pinned) before it dives.
#[test]
fn the_parrot_hangs_for_its_tell_before_it_dives() {
    const PARROT: &str = "telling_parrot";
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_character_at(
        PARROT,
        "Stochastic Parrot",
        (p.x + 60.0, p.y - 160.0),
        (18.0, 16.0),
        CharacterBrain::Custom("stochastic_parrot".to_string()),
        "stochastic_parrot",
    );
    let mut tell_speeds = Vec::new();
    for _ in 0..900 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let mut q = world.query::<(&FeatureId, &BodyKinematics, Option<&PinnedRow>)>();
        let Some((_, kin, pin)) = q.iter(world).find(|(id, ..)| id.as_str() == PARROT) else { continue };
        if pin.is_some_and(|pin| pin.is_pinned() && pin.rows.first().map(String::as_str) == Some("windup")) {
            tell_speeds.push(kin.vel.length());
        } else if !tell_speeds.is_empty() {
            break;
        }
    }
    // 0.35 s of tell is 21 ticks.
    assert!(tell_speeds.len() >= 15, "it held its tell before the dive: {} ticks", tell_speeds.len());
    let fastest = tell_speeds.iter().skip(4).cloned().fold(0.0, f32::max);
    assert!(fastest < 60.0, "it hangs nearly still while it tells: {fastest}");
}
