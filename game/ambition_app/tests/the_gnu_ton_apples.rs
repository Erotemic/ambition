//! GNU-ton's apples gone bad, the few enemies of his gravity works (Jon,
//! 2026-10-30: "The apples need a telegraph and attack behavior"). The Bad
//! Apple glares at the player (its `notice` row), rears back with its maw wide
//! (its `windup` row, standing still), then rolls at the player jaws first,
//! far faster than it walks.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::combat::moveset::MeleeSwingQuery;
use ambition_platformer2d::combat::AttackPhase;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::sprite_sheet::character::PinnedRow;
use bevy::prelude::World;

const APPLE: &str = "bad_apple_under_test";

fn player_pos(world: &mut World) -> ae::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

#[test]
fn the_bad_apple_glares_rears_back_then_rolls_at_you() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_character_at(APPLE, "Bad Apple", (p.x + 160.0, p.y), (20.0, 20.0), CharacterBrain::Custom("bad_apple".to_string()), "bad_apple");
    let mut noticed = None;
    let mut windup_speeds = Vec::new();
    let mut strike_speeds = Vec::new();
    for tick in 0..900 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let mut q = world.query::<(&FeatureId, &BodyKinematics, Option<&PinnedRow>, MeleeSwingQuery)>();
        let Some((_, kin, pin, swing)) = q.iter(world).find(|(id, ..)| id.as_str() == APPLE) else { continue };
        let row = pin.filter(|pin| pin.is_pinned()).and_then(|pin| pin.rows.first().cloned());
        if row.as_deref() == Some("notice") && noticed.is_none() {
            noticed = Some(tick);
        }
        let Some(swing) = swing.swing() else {
            if !strike_speeds.is_empty() {
                break;
            }
            continue;
        };
        match swing.phase() {
            Some(AttackPhase::Startup) => {
                assert_eq!(row.as_deref(), Some("windup"), "its windup row is held over the startup");
                windup_speeds.push(kin.vel.x.abs());
            }
            Some(AttackPhase::Active) => strike_speeds.push(kin.vel.x.abs()),
            _ => {}
        }
    }
    assert!(noticed.is_some(), "it never stood and glared at the player");
    assert!(!windup_speeds.is_empty(), "it never wound up its chomp");
    let still = windup_speeds.iter().cloned().fold(0.0, f32::max);
    let roll = strike_speeds.iter().cloned().fold(0.0, f32::max);
    assert!(still < 40.0, "it stands still through its windup: {still}");
    assert!(roll > 300.0, "it rolls at the player over its strike: {roll}");
}
