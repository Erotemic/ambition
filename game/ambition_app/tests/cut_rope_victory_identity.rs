//! Does the cut-rope VICTORY road build a damageable body the strike resolver
//! can name?
//!
//! ⭐⭐ THIS IS ROAD FOUR. `every_damageable_body_is_identified` drives the
//! placement, enemy and boss roads and says so; `body_identity.rs`'s header
//! records "zero unidentified across 120 frame-samples" measured over exactly
//! those three. `boss_lifecycle`'s header names what is left: *"what remains
//! genuinely unpinned here is the victory NPC and the in-place replay"*. ⇒ The
//! observer claims to be exhaustive BY CONSTRUCTION rather than by discipline,
//! and a road it has never seen is the only thing that can test that claim.
//!
//! ⛔ THE ANTI-VACUITY FLOOR IS THE FIRST ASSERTION, NOT A FOOTNOTE. A run where
//! the NPC never spawns reports zero unidentified bodies and reads exactly like a
//! clean one. This asserts the NPC EXISTS before it reads the census, so a
//! green here cannot mean "the road was never travelled".

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::persistence::save::AmbitionGameSave;
use ambition_platformer2d::persistence::save_data::PersistedEncounterState;
use bevy::prelude::*;

const CUT_ROPE_ROOM: &str = "you_have_to_cut_the_rope";

fn cut_rope_sim() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED, not optional: the tolerant road falls back to the authored
        // start room and would run this whole test in the wrong room, green.
        .with_required_start_room(CUT_ROPE_ROOM);
    Platformer2dSimHarness::new_with_options(opts).expect("the cut-rope room builds headlessly")
}

/// Every boss placement id currently in the room.
fn boss_placement_ids(world: &mut World) -> Vec<String> {
    let mut q = world.query::<&BossConfig>();
    q.iter(world).map(|c| c.id.clone()).collect()
}

#[test]
fn the_cut_rope_victory_npc_is_a_damageable_body_with_a_stable_identity() {
    let mut sim = cut_rope_sim();

    // The victory NPC spawns on EITHER a fresh kill or a placement that reads
    // cleared in the save. The save road is the one a test can drive without
    // fighting the whole encounter, and it is a road a PLAYER travels too —
    // it is what re-entering the room after winning does.
    let ids = boss_placement_ids(sim.world_mut());
    assert!(
        !ids.is_empty(),
        "the cut-rope room published no boss placement, so marking one cleared \
         cannot drive the victory road and everything below would be vacuous"
    );
    {
        let world = sim.world_mut();
        let mut save = world.resource_mut::<AmbitionGameSave>();
        for id in &ids {
            save.data_mut()
                .set_boss(id.clone(), PersistedEncounterState::Cleared);
        }
    }

    // The spawn is one system in the encounter-victory slot; a handful of ticks
    // covers the command application and the identity systems behind it.
    sim.step_n(AgentAction::default(), 30);

    // ⛔ FLOOR FIRST. Everything below is about a body that has to exist.
    let npc_count = {
        let world = sim.world_mut();
        let mut q = world.query::<&ambition_content::bosses::SmirkingBehemothVictoryNpc>();
        q.iter(world).count()
    };
    // ⭐ THIS FLOOR IS SELF-VERIFYING, WHICH IS WHY IT CARRIES NO POISON ARM.
    // The usual objection to an unpoisoned assertion is that nobody has seen it
    // fail, so it might be incapable of failing. Here the PASSING direction is
    // the proof: this query returns a COUNT, and the only way to clear the
    // assertion is to have found bodies. A query that named the wrong component,
    // or a run in which the road went untravelled, both return zero and redden
    // it. ⇒ A green here cannot mean "the fixture could not see the NPC"; that
    // possibility and the failure mode are the same observation.
    assert!(
        npc_count > 0,
        "the victory NPC never spawned, so this test measured nothing. Boss \
         placements marked cleared: {ids:?}"
    );

    // ⭐⭐ TWO DIFFERENT QUESTIONS, AND CONFLATING THEM IS THE ROW'S OWN ERROR.
    // The census answers "was it identified AT THE MOMENT it became damageable";
    // this live scan answers "is it identified NOW". `ensure_sim_id` mints from an
    // authored `FeatureId` or the primary-player slot in a LATER system, so a body
    // can fail the first and pass the second. The review's stated consequence —
    // two coincident unidentified victims comparing equal and inheriting query
    // order — needs them to STAY unidentified, which only the second can show.
    let live_unidentified: Vec<String> = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<
            (Entity, Option<&Name>, Option<&ambition_platformer2d::platformer::sim_id::SimId>),
            (
                With<ambition_platformer2d::combat::components::CenteredAabb>,
                With<ambition_platformer2d::combat::components::ActorFaction>,
            ),
        >();
        q.iter(world)
            .filter(|(_, _, id)| id.is_none())
            .map(|(e, name, _)| {
                format!("{e} ({})", name.map(|n| n.as_str()).unwrap_or("<unnamed>"))
            })
            .collect()
    };

    let census = sim
        .world()
        .resource::<ambition_platformer2d::actors::features::BodyIdentityCensus>()
        .clone();
    eprintln!(
        "[road4] observed={} unidentified_at_insertion={} first={:?} still_unidentified_now={:?}",
        census.observed, census.unidentified, census.first_unidentified, live_unidentified
    );
    assert!(
        census.observed > 0,
        "the identity observer saw NO body become damageable in a run that \
         spawned {npc_count} victory NPC(s) — the census is not installed in \
         this composition, and a zero offender count from it means nothing"
    );
    assert!(
        live_unidentified.is_empty(),
        "damageable bodies are STILL unnamed after {} ticks, so the strike \
         resolver would compare them equal and fall back on query order: {:?}",
        30,
        live_unidentified
    );
    // ⛔⛔ NOT `census.unidentified == 0`, AND THAT IS THE POINT OF THIS FILE.
    // The insertion observer fires on the tick the second of
    // `CenteredAabb`/`ActorFaction` lands, which is BEFORE
    // `sim_identity::ensure_sim_id` — the engine's DESIGNED late-mint road for
    // exactly two authored facts, an authored placement's `FeatureId` and the
    // primary player's slot. So the primary player is counted here in every
    // composition, forever, by construction. Measured on this road:
    // `unidentified_at_insertion=1`, and the one was `(Player)`.
    // ⇒ Asserting zero there pins a transient the engine closes on purpose. The
    // invariant the projectile contact protocol actually needs is the one above:
    // nothing is STILL nameless when a consumer could read it.
    assert!(
        census.unidentified <= 1,
        "more than one body was unidentified at insertion. One is expected — the \
         primary player, minted `slot:0` by `ensure_sim_id` a system later. A \
         second means a road that mints identity NEITHER at construction NOR from \
         an authored `FeatureId`, which `ensure_sim_id` cannot rescue. First: {:?}",
        census.first_unidentified
    );
}
