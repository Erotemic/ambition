//! Two CPUs, one character, in the COMPLETE shipped composition — and Emmy
//! Ethereal's authored exception to the rule.
//!
//! why this file exists when four other suites already cover the policy.
//! The pieces were each pinned where they were cheapest to pin: Emmy's authoring
//! in `ambition_content`, the fold in `ambition_characters`, the seed policy and
//! seating in the actor monolith, the stage-level divergence in the demo app's own
//! suite. none of them could seat the REAL Emmy. `ambition_demo_smash_app`
//! does not compose `ambition_content`, so a roster naming `npc_emmy_noether` seats
//! nobody there, and the monolith's tests register a synthetic stand-in that
//! authors the trait. So the claim *"the character a player can actually pick off
//! the smash grid gets the shared stream"* was the one link asserted nowhere.
//!
//!  this file closes it, through `build_visible_app` — the one composition the
//! desktop binary runs — reading the grid the select screen will actually show.

use ambition_demo_smash::select::SmashRoster;
use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;

/// Emmy, by the id the catalog and the select grid both use.
const EMMY: &str = "npc_emmy_noether";

/// An ordinary selectable fighter, as the CONTROL. it must be a real grid
/// member in this composition or the contrast proves nothing — asserted below.
const ORDINARY: &str = "npc_pirate_admiral";

/// The rung both seats play at. Any single value works; what matters is that the
/// two seats share it, because difficulty is one of the seed's terms and a test
/// that varied it would be measuring the wrong difference.
const RUNG: u8 = 5;

/// What one seated body's fighter brain is thinking with, keyed by seat.
///
/// `FighterState::noise` IS the cognitive stream — construction stores the seed
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
/// the frame is load-bearing and this is the second suite to need the note:
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
/// the roster is the stage's own builder (`smash_roster_at_levels`), not a
/// hand-built one: it publishes itself under the smash experience and names the
/// `duelist_l{rung}` policies that experience registers, which is what makes each
/// seat resolve to a real FIGHTER brain rather than a refused profile.
fn play_mirror_match(
    character: &str,
    ticks: usize,
) -> (
    Vec<(usize, u64)>,
    Vec<Vec<(usize, ambition_platformer2d::engine_core::Vec2)>>,
    // ⭐ WHICH FRAMES A GRAB WAS LIVE ON. A mutual grab is a TIE, and resolving
    // one is the single gameplay rule on this stage that treats two mirrored
    // bodies differently — see `acquire_captures`. The mirror below is allowed
    // to break there and nowhere else, so the reflection test needs to know
    // where "there" was.
    Vec<bool>,
) {
    let mut app = host();

    // NON-VACUITY, and it is the whole point of running in this host: the
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
    let countdown = roster.rules.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut streams: Vec<(usize, u64)> = Vec::new();
    let mut frames: Vec<Vec<(usize, ambition_platformer2d::engine_core::Vec2)>> = Vec::new();
    let mut grabbing: Vec<bool> = Vec::new();
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
            let held = {
                let world = app.world_mut();
                let mut q = world.query::<&ambition_platformer2d::combat::capture::CapturedBy>();
                q.iter(world).next().is_some()
            };
            frames.push(seated);
            grabbing.push(held);
        }
    }
    (streams, frames, grabbing)
}

