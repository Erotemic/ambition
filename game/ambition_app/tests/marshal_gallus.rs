//! LEADENSADDLE: Marshal Gallus's fight in the Ore Corral, played headless to
//! his defeat (`docs/planning/game/leadensaddle.md`).
//!
//! The probe strikes him with real hits between his own moves, so his three
//! phases (the duel, the transition, the high noon enrage) play as the game
//! runs them; it does not stand in for a player's skill. What it asks is what
//! the room promises: the way in shuts while he fights, his fall is saved, and
//! the fall opens the grille up to the lookout and the way back.

use std::collections::BTreeSet;

use crate::boss_lifecycle::{boss_cleared, boss_phase, boss_pos, lower_guard};
use crate::common::{base, fixed_60hz_room_options};
use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::boss_encounter::BossEncounterPhase;

const CORRAL: &str = "frontier_ore_corral";
const PLACEMENT: &str = "frontier.marshal_gallus";

/// The gate solids that stand in the live room, by name.
fn standing(sim: &mut Platformer2dSimHarness) -> BTreeSet<String> {
    ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<ambition_platformer2d::world::FeatureEcsWorldOverlay>(
        sim.world_mut(),
    )
    .expect("the live room has a collision overlay")
    .gate_solids
    .iter()
    .map(|solid| solid.name.clone())
    .collect()
}

fn stands(sim: &mut Platformer2dSimHarness, wall: &str) -> bool {
    standing(sim).contains(&format!("gated_lock:{wall}"))
}

/// A real blow of `damage` on the Marshal's body.
fn strike(sim: &mut Platformer2dSimHarness, damage: i32) {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    lower_guard(sim.world_mut(), PLACEMENT);
    let at = boss_pos(sim.world_mut(), PLACEMENT).expect("the Marshal has a body");
    sim.world_mut().write_message(HitEvent {
        origin: None,
        properties: Default::default(),
        contact: None,
        pieces: Vec::new(),
        volume: ambition_platformer2d::engine_core::Aabb::new(
            ambition_platformer2d::engine_core::Vec2::new(at.x, at.y),
            ambition_platformer2d::engine_core::Vec2::splat(64.0),
        )
        .into(),
        damage,
        source: HitSource::Melee,
        attacker: None,
        room: None,
        target: HitTarget::UnresolvedFeatures,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
        strike_sfx: None,
        attacker_move_instance: None,
    });
}

#[test]
fn the_marshal_falls_through_his_three_phases_and_opens_the_lookout() {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(CORRAL))
        .unwrap_or_else(|e| panic!("`{CORRAL}` builds: {e}"));
    sim.step(base());
    assert!(stands(&mut sim, "corral_lookout_grille"), "the grille to the lookout stands before the fight");

    let mut phases: Vec<BossEncounterPhase> = Vec::new();
    let mut open_while_fighting = Vec::new();
    for frame in 0..7200 {
        let phase = boss_phase(sim.world_mut(), PLACEMENT);
        if let Some(phase) = phase {
            if phases.last() != Some(&phase) {
                phases.push(phase);
            }
            let fighting = !matches!(phase, BossEncounterPhase::Dormant | BossEncounterPhase::Death);
            if fighting && !stands(&mut sim, "corral_west_gate") {
                open_while_fighting.push((frame, phase));
            }
            // One blow a second between his own moves, so each phase plays.
            if !phase.boss_invulnerable() && frame % 60 == 0 {
                strike(&mut sim, 5);
            }
        }
        sim.step(base());
        if boss_cleared(&sim, PLACEMENT) {
            break;
        }
    }
    for _ in 0..120 {
        sim.step(base());
    }

    for wanted in [BossEncounterPhase::Phase1, BossEncounterPhase::Phase2, BossEncounterPhase::Enrage, BossEncounterPhase::Death] {
        assert!(phases.contains(&wanted), "the fight never reached {wanted:?}: {phases:?}");
    }
    assert!(boss_cleared(&sim, PLACEMENT), "his fall was not saved (phases {phases:?})");
    assert!(open_while_fighting.is_empty(), "the way in stood open while he fought: {open_while_fighting:?}");
    let standing = standing(&mut sim);
    assert!(!standing.contains("gated_lock:corral_lookout_grille"), "his fall did not open the grille: {standing:?}");
    assert!(!standing.contains("gated_lock:corral_west_gate"), "his fall did not open the way back: {standing:?}");
}
