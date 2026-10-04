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
use ambition_platformer2d::characters::brain::ActionSet;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::combat::components::{ActorDisposition, ActorTarget};
use ambition_platformer2d::combat::moveset::MeleeSwingQuery;
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
        MeleeSwingQuery,
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
    if melee.swing().is_some() {
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
    // ⛔ SPAWN BY CHARACTER, NOT BY ARCHETYPE. A `Custom(..)` row that no longer
    // exists falls back to a generic `combatant`, so the fixture keeps passing
    // while asserting on a body that is not the one it names.

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

/// A Smash enemy presses its attack from the reach of its own move (Q35).
///
/// The goblin brute's hammer reaches 48.4 px from the centre of its body. No
/// profile authors a hit band, so the brute stops and swings there. The band
/// of a body with no attack move is 36 px: a brute that swings from there
/// reads no move geometry.
///
/// This is the composed witness of `BrainSnapshot::melee_reach`. The real
/// brain tick builds the snapshot from the live `ActorMoveset` of the body.
#[test]
fn a_smash_enemy_swings_from_the_reach_of_its_own_move() {
    use ambition_platformer2d::combat::moveset::ActorMoveset;

    const BRUTE: &str = "reach_view_brute";
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_character_at(
        BRUTE,
        "Goblin Brute",
        (p.x + 140.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("goblin_brute".to_string()),
        "npc_goblin_brute",
    );

    // The brain decides on the positions at the start of a tick, which are
    // those seen after the step before. The press tick also moves the body, so
    // the distance after that step is not the distance the brain decided on.
    let mut decided_at: Option<f32> = None;
    let mut seen_before: Option<f32> = None;
    let mut reach = None;
    for _ in 0..480 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let player = player_pos(world);
        let mut q = world.query::<(&FeatureId, &BodyKinematics, &ActorControl, &ActorMoveset)>();
        let Some((_, kin, control, moveset)) = q.iter(world).find(|(f, ..)| f.as_str() == BRUTE)
        else {
            continue;
        };
        reach = moveset
            .0
            .move_for_verb("attack")
            .map(|spec| spec.frame_data().reach);
        if control.0.melee_pressed {
            decided_at = seen_before;
            break;
        }
        seen_before = Some((kin.pos - player).length());
    }

    let reach = reach.expect("the brute carries an attack move");
    let first_press = decided_at.expect("the brute walks, then presses its attack in 8 s");
    println!("brute reach {reach}, first press decided at {first_press}");
    assert!(
        reach > 40.0,
        "the fixture needs a move that reaches past the 36 px band of a body \
         with no attack move, or the two readings agree: {reach}"
    );
    assert!(
        first_press <= reach + 0.5,
        "the brute pressed at {first_press} px, outside the {reach} px reach of its move"
    );
    assert!(
        first_press > reach - 4.0,
        "the brute closed to {first_press} px before it pressed, and its move \
         reaches {reach} px: it read a band that is not the move geometry"
    );
}
