//! Compare adjacent registered AI ladder rungs in CPU-vs-CPU matches.
//!
//! `cargo run -p ambition_demo_smash_app --bin ladder_rig [--seeds N] [--weight name=value ...]`
//!
//! ⭐ `--weight` is what makes this a rig for a SCORING change and not only for a
//! ladder. Three open rows want a weight refit — the scorer's speed term is
//! degenerate, and the weights it is read against were fitted while it was a
//! constant — and refitting means running the same bouts, at the same seeds,
//! with one number moved. Run it twice and compare; the header names the weights
//! each run used.
//!
//! The registered ladder is sparse: levels 1, 3, 5, 6, and 9. The rig reports
//! time to elimination, stocks remaining, and engagement evidence for each pair,
//! using medians across deterministic seeds. Unregistered levels are invalid for
//! this measurement because their generic fallback does not represent a ladder rung.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::{FighterStocks, MatchSeat};
use ambition_platformer2d::engine_core as ae;

/// One minute at 60Hz — the same budget `ladder_probe` uses, so the two are
/// readable against each other.
const TICKS: usize = 3_600;

/// The rungs the demo actually registers. See the sparseness warning above.
const RUNGS: &[u8] = &[1, 3, 5, 6, 9];

/// Nothing changed but the sample count, so every verdict in between was noise wearing a direction
/// — the exact failure this file's own header warns about one paragraph up, reached by its own
/// default.
///
/// fifteen seeds is roughly twenty minutes. That is the price of an answer
/// here; a faster number is not a cheaper one, it is a different question.
const DEFAULT_SEEDS: usize = 15;

/// What one match said.
///
/// Use elapsed time rather than stocks because stock counts saturate when both
/// seats lose all lives and cannot distinguish match quality.
#[derive(Clone, Copy, Debug)]
struct Bout {
    /// Tick each seat was eliminated on, or `TICKS` for a seat that survived.
    /// The LATER one won.
    eliminated: [usize; 2],
    /// Stocks remaining at the end — kept because a seat that survived with
    /// three is a different result from one that survived with one, and the
    /// time column cannot tell them apart.
    stocks: [u32; 2],
    /// Highest damage each seat ever carried, as a RATIO of its pool.
    ///
    /// `1.69` is 169%, not 1.69% — exactly what
    /// `BodyHealth::damage_percent` documents. The `×100` lives at the one print
    /// site. Reading this as a percentage is what made the column report a 169%
    /// duel as `1.69%` for its whole life, and what made the row marker below
    /// call real fights unfought.
    ///
    /// the column that says whether the other two mean anything. This
    /// file's own header demands it — *"pair every 'it won' with 'and it
    /// engaged'. A fighter that stands still beats one that walks off the
    /// stage"* — and it went a week reporting outlast times with no way to tell
    /// a duel from two solo walks off the edge. A pair whose peaks stay near
    /// zero was never a fight, whatever its verdict column says.
    peak_percent: [f32; 2],
}

fn main() {
    let seeds = seed_count();
    if std::env::args().any(|arg| arg == "--scenarios") {
        return run_scenarios(seeds);
    }
    // SAY WHAT THIS RUN MEASURED UNDER. A rig that reports numbers without
    // naming the weights they were produced at is two runs nobody can compare,
    // and comparing two runs is the entire purpose of the override.
    let weights = weights_from_args();
    if weights != ambition_platformer2d::characters::brain::fighter::UtilityWeights::v1() {
        println!("[ladder_rig] weights OVERRIDDEN: {weights:?}");
    } else {
        println!("[ladder_rig] weights: v1 (profile default)");
    }
    println!(
        // ⛔ "stocks" ALONE IS AMBIGUOUS AND WAS MISREAD. The column is stocks
        // REMAINING, so `0 : 0` means BOTH fighters were fully eliminated — the
        // opposite of the "nobody lost a stock" it reads as at a glance. Say
        // LEFT in the header, where the reader is.
        "[ladder_rig] higher vs lower   eliminated(hi:lo)   stocks LEFT(hi:lo)   peak%(hi:lo)   verdict   \
         (median of {seeds} seeds, {}s each)",
        TICKS / 60
    );
    for pair in RUNGS.windows(2) {
        let (lower, higher) = (pair[0], pair[1]);
        let bouts: Vec<Bout> = (0..seeds)
            .map(|seed| run_bout(higher, lower, seed as u64))
            .collect();
        report(higher, lower, &bouts);
    }
}