/// EMMY'S AUTHORED MIRROR SYMMETRY REACHES THE REAL SELECTABLE CHARACTER,
/// through the composition the desktop binary runs.
///
/// Two CPU seats of the Emmy a player picks off the grid receive the SAME
/// deterministic cognitive stream, because her character definition authors
/// `preserving_mirror_symmetry()`. nothing here reaches into the brain seed or
/// registers a stand-in: the only inputs are the host, the grid and the stage's own
/// roster builder.
#[test]
fn both_emmy_seats_receive_one_cognitive_stream_in_the_real_host() {
    let (streams, frames, _) = play_mirror_match(EMMY, 120);
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

/// THE CONTROL, IN THE SAME COMPOSITION: an ordinary selectable fighter's two
/// seats do NOT share a stream.
///
/// Without this, the test above is satisfied by the very defect this change
/// removed — a participant-blind seed gave EVERY pair of same-character CPUs one
/// stream, Emmy included.  the pair is the assertion; neither half means much
/// alone.
#[test]
fn two_seats_of_an_ordinary_selectable_fighter_do_not_share_a_stream() {
    let (streams, _, _) = play_mirror_match(ORDINARY, 30);
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

/// How long the two bodies stay an exact reflection of each other, and how
/// many frames were observed at all.
///
/// the midline is the stage's own symmetry, read off the first frame both bodies
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

/// Two Emmys remain mirror-symmetric substantially longer than fighters with
/// independent streams. The comparison uses the same observation window for
/// both pairs so the assertion measures controller-stream behavior rather than
/// a short-window symmetry shared by every fighter.
#[test]
fn two_emmys_hold_a_mirror_far_longer_than_two_ordinary_fighters() {
    // One window for both, so the comparison cannot be an artifact of two
    // different observation lengths.
    const WINDOW: usize = 1200;

    let (emmy_streams, emmy_frames, emmy_grabbing) = play_mirror_match(EMMY, WINDOW);
    let (ordinary_streams, ordinary_frames, _) = play_mirror_match(ORDINARY, WINDOW);

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

    // THE CLAIM: the ordinary pair's reflection breaks, and Emmy's does not.
    assert!(
        ordinary_mirrored < ordinary_seen,
        "two {ORDINARY} CPUs stayed an exact reflection for the whole \
         {ordinary_seen}-frame match despite having different cognitive streams — \
         so this window cannot tell shared cognition from separate cognition, and \
         the Emmy figure below means nothing"
    );
    // ⛔⛔ A RATE, NOT A COUNT, and this compared counts across matches of
    // DIFFERENT LENGTHS. The claim above is "the ordinary pair's reflection
    // breaks and Emmy's does not", which is a property of the fraction of the
    // match spent reflected — but a decisive Emmy match is a SHORT one, so a
    // perfect mirror could lose to a long sloppy one on absolute frames.
    // Measured: 856 of 856 (100%) against 440 of 1376 (32%), and `856 > 880`
    // is false. The stronger result failed the weaker test.
    // ⛔⛔ THE MARGIN WAS 2.0 AND IT WAS FITTED BEFORE ANY SHARED ACTION WAS
    // ABSOLUTE-DIRECTIONAL. Re-derived 2026-08-24 when the platform fighter
    // stopped guarding in the air (`ShieldTuning::air_guard`) and a shield press
    // up there became the AIR DODGE. That one rule moved BOTH ends, and the
    // mechanism is the dodge's own geometry:
    //
    //   `vel = (frame.side() * aim.x + frame.down() * aim.y) * air_dodge_speed`
    //
    //   - a NEUTRAL stick sets velocity to ZERO, and two bodies that both halt
    //     in mid-air keep whatever reflection they had. The INDEPENDENT pair
    //     rose from a recorded 32% to 52%.
    //   - a DIRECTIONAL stick aims in the gravity frame's axes, not
    //     facing-relative — which is correct for the genre — so two mirrored
    //     bodies sharing one stream dodge the same ABSOLUTE way and stop being
    //     each other's reflection. Emmy fell from a recorded 100% to 84%.
    //
    // ⇒ so the claim is unchanged and only the constant moved, because the
    // world it was measured in did. 84% against 52% is still decisive; the same
    // shape as before, with the floor raised and the ceiling lowered by one
    // mechanic that belongs to both pairs equally.
    let emmy_rate = emmy_mirrored as f64 / emmy_seen as f64;
    let ordinary_rate = ordinary_mirrored as f64 / ordinary_seen as f64;
    assert!(
        emmy_rate > ordinary_rate * 1.5,
        "two Emmys held a mirror for {emmy_mirrored} of {emmy_seen} frames \
         ({:.0}%) while two {ORDINARY} held one for {ordinary_mirrored} of \
         {ordinary_seen} ({:.0}%) — not the decisive difference a shared \
         cognitive stream should produce",
        emmy_rate * 100.0,
        ordinary_rate * 100.0
    );
    // ⛔⛔ THE MIRROR MAY BREAK IN EXACTLY ONE PLACE, and this is the clause
    // that says where. A shared cognitive stream keeps two bodies reflected
    // under symmetric circumstances — unless a GAMEPLAY rule has to treat them
    // differently, and this stage has one: two bodies that grab each other on
    // the same tick are a TIE, and `acquire_captures` resolves it in favour of
    // the lower `SimId`.
    //
    // ⚠ THERE IS NO SYMMETRIC ALTERNATIVE. A mirror is a fixed point —
    // identical inputs produce identical states however a tie is resolved — so
    // the only resolution that preserves the reflection is granting NEITHER
    // grab, and that was tried and measured: 126 attempts over a minute, zero
    // captures, zero pummels, zero throws. Granting BOTH is the deadlock that
    // cost D194 a third of a match.
    //
    // ⭐ so the claim is now "the reflection holds until a grab is live", which
    // is STRONGER where it matters: a cognition leak breaks the mirror with no
    // capture anywhere in sight, and that still fails here.
    let first_grab = emmy_grabbing.iter().position(|held| *held);
    // ⭐ REPORTED ON SUCCESS TOO: the clause below is only doing work when the
    // mirror actually breaks, and a reader who cannot see these three numbers
    // cannot tell a guard that held from one that had nothing to hold.
    println!(
        "[mirror] emmy held {emmy_mirrored} of {emmy_seen} frames, first grab at \
         {first_grab:?}; {ORDINARY} held {ordinary_mirrored} of {ordinary_seen}"
    );
    assert!(
        emmy_mirrored == emmy_seen || first_grab.is_some_and(|grab| emmy_mirrored + 1 >= grab),
        "two Emmys broke their mirror after {emmy_mirrored} of {emmy_seen} frames, \
         and the first grab on the stage was at frame {first_grab:?} — so the \
         reflection did not end where the one rule that arbitrates between two \
         mirrored bodies fires. Check for a per-seat gameplay asymmetry on the \
         stage before suspecting the cognition seed"
    );
}
