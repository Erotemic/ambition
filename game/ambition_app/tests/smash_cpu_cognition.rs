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

/// What one seated fighter is on one tick: the state that two mirrored
/// fighters must hold as reflections of each other (Q49).
#[derive(Clone, Debug)]
struct SeatState {
    pos: ambition_platformer2d::engine_core::Vec2,
    vel: ambition_platformer2d::engine_core::Vec2,
    facing: f32,
    control: Option<ambition_platformer2d::characters::actor::control::ActorControlFrame>,
    /// The move that plays and its clock.
    playing: Option<(String, f32)>,
    /// The position in the fighter's cognitive stream.
    stream: Option<u64>,
}

/// The two seats' states, seat 0 first, or `None` unless exactly two are seated.
fn seat_states(app: &mut App) -> Option<[SeatState; 2]> {
    use ambition_platformer2d::characters::control::ActorControl;
    use ambition_platformer2d::combat::moveset::MovePlayback;
    let world = app.world_mut();
    let mut rows: Vec<(usize, SeatState)> = world
        .query::<(
            &MatchSeat,
            &BodyKinematics,
            Option<&ActorControl>,
            Option<&MovePlayback>,
            Option<&Brain>,
        )>()
        .iter(world)
        .map(|(seat, kin, control, playback, brain)| {
            (
                seat.0,
                SeatState {
                    pos: kin.pos,
                    vel: kin.vel,
                    facing: kin.facing,
                    control: control.map(|control| control.0.clone()),
                    playing: playback.map(|playback| (playback.spec.id.clone(), playback.t)),
                    stream: match brain {
                        Some(Brain::StateMachine(StateMachineCfg::Fighter { state, .. })) => Some(state.noise),
                        _ => None,
                    },
                },
            )
        })
        .collect();
    rows.sort_by_key(|(seat, _)| *seat);
    match rows.as_slice() {
        [(_, a), (_, b)] => Some([a.clone(), b.clone()]),
        _ => None,
    }
}

/// The first field on which seat 1 is not seat 0 reflected about `mid`, or
/// `None`. Lateral values negate about the midline; vertical values, the move
/// and its clock, the buttons and the stream are equal. The stream is asked
/// last, so a pair whose streams differ still reports the first body field
/// that parts.
fn first_unreflected_field(a: &SeatState, b: &SeatState, mid: f32) -> Option<String> {
    let near = |x: f32, y: f32, tolerance: f32| (x - y).abs() <= tolerance;
    let lateral = [
        ("pos.x", a.pos.x - mid, -(b.pos.x - mid), 0.5),
        ("vel.x", a.vel.x, -b.vel.x, 0.5),
        ("facing", a.facing, -b.facing, 1e-4),
    ];
    for (name, x, y, tolerance) in lateral {
        if !near(x, y, tolerance) {
            return Some(format!("{name}: {x} against reflected {y}"));
        }
    }
    let vertical = [("pos.y", a.pos.y, b.pos.y, 0.5), ("vel.y", a.vel.y, b.vel.y, 0.5)];
    for (name, x, y, tolerance) in vertical {
        if !near(x, y, tolerance) {
            return Some(format!("{name}: {x} against {y}"));
        }
    }
    if a.playing.as_ref().map(|(id, _)| id) != b.playing.as_ref().map(|(id, _)| id) {
        return Some(format!("move: {:?} against {:?}", a.playing, b.playing));
    }
    if let (Some((_, ta)), Some((_, tb))) = (&a.playing, &b.playing) {
        if !near(*ta, *tb, 1e-4) {
            return Some(format!("move clock: {ta} against {tb}"));
        }
    }
    match (&a.control, &b.control) {
        (Some(x), Some(y)) => {
            let lateral = [
                ("control.locomotion.x", x.locomotion.x, -y.locomotion.x),
                ("control.attack_axis.x", x.attack_axis.x, -y.attack_axis.x),
                ("control.facing", x.facing, -y.facing),
            ];
            for (name, p, q) in lateral {
                if !near(p, q, 1e-4) {
                    return Some(format!("{name}: {p} against reflected {q}"));
                }
            }
            let vertical = [
                ("control.locomotion.y", x.locomotion.y, y.locomotion.y),
                ("control.attack_axis.y", x.attack_axis.y, y.attack_axis.y),
            ];
            for (name, p, q) in vertical {
                if !near(p, q, 1e-4) {
                    return Some(format!("{name}: {p} against {q}"));
                }
            }
            let buttons = [
                ("control.melee_pressed", x.melee_pressed, y.melee_pressed),
                ("control.melee_held", x.melee_held, y.melee_held),
                ("control.jump_pressed", x.jump_pressed, y.jump_pressed),
                ("control.jump_held", x.jump_held, y.jump_held),
                ("control.burst_pressed", x.burst_pressed, y.burst_pressed),
                ("control.shield_held", x.shield_held, y.shield_held),
                ("control.grab_pressed", x.grab_pressed, y.grab_pressed),
                ("control.special_pressed", x.special_pressed, y.special_pressed),
                ("control.fire", x.fire.is_some(), y.fire.is_some()),
            ];
            for (name, p, q) in buttons {
                if p != q {
                    return Some(format!("{name}: {p} against {q}"));
                }
            }
        }
        (None, None) => {}
        _ => return Some("control: one seat has none".to_string()),
    }
    if a.stream != b.stream {
        return Some(format!("stream: {:?} against {:?}", a.stream, b.stream));
    }
    None
}