/// Give BOTH fighters distinct noise streams derived from one seed.
///
/// distinct, not shared. Two brains stepping the same stream would make
/// the higher rung's jitter a function of the lower one's, which is a
/// correlation no real match has — and it would hide exactly the kind of
/// difference this rig exists to find.
/// Override every live fighter's utility weights.
///
/// ⭐ THE RIG COULD COMPARE RUNGS AND NOT WEIGHTS, and a weight is what three
/// open rows are waiting on. `frame_advantage` is degenerate against an
/// uncommitted opponent (D188); fixing its scale doubles one matchup and thirds
/// another, and the weights it is read against were fitted while it was a
/// constant. Refitting them needs exactly this: the same bout machinery, the
/// same seeds, one number changed.
///
/// Applied to the live `FighterState`'s config after seating, beside the noise
/// stream and for the same reason — the brain does not exist until then.
///
/// ⛔ It is an OVERRIDE, not a model of how a fighter gets its weights. A live
/// CPU's come from its profile; sweeping them here is the point, so this
/// deliberately does not go through that seam. Do not "fix" it to match the
/// builder.
fn force_utility_weights(
    app: &mut bevy::app::App,
    weights: ambition_platformer2d::characters::brain::fighter::UtilityWeights,
) -> bool {
    use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
    let world = app.world_mut();
    let mut q = world.query::<&mut Brain>();
    let mut found = false;
    for mut brain in q.iter_mut(world) {
        if let Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) = &mut *brain {
            cfg.profile.utility_weights = weights;
            found = true;
        }
    }
    found
}

/// The weights this run measures under: `v1` unless `--weight name=value` says
/// otherwise, repeatable.
///
/// Named rather than positional because six numbers in a row is a puzzle, and a
/// rig whose invocation cannot be read is a rig whose results cannot be trusted.
fn weights_from_args() -> ambition_platformer2d::characters::brain::fighter::UtilityWeights {
    let mut weights = ambition_platformer2d::characters::brain::fighter::UtilityWeights::v1();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg != "--weight" {
            continue;
        }
        let Some(pair) = args.next() else {
            break;
        };
        let Some((name, value)) = pair.split_once('=') else {
            eprintln!("[ladder_rig] --weight wants name=value, got '{pair}'");
            std::process::exit(2);
        };
        let Ok(value) = value.parse::<f32>() else {
            eprintln!("[ladder_rig] '{value}' is not a number");
            std::process::exit(2);
        };
        match name {
            "reach_fit" => weights.reach_fit = value,
            "frame_advantage" => weights.frame_advantage = value,
            "kill_potential" => weights.kill_potential = value,
            "stage_risk" => weights.stage_risk = value,
            "expected_payoff" => weights.expected_payoff = value,
            "capture_value" => weights.capture_value = value,
            other => {
                eprintln!("[ladder_rig] no weight named '{other}'");
                std::process::exit(2);
            }
        }
    }
    weights
}

fn force_noise_seed(app: &mut bevy::app::App, seed: u64) -> bool {
    use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
    let world = app.world_mut();
    let mut brains = world.query::<(&MatchSeat, &mut Brain)>();
    let mut applied = false;
    for (seat, mut brain) in brains.iter_mut(world) {
        if let Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) = &mut *brain {
            // A zero stream is a legitimate SplitMix64 state but an unhelpful
            // one to start every seat on; the seat index separates them.
            state.noise = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (seat.0 as u64 + 1);
            applied = true;
        }
    }
    applied
}

