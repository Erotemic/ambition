//! Count what two CPUs actually DO to each other over a match.
//!
//! `cargo run -p ambition_demo_smash_app --bin smash_tool -- match-report -- [SECONDS] [CHARACTER] [--runs N]`
//!
//! With `--features causal` it also prints what the brain decided, grouped by
//! the situation it answered. The outcome half says what a fight did; the
//! decision half says why. A behaviour change can have second-order effects
//! (for example, through the situation classifier), and both halves are
//! needed to see them.
//!
//! Mechanics can be authored, tuned, and reachable, and still unused by the
//! CPUs. Counting in a real match finds that; unit tests do not. Run this
//! after any change that claims to affect how a fight goes.
//!
//! ⛔⛔ A BEFORE/AFTER IS `--runs 3` MEDIANS, NEVER ONE RUN. Measured
//! 2026-10-30 (pointed vs pugnacious, 60 s): at rung 9 three seeds gave
//! moves 22–49–71 and damage 9–105–120 — a 3× spread from the match's own
//! chaos, wider than any change under test. The rule for a claim about the
//! CPU: `--runs 3`, read the median line (both seats summed), at rungs 5
//! AND 9, for the pair (`pointed_polygon --opponent pugnacious_polygon`) AND
//! the mirror (`pugnacious_polygon`); the shipped roster needs the shipped
//! composition (`ambition_app_tools`'s `smash_cpu_report`). A single run is
//! a trace, good for reading a mechanism, never for a number.
//!
//! Read `live ticks` beside every count: a round that ends on a KO freezes
//! the simulation for the rest of the window, so a 60 s run can hold 16 s of
//! fighting. Read `whiffs` with its two splits: `⇒ evaded` (the foe was
//! unhittable for a tick of the move) and the per-move table's `along` /
//! `across` (the whiffs' nearest approach to a live hitbox, with the hits'
//! as the control). A whiff reading `0 / 0` overlapped the foe and was
//! refused a verdict; it is not an admission miss. The recovery specials
//! (`polygon_brawler_uppercut`, `polygon_rising_edge`) are pressed as
//! recovery routes and whiff by nature; they are a quarter to a third of
//! all whiffs (pooled over three runs: 20 of 56, 29 of 93, 28 of 112).
//!
//! It is observational and has no pass/fail threshold. The asserting guard is
//! in `tests/the_repertoire_gets_used.rs`.
//!
//! Use `--runs N`: one run is not a measurement. Each fighter has an
//! execution-noise stream, and one thirty-second sample is too noisy to tune
//! against. With `--runs` the spread prints as `min–median–max`.

use crate::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;
use bevy::prelude::*;

/// One seat's tally. Ticks unless the name says otherwise.
#[derive(Default, Clone)]
struct Tally {
    /// Peak percent, not the final reading. A KO resets a body to zero, so the
    /// last value says how recently somebody died.
    damage: i32,
    hitstun: usize,
    tumbling: usize,
    knocked_down: usize,
    evading: usize,
    /// Ticks this body could not be struck at all. Separates "the CPUs are
    /// defensive" from "the CPUs are unhittable".
    unhittable: usize,
    /// Which of the terms in `body_vulnerable` was false, counted separately,
    /// so the owning term can be fixed.
    unhit_invuln: usize,
    unhit_evading: usize,
    /// The ledge's share of `unhit_evading`. It is a part of that column, so
    /// the columns do not add up to `unhittable`. The ledge has its own
    /// intangibility flag, so it can be tuned apart from the dodge roll.
    unhit_ledge: usize,
    unhit_parry_window: usize,
    unhit_iframes: usize,
    /// How often this fighter changes its walking direction: sign changes in
    /// its locomotion intent, ignoring ticks with no intent.
    ///
    /// The initial dash (D217) re-arms on each new direction, so a body that
    /// flips often restarts its dash instead of travelling. In the kernel, a
    /// body flipping every 4 ticks covers 675px where a steady one covers 1339.
    steer_flips: usize,
    /// Ticks this fighter asked for a direction at all, so `steer_flips` has a
    /// denominator and "rarely flips" cannot mean "rarely moves".
    steer_held: usize,
    shielding: usize,
    parries_caught: usize,
    tech_armed: usize,
    charge_held: usize,
    /// The highest charge fraction this seat ever reached.
    best_charge: f32,
    /// Ticks on which the world MOVED: a position, or a move's clock, changed
    /// for some seat. A round that ends on a KO freezes the simulation for
    /// the rest of the window (`match_over`), so two 60 s runs can hold very
    /// different amounts of fighting; every count above is per live tick,
    /// not per second asked for.
    live_ticks: usize,
    /// Distinct move starts, so a match that throws one move reads as one.
    moves_started: usize,
    /// Moves that ENDED without their volume ever overlapping a hurtbox
    /// (`MoveContact::overlapped` false for the whole playback): the whiffs.
    /// A move still playing when the window closes is not counted either way.
    whiffs: usize,
    /// The whiffs during which the nearest opponent was UNHITTABLE for at
    /// least one tick (dodging, invulnerable, parrying, in i-frames): swings
    /// the foe evaded, as against swings that found nothing. The remainder,
    /// `whiffs - whiffs_evaded`, is the admission's share.
    whiffs_evaded: usize,
    /// The fastest launch this body was handed, and the speed its own tuning
    /// says a launch must beat to become a tumble. Together they separate the
    /// two causes of "nobody tumbled".
    ///
    /// Sampled only on hitstun. Every attack lunges (George's lunge reads
    /// 1500 px/s against a 500 px/s tumble threshold), so plain top speed is
    /// not a launch.
    top_speed: f32,
    tumble_speed: f32,
    /// Ticks within a body-width or two of the nearest opponent. "Moves thrown"
    /// cannot tell a quiet match from one where nobody was in range.
    in_range: usize,
    /// Times this body's percent fell back to zero from a live reading: a KO,
    /// seen at the one edge that survives the body being replaced.
    kos: usize,
    /// Every move start, by id. The decision histogram says what the brain
    /// pressed; this says what the body threw. They differ where the runtime
    /// takes a cancel window's nomination, which is where chains live.
    started: std::collections::BTreeMap<String, usize>,
    /// Every whiff, by id — the moves of `started` that ended untouched. Beside
    /// `started` it says WHICH move the brain throws into air; a whiff total
    /// alone cannot tell an admission that leads wrong from a move with no
    /// reach.
    whiffed: std::collections::BTreeMap<String, usize>,
    /// The side gap to the nearest opponent on the tick each move STARTED,
    /// by id, split by how the move ended: `(hits, whiffs)`. A whiff that
    /// started far and a hit that started close say the admission's lead is
    /// what missed; both starting close say the move or the foe's reaction.
    start_gaps: std::collections::BTreeMap<String, (Vec<f32>, Vec<f32>)>,
    /// For each whiff, by id: the nearest the foe's hurtbox came to a live
    /// hitbox over the move's active ticks, `(along, across)` in the move's
    /// frame (+along beyond the box, −along short of it; −across above). The
    /// axis that is not zero names what missed: reach, height, or facing.
    whiff_miss: std::collections::BTreeMap<String, Vec<(f32, f32)>>,
    /// The same nearest approach for the moves that TOUCHED: the control
    /// for `whiff_miss`. A hit reads `(0, 0)` when the frame is right; a
    /// whiff that also reads `(0, 0)` overlapped and was refused a verdict
    /// (intangible foe, clank, a hit the road declined), not missed.
    hit_miss: std::collections::BTreeMap<String, Vec<(f32, f32)>>,
    /// Whiffs that showed no active tick with a foe present at all (the
    /// move was cancelled or interrupted before its hitbox opened).
    whiff_blind: std::collections::BTreeMap<String, usize>,
    /// Launches handed to this body: rising edges of hitstun, the same edge
    /// `top_speed` is sampled on. Separates one big hit from forty.
    launches: usize,
    /// Ticks inside the hard control lock at the start of a launch
    /// (`BodyCombat::recoil_lock_timer`), which presentation reads as the
    /// launch beat. Beside `launches` it gives the beat length in practice;
    /// `0` means the beat is inert.
    beat_ticks: usize,
    /// The speed a launched body flies at, one sample per tick of involuntary
    /// flight (`hitstun > 0 || tumbling`, the predicate `LaunchedBodiesView`
    /// publishes).
    ///
    /// Not `top_speed`, which is the speed when the launch was written.
    /// Gravity keeps working in flight. Presentation gates its launch cues on
    /// this distribution, so fit such thresholds to it.
    flight_speeds: Vec<f32>,
    /// Ticks this body spent held by somebody. A refused grab and a grab never
    /// attempted look the same in the move table.
    held: usize,
}

