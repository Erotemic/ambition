//! Regression: a normal hostile enemy spawned next to the player must ATTACK it.
//!
//! that turn hostile — stopped attacking after a series of unifications: they just
//! stand there. Bosses are fine. This pins the melee chain for a plain
//! `ActorFaction::Enemy`, `hostile_to_player` actor: brain commits melee →
//! `emit_brain_action_messages` resolves the ActionSet → `ActorActionMessage::Melee`
//! → the body's `"attack"` moveset move (`trigger_moveset_moves` →
//! `advance_move_playback`) → the active window spawns a strike. We observe every
//! link so a failure says WHICH one is broken, not just "no attack".

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::actors::features::FeatureId;
use ambition_platformer2d::characters::brain::ActionSet;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::combat::components::{ActorDisposition, ActorTarget};
use ambition_platformer2d::combat::BodyMelee;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::World;

const ENEMY_ID: &str = "test_aggressor";

fn player_pos(world: &mut World) -> ae::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

#[derive(Default, Debug)]
struct EnemyTally {
    present_frames: usize,
    hostile_frames: usize,
    target_some_frames: usize,
    action_set_has_melee: bool,
    melee_pressed_frames: usize,
    swinging_frames: usize,
    min_dist: f32,
}

fn observe(world: &mut World, player: ae::Vec2, t: &mut EnemyTally) {
    let mut q = world.query::<(
        &FeatureId,
        &BodyKinematics,
        &ActorControl,
        &ActorDisposition,
        &ActorTarget,
        &ActionSet,
        &BodyMelee,
    )>();
    let Some((_, kin, control, disp, target, actions, melee)) =
        q.iter(world).find(|(f, ..)| f.as_str() == ENEMY_ID)
    else {
        return;
    };
    t.present_frames += 1;
    if disp.is_hostile() {
        t.hostile_frames += 1;
    }
    if target.entity.is_some() {
        t.target_some_frames += 1;
    }
    t.action_set_has_melee = actions.melee.is_some();
    if control.0.melee_pressed {
        t.melee_pressed_frames += 1;
    }
    if melee.is_swinging() {
        t.swinging_frames += 1;
    }
    let d = (kin.pos - player).length();
    if t.present_frames == 1 || d < t.min_dist {
        t.min_dist = d;
    }
}

#[test]
fn a_hostile_enemy_next_to_the_player_attacks_it() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");

    let p = player_pos(sim.world_mut());
    // This said only

    // `Custom("cellular_automaton_fighter")`, and that archetype row was

    // DELETED when the automaton became a character — so this fixture had

    // been quietly spawning a generic `combatant` and asserting on it.

    sim.spawn_enemy_character_at(
        ENEMY_ID,
        "Perfect Cellular Automaton",
        (p.x + 60.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );

    let mut t = EnemyTally::default();
    // Stand still and let the enemy come to us; ~4s is plenty for an in-range
    // fighter to commit several swings.
    for _ in 0..240 {
        sim.step(AgentAction::default());
        let p = player_pos(sim.world_mut());
        observe(sim.world_mut(), p, &mut t);
    }

    println!("enemy attack tally: {t:#?}");
    assert!(t.present_frames > 100, "enemy should persist: {t:#?}");
    assert!(
        t.hostile_frames == t.present_frames,
        "a hostile_to_player enemy must STAY hostile (not stand down): {t:#?}"
    );
    assert!(
        t.target_some_frames > 0,
        "the enemy must acquire the player as a target: {t:#?}"
    );
    assert!(
        t.action_set_has_melee,
        "the enemy's ActionSet must carry a melee slot: {t:#?}"
    );
    assert!(
        t.melee_pressed_frames > 0,
        "the enemy brain must commit a melee press at least once: {t:#?}"
    );
    assert!(
        t.swinging_frames > 0,
        "the enemy must actually START a melee swing (the reported bug: it never does): {t:#?}"
    );
}