/// Run rung pairs through scenarios reproducible by body placement alone.
/// Scenarios requiring velocity, phases, projectiles, or other explicit state are
/// skipped using `Scenario::unreproduced_by_placement`.
fn run_scenarios(seeds: usize) {
    let suite = ambition_platformer2d::combat::brain::fighter::scenarios::suite();
    let playable: Vec<_> = suite
        .iter()
        .filter(|s| s.starting_positions().is_some() && s.is_reproduced_by_placement())
        .collect();
    println!(
        "[ladder_rig] --scenarios: PLACEMENT ONLY — {} of {} fixture(s) are \
         reproduced by placing two bodies (median of {seeds} seeds, {}s each)",
        playable.len(),
        suite.len(),
        TICKS / 60
    );
    // ⛔ THE SCENARIO TABLE PRINTED NO COLUMN HEADER AT ALL, so every reader had
    // to infer five columns from the numbers — and `stocks` was read as "stocks
    // lost" in a planning row, inverting what the rows meant.
    println!(
        "[ladder_rig] fixture            rungs     eliminated(hi:lo)              stocks LEFT   \
         peak%(hi:lo)     verdict"
    );
    for scenario in &suite {
        if scenario.starting_positions().is_none() {
            println!(
                "[ladder_rig]   {:<22} SKIPPED (no opponent — not a bout)",
                scenario.name
            );
            continue;
        }
        let missing = scenario.unreproduced_by_placement();
        if !missing.is_empty() {
            println!(
                "[ladder_rig]   {:<22} SKIPPED (this rig cannot set up: {}) — its \
                 premise is not reproduced by a placement, so a row here would be \
                 a positional fixture under a tactical name",
                scenario.name,
                missing.join(", ")
            );
            continue;
        }
        for pair in RUNGS.windows(2) {
            let (lower, higher) = (pair[0], pair[1]);
            let bouts: Vec<Bout> = (0..seeds)
                .map(|seed| run_bout_at(higher, lower, seed as u64, Some(scenario.clone())))
                .collect();
            report_row(
                &format!("{:<18} {higher:>2} vs {lower:<2}", scenario.name),
                &bouts,
            );
        }
    }
}

fn seed_count() -> usize {
    flag_value("--seeds")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_SEEDS)
}

/// The value that followed `name` on the command line.
fn flag_value(name: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == name {
            return args.next();
        }
    }
    None
}

/// WHO IS FIGHTING — and it is a flag because the answer changes the reading
/// of every column.
///
/// the ladder's own fighters are the demo's STAND-INS, and this rig had no way to say
/// otherwise.
///
/// Two instruments, one nominal subject, two orders of magnitude. A rig that cannot change who is
/// fighting cannot tell you which of those is about the AI.
fn fighters() -> [String; 2] {
    [
        flag_value("--character")
            .unwrap_or_else(|| ambition_demo_smash::SMASH_CHARACTER_ID.to_string()),
        flag_value("--opponent")
            .unwrap_or_else(|| ambition_demo_smash::SMASH_OPPONENT_ID.to_string()),
    ]
}

fn median(mut values: Vec<f32>) -> f32 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    values.get(values.len() / 2).copied().unwrap_or(0.0)
}

fn secs(ticks: f32) -> String {
    if ticks >= TICKS as f32 {
        format!(">{}s", TICKS / 60)
    } else {
        format!("{:.1}s", ticks / 60.0)
    }
}

/// `median [min-max]`, or just the median when every seed agreed.
///
/// the SPREAD is what says whether a difference is a difference. The two
/// top rungs here separate by a couple of seconds on medians whose seeds range
/// over tens — a gap a median alone reports as a verdict.
fn span(values: &[f32]) -> String {
    let mid = median(values.to_vec());
    let lo = values.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    if (hi - lo).abs() < 1.0 {
        secs(mid)
    } else {
        format!("{} [{}-{}]", secs(mid), secs(lo), secs(hi))
    }
}

fn report(higher: u8, lower: u8, bouts: &[Bout]) {
    report_row(&format!("{higher:>2} vs {lower:<2}"), bouts);
}