#[derive(clap::Args, Debug)]
pub struct MatchReportArgs {
    /// Seconds of match to simulate.
    #[arg(default_value_t = 30)]
    pub seconds: usize,
    /// Which fighter to report on.
    #[arg(default_value_t = ambition_demo_smash::SMASH_GEORGE_BOOUL.to_string())]
    pub character: String,
    /// Seat 1's fighter. A mirror of `character` when omitted: the mirror is
    /// the symmetry instrument, a pair is the matchup one.
    #[arg(long, value_name = "ID")]
    pub opponent: Option<String>,
    /// The ladder rung both seats play at (1..=9).
    #[arg(long, default_value_t = 5)]
    pub level: u8,
    /// How many runs to average over. Zero is treated as one, as it always was.
    #[arg(long, default_value_t = 1)]
    pub runs: usize,
    /// Load a difficulty ladder from a `.ron` and play it in place of the
    /// rows of Smash's pack (`data/fighter_brain_ladder.ron`).
    #[arg(long, value_name = "PATH")]
    pub ladder: Option<String>,
    /// Also trace the match camera: the view's size and where the fighters
    /// sit in it, every tick, reported as a block after the fighter table.
    #[arg(long)]
    pub camera: bool,
}

pub fn run(args: MatchReportArgs) {
    run_with(args, build_demo_app)
}

