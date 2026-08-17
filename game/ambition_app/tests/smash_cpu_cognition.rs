//! **Two CPUs, one character, in the COMPLETE shipped composition** — and Emmy
//! Ethereal's authored exception to the rule. (queue D128, 2026-08-17)
//!
//! ⭐⭐ **why this file exists when four other suites already cover the policy.**
//! The pieces were each pinned where they were cheapest to pin: Emmy's authoring
//! in `ambition_content`, the fold in `ambition_characters`, the seed policy and
//! seating in the actor monolith, the stage-level divergence in the demo app's own
//! suite. ⛔ **none of them could seat the REAL Emmy.** `ambition_demo_smash_app`
//! does not compose `ambition_content`, so a roster naming `npc_emmy_noether` seats
//! nobody there, and the monolith's tests register a synthetic stand-in that
//! authors the trait. So the claim *"the character a player can actually pick off
//! the smash grid gets the shared stream"* was the one link asserted nowhere.
//!
//! ⇒ this file closes it, through `build_visible_app` — the one composition the
//! desktop binary runs — reading the grid the select screen will actually show.
//!
//! ⚠ **ungated on purpose.** The pieces this proves are a CPU/brain concern, so
//! `smash_in_the_host` would have been the obvious neighbour, and it is
//! `#![cfg(feature = "input")]` because it drives real key presses. Nothing here
//! needs a keypress — a roster and a route command are enough — and putting it
//! behind that feature would have hidden it from the project gate, which is the
//! failure mode `awaiting-maintainer-decision.md` §9 is about.

use ambition_demo_smash::select::SmashRoster;
use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
use ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry;
use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;

/// Emmy, by the id the catalog and the select grid both use.
const EMMY: &str = "npc_emmy_noether";

/// An ordinary selectable fighter, as the CONTROL. ⚠ it must be a real grid
/// member in this composition or the contrast proves nothing — asserted below.
const ORDINARY: &str = "npc_pirate_admiral";

/// The rung both seats play at. Any single value works; what matters is that the
/// two seats share it, because difficulty is one of the seed's terms and a test
/// that varied it would be measuring the wrong difference.
const RUNG: u8 = 5;

/// What one seated body's fighter brain is thinking with, keyed by seat.
///
/// ⭐ `FighterState::noise` IS the cognitive stream — construction stores the seed
/// there verbatim and every later sample advances from it — so it is the smallest
/// deterministic property that answers *"are these two fighters the same mind?"*.
fn fighter_streams(app: &mut App) -> Vec<(usize, u64)> {
    let world = app.world_mut();
    let mut streams: Vec<(usize, u64)> = world
        .query::<(&MatchSeat, &Brain)>()
        .iter(world)
        .filter_map(|(seat, brain)| match brain {
            Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) => {
                Some((seat.0, state.noise))
            }
            _ => None,
        })
        .collect();
    streams.sort_by_key(|(seat, _)| *seat);
    streams
}

fn seat_positions(app: &mut App) -> Vec<(usize, ambition_platformer2d::engine_core::Vec2)> {
    let world = app.world_mut();
    let mut rows: Vec<_> = world
        .query::<(&MatchSeat, &BodyKinematics)>()
        .iter(world)
        .map(|(seat, kin)| (seat.0, kin.pos))
        .collect();
    rows.sort_by_key(|(seat, _)| *seat);
    rows
}

/// The composed host, one frame in — which is where the seatable registry exists.
///
/// ⚠ **the frame is load-bearing and this is the second suite to need the note**:
/// `PreparedCharacterRegistry` is filled by a `Startup` system, so a build that
/// has never updated has a catalog and no registry, and the grid assembled from it
/// would be empty. `smash_roster_movesets` carries the same warning.
fn host() -> App {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    app
}