/// The first frame on which the two seats are not reflections of each other
/// about the stage's centre line, and the field that parts there.
/// `with_stream: false` ignores a difference of stream alone.
///
/// The centre line is the stage's (`ambition_demo_smash::stage_centre`), not
/// the mean of the first frame: a mean moves with an asymmetric placement and
/// so hides it.
fn first_part(frames: &[[SeatState; 2]], with_stream: bool) -> Option<(usize, String)> {
    let mid = ambition_demo_smash::stage_centre().x;
    frames.iter().enumerate().find_map(|(index, [a, b])| {
        first_unreflected_field(a, b, mid)
            .filter(|field| with_stream || !field.starts_with("stream:"))
            .map(|field| (index, field))
    })
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
    // The two seats' full state on each frame both are seated.
    Vec<[SeatState; 2]>,
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

    let mut frames: Vec<[SeatState; 2]> = Vec::new();
    let mut grabbing: Vec<bool> = Vec::new();

    // ⛔⛤ **WAIT FOR THE PREMISE, THEN MEASURE — THIS USED TO SPEND A FIXED
    // BUDGET AND COUNT WHATEVER LANDED IN IT, AND THAT IS A MEMBER OF THE P0
    // FLAKY-BINARY FAMILY.** The loop ran `countdown + ticks` updates and kept
    // only those with two seated bodies, so **anything that delayed SEATING ate
    // the observation window silently** and the failure read *"only N frames had
    // two seated bodies, so the match did not really run"* — which names the
    // wrong cause: the match ran fine, it just started late.
    //
    // ⭐ MEASURED 2026-09-12, ten runs of ONE binary with no code change between
    // them: the first two-seated tick was `5, 5, 5, 5, 37, 35, 5, 32, 5, 45` —
    // **bimodal, and varying by forty frames** — while the LAST two-seated tick
    // was 137 in all ten. ⇒ **Nothing ends early; the budget is eaten at the
    // START.** A run observed in the gate lane at `4e9d34bee` got 48 frames,
    // which on this evidence is the same mechanism further out, not a KO.
    //
    // ⇒ So the seating is the PREMISE and is waited for, and `ticks` is the
    // MATCH window measured after it holds. A fixture that never reaches its
    // subject must say so rather than quietly measure less.
    const SEATING_BUDGET: usize = 600;
    let mut warmup = 0usize;
    let first_two_seated = loop {
        app.update();
        warmup += 1;
        if let Some(seated) = seat_states(&mut app) {
            break seated;
        }
        assert!(
            warmup < SEATING_BUDGET,
            "`{character}` never seated two bodies within {SEATING_BUDGET} updates \
             (the opening countdown is {countdown}), so the match never started and \
             nothing below would be measuring it. This is the PREMISE failing, not \
             the match ending early — see the seating trace."
        );
    };

    // The streams as CONSTRUCTED, read on the first frame both bodies exist —
    // before either has consumed a sample, so this is the seed the composition
    // chose rather than a position in the walk. ⭐ It is an unconditional `let`
    // now rather than an empty vec filled in a loop: waiting for the premise
    // means this frame is guaranteed to exist, so there is no "not seated yet"
    // state left for an `is_empty()` check to stand for.
    let streams = fighter_streams(&mut app);
    let held_now = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query::<&ambition_platformer2d::combat::capture::CapturedBy>();
        q.iter(world).next().is_some()
    };
    grabbing.push(held_now(&mut app));
    frames.push(first_two_seated);

    // ⚠ STILL CONDITIONAL ON TWO SEATS, because a body LEAVING mid-window (a KO,
    // a despawn) is a real gameplay outcome this must not paper over. The
    // difference is that a short `frames` now means exactly that, instead of
    // meaning the match started late.
    let mut left_mid_window: Option<usize> = None;
    for tick in 1..ticks {
        app.update();
        if let Some(seated) = seat_states(&mut app) {
            grabbing.push(held_now(&mut app));
            frames.push(seated);
        } else if left_mid_window.is_none() {
            left_mid_window = Some(tick);
        }
    }

    // ⚠ EVERY FIELD ANSWERS A DIFFERENT QUESTION, and `warmup` is the one that
    // was invisible before: how many updates the world needed before the match
    // existed at all. `left_mid_window` is now the ONLY way `observed` can fall
    // short of `ticks`, which is what makes a short window mean one thing.
    eprintln!(
        "[mirror-match] character={character} ticks={ticks} countdown={countdown} \
         warmup={warmup} observed={} left_mid_window={left_mid_window:?}",
        frames.len(),
    );
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