/// [`run`] on an app `build` composes. The demo shell seats George and the two
/// stand-ins; the SHIPPED composition (`ambition_app_tools`'s
/// `smash_cpu_report`) seats the whole roster, the polygons among them.
pub fn run_with(args: MatchReportArgs, build: fn() -> App) {
    let seconds: usize = args.seconds;
    let character = args.character;
    let opponent = args.opponent.unwrap_or_else(|| character.clone());
    let label = format!("{character} vs {opponent}");
    let level = args.level.clamp(1, 9);
    let runs = if args.runs > 0 { args.runs } else { 1 };

    #[cfg(feature = "causal")]
    let mut decisions = DecisionTally::new();
    let mut carried: Vec<String> = Vec::new();
    // Parse once: a parse failure is a caller error and must stop before any
    // match runs.
    let authored_ladder = args.ladder.as_deref().map(|path| {
        let text = std::fs::read_to_string(path).unwrap_or_else(|err| {
            eprintln!("[match_report] --ladder {path}: {err}");
            std::process::exit(2);
        });
        let ladder =
            ambition_platformer2d::characters::brain::fighter::FighterBrainLadder::from_ron(&text)
                .unwrap_or_else(|err| {
                    eprintln!("[match_report] --ladder {path} did not parse: {err}");
                    std::process::exit(2);
                });
        ladder
    });
    // Name the ladder before the first number. Do it here, not in a report
    // function: `runs == 1` and `runs > 1` take different report paths.
    println!(
        "match_report: ladder: {}",
        if authored_ladder.is_some() {
            "the rows of the --ladder file"
        } else {
            "the rows of Smash's pack (`data/fighter_brain_ladder.ron`)"
        }
    );
    let mut cameras: Vec<CameraTally> = Vec::new();
    let all: Vec<Vec<Tally>> = (0..runs)
        .map(|i| {
            let (tallies, camera) = run_one(
                build,
                &character,
                &opponent,
                seconds,
                level,
                0x5F37_7A11_u64.wrapping_mul(i as u64 + 1),
                #[cfg(feature = "causal")]
                &mut decisions,
                &mut carried,
                authored_ladder.as_ref(),
                args.camera,
            );
            cameras.push(camera);
            tallies
        })
        .collect();

    // An empty report is unreadable. A character this composition does not
    // carry seats no fighter, and every tally stays zero, which looks like a
    // quiet fight. This binary runs the demo shell, not the full app.
    if all.iter().all(|run| {
        run.iter()
            .all(|tally| tally.damage == 0 && tally.moves_started == 0)
    }) {
        eprintln!(
            "match_report: nothing was seated for '{character}'. This binary composes the \
             SMASH DEMO shell, and the ids its catalog ACTUALLY carries are: {}. A character \
             the composition does not have seats no fighter brain, so every column would be \
             zero.",
            if carried.is_empty() {
                "NONE — the roster resource resolved empty".to_string()
            } else {
                carried
                    .iter()
                    .map(|id| format!("`{id}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        );
        std::process::exit(2);
    }
    if runs == 1 {
        report_one(&label, seconds, &all[0], &carried);
    } else {
        report_spread(&label, seconds, &all, &carried);
    }
    if args.camera {
        for camera in &cameras {
            report_camera(camera);
        }
    }
    #[cfg(feature = "causal")]
    report_decisions(&decisions);
}

/// Every `(situation, verb)` the brain chose, counted. Empty without the causal
/// feature, which is why the printer is behind the same gate.
#[cfg(feature = "causal")]
type DecisionTally = std::collections::BTreeMap<(String, String), usize>;

/// Count this tick's fighter decisions off the causal log.
///
/// Reads the fact, not the `AMBITION_FIGHTER_TRACE=1` prose:
/// `first("fighter_decision").get("chose")` is a field lookup that wording
/// changes cannot break.
#[cfg(feature = "causal")]
fn collect_decisions(app: &App, into: &mut DecisionTally) {
    let Some(log) = app
        .world()
        .get_resource::<ambition_platformer2d::causal::CausalRecording>()
    else {
        return;
    };
    let Some(stamped) = log.tick() else { return };
    for subject in log.subjects_on(stamped) {
        let explanation = log.explain(stamped, &subject);
        let Some(decided) = explanation.first("fighter_decision") else {
            continue;
        };
        let situation = decided
            .get("situation")
            .map(|value| format!("{value}"))
            .unwrap_or_else(|| "?".to_string());
        let chose = decided
            .get("chose")
            .map(|value| format!("{value}"))
            .unwrap_or_else(|| "?".to_string());
        *into
            .entry((situation.clone(), format!("move {chose}")))
            .or_default() += 1;
        // The attack is a second decision, so count it too. `"none"` is a real
        // answer here, not a move called none.
        let attack = decided
            .get("attack")
            .map(|value| format!("{value}"))
            .unwrap_or_else(|| "?".to_string());
        if attack != "none" {
            *into
                .entry((situation, format!("attack {attack}")))
                .or_default() += 1;
        }
    }
}

#[cfg(feature = "causal")]
fn report_decisions(decisions: &DecisionTally) {
    if decisions.is_empty() {
        println!(
            "\nno fighter decisions were recorded — the causal feature is on but \
             nothing published, which is a defect in the recording rather than a \
             quiet fight"
        );
        return;
    }
    println!("\nwhat the brain decided, by the question it was answering:");
    let total: usize = decisions.values().sum();
    let mut rows: Vec<_> = decisions.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for ((situation, chose), count) in rows {
        println!(
            "  {:<14} {:<28} {:>6}  {:>5.1}%",
            situation,
            chose,
            count,
            100.0 * *count as f32 / total as f32
        );
    }
    println!("  {:<14} {:<28} {:>6}", "", "total", total);
}

/// One match, under one execution-noise stream.
fn run_one(
    build: fn() -> App,
    character: &str,
    opponent: &str,
    seconds: usize,
    level: u8,
    noise_seed: u64,
    #[cfg(feature = "causal")] decisions: &mut DecisionTally,
    // Out-param: the header must measure the composition, and this is where
    // a built app is in hand. Filled on every run.
    carried: &mut Vec<String>,
    authored_ladder: Option<&ambition_platformer2d::characters::brain::fighter::FighterBrainLadder>,
    trace_camera: bool,
) -> (Vec<Tally>, CameraTally) {
    let mut app = build();
    // ⛔⛔ ONE `update()` IS ONE SIM TICK, WHATEVER COMPOSITION BUILT THE APP.
    // The demo builder pins this itself; the shipped composition
    // (`smash_cpu_report`) arrived with `TimeUpdateStrategy::Automatic`, so
    // the fixed clock ran off the WALL clock and a 60 s window held as many
    // sim ticks as the machine's load allowed — three concurrent reports of
    // one binary gave different medians (rung 5 pair: moves 24 vs 28, whiffs
    // 17 vs 25). Pinned here, the window is 3,600 ticks whatever the load.
    // ⚠ A SEED STILL DIVERGES ACROSS INVOCATIONS: a duplicate `--runs 3`
    // of one binary at rung 5 reproduced run 0 exactly (579 live ticks, 8
    // moves, 7 whiffs) and not runs 1-2 (medians moves 28 vs 30, whiffs 25
    // vs 23). The simulation is not invocation-deterministic yet; that is
    // a kernel question, and the medians rule stands until it is answered.
    let timestep = app
        .world()
        .resource::<bevy::time::Time<bevy::time::Fixed>>()
        .timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    if let Some(ladder) = authored_ladder {
        super::declare_smash_ladder(&mut app, ladder.clone());
    }
    #[cfg(feature = "causal")]
    {
        app.add_plugins(ambition_platformer2d::causal::CausalPlugin);
        ambition_platformer2d::causal::record_domains(
            &mut app,
            ambition_platformer2d::causal::RecordingPolicy::only([
                ambition_platformer2d::causal::domains::BRAIN,
            ]),
        );
    }
    for _ in 0..30 {
        app.update();
    }
    // Resolved at `Startup` from the ids the assembled catalog carries. Absent
    // only if the demo's own plugin did not run; the header then says so.
    *carried = app
        .world()
        .get_resource::<ambition_demo_smash::select::SmashRoster>()
        .map(|roster| roster.0.clone())
        .unwrap_or_default();
    // Both seats are CPUs. `SmashSelect::roster` makes every locked seat
    // human, and nobody presses anything here.
    let characters = [character, opponent];
    let roster = ambition_demo_smash::smash_roster_at_levels(characters, &[level, level]);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // Wait past the 3-2-1-GO: fighters are held for the whole ceremony. Read
    // the count from the ruleset.
    let countdown = ambition_demo_smash::smash_roster(characters)
        .rules
        .opening_countdown_ticks;
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }

    // Force the stream: the point is the spread across streams (as
    // `ladder_probe` does).
    {
        use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
        let world = app.world_mut();
        let mut q = world.query::<&mut Brain>();
        for (index, mut brain) in q.iter_mut(world).enumerate() {
            if let Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) = &mut *brain {
                state.noise = noise_seed.wrapping_mul(index as u64 + 1).wrapping_add(1);
            }
        }
    }
    let ticks = seconds * 60;
    let mut totals: Vec<Tally> = vec![Tally::default(); 4];
    let mut live_move: Vec<Option<(String, f32, bool, bool, f32, Option<(f32, f32)>)>> =
        vec![None; 4];
    let mut parry_was: Vec<f32> = vec![0.0; 4];
    let mut hitstun_was: Vec<f32> = vec![0.0; 4];
    let mut last_damage: Vec<i32> = vec![0; 4];
    let mut steer_was: Vec<f32> = vec![0.0; 4];
    let mut last_world: Vec<(usize, f32, f32, Option<f32>)> = Vec::new();
    let mut camera = CameraTally {
        room: ambition_demo_smash::smash_stage().world.size,
        ..CameraTally::default()
    };
    for _ in 0..ticks {
        app.update();
        if trace_camera {
            sample_camera(&mut app, &mut camera);
        }
        sample(
            &mut app,
            &mut totals,
            &mut live_move,
            &mut parry_was,
            &mut hitstun_was,
            &mut last_damage,
            &mut steer_was,
            &mut last_world,
        );
        #[cfg(feature = "causal")]
        collect_decisions(&app, decisions);
    }

    (totals, camera)
}

/// THE CAMERA TRACE: one row per tick of the view the match camera resolved
/// and where the seated fighters sat in it. `report_camera` reduces it.
#[derive(Default)]
struct CameraTally {
    /// Per tick: the visible view (w, h) and its centre, in world px.
    frames: Vec<(ae::Vec2, ae::Vec2)>,
    /// Per tick: the view the cast asked for (base) and the view requested
    /// after zoom, so a cut-off can be told apart from a lagging zoom.
    asked: Vec<(ae::Vec2, ae::Vec2)>,
    /// Per tick, per seat: the body's centre and size, in world px.
    bodies: Vec<Vec<(ae::Vec2, ae::Vec2)>>,
    /// The room's size, read once.
    room: ae::Vec2,
    /// The match's camera box, in world px: the room grown by its
    /// `cast_framing.bounds`, or the room itself. A body past it is framed
    /// at the box and leaves the view by design.
    camera_box: Option<(ae::Vec2, ae::Vec2)>,
}

fn sample_camera(app: &mut App, camera: &mut CameraTally) {
    let world = app.world_mut();
    let mut views = world.query_filtered::<
        &ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot,
        With<ambition_platformer2d::sim_view::LocalView>,
    >();
    let Some(frame) = views.iter(world).find_map(|view| view.frame().cloned()) else {
        return;
    };
    camera
        .frames
        .push((frame.snapshot.visible_view, frame.snapshot.center_world));
    camera
        .asked
        .push((frame.snapshot.base_view, frame.snapshot.requested_view));
    if camera.camera_box.is_none() {
        camera.camera_box = world
            .get_resource::<ambition_platformer2d::platformer::markers::CastFramingTuning>()
            .and_then(|tuning| tuning.bounds)
            .map(|bounds| {
                (
                    ae::Vec2::new(-bounds.side, -bounds.rise),
                    camera.room + ae::Vec2::new(bounds.side, bounds.fall),
                )
            });
    }
    let mut seats = world.query::<(&MatchSeat, &ae::BodyKinematics)>();
    let mut bodies: Vec<(usize, ae::Vec2, ae::Vec2)> = seats
        .iter(world)
        .map(|(seat, kin)| (seat.0, kin.pos, kin.size))
        .collect();
    bodies.sort_by_key(|(seat, ..)| *seat);
    camera
        .bodies
        .push(bodies.into_iter().map(|(_, pos, size)| (pos, size)).collect());
}

fn median(values: &mut [f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    values[values.len() / 2]
}

/// Reduce the trace to what a camera feel is judged by: how big the view is,
/// how fast it zooms, how far past the room it goes, how tall a fighter is
/// on screen, where the fighters sit vertically, and how often one is cut
/// off by the view's edge.
fn report_camera(camera: &CameraTally) {
    let ticks = camera.frames.len();
    if ticks == 0 {
        println!("camera: no view was resolved (headless app without a local view?)");
        return;
    }
    let mut widths: Vec<f32> = camera.frames.iter().map(|(view, _)| view.x).collect();
    let (w_min, w_max) = widths
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), w| (lo.min(*w), hi.max(*w)));
    let w_med = median(&mut widths);
    let mut open_rate = 0.0f32;
    let mut close_rate = 0.0f32;
    let mut moving_ticks = 0usize;
    let mut open_event = String::new();
    for (tick, pair) in camera.frames.windows(2).enumerate() {
        let d = (pair[1].0.x - pair[0].0.x) * 60.0;
        if d > open_rate {
            open_event = format!(
                "    fastest open at t{}: {:.0} -> {:.0} px wide; bodies {:?}",
                tick + 1,
                pair[0].0.x,
                pair[1].0.x,
                camera.bodies.get(tick + 1).map(|bodies| bodies
                    .iter()
                    .map(|(p, _)| (p.x.round(), p.y.round()))
                    .collect::<Vec<_>>())
            );
        }
        open_rate = open_rate.max(d);
        close_rate = close_rate.min(d);
        if d.abs() > 60.0 {
            moving_ticks += 1;
        }
    }
    let past_room = camera
        .frames
        .iter()
        .filter(|(view, centre)| {
            centre.x - view.x * 0.5 < -0.5
                || centre.x + view.x * 0.5 > camera.room.x + 0.5
                || centre.y - view.y * 0.5 < -0.5
                || centre.y + view.y * 0.5 > camera.room.y + 0.5
        })
        .count();
    let mut heights: Vec<f32> = Vec::new();
    let mut seats_in_frame: Vec<f32> = Vec::new();
    let mut cut_off = 0usize;
    let mut cut_off_past_box = 0usize;
    let mut cut_off_events: Vec<String> = Vec::new();
    let mut body_ticks = 0usize;
    let (box_min, box_max) = camera
        .camera_box
        .unwrap_or((ae::Vec2::ZERO, camera.room));
    for (tick, ((view, centre), bodies)) in camera.frames.iter().zip(&camera.bodies).enumerate() {
        let (base, requested) = camera.asked.get(tick).copied().unwrap_or_default();
        for (pos, size) in bodies {
            body_ticks += 1;
            let past_box = pos.x < box_min.x || pos.x > box_max.x || pos.y < box_min.y || pos.y > box_max.y;
            heights.push(size.y / view.y);
            seats_in_frame.push((pos.y - (centre.y - view.y * 0.5)) / view.y);
            // A body ON the view's edge is in frame: a view floored at the
            // cast's span puts a launched body exactly there, and half a
            // pixel of float is not a cut-off anyone can see.
            let half = *size * 0.5 - ae::Vec2::splat(1.0);
            if pos.x - half.x < centre.x - view.x * 0.5
                || pos.x + half.x > centre.x + view.x * 0.5
                || pos.y - half.y < centre.y - view.y * 0.5
                || pos.y + half.y > centre.y + view.y * 0.5
            {
                if past_box {
                    cut_off_past_box += 1;
                } else {
                    cut_off += 1;
                    if cut_off_events.len() < 6 {
                        cut_off_events.push(format!(
                            "    t{tick} body at ({:.0},{:.0}) size {:.0}x{:.0} outside a {:.0}x{:.0} view centred ({:.0},{:.0}); base {:.0}x{:.0} requested {:.0}x{:.0}; bodies {:?}",
                            pos.x, pos.y, size.x, size.y, view.x, view.y, centre.x, centre.y, base.x, base.y, requested.x, requested.y,
                            bodies.iter().map(|(p, _)| (p.x.round(), p.y.round())).collect::<Vec<_>>()
                        ));
                    }
                }
            }
        }
    }
    let (h_min, h_max) = heights
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), h| (lo.min(*h), hi.max(*h)));
    let h_med = median(&mut heights);
    let sit_med = median(&mut seats_in_frame);
    println!(
        "\ncamera: {ticks} ticks, room {:.0}x{:.0}",
        camera.room.x, camera.room.y
    );
    println!("  view width px     min {w_min:.0}  median {w_med:.0}  max {w_max:.0}");
    println!(
        "  zoom rate px/s    open {open_rate:.0}  close {close_rate:.0}  ticks moving >60px/s {moving_ticks} ({:.0}%)",
        moving_ticks as f32 * 100.0 / ticks.max(1) as f32
    );
    if !open_event.is_empty() {
        println!("{open_event}");
    }
    println!(
        "  view past room    {past_room} ticks ({:.0}%)",
        past_room as f32 * 100.0 / ticks as f32
    );
    println!(
        "  fighter height    min {:.2}  median {:.2}  max {:.2} of the view",
        h_min, h_med, h_max
    );
    println!("  fighter sits at   {sit_med:.2} of the view height (0 = top)");
    println!(
        "  fighter cut off   {cut_off} of {body_ticks} body-ticks ({:.1}%) inside the camera box; {cut_off_past_box} past it (by design)",
        cut_off as f32 * 100.0 / body_ticks.max(1) as f32
    );
    for event in &cut_off_events {
        println!("{event}");
    }
}

