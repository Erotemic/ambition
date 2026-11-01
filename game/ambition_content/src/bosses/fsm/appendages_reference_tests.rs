//! The NATIVE conductor's lesser appendages (test-only reference): noodlings,
//! summoned in a pair to swim at you.

use ambition_platformer2d_core as ae;
use ae::Vec2;
use bevy::prelude::*;

use crate::bosses::hall::Hall;

/// The minion: a small bell with one meatball (`npc_fsm_noodling`).
pub const NOODLING: &str = "npc_fsm_noodling";
const NOODLING_HALF: Vec2 = Vec2::new(20.0, 20.0);
/// Where they appear: out of the god's bell, either side of it.
const SPREAD: f32 = 120.0;

/// Summon a pair of noodlings either side of the god. `serial` makes their ids
/// unique and deterministic (the conductor's tick count).
pub fn summon(effects: &mut MessageWriter<ambition_characters::summon::SummonRequest>, god: Entity, at: Vec2, hall: &Hall, serial: u32) {
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        let x = (at.x + side * SPREAD).clamp(hall.left + NOODLING_HALF.x * 2.0, hall.right - NOODLING_HALF.x * 2.0);
        effects.write(ambition_characters::summon::SummonRequest {
            owner: god,
            spec: ambition_characters::summon::SummonSpec {
                id: format!("fsm_noodling:{serial}:{i}"),
                pos: Vec2::new(x, at.y),
                half_size: NOODLING_HALF,
                character_id: NOODLING.to_string(),
                encounter_id: super::conductor_reference_tests::FSM_ID.to_string(),

                // Shared allegiance prevents the boss from hitting its appendages.
                faction: ambition_characters::actor::ActorFaction::Boss,
                calling: ambition_characters::summon::SummonCalling::Fight,
                ridden_by_summoner: None,
                health: None,
                keeps_contact_damage: true,
                holds: Vec::new(),
            },
        });
    }
}
