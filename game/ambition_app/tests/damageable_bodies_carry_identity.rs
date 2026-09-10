//! D-DAMAGEABLE-BODY-IDENTITY — does a body the resolver can strike reach the
//! world without a stable identity?
//!
//! ⭐⭐ **THE POPULATION IS `StrikeVictim`'S OWN QUERY, NOT "things that look like
//! actors".** That query is matched by any entity carrying BOTH `CenteredAabb`
//! and `ActorFaction` — its two non-`Option`, non-`Has` component fields — so
//! that pair, and nothing else, is what makes a body a candidate victim. Its
//! `sim_id` field is `Option<&SimId>` and documents the rule: *"a body without
//! one still gets hit, it just cannot win the tie."* A body with no identity is
//! therefore legal, and every geometric tie it takes part in is decided by
//! something that does not survive a rewind.
//!
//! ⛔⛔ **A STATIC SCAN CANNOT ANSWER THIS, AND ONE ALREADY TRIED.** Bodies
//! receive `CenteredAabb` and `ActorFaction` from SEPARATE inserts — a bundle,
//! a construction road, a later `insert` — so a scanner keyed on "one spawn
//! call naming both" reports 2 sites out of 2 and is describing its own method
//! rather than the tree. That scanner was written on 2026-09-10 and deleted the
//! same hour. The instrument has to be the running world.
//!
//! ⚠ **AND THE `debug_assert` IN THE PROJECTILE RESOLVER IS ONLY A PARTIAL
//! CENSUS.** It fires when TWO coincident victims are both unidentified, so a
//! LONE unidentified body passes it in silence forever. It answers "is any tie
//! already undecidable", not "is every body identified", and only the second
//! question makes the first one permanently answered.
//!
//! ⛔⛔ **AND THE POPULATION IT REACHES IS THIN — THREE BODIES. MEASURED, not
//! assumed: the sandbox room plus the staged mob.** So this is a REGRESSION
//! guard over a small, real population, and it is NOT a clean bill of health for
//! the tree. A body type no arm here reaches is not counted, and most of the
//! shipped roster is not reached.
//!
//! ⚠ **THE DEFECT THIS ROW WAS OPENED FOR IS NOT VISIBLE FROM HERE.**
//! `game/ambition_content/src/bosses/cut_rope/victory.rs` spawns a fully
//! damageable post-boss NPC — `CenteredAabb`, `ActorFaction::Npc`,
//! `DamageableVolumes`, a brain — with a bare `commands.spawn`, and `SimId`
//! appears nowhere in that file. It is post-boss content this sandbox never
//! reaches, so this test is green WITH that body in the tree. ⇒ The stronger
//! instrument for this row today is the static one,
//! `scripts/measure_damageable_bundles_without_identity.py`; this is the arm
//! that stops the roads we DO exercise from regressing.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::combat::components::{ActorFaction, CenteredAabb, FeatureId};
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::{Entity, Name, World};

/// Every entity the strike resolver would consider a victim, and whether it
/// carries a stable identity.
fn census(world: &mut World) -> (Vec<String>, usize) {
    let mut q = world.query::<(
        Entity,
        &CenteredAabb,
        &ActorFaction,
        Option<&SimId>,
        Option<&Name>,
        Option<&FeatureId>,
    )>();
    let mut unidentified = Vec::new();
    let mut total = 0usize;
    for (entity, _, _, sim, name, feature) in q.iter(world) {
        total += 1;
        if sim.is_none() {
            let who = name
                .map(|n| n.as_str().to_string())
                .or_else(|| feature.map(|f| f.as_str().to_string()))
                .unwrap_or_else(|| format!("{entity:?}"));
            unidentified.push(who);
        }
    }
    unidentified.sort();
    (unidentified, total)
}