fn report_one(label: &str, seconds: usize, totals: &[Tally], carried: &[String]) {
    println!("match_report: {label}, {seconds}s of CPU-versus-CPU");
    println!("{}\n", composition_scope(carried));
    println!(
        "{:<6} {:>7} {:>7} {:>7} {:>8} {:>8} {:>7} {:>8} {:>7} {:>7} {:>7} {:>7} {:>7} {:>8} {:>8}",
        "seat",
        "damage",
        "moves",
        "whiffs",
        "hitstun",
        "tumbling",
        "downed",
        "evading",
        "unhit",
        "shield",
        "parries",
        "techs",
        "charge",
        "launch",
        "tumble@",
    );
    for (seat, tally) in totals.iter().enumerate() {
        if tally.damage == 0 && tally.moves_started == 0 {
            continue;
        }
        println!(
            "{:<6} {:>7} {:>7} {:>7} {:>8} {:>8} {:>7} {:>8} {:>7} {:>7} {:>7} {:>7} {:>6.2} {:>8.0} {:>8.0}",
            seat,
            tally.damage,
            tally.moves_started,
            tally.whiffs,
            tally.hitstun,
            tally.tumbling,
            tally.knocked_down,
            tally.evading,
            tally.unhittable,
            tally.shielding,
            tally.parries_caught,
            tally.tech_armed,
            tally.best_charge,
            tally.top_speed,
            tally.tumble_speed,
        );
    }
    report_thrown(totals.iter());
    // How often a fighter changes its walking direction. See
    // `Tally::steer_flips`.
    println!("\nhow often each body changes its walking direction:");
    println!(
        "{:<6} {:>12} {:>12} {:>18}",
        "seat", "steered", "flips", "ticks per flip"
    );
    for (seat, tally) in totals.iter().enumerate() {
        if tally.steer_held == 0 {
            continue;
        }
        println!(
            "{:<6} {:>12} {:>12} {:>18.1}",
            seat,
            tally.steer_held,
            tally.steer_flips,
            tally.steer_held as f32 / tally.steer_flips.max(1) as f32
        );
    }
    println!(
        "\nlive ticks: {} of {} (a KO ends the round and freezes the rest of the window)",
        totals.iter().map(|t| t.live_ticks).max().unwrap_or(0),
        seconds * 60
    );
    println!("\nwhy each body could not be struck, by the term that refused:");
    println!(
        "{:<6} {:>10} {:>10} {:>10} {:>14} {:>10}",
        "seat", "invuln", "evading", "of-ledge", "parry-window", "i-frames"
    );
    for (seat, tally) in totals.iter().enumerate() {
        if tally.unhittable == 0 {
            continue;
        }
        println!(
            "{:<6} {:>10} {:>10} {:>10} {:>14} {:>10}",
            seat,
            tally.unhit_invuln,
            tally.unhit_evading,
            // A share of the column before it: these do not sum to `unhittable`.
            tally.unhit_ledge,
            tally.unhit_parry_window,
            tally.unhit_iframes
        );
    }
    println!(
        "\nticks are counts of SAMPLED TICKS in that state; damage is final percent, \
         parries and techs are events, charge is the best fraction reached."
    );
    // Flag a match where nobody was launched, however much damage it has.
    if totals.iter().all(|t| t.tumbling == 0) {
        println!(
            "\n⚠ NOBODY TUMBLED. Hits are landing and nothing is being launched — \
             check the tumble threshold against the launches actually resolved."
        );
    }
    if totals.iter().all(|t| t.best_charge <= 0.0) {
        println!("\n⚠ NOBODY CHARGED A SMASH. The multiplier is authored and unpaid.");
    }
}