/// Seat two CPUs on ONE character on the real smash stage and run the match for
/// `ticks`, returning every stream observed and the seat positions per tick.
///
/// ⚠ **the roster is the stage's own builder** (`smash_roster_at_levels`), not a
/// hand-built one: it publishes itself under the smash experience and names the
/// `duelist_l{rung}` policies that experience registers, which is what makes each
/// seat resolve to a real FIGHTER brain rather than a refused profile.
fn play_mirror_match(
    character: &str,
    ticks: usize,
) -> (
    Vec<(usize, u64)>,
    Vec<Vec<(usize, ambition_platformer2d::engine_core::Vec2)>>,
) {
    let mut app = host();

    // ⛔ NON-VACUITY, and it is the whole point of running in this host: the
    // character must be one a player can actually PICK. `SmashRoster::assemble`
    // filters the wish list down to what this composition can seat, so a name
    // that survives it is a name on the select screen.
    {
        let registry = app
            .world()
            .get_resource::<PreparedCharacterRegistry>()
            .expect("the composed host has a prepared-character registry");
        let grid = SmashRoster::assemble(registry);
        let ids: Vec<&str> = grid.ids().collect();
        assert!(
            ids.contains(&character),
            "`{character}` is not on the assembled smash grid in this composition, \
             so seating it proves nothing about what a player can pick. Grid: {ids:?}"
        );
    }

    let roster = ambition_demo_smash::smash_roster_at_levels([character, character], &[RUNG, RUNG]);
    let countdown = roster.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut streams: Vec<(usize, u64)> = Vec::new();
    let mut frames: Vec<Vec<(usize, ambition_platformer2d::engine_core::Vec2)>> = Vec::new();
    for _ in 0..(countdown + ticks) {
        app.update();
        let seated = seat_positions(&mut app);
        if seated.len() == 2 {
            if streams.is_empty() {
                // The streams as CONSTRUCTED, read on the first frame both bodies
                // exist — before either has consumed a sample, so this is the
                // seed the composition chose rather than a position in the walk.
                streams = fighter_streams(&mut app);
            }
            frames.push(seated);
        }
    }
    (streams, frames)
}

/// ⭐⭐ **EMMY'S AUTHORED MIRROR SYMMETRY REACHES THE REAL SELECTABLE CHARACTER,
/// through the composition the desktop binary runs.**
///
/// Two CPU seats of the Emmy a player picks off the grid receive the SAME
/// deterministic cognitive stream, because her character definition authors
/// `preserving_mirror_symmetry()`. ⛔ nothing here reaches into the brain seed or
/// registers a stand-in: the only inputs are the host, the grid and the stage's own
/// roster builder.
#[test]
fn both_emmy_seats_receive_one_cognitive_stream_in_the_real_host() {
    let (streams, frames) = play_mirror_match(EMMY, 120);
    assert_eq!(
        streams.len(),
        2,
        "the Emmy mirror match never seated two CPU FIGHTERS, so there is no \
         cognition to compare — got {streams:?}"
    );
    assert!(
        frames.len() > 60,
        "only {} frames had two seated bodies, so the match did not really run",
        frames.len()
    );
    assert_eq!(
        streams[0].1, streams[1].1,
        "the two Emmy seats got DIFFERENT cognitive streams ({streams:?}), so her \
         authored mirror symmetry does not survive the full host composition — \
         check that `preserves_mirror_symmetry` is still carried from her \
         definition through preparation to `ActorConfig`"
    );
}

/// ⛔ **THE CONTROL, IN THE SAME COMPOSITION: an ordinary selectable fighter's two
/// seats do NOT share a stream.**
///
/// Without this, the test above is satisfied by the very defect this change
/// removed — a participant-blind seed gave EVERY pair of same-character CPUs one
/// stream, Emmy included. ⇒ **the pair is the assertion**; neither half means much
/// alone.
#[test]
fn two_seats_of_an_ordinary_selectable_fighter_do_not_share_a_stream() {
    let (streams, _) = play_mirror_match(ORDINARY, 30);
    assert_eq!(
        streams.len(),
        2,
        "the {ORDINARY} mirror match never seated two CPU FIGHTERS — got {streams:?}"
    );
    assert_ne!(
        streams[0].1, streams[1].1,
        "two seats of {ORDINARY} share one cognitive stream ({streams:?}), so every \
         same-character CPU pair is one mind again and Emmy's trait is not an \
         exception to anything"
    );
}

/// **How long the two bodies stay an exact reflection of each other**, and how
/// many frames were observed at all.
///
/// ⭐ the midline is the stage's own symmetry, read off the first frame both bodies
/// exist on rather than written here as a literal. "A reflection" then means seat 1
/// is seat 0 flipped about it:
///
/// ```text
/// mirror error = |(x0 − mid) + (x1 − mid)|  +  |y0 − y1|
/// ```
fn mirrored_frames(
    frames: &[Vec<(usize, ambition_platformer2d::engine_core::Vec2)>],
) -> (usize, usize) {
    let Some(first) = frames.first() else {
        return (0, 0);
    };
    let mid = (first[0].1.x + first[1].1.x) / 2.0;
    let mut held = 0usize;
    for frame in frames {
        let error = ((frame[0].1.x - mid) + (frame[1].1.x - mid)).abs()
            + (frame[0].1.y - frame[1].1.y).abs();
        if error > 1.0 {
            break;
        }
        held += 1;
    }
    (held, frames.len())
}

