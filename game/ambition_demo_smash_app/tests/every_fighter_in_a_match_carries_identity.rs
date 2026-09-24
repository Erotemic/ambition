//! D-DAMAGEABLE-BODY-IDENTITY, for the roster.
//!
//! The population is `StrikeVictim`'s own query: an entity with both
//! `CenteredAabb` and `ActorFaction` is a candidate victim, and its `sim_id`
//! is optional. `victim_identity_key` sorts an absent identity last, but two
//! absent identities compare equal and leave the tie to Bevy query order,
//! which a resimulation does not reproduce. That tie is a desync.
//!
//! `ambition_app/tests/damageable_bodies_carry_identity.rs` covers the rl_sim
//! sandbox. A seated match fighter is built on a different road
//! (`character_runtime/match_activation.rs`), and that road is the whole
//! Smash roster. A static scan cannot settle it, because `CenteredAabb` and
//! `ActorFaction` arrive from separate inserts
//! (`scripts/measure_damageable_bundles_without_identity.py` is a lower
//! bound). This test checks the live world.

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

/// Every fighter a match seats carries a `SimId`.
///
/// The anti-vacuity floor comes first: a route that never reached gameplay
/// gives zero candidates and zero unidentified, which would pass.
#[test]
fn every_damageable_body_a_match_seats_carries_a_stable_identity() {
    let mut app = build_demo_app();
    // Boot ticks first: routing before the shell is up seats nobody.
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
    // The floor is the roster, not an exact count: pinning 2 would fail on a
    // four-player match.
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
