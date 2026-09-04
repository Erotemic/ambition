//! Compare adjacent registered AI ladder rungs in CPU-vs-CPU matches.
//!
//! `cargo run -p ambition_demo_smash_app --bin smash_tool -- ladder-rig [--seeds N] [--weight name=value ...]`
//!
//! ⭐ `--weight` is what makes this a rig for a SCORING change and not only for a
//! ladder. Three open rows want a weight refit — the scorer's speed term is
//! degenerate, and the weights it is read against were fitted while it was a
//! constant — and refitting means running the same bouts, at the same seeds,
//! with one number moved. Run it twice and compare; the header names the weights
//! each run used.
//!
//! ⭐ **THE OTHER THREE FLAGS, AND WHAT EACH ONE CONTROLS FOR** (all added
//! 2026-09-04, each because a measurement had been quietly answering a different
//! question than the one asked):
//!
//! - `--paired` — run each seed TWICE with the rungs swapped between seats and
//!   test the WITHIN-SEED difference. Every cell of the 15-seed matrix came back
//!   `(within spread)` because seed variance exceeded the effect; pairing removes
//!   that variance rather than out-sampling it, and cancels the seat/placement
//!   confound (7 of the 9 fixtures put SELF, always the higher rung, offstage).
//!   ⇒ It changed 14 of 36 verdicts: a 24:12 skew toward the lower rung became
//!   16:19. The unpaired reading was measuring the seat.
//! - `--stage flat|platforms` — which layout to fight on. Every number recorded
//!   before this flag was taken on `flat`, because it was the only stage; that
//!   made the layout a confounder rather than a choice. The tiers roughly halve
//!   the lethality, so the flag is not cosmetic.
//! - `--no-rollout` — zero `rollout_depth`/`rollout_k` on every fighter. ⭐ Its
//!   control is FREE and exact: rollout is already off below level 6, so the
//!   bottom rungs must be identical between arms, and they are — to the decimal.
//!   Anything that moves at `6 vs 5` or `9 vs 6` is the rollout and nothing else.
//!
//! ⛔ Every table names its stage, its weights, its design and which ladder the
//! fighters actually got, because this rig spent its whole life reporting numbers
//! measured on the ENGINE FLOOR without saying so.
//!
//! The registered ladder is sparse: levels 1, 3, 5, 6, and 9. The rig reports
//! time to elimination, stocks remaining, and engagement evidence for each pair,
//! using medians across deterministic seeds. Unregistered levels are invalid for
//! this measurement because their generic fallback does not represent a ladder rung.

use crate::build_demo_app;
use ambition_platformer2d::actor::{FighterStocks, MatchSeat};
use ambition_platformer2d::engine_core as ae;

/// One minute at 60Hz — the same budget `ladder_probe` uses, so the two are
/// readable against each other.
const TICKS: usize = 3_600;

/// The rungs the demo actually registers. See the sparseness warning above.
const RUNGS: &[u8] = &[1, 3, 5, 6, 9];

/// The rungs this run walks — `--rungs` when given, the registered ladder
/// otherwise. Pairs are consecutive, so `6,6` is one bout of a rung against
/// itself and `1,3,5,6,9` is the four the ladder has always compared.
fn rungs() -> Vec<u8> {
    let Some(spec) = args().rungs.as_deref() else {
        return RUNGS.to_vec();
    };
    let parsed: Vec<u8> = spec
        .split(',')
        .map(|part| {
            part.trim().parse::<u8>().unwrap_or_else(|_| {
                // ⛔ A rung list that does not parse must not fall back to the
                // default: the run would silently measure the ladder while its
                // header claimed otherwise, which is the exact class of failure
                // this file spent a day removing.
                eprintln!("[ladder_rig] --rungs wants comma-separated levels, got '{spec}'");
                std::process::exit(2);
            })
        })
        .collect();
    if parsed.len() < 2 {
        eprintln!("[ladder_rig] --rungs needs at least two levels to make a pair");
        std::process::exit(2);
    }
    parsed
}

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
    /// TOTAL damage each seat absorbed across the whole match, summed from
    /// per-tick increases, as a ratio like [`Self::peak_percent`].
    ///
    /// ⛔ **PEAK IS NOT DAMAGE DEALT, which is what it was briefly used as.**
    /// Percent RESETS on death, so a seat killed three times at 100% shows a
    /// peak of 100 and a seat pressured to 250% and never killed shows 250 —
    /// the peak of the fighter who died more is LOWER. Worse as a tiebreak: a
    /// high peak before a kill means the killer needed more damage to close,
    /// which is the opposite of skill. Summing the increases counts every point
    /// landed and is blind to how they were grouped.
    damage_taken: [f32; 2],
    /// The CLOSEST the two seats ever came, in world px.
    ///
    /// ⛔ **Added to tell "they never met" from "they met and whiffed".** The
    /// platformed stage produced 41 unfought bouts of 540 against the flat
    /// stage's 3, and `unfought` alone cannot say whether the fighters failed to
    /// NAVIGATE to each other or reached each other and declined to commit —
    /// which are a pathing problem and a scoring problem, fixed in different
    /// places. A bout whose closest approach is a body-width apart met; one that
    /// stayed hundreds of pixels apart did not.
    closest_approach: f32,
}

