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
//! fighting. Read `whiffs` with its splits: `⇒ evaded` (the foe was
//! unhittable for a tick of the move); the per-move table's `inside` /
//! `along` / `across` (the whiffs' nearest approach to the authored box,
//! with the hits' as the control); and the cause histogram, which classifies
//! every tick on which one of combat's own `Hitbox` entities overlapped the
//! victim's hurt geometry and recorded no hit. ⛔ A PER-AXIS MEDIAN LIED
//! HERE ONCE: `along 0 / across 0` read as "overlapped, no verdict" when the
//! population was half short and half high; the cause histogram — zero
//! unexplained overlap ticks, pair and mirror, rung 9, three runs — is the
//! instrument that says whether a hit was refused, and `inside` counts the
//! whiffs whose box truly held the foe. The recovery specials
//! (`polygon_brawler_uppercut`, `polygon_rising_edge`) are pressed as
//! recovery routes and whiff by nature; they are a quarter to a third of
//! all whiffs (pooled over three runs: 20 of 56, 29 of 93, 28 of 112).
//!
//! DETERMINISM (measured 2026-10-30, rung 5 pair, pinned clock): a seed
//! played in a fresh process (`--runs 1 --first-run N`) reproduces the same
//! seed's run inside `--runs 3` exactly, run for run — nothing leaks between
//! runs of one process. Across invocations, seed 0 fell into TWO outcome
//! classes (4 and 4 of 8 processes: live 2551-2553 / moves 30 / damage 39,
//! and live 2636-2638 / moves 28 / damage 22), with ±2 live-tick jitter
//! inside a class. The per-tick trace (`AMBITION_MATCH_REPORT_TRACE_TICKS`)
//! showed a divergent pair to be ONE trajectory shifted by whole ticks at
//! the start: the route transition's asynchronous loading takes 8, 9 or 10
//! frames before the fighters are released, and the start tick decides the
//! match (9 frames → one outcome in 5 of 5 processes, 8 or 10 → the other
//! in 3 of 3, with the tool anchored to the live edge). The aligned traces
//! name the read: a frame with NO sim advance where the other process
//! advanced — `WorldTime::sim_dt` is `frame_dt × ClockState::time_scale`,
//! and that scale is eased per FRAME by wall dt
//! (`smooth_sim_clock_toward_target_system`) from the 0 the transition
//! holds to the 1.0 `match_live` asks for. The sim is deterministic given
//! its dt sequence; the dt sequence is not a sim fact, and the fix is in
//! the sim (a fixed-dt step; the ramp as per-tick sim state or as
//! presentation only). Not the std maps (swapped for
//! BTree: 3 classes in 8), not the entity ids (`GatedLockWallCache` is
//! initialised at build now: ids identical, outcomes still split), not
//! settle time, not wall time, not a run-to-run leak.
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
    /// Whiffs that were recovery presses: a move whose recovery route offers
    /// a way home, started off the stage. The way home misses by design;
    /// `whiffs - whiffs_recovery` is the attack whiffs.
    whiffs_recovery: usize,
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
    /// EVERY TICK on which one of this seat's live hitboxes (the `Hitbox`
    /// entities combat resolves, placed as combat places them) overlapped an
    /// opponent's hurt geometry (its published `DamageableVolumes`, else its
    /// `CenteredAabb` — the same rule as `StrikeVictim::hurt_pieces`) and no
    /// hit was recorded against that opponent, by the first rule that
    /// explains it. `already reached` is the pulse's own ledger: that
    /// opponent was hit or deflected by this pulse on an earlier tick.
    overlap_no_hit_ticks: std::collections::BTreeMap<&'static str, usize>,
    /// Per move id, per cause: the whiffs whose overlap ticks were explained
    /// by that cause (the cause seen most over the move).
    whiff_causes: std::collections::BTreeMap<String, std::collections::BTreeMap<&'static str, usize>>,
    /// Ticks on which the instrument's body box (`kin.size / 2`) overlapped a
    /// live hitbox while combat's hurt geometry did not: the geometry
    /// difference (f), if any.
    body_box_only_ticks: usize,
    /// Ticks on which this seat's playback clock was inside an Active window
    /// that authors volumes, and ticks among those on which a `Hitbox`
    /// entity owned by this seat existed. The two differ by however many
    /// ticks the authored strike was not in the world.
    active_window_ticks: usize,
    hitbox_present_ticks: usize,
    /// Ticks on which the authored box (as the instrument places it) held
    /// the foe's body box while no strike entity of this seat reached it.
    instrument_disagreements: usize,
    /// On ticks with both: the distance between the centre of the authored
    /// box as the instrument places it (spec offset, body position, move
    /// facing) and the centre of the entity's world volume, pooled.
    hitbox_offsets: Vec<f32>,
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
    /// Seat a third and fourth fighter (repeat the flag): a free-for-all.
    /// The camera instrument reads 3- and 4-fighter framing through it.
    #[arg(long = "also", value_name = "ID")]
    pub also: Vec<String>,
    /// The ladder rung both seats play at (1..=9).
    #[arg(long, default_value_t = 5)]
    pub level: u8,
    /// How many runs to average over. Zero is treated as one, as it always was.
    #[arg(long, default_value_t = 1)]
    pub runs: usize,
    /// The index of the first run, so `--runs 1 --first-run 2` in a fresh
    /// process plays exactly the seed that run 2 of `--runs 3` plays in one
    /// process. The determinism witness: the two agree, or state leaks
    /// between runs of one process.
    #[arg(long, default_value_t = 0)]
    pub first_run: usize,
    /// Load a difficulty ladder from a `.ron` and play it in place of the
    /// rows of Smash's pack (`data/fighter_brain_ladder.ron`).
    #[arg(long, value_name = "PATH")]
    pub ladder: Option<String>,
    /// Also trace the match camera: the view's size and where the fighters
    /// sit in it, every tick, reported as a block after the fighter table.
    #[arg(long)]
    pub camera: bool,
    /// Also name every rollback-registered type whose checksum changed on a
    /// frame that ran no sim tick: a per-frame writer of sim state, which a
    /// resimulated frame cannot reproduce.
    #[arg(long)]
    pub sim_writers: bool,
    /// Spawn and despawn this many empty entities as the match goes live, so
    /// the sim's next spawns take recycled ids. A sim whose outcome changes
    /// under it orders something by `Entity`.
    #[arg(long, default_value_t = 0)]
    pub recycle_ids: usize,
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
                0x5F37_7A11_u64.wrapping_mul((args.first_run + i) as u64 + 1),
                #[cfg(feature = "causal")]
                &mut decisions,
                &mut carried,
                authored_ladder.as_ref(),
                args.camera,
                args.sim_writers,
                args.recycle_ids,
                &args.also,
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
    sim_writers: bool,
    recycle_ids: usize,
    also: &[String],
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
    // THE DETERMINISM WITNESS: `AMBITION_MATCH_REPORT_SETTLE_S=<secs>` keeps
    // ticking the app for that many WALL seconds before the match is asked
    // for, so every asset the roster needs has arrived before the first sim
    // tick that could read it. Assets load on IO threads in wall time; the
    // sim clock is pinned. If fresh processes agree with this and not
    // without it, the divergence is content arriving mid-match.
    if let Some(secs) = std::env::var("AMBITION_MATCH_REPORT_SETTLE_S")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|s| *s > 0.0)
    {
        let start = std::time::Instant::now();
        let mut frames = 0usize;
        while start.elapsed().as_secs_f32() < secs {
            app.update();
            frames += 1;
        }
        eprintln!("[settle] {frames} frames over {secs} s before the match");
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
    let characters: Vec<&str> =
        [character, opponent].into_iter().chain(also.iter().map(String::as_str)).take(4).collect();
    let levels = vec![level; characters.len()];
    let roster = ambition_demo_smash::smash_roster_at_levels(characters.clone(), &levels);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // ⛔⛔ ANCHOR TO THE TICK THE MATCH GOES LIVE, NOT TO A FRAME COUNT. The
    // route transition takes a number of frames that varies per process
    // (its loading is asynchronous), so "countdown + 30 updates after GoTo"
    // landed 0-3 ticks earlier or later in the match from one process to
    // the next. Measured 2026-10-30 (rung 5 pair, seed 0, pinned clock):
    // per-tick traces of eight fresh processes were the SAME trajectory
    // shifted by whole ticks at the start, and the noise forced below then
    // seeded different match ticks, which is the whole "two outcome
    // classes" story — the simulation itself is deterministic. The fighters
    // are held for the ceremony, so the first tick a seat moves is the live
    // edge, and it is a sim fact, not a frame count.
    let countdown = ambition_demo_smash::smash_roster(characters)
        .rules
        .opening_countdown_ticks;
    let mut waited = 0usize;
    // THE RAMP AGAINST THE RELEASE: the sim clock's scale on every frame
    // from the route request to the first live tick and a little past it,
    // so "did the 0 → 1 ramp overlap live play" is read, not inferred.
    let mut scale_trace: Vec<(usize, f32, usize, bool)> = Vec::new();
    let mut release_frame: Option<usize> = None;
    let mut full_speed_frame: Option<usize> = None;
    loop {
        app.update();
        waited += 1;
        let world = app.world_mut();
        let scale = world
            .get_resource::<ambition_platformer2d::time::ClockState>()
            .map_or(f32::NAN, |clock| clock.time_scale);
        let mut seats = world.query::<(&MatchSeat, &ae::BodyKinematics)>();
        let seated = seats.iter(world).count();
        let moving = seats
            .iter(world)
            .any(|(_, kin)| kin.vel.length_squared() > 1e-6);
        scale_trace.push((waited, scale, seated, moving));
        if moving && release_frame.is_none() {
            release_frame = Some(waited);
        }
        if (scale - 1.0).abs() < 1e-4 && seated > 0 && full_speed_frame.is_none() {
            full_speed_frame = Some(waited);
        }
        let past_release = release_frame.is_some_and(|f| waited >= f + 12);
        if past_release || waited > countdown as usize + 3600 {
            break;
        }
    }
    let waited = release_frame.unwrap_or(waited);
    eprintln!(
        "[live-edge] the match went live {waited} frames after the route was asked for (opening countdown {countdown} ticks); sim clock at full speed from frame {}",
        full_speed_frame.map_or("never".to_string(), |f| f.to_string())
    );
    for (frame, scale, seated, moving) in scale_trace.iter().filter(|(f, ..)| *f + 8 >= waited) {
        eprintln!("[clock] frame {frame} scale {scale:.9} ({:e}) seated {seated} moving {moving}", scale);
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
    // WHICH ENTITY EACH SEAT GOT. Query iteration is entity order, so two
    // processes that seat the fighters in a different order run every
    // tie-break the other way round; printing the ids lets a divergence be
    // matched against the spawn order instead of guessed at.
    {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let mut seats: Vec<(usize, Entity)> = q.iter(world).map(|(e, s)| (s.0, e)).collect();
        seats.sort();
        let text: Vec<String> = seats.iter().map(|(s, e)| format!("seat{s}={e:?}")).collect();
        eprintln!("[seats] {}", text.join(" "));
        // The entities seated just before the fighters: when two processes
        // seat the fighters one id apart, the entity that one of them spawned
        // and the other did not (yet) is in this window.
        if let (Some(first), Some(last)) = (seats.first(), seats.last()) {
            let lo = first.1.index().index().saturating_sub(6);
            let hi = last.1.index().index() + 2;
            let mut all = world.query::<Entity>();
            let mut near: Vec<Entity> =
                all.iter(world).filter(|e| (lo..=hi).contains(&e.index().index())).collect();
            near.sort();
            for entity in near {
                let names: Vec<String> = world
                    .inspect_entity(entity)
                    .map(|infos| {
                        infos
                            .map(|info| info.name().rsplit("::").next().unwrap_or("?").to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                eprintln!("[entity] {entity:?}: {}", names.join(","));
            }
        }
    }
    let ticks = seconds * 60;
    let mut totals: Vec<Tally> = vec![Tally::default(); 4];
    let mut live_move: Vec<Option<LiveMove>> = vec![None; 4];
    let mut parry_was: Vec<f32> = vec![0.0; 4];
    let mut hitstun_was: Vec<f32> = vec![0.0; 4];
    let mut last_damage: Vec<i32> = vec![0; 4];
    let mut steer_was: Vec<f32> = vec![0.0; 4];
    let mut last_world: Vec<(usize, f32, f32, Option<f32>)> = Vec::new();
    let mut camera = CameraTally {
        room: ambition_demo_smash::smash_stage().world.size,
        ..CameraTally::default()
    };
    {
        let world = app.world_mut();
        let spare: Vec<Entity> = (0..recycle_ids).map(|_| world.spawn_empty().id()).collect();
        for entity in spare {
            world.despawn(entity);
        }
    }
    let probes = sim_writers
        .then(|| app.world().get_resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>().cloned())
        .flatten();
    if sim_writers && probes.is_none() {
        eprintln!("[sim-writers] this composition registers no checksum probes; nothing to read");
    }
    let mut writers: std::collections::BTreeMap<&'static str, usize> = Default::default();
    let mut idle_frames = 0usize;
    let mut render_sizes: std::collections::BTreeMap<usize, Option<bevy::math::Vec2>> = Default::default();
    for _ in 0..ticks {
        let before = probes.as_ref().map(|p| (sim_tick_of(&app), p.census_all(app.world_mut())));
        let room_entities = probes.as_ref().map(|_| room_instance_entities(&mut app));
        app.update();
        if probes.is_some() {
            let tick = sim_tick_of(&app);
            let world = app.world_mut();
            let mut q = world.query::<(&MatchSeat, Option<&ambition_platformer2d::combat::components::ActorRenderSize>)>();
            for (seat, size) in q.iter(world) {
                let size = size.map(|s| s.0);
                if render_sizes.get(&seat.0) != Some(&size) {
                    eprintln!("[sim-writers] t{tick} seat {} render size {:?}", seat.0, size);
                    render_sizes.insert(seat.0, size);
                }
            }
        }
        if let (Some(was), Some((tick_before, _))) = (&room_entities, &before) {
            if sim_tick_of(&app) == *tick_before {
                let now = room_instance_entities(&mut app);
                for (entity, kinds) in was.iter().filter(|(e, _)| !now.contains_key(e)) {
                    eprintln!("[sim-writers] t{tick_before} despawned {entity:?}: {kinds}");
                }
                for (entity, kinds) in now.iter().filter(|(e, _)| !was.contains_key(e)) {
                    eprintln!("[sim-writers] t{tick_before} spawned {entity:?}: {kinds}");
                }
            }
        }
        if let (Some(p), Some((tick_before, census_before))) = (&probes, before) {
            if sim_tick_of(&app) == tick_before {
                idle_frames += 1;
                for (name, reading) in p.census_all(app.world_mut()) {
                    let was = census_before.get(name);
                    if was.is_none_or(|w| (w.count, w.xor) != (reading.count, reading.xor)) {
                        *writers.entry(name).or_default() += 1;
                        eprintln!(
                            "[sim-writers] t{tick_before} {name} count {} -> {}",
                            was.map_or(0, |w| w.count),
                            reading.count
                        );
                    }
                }
            }
        }
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
    if probes.is_some() {
        eprintln!("[sim-writers] {idle_frames} frames ran no sim tick; types written on them:");
        for (name, frames) in &writers {
            eprintln!("[sim-writers]   {frames:5}  {name}");
        }
    }

    (totals, camera)
}

/// Every room-instance entity and its component names, short.
fn room_instance_entities(app: &mut App) -> std::collections::BTreeMap<Entity, String> {
    let world = app.world_mut();
    let mut q = world.query::<Entity>();
    let entities: Vec<Entity> = q.iter(world).collect();
    entities
        .into_iter()
        .map(|entity| {
            let names: Vec<String> = world
                .inspect_entity(entity)
                .map(|infos| {
                    infos
                        .map(|info| info.name().rsplit("::").next().unwrap_or("?").to_string())
                        .collect()
                })
                .unwrap_or_default();
            (entity, names.join(","))
        })
        .filter(|(_, names)| names.contains("InRoomInstance"))
        .collect()
}

/// The sim tick the app has reached, or 0 before the clock exists.
fn sim_tick_of(app: &App) -> u64 {
    app.world()
        .get_resource::<ambition_platformer2d::time::SimTick>()
        .map_or(0, |t| t.get())
}

/// THE CAMERA TRACE: one row per tick of the view the match camera resolved
/// and where the seated fighters sat in it. `report_camera` reduces it.
#[derive(Default)]
struct CameraTally {
    /// Finish-zoom requests seen (the verdict's, and a finishing blow's), and
    /// the ticks they landed on.
    finish_zooms: Vec<usize>,
    /// Stocks spent (KOs), and every LAST-STOCK launch with the finishing-blow
    /// predictor's inputs and verdict, so a zoom that never fires is explained.
    stocks_spent: usize,
    last_stock_launches: Vec<String>,
    /// Per seat, the stocks seen last tick, so a spend is a state change.
    stocks_seen: Vec<u32>,
    /// Whether the finish zoom was closing last tick: a rising edge is one zoom.
    zoom_was_closing: bool,
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
    let tick = camera.frames.len();
    {
        let mut seats = world.query::<(&MatchSeat, &ambition_platformer2d::combat::FighterStocks)>();
        let mut now: Vec<(usize, u32)> = seats.iter(world).map(|(seat, stocks)| (seat.0, stocks.remaining)).collect();
        now.sort();
        if camera.stocks_seen.len() == now.len() {
            for ((_, remaining), seen) in now.iter().zip(&camera.stocks_seen) {
                if remaining < seen {
                    camera.stocks_spent += (seen - remaining) as usize;
                }
            }
        }
        camera.stocks_seen = now.into_iter().map(|(_, r)| r).collect();
    }
    if let Some(zoom) = world.get_resource::<ambition_platformer2d::platformer::camera_ease::FinishZoomState>() {
        let closing = zoom.closeness > 0.0;
        if closing && !camera.zoom_was_closing {
            camera.finish_zooms.push(tick);
        }
        camera.zoom_was_closing = closing;
    }
    {
        let mut launches = world.query::<(
            Entity,
            &ambition_platformer2d::combat::FighterStocks,
            &ae::BodyKinematics,
            &ae::BodyFlightState,
            Option<&ambition_platformer2d::platformer::frame_env::ResolvedMotionFrame>,
        )>();
        let rows: Vec<String> = launches
            .iter(world)
            .filter(|(_, stocks, _, flight, _)| {
                stocks.remaining == 1 && flight.pending_launch_state().velocity.length() > 500.0
            })
            .map(|(_, _, kin, flight, frame)| {
                let staged = flight.pending_launch_state().velocity;
                let gravity = frame.map(|f| f.get().acceleration());
                let world_size = ambition_demo_smash::smash_stage().world;
                let predicted = gravity.is_some_and(|g| ae::hit_response::launch_leaves_the_world(kin.pos, staged, g, &world_size));
                format!(
                    "    t{tick} last-stock launch at ({:.0},{:.0}) v ({:.0},{:.0}) gravity {:?} -> leaves {predicted}",
                    kin.pos.x, kin.pos.y, staged.x, staged.y, gravity.map(|g| (g.x.round(), g.y.round()))
                )
            })
            .collect();
        if camera.last_stock_launches.len() < 12 {
            camera.last_stock_launches.extend(rows);
        }
    }
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
        "  finish zooms      {} at ticks {:?}; stocks spent {}",
        camera.finish_zooms.len(),
        camera.finish_zooms,
        camera.stocks_spent
    );
    for row in &camera.last_stock_launches {
        println!("{row}");
    }
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
    live_move: &mut [Option<LiveMove>],
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
    // THE HURT SIDE, as combat reads it: each seat's hurt pieces and every
    // reason a strike that reaches them records nothing.
    struct Victim {
        seat: usize,
        entity: Entity,
        center: ae::Vec2,
        hurt: Vec<ae::CombatVolume>,
        body_box: ae::CombatVolume,
        dead: bool,
        respawn: bool,
        invuln: bool,
        damage_invuln: bool,
        shielding: bool,
        evading: bool,
        ledge: bool,
        hitstop: bool,
    }
    let mut vq = world.query::<(
        Entity,
        &MatchSeat,
        &ae::CenteredAabb,
        Option<&ambition_platformer2d::combat::DamageableVolumes>,
        &BodyHealth,
        &BodyCombat,
        &ae::BodyKinematics,
        Option<&ae::BodyShieldState>,
        Option<&ae::BodyMotionFacts>,
        bevy::ecs::query::Has<ambition_platformer2d::combat::death_rules::OutOfPlay>,
    )>();
    let victims: Vec<Victim> = vq
        .iter(world)
        .map(|(entity, seat, aabb, volumes, health, combat, kin, shield, facts, out_of_play)| {
            let hurt = match volumes {
                Some(v) if v.intangible() => Vec::new(),
                Some(v) if v.published() => v.volumes.clone(),
                _ => vec![ae::CombatVolume::aabb(aabb.aabb())],
            };
            Victim {
                seat: seat.0,
                entity,
                center: aabb.center,
                hurt,
                body_box: ae::CombatVolume::aabb(ae::Aabb::new(kin.pos, kin.size * 0.5)),
                dead: out_of_play || !health.alive(),
                respawn: health.health.invulnerable.holds(
                    ambition_platformer2d::characters::actor::Invulnerability::RESPAWN,
                ),
                invuln: health.health.invulnerable.any(),
                damage_invuln: !combat.vulnerable(),
                shielding: shield.is_some_and(|s| s.active || s.parrying()),
                evading: facts.is_some_and(|f| f.evading()),
                ledge: facts.is_some_and(|f| f.ledge_intangible),
                hitstop: combat.hitstop_timer > 0.0,
            }
        })
        .collect();
    let mut hq = world.query::<(
        &ambition_platformer2d::combat::strike::Hitbox,
        &ambition_platformer2d::combat::strike::HitboxHits,
    )>();
    // (owner seat, cause) for every overlap that recorded no hit this tick,
    // and the body-box-only overlaps.
    let mut overlaps: Vec<(usize, &'static str)> = Vec::new();
    let mut body_box_only: Vec<usize> = Vec::new();
    // (owner seat, world centre) of every live strike entity this tick.
    let mut strike_centres: Vec<(usize, ae::Vec2)> = Vec::new();
    // (owner seat, world bounds) of every live strike entity this tick.
    let mut strike_bounds: Vec<(usize, ae::Aabb)> = Vec::new();
    for (hitbox, hits) in hq.iter(world) {
        let Some(owner) = victims.iter().find(|v| v.entity == hitbox.owner) else {
            continue;
        };
        let volume = hitbox.world_volume(owner.center);
        let b = volume.bounds();
        strike_centres.push((owner.seat, (b.min + b.max) * 0.5));
        strike_bounds.push((owner.seat, b));
        for victim in victims.iter().filter(|v| v.entity != owner.entity) {
            let reaches = victim.hurt.iter().any(|piece| volume.intersects(piece));
            if !reaches {
                if volume.intersects(&victim.body_box) {
                    body_box_only.push(owner.seat);
                }
                continue;
            }
            let cause = if hits.hit.contains(&victim.entity) {
                "already reached this pulse"
            } else if victim.dead {
                "victim dead / out of play"
            } else if victim.respawn {
                "victim respawn grace"
            } else if victim.invuln {
                "victim invulnerable (other)"
            } else if victim.damage_invuln {
                "victim damage-invuln timer"
            } else if victim.shielding {
                "victim shielding"
            } else if victim.evading || victim.ledge {
                "victim evading / ledge"
            } else if owner.hitstop {
                "attacker in hitstop"
            } else {
                "unexplained"
            };
            overlaps.push((owner.seat, cause));
        }
    }
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
    // Which playing moves offer a way home, by id: asked of the frame data
    // once per tick per playing move, not per sample.
    let playback_specs: Vec<(String, bool)> = {
        let mut q = world.query::<&MovePlayback>();
        q.iter(world)
            .map(|p| (p.spec.id.clone(), p.spec.frame_data().recovery_route.offers_a_way_home()))
            .collect()
    };
    // A frozen simulation (the window outliving the match) changes nothing.
    let world_now: Vec<(usize, f32, f32, Option<f32>)> = rows
        .iter()
        .map(|row| (row.0, row.10, row.13, row.6.as_ref().map(|p| p.1)))
        .collect();
    let live = *last_world != world_now;
    // `AMBITION_MATCH_REPORT_TRACE_TICKS=1`: one line per sampled tick with
    // every seat's position, velocity and move clock, so two processes'
    // traces can be diffed to the FIRST tick they disagree on.
    static TRACE_TICKS: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::env::var("AMBITION_MATCH_REPORT_TRACE_TICKS").is_ok_and(|v| v != "0")
    });
    if *TRACE_TICKS {
        let mut cells: Vec<String> = rows
            .iter()
            .map(|row| {
                format!(
                    "s{} {:.3},{:.3} v{:.3},{:.3} hp{} {}",
                    row.0,
                    row.10,
                    row.13,
                    row.5,
                    row.3,
                    row.1,
                    row.6.as_ref().map_or("-".to_string(), |p| format!("{}@{:.3}", p.0, p.1))
                )
            })
            .collect();
        cells.sort();
        let sim_tick = world
            .get_resource::<ambition_platformer2d::time::SimTick>()
            .map_or(0, |t| t.get());
        let sim_dt = world
            .get_resource::<ambition_platformer2d::time::WorldTime>()
            .map_or(f32::NAN, |t| t.sim_dt());
        eprintln!("[tick] t{sim_tick} dt{sim_dt:.9} {}", cells.join(" | "));
    }
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
        let mut causes_now: Vec<&'static str> = Vec::new();
        for (owner, cause) in overlaps.iter().filter(|(owner, _)| *owner == seat) {
            let _ = owner;
            *tally.overlap_no_hit_ticks.entry(cause).or_default() += 1;
            causes_now.push(cause);
        }
        tally.body_box_only_ticks += body_box_only.iter().filter(|o| **o == seat).count();
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
        let recovery_route_of = |id: &str| {
            playback_specs
                .iter()
                .find(|(spec_id, _)| spec_id == id)
                .is_some_and(|(_, offers)| *offers)
        };
        if let Some((id, t, charge, overlapped, boxes, move_facing)) = playback {
            let fresh = match &live_move[seat] {
                Some(last) => last.id != id || t < last.t,
                None => true,
            };
            if fresh {
                // The move before this one ended; a whiff if it never touched.
                if let Some(last) = live_move[seat].take() {
                    tally.end_move(&last);
                }
                tally.moves_started += 1;
                *tally.started.entry(id.clone()).or_default() += 1;
            }
            let carried = (!fresh).then(|| live_move[seat].as_ref()).flatten();
            let touched = overlapped || carried.is_some_and(|m| m.touched);
            let evaded = foe_unhittable || carried.is_some_and(|m| m.evaded);
            let gap = carried.map_or(nearest_gap, |m| m.start_gap);
            let mut causes = carried.map(|m| m.causes.clone()).unwrap_or_default();
            for cause in &causes_now {
                *causes.entry(cause).or_default() += 1;
            }
            let recovery_press = carried.map_or_else(
                || {
                    let stage = ambition_demo_smash::smash_stage().world.size;
                    let offstage = here < 0.0 || here > stage.x || pos_y > stage.y;
                    offstage && recovery_route_of(&id)
                },
                |m| m.recovery_press,
            );
            // THE NEAREST APPROACH of the foe's hurtbox to any live hitbox,
            // over the move's active ticks, in the move's own frame: along
            // (+ beyond the box, − short of it or behind) and across (+
            // below, − above). The smallest total over the move is kept.
            if !boxes.is_empty() {
                tally.active_window_ticks += 1;
                let mine: Vec<ae::Vec2> = strike_centres
                    .iter()
                    .filter(|(owner, _)| *owner == seat)
                    .map(|(_, c)| *c)
                    .collect();
                if !mine.is_empty() {
                    tally.hitbox_present_ticks += 1;
                    let sign = if move_facing < 0.0 { -1.0 } else { 1.0 };
                    for ((lo_x, lo_y), (hi_x, hi_y)) in &boxes {
                        let authored = ae::Vec2::new(
                            here + sign * 0.5 * (lo_x + hi_x),
                            pos_y + 0.5 * (lo_y + hi_y),
                        );
                        let nearest = mine
                            .iter()
                            .map(|c| (*c - authored).length())
                            .fold(f32::INFINITY, f32::min);
                        tally.hitbox_offsets.push(nearest);
                    }
                }
            }
            let mut miss = carried.and_then(|m| m.miss);
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
                    // THE TWO INSTRUMENTS MUST AGREE: an authored box that
                    // contains the foe's body box here, while no strike
                    // entity of this seat reaches that body box, is a
                    // discrepancy to print, not to average away.
                    if total == 0.0 {
                        let foe_box = ae::Aabb::new(ae::Vec2::new(*fx, *fy), *fhalf);
                        let kernel_reaches = strike_bounds.iter().any(|(owner, b)| {
                            *owner == seat
                                && b.min.x <= foe_box.max.x
                                && foe_box.min.x <= b.max.x
                                && b.min.y <= foe_box.max.y
                                && foe_box.min.y <= b.max.y
                        });
                        if !kernel_reaches {
                            tally.instrument_disagreements += 1;
                            if tally.instrument_disagreements <= 3 {
                                let mine: Vec<String> = strike_bounds
                                    .iter()
                                    .filter(|(owner, _)| *owner == seat)
                                    .map(|(_, b)| format!("[{:.0},{:.0}]..[{:.0},{:.0}]", b.min.x, b.min.y, b.max.x, b.max.y))
                                    .collect();
                                eprintln!(
                                    "[disagree] seat {seat} {id} t={t:.3} me=({here:.0},{pos_y:.0}) facing={sign} box=({lo_x:.0},{lo_y:.0})..({hi_x:.0},{hi_y:.0}) foe=({fx:.0},{fy:.0}) half=({:.0},{:.0}) strike entities: {}",
                                    fhalf.x, fhalf.y, if mine.is_empty() { "NONE".to_string() } else { mine.join(" ") }
                                );
                            }
                        }
                    }
                    if miss.is_none_or(|best: (f32, f32)| total < best.0.abs() + best.1.abs()) {
                        miss = Some(m);
                    }
                }
            }
            live_move[seat] = Some(LiveMove {
                id,
                t,
                touched,
                evaded,
                start_gap: gap,
                miss,
                causes,
                recovery_press,
            });
            if let Some(fraction) = charge {
                tally.charge_held += 1;
                tally.best_charge = tally.best_charge.max(fraction);
            }
        } else {
            if let Some(last) = live_move[seat].take() {
                tally.end_move(&last);
            }
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

/// One seat's move in flight, and what the instrument saw of it so far.
#[derive(Clone, Debug)]
struct LiveMove {
    id: String,
    t: f32,
    /// Its contact resolved a hit, a block or a landing at some tick.
    touched: bool,
    /// An opponent was unhittable for some tick of it.
    evaded: bool,
    /// Centre gap to the nearest opponent when it started.
    start_gap: f32,
    /// Nearest approach of the foe to a live hitbox over its active ticks.
    miss: Option<(f32, f32)>,
    /// Ticks of live-hitbox/hurtbox overlap that recorded no hit, by cause.
    causes: std::collections::BTreeMap<&'static str, usize>,
    /// A recovery route (the move offers a way home) thrown while the body
    /// was off the stage: a press whose job is the way home, not the foe.
    recovery_press: bool,
}

impl Tally {
    /// A move ended: file it as a hit or a whiff, with the gap it started at.
    fn end_move(&mut self, m: &LiveMove) {
        let gaps = self.start_gaps.entry(m.id.clone()).or_default();
        if m.touched {
            gaps.0.push(m.start_gap);
            if let Some(miss) = m.miss {
                self.hit_miss.entry(m.id.clone()).or_default().push(miss);
            }
        } else {
            self.whiffs += 1;
            self.whiffs_recovery += usize::from(m.recovery_press);
            self.whiffs_evaded += usize::from(m.evaded);
            *self.whiffed.entry(m.id.clone()).or_default() += 1;
            gaps.1.push(m.start_gap);
            match m.miss {
                Some(miss) => self.whiff_miss.entry(m.id.clone()).or_default().push(miss),
                None => *self.whiff_blind.entry(m.id.clone()).or_default() += 1,
            }
            // The cause seen on most of its overlapping ticks, or none.
            let cause = m
                .causes
                .iter()
                .max_by_key(|(_, n)| **n)
                .map_or("no overlap", |(cause, _)| *cause);
            *self
                .whiff_causes
                .entry(m.id.clone())
                .or_default()
                .entry(cause)
                .or_default() += 1;
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
        misses: Vec<(f32, f32)>,
        blind: usize,
        hit_misses: Vec<(f32, f32)>,
        causes: std::collections::BTreeMap<&'static str, usize>,
    }
    let mut cause_ticks: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    let mut body_box_only = 0usize;
    let (mut active, mut present, mut disagree) = (0usize, 0usize, 0usize);
    let mut offsets: Vec<f32> = Vec::new();
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
            moves.entry(id.as_str()).or_default().misses.extend_from_slice(misses);
        }
        for (id, count) in &tally.whiff_blind {
            moves.entry(id.as_str()).or_default().blind += count;
        }
        for (id, misses) in &tally.hit_miss {
            moves.entry(id.as_str()).or_default().hit_misses.extend_from_slice(misses);
        }
        for (id, causes) in &tally.whiff_causes {
            let row = moves.entry(id.as_str()).or_default();
            for (cause, n) in causes {
                *row.causes.entry(cause).or_default() += n;
            }
        }
        for (cause, n) in &tally.overlap_no_hit_ticks {
            *cause_ticks.entry(cause).or_default() += n;
        }
        body_box_only += tally.body_box_only_ticks;
        active += tally.active_window_ticks;
        present += tally.hitbox_present_ticks;
        disagree += tally.instrument_disagreements;
        offsets.extend_from_slice(&tally.hitbox_offsets);
    }
    let disagreements: usize = 0;
    let _ = disagreements;
    offsets.sort_by(f32::total_cmp);
    let n = offsets.len();
    let offset_med = offsets.get(n / 2).copied().unwrap_or(0.0);
    let offset_p90 = offsets.get(n * 9 / 10).copied().unwrap_or(0.0);
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
    // A whiff is INSIDE when both axes read zero; the medians are over the
    // whiffs that missed, per axis, so a half-short / half-high population
    // cannot read as 0 / 0.
    let inside = |m: &[(f32, f32)]| m.iter().filter(|(a, b)| *a == 0.0 && *b == 0.0).count();
    let outside_along = |m: &[(f32, f32)]| -> Vec<f32> {
        m.iter().filter(|(a, b)| *a != 0.0 || *b != 0.0).map(|(a, _)| *a).collect()
    };
    let outside_across = |m: &[(f32, f32)]| -> Vec<f32> {
        m.iter().filter(|(a, b)| *a != 0.0 || *b != 0.0).map(|(_, b)| *b).collect()
    };
    // The one number a per-axis median cannot fake: the total miss.
    let outside_total = |m: &[(f32, f32)]| -> Vec<f32> {
        m.iter()
            .filter(|(a, b)| *a != 0.0 || *b != 0.0)
            .map(|(a, b)| a.abs() + b.abs())
            .collect()
    };
    // How the missing whiffs missed: short of / beyond the box along, above /
    // below it across (a whiff can be in two of these).
    let how = |m: &[(f32, f32)]| -> String {
        let (mut short, mut beyond, mut above, mut below) = (0, 0, 0, 0);
        for (a, b) in m {
            if *a < 0.0 {
                short += 1;
            } else if *a > 0.0 {
                beyond += 1;
            }
            if *b < 0.0 {
                above += 1;
            } else if *b > 0.0 {
                below += 1;
            }
        }
        format!("{short}/{beyond}/{above}/{below}")
    };
    println!(
        "\nwhat the bodies actually threw: {} distinct\n  {:<28} {:>5} {:>5}   {:>5} {:>5}   {:>6} {:>5} {:>6} {:>6} {:>13} {:>5}   {:>6}",
        rows.len(),
        "move",
        "thrown",
        "whiff",
        "gap@h",
        "gap@w",
        "inside",
        "miss",
        "along",
        "across",
        "sh/bey/ab/bel",
        "blind",
        "h.insd",
    );
    for (id, row) in rows.iter().take(12) {
        println!(
            "  {id:<28} {:>5} {:>5}   {:>5} {:>5}   {:>6} {:>5} {:>6} {:>6} {:>13} {:>5}   {:>3}/{:<3}",
            row.thrown,
            row.whiffed,
            median(&row.hit_gaps),
            median(&row.whiff_gaps),
            inside(&row.misses),
            median(&outside_total(&row.misses)),
            median(&outside_along(&row.misses)),
            median(&outside_across(&row.misses)),
            how(&row.misses),
            row.blind,
            inside(&row.hit_misses),
            row.hit_misses.len(),
        );
    }
    println!(
        "  gap@h / gap@w: median centre gap when a hit / a whiff started. inside: whiffs whose \
         authored box held the foe's body box on some active tick (both axes zero). miss / along / \
         across: over the OTHER whiffs, the median nearest approach of the foe's body box to the \
         authored box — total, and per axis in the move's frame (+along beyond, −along short or \
         behind; −across above); sh/bey/ab/bel counts how they missed. blind: whiffs \
         whose hitbox never opened with a foe present. h.insd: hits that read inside / hits measured \
         — the control."
    );
    println!("\nwhy each whiff that overlapped recorded nothing (the cause seen on most of its ticks):");
    for (id, row) in rows.iter().take(12) {
        if row.causes.is_empty() {
            continue;
        }
        let mut causes: Vec<_> = row.causes.iter().collect();
        causes.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        let text: Vec<String> = causes.iter().map(|(c, n)| format!("{c} {n}")).collect();
        println!("  {id:<28} {}", text.join(", "));
    }
    println!(
        "\nlive-hitbox / hurt-geometry overlap ticks that recorded no hit, by cause (combat's own \
         `Hitbox` volumes against the victim's published volumes or `CenteredAabb`):"
    );
    let mut ticks: Vec<_> = cause_ticks.iter().collect();
    ticks.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (cause, n) in ticks {
        println!("  {cause:<32} {n:>6}");
    }
    println!(
        "  body box (kin.size/2) overlapped where combat's hurt geometry did not: {body_box_only} ticks"
    );
    println!(
        "  strike presence: Active-window ticks {active} — a `Hitbox` entity existed on {present} of \
         them; authored-box vs entity-centre offset median {offset_med:.1} px, p90 {offset_p90:.1} px (n={n}); \
         ticks where the authored box held the foe and no strike entity reached it: {disagree}",
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
    println!("  per live minute: moves {}  whiffs {}  attack whiffs {}  damage {}",
        per_minute(|t| t.moves_started as f32),
        per_minute(|t| t.whiffs as f32),
        per_minute(|t| (t.whiffs - t.whiffs_recovery) as f32),
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
    println!("  ⇒ recovery  {}   (a way-home move started off the stage)", spread(|t| t.whiffs_recovery as f32));
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