fn sample(
    app: &mut App,
    totals: &mut [Tally],
    live_move: &mut [Option<(String, f32, bool, bool, f32, Option<(f32, f32)>)>],
    parry_was: &mut [f32],
    hitstun_was: &mut [f32],
    last_damage: &mut [i32],
    steer_was: &mut [f32],
    last_world: &mut Vec<(usize, f32, f32, Option<f32>)>,
) {
    // The steer lives on the control frame, not the body, so sample it in its
    // own pass. Count sign changes: the initial dash is paid per change.
    {
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &ambition_platformer2d::characters::control::ActorControl,
        )>();
        let steers: Vec<(usize, f32)> = q
            .iter(world)
            .map(|(seat, control)| (seat.0, control.0.locomotion.vec().x))
            .collect();
        for (seat, x) in steers {
            let Some(slot) = totals.get_mut(seat) else {
                continue;
            };
            let dir = if x.abs() > 0.5 { x.signum() } else { 0.0 };
            if dir != 0.0 {
                slot.steer_held += 1;
                if steer_was[seat] != 0.0 && dir != steer_was[seat] {
                    slot.steer_flips += 1;
                }
                steer_was[seat] = dir;
            }
        }
    }
    let world = app.world_mut();
    let mut q = world.query::<(
        &MatchSeat,
        &BodyHealth,
        &BodyCombat,
        &ae::BodyKinematics,
        Option<&MovePlayback>,
        Option<&ae::BodyMotionFacts>,
        Option<&ae::BodyShieldState>,
        Option<&ambition_platformer2d::actor::MotionModel>,
        Option<&ambition_platformer2d::combat::capture::CapturedBy>,
    )>();
    let rows: Vec<_> = q
        .iter(world)
        .map(
            |(seat, health, combat, kin, playback, facts, shield, model, captured)| {
                (
                    seat.0,
                    health.damage_taken(),
                    // Ask the damage rule's own answer; do not reconstruct eligibility.
                    (
                        health.health.invulnerable.any(),
                        facts.is_some_and(|f| f.evading()),
                        // The ledge's own intangibility.
                        facts.is_some_and(|f| f.ledge_intangible),
                        shield.is_some_and(|s| s.parrying()),
                        !combat.vulnerable(),
                    ),
                    combat.hitstun_timer,
                    kin.vel.length(),
                    match model {
                        Some(ae::MotionModel::AxisSwept(axis)) => {
                            axis.params.abilities.tumble_speed
                        }
                        _ => 0.0,
                    },
                    playback.map(|p| {
                        // The hit boxes live at this tick, body-local, facing +x.
                        let boxes: Vec<((f32, f32), (f32, f32))> = p
                            .spec
                            .windows
                            .iter()
                            .filter(|w| {
                                matches!(w.tag, ambition_platformer2d::entity_catalog::WindowTag::Active)
                                    && w.start_s <= p.t
                                    && p.t < w.end_s
                            })
                            .flat_map(|w| {
                                w.volumes.iter().map(|v| {
                                    let c = v.shape.coverage_box();
                                    (c.min, c.max)
                                })
                            })
                            .collect();
                        (
                            p.spec.id.clone(),
                            p.t,
                            p.smash_charge_fraction(),
                            p.contact().overlapped,
                            boxes,
                            p.facing,
                        )
                    }),
                    facts.copied(),
                    shield.copied(),
                    match model {
                        Some(ae::MotionModel::AxisSwept(axis)) => Some(axis.state.tech_press_timer),
                        _ => None,
                    },
                    kin.pos.x,
                    captured.is_some(),
                    combat.recoil_lock_timer,
                    kin.pos.y,
                    kin.size * 0.5,
                )
            },
        )
        .collect();
    // Was anybody in range? Only the distance between the bodies separates a
    // quiet match from a busy one.
    const IN_RANGE_PX: f32 = 120.0;
    let positions: Vec<(usize, f32, f32, ae::Vec2)> =
        rows.iter().map(|row| (row.0, row.10, row.13, row.14)).collect();
    // A frozen simulation (the window outliving the match) changes nothing.
    let world_now: Vec<(usize, f32, f32, Option<f32>)> = rows
        .iter()
        .map(|row| (row.0, row.10, row.13, row.6.as_ref().map(|p| p.1)))
        .collect();
    let live = *last_world != world_now;
    *last_world = world_now;
    // Who could not be struck this tick: the four terms `unhittable` counts
    // and the ledge's intangibility, so a whiff can say whether its foe was
    // there to be hit.
    let unhittable_now: Vec<(usize, bool)> = rows
        .iter()
        .map(|row| (row.0, row.2 .0 || row.2 .1 || row.2 .2 || row.2 .3 || row.2 .4))
        .collect();
    for (
        seat,
        damage,
        vulnerable,
        hitstun,
        speed,
        tumble_speed,
        playback,
        facts,
        shield,
        tech_timer,
        here,
        captured,
        recoil_lock,
        pos_y,
        _half,
    ) in rows
    {
        let Some(tally) = totals.get_mut(seat) else {
            continue;
        };
        tally.live_ticks += usize::from(live);
        if damage == 0 && last_damage[seat] > 0 {
            tally.kos += 1;
        }
        last_damage[seat] = damage;
        tally.damage = tally.damage.max(damage);
        if captured {
            tally.held += 1;
        }
        if positions
            .iter()
            .any(|(other, x, _, _)| *other != seat && (x - here).abs() <= IN_RANGE_PX)
        {
            tally.in_range += 1;
        }
        // Sample on the rising edge of hitstun, the tick the launch was written.
        // Later ticks include gravity's work.
        if hitstun > 0.0 && hitstun_was[seat] <= 0.0 {
            tally.top_speed = tally.top_speed.max(speed);
            tally.launches += 1;
        }
        if hitstun > 0.0 && recoil_lock > 0.0 {
            tally.beat_ticks += 1;
        }
        if hitstun > 0.0 || facts.is_some_and(|f| f.tumbling) {
            tally.flight_speeds.push(speed);
        }
        hitstun_was[seat] = hitstun;
        tally.tumble_speed = tumble_speed;
        if hitstun > 0.0 {
            tally.hitstun += 1;
        }
        if let Some(facts) = facts {
            if facts.tumbling {
                tally.tumbling += 1;
            }
            if facts.knocked_down {
                tally.knocked_down += 1;
            }
            if facts.evading() {
                tally.evading += 1;
            }
        }
        let (invuln, evading, ledge, parry_window, iframes) = vulnerable;
        if invuln || evading || parry_window || iframes {
            tally.unhittable += 1;
        }
        if invuln {
            tally.unhit_invuln += 1;
        }
        if evading {
            tally.unhit_evading += 1;
        }
        if ledge {
            tally.unhit_ledge += 1;
        }
        if parry_window {
            tally.unhit_parry_window += 1;
        }
        if iframes {
            tally.unhit_iframes += 1;
        }
        if let Some(shield) = shield {
            if shield.active {
                tally.shielding += 1;
            }
            // An event, not a state: count the tick the timer rises.
            if shield.parry_caught_timer > parry_was[seat] {
                tally.parries_caught += 1;
            }
            parry_was[seat] = shield.parry_caught_timer;
        }
        if tech_timer.is_some_and(|t| t > 0.0) {
            tally.tech_armed += 1;
        }
        let foe_unhittable = unhittable_now
            .iter()
            .any(|(other, unhittable)| *other != seat && *unhittable);
        let nearest_foe = positions
            .iter()
            .filter(|(other, _, _, _)| *other != seat)
            .min_by(|a, b| (a.1 - here).abs().total_cmp(&(b.1 - here).abs()));
        let nearest_gap = nearest_foe.map_or(f32::INFINITY, |foe| (foe.1 - here).abs());
        if let Some((id, t, charge, overlapped, boxes, move_facing)) = playback {
            let fresh = match &live_move[seat] {
                Some((last_id, last_t, _, _, _, _)) => last_id != &id || t < *last_t,
                None => true,
            };
            if fresh {
                // The move before this one ended; a whiff if it never touched.
                if let Some((last_id, _, touched, evaded, gap, miss)) = &live_move[seat] {
                    tally.end_move(last_id, *touched, *evaded, *gap, *miss);
                }
                tally.moves_started += 1;
                *tally.started.entry(id.clone()).or_default() += 1;
            }
            let carried = (!fresh).then(|| live_move[seat].as_ref()).flatten();
            let touched = overlapped || carried.is_some_and(|m| m.2);
            let evaded = foe_unhittable || carried.is_some_and(|m| m.3);
            let gap = carried.map_or(nearest_gap, |m| m.4);
            // THE NEAREST APPROACH of the foe's hurtbox to any live hitbox,
            // over the move's active ticks, in the move's own frame: along
            // (+ beyond the box, − short of it or behind) and across (+
            // below, − above). The smallest total over the move is kept.
            let mut miss = carried.and_then(|m| m.5);
            if let Some((_, fx, fy, fhalf)) = nearest_foe {
                let sign = if move_facing < 0.0 { -1.0 } else { 1.0 };
                let rel = ((fx - here) * sign, fy - pos_y);
                for ((lo_x, lo_y), (hi_x, hi_y)) in &boxes {
                    let axis = |v: f32, lo: f32, hi: f32, h: f32| -> f32 {
                        if v < lo - h {
                            v - (lo - h)
                        } else if v > hi + h {
                            v - (hi + h)
                        } else {
                            0.0
                        }
                    };
                    let m = (axis(rel.0, *lo_x, *hi_x, fhalf.x), axis(rel.1, *lo_y, *hi_y, fhalf.y));
                    let total = m.0.abs() + m.1.abs();
                    if miss.is_none_or(|best: (f32, f32)| total < best.0.abs() + best.1.abs()) {
                        miss = Some(m);
                    }
                }
            }
            live_move[seat] = Some((id, t, touched, evaded, gap, miss));
            if let Some(fraction) = charge {
                tally.charge_held += 1;
                tally.best_charge = tally.best_charge.max(fraction);
            }
        } else {
            if let Some((last_id, _, touched, evaded, gap, miss)) = &live_move[seat] {
                tally.end_move(last_id, *touched, *evaded, *gap, *miss);
            }
            live_move[seat] = None;
        }
    }
}