/// ⭐ TWO EMMYS ARE ONE FIGHTER REFLECTED, TICK BY TICK, UNTIL THE FIRST
/// GRAB (Q49, MIRROR-SYMMETRY).
///
/// Each frame compares the state that can part (`first_unreflected_field`):
/// position, velocity, facing, the move and its clock, the published control
/// and the stream position. Lateral values must negate about the stage
/// midline; the others must be equal. A part reports its frame and field.
///
/// The stage has one authored asymmetry: two bodies that grab each other on
/// one tick are a tie, and the lower `SimId` takes the hold
/// (`two_bodies_grabbing_each_other_on_one_tick_make_one_hold`). A mirror is
/// a fixed point, so no tie rule keeps the reflection; granting neither grab
/// was measured at zero captures in a minute. So the mirror may part when the
/// first grab holds, and not before. Measured 2026-10-08: the first part is on
/// frame 883, when seat 0's grab dash takes seat 1 and seat 1's is cancelled.
///
/// The control: two ordinary fighters, whose streams differ by policy, part
/// on a body field inside the same window, so the comparison can say no.
#[test]
fn two_emmys_are_one_fighter_reflected_until_the_first_grab() {
    // One window for both pairs.
    const WINDOW: usize = 1200;

    let (emmy_streams, emmy_frames, emmy_grabbing) = play_mirror_match(EMMY, WINDOW);
    let (ordinary_streams, ordinary_frames, _) = play_mirror_match(ORDINARY, WINDOW);

    // Non-vacuity: both matches ran, the spawns are apart, and the pairs
    // differ in cognition.
    assert!(
        emmy_frames.len() > 500 && ordinary_frames.len() > 500,
        "a match did not run long enough to compare (emmy {} frames, ordinary {} frames)",
        emmy_frames.len(),
        ordinary_frames.len(),
    );
    let first = &emmy_frames[0];
    assert!(
        (first[0].pos.x - first[1].pos.x).abs() > 1.0,
        "the two seats spawned on top of each other ({}, {}), so 'a mirror about \
         the midline' is not a claim this stage can express",
        first[0].pos.x,
        first[1].pos.x,
    );
    assert_eq!(
        emmy_streams[0].1, emmy_streams[1].1,
        "the Emmys did not share a stream, so this test is not observing the \
         exception ({emmy_streams:?})"
    );
    assert_ne!(
        ordinary_streams[0].1, ordinary_streams[1].1,
        "the control pair shared a stream too, so there is no contrast ({ordinary_streams:?})"
    );

    let emmy_part = first_part(&emmy_frames, true);
    let first_grab = emmy_grabbing.iter().position(|held| *held);
    let ordinary_part = first_part(&ordinary_frames, false);
    // Reported on success too: the grab clause works only when the mirror
    // parts, and a reader must see where it did.
    println!(
        "[mirror] emmy first part {emmy_part:?} of {} frames, first grab at \
         {first_grab:?}; {ORDINARY} first body part {ordinary_part:?}",
        emmy_frames.len(),
    );

    // THE CONTROL: the comparison says no to two fighters with two streams.
    assert!(
        ordinary_part.is_some(),
        "two {ORDINARY} CPUs with different streams stayed reflections on every \
         body field for {} frames, so this comparison cannot tell one mind from two",
        ordinary_frames.len(),
    );
    // THE CLAIM: two Emmys part only where the grab tie is resolved.
    if let Some((part, field)) = &emmy_part {
        assert!(
            first_grab.is_some_and(|grab| part + 1 >= grab),
            "two Emmys parted on frame {part} of {} ({field}) and the first grab \
             was at frame {first_grab:?}. A mirror that parts before the grab tie \
             has an asymmetry no authored rule names: a `signum(0)` side, a \
             list-order tie or a per-seat draw (fighter-brain.md, Q49)",
            emmy_frames.len(),
        );
    }
}

