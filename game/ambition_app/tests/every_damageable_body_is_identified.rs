//! ⛔⛔ **IS EVERY DAMAGEABLE BODY IDENTIFIED WHERE IT IS BUILT?** (D-DAMAGEABLE-BODY-IDENTITY)
//!
//! The projectile contact protocol calls a missing target identity a
//! CONSTRUCTION failure rather than a sort fallback, so the invariant belongs
//! where bodies are built and the resolver can only report it.
//! `construction/mod.rs` mints `SimId::placement(..)` for enemies, bosses,
//! giants, hands, shrines, riders and summons — **so the placed roads are
//! covered, and what has never been established is whether any damageable body
//! reaches the world without one.**
//!
//! ⚠ **A STATIC SCAN CANNOT ANSWER THIS, and the row says why:** bodies receive
//! `CenteredAabb` and `ActorFaction` from separate inserts, so a bundle-shaped
//! scanner reports 2 of 2 and is describing its own method rather than the tree.
//! ⇒ The instrument is a RUNTIME census over the population the strike road
//! actually queries.
//!
//! ⚠ **AND THE EXISTING CHECK IS A PARTIAL CENSUS.** The projectile resolver's
//! `debug_assert` (`c2188fa7a`) flags only the COINCIDENT pair — two candidates
//! at an identical distance with no identity to break the tie — so **a lone
//! unidentified body passes it in silence.** This counts the population instead
//! of waiting for two of them to collide.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::combat::components::{ActorFaction, CenteredAabb};
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;

/// Frames to let the shipped room finish constructing its cast.
const SETTLE_FRAMES: usize = 120;

/// ⭐⭐ THE ANTI-VACUITY FLOOR, AND IT FIRED ON THE FIRST RUN — which is the
/// reason the fixture below spawns anything at all.
///
/// A census that finds NO damageable bodies reports zero unidentified ones and
/// reads exactly like a healthy tree. **MEASURED 2026-09-10: the sandbox world
/// alone, 120 frames in, holds TWO** — so a census of it would have printed
/// `0 without a SimId` over a population of two and read as a clean bill of
/// health for the whole tree. ⇒ The floor refused the READING rather than the
/// tree, and the answer was to exercise more construction roads rather than to
/// lower it.
const DAMAGEABLE_FLOOR: usize = 4;

/// **Every body the strike road can hit carries a stable identity.**
///
/// ⚠ THE POPULATION IS THE STRIKE ROAD'S OWN. `StrikeVictim` requires exactly
/// `CenteredAabb` + `ActorFaction`; every other field is optional with a defined
/// absence semantic. So this queries those two and nothing narrower — a filter
/// of my own choosing would be a census of my opinion about who can be hit.
#[test]
fn every_body_the_strike_road_can_hit_has_a_stable_identity() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    for _ in 0..SETTLE_FRAMES {
        sim.step(AgentAction::default());
    }

    // ⭐ DRIVE THE ROADS THE ROW NAMES, rather than censusing whatever the
    // sandbox happens to hold. `construction/mod.rs` mints `SimId::placement`
    // for enemies, bosses, giants, hands, shrines, riders and summons; the
    // question is whether a damageable body reaches the world WITHOUT one, and
    // that is only askable of roads something actually travels.
    //
    // ⚠ THIS IS A DELIBERATE POPULATION AND THE TEST SAYS SO. It is not "every
    // body the game can build" — it is the enemy road, the boss road and the
    // sandbox's own cast. A road nobody drives here is not covered, and adding
    // one is how this census grows.
    sim.spawn_enemy_character_at(
        "census_enemy",
        "Census Enemy",
        (240.0, 200.0),
        (22.0, 39.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Custom(
            "pirate_raider".to_string(),
        ),
        "npc_pirate_raider",
    );
    sim.spawn_boss_at(
        "census_boss",
        "Census Boss",
        (320.0, 200.0),
        (60.0, 40.0),
        ambition_platformer2d::entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    for _ in 0..SETTLE_FRAMES {
        sim.step(AgentAction::default());
    }

    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &CenteredAabb, &ActorFaction, Option<&SimId>, Option<&Name>)>();
    let mut total = 0usize;
    let mut unidentified: Vec<String> = Vec::new();
    let mut identities: Vec<String> = Vec::new();
    for (entity, _aabb, _faction, sim_id, name) in q.iter(world) {
        total += 1;
        match sim_id {
            Some(sim_id) => identities.push(sim_id.as_str().to_string()),
            None => unidentified.push(match name {
                Some(name) => format!("{entity} ({name})"),
                None => format!("{entity} (no Name either)"),
            }),
        }
    }

    identities.sort();
    println!(
        "[identity] {total} damageable bodies, {} without a `SimId`; identified: {identities:?}",
        unidentified.len()
    );
    for row in &unidentified {
        println!("[identity]   unidentified: {row}");
    }

    // ⛔⛔ THE PER-ROAD FLOOR, BEFORE THE TOTAL — because a total hides one road
    // collapsing behind the others. If `spawn_boss_at` silently stopped
    // producing a damageable body, the total would still clear its floor on the
    // sandbox's own cast plus the enemy, and this census would report a clean
    // bill of health for a road it no longer travels.
    for road in ["census_enemy", "census_boss"] {
        assert!(
            identities.iter().any(|id| id.contains(road)),
            "the `{road}` construction road produced no damageable body, so this \
             census did not travel it. Identities seen: {identities:?}. ⚠ A TOTAL \
             WOULD HAVE HIDDEN THIS: the sandbox's own cast plus the other spawn \
             clears the floor below without either road being exercised."
        );
    }
    assert!(
        total >= DAMAGEABLE_FLOOR,
        "the census found {total} damageable bodies against a floor of \
         {DAMAGEABLE_FLOOR}. A world with no damageable bodies reports zero \
         unidentified ones and is indistinguishable from a healthy one, so this \
         arm refuses the READING rather than the tree — check that the room \
         still constructs its cast within {SETTLE_FRAMES} frames"
    );
    assert!(
        unidentified.is_empty(),
        "{} of {total} damageable bodies reached the world with no `SimId`: \
         {unidentified:?}. The contact protocol calls a missing target identity \
         a CONSTRUCTION failure rather than a sort fallback — the resolver can \
         only report it, so whatever built these has to mint one. ⚠ The \
         projectile resolver's own `debug_assert` cannot see this: it flags only \
         a COINCIDENT pair, so a lone unidentified body passes it in silence.",
        unidentified.len()
    );
}