/// ⛔ EVERY DAMAGEABLE BODY THESE ROADS BUILD CARRIES A `SimId`, AND THE COUNT IS
/// A RATCHET.
///
/// ⭐ THE FLOOR IS NOT DECORATION. A census that finds nothing is
/// indistinguishable from a census that looked at nothing: if the sandbox
/// composed no damageable body at all, "zero unidentified" would be trivially
/// true and this test would pass forever while measuring an empty world. So the
/// total is asserted non-trivial first.
#[test]
fn every_damageable_body_these_roads_build_carries_a_stable_identity() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }

    // A body through the REAL construction road — `stage_actor` is the seam the
    // shipped spawner uses, so this arm measures production and not a fixture's
    // idea of a body.
    sim.spawn_enemy_character_at(
        "identity_census_mob",
        "Perfect Cellular Automaton",
        (600.0, 300.0),
        (14.0, 23.0),
        CharacterBrain::Passive,
        "perfect_cellular_automaton",
    );
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }

    let (unidentified, total) = census(sim.world_mut());
    // ⭐ THREE IS THE MEASURED POPULATION, and pinning it rather than a token `>= 1`
    // is what keeps the census honest in the other direction too: if the sandbox
    // ever stops composing a damageable body, this reports that instead of
    // quietly reporting "all identified" over nothing.
    assert!(
        total >= 3,
        "the census saw {total} damageable bodies and the measured population of \
         these roads is 3 — the sandbox room's own plus the staged mob. Fewer \
         means this fixture is measuring a thinner world than it was written \
         against, and 'all identified' over an empty set is not a result"
    );
    assert!(
        unidentified.is_empty(),
        "{} of {total} damageable bodies reached the world with no `SimId`: {:?}. \
         `StrikeVictim.sim_id` is optional by design — such a body still gets hit \
         — but it cannot win a geometric tie, so every tie it takes part in is \
         resolved by Bevy query order, which a rollback resimulation does not \
         promise to reproduce. Identity belongs where the body is BUILT",
        unidentified.len(),
        unidentified,
    );
}

/// ⛔⛔ **THE ROLLBACK LANE, WHERE AN UNIDENTIFIED BODY COSTS THE MOST.** A smash
/// match is resimulated, and `StrikeVictim.sim_id`'s own doc gives the reason it
/// exists: *"`Entity` is not stable across a rewind."* So this arm asks the
/// question of the composition that actually rewinds, rather than of a sandbox
/// that never does.
///
/// ⭐ IT TAKES THE SHORTCUT ITS NEIGHBOURS TAKE — insert a `MatchParticipantRoster`
/// and go straight to the gameplay route — because the select screen is not what
/// is under test here. `smash_roster`, not `smash_roster_at_levels`: the levelled
/// helper overwrites every participant as a CPU.
///
/// ⚠ WAIT FOR THE ROUND TO GO LIVE, NOT FOR A FIXED FRAME COUNT. A fixed settle
/// encodes the opening ceremony's LENGTH, and dev mode runs that ceremony ten
/// times faster — so the same number lands in a different world. The condition
/// is observable: a cast exists and nothing in it is still held by
/// `ScriptedControl`. Both halves, because a cast that does not exist yet is not
/// a cast whose hold has come off.
#[test]
fn every_fighter_in_a_match_carries_a_stable_identity() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    let mut live = false;
    for _ in 0..900 {
        app.update();
        let (seated, held) = {
            let world = app.world_mut();
            let mut all = world.query::<&MatchSeat>();
            let seated = all.iter(world).count();
            let mut q = world.query_filtered::<
                &MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            (seated, q.iter(world).count())
        };
        if seated > 0 && held == 0 {
            live = true;
            break;
        }
    }
    assert!(
        live,
        "premise: the round never went live, so there is no cast to census and \
         'every fighter is identified' would be true of nobody"
    );

    let (unidentified, total) = census(app.world_mut());
    assert!(
        total >= 2,
        "the match census saw {total} damageable bodies and a two-seat match has \
         at least two fighters. Fewer means the cast is not in \
         `StrikeVictim`'s population at all and this arm measures nothing"
    );
    assert!(
        unidentified.is_empty(),
        "{} of {total} damageable bodies in a LIVE MATCH carry no `SimId`: {:?}. \
         A match is resimulated, and the field exists because `Entity` does not \
         survive a rewind — so every geometric tie between these bodies is \
         resolved by Bevy query order, which a resimulation does not promise to \
         reproduce. Identity belongs where the body is BUILT",
        unidentified.len(),
        unidentified,
    );
}