#[derive(clap::Args, Debug, Clone, Default)]
pub struct LadderRigArgs {
    /// How many seeds to run.
    #[arg(long)]
    pub seeds: Option<usize>,
    /// Run the below-the-ledge sweep instead of the ladder.
    #[arg(long)]
    pub sweep_below: bool,
    /// Run the named scenarios instead of the ladder.
    #[arg(long)]
    pub scenarios: bool,
    /// Override a utility weight, as `NAME=VALUE`. Repeatable.
    #[arg(long = "weight", value_name = "NAME=VALUE")]
    pub weights: Vec<String>,
    /// Disable rollout search for the run.
    #[arg(long)]
    pub no_rollout: bool,
    /// Reaction delay in milliseconds.
    #[arg(long)]
    pub reaction_ms: Option<u64>,
    /// Actions-per-minute cap.
    #[arg(long)]
    pub apm: Option<f32>,
    /// Decision noise.
    #[arg(long)]
    pub noise: Option<f32>,
    /// Fighter under test.
    #[arg(long)]
    pub character: Option<String>,
    /// Fighter to test against.
    #[arg(long)]
    pub opponent: Option<String>,
    /// Rungs to walk, comma-separated, consecutive pairs compared. Defaults to
    /// the registered ladder `1,3,5,6,9`.
    ///
    /// ⭐⭐ **THE NULL CONTROL THIS RIG COULD NOT RUN.** Every verdict it has ever
    /// printed compares two DIFFERENT levels, so nothing has ever answered the
    /// prior question: **do two IDENTICAL fighters split evenly?** `--rungs 6,6`
    /// asks exactly that. If a rung against itself does not come out near even
    /// under `--paired`, the bias is in the instrument — seats, fixtures, or the
    /// verdict — and every difference this rig has attributed to skill is
    /// suspect by that amount. A measurement tool that cannot measure zero
    /// cannot be trusted about small numbers.
    #[arg(long)]
    pub rungs: Option<String>,
    /// Run each seed TWICE with the rungs swapped between seats, and report the
    /// within-seed difference.
    ///
    /// ⭐ **WHY: every cell of the 15-seed matrix came back `(within spread)`,**
    /// which is the rig saying the seed-to-seed variance is larger than the
    /// effect. Pairing removes that variance instead of trying to out-sample it:
    /// the same seed plays both role assignments, so the comparison is a
    /// DIFFERENCE within one seed rather than a difference of two medians drawn
    /// from a wide distribution.
    ///
    /// ⭐⭐ It also cancels the confound I could not otherwise rule out. The
    /// fixtures place SELF — always seat 0, always the higher rung — and 7 of the
    /// 9 place it badly (*"Self is past a blastzone"*). Under `--paired` each
    /// rung stands in that spot equally often, so a residue cannot be the
    /// placement.
    ///
    /// ⚠ Costs exactly double the bouts. That is the price of the control.
    #[arg(long)]
    pub paired: bool,
    /// Stage to fight on: `flat` (default) or `platforms`.
    ///
    /// ⭐ Every ladder number recorded before 2026-09-04 was measured on `flat`,
    /// which was the only stage there was. That makes the stage a CONFOUNDER
    /// sitting under the whole corpus — spacing, recovery and edgeguard results
    /// were all taken on one layout — and this flag is what turns it into a
    /// variable that can be compared instead of a constant nobody chose.
    #[arg(long, default_value = "flat")]
    pub stage: String,
}

/// ⛔ **PARSED ONCE, READ FROM DEPTH.** `flag_value` was called from inside
/// `run_ladder`'s innermost loop and from three other functions, so threading a
/// struct through would rewrite six signatures in a 685-line file for no gain in
/// what the tool DOES. The surface is now declarative and `--help` documents it;
/// the reads stay where they were, against a value parsed once at entry instead
/// of a fresh `std::env::args()` scan each time.
/// ⚠ It is a process global, which is correct here and would not be in a
/// library: `run` is the only writer and it writes before anything reads.
static ARGS: std::sync::OnceLock<LadderRigArgs> = std::sync::OnceLock::new();

fn args() -> &'static LadderRigArgs {
    ARGS.get_or_init(LadderRigArgs::default)
}

pub fn run(cli: LadderRigArgs) {
    let _ = ARGS.set(cli);
    let seeds = seed_count();
    if args().sweep_below {
        return run_sweep_below(seeds);
    }
    if args().scenarios {
        return run_scenarios(seeds);
    }
    // SAY WHAT THIS RUN MEASURED UNDER. A rig that reports numbers without
    // naming the weights they were produced at is two runs nobody can compare,
    // and comparing two runs is the entire purpose of the override.
    match weights_from_args() {
        Some(weights) => println!(
            "[ladder_rig] weights OVERRIDDEN on EVERY fighter: {weights:?} \
             (the authored per-level weights are not in play)"
        ),
        // ⚠ "not overridden", NOT "the authored rows". Those are different
        // claims and the line below is the one that says which rows a rung
        // actually got: this rig prints both, and an earlier wording had them
        // contradicting each other on consecutive lines.
        None => println!(
            "[ladder_rig] weights: not overridden — each rung keeps whatever its \
             profile source gave it (see the ladder line below)"
        ),
    }
    report_which_ladder_is_in_play();
    println!(
        // ⛔ "stocks" ALONE IS AMBIGUOUS AND WAS MISREAD. The column is stocks
        // REMAINING, so `0 : 0` means BOTH fighters were fully eliminated — the
        // opposite of the "nobody lost a stock" it reads as at a glance. Say
        // LEFT in the header, where the reader is.
        "[ladder_rig] higher vs lower   survived(hi:lo)   stocks LEFT(hi:lo)   dealt%(hi:lo)   peak%(hi:lo)   \
         verdict = who OUTFOUGHT: stocks taken, then damage dealt   \
         (median of {seeds} seeds, {}s each, {})",
        TICKS / 60,
        // The design belongs in EVERY table's header, not just the scenario
        // one. This mode had no such line while `--paired` silently did nothing
        // here, so a reader had two reasons to be misled and no way to see either.
        if args().paired {
            "PAIRED — each seed run twice with the rungs swapped between seats"
        } else {
            "unpaired"
        }
    );
    for pair in rungs().windows(2) {
        let (lower, higher) = (pair[0], pair[1]);
        let bouts: Vec<Bout> = (0..seeds)
            .flat_map(|seed| bouts_for_seed(higher, lower, seed as u64, None))
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

/// `--no-rollout`: zero `rollout_depth`/`rollout_k` on every live fighter.
///
/// ⭐ THE A/B FOR THE l6 REGRESSION. `--sweep-below` measures l1 and l6 failing to
/// recover from below while l3/l5/l9 succeed, and l6 is exactly where
/// `for_level` switches rollout on — but l9 has the same rollout and recovers, so
/// the correlation needs a controlled test rather than a story. Re-run the sweep
/// with this flag: if l6 then recovers, rollout is the cause; if it still fails,
/// rollout is a coincidence and the suspect list moves on.
///
/// ⚠ It pokes the LIVE cfg, exactly as `force_utility_weights` does, because the
/// published policy is what builds the profile and a rig must not fork it.
fn force_no_rollout(app: &mut bevy::app::App) -> bool {
    use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
    let world = app.world_mut();
    let mut q = world.query::<&mut Brain>();
    let mut found = false;
    for mut brain in q.iter_mut(world) {
        if let Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) = &mut *brain {
            cfg.profile.rollout_depth = 0;
            cfg.profile.rollout_k = 0;
            found = true;
        }
    }
    found
}

/// `--reaction-ms N`: override every live fighter's reaction time.
///
/// ⭐ THE A/B FOR THE l1 FAILURE, which `--no-rollout` does NOT rescue, so it has a
/// different cause. The published ladder runs 500ms at l1 down to 150ms at l9, and
/// the `recovery_below` fixture drops the body **208px above the blastzone**
/// (mapped y=512 on a 640x480 stage whose fall margin puts death at y=720). If the
/// fall is shorter than half a second, l1 simply cannot react in time and its
/// 45/45 is arithmetic rather than a defect. Setting this to 0 answers it: if l1
/// then recovers, reaction time is the whole story.
fn force_reaction_ms(app: &mut bevy::app::App, reaction_ms: f32) -> bool {
    use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
    let world = app.world_mut();
    let mut q = world.query::<&mut Brain>();
    let mut found = false;
    for mut brain in q.iter_mut(world) {
        if let Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) = &mut *brain {
            cfg.profile.reaction_ms = reaction_ms;
            found = true;
        }
    }
    found
}

