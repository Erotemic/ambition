//! The Shock Warden's riot shield, played headlessly against the real player
//! (`ambition_combat::guard`; Jon, 2026-10-19). A strike into the shield from
//! the front is turned aside: it does him no harm, the player is thrown back
//! and locked out, and he answers with a shove of the shield. A strike down
//! on him from over the top of the shield gets through.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::combat::guard::{GuardState, GuardVolumes};
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const WARDEN: &str = "shield_test_warden";

/// The sandbox, with a hostile Shock Warden to the player's right, settled
/// and facing the player.
fn a_warden_beside_you() -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("the sandbox builds");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let (p, ..) = player(&mut sim);
    sim.spawn_enemy_character_at(
        WARDEN,
        "Shock Warden",
        (p.pos.x + 160.0, p.pos.y - 10.0),
        (16.0, 32.0),
        CharacterBrain::Passive,
        "npc_shock_warden",
    );
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }
    sim
}

#[derive(Debug)]
struct Warden {
    kin: ae::BodyKinematics,
    hp: i32,
    guards: Vec<ae::Aabb>,
    state: GuardState,
    bashing: bool,
}

fn warden(sim: &mut Platformer2dSimHarness) -> Warden {
    let world = sim.world_mut();
    let mut q = world.query::<(
        Entity,
        &FeatureId,
        &ae::BodyKinematics,
        &BodyHealth,
        Option<&GuardVolumes>,
        Option<&GuardState>,
        Option<&MovePlayback>,
    )>();
    let (_, _, kin, health, guards, state, playback) =
        q.iter(world).find(|(_, id, ..)| id.as_str() == WARDEN).expect("the warden was built");
    Warden {
        kin: kin.clone(),
        hp: health.current(),
        guards: guards.map_or_else(Vec::new, |guards| guards.pieces.iter().map(|piece| piece.volume).collect()),
        state: state.copied().unwrap_or_default(),
        bashing: playback.is_some_and(|playback| playback.spec.id == "shock_warden_shield_bash"),
    }
}

fn player(sim: &mut Platformer2dSimHarness) -> (ae::BodyKinematics, i32, f32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(&ae::BodyKinematics, &BodyHealth, &BodyCombat), PrimaryPlayerOnly>();
    let (kin, health, combat) = q.iter(world).next().expect("the player");
    (kin.clone(), health.current(), combat.recoil_lock_timer)
}

fn place_player(sim: &mut Platformer2dSimHarness, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q = world
        .query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

/// A strike into his shield from the front does him no harm; it throws the
/// player back and locks it out, and he answers with a shove of the shield
/// that lands.
#[test]
fn a_strike_into_his_shield_is_turned_aside_and_answered() {
    let mut sim = a_warden_beside_you();
    let w = warden(&mut sim);
    let shield = *w.guards.first().unwrap_or_else(|| panic!("his shield is not placed, so nothing here is tested: {w:?}"));
    assert!(w.kin.facing < 0.0, "he faces the player, to his left: {w:?}");
    let warden_hp = w.hp;
    // Stand just in front of his shield, facing it, and swing.
    let (p, hp_before, _) = player(&mut sim);
    let at = ae::Vec2::new(shield.min.x - 22.0, p.pos.y);
    place_player(&mut sim, at);
    sim.step(AgentAction { move_x: 0.2, ..Default::default() });
    sim.step(AgentAction { attack: true, attack_held: true, ..Default::default() });
    let (mut locked, mut bashed, mut hp_after, mut thrown) = (0.0f32, false, hp_before, 0.0f32);
    for frame in 0..80 {
        sim.step(AgentAction::default());
        let (p, hp, lock) = player(&mut sim);
        let w = warden(&mut sim);
        locked = locked.max(lock);
        bashed |= w.bashing;
        if frame < 20 {
            thrown = thrown.max(at.x - p.pos.x);
        }
        if bashed {
            hp_after = hp_after.min(hp);
        }
    }
    let w = warden(&mut sim);
    assert!(w.state.deflects >= 1, "his shield turned nothing aside: {w:?}");
    assert_eq!(w.hp, warden_hp, "the strike hurt him through his shield: {w:?}");
    assert!(locked >= 0.3, "the player was not locked out after the deflect: {locked}");
    assert!(thrown > 12.0, "the player was not thrown back from the shield: {thrown} px");
    assert!(bashed, "he never answered with his shield: {w:?}");
    assert!(hp_after < hp_before, "his shove never landed on the locked-out player ({hp_before} -> {hp_after})");
}

/// From over the top of his shield, a strike down on him gets through.
#[test]
fn a_strike_down_from_over_his_shield_gets_through() {
    let mut sim = a_warden_beside_you();
    let w = warden(&mut sim);
    let shield = *w.guards.first().unwrap_or_else(|| panic!("his shield is not placed: {w:?}"));
    let hp_before = w.hp;
    // Above his head, higher than his shield, falling onto him: a down-air.
    let (p, ..) = player(&mut sim);
    let above = ae::Vec2::new(w.kin.pos.x, shield.min.y - p.size.y * 0.5 - 18.0);
    place_player(&mut sim, above);
    for frame in 0..40 {
        let attack = frame % 8 == 0;
        sim.step(AgentAction { move_y: 1.0, attack, attack_held: attack, ..Default::default() });
    }
    let w = warden(&mut sim);
    assert!(w.hp < hp_before, "a strike down over his shield did not reach him: {w:?}");
    assert_eq!(w.state.deflects, 0, "his shield turned a strike from above: {w:?}");
}
