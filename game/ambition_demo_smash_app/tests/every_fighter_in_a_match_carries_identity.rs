//! D-DAMAGEABLE-BODY-IDENTITY, the arm that reaches the ROSTER.
//!
//! ⭐⭐ **THE POPULATION IS `StrikeVictim`'S OWN QUERY**: an entity carrying both
//! `CenteredAabb` and `ActorFaction` is a candidate victim, and its `sim_id` is
//! `Option` — *"a body without one still gets hit, it just cannot win the tie."*
//! `victim_identity_key` sorts an absent identity LAST, which fixes the
//! inversion; what it cannot fix is TWO absent identities, which compare equal
//! and hand the tie to Bevy query order. Query order is not reproduced by a
//! resimulation, so that tie is a desync.
//!
//! ⛔⛔ **THE EXISTING RUNTIME GATE REACHES THREE BODIES AND SAYS SO.**
//! `ambition_app/tests/damageable_bodies_carry_identity.rs` sweeps the rl_sim
//! sandbox: the player, a staged mob. **A seated match fighter is a different
//! construction road** — `character_runtime/match_activation.rs` — and no arm
//! reached it. That road is the whole Smash roster, in every match, which is
//! the largest damageable population the game has.
//!
//! ⚠ **A STATIC SCAN CANNOT ANSWER IT.** `CenteredAabb` and `ActorFaction`
//! arrive from separate inserts down a bundle chain, so a scanner keyed on "one
//! spawn call naming both" describes its own method.
//! `scripts/measure_damageable_bundles_without_identity.py` says this in its own
//! output and calls itself a lower bound; it names `match_activation.rs:90` as a
//! LEAD. This test is the instrument that can settle it.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::combat::components::{ActorFaction, CenteredAabb};
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;

/// Seat a match on the gameplay route, exactly as the other acceptance arms do.
fn start_a_match(app: &mut App) {
    let mut roster = ambition_demo_smash::smash_roster_at_levels(
        [
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
        ],
        &[5, 5],
    );
    roster.rules.stocks = Some(1);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
}

/// Every entity the strike resolver would consider a victim, split by whether it
/// carries a stable identity.
fn census(world: &mut World) -> (Vec<String>, usize) {
    let mut q = world.query::<(
        Entity,
        &CenteredAabb,
        &ActorFaction,
        Option<&SimId>,
        Option<&Name>,
    )>();
    let mut unidentified = Vec::new();
    let mut total = 0usize;
    for (entity, _, _, sim, name) in q.iter(world) {
        total += 1;
        if sim.is_none() {
            unidentified.push(
                name.map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| format!("{entity:?}")),
            );
        }
    }
    unidentified.sort();
    (unidentified, total)
}

/// ⛔ EVERY FIGHTER A MATCH SEATS CARRIES A `SimId`.
///
/// ⚠ THE ANTI-VACUITY FLOOR IS FIRST AND IT IS NOT OPTIONAL. A route that never
/// reached gameplay produces zero candidate victims, zero unidentified, and a
/// green run that certifies nothing — which is this repository's most repeated
/// instrument failure. The floor asserts the match actually stood two fighters
/// up before the identity claim is allowed to pass.
#[test]
fn every_damageable_body_a_match_seats_carries_a_stable_identity() {
    let mut app = build_demo_app();
    // ⚠ THE BOOT TICKS ARE NOT DECORATION. Routing to gameplay before the shell
    // has come up seats nobody, and the anti-vacuity floor below then reports a
    // harness failure rather than a finding — which is exactly what it did on
    // the first run of this file.
    for _ in 0..30 {
        app.update();
    }
    start_a_match(&mut app);
    for _ in 0..240 {
        app.update();
    }

    let seats = {
        let world = app.world_mut();
        let mut q = world.query::<&ambition_platformer2d::actor::MatchSeat>();
        q.iter(world).count()
    };
    assert!(
        seats >= 2,
        "the match seated {seats} fighter(s), so this run reached no roster to \
         ask about — the claim below would pass over an empty world"
    );

    let (unidentified, total) = census(app.world_mut());
    // MEASURED 2026-09-11 at this floor: the seated match presents exactly 2
    // candidate victims, both identified. The floor is the ROSTER, not that
    // number — pinning 2 would redden on a four-player match, which is growth.
    assert!(
        total >= 2,
        "{total} candidate victim(s) in a seated match: the census query matched \
         nothing, so it is measuring its own filter rather than the roster"
    );
    assert!(
        unidentified.is_empty(),
        "{} of {total} damageable bodies in a live match carry NO `SimId`: {:?}\n\
         ⇒ Two of these at one position tie on every geometric key, and the \
         resolver's final tie-break compares two absent identities as EQUAL — so \
         Bevy query order decides who is struck. A resimulation does not \
         reproduce query order, which makes that a desync and not a preference.",
        unidentified.len(),
        unidentified
    );
}