/// `--apm N` / `--noise X`: override the other two per-level knobs.
///
/// ⭐ THE REMAINING SUSPECTS FOR l1, after `--no-rollout` and `--reaction-ms 0`
/// BOTH left it failing 45/45. `for_level` gives l1 the lowest `apm_cap` (120 —
/// two actions per second, which can throttle a recovery input) and the highest
/// `execution_noise` (0.45). Testing both at once first: if l1 still fails,
/// neither is the cause and the suspect moves out of the profile entirely.
fn force_apm_and_noise(app: &mut bevy::app::App, apm: Option<f32>, noise: Option<f32>) -> bool {
    use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
    let world = app.world_mut();
    let mut q = world.query::<&mut Brain>();
    let mut found = false;
    for mut brain in q.iter_mut(world) {
        if let Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) = &mut *brain {
            if let Some(apm) = apm {
                cfg.profile.apm_cap = apm;
            }
            if let Some(noise) = noise {
                cfg.profile.execution_noise = noise;
            }
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
/// The weight override, or `None` when the caller passed no `--weight`.
///
/// ⛔⛔ **THIS RETURNED `v1()` UNCONDITIONALLY AND EVERY RUN APPLIED IT TO EVERY
/// FIGHTER, WHICH FLATTENED THE DIFFICULTY LADDER THE RIG EXISTS TO MEASURE.**
/// `UtilityWeights::v1()` is not a neutral default — it is *exactly* the LEVEL 9
/// row of `fighter_brain_ladder.ron` (frame_advantage 0.6, kill_potential 0.4,
/// stage_risk -0.8, expected_payoff 0.5). So a "level 1 versus level 3" bout was
/// two fighters with LEVEL 9 PRIORITIES wearing level 1 and level 3 reflexes,
/// and the authored utility ladder — the half that says how much a rung cares
/// about kills and how far it will chase one offstage — was overwritten before
/// the first tick. Every ladder number this rig ever produced measured a ladder
/// that differs only in `reaction_ms`, `apm_cap`, `execution_noise` and
/// `read_weight`.
///
/// ⚠ The old log line called it *"weights: v1 (profile default)"*, which is
/// wrong twice: `v1` is not the profile's default (the profile authors weights
/// PER LEVEL), and "default" reads as "nothing was changed" at exactly the
/// moment something was.
///
/// ⇒ `--weight` still forces, on every fighter, which is what makes the rig
/// usable for a scoring change — the documented intent. Passing none now leaves
/// each rung the weights its level authored.
/// SAY WHICH DIFFICULTY LADDER THIS RUN'S FIGHTERS ACTUALLY GOT.
///
/// ⛔⛔ **EVERY RUN THIS RIG HAS EVER PRODUCED WAS ON THE ENGINE FLOOR AND NO
/// OUTPUT SAID SO.** A rung's profile comes from `profile_for_level`, which
/// prefers `Res<AuthoredFighterLadder>` and falls back to
/// `FighterBrainProfile::for_level` — and that floor sets
/// `utility_weights: UtilityWeights::default()`, which IS `v1()`, for EVERY
/// level. The authored rows are inserted by `ambition_content`, which neither
/// `ambition_demo_smash` nor this crate depends on. So the floor's rungs differ
/// in `reaction_ms`, `apm_cap`, `execution_noise` and `read_weight` and in
/// nothing else, while the game the player runs (`ambition_app`, which DOES
/// compose `ambition_content`) gives its fighters the authored ladder.
///
/// ⇒ **The rig has been measuring a different fighter from the shipped one**, and
/// the only reason that was discoverable at all is that removing an unrelated
/// override changed nothing. This line makes the condition part of the output
/// instead of a property somebody has to go and derive.
///
/// ⚠ It REPORTS rather than repairs, deliberately. Fixing it means deciding who
/// owns Smash's difficulty ladder — `super-smash-siblings.md` puts "CPU-fill/
/// difficulty policy" in what Smash owns, and `for_level`'s own doc says a game
/// that cares ships its own nine rows — but `ambition_content` already inserts
/// one, so a second `insert_resource` would make the winner a plugin-order
/// accident. That is a product decision, not a measurement fix.
fn report_which_ladder_is_in_play() {
    let mut app = build_demo_app();
    app.update();
    let authored = app
        .world()
        .get_resource::<ambition_platformer2d::characters::brain::fighter::AuthoredFighterLadder>()
        .is_some();
    if authored {
        println!("[ladder_rig] ladder: the AUTHORED rows (AuthoredFighterLadder is installed)");
    } else {
        println!(
            "[ladder_rig] ⛔ ladder: the ENGINE FLOOR — no AuthoredFighterLadder in this app, so \
             every rung carries the floor's `UtilityWeights::default()` (== v1, the level-9 row) \
             and differs only in reaction/APM/noise/read. This is NOT the ladder the shipped game \
             gives its fighters."
        );
    }
}

fn weights_from_args(
) -> Option<ambition_platformer2d::characters::brain::fighter::UtilityWeights> {
    if args().weights.is_empty() {
        return None;
    }
    let mut weights = ambition_platformer2d::characters::brain::fighter::UtilityWeights::v1();
    for pair in &args().weights {
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
    Some(weights)
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
        .filter(|s| {
            s.starting_positions().is_some()
                && s.unreproduced_by_placement().iter().all(|what| {
                    *what == "velocity"
                        || *what == "ledge hang"
                        || *what == "projectiles"
                        || (*what == "body phase" && s.starting_hitstun().is_some())
                })
        })
        .collect();
    report_which_ladder_is_in_play();
    // ⛔ THE SCENARIO TABLE NEVER NAMED ITS WEIGHTS. This mode returns before
    // the ladder mode's announcement, so every scenario table ever printed —
    // including the ones quoted into `fighter-brain.md` — travelled without the
    // scoring configuration that produced it. Same rule as the stage below: a
    // number crossing a document boundary carries its method or it is not a
    // measurement.
    match weights_from_args() {
        Some(weights) => println!(
            "[ladder_rig] weights OVERRIDDEN on EVERY fighter: {weights:?} \
             (the authored per-level weights are not in play)"
        ),
        // ⚠ "not overridden", NOT "the authored rows". Those are different
        // claims and the line below is the one that says which rows a rung
        // actually got: this rig prints both, and an earlier wording had them
        // contradicting each other on consecutive lines.
        None => println!(
            "[ladder_rig] weights: not overridden — each rung keeps whatever its \
             profile source gave it (see the ladder line below)"
        ),
    }
    println!(
        // ⛔ THE STAGE IS IN THE HEADER because it stopped being a constant.
        // Every number below depends on it, and a table that does not name the
        // layout it was measured on cannot be compared with another one — which
        // is the entire reason `--stage` exists.
        "[ladder_rig] --scenarios: PLACEMENT ONLY — {} of {} fixture(s) are \
         reproduced by placing two bodies (median of {seeds} seeds, {}s each, \
         stage `{}`, {}, rungs {})",
        playable.len(),
        suite.len(),
        TICKS / 60,
        args().stage,
        // ⛔ THE DESIGN IS PART OF THE NUMBER. A paired table and an unpaired one
        // answer the same question with different controls, and two runs whose
        // headers do not say which cannot be compared.
        if args().paired {
            "PAIRED — each seed run twice with the rungs swapped between seats, \
             tested on within-seed differences"
        } else {
            "unpaired — seat 0 is always the higher rung"
        },
        // The rungs too, now that they are a flag: a null-control run (`6,6`)
        // and a ladder run otherwise print identical columns.
        rungs()
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    // ⛔ THE SCENARIO TABLE PRINTED NO COLUMN HEADER AT ALL, so every reader had
    // to infer five columns from the numbers — and `stocks` was read as "stocks
    // lost" in a planning row, inverting what the rows meant.
    println!(
        "[ladder_rig] fixture            rungs     survived(hi:lo)                stocks LEFT   dealt%(hi:lo)     peak%(hi:lo)     verdict = who OUTFOUGHT (stocks taken, then damage DEALT)"
    );
    for scenario in &suite {
        if scenario.starting_positions().is_none() {
            println!(
                "[ladder_rig]   {:<22} SKIPPED (no opponent — not a bout)",
                scenario.name
            );
            continue;
        }
        // ⭐ `velocity` no longer disqualifies a fixture: `place_at` sets it
        // through `TransitVelocity::Set`. Everything else this rig still cannot
        // arrange — body phase, projectiles, a ledge hang — remains a skip, and
        // the message still names exactly what is missing.
        let phase_is_hitstun_only = scenario.starting_hitstun().is_some();
        let missing: Vec<&'static str> = scenario
            .unreproduced_by_placement()
            .into_iter()
            .filter(|what| *what != "velocity")
            .filter(|what| !(*what == "body phase" && phase_is_hitstun_only))
            .filter(|what| *what != "ledge hang")
            .filter(|what| *what != "projectiles")
            .collect();
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
        for pair in rungs().windows(2) {
            let (lower, higher) = (pair[0], pair[1]);
            let bouts: Vec<Bout> = (0..seeds)
                .flat_map(|seed| bouts_for_seed(higher, lower, seed as u64, Some(scenario)))
                .collect();
            report_row(
                &format!("{:<18} {higher:>2} vs {lower:<2}", scenario.name),
                &bouts,
            );
        }
    }
}

/// `--sweep-below`: vary ONLY the level of the fighter placed below the stage.
///
/// ⭐ WHY THIS EXISTS. `--scenarios` walks `RUNGS.windows(2)`, which moves BOTH
/// seats at once and yields four points for `recovery_below` — and at four points
/// a threshold, a monotone trend and a scatter are indistinguishable. Two of the
/// four fail totally (45/45 unfought) and the pattern is non-monotonic in every
/// parameter `for_level` varies, so the honest next step is more points with one
/// variable moving.
///
/// The partner is pinned at level 5 so the only thing changing between rows is
/// the profile of the body that has to recover.
fn run_sweep_below(seeds: usize) {
    const PARTNER: u8 = 5;
    let scenario = ambition_platformer2d::combat::brain::fighter::scenarios::suite()
        .into_iter()
        .find(|s| s.name == "recovery_below")
        .expect("the suite authors recovery_below");
    println!(
        "[ladder_rig] --sweep-below: `recovery_below`, partner pinned at l{PARTNER}, \
         median of {seeds} seeds, {}s each. Read `unfought n/{seeds}`: that is the \
         count of bouts where NEITHER seat landed a hit, which is what a failure to \
         recover looks like.",
        TICKS / 60
    );
    println!(
        "[ladder_rig] fixture            rungs     survived(hi:lo)                stocks LEFT   dealt%(hi:lo)     peak%(hi:lo)     verdict = who OUTFOUGHT (stocks taken, then damage DEALT)"
    );
    // ⛔ PUBLISHED LEVELS ONLY. `smash_roster_at_levels` builds a
    // `duelist_l{level}` policy key, and only 1/3/5/6/9 are published in
    // `SMASH_CATALOG_RON` — asking for `l2` refuses the seat, nothing ever gets
    // seated, and the bout measures the default spawn. ⭐ Which is exactly what
    // the `placed` assert caught when this swept 1..=9: a loud stop rather than
    // nine rows of a fixture that never applied.
    for below in RUNGS.iter().copied() {
        let bouts: Vec<Bout> = (0..seeds)
            .flat_map(|seed| bouts_for_seed(below, PARTNER, seed as u64, Some(&scenario)))
            .collect();
        report_row(
            &format!("{:<18} {below:>2} vs {PARTNER:<2}", "recovery_below"),
            &bouts,
        );
    }
}

fn seed_count() -> usize {
    args().seeds.unwrap_or(DEFAULT_SEEDS)
}

/// The value the caller gave for `name`, from the parsed surface above.
fn flag_value(name: &str) -> Option<String> {
    let a = args();
    match name {
        "--seeds" => a.seeds.map(|v| v.to_string()),
        "--reaction-ms" => a.reaction_ms.map(|v| v.to_string()),
        "--apm" => a.apm.map(|v| v.to_string()),
        "--noise" => a.noise.map(|v| v.to_string()),
        "--character" => a.character.clone(),
        "--opponent" => a.opponent.clone(),
        other => unreachable!("ladder_rig asked for an unmapped flag: {other}"),
    }
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

/// How close the fighters got, across the bouts of this row that ended untouched.
///
/// ⭐ **THE DIAGNOSIS THE `unfought` COUNT COULD NOT GIVE.** "Neither landed a
/// hit" has two causes that are fixed in different places: they never reached
/// each other (navigation), or they reached each other and declined to commit
/// (scoring). A median closest approach around a body width says the second; one
/// in the hundreds of px says the first. The platformed stage's 41 unfought
/// bouts against the flat stage's 3 is the measurement that wanted this.
fn approach_of_the_unfought(bouts: &[Bout]) -> String {
    const FOUGHT_AT_ALL: f32 = 0.01;
    let mut d: Vec<f32> = bouts
        .iter()
        .filter(|b| b.peak_percent[0] < FOUGHT_AT_ALL && b.peak_percent[1] < FOUGHT_AT_ALL)
        .map(|b| b.closest_approach)
        .filter(|d| d.is_finite())
        .collect();
    if d.is_empty() {
        // Either no unfought bout, or none where both bodies ever coexisted.
        // Both are honestly "no distance to report" rather than zero.
        return "—".to_string();
    }
    d.sort_by(f32::total_cmp);
    format!("{:.0}px", median(d))
}

fn report(higher: u8, lower: u8, bouts: &[Bout]) {
    report_row(&format!("{higher:>2} vs {lower:<2}"), bouts);
}

/// One line, under whatever label the caller is grouping by.
fn report_row(label: &str, bouts: &[Bout]) {
    let hi_all: Vec<f32> = bouts.iter().map(|b| b.eliminated[0] as f32).collect();
    let lo_all: Vec<f32> = bouts.iter().map(|b| b.eliminated[1] as f32).collect();
    // The survival medians are no longer computed here: `span` derives its own
    // for the column it prints, and nothing else wants them now that the verdict
    // is the outcome rather than the clock. Keeping them would be two authors of
    // the same number.
    let hi_stocks = median(bouts.iter().map(|b| b.stocks[0] as f32).collect());
    let lo_stocks = median(bouts.iter().map(|b| b.stocks[1] as f32).collect());
    // ⛔⛔ **THE VERDICT IS WHAT A SEAT DID TO THE OTHER ONE, NOT HOW LONG IT
    // AVOIDED BEING HIT.** This read "the seat that lasted LONGER won", and the
    // 15-seed matrix (2026-09-03, `fighter-brain.md`) showed that scoreboard
    // cannot rank skill at all: **35 of 36 verdicts landed inside the seed
    // spread**, and the two reasons were both visible in these very columns.
    //
    // Survival-until-a-cap SATURATES AT BOTH ENDS and pays for passivity in the
    // middle. At the low rungs every fixture returned "both survive" — 60s is
    // not long enough for weak CPUs to resolve anything, so half the matrix was
    // structurally unable to answer. At the high rungs the stocks columns were
    // `0 : 0` almost everywhere: stronger CPUs took FEWER stocks, because a
    // fighter that never commits cannot be punished and therefore outlasts one
    // that fights.
    //
    // ⇒ Score the OUTCOME instead, lexicographically: stocks taken off the
    // opponent first, damage dealt to it as the tiebreak. Both are already
    // collected. Stocks are the thing the game is played for; damage is
    // continuous and never saturates, which is what lets a row discriminate when
    // neither seat closed a stock. Survival keeps its column — it is still the
    // honest answer to "how long did this last" — it just stops being the
    // verdict.
    //
    // ⚠ The damage term is `damage_taken`, SUMMED FROM PER-TICK RISES, and not
    // `peak_percent`. Peak is the most a seat ever carried at once; percent
    // resets on death, so peak systematically under-reads the fighter who died
    // more and, as a tiebreak, rewards needing MORE damage to close a stock.
    let dealt = |seat: usize| median(bouts.iter().map(|b| b.damage_taken[1 - seat]).collect());
    let stocks_taken = |seat: usize| {
        median(
            bouts
                .iter()
                .map(|b| (ambition_demo_smash::STARTING_STOCKS - b.stocks[1 - seat]) as f32)
                .collect(),
        )
    };
    let (hi_took, lo_took) = (stocks_taken(0), stocks_taken(1));
    let (hi_dealt, lo_dealt) = (dealt(0), dealt(1));
    let verdict = if hi_took != lo_took {
        if hi_took > lo_took {
            "higher outfights"
        } else {
            "LOWER outfights"
        }
    } else if hi_dealt > lo_dealt {
        "higher outfights"
    } else if lo_dealt > hi_dealt {
        "LOWER outfights"
    } else {
        "even"
    };
    // a verdict inside the seeds' own spread is not a verdict. Reported
    // rather than suppressed: the reader should see the overlap and discount the
    // word, not be handed a cleaner-looking table.
    //
    // ⚠ Measured on the DECIDING quantity. It used to test the survival times
    // while the word above described survival; now the word describes damage
    // dealt, so the spread that matters is damage's. Leaving it on the old
    // column would have marked a decisive damage gap "within spread" whenever
    // the two seats happened to die at similar times.
    let hi_dealt_all: Vec<f32> = bouts.iter().map(|b| b.damage_taken[1]).collect();
    let lo_dealt_all: Vec<f32> = bouts.iter().map(|b| b.damage_taken[0]).collect();
    // ⭐ PAIRED RUNS ARE TESTED ON THE DIFFERENCES, NOT ON TWO POOLED MEDIANS.
    // `--paired` emits consecutive (straight, mirrored) bouts of ONE seed, so the
    // within-seed difference in damage dealt is available and it is the whole
    // reason to pay double: seed-to-seed variance appears in both halves of a
    // pair and cancels in the difference, while a pooled median still carries it.
    // Testing pooled medians on paired data would spend the extra bouts and keep
    // the variance that made every cell `(within spread)`.
    // ⛔⛔ AND IT REFUSES DATA THAT IS NOT ACTUALLY PAIRED. When `--paired` was a
    // no-op in the ladder mode, this branch still ran: `chunks_exact(2)` over an
    // ODD, unpaired vector formed one chunk (or none), the "range" of a single
    // difference is zero, and `|mid| < 0.5 * 0` is false for every row — so every
    // verdict printed WITHOUT its `(within spread)` qualifier and the table
    // looked decisive everywhere. A significance test that reports significance
    // when its input is malformed is worse than no test, so the shape is checked
    // rather than assumed.
    let properly_paired = args().paired && bouts.len() >= 4 && bouts.len() % 2 == 0;
    if args().paired && !properly_paired {
        println!(
            "[ladder_rig] ⛔ {label}: --paired asked for, but this row has {} bout(s) \
             — not an even number of at least two pairs. Falling back to the \
             unpaired spread test rather than testing a difference that does not \
             exist.",
            bouts.len()
        );
    }
    let overlaps = if properly_paired {
        let diffs: Vec<f32> = bouts
            .chunks_exact(2)
            .map(|pair| {
                let dealt = |b: &Bout, seat: usize| b.damage_taken[1 - seat];
                (dealt(&pair[0], 0) + dealt(&pair[1], 0)) / 2.0
                    - (dealt(&pair[0], 1) + dealt(&pair[1], 1)) / 2.0
            })
            .collect();
        let mid = median(diffs.clone());
        let lo = diffs.iter().copied().fold(f32::INFINITY, f32::min);
        let hi = diffs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        mid.abs() < 0.5 * (hi - lo)
    } else {
        (hi_dealt - lo_dealt).abs()
            < 0.5
                * ((hi_dealt_all.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                    - hi_dealt_all.iter().copied().fold(f32::INFINITY, f32::min))
                .max(
                    lo_dealt_all.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                        - lo_dealt_all.iter().copied().fold(f32::INFINITY, f32::min),
                ))
    };
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
    // ⛔ THE LABEL BELOW IS COMPUTED ON MEDIANS, AND THE OUTCOME IT DESCRIBES IS
    // BIMODAL — a bout either ends untouched or turns into a real fight. A stable
    // 50/50 split produces a stable median too, so "NEITHER LANDED A HIT" on its
    // own cannot distinguish "every bout was unfought" from "just over half were".
    // ⇒ report the COUNT beside the label, so the reader can tell which.
    let unfought = bouts
        .iter()
        .filter(|b| b.peak_percent[0] < FOUGHT_AT_ALL && b.peak_percent[1] < FOUGHT_AT_ALL)
        .count();
    let verdict = if hi_peak < FOUGHT_AT_ALL && lo_peak < FOUGHT_AT_ALL {
        format!(
            "{verdict} — BUT NEITHER LANDED A HIT (unfought {unfought}/{}, closest {})",
            bouts.len(),
            approach_of_the_unfought(bouts)
        )
    } else if unfought > 0 {
        // The other half of the same point: a row that reads as a normal fight can
        // still be hiding bouts that ended untouched — now with the reason
        // attached, because "nobody hit anybody" has two very different causes.
        format!(
            "{verdict} [unfought {unfought}/{}, closest {}]",
            bouts.len(),
            approach_of_the_unfought(bouts)
        )
    } else {
        verdict
    };
    println!(
        "[ladder_rig]   {label:<26} {:>20} : {:<20} {hi_stocks:>3.0} : {lo_stocks:<3.0}   \
         {:>6.0}% : {:<6.0}%   {:>6.1}% : {:<6.1}%  {verdict}",
        span(&hi_all),
        span(&lo_all),
        // ×100 HERE and nowhere else. The ratio is what every other reader
        // of `damage_percent` wants; a percentage is a display concern, and
        // baking it into the stored column is how the threshold above came to be
        // written in the wrong units.
        // ⛔ THE DECIDING COLUMN. The verdict ranks stocks taken and then damage
        // DEALT, and neither was visible: the peak column beside it answers a
        // different question (the most a seat ever CARRIED), and a reader given
        // a verdict whose evidence is not on the row can only take it on trust.
        // Dealt by a seat is what the OTHER one absorbed, so the indices cross.
        hi_dealt * 100.0,
        lo_dealt * 100.0,
        hi_peak * 100.0,
        lo_peak * 100.0
    );
}

/// Every bout one seed contributes, honouring `--paired`.
///
/// ⛔⛔ **THIS EXISTS BECAUSE `--paired` WAS WIRED INTO ONE OF THE THREE MODES AND
/// SILENTLY DID NOTHING IN THE OTHERS.** The scenarios loop paired; the ladder
/// and below-sweep loops kept calling `run_bout` once per seed. A `--paired`
/// ladder run therefore produced numbers IDENTICAL to an unpaired one — which is
/// how it was caught, by running both and diffing — while the header claimed a
/// design it had not used. One function now owns "a seed becomes these bouts",
/// so a mode added later cannot forget.
fn bouts_for_seed(
    higher: u8,
    lower: u8,
    seed: u64,
    start: Option<&ambition_platformer2d::combat::brain::fighter::scenarios::Scenario>,
) -> Vec<Bout> {
    let straight = run_bout_at(higher, lower, seed, start.cloned());
    if !args().paired {
        return vec![straight];
    }
    // ⛔⛔ PAIRING A RUNG WITH ITSELF IS A TAUTOLOGY, and it looks like a clean
    // null control, which is how it fooled its own author. With `higher ==
    // lower` the swapped call below is the SAME call, so the pair is `[B,
    // B.mirrored()]` — a bout averaged with its own transpose. The columns come
    // out equal by construction, for any bout, on a biased instrument as
    // readily as an unbiased one. ⇒ Run `--rungs X,X` WITHOUT `--paired` to
    // measure the seat term; the paired form measures nothing.
    if higher == lower {
        eprintln!(
            "[ladder_rig] ⛔ --paired with a rung against itself ({higher} vs {lower}) is \
             degenerate: the mirrored bout is the SAME bout, so equal columns are \
             guaranteed and prove nothing about bias. Drop --paired to measure the \
             seat term."
        );
    }
    // ⛔ THE SAME SEED, THE ROLES SWAPPED, AND THE RESULT PUT BACK THE RIGHT WAY
    // ROUND. `run_bout_at(lower, higher, ..)` seats the LOWER rung where the
    // fixture puts SELF, so `mirrored` swaps the pair back and every `[0]` below
    // still means "the higher rung". Reporting the raw mirror would average each
    // rung with the other one.
    let swapped = run_bout_at(lower, higher, seed, start.cloned());
    vec![straight, swapped.mirrored()]
}

impl Bout {
    /// The same bout read from the OTHER seat's side.
    ///
    /// Used by `--paired`, where the second run of a seed puts the lower rung in
    /// seat 0. Every per-seat array is swapped so index 0 keeps meaning "the
    /// higher rung" for the caller, which is the only way a paired vector can be
    /// summarised by the same reporter as an unpaired one.
    fn mirrored(self) -> Self {
        Self {
            eliminated: [self.eliminated[1], self.eliminated[0]],
            stocks: [self.stocks[1], self.stocks[0]],
            peak_percent: [self.peak_percent[1], self.peak_percent[0]],
            damage_taken: [self.damage_taken[1], self.damage_taken[0]],
            // Symmetric between the seats, so the mirror leaves it alone.
            closest_approach: self.closest_approach,
        }
    }
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
fn place_at(
    app: &mut bevy::app::App,
    me: ae::Vec2,
    foe: ae::Vec2,
    velocities: Option<(ae::Vec2, ae::Vec2)>,
    hitstun: Option<(f32, f32)>,
    ledge_hangs: Option<(bool, bool)>,
    shots: &[(ae::Vec2, ae::Vec2)],
) -> bool {
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
        // `Zero` by default, because a body carrying the spawn's fall speed
        // into a "standing at the ledge" premise is not in that premise.
        //
        // ⭐ BUT A SCENARIO MAY ASK FOR A VELOCITY, and `TransitVelocity::Set`
        // is how the authority accepts one — the same road, not a field write.
        // Before this the rig could only place, so every fixture whose premise
        // included motion was skipped as "cannot set up: velocity". Measured
        // 2026-09-03: that was 3 of the 4 skips, and `edgeguard_window` needed
        // nothing else.
        let velocity = match velocities {
            Some((me_vel, foe_vel)) => {
                TransitVelocity::Set(if seat.0 == 0 { me_vel } else { foe_vel })
            }
            None => TransitVelocity::Zero,
        };
        transit_body(&mut model, &mut clusters, target, velocity);
    }
    // ⭐ HITSTUN IS A TIMER, NOT AN ENUM. `BodyPhase` is derived — the runtime's
    // `body_phase()` reads it from `BodyCombat.hitstun_timer` — so a fixture
    // that starts a body "in hitstun" is reproduced by writing the timer the
    // phase is computed FROM. Writing a phase field would be writing the
    // thermometer.
    //
    // ⚠ Separate pass because it is a different component: `transit_body` owns
    // pose and velocity, and nothing about hitstun is a transit.
    // ⭐ A REAL BOLT, NOT A FABRICATED ONE. The fixture's premise is "an
    // opponent at range with a shot in the air"; its `damage: 3` describes its
    // own 800x600 stage the way its coordinates do. So the rig fires the volley
    // ability's OWN authored spec (`abilities::ranged::volley::authored_bolt`)
    // from the foe toward the subject, and maps the fixture's offset the same
    // way `starting_positions_on` maps its positions. Building a
    // `ProjectileSpawn` out of the fixture's numbers would stage a projectile no
    // ability authors.
    if !shots.is_empty() {
        use ambition_platformer2d::projectiles::spawn_request::{
            ProjectileSpawnRequest, ProjectileStart,
        };
        let world = app.world_mut();
        let mut seats = world.query::<(&MatchSeat, BodyClusterQueryData)>();
        let mut subject = None;
        let mut shooter = None;
        for (seat, cluster) in seats.iter(world) {
            if seat.0 == 0 {
                subject = Some(cluster.kinematics.pos);
            } else {
                shooter = Some(cluster.kinematics.pos);
            }
        }
        if let (Some(subject_pos), Some(_)) = (subject, shooter) {
            let mut foes =
                world.query_filtered::<bevy::prelude::Entity, bevy::prelude::With<MatchSeat>>();
            let owner = foes.iter(world).last();
            if let Some(owner) = owner {
                for (offset, dir) in shots {
                    let origin = subject_pos + *offset;
                    world.write_message(ProjectileSpawnRequest::open(
                        owner,
                        ambition_platformer2d::abilities::ranged::volley::authored_bolt(
                            origin, *dir,
                        ),
                        ProjectileStart::StepThisTick,
                    ));
                }
            }
        }
    }
    // ⭐ A HANG IS NOT A POSITION, so it is arranged AFTER `transit_body` —
    // which clears `ledge_grab` on purpose (`reconcile_transit`: "the ledge
    // anchor was a fact of the departure point"). Setting it before the transit
    // would be undone by the transit.
    //
    // The anchor comes from the REAL platform, not from the fixture's stage:
    // `smash_stage().world.blocks[0]` is the one thing you can stand on, and the
    // ledge is its top corner on the side the fixture put the body. Guessing the
    // geometry would stage a body hanging in mid-air, which is a fixture staging
    // something its premise did not describe.
    if let Some((me_hangs, foe_hangs)) = ledge_hangs {
        use ambition_platformer2d::engine_core::ledge_grab::{LedgeContact, LedgeGrabState};
        use ambition_platformer2d::engine_core::AabbExt as _;
        let platform = ambition_demo_smash::smash_stage().world.blocks[0].aabb;
        let centre = platform.center();
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
        )>();
        for (seat, mut cluster_item, mut model) in q.iter_mut(world) {
            let hangs = if seat.0 == 0 { me_hangs } else { foe_hangs };
            if !hangs {
                continue;
            }
            let mut clusters = cluster_item.as_clusters_mut();
            // Which ledge: the side the fixture placed the body on.
            let on_left = clusters.kinematics.pos.x < centre.x;
            let edge_x = if on_left { platform.left() } else { platform.right() };
            let contact = LedgeContact {
                // +1 = wall on the player's LEFT. Hanging off the platform's
                // left edge puts the wall on the player's RIGHT, hence -1.
                wall_normal_x: if on_left { -1.0 } else { 1.0 },
                anchor: ae::Vec2::new(edge_x, platform.top()),
                climb_target: ae::Vec2::new(edge_x, platform.top()),
            };
            // Snap to the anchor through the authority, then declare the hang.
            transit_body(
                &mut model,
                &mut clusters,
                contact.anchor,
                TransitVelocity::Zero,
            );
            if let ambition_platformer2d::actor::MotionModel::AxisSwept(axis) = &mut *model {
                axis.state.ledge_grab = Some(LedgeGrabState::hanging(contact));
            }
        }
    }
    if let Some((me_stun, foe_stun)) = hitstun {
        let world = app.world_mut();
        let mut q = world
            .query::<(&MatchSeat, &mut ambition_platformer2d::characters::actor::BodyCombat)>();
        for (seat, mut combat) in q.iter_mut(world) {
            let seconds = if seat.0 == 0 { me_stun } else { foe_stun };
            if seconds > 0.0 {
                combat.hitstun_timer = seconds;
            }
        }
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
    // ⛔ BEFORE the route below, because the route is what prepares the session:
    // the preparation source reads this resource once, when the match is asked
    // for. Setting it afterwards would change nothing and look like it worked.
    app.world_mut()
        .insert_resource(match args().stage.trim().to_ascii_lowercase().as_str() {
            "platforms" => ambition_demo_smash::SmashStageChoice::Platforms,
            "flat" | "" => ambition_demo_smash::SmashStageChoice::Flat,
            other => panic!(
                "unknown --stage {other:?}; the rig fights on `flat` or `platforms`. \
                 Defaulting would silently measure a stage nobody asked for, and \
                 the stage is exactly the variable this flag exists to control."
            ),
        });
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
    let mut damage_taken = [0.0f32; 2];
    // Starts at infinity so the first tick with both bodies present sets it; a
    // bout where they never coexist keeps it, and `report_row` prints it as `—`.
    let mut closest_approach = f32::INFINITY;
    let mut last_percent = [0.0f32; 2];
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
                // Only when the caller asked. Forcing unconditionally is what
                // flattened the ladder; see `weights_from_args`.
                if let Some(weights) = weights {
                    force_utility_weights(&mut app, weights);
                }
                if args().no_rollout {
                    force_no_rollout(&mut app);
                }
                if let Some(ms) = flag_value("--reaction-ms").and_then(|v| v.parse().ok()) {
                    force_reaction_ms(&mut app, ms);
                }
                let apm = flag_value("--apm").and_then(|v| v.parse().ok());
                let noise = flag_value("--noise").and_then(|v| v.parse().ok());
                if apm.is_some() || noise.is_some() {
                    force_apm_and_noise(&mut app, apm, noise);
                }
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
                let velocities = scenario.starting_velocities();
                let hitstun = scenario.starting_hitstun();
                let ledge_hangs = scenario.starting_ledge_hangs();
                let shots = scenario.starting_shots();
                placed = stage_bounds(&mut app)
                    .and_then(|bounds| scenario.starting_positions_on(bounds))
                    .is_some_and(|(me, foe)| {
                        place_at(&mut app, me, foe, velocities, hitstun, ledge_hangs, &shots)
                    });
            }
        }
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &FighterStocks,
            &ambition_platformer2d::characters::actor::BodyHealth,
            &ambition_platformer2d::engine_core::BodyKinematics,
        )>();
        let mut seen = [false; 2];
        let mut at: [Option<ae::Vec2>; 2] = [None, None];
        for (seat, remaining, health, kin) in q.iter(world) {
            if seat.0 < 2 {
                seen[seat.0] = true;
                at[seat.0] = Some(kin.pos);
                stocks[seat.0] = remaining.remaining;
                let now = health.damage_percent();
                peak_percent[seat.0] = peak_percent[seat.0].max(now);
                // Only the RISES. A death resets the percent, so the step is
                // negative there and contributes nothing — which is what makes
                // this a total across stocks rather than a reading of the last
                // one.
                damage_taken[seat.0] += (now - last_percent[seat.0]).max(0.0);
                last_percent[seat.0] = now;
            }
        }
        // ⛔ BOTH SEATS OR NOTHING. A tick where one body is absent — mid-seating,
        // or eliminated — has no separation to speak of, and folding a distance
        // to a missing body in would make "they never met" unmeasurable exactly
        // when a fighter is dead.
        if let (Some(a), Some(b)) = (at[0], at[1]) {
            closest_approach = closest_approach.min(a.distance(b));
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
        damage_taken,
        closest_approach,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bout() -> Bout {
        Bout {
            eliminated: [100, 200],
            stocks: [1, 2],
            peak_percent: [0.5, 1.5],
            damage_taken: [10.0, 30.0],
            closest_approach: 48.0,
        }
    }

    /// ⛔ THE MIRROR MUST PUT THE SEATS BACK, AND ITS FAILURE LOOKS LIKE SUCCESS.
    ///
    /// `--paired` runs the second half of each seed as `run_bout_at(lower,
    /// higher, ..)`, which seats the LOWER rung where the fixture puts SELF. If
    /// that result were reported unmirrored, every pair would average each rung
    /// with the other one and the table would fill with balanced-looking rows
    /// and near-zero differences — a *more* convincing table than the truth, and
    /// wrong. Every per-seat array has to swap, so a field added later without
    /// being swapped is caught here rather than by somebody wondering why
    /// pairing made the effect vanish.
    #[test]
    fn mirroring_a_bout_swaps_every_per_seat_reading() {
        let m = bout().mirrored();
        assert_eq!(m.eliminated, [200, 100]);
        assert_eq!(m.stocks, [2, 1]);
        assert_eq!(m.peak_percent, [1.5, 0.5]);
        assert_eq!(m.damage_taken, [30.0, 10.0]);
        // ⚠ NOT swapped, and deliberately asserted: the separation between two
        // bodies is symmetric, so mirroring must leave it alone. A "swap every
        // field" reflex would corrupt nothing visible here and quietly make the
        // value meaningless the day it becomes per-seat.
        assert_eq!(m.closest_approach, 48.0);
    }

    /// Mirroring twice is the identity — the property that says the swap is a
    /// permutation and not a rewrite.
    #[test]
    fn mirroring_twice_is_the_original_bout() {
        let once = bout().mirrored();
        let twice = once.mirrored();
        let orig = bout();
        assert_eq!(twice.eliminated, orig.eliminated);
        assert_eq!(twice.stocks, orig.stocks);
        assert_eq!(twice.peak_percent, orig.peak_percent);
        assert_eq!(twice.damage_taken, orig.damage_taken);
    }

    /// A PAIR OF MIRRORED BOUTS CARRIES NO SEAT ADVANTAGE.
    ///
    /// The property `--paired` is bought for: if a seat is worth something on its
    /// own — and 7 of the 9 fixtures place seat 0 offstage — a straight bout and
    /// its mirror give that advantage to each rung exactly once, so the pair's
    /// mean is free of it. Stated as arithmetic on a bout whose whole difference
    /// IS the seat.
    #[test]
    fn a_mirrored_pair_cancels_a_pure_seat_effect() {
        // A bout decided entirely by which seat you are in: seat 0 always deals
        // 10, seat 1 always deals 30, whoever is sitting there.
        let straight = bout();
        let mirrored = bout().mirrored();
        let dealt = |b: &Bout, seat: usize| b.damage_taken[1 - seat];
        let hi = (dealt(&straight, 0) + dealt(&mirrored, 0)) / 2.0;
        let lo = (dealt(&straight, 1) + dealt(&mirrored, 1)) / 2.0;
        assert_eq!(
            hi, lo,
            "a pure seat effect survived the pairing, so `--paired` is not \
             cancelling the thing it exists to cancel"
        );
    }
}