/// One line, under whatever label the caller is grouping by.
fn report_row(label: &str, bouts: &[Bout]) {
    let hi_all: Vec<f32> = bouts.iter().map(|b| b.eliminated[0] as f32).collect();
    let lo_all: Vec<f32> = bouts.iter().map(|b| b.eliminated[1] as f32).collect();
    let hi_out = median(hi_all.clone());
    let lo_out = median(lo_all.clone());
    let hi_stocks = median(bouts.iter().map(|b| b.stocks[0] as f32).collect());
    let lo_stocks = median(bouts.iter().map(|b| b.stocks[1] as f32).collect());
    // The seat that lasted LONGER won.
    let verdict = if hi_out > lo_out {
        "higher lasts"
    } else if lo_out > hi_out {
        "LOWER lasts"
    } else if hi_out >= TICKS as f32 {
        "both survive"
    } else {
        "both die together"
    };
    // a verdict inside the seeds' own spread is not a verdict. Reported
    // rather than suppressed: the reader should see the overlap and discount the
    // word, not be handed a cleaner-looking table.
    let overlaps = (hi_out - lo_out).abs()
        < 0.5
            * ((hi_all.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                - hi_all.iter().copied().fold(f32::INFINITY, f32::min))
            .max(
                lo_all.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                    - lo_all.iter().copied().fold(f32::INFINITY, f32::min),
            ));
    let verdict = if overlaps {
        format!("{verdict} (within spread)")
    } else {
        verdict.to_string()
    };
    let hi_peak = median(bouts.iter().map(|b| b.peak_percent[0]).collect());
    let lo_peak = median(bouts.iter().map(|b| b.peak_percent[1]).collect());
    // Damage percent is represented as a ratio, so 0.01 means one percent.
    // Rows below that threshold for both fighters are reported as unfought.
    const FOUGHT_AT_ALL: f32 = 0.01;
    let verdict = if hi_peak < FOUGHT_AT_ALL && lo_peak < FOUGHT_AT_ALL {
        format!("{verdict} — BUT NEITHER LANDED A HIT")
    } else {
        verdict
    };
    println!(
        "[ladder_rig]   {label:<26} {:>20} : {:<20} {hi_stocks:>3.0} : {lo_stocks:<3.0}          {:>6.1}% : {:<6.1}%  {verdict}",
        span(&hi_all),
        span(&lo_all),
        // ×100 HERE and nowhere else. The ratio is what every other reader
        // of `damage_percent` wants; a percentage is a display concern, and
        // baking it into the stored column is how the threshold above came to be
        // written in the wrong units.
        hi_peak * 100.0,
        lo_peak * 100.0
    );
}

/// Seat the two rungs and run a full match.
///
/// the 30 warm-up updates before the roster lands are `ladder_probe`'s, and
/// for its reason: the shell has to reach its stage before a roster means
/// anything.
/// The running stage's own extent, which is what a fixture's relative geometry
/// gets mapped onto.
fn stage_bounds(app: &mut bevy::app::App) -> Option<ae::Aabb> {
    use ambition_platformer2d::platformer::lifecycle::session_world_component;
    session_world_component::<ae::RoomGeometry>(app.world())
        .map(|geometry| ae::Aabb::new(geometry.0.size * 0.5, geometry.0.size * 0.5))
}

/// Put the two seated bodies where a scenario says they stand.
///
/// AFTER seating, and only once both seats exist. A roster cannot say
/// where its fighters stand — the stage decides — so this is a measurement
/// binary reaching into the sim. It is deliberate and it is not a seam to
/// promote: a game that placed fighters this way would be fighting its own
/// stage.
///
/// Returns `false` until both seats are present, so the caller keeps trying
/// rather than placing one body and calling it a scenario.
fn place_at(app: &mut bevy::app::App, me: ae::Vec2, foe: ae::Vec2) -> bool {
    use ambition_platformer2d::actor::{transit_body, BodyClusterQueryData, TransitVelocity};
    let world = app.world_mut();
    let mut q = world.query::<(
        &MatchSeat,
        BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    )>();
    let seats: Vec<usize> = q.iter(world).map(|(seat, ..)| seat.0).collect();
    if !seats.contains(&0) || !seats.contains(&1) {
        return false;
    }
    for (seat, mut cluster_item, mut model) in q.iter_mut(world) {
        let target = if seat.0 == 0 { me } else { foe };
        let mut clusters = cluster_item.as_clusters_mut();
        // `transit_body`, not `body.pos = ..`. ADR 0024 routes every pose
        // and velocity write through the movement authority, and
        // `engine.pose-writes-are-authority-only` caught the bare version of
        // this — with a rationale naming the TwinTrack demo, which *"relocated a
        // body outside the authority for two days"*.
        //
        // and it is not only a rule: `transit_body` calls `reconcile_transit`,
        // which the field write skipped — so a body teleported to a ledge kept
        // whatever surface and frame state it had at the spawn point, and the
        // scenario measured a fighter standing in a premise its motion model did
        // not agree with.
        //
        // `Zero`, because a body carrying the spawn's fall speed into a
        // "standing at the ledge" premise is not in that premise.
        transit_body(&mut model, &mut clusters, target, TransitVelocity::Zero);
    }
    true
}