/// The composition this measurement is of, printed on every run.
///
/// This binary composes the smash demo shell, whose catalog is a fraction
/// of the full app's. A change can look fine here and regress only in the
/// full app, for characters this shell cannot seat. The list comes from
/// `SmashRoster`, which is resolved at `Startup` from the catalog, so it
/// cannot go stale.
fn composition_scope(carried: &[String]) -> String {
    let names = if carried.is_empty() {
        "NOTHING — the catalog resolved empty".to_string()
    } else {
        carried
            .iter()
            .map(|id| format!("`{id}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "⚠ composition: the SMASH DEMO SHELL, which carries {} id(s) — {names}. The shipped \
         app's grid may be larger, and a number taken here is a claim about THIS composition \
         only - `app_it` is where the full roster lives.",
        carried.len(),
    )
}

impl Tally {
    /// A move ended: file it as a hit or a whiff, with the gap it started at.
    fn end_move(
        &mut self,
        id: &str,
        touched: bool,
        evaded: bool,
        start_gap: f32,
        miss: Option<(f32, f32)>,
    ) {
        let gaps = self.start_gaps.entry(id.to_string()).or_default();
        if touched {
            gaps.0.push(start_gap);
            if let Some(m) = miss {
                self.hit_miss.entry(id.to_string()).or_default().push(m);
            }
        } else {
            self.whiffs += 1;
            self.whiffs_evaded += usize::from(evaded);
            *self.whiffed.entry(id.to_string()).or_default() += 1;
            gaps.1.push(start_gap);
            match miss {
                Some(m) => self.whiff_miss.entry(id.to_string()).or_default().push(m),
                None => *self.whiff_blind.entry(id.to_string()).or_default() += 1,
            }
        }
    }
}

/// The per-move table: what the bodies threw and how much of it touched
/// nothing. Pooled over whatever tallies are handed in (one run's seats, or
/// every seat of every run), so a `--runs 3` report carries it too.
fn report_thrown<'a>(tallies: impl Iterator<Item = &'a Tally>) {
    #[derive(Default)]
    struct Row {
        thrown: usize,
        whiffed: usize,
        hit_gaps: Vec<f32>,
        whiff_gaps: Vec<f32>,
        along: Vec<f32>,
        across: Vec<f32>,
        blind: usize,
        hit_along: Vec<f32>,
        hit_across: Vec<f32>,
    }
    let mut moves: std::collections::BTreeMap<&str, Row> = std::collections::BTreeMap::new();
    for tally in tallies {
        for (id, count) in &tally.started {
            moves.entry(id.as_str()).or_default().thrown += count;
        }
        for (id, count) in &tally.whiffed {
            moves.entry(id.as_str()).or_default().whiffed += count;
        }
        for (id, (hits, whiffs)) in &tally.start_gaps {
            let row = moves.entry(id.as_str()).or_default();
            row.hit_gaps.extend_from_slice(hits);
            row.whiff_gaps.extend_from_slice(whiffs);
        }
        for (id, misses) in &tally.whiff_miss {
            let row = moves.entry(id.as_str()).or_default();
            row.along.extend(misses.iter().map(|m| m.0));
            row.across.extend(misses.iter().map(|m| m.1));
        }
        for (id, count) in &tally.whiff_blind {
            moves.entry(id.as_str()).or_default().blind += count;
        }
        for (id, misses) in &tally.hit_miss {
            let row = moves.entry(id.as_str()).or_default();
            row.hit_along.extend(misses.iter().map(|m| m.0));
            row.hit_across.extend(misses.iter().map(|m| m.1));
        }
    }
    if moves.is_empty() {
        return;
    }
    let mut rows: Vec<_> = moves.into_iter().collect();
    rows.sort_by(|a, b| b.1.thrown.cmp(&a.1.thrown).then_with(|| a.0.cmp(b.0)));
    let median = |xs: &[f32]| -> String {
        if xs.is_empty() {
            return "-".to_string();
        }
        let mut xs = xs.to_vec();
        xs.sort_by(f32::total_cmp);
        format!("{:.0}", xs[xs.len() / 2])
    };
    // Print the total first and say when rows are withheld, so a new move
    // is not dropped in silence.
    println!(
        "\nwhat the bodies actually threw: {} distinct\n  {:<28} {:>5} {:>5}   {:>5} {:>5}   {:>6} {:>6} {:>5}   {:>6} {:>6}",
        rows.len(),
        "move",
        "thrown",
        "whiff",
        "gap@h",
        "gap@w",
        "along",
        "across",
        "blind",
        "h.alng",
        "h.acrs",
    );
    for (id, row) in rows.iter().take(12) {
        println!(
            "  {id:<28} {:>5} {:>5}   {:>5} {:>5}   {:>6} {:>6} {:>5}   {:>6} {:>6}",
            row.thrown,
            row.whiffed,
            median(&row.hit_gaps),
            median(&row.whiff_gaps),
            median(&row.along),
            median(&row.across),
            row.blind,
            median(&row.hit_along),
            median(&row.hit_across),
        );
    }
    println!(
        "  gap@h / gap@w: median centre gap when a hit / a whiff started. along / across: for the \
         whiffs, the median nearest approach of the foe's hurtbox to a live hitbox in the move's \
         frame (+along beyond, −along short or behind; −across above). blind: whiffs whose hitbox \
         never opened with a foe present. h.alng / h.acrs: the same approach for the moves that \
         touched — the control; a whiff reading 0 / 0 overlapped and was refused a verdict."
    );
    if rows.len() > 12 {
        println!("  … and {} more not shown", rows.len() - 12);
    }
}

/// `min–median–max` across runs: one run is a sample of a noisy process.
fn report_spread(label: &str, seconds: usize, all: &[Vec<Tally>], carried: &[String]) {
    println!(
        "match_report: {label}, {seconds}s × {} runs, per-run TOTALS across both seats\n{}\n",
        all.len(),
        composition_scope(carried)
    );
    let spread = |pick: fn(&Tally) -> f32| -> String {
        let mut values: Vec<f32> = all
            .iter()
            .map(|run| run.iter().map(pick).sum::<f32>())
            .collect();
        values.sort_by(f32::total_cmp);
        let median = values[values.len() / 2];
        format!(
            "{:.0}–{:.0}–{:.0}",
            values.first().copied().unwrap_or(0.0),
            median,
            values.last().copied().unwrap_or(0.0)
        )
    };
    let peak = |pick: fn(&Tally) -> f32| -> String {
        let mut values: Vec<f32> = all
            .iter()
            .map(|run| run.iter().map(pick).fold(0.0f32, f32::max))
            .collect();
        values.sort_by(f32::total_cmp);
        format!(
            "{:.2}–{:.2}–{:.2}",
            values.first().copied().unwrap_or(0.0),
            values[values.len() / 2],
            values.last().copied().unwrap_or(0.0)
        )
    };
    // Per run, not summed over seats: both seats share one clock.
    let live = {
        let mut values: Vec<f32> = all
            .iter()
            .map(|run| run.iter().map(|t| t.live_ticks as f32).fold(0.0, f32::max))
            .collect();
        values.sort_by(f32::total_cmp);
        format!(
            "{:.0}–{:.0}–{:.0}",
            values.first().copied().unwrap_or(0.0),
            values[values.len() / 2],
            values.last().copied().unwrap_or(0.0)
        )
    };
    println!("  live ticks  {live}   (a KO ends the round and freezes the rest of the window)");
    // Per live minute, per run: the only form in which two runs of different
    // length compare. Each run's rate, then the spread of the rates.
    let per_minute = |pick: fn(&Tally) -> f32| -> String {
        let mut values: Vec<f32> = all
            .iter()
            .map(|run| {
                let live = run.iter().map(|t| t.live_ticks as f32).fold(0.0, f32::max);
                run.iter().map(pick).sum::<f32>() * 3600.0 / live.max(1.0)
            })
            .collect();
        values.sort_by(f32::total_cmp);
        format!(
            "{:.1}–{:.1}–{:.1}",
            values.first().copied().unwrap_or(0.0),
            values[values.len() / 2],
            values.last().copied().unwrap_or(0.0)
        )
    };
    println!("  per live minute: moves {}  whiffs {}  damage {}",
        per_minute(|t| t.moves_started as f32),
        per_minute(|t| t.whiffs as f32),
        per_minute(|t| t.damage as f32),
    );
    for (i, run) in all.iter().enumerate() {
        println!(
            "  run {i}: live {}  moves {}  whiffs {}  damage {}  KOs {}",
            run.iter().map(|t| t.live_ticks).max().unwrap_or(0),
            run.iter().map(|t| t.moves_started).sum::<usize>(),
            run.iter().map(|t| t.whiffs).sum::<usize>(),
            run.iter().map(|t| t.damage).sum::<i32>(),
            run.iter().map(|t| t.kos).sum::<usize>(),
        );
    }
    println!("  damage      {}", spread(|t| t.damage as f32));
    println!("  moves       {}", spread(|t| t.moves_started as f32));
    println!("  whiffs      {}", spread(|t| t.whiffs as f32));
    println!("  ⇒ evaded    {}", spread(|t| t.whiffs_evaded as f32));
    println!("  hitstun     {}", spread(|t| t.hitstun as f32));
    println!("  tumbling    {}", spread(|t| t.tumbling as f32));
    println!("  downed      {}", spread(|t| t.knocked_down as f32));
    println!("  evading     {}", spread(|t| t.evading as f32));
    println!("  unhittable  {}", spread(|t| t.unhittable as f32));
    println!("  shielding   {}", spread(|t| t.shielding as f32));
    // The initial dash is paid per direction change (D217).
    println!("  steer held  {}", spread(|t| t.steer_held as f32));
    println!("  steer flips {}", spread(|t| t.steer_flips as f32));
    println!(
        "  ⇒ steered ticks per flip {}",
        spread(|t| t.steer_held as f32 / t.steer_flips.max(1) as f32)
    );
    println!("  parries     {}", spread(|t| t.parries_caught as f32));
    println!("  techs       {}", spread(|t| t.tech_armed as f32));
    println!("  in range    {}", spread(|t| t.in_range as f32));
    println!("  KOs         {}", spread(|t| t.kos as f32));
    println!("  held        {}", spread(|t| t.held as f32));
    println!("  launches    {}", spread(|t| t.launches as f32));
    println!("  launch beat {}", spread(|t| t.beat_ticks as f32));
    println!("  best charge {}", peak(|t| t.best_charge));
    println!("  peak launch {}", peak(|t| t.top_speed));
    // Pooled, not min–median–max: this is a distribution over ticks of flight,
    // and a presentation gate on flight speed picks a percentile of it.
    let mut flight: Vec<f32> = all
        .iter()
        .flat_map(|run| run.iter().flat_map(|t| t.flight_speeds.iter().copied()))
        .collect();
    flight.sort_by(f32::total_cmp);
    if !flight.is_empty() {
        let at = |q: f32| flight[((flight.len() - 1) as f32 * q) as usize];
        println!(
            "  flight speed p25 {:.0}  p50 {:.0}  p75 {:.0}  p90 {:.0}  p99 {:.0}  max {:.0}  (n={})",
            at(0.25),
            at(0.50),
            at(0.75),
            at(0.90),
            at(0.99),
            flight[flight.len() - 1],
            flight.len(),
        );
    }
    report_thrown(all.iter().flat_map(|run| run.iter()));
    println!(
        "\nmin–median–max across runs. Counts are summed over both seats; charge and \
         launch are the best either seat reached. `flight speed` is pooled over every \
         tick of involuntary flight in every run — the distribution a presentation gate \
         on flight speed actually sees.\n\n⚠ THE MATCHUP IS PART OF THE SAMPLE SIZE. \
         Every run here is one character against itself. Weight and fall speed move \
         every distribution above, so a constant fitted to this is fitted to this \
         matchup — write it down beside the sample size."
    );
}