/// A match of two CPU fighters on the Smash stage, in the real host, after
/// `steps` steps of the developer's gravity cycle. Returns the DOWN of the
/// fighters' frames, and over [`CYCLED_MATCH_UPDATES`] updates: how many
/// samples of a seated fighter brain there were, and how many of them were on
/// the ground.
const CYCLED_MATCH_UPDATES: usize = 300;

fn a_match_in_cycled_gravity(steps: usize, by_the_key: bool) -> ((f32, f32), usize, usize) {
    use ambition_platformer2d::engine_core::BodyGroundState;
    use ambition_platformer2d::platformer::frame_env::ResolvedMotionFrame;
    use ambition_platformer2d::world::AmbientGravityRequest;
    let mut app = host();
    let roster = ambition_demo_smash::smash_roster_at_levels([ORDINARY, ORDINARY], &[RUNG, RUNG]);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE)));
    let mut warmup = 0;
    while seat_positions(&mut app).len() != 2 {
        app.update();
        warmup += 1;
        assert!(warmup < 600, "two CPU seats never seated: the match did not start");
    }
    assert_eq!(fighter_streams(&mut app).len(), 2, "each seat has a fighter brain");
    for _ in 0..steps {
        if by_the_key {
            press_the_gravity_key(&mut app);
        } else {
            // What the key and the developer menu's Gravity row write.
            app.world_mut().write_message(AmbientGravityRequest::Cycle);
        }
    }
    let (mut down, mut samples, mut grounded) = ((0.0, 0.0), 0, 0);
    for _ in 0..CYCLED_MATCH_UPDATES {
        app.update();
        let world = app.world_mut();
        for (_, frame, ground) in world.query::<(&MatchSeat, &ResolvedMotionFrame, &BodyGroundState)>().iter(world) {
            down = (frame.down().x, frame.down().y);
            samples += 1;
            grounded += usize::from(ground.on_ground);
        }
    }
    (down, samples, grounded)
}