fn run_bout(higher: u8, lower: u8, seed: u64) -> Bout {
    run_bout_at(higher, lower, seed, None)
}

/// One bout, optionally started from a scenario's positions.
fn run_bout_at(
    higher: u8,
    lower: u8,
    seed: u64,
    start: Option<ambition_platformer2d::combat::brain::fighter::scenarios::Scenario>,
) -> Bout {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            fighters(),
            &[higher, lower],
        ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    // A seat that is ELIMINATED stops existing, so the last value seen is the
    // answer — reading only at the end would report zero for both.
    let mut stocks = [ambition_demo_smash::STARTING_STOCKS; 2];
    let mut eliminated = [TICKS; 2];
    let mut peak_percent = [0.0f32; 2];
    // A seat is not eliminated until seating has completed; bodies may be absent
    // during the seating transaction.
    let mut appeared = [false; 2];
    // Apply the seed to the live `FighterState` after seating, when the brain and
    // its noise stream exist.
    let mut seeded = false;
    let weights = weights_from_args();
    let mut placed = start.is_none();
    for tick in 0..TICKS {
        app.update();
        if !seeded {
            seeded = force_noise_seed(&mut app, seed);
            if seeded {
                force_utility_weights(&mut app, weights);
            }
        }
        if !placed {
            if let Some(scenario) = start.as_ref() {
                // mapped onto the RUNNING stage, not pasted. The fixture's
                // numbers describe an 800x600 stage of its own; the smash stage
                // is a different size in a different place. Pasting them put
                // every recovery quadrant far outside any platform, where the
                // blastzone took it instantly — two of them printed identical
                // columns, which is how it was found.
                placed = stage_bounds(&mut app)
                    .and_then(|bounds| scenario.starting_positions_on(bounds))
                    .is_some_and(|(me, foe)| place_at(&mut app, me, foe));
            }
        }
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &FighterStocks,
            &ambition_platformer2d::characters::actor::BodyHealth,
        )>();
        let mut seen = [false; 2];
        for (seat, remaining, health) in q.iter(world) {
            if seat.0 < 2 {
                seen[seat.0] = true;
                stocks[seat.0] = remaining.remaining;
                peak_percent[seat.0] = peak_percent[seat.0].max(health.damage_percent());
            }
        }
        // An ELIMINATED seat stops existing — that disappearance is the event,
        // and it is why the loop reads every tick instead of once at the end.
        for slot in 0..2 {
            appeared[slot] |= seen[slot];
            if appeared[slot] && !seen[slot] && eliminated[slot] == TICKS {
                eliminated[slot] = tick;
                stocks[slot] = 0;
            }
        }
    }
    assert!(
        placed,
        "a scenario bout ran {TICKS} ticks and the fighters were never placed, so \
         it measured the stage's default spawn while claiming a scenario"
    );
    assert!(
        seeded,
        "no fighter brain ever took the noise seed, so every run of this bout is \
         identical and the median is one sample reported N times"
    );
    assert!(
        appeared == [true, true],
        "a ladder bout ran {TICKS} ticks and seat {:?} never appeared — the \
         match never seated, and every column below would be measuring an empty \
         stage",
        appeared.iter().position(|seen| !seen)
    );
    Bout {
        eliminated,
        stocks,
        peak_percent,
    }
}