/// ⭐⭐ **AND THE SHARED STREAM SHOWS: two Emmys hold a mirror far longer than two
/// of anybody else.** This is the only assertion in this file a player could see.
///
/// ⛔⛔ **COMPARATIVE ON PURPOSE, because measuring Emmy alone is VACUOUS — a first
/// draft did exactly that and passed for every character.** Measured over a
/// 120-tick window both pairs stayed mirrored to within 0.0003px, Emmy and the
/// Pirate Admiral alike: the fighters have not yet consumed enough execution noise
/// for two different streams to show, so *"Emmy mirrored for the whole window"* was
/// a fact about the window, not about Emmy. ⇒ **the honest form is a contrast
/// against a character with its own streams, over a window long enough for the
/// difference to exist.**
///
/// Measured 2026-08-17 in this host, at rung 5:
///
/// ```text
///                      streams   mirrored for      match ran
/// npc_emmy_noether          IDENTICAL 2576 of 2576 fr   2576 fr  (a stalemate: they
///                                                           answer every move
///                                                           with its reflection)
/// npc_pirate_admiral   DIFFERENT  488 of 1548 fr   1548 fr  (they fight, and it
///                                                           ends)
/// ```
///
/// ⚠ **488 frames is ~8.1s, and Jon reported exactly that in play** — *"it took a
/// while for Booule to desync, but they eventually did"*. ⛔ **that number is a
/// finding, not a threshold to tighten here.** The fighter brain spends its stream
/// on press timing alone, so two bodies both walking toward each other stay mirrored
/// whatever their streams say. Two fixes for it were built, measured and reverted —
/// one broke five behavioural guards, the other bought one frame — and the note on
/// `brain_builders::fighter_cognition_seed` records both so nobody builds a third
/// before reading them.
///
/// ⚠ **the shorter Admiral match is itself the finding**: two fighters that think
/// differently actually resolve their match, while two Emmys mirror each other into
/// a much longer one. Nothing here asserts that — it is context for whoever reads a
/// failure.
///
/// ⚠ **a failure is not automatically a cognition bug.** The mirror is emergent —
/// identical cognition reading symmetric information — so if the stage grows a
/// per-seat asymmetry (a spawn offset by seat index is a known pending change, D128
/// defect 3) Emmy's window shrinks for a good reason. Re-derive the numbers before
/// touching the seed.
#[test]
fn two_emmys_hold_a_mirror_far_longer_than_two_ordinary_fighters() {
    // One window for both, so the comparison cannot be an artifact of two
    // different observation lengths.
    const WINDOW: usize = 1200;

    let (emmy_streams, emmy_frames) = play_mirror_match(EMMY, WINDOW);
    let (ordinary_streams, ordinary_frames) = play_mirror_match(ORDINARY, WINDOW);

    let (emmy_mirrored, emmy_seen) = mirrored_frames(&emmy_frames);
    let (ordinary_mirrored, ordinary_seen) = mirrored_frames(&ordinary_frames);

    // Non-vacuity, three ways: both matches must really have run, the spawns must
    // really be mirrored, and the two pairs must really differ in cognition — or
    // the comparison below is measuring nothing.
    assert!(
        emmy_seen > 500 && ordinary_seen > 500,
        "a match did not run long enough to compare (emmy {emmy_seen} frames, \
         ordinary {ordinary_seen} frames)"
    );
    let first = &emmy_frames[0];
    assert!(
        (first[0].1.x - first[1].1.x).abs() > 1.0,
        "the two seats spawned on top of each other ({}, {}), so 'a mirror about \
         the midline' is not a claim this stage can express any more",
        first[0].1.x,
        first[1].1.x,
    );
    assert_eq!(
        emmy_streams[0].1, emmy_streams[1].1,
        "the Emmys did not share a stream, so this test is not observing the \
         exception ({emmy_streams:?})"
    );
    assert_ne!(
        ordinary_streams[0].1, ordinary_streams[1].1,
        "the control pair shared a stream too, so there is no contrast to measure \
         ({ordinary_streams:?})"
    );

    // ⭐ THE CLAIM: the ordinary pair's reflection breaks, and Emmy's does not.
    assert!(
        ordinary_mirrored < ordinary_seen,
        "two {ORDINARY} CPUs stayed an exact reflection for the whole \
         {ordinary_seen}-frame match despite having different cognitive streams — \
         so this window cannot tell shared cognition from separate cognition, and \
         the Emmy figure below means nothing"
    );
    assert!(
        emmy_mirrored > ordinary_mirrored * 2,
        "two Emmys held a mirror for {emmy_mirrored} of {emmy_seen} frames while \
         two {ORDINARY} held one for {ordinary_mirrored} of {ordinary_seen} — not \
         the decisive difference a shared cognitive stream should produce"
    );
    assert_eq!(
        emmy_mirrored, emmy_seen,
        "two Emmys broke their mirror after {emmy_mirrored} of {emmy_seen} frames. \
         Under symmetric circumstances a shared stream should keep them reflected; \
         check for a per-seat gameplay asymmetry on the stage before suspecting the \
         cognition seed"
    );
}