/// One press of `\`, through the host's own key path.
#[cfg(feature = "input")]
fn press_the_gravity_key(app: &mut App) {
    use leafwing_input_manager::prelude::Buttonlike;
    Buttonlike::press(&KeyCode::Backslash, app.world_mut());
    app.update();
    Buttonlike::release(&KeyCode::Backslash, app.world_mut());
    app.update();
}

#[cfg(not(feature = "input"))]
fn press_the_gravity_key(_app: &mut App) {
    panic!("the key path needs the `input` feature");
}

/// The developer's gravity key reaches a hosted Smash match, and no CPU
/// fighter stands on a floor there.
///
/// `\` (and the developer menu's Gravity row) steps the ambient gravity of
/// the primary seat's room. The desktop binary hosts Smash beside its own
/// game, so the key turns the frame of each CPU fighter brain: the one road
/// in the shipped composition that puts a fighter brain in turned gravity.
///
/// The fighter brain measures the floor it stands on along world x
/// (`WorldView::floor_ahead`, and the retreat sign and floor share of
/// `situation.rs`). In turned gravity that is the wrong axis. It is NOT
/// converted, and this test is the reason and its guard: the stage is a
/// platform in the open, so in turned gravity each fighter falls out and the
/// match ends. A fighter that is never on the ground has no floor to measure.
///
/// Measured 2026-10-06, a fresh match for each direction, 900 updates: normal
/// gravity 1457 of 1800 samples on the ground; toward -x 0 of 1056 (the last
/// fighter was gone at update 662); toward -y 0 of 732 (gone at 366); toward
/// +x 0 of 1056. This test runs 300 updates of each, to keep the lane short;
/// the bound is 2% of the samples. The control's own numbers do not pass the
/// bound, so the bound can fail.
///
/// The control is normal gravity, where the fighters stand and fight.
///
/// If a turned direction goes red, a fighter brain stands on a floor in
/// turned gravity (a stage with a wall or a ceiling, or a fighter that
/// recovers along the turned DOWN). Then the world-x floor chain is live:
/// restate it on the viewer's axes, as `floor_below` and `ground_below` are
/// (LEVEL-BOX-READERS, `docs/planning/queue.md`).
#[test]
fn the_gravity_key_turns_a_hosted_smash_match_and_no_cpu_fighter_stands_in_turned_gravity() {
    let (down, samples, grounded) = a_match_in_cycled_gravity(0, false);
    eprintln!("[cycled-match] steps=0 down={down:?} samples={samples} grounded={grounded}");
    assert_eq!(down, (0.0, 1.0), "control: normal gravity");
    assert!(
        grounded * 10 > samples * 3,
        "control: in normal gravity the fighters stand on the stage: {grounded} of {samples} samples on the ground"
    );
    for (steps, turned) in [(1, (-1.0, 0.0)), (2, (0.0, -1.0)), (3, (1.0, 0.0))] {
        let (down, samples, grounded) = a_match_in_cycled_gravity(steps, false);
        eprintln!("[cycled-match] steps={steps} down={down:?} samples={samples} grounded={grounded}");
        assert_eq!(down, turned, "{steps} step(s) of the gravity cycle did not turn the frame of a CPU fighter");
        assert!(samples > 0, "{steps} step(s): no fighter was sampled");
        assert!(
            grounded * 50 <= samples,
            "gravity toward {turned:?}: {grounded} of {samples} samples of a CPU fighter were on the ground"
        );
    }
}

/// The same step through the real key: the road is the key, not only the
/// message a test can write.
#[cfg(feature = "input")]
#[test]
fn the_backslash_key_turns_the_frame_of_each_cpu_fighter_in_a_hosted_match() {
    let (down, samples, _) = a_match_in_cycled_gravity(1, true);
    assert!(samples > 0, "no fighter was sampled");
    assert_eq!(down, (-1.0, 0.0), "one press of the key did not turn the frame of a CPU fighter");
}
