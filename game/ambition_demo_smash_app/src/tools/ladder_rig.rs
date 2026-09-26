//! Compare adjacent registered AI ladder rungs in CPU-vs-CPU matches.
//!
//! `cargo run -p ambition_demo_smash_app --bin smash_tool -- ladder-rig [--seeds N] [--weight name=value ...]`
//!
//! `--weight` makes this a rig for a scoring change, not only for a ladder. A
//! weight refit runs the same bouts at the same seeds with one number moved.
//! Run twice and compare; the header names the weights of each run.
//!
//! Other flags, and what each one controls for:
//!
//! - `--paired` runs each seed twice with one term swapped between the seats
//!   and tests the within-seed difference. This removes seed variance and
//!   cancels the seat/placement confound. `Pairing::of` selects the term:
//!   unequal rungs swap the rungs; one rung with two fighters swaps the
//!   fighters; one rung with one fighter swaps the two seats' noise streams
//!   (the seat null control).
//! - `--stage <name>` selects the layout, spelled as the select screen's stage
//!   button (`flat`, `platforms`, `narrow`). It resolves through
//!   `SmashStageChoice::ALL`. The layout changes lethality, so compare runs
//!   only on the same stage.
//! - `--no-rollout` sets `rollout_depth`/`rollout_k` to zero on every fighter.
//!   Rollout is already off below level 6, so the bottom rungs must be
//!   identical between arms. Any change at `6 vs 5` or `9 vs 6` comes from
//!   the rollout.
//!
//! Every table names its stage, weights, design, and the ladder the fighters
//! got.
//!
//! The registered ladder is sparse: levels 1, 3, 5, 6, and 9. The rig reports
//! time to elimination, stocks remaining, and engagement evidence for each pair,
//! using medians across deterministic seeds. Unregistered levels are invalid for
//! this measurement because their generic fallback does not represent a ladder rung.

use crate::build_demo_app;
use ambition_platformer2d::actor::{FighterStocks, MatchSeat};
use ambition_platformer2d::engine_core as ae;

/// The shipped match budget. The rig uses the demo's own constant so that it
/// measures the shipped match. A shorter clock ends bouts before any stock
/// is taken, and then every verdict falls through to the damage tiebreak.
/// `--seconds` shortens it for quick runs; the header names the clock.
const DEFAULT_TICKS: usize = ambition_demo_smash::SMASH_TIME_LIMIT_TICKS as usize;

/// The match budget this run is using, in ticks.
///
/// The match budget of this run, in ticks.
///
/// The verdict is "stocks taken, then damage dealt". When stocks tie, a
/// "weaker" rung may only deal less damage per minute, for example because it
/// refuses bad commitments. A longer clock separates "weaker" from "patient".
/// See `awaiting-maintainer-decision.md`.
///
/// A run at a non-default clock is not comparable to one at the default.
fn ticks() -> usize {
    args().seconds.map_or(DEFAULT_TICKS, |s| s.max(1) * 60)
}

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
                // Do not fall back to the default list: the header would then name a
                // ladder that the run did not measure.
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

/// Seeds per row. Fewer seeds make verdicts noise with a direction. Fifteen
/// seeds take about twenty minutes.
const DEFAULT_SEEDS: usize = 15;

/// What one match said.
///
/// Use elapsed time rather than stocks because stock counts saturate when both
/// seats lose all lives and cannot distinguish match quality.
#[derive(Clone, Copy, Debug)]
struct Bout {
    /// Tick each seat was eliminated on, or [`ticks()`] for a seat that survived.
    /// The later one won.
    eliminated: [usize; 2],
    /// Stocks remaining at the end. The time column cannot tell a survivor
    /// with three stocks from one with one.
    stocks: [u32; 2],
    /// Highest damage each seat ever carried, as a ratio of its pool.
    /// `1.69` is 169%, as `BodyHealth::damage_percent` documents. The `×100`
    /// is applied at the one print site.
    ///
    /// This column shows whether the fight happened. A pair whose peaks stay
    /// near zero never fought, whatever its verdict says.
    peak_percent: [f32; 2],
    /// Total damage each seat absorbed across the whole match, summed from
    /// per-tick increases, as a ratio like [`Self::peak_percent`].
    ///
    /// Do not use the peak as damage dealt. Percent resets on death, so a seat
    /// killed three times at 100% shows a lower peak than a seat pressured to
    /// 250% and never killed. The sum of increases counts every point landed.
    damage_taken: [f32; 2],
    /// The closest the two seats ever came, in world px.
    ///
    /// This tells "they never met" (a pathing problem) from "they met and did
    /// not commit" (a scoring problem). A bout whose closest approach is about
    /// a body width met; one that stayed hundreds of pixels apart did not.
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
    ///
    /// This replaces the whole weight set on every rung with
    /// `UtilityWeights::v1()` plus your fields. `v1()` is the shipped rung-9
    /// row, so on a `--ladder` run it flattens the authored weight ramp and
    /// leaves only reaction/APM/noise/read to separate the rungs. That is a
    /// valid controlled arm, but it is not "the shipped ladder with one number
    /// moved". The header prints every rung after the override.
    ///
    /// To move a weight relative to the ladder, use [`Self::weight_scales`].
    #[arg(long = "weight", value_name = "NAME=VALUE")]
    pub weights: Vec<String>,

    /// Multiply one utility weight on every rung by a factor, as
    /// `NAME=FACTOR`. Repeatable.
    ///
    /// Use this for a refit: it keeps the authored ramp. For example,
    /// `--weight-scale kill_potential=1.3` gives rung 1 `0.00`, rung 5 `0.91`,
    /// and rung 9 `1.495`. `--weight kill_potential=1.3` would give all rungs
    /// `1.3`.
    ///
    /// A weight authored as zero cannot move (rungs 1 and 2 author
    /// `kill_potential: 0.00`). To change it, use `--weight` or edit the `.ron`.
    ///
    /// Applied after `--weight`, so passing both scales the value you set.
    #[arg(long = "weight-scale", value_name = "NAME=FACTOR")]
    pub weight_scales: Vec<String>,
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
    /// Null control: `--rungs 6,6` alone is not one. With no
    /// `--character`/`--opponent` the row seats the demo's two default ids, and
    /// they are different bodies (`smash_duelist_a` wears `player_robot_v3`,
    /// `smash_duelist_b` wears `player_robot_v2`, with different hurtboxes and
    /// animation sets). So `--rungs 6,6 --paired` on the defaults is a fighter
    /// comparison.
    ///
    /// The null control is `--rungs X,X --character F --opponent F --paired`.
    /// It pairs by swapping the seats' noise streams (see [`Mirror::Noise`]).
    /// Any result other than `even` there is the seat term, and every ladder
    /// verdict carries it. Measured on rung 6, shipped ladder, 40 paired seeds:
    ///
    /// ```text
    /// smash_duelist_a     seat0 17 : 6  seat1   (+17 tied)   p = 0.035
    /// smash_duelist_b     seat0 11 : 4  seat1   (+25 tied)   p = 0.118  (within spread)
    /// smash_george_booul  seat0 12 : 8  seat1   (+20 tied)   p = 0.503  (within spread)
    /// pooled              seat0 40 : 18 seat1   (+62 tied)   p = 0.0054
    /// ```
    ///
    /// Seat 0 takes about 69% of decided pairs, for all three fighters. An
    /// unpaired row (the default) carries this seat term undiscounted; paired
    /// rung and fighter arms cancel it.
    ///
    /// The cause is not placement: `ambition_demo_smash::respawn_placement`
    /// places seats 0 and 1 symmetrically at ±32px. Decision order within a
    /// tick is a likely cause and is not measured.
    ///
    /// About a third of the pairs tie exactly. That is expected when only one
    /// term changes, and shows that the swap works.
    #[arg(long)]
    pub rungs: Option<String>,
    /// Run each seed twice with the rungs swapped between seats, and report the
    /// within-seed difference.
    ///
    /// This removes seed-to-seed variance: the same seed plays both role
    /// assignments, so the comparison is a difference within one seed.
    ///
    /// It also cancels the placement confound. The fixtures place self (seat 0,
    /// the higher rung) and most place it badly. With `--paired`, each rung
    /// stands in that spot equally often.
    ///
    /// This doubles the bout count.
    #[arg(long)]
    pub paired: bool,
    /// Match budget in seconds. Absent means the demo's own
    /// `SMASH_TIME_LIMIT_TICKS` — the shipped eight minutes.
    ///
    /// See [`ticks()`] for why this is a parameter: a longer clock separates
    /// "this rung is weaker" from "this rung is patient".
    #[arg(long, value_name = "SECONDS")]
    pub seconds: Option<usize>,
    /// Load an authored difficulty ladder from a `.ron` file and install it, so
    /// the rig measures that ladder instead of the engine floor.
    ///
    /// Use this to measure the shipped fighter.
    ///
    /// Without it, the rig measures the engine floor: the demo app installs no
    /// `AuthoredFighterLadder`, so `profile_for_level` falls back to
    /// `FighterBrainProfile::for_level`. The floor gives every rung the level-9
    /// weights (`UtilityWeights::default()` is `v1()`) and turns rollout on at
    /// level 6; the authored ladder disables rollout on all rows.
    ///
    /// `project_authored_fighter_ladder` rewrites live profiles that differ
    /// from their rung on every tick. So the tuning flags (`--weight`, `--apm`,
    /// `--noise`, `--reaction-ms`) are written into the rows this flag reads,
    /// not into live brains. See `ProfileOverride`.
    ///
    /// Point it at `game/ambition_content/assets/data/fighter_brain_ladder.ron`
    /// to measure what a player fights. Reading a file is a measurement-tool
    /// choice. Whether the demo app composes `ambition_content` is a product
    /// decision (`awaiting-maintainer-decision.md`).
    #[arg(long, value_name = "PATH")]
    pub ladder: Option<String>,
    /// Print one line per bout beneath each row, not just the medians.
    ///
    /// A median cannot tell "both bodies died together" from "neither died
    /// before the match resolved". The per-bout lines can.
    #[arg(long)]
    pub per_bout: bool,
    /// Stage to fight on. The names are the stage button's own labels,
    /// lowercased — `flat` (the demo's default), `platforms`, `narrow` — and an
    /// unknown one is refused with the live list rather than defaulted.
    ///
    /// The stage is a confounder: spacing, recovery, and edgeguard results
    /// depend on the layout.
    ///
    /// The default is empty, not `"flat"`. Empty resolves to
    /// `SmashStageChoice::default()` at the point of use, so the rig follows
    /// the demo if that default changes.
    #[arg(long, default_value = "")]
    pub stage: String,
}

/// Parsed once in `run` and read from anywhere, so deep functions need no
/// extra parameter. A process global is correct here: `run` is the only
/// writer and it writes before anything reads.
static ARGS: std::sync::OnceLock<LadderRigArgs> = std::sync::OnceLock::new();

fn args() -> &'static LadderRigArgs {
    ARGS.get_or_init(LadderRigArgs::default)
}

pub fn run(cli: LadderRigArgs) {
    let _ = ARGS.set(cli);
    let seeds = seed_count();

    // Print the header once, before the mode is chosen, so that every mode
    // (including a new one) reports its ladder, fighters, and clock.
    report_which_ladder_is_in_play();

    if args().sweep_below {
        return run_sweep_below(seeds);
    }
    if args().scenarios {
        return run_scenarios(seeds);
    }
    // Name the weights of this run, so that two runs can be compared.
    match weights_from_args() {
        Some(weights) => println!(
            "[ladder_rig] weights OVERRIDDEN on EVERY fighter: {weights:?} \
             (the authored per-level weights are not in play)"
        ),
        // "Not overridden" is not "the authored rows": the `ladder:` line says
        // which rows each rung got. `--weight-scale` factors also appear on that
        // line, beside the rows they modified.
        None if args().weight_scales.is_empty() => println!(
            "[ladder_rig] weights: not overridden — each rung keeps whatever its \
             profile source gave it (the `ladder:` line ABOVE names the file it \
             read and prints every rung)"
        ),
        None => println!(
            "[ladder_rig] weights: the AUTHORED per-rung weights, SCALED — no `--weight` \
             replaced them, so the ladder's ramp is intact and `--weight-scale` multiplied \
             it. The `ladder:` line ABOVE names the factors and prints every rung after them"
        ),
    }
    // Print the bar last, just before the column header, because the reader
    // needs it while reading the rows.
    report_what_this_run_could_report();
    println!(
        // "Stocks LEFT": `0 : 0` means both fighters were fully eliminated.
        "[ladder_rig] higher vs lower   survived(hi:lo)   stocks LEFT(hi:lo)   dealt%(hi:lo)   peak%(hi:lo)   \
         verdict = who OUTFOUGHT. ⚠ PAIRED rows decide it per SEED (stocks, then \
         damage on a stock tie) and the columns beside it are pooled medians, \
         DESCRIPTIVE ONLY; UNPAIRED rows decide it from those medians   \
         (median of {seeds} seeds, {}s each, {})",
        ticks() / 60,
        // Every table header names its design.
        if args().paired {
            pairing_axis()
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

/// The profile fields this run overrides, as one value applied where the
/// profile is owned.
///
/// `project_authored_fighter_ladder` has no change filter. Every tick it
/// rewrites any `cfg.profile` that differs from its rung. So an override
/// written onto a live brain is reverted within one tick on the ladder road.
///
/// With a ladder installed, the override goes into the rows before the
/// resource is inserted, so the projection carries it. With no ladder, the
/// floor owns the profile and the override goes onto the live brains.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ProfileOverride<'a> {
    weights: Option<ambition_platformer2d::characters::brain::fighter::UtilityWeights>,
    apm_cap: Option<f32>,
    execution_noise: Option<f32>,
    reaction_ms: Option<f32>,
    /// `--no-rollout` zeroes both rollout fields together; they are one knob.
    no_rollout: bool,
    /// `--weight-scale` factors by weight name.
    ///
    /// A borrowed slice, not fields: these multiply the authored row, and
    /// `apply` sees one profile at a time, so the factors travel with the
    /// override.
    scales: &'a [(String, f32)],
}

impl<'a> ProfileOverride<'a> {
    /// An override that changes nothing. `from_args` compares against it, and
    /// tests start from it.
    const NOTHING: Self = Self {
        weights: None,
        apm_cap: None,
        execution_noise: None,
        reaction_ms: None,
        no_rollout: false,
        scales: &[],
    };

    /// What the caller asked for, or `None` when they asked for nothing.
    ///
    /// `None` differs from an all-default `Some`: an override that changes no
    /// field still writes the profile.
    fn from_args() -> Option<Self> {
        // Parsed once per process and kept in a static so the override can
        // borrow it for as long as `args()` lives.
        static SCALES: std::sync::OnceLock<Vec<(String, f32)>> = std::sync::OnceLock::new();
        let scales =
            SCALES.get_or_init(|| named_numbers("--weight-scale", &args().weight_scales));
        let me = Self {
            weights: weights_from_args(),
            apm_cap: flag_value("--apm").and_then(|v| v.parse().ok()),
            execution_noise: flag_value("--noise").and_then(|v| v.parse().ok()),
            reaction_ms: flag_value("--reaction-ms").and_then(|v| v.parse().ok()),
            no_rollout: args().no_rollout,
            scales,
        };
        (me != Self::NOTHING).then_some(me)
    }

    fn apply(
        self,
        profile: &mut ambition_platformer2d::characters::brain::fighter::FighterBrainProfile,
    ) {
        if let Some(weights) = self.weights {
            profile.utility_weights = weights;
        }
        if let Some(apm) = self.apm_cap {
            profile.apm_cap = apm;
        }
        if let Some(noise) = self.execution_noise {
            profile.execution_noise = noise;
        }
        if let Some(ms) = self.reaction_ms {
            profile.reaction_ms = ms;
        }
        if self.no_rollout {
            profile.rollout_depth = 0;
            profile.rollout_k = 0;
        }
        // Apply scales last, so they multiply the row (or the `--weight` value
        // that replaced it).
        for (name, factor) in self.scales {
            if let Some(field) = weight_field_mut(&mut profile.utility_weights, name) {
                *field *= factor;
            }
        }
    }
}

fn authored_ladder(
) -> Option<ambition_platformer2d::characters::brain::fighter::AuthoredFighterLadder> {
    use ambition_platformer2d::characters::brain::fighter::{
        AuthoredFighterLadder, FighterBrainLadder,
    };
    let path = args().ladder.as_deref()?;
    let text = std::fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("[ladder_rig] --ladder {path}: {err}");
        std::process::exit(2);
    });
    let mut ladder = FighterBrainLadder::from_ron(&text).unwrap_or_else(|err| {
        eprintln!("[ladder_rig] --ladder {path} did not parse: {err}");
        std::process::exit(2);
    });
    // Put the override into the rows, not onto live brains: with this resource
    // installed, the projection owns `cfg.profile`. See `ProfileOverride`.
    if let Some(over) = ProfileOverride::from_args() {
        for rung in ladder.rungs_mut() {
            over.apply(rung);
        }
        // An override can flatten the ladder (`--apm 1` gives every rung one cap).
        // Report it, do not refuse it: a controlled arm may do this on purpose.
        let problems = ladder.problems();
        if !problems.is_empty() {
            eprintln!(
                "[ladder_rig] ⚠ the overridden ladder is no longer well-formed \
                 ({} problem(s)) — fine for a controlled arm, not for a \
                 difficulty reading: {}",
                problems.len(),
                problems.join("; ")
            );
        }
    }
    Some(AuthoredFighterLadder(ladder))
}

/// A short, stable digest of the ladder file's bytes, so two runs can be shown
/// to have read the same rows rather than the same path.
///
/// Not a cryptographic hash, and not `Hash` on the parsed rows: parsing
/// drops comments and formatting, and the reader asks about the input file.
fn ladder_digest() -> Option<String> {
    let path = args().ladder.as_deref()?;
    let text = std::fs::read_to_string(path).ok()?;
    // FNV-1a, 64-bit. Small, dependency-free, and enough to separate two
    // hand-edited difficulty tables.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Some(format!("{hash:016x}"))
}

/// One line per rung, carrying every authored field a cell's outcome can depend
/// on — so the arm is readable from its own log without opening the `.ron`.
///
/// Prints all rungs, not only the pair under test, so the log also serves a
/// later arm.
fn ladder_rungs_summary() -> Vec<String> {
    let Some(ladder) = authored_ladder() else {
        return Vec::new();
    };
    ladder
        .0
        .rungs()
        .iter()
        .map(|rung| {
            let w = &rung.utility_weights;
            format!(
                "rung {:>1}  reaction {:>5.0}ms  apm {:>5.0}  noise {:.2}  read {:.2}  \
                 rollout {}/{}  weights reach {:.2} fadv {:.2} kill {:.2} risk {:.2} payoff {:.2} capture {:.2}",
                rung.level,
                rung.reaction_ms,
                rung.apm_cap,
                rung.execution_noise,
                rung.read_weight,
                rung.rollout_depth,
                rung.rollout_k,
                w.reach_fit,
                w.frame_advantage,
                w.kill_potential,
                w.stage_risk,
                w.expected_payoff,
                w.capture_value,
            )
        })
        .collect()
}

/// Two-sided sign test on paired differences: is this split surprising for a
/// fair coin?
///
/// Returns `true` for "within spread" (not significant at p < 0.05).
///
/// The sign test uses only which side dealt more damage, not by how much, so
/// one lopsided bout cannot carry a cell. Bout damage is bounded, skewed, and
/// bimodal, where magnitude-based tests fail. Ties are dropped: they are
/// evidence about neither rung.
///
/// Test-only. Production uses `paired_outcomes` and `paired_verdict`. This
/// helper keeps the tests of the shared core (ties dropped, more agreeing
/// evidence never less significant). The production tie path is covered by
/// `level_pairs_are_dropped_rather_than_counted`.
#[cfg(test)]
fn sign_test_says_within_spread(diffs: &[f32]) -> bool {
    let positives = diffs.iter().filter(|d| **d > 0.0).count();
    let negatives = diffs.iter().filter(|d| **d < 0.0).count();
    // The threshold is inline: a wrapper used only here would be dead code in
    // a non-test build.
    sign_test_p(positives, negatives) >= 0.05
}

/// The smallest majority that clears p < 0.05 at `pairs` usable pairs, or
/// `None` when no split can. The header bar and the tests both use this, so
/// the bar has one source.
fn smallest_clearing_majority(pairs: usize) -> Option<usize> {
    (pairs / 2..=pairs).find(|&k| sign_test_p(k, pairs - k) < 0.05)
}

fn sign_test_p(positives: usize, negatives: usize) -> f64 {
    let n = positives + negatives;
    // No special case for small n: the exact tail covers it. Five unanimous
    // pairs give 2 * 0.5^5 = 0.0625, which is not below 0.05. So fewer than six
    // usable pairs cannot reach significance. Such a cell is `(within spread)`
    // because the run is too short, not because the rungs are alike.
    let k = positives.max(negatives);
    // Two-sided exact binomial tail: 2 * P(X >= k) for X ~ Binomial(n, 0.5).
    // n is small, so the exact sum is cheap and better than a normal
    // approximation. Return the p value, not a bool, so the row can print it:
    // 0.146 ("nearly") and 0.774 ("a coin") both read `(within spread)`.
    let mut tail = 0.0f64;
    let mut term = 0.5f64.powi(n as i32); // C(n,0) * 0.5^n
    for i in 0..=n {
        if i >= k {
            tail += term;
        }
        // C(n, i+1) = C(n, i) * (n - i) / (i + 1)
        if i < n {
            term = term * (n - i) as f64 / (i + 1) as f64;
        }
    }
    (2.0 * tail).min(1.0)
}



/// Name the two fighters of the run, also when they are defaults.
///
/// The defaults `smash_duelist_a` and `smash_duelist_b` are stand-ins that
/// play `smash_duelist_a.ron`'s table, not George's full repertoire. Printing the
/// default lets a reader see that.
fn report_which_fighters_are_in_play() {
    let [higher, lower] = fighters();
    let chosen = flag_value("--character").is_some() || flag_value("--opponent").is_some();
    let george = ambition_demo_smash::SMASH_GEORGE_BOOUL;
    let stand_ins = higher != george && lower != george;
    println!(
        "[ladder_rig] fighters: `{higher}` (higher rung) vs `{lower}` (lower rung){}{}",
        if chosen { "" } else { " — DEFAULTED, nobody passed --character/--opponent" },
        if stand_ins {
            // State what the run is about, not a defect in the fighters.
            format!(
                ". ⛔ Neither is `{george}`, the demo's one fully authored fighter — \
                 these carry `smash_duelist_a.ron`, so this measures the STAND-INS. \
                 Concretely: their unanswered presses are George's plus EIGHT MORE, \
                 every one a `special` (only `special_forward` answers), because \
                 that table is the one contract that does not go through \
                 `SmashRepertoire`. They also bind no `attack_dash`, which is why \
                 they keep tilts George never throws"
            )
        } else {
            String::new()
        }
    );
}

/// The stage this run fights on, resolved once.
///
/// The stage of this run, resolved once. The world and the header both use
/// this, and the header prints `label()`, the same string as the game's
/// stage button. So the header cannot disagree with the run.
fn resolved_stage() -> ambition_demo_smash::SmashStageChoice {
    // Resolve from `SmashStageChoice::ALL`, not from string literals, so a new
    // stage is reachable by its button label with no edit here.
    let asked = args().stage.trim().to_ascii_lowercase();
    if asked.is_empty() {
        // No `--stage`: use the demo's own default, so the rig follows it.
        return ambition_demo_smash::SmashStageChoice::default();
    }
    ambition_demo_smash::SmashStageChoice::ALL
        .into_iter()
        .find(|stage| stage.label().to_ascii_lowercase() == asked)
        .unwrap_or_else(|| {
            let known: Vec<String> = ambition_demo_smash::SmashStageChoice::ALL
                .iter()
                .map(|stage| format!("`{}`", stage.label().to_ascii_lowercase()))
                .collect();
            panic!(
                "unknown --stage {asked:?}; the rig fights on {}. Defaulting \
                 would silently measure a stage nobody asked for, and the stage \
                 is exactly the variable this flag exists to control.",
                known.join(" or ")
            )
        })
}

/// Say how long a bout ran, and say loudly when that is not a real match.
fn report_which_clock_is_in_play() {
    let shipped = ambition_demo_smash::SMASH_TIME_LIMIT_TICKS as usize;
    let used = ticks();
    if used == shipped {
        println!(
            "[ladder_rig] clock: {}s per bout — the SHIPPED match limit \
             (ambition_demo_smash::SMASH_TIME_LIMIT_TICKS)",
            used / 60
        );
    } else {
        println!(
            "[ladder_rig] ⛔ clock: {}s per bout, but the SHIPPED match limit is \
             {}s. This run measures the first {} of a match. A bout that \
             cannot end leaves stocks TIED, and a tied stock count sends every \
             verdict to the damage tiebreak — so read every row below as \
             \"dealt more damage in {}s\", never as \"won\".",
            used / 60,
            shipped / 60,
            // Use more decimals below 1%: `{:.0}%` prints "0%" for a two-second run.
            {
                let share = 100.0 * used as f32 / shipped as f32;
                if share < 1.0 {
                    format!("{share:.2}%")
                } else {
                    format!("{share:.0}%")
                }
            },
            used / 60
        );
    }
}

/// Say what majority this run's seed count could even report, before any row.
///
/// A cell reads `(within spread)` when the rungs are alike or when the run is
/// too short for any split to clear. Below six usable pairs nothing clears.
///
/// The bar falls as seeds rise (83.3% at 12 pairs, 80.0% at 15, 71.4% at 28).
/// So "significant" at two run lengths means two different majorities.
fn report_what_this_run_could_report() {
    // Ties are dropped, so this is the ceiling on usable pairs. A run with
    // ties has a harsher bar than this line states.
    let pairs = seed_count();
    if !args().paired {
        return;
    }
    match smallest_clearing_majority(pairs) {
        Some(k) => println!(
            "[ladder_rig] significance bar: with {pairs} paired seeds a cell needs \
             {k} of {pairs} ({:.0}%) going one way to print without \
             `(within spread)`. ⚠ Ties are dropped, so a row with `+N tied` faces \
             a HARSHER bar than this. A longer run accepts a WEAKER majority — \
             compare two runs by the percentage, never by the p.",
            100.0 * k as f64 / pairs as f64
        ),
        None => println!(
            "[ladder_rig] ⛔ significance bar: {pairs} paired seeds CANNOT reach \
             p < 0.05 at any split — not even a unanimous sweep. Every row below \
             will print `(within spread)` because the RUN IS TOO SHORT, which is \
             a different statement about the fighters than `the rungs are alike`. \
             Six or more seeds is the floor."
        ),
    }
}

fn report_which_ladder_is_in_play() {
    report_which_clock_is_in_play();
    report_which_fighters_are_in_play();
    let mut app = build_demo_app();
    if let Some(ladder) = authored_ladder() {
        app.world_mut().insert_resource(ladder);
    }
    app.update();
    let authored = app
        .world()
        .get_resource::<ambition_platformer2d::characters::brain::fighter::AuthoredFighterLadder>()
        .is_some();
    if authored {
        // Print the path, a digest of the parsed text, and every rung, so two
        // arms that differ only by file have different logs. The digest covers the
        // bytes, because paths get reused and edited in place.
        let rungs = ladder_rungs_summary();
        // Say whether the rows below are still the file's. The override goes
        // into the rows (`ProfileOverride`), so the printed rungs can differ from
        // the bytes the digest covers.
        println!(
            "[ladder_rig] ladder: the rows from `{}` (digest {}) — \
             AuthoredFighterLadder is installed.{}",
            args().ladder.as_deref().unwrap_or("<none>"),
            ladder_digest().unwrap_or_else(|| "n/a".to_string()),
            match ProfileOverride::from_args() {
                None => " Authored, unmodified.".to_string(),
                Some(over) => format!(
                    " ⚠ MODIFIED BY THIS RUN before installing: {over:?}. The digest is the \
                     FILE's and does NOT cover these changes — two runs agreeing on it did \
                     not necessarily measure the same rungs. The rungs below are what ran."
                ),
            },
        );
        for line in rungs {
            println!("[ladder_rig]   {line}");
        }
    } else {
        println!(
            "[ladder_rig] ⛔ ladder: the ENGINE FLOOR — no AuthoredFighterLadder in this app, so \
             every rung carries the floor's `UtilityWeights::default()` (== v1, the level-9 row) \
             and differs in reaction/APM/noise/read. ⛔⛔ AND THE FLOOR SWITCHES THE L3 ROLLOUT ON \
             AT RUNGS 6-9 (`rollout_depth: 12`), WHICH THE SHIPPED LADDER SETS TO 0 ON ALL NINE — \
             so the top rungs here run a search no player ever meets, and `read_weight` and the \
             Dodge/Shield suppression become live with it. This is NOT the ladder the shipped game \
             gives its fighters."
        );
    }
}

/// One weight by name, for the two flags that address weights by name.
///
/// Keep this match in step with `UtilityWeights`: a field missing here
/// cannot be swept by the rig.
fn weight_field_mut<'a>(
    weights: &'a mut ambition_platformer2d::characters::brain::fighter::UtilityWeights,
    name: &str,
) -> Option<&'a mut f32> {
    Some(match name {
        "reach_fit" => &mut weights.reach_fit,
        "frame_advantage" => &mut weights.frame_advantage,
        "kill_potential" => &mut weights.kill_potential,
        "stage_risk" => &mut weights.stage_risk,
        "expected_payoff" => &mut weights.expected_payoff,
        "capture_value" => &mut weights.capture_value,
        "displacement_value" => &mut weights.displacement_value,
        _ => return None,
    })
}

/// Parse a repeated `NAME=VALUE` flag into named numbers, refusing rather than
/// defaulting on anything it does not understand.
fn named_numbers(flag: &str, pairs: &[String]) -> Vec<(String, f32)> {
    pairs
        .iter()
        .map(|pair| {
            let Some((name, value)) = pair.split_once('=') else {
                eprintln!("[ladder_rig] {flag} wants name=value, got '{pair}'");
                std::process::exit(2);
            };
            let Ok(value) = value.parse::<f32>() else {
                eprintln!("[ladder_rig] '{value}' is not a number");
                std::process::exit(2);
            };
            // Refuse unknown names, so a typo does not run the unmodified weights
            // under a header that claims otherwise.
            let mut probe = ambition_platformer2d::characters::brain::fighter::UtilityWeights::v1();
            if weight_field_mut(&mut probe, name).is_none() {
                eprintln!("[ladder_rig] no weight named '{name}'");
                std::process::exit(2);
            }
            (name.to_string(), value)
        })
        .collect()
}

fn weights_from_args(
) -> Option<ambition_platformer2d::characters::brain::fighter::UtilityWeights> {
    if args().weights.is_empty() {
        return None;
    }
    let mut weights = ambition_platformer2d::characters::brain::fighter::UtilityWeights::v1();
    for (name, value) in named_numbers("--weight", &args().weights) {
        *weight_field_mut(&mut weights, &name).expect("named_numbers refused unknown names") =
            value;
    }
    Some(weights)
}

fn force_noise_seed(app: &mut bevy::app::App, seed: u64, swap_streams: bool) -> bool {
    use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
    let world = app.world_mut();
    let mut brains = world.query::<(&MatchSeat, &mut Brain)>();
    let mut applied = false;
    for (seat, mut brain) in brains.iter_mut(world) {
        if let Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) = &mut *brain {
            // The stream is keyed on the seat, so it is a per-seat term.
            // `Mirror::Noise` exchanges the two seats' streams to cancel it.
            state.noise = noise_stream(seed, seat.0, swap_streams);
            applied = true;
        }
    }
    applied
}

/// The SplitMix64 state one seat starts on, for one seed and one pairing.
///
/// `swap_streams` must exchange the two seats' streams, not derive new
/// ones: [`Mirror::Noise`] cancels the stream term by giving each seat the
/// other seat's stream. `^ 1` is an involution on the two seats. `seat + 1`
/// would add two new streams and cancel nothing.
fn noise_stream(seed: u64, seat: usize, swap_streams: bool) -> u64 {
    // A zero stream is a legitimate state but an unhelpful one to start every
    // seat on, hence the `+ 1`.
    let stream_of = seat ^ usize::from(swap_streams);
    seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (stream_of as u64 + 1)
}

/// Apply this run's [`ProfileOverride`] to every live fighter — the floor road.
///
/// One function for every flag, because it is one decision: this run
/// measures a modified profile.
///
/// It writes the live cfg, not the published policy. That is intended for
/// a sweep. It is correct only where the floor owns the profile. With an
/// `AuthoredFighterLadder` installed, the override belongs in the rows (see
/// `ProfileOverride`).
fn force_profile(app: &mut bevy::app::App, over: ProfileOverride) -> bool {
    use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
    let world = app.world_mut();
    let mut q = world.query::<&mut Brain>();
    let mut found = false;
    for mut brain in q.iter_mut(world) {
        if let Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) = &mut *brain {
            over.apply(&mut cfg.profile);
            found = true;
        }
    }
    found
}

/// Run rung pairs through scenarios reproducible by body placement alone.
/// Scenarios requiring velocity, phases, projectiles, or other explicit state are
/// skipped using `Scenario::unreproduced_by_placement`.
fn run_scenarios(seeds: usize) {
    // The ledge-hang fixture is anchored to Flat's platform whatever `--stage`
    // says, so this mode refuses other stages.
    //
    // `place_at` reads `smash_stage().world.blocks[0].aabb` (always Flat,
    // `x = 80..560`) and installs `LedgeGrabState::hanging` on that contact.
    // On Narrow (`x = 160..480`) the fighter would hang in mid-air.
    //
    // Refuse, do not filter: dropping ledge-hang scenarios would change the
    // suite's population without saying so. The repair is to derive the anchor
    // from the session's `RoomGeometry` (as `stage_bounds()` does); it is not
    // done yet.
    let stage = resolved_stage();
    if stage != ambition_demo_smash::SmashStageChoice::Flat {
        panic!(
            "--scenarios cannot run on `{}`: the ledge-hang fixtures anchor to \
             Flat's platform regardless of --stage, so every hang would be \
             staged against the wrong ledge. Run them on `flat`, or repair \
             `place_at` to derive the ledge from the session's RoomGeometry \
             first. ⚠ Distrust any non-Flat scenario numbers recorded before \
             this refusal existed.",
            stage.label(),
        );
    }
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
    // Name the weights: this mode returns before the ladder mode's header.
    match weights_from_args() {
        Some(weights) => println!(
            "[ladder_rig] weights OVERRIDDEN on EVERY fighter: {weights:?} \
             (the authored per-level weights are not in play)"
        ),
        // "Not overridden" is not "the authored rows"; the `ladder:` line says
        // which rows each rung got.
        None => println!(
            "[ladder_rig] weights: not overridden — each rung keeps whatever its \
             profile source gave it (the `ladder:` line ABOVE names the file it \
             read and prints every rung)"
        ),
    }
    println!(
        // Name the stage: every number below depends on it.
        "[ladder_rig] --scenarios: PLACEMENT ONLY — {} of {} fixture(s) are \
         reproduced by placing two bodies (median of {seeds} seeds, {}s each, \
         stage `{}`, {}, rungs {})",
        playable.len(),
        suite.len(),
        ticks() / 60,
        resolved_stage().label(),
        // Name the design: paired and unpaired tables use different controls.
        if args().paired {
            pairing_axis()
        } else {
            "unpaired — seat 0 is always the higher rung"
        },
        // Name the rungs: a null-control run (`6,6`) and a ladder run print the
        // same columns.
        rungs()
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    // Print a column header. "Stocks LEFT" is remaining stocks, not lost.
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
        // `velocity` does not disqualify a fixture: `place_at` sets it through
        // `TransitVelocity::Set`. Other state this rig cannot arrange is a skip,
        // and the message names what is missing.
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

/// `--sweep-below`: vary only the level of the fighter placed below the stage.
///
/// `--scenarios` walks `RUNGS.windows(2)`, which moves both seats at once
/// and gives only four points for `recovery_below`. Four points cannot
/// separate a threshold, a trend, and scatter. This mode moves one
/// variable: the partner is pinned at level 5, so only the profile of the
/// body that must recover changes between rows.
fn run_sweep_below(seeds: usize) {
    // The header is printed by `run` before the mode is chosen.
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
        ticks() / 60
    );
    println!(
        "[ladder_rig] fixture            rungs     survived(hi:lo)                stocks LEFT   dealt%(hi:lo)     peak%(hi:lo)     verdict = who OUTFOUGHT (stocks taken, then damage DEALT)"
    );
    // Published levels only. `smash_roster_at_levels` builds a
    // `duelist_l{level}` policy key, and only 1/3/5/6/9 are published in
    // `SMASH_CATALOG_RON`. Another level refuses the seat, and the `placed`
    // assert then stops the run.
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

/// Refuse a `--character`/`--opponent` this app cannot seat, and name what
/// it can.
///
/// An absent or empty registry is a refusal, not a pass. It means the
/// warm-up updates did not prepare the cast, and skipping the check would
/// accept every id, typos included.
fn assert_seatable(app: &bevy::prelude::App, ids: [String; 2]) {
    let registry = app
        .world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>();
    let Some(registry) = registry else {
        panic!(
            "[ladder_rig] no `PreparedCharacterRegistry` after the warm-up updates, so \
             `{}` and `{}` cannot be checked against anything. The cast this bout is \
             about to seat was never prepared; a run from here would measure whatever \
             the seating fell back to.",
            ids[0], ids[1]
        );
    };
    let mut known: Vec<&str> = registry.ids().collect();
    known.sort_unstable();
    assert!(
        !known.is_empty(),
        "[ladder_rig] the prepared character registry is EMPTY, so every id would be \
         refused and none accepted. That is a broken composition, not a bad flag."
    );
    for id in &ids {
        assert!(
            known.contains(&id.as_str()),
            "[ladder_rig] `{id}` is not a character this app can seat. \
             `ambition_demo_smash_app` composes the demo's own cast and has no \
             `ambition_content` edge, so Ambition's authored fighters are NOT \
             available here — for those, use the app acceptance harness instead. \
             Seatable here: {known:?}"
        );
    }
}

fn fighters_seated(swapped: bool) -> [String; 2] {
    let [a, b] = fighters();
    if swapped {
        [b, a]
    } else {
        [a, b]
    }
}

/// What `--paired` actually swaps for this run, in the words the header prints.
///
/// `bouts_for_seed` has three pairings. This reads the same inputs it
/// branches on, so the header names the term that the rows actually swap.
fn pairing_axis() -> &'static str {
    let rungs = rungs();
    let equal = |p: &[u8]| p[0] == p[1];
    let [a, b] = fighters();
    match (
        rungs.windows(2).all(equal),
        rungs.windows(2).any(equal),
        a == b,
    ) {
        (false, false, _) => "PAIRED — each seed run twice with the RUNGS swapped between seats",
        (true, _, false) => {
            "PAIRED — one rung on both sides, so the variable is the FIGHTER: each seed runs \
             twice with the two ids swapped between seats"
        }
        (true, _, true) => {
            "PAIRED — ⭐ SEAT NULL CONTROL: one rung, one fighter, each seed run twice with the \
             two seats' NOISE STREAMS swapped. `higher`/`lower` below are SEAT 0 and SEAT 1 and \
             nothing else distinguishes them, so any verdict but `even` here is the INSTRUMENT \
             and every ladder number carries it"
        }
        (false, true, _) => {
            "PAIRED — ⚠ MIXED: this rung list contains both equal and unequal pairs, so the rows \
             below do not share a control. Equal-rung rows swap fighters or noise streams; \
             unequal ones swap the rungs. Run them separately to compare"
        }
    }
}

fn fighters() -> [String; 2] {
    [
        flag_value("--character")
            .unwrap_or_else(|| ambition_demo_smash::SMASH_CHARACTER_ID.to_string()),
        flag_value("--opponent")
            .unwrap_or_else(|| ambition_demo_smash::SMASH_OPPONENT_ID.to_string()),
    ]
}

/// The row's word and whether to qualify it. This is the one place a row's
/// meaning is decided, so tests can check the row itself and not only its
/// parts: a test of `paired_verdict` alone cannot see `report_row` bypass it.
fn row_verdict(bouts: &[Bout], properly_paired: bool) -> (&'static str, bool, Option<PairedSplit>) {
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
    // On a paired row these pooled medians are descriptive; the paired
    // outcomes decide. On an unpaired row they are the best answer available.
    let pooled_verdict = if hi_took != lo_took {
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
    if properly_paired {
        // Sign test: count the pairs that favour the higher rung and ask how
        // surprising that split is for a fair coin. It gains power with seeds,
        // ignores outlier magnitude, and assumes no distribution. Bout damage is
        // bounded, skewed, and bimodal. Do not use the range of the differences:
        // it only grows with seeds, so the test would get harder with more data.
        let (word, within, split) = paired_verdict(&paired_outcomes(bouts));
        (word, within, Some(split))
    } else {
        // An unpaired row makes no inference. It does not cancel the seat, and
        // most fixtures place seat 0 offstage, so a significance statement would
        // be about the design, not the rungs. The pooled-median word stays as a
        // description, and the print site names the design instead of a
        // qualifier. No split: an unpaired row has no per-seed pairs.
        (pooled_verdict, false, None)
    }
}

/// Which rung won ONE mirrored seed — the single authority for a paired row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PairedOutcome {
    Higher,
    Even,
    Lower,
}

/// Reduce each mirrored seed to one outcome, scored the way the row is scored:
/// stocks taken first, damage dealt only when the stocks tie.
///
/// The halves are already oriented: `bouts_for_seed` calls `.mirrored()` on
/// the swapped half, so `[0]` is the higher rung in both bouts. Do not swap
/// here; that would undo the mirror (see
/// `mirroring_a_bout_swaps_every_per_seat_reading`).
///
/// Sum across the pair; do not compare bout by bout. The seat term appears
/// once on each side and cancels only in the sum.
fn paired_outcomes(bouts: &[Bout]) -> Vec<PairedOutcome> {
    bouts
        .chunks_exact(2)
        .map(|pair| {
            let took = |seat: usize| -> u32 {
                pair.iter()
                    .map(|b| ambition_demo_smash::STARTING_STOCKS - b.stocks[1 - seat])
                    .sum()
            };
            let dealt = |seat: usize| -> f32 { pair.iter().map(|b| b.damage_taken[1 - seat]).sum() };
            let (hi_took, lo_took) = (took(0), took(1));
            if hi_took != lo_took {
                return if hi_took > lo_took {
                    PairedOutcome::Higher
                } else {
                    PairedOutcome::Lower
                };
            }
            let (hi_dealt, lo_dealt) = (dealt(0), dealt(1));
            if hi_dealt > lo_dealt {
                PairedOutcome::Higher
            } else if lo_dealt > hi_dealt {
                PairedOutcome::Lower
            } else {
                PairedOutcome::Even
            }
        })
        .collect()
}

/// The per-seed split a paired verdict came from. It is returned so the row
/// can print it and a reader can check the sign test by hand.
// No `Eq`: it carries an `f64`. Tests compare the counts and the rendered
// string.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PairedSplit {
    higher: usize,
    lower: usize,
    tied: usize,
    /// The exact two-sided sign-test tail this split produced.
    p: f64,
}

impl PairedSplit {
    /// `10:2`, plus `+1 tied` only when a pair tied, so a dropped pair is
    /// always visible.
    fn describe(self) -> String {
        let pairs = if self.tied == 0 {
            format!("{}:{}", self.higher, self.lower)
        } else {
            format!("{}:{} +{} tied", self.higher, self.lower, self.tied)
        };
        // Print p beside the split: two "significant" results can differ a lot.
        // Use scientific notation below 0.001 so small values stay distinct.
        let p = if self.p < 0.001 {
            format!("{:.1e}", self.p)
        } else {
            format!("{:.3}", self.p)
        };
        // Print the majority as a percentage: a larger n accepts a weaker
        // majority (83.3% at n=12, 71.4% at n=28), so compare runs by the
        // percentage, not by p.
        let usable = self.higher + self.lower;
        let pct = if usable == 0 {
            String::new()
        } else {
            let share = 100.0 * self.higher.max(self.lower) as f64 / usable as f64;
            // Name the denominator when a tie changed it: `0:3 +1 tied` is 100% of
            // three, not of four.
            if self.tied == 0 {
                format!(" = {share:.0}%")
            } else {
                format!(" = {share:.0}% of {usable} usable")
            }
        };
        format!("{pairs}{pct}, p={p}")
    }
}

fn paired_verdict(outcomes: &[PairedOutcome]) -> (&'static str, bool, PairedSplit) {
    let higher = outcomes.iter().filter(|o| **o == PairedOutcome::Higher).count();
    let lower = outcomes.iter().filter(|o| **o == PairedOutcome::Lower).count();
    // Ties are dropped, as in the sign test.
    let tied = outcomes.len() - higher - lower;
    let word = match higher.cmp(&lower) {
        std::cmp::Ordering::Greater => "higher outfights",
        std::cmp::Ordering::Less => "LOWER outfights",
        std::cmp::Ordering::Equal => "even",
    };
    let p = sign_test_p(higher, lower);
    (
        word,
        p >= 0.05,
        PairedSplit { higher, lower, tied, p },
    )
}

/// The median of a sample. For an even count it is the mean of the two
/// middle values. Paired runs always have an even count.
fn median(mut values: Vec<f32>) -> f32 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if values.is_empty() {
        return 0.0;
    }
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) / 2.0
    }
}

fn secs(elapsed: f32) -> String {
    // Named `elapsed`, not `ticks`, so it does not shadow the `ticks()` clock.
    if elapsed >= ticks() as f32 {
        format!(">{}s", ticks() / 60)
    } else {
        format!("{:.1}s", elapsed / 60.0)
    }
}

/// `median [min-max]`, or just the median when every seed agreed.
///
/// The spread shows whether a difference is real: the top rungs can differ
/// by a few seconds on medians whose seeds range over tens.
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
/// "Neither landed a hit" has two causes. A median closest approach near a
/// body width means they met and did not commit (scoring). Hundreds of
/// pixels means they never met (navigation).
fn approach_of_the_unfought(bouts: &[Bout]) -> String {
    const FOUGHT_AT_ALL: f32 = 0.01;
    let mut d: Vec<f32> = bouts
        .iter()
        .filter(|b| b.peak_percent[0] < FOUGHT_AT_ALL && b.peak_percent[1] < FOUGHT_AT_ALL)
        .map(|b| b.closest_approach)
        .filter(|d| d.is_finite())
        .collect();
    if d.is_empty() {
        // No unfought bout, or none where both bodies existed: report no
        // distance, not zero.
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
    // `span` computes the survival medians for the column it prints.
    let hi_stocks = median(bouts.iter().map(|b| b.stocks[0] as f32).collect());
    let lo_stocks = median(bouts.iter().map(|b| b.stocks[1] as f32).collect());
    // The verdict is what a seat did to the other one: stocks taken first,
    // then damage dealt as the tiebreak. Survival until a cap saturates at
    // both ends and rewards passivity, so it keeps its column but is not the
    // verdict.
    //
    // Damage uses `damage_taken` (summed per-tick rises), not `peak_percent`.
    // Percent resets on death, so peak under-reads the fighter who died more.
    let dealt = |seat: usize| median(bouts.iter().map(|b| b.damage_taken[1 - seat]).collect());
    let (hi_dealt, lo_dealt) = (dealt(0), dealt(1));
    // A verdict inside the seeds' own spread is not a verdict. It is reported
    // with a qualifier, not suppressed.
    //
    // Paired runs are tested on per-seed `PairedOutcome`s (stocks first), not
    // on two pooled medians. `--paired` emits consecutive (straight, mirrored)
    // bouts of one seed, so seed variance cancels within each pair.
    //
    // Check that the data is actually paired. A malformed input must not
    // produce a significance result.
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
    let (verdict, overlaps, split) = row_verdict(bouts, properly_paired);
    // Print the split beside the word. It is the only number on the line that
    // decided anything; the medians are descriptive.
    let seen = split.map(|s| format!(" [{}]", s.describe())).unwrap_or_default();
    let verdict = if overlaps {
        format!("{verdict}{seen} (within spread)")
    } else if properly_paired {
        format!("{verdict}{seen}")
    } else {
        // Not a significance claim: it names the design, so the reader discounts
        // the word for the right reason.
        format!("{verdict} (unpaired — seat not cancelled)")
    };
    if args().per_bout {
        // Raw, in run order. A paired run emits each seed's straight bout and then
        // its mirror, so the pairs are adjacent.
        for (index, b) in bouts.iter().enumerate() {
            println!(
                "[ladder_rig]     bout {index:>3} {label}  eliminated {:>5} : {:<5} \
                 stocks {} : {}  dealt {:>6.1}% : {:<6.1}%  closest {:.0}px",
                b.eliminated[0],
                b.eliminated[1],
                b.stocks[0],
                b.stocks[1],
                b.damage_taken[1] * 100.0,
                b.damage_taken[0] * 100.0,
                if b.closest_approach.is_finite() {
                    b.closest_approach
                } else {
                    -1.0
                }
            );
        }
    }
    let hi_peak = median(bouts.iter().map(|b| b.peak_percent[0]).collect());
    let lo_peak = median(bouts.iter().map(|b| b.peak_percent[1]).collect());
    // Damage percent is represented as a ratio, so 0.01 means one percent.
    // Rows below that threshold for both fighters are reported as unfought.
    const FOUGHT_AT_ALL: f32 = 0.01;
    // The label is computed on medians, and the outcome is bimodal (untouched
    // or a real fight). Print the count beside it, so "every bout unfought"
    // differs from "just over half".
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
        // A normal-looking row can still hide untouched bouts. Report them with
        // the closest approach, which tells the cause.
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
        // ×100 here and nowhere else; the stored value is a ratio.
        // The dealt column is the deciding one. Damage dealt by a seat is what
        // the other seat absorbed, so the indices cross.
        hi_dealt * 100.0,
        lo_dealt * 100.0,
        hi_peak * 100.0,
        lo_peak * 100.0
    );
}

/// What the second bout of a `--paired` seed exchanges between the two seats.
///
/// Each variant controls for exactly one question. The wrong one still
/// gives symmetric-looking output, but answers a different question.
///
/// There is no `Rungs` variant: the rung swap passes the two rungs to
/// `run_bout_at` in the other order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mirror {
    /// The first bout of a pair, and every unpaired bout.
    Straight,
    /// The two fighter ids change seats. The control for *is A stronger than B*.
    Fighters,
    /// The two seats' noise streams change places, and nothing else does.
    ///
    /// This makes the null control runnable. With one rung and one fighter,
    /// only the seat differs: placement, and the stream `force_noise_seed`
    /// derives from the seat index. Swapping the streams cancels the stream,
    /// so the pair measures the seat term alone.
    Noise,
}

/// Every bout one seed contributes, honouring `--paired`.
///
/// One function owns "a seed becomes these bouts", so every mode honours
/// `--paired`.
fn bouts_for_seed(
    higher: u8,
    lower: u8,
    seed: u64,
    start: Option<&ambition_platformer2d::combat::brain::fighter::scenarios::Scenario>,
) -> Vec<Bout> {
    let straight = run_bout_at(higher, lower, seed, start.cloned(), Mirror::Straight);
    if !args().paired {
        return vec![straight];
    }

    // Same seed, roles swapped. `run_bout_at(lower, higher, ..)` seats the
    // lower rung where the fixture puts self, so `mirrored` swaps the result
    // back and `[0]` stays the higher rung. The seat null is the exception;
    // `Pairing::reorient` decides that.
    let [a, b] = fighters();
    let pairing = Pairing::of(higher, lower, a == b);
    let swapped = if pairing.swap_rungs {
        run_bout_at(lower, higher, seed, start.cloned(), pairing.mirror)
    } else {
        run_bout_at(higher, lower, seed, start.cloned(), pairing.mirror)
    };
    vec![
        straight,
        if pairing.reorient {
            swapped.mirrored()
        } else {
            swapped
        },
    ]
}

/// How one seed becomes a pair: what the second bout swaps, and whether its
/// columns are turned back round before they are reported.
///
/// A table, so a test can read the choice. When each arm built its own
/// pair at the call site, adding `.mirrored()` to the noise arm passed every
/// test. `the_seat_null_is_the_one_pairing_that_must_not_reorient` reads
/// this table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Pairing {
    /// What the second bout exchanges between the seats.
    mirror: Mirror,
    /// Whether the second bout is run with its two rungs the other way round.
    swap_rungs: bool,
    /// Whether the second bout's per-seat columns are swapped back, so that
    /// index 0 keeps meaning what the straight bout's index 0 meant.
    reorient: bool,
}

impl Pairing {
    /// `same_fighter` means the two ids resolve to one fighter. Different ids
    /// can still wear different bodies; see the `Mirror::Fighters` arm.
    fn of(higher: u8, lower: u8, same_fighter: bool) -> Self {
        match (higher == lower, same_fighter) {
            // Unequal rungs: the ladder's own question. The rung is the variable.
            (false, _) => Self {
                mirror: Mirror::Straight,
                swap_rungs: true,
                reorient: true,
            },
            // Equal rungs, different fighters: pair on the fighter. The rung swap
            // would cancel a term this row does not contain, so swap the seats the
            // two fighters occupy.
            //
            // This is a fighter comparison, not a seat null control. The demo's
            // default ids (`smash_duelist_a`, `smash_duelist_b`) share
            // `smash_duelist_a.ron`'s table and the same hitboxes, but wear different sheets,
            // so their hurtboxes and animation sets differ.
            (true, false) => Self {
                mirror: Mirror::Fighters,
                swap_rungs: false,
                reorient: true,
            },
            // One rung, one fighter: the seat null control. The seats still differ,
            // because `noise_stream` derives each stream from the seat index. Swap
            // the streams, and the pair measures the seat term alone.
            //
            // Do not reorient. Here the subject is the seat, so index 0 must mean
            // seat 0 in both halves. Mirroring would average the two seats.
            (true, true) => Self {
                mirror: Mirror::Noise,
                swap_rungs: false,
                reorient: false,
            },
        }
    }
}

impl Bout {
    /// The same bout read from the other seat's side.
    ///
    /// `--paired` runs the second bout of a seed with the lower rung in seat 0.
    /// This swaps every per-seat array so index 0 means the higher rung, and
    /// the same reporter serves paired and unpaired vectors.
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
/// The running stage's extent. Fixture geometry is mapped onto it.
fn stage_bounds(app: &mut bevy::app::App) -> Option<ae::Aabb> {
    use ambition_platformer2d::platformer::lifecycle::session_world_component;
    session_world_component::<ae::RoomGeometry>(app.world())
        .map(|geometry| ae::Aabb::new(geometry.0.size * 0.5, geometry.0.size * 0.5))
}

/// Put the two seated bodies where a scenario says they stand.
///
/// Call after seating. A roster cannot say where its fighters stand, so this
/// measurement tool writes into the sim. Do not promote this to a game seam.
///
/// Returns `false` until both seats exist, so the caller keeps trying.
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
        // Use `transit_body`, not `body.pos = ..`. ADR 0024 routes every pose and
        // velocity write through the movement authority
        // (`engine.pose-writes-are-authority-only`). `transit_body` also calls
        // `reconcile_transit`, which resets surface and frame state for the new
        // position.
        //
        // Velocity is `Zero` unless the scenario sets one through
        // `TransitVelocity::Set`.
        let velocity = match velocities {
            Some((me_vel, foe_vel)) => {
                TransitVelocity::Set(if seat.0 == 0 { me_vel } else { foe_vel })
            }
            None => TransitVelocity::Zero,
        };
        transit_body(&mut model, &mut clusters, target, velocity);
    }
    // Fire the volley ability's own authored bolt
    // (`abilities::ranged::volley::authored_bolt`) from the foe toward the
    // subject. Map the fixture's offset as `starting_positions_on` maps its
    // positions. Do not build a `ProjectileSpawn` from the fixture's numbers:
    // they describe the fixture's own 800x600 stage.
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
    // Arrange the hang after `transit_body`, which clears `ledge_grab`
    // (`reconcile_transit`).
    //
    // The anchor is the top corner of `smash_stage().world.blocks[0]` on the
    // side the fixture placed the body. This is Flat's platform; see
    // `run_scenarios`.
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
                // +1 = wall on the player's left. Hanging off the platform's
                // left edge puts the wall on the player's right, hence -1.
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
    // Hitstun is a timer: `body_phase()` derives the phase from
    // `BodyCombat.hitstun_timer`, so write the timer, not a phase.
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

/// One bout, optionally started from a scenario's positions. The 30 warm-up
/// updates let the shell reach its stage before the roster is inserted.
fn run_bout_at(
    higher: u8,
    lower: u8,
    seed: u64,
    start: Option<ambition_platformer2d::combat::brain::fighter::scenarios::Scenario>,
    mirror: Mirror,
) -> Bout {
    let mut app = build_demo_app();
    // Insert before the warm-up updates: `project_authored_fighter_ladder`
    // applies rows to brains with `Added<Brain>`, so a later ladder never
    // reaches them.
    if let Some(ladder) = authored_ladder() {
        app.world_mut().insert_resource(ladder);
    }
    // Insert before the route: the preparation source reads this resource once,
    // when the match is requested.
    app.world_mut()
        .insert_resource(resolved_stage());
    for _ in 0..30 {
        app.update();
    }
    // Check the ids against the app that will seat them, before seating. An
    // unknown id otherwise fails much later in the noise-seed guard.
    // `ambition_demo_smash_app` has no `ambition_content` edge, so Ambition's
    // authored cast is not seatable here. `PreparedCharacterRegistry` is what
    // the composition prepared, so the check cannot drift.
    assert_seatable(&app, fighters_seated(mirror == Mirror::Fighters));
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            fighters_seated(mirror == Mirror::Fighters),
            &[higher, lower],
        ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    // An eliminated seat stops existing, so keep the last value seen.
    let mut stocks = [ambition_demo_smash::STARTING_STOCKS; 2];
    let mut eliminated = [ticks(); 2];
    let mut peak_percent = [0.0f32; 2];
    let mut damage_taken = [0.0f32; 2];
    // Starts at infinity. A bout where the bodies never coexist keeps it, and
    // `report_row` prints `—`.
    let mut closest_approach = f32::INFINITY;
    let mut last_percent = [0.0f32; 2];
    // A seat is not eliminated until seating has completed; bodies may be absent
    // during the seating transaction.
    let mut appeared = [false; 2];
    // Apply the seed to the live `FighterState` after seating, when the brain and
    // its noise stream exist.
    let mut seeded = false;
    let overrides = ProfileOverride::from_args();
    // Who owns `cfg.profile` for this run: the installed ladder, or the floor.
    let ladder_owns_profile = args().ladder.is_some();
    let mut placed = start.is_none();
    for tick in 0..ticks() {
        app.update();
        if !seeded {
            seeded = force_noise_seed(&mut app, seed, mirror == Mirror::Noise);
            if seeded {
                // Floor road only: with a ladder installed, the rows already carry the
                // override (see `ProfileOverride`). Only when the caller asked.
                if let Some(over) = overrides.filter(|_| !ladder_owns_profile) {
                    force_profile(&mut app, over);
                }
            }
        }
        if !placed {
            if let Some(scenario) = start.as_ref() {
                // Map the fixture onto the running stage. Its numbers describe an
                // 800x600 stage of its own.
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
                // Sum only the rises. A death resets the percent, so the step there is
                // negative and adds nothing.
                damage_taken[seat.0] += (now - last_percent[seat.0]).max(0.0);
                last_percent[seat.0] = now;
            }
        }
        // Both seats or nothing: a tick with one body absent has no separation.
        if let (Some(a), Some(b)) = (at[0], at[1]) {
            closest_approach = closest_approach.min(a.distance(b));
        }
        // The disappearance of a seat is the elimination event, so read every
        // tick.
        for slot in 0..2 {
            appeared[slot] |= seen[slot];
            if appeared[slot] && !seen[slot] && eliminated[slot] == ticks() {
                eliminated[slot] = tick;
                stocks[slot] = 0;
            }
        }

        // Stop when both seats are eliminated. This changes no column: each is
        // recorded above before this check. It saves most of the ticks of a bout.
        //
        // Most of the remaining cost is fixed setup per bout (app build, warm-up,
        // route), about 2.5 s against about 0.5 s of simulation for an 85 s bout.
        // Reusing one app across bouts would cut that, but each bout needs a clean
        // world for determinism.
        if appeared == [true, true] && eliminated.iter().all(|&t| t != ticks()) {
            break;
        }
    }
    assert!(
        placed,
        "a scenario bout ran {} ticks and the fighters were never placed, so \
         it measured the stage's default spawn while claiming a scenario",
        ticks()
    );
    assert!(
        seeded,
        "no fighter brain ever took the noise seed, so every run of this bout is \
         identical and the median is one sample reported N times"
    );
    assert!(
        appeared == [true, true],
        "a ladder bout ran {} ticks and seat {:?} never appeared — the \
         match never seated, and every column below would be measuring an empty \
         stage",
        ticks(),
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

    /// More agreeing evidence must not make a result less significant. A range
    /// criterion (`|median| < 0.5 * (max - min)`) fails this, because a range
    /// only grows with n. This pins the direction, not a single verdict.
    #[test]
    fn adding_agreeing_evidence_never_makes_a_result_less_significant() {
        // Unanimous pairs with one huge outlier, which would drag a
        // magnitude-sensitive test.
        let mut diffs = vec![1.0f32, 2.0, 1.5, 0.5, 3.0, 1.0, 900.0];
        assert!(
            !sign_test_says_within_spread(&diffs),
            "seven unanimous pairs should be significant (p = 2 * 0.5^7 = 0.016)"
        );
        // Every further pair agrees. Significance must not evaporate.
        for extra in [1.0f32, 2.0, 0.25, 5.0, 0.75, 1200.0, 0.1] {
            diffs.push(extra);
            assert!(
                !sign_test_says_within_spread(&diffs),
                "adding an AGREEING pair made the result stop being significant \
                 at n = {} — the test is running backwards, which is exactly the \
                 defect the range criterion had",
                diffs.len()
            );
        }
    }

    /// The test must still withhold significance when it should. The third case
    /// is the one a magnitude test fails: one huge difference against a majority
    /// of small opposing ones is not evidence.
    #[test]
    fn the_sign_test_still_withholds_significance_where_it_must() {
        // A near-even split, plenty of pairs.
        let even: Vec<f32> = (0..20)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        assert!(
            sign_test_says_within_spread(&even),
            "ten against ten is a fair coin and must carry the qualifier"
        );

        // Underpowered: five unanimous pairs give p = 2 * 0.5^5 = 0.0625.
        assert!(
            sign_test_says_within_spread(&[1.0, 1.0, 1.0, 1.0, 1.0]),
            "five pairs cannot be significant at any effect size, so a five-pair \
             run must report within spread — underpowered, not null"
        );

        // The magnitude trap: one pair favours the higher rung by 5000, eight
        // favour the lower by a little. The sign test sees 1 against 8.
        // 8 of 9 is p = 2 * (9 + 1) / 512 = 0.039 (significant); 7 of 8 is
        // p = 2 * (8 + 1) / 256 = 0.070 (not significant).
        let outlier = vec![5000.0f32, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0];
        assert!(
            !sign_test_says_within_spread(&outlier),
            "eight of nine pairs agreeing IS significant (p = 0.039), and it must \
             point at the EIGHT rather than at the one big number"
        );
        assert_eq!(
            outlier.iter().filter(|d| **d < 0.0).count(),
            8,
            "the fixture above must actually be 8-against-1 for that to mean \
             what it says"
        );
        // One pair fewer is not significant, so the assertion above tests the
        // threshold.
        assert!(
            sign_test_says_within_spread(&outlier[..8]),
            "7 of 8 is p = 0.070 and must carry the qualifier — if this passes \
             without the qualifier the threshold has drifted"
        );

        // Ties are dropped, so twenty ties must not change the answer in either
        // direction.
        for real in [
            vec![1.0f32; 6],                  // significant on its own
            vec![1.0f32; 5],                  // underpowered on its own
            vec![1.0f32, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0], // a fair coin
        ] {
            let mut padded = real.clone();
            padded.extend(std::iter::repeat(0.0).take(20));
            assert_eq!(
                sign_test_says_within_spread(&padded),
                sign_test_says_within_spread(&real),
                "twenty ties changed the verdict for {real:?} — ties are \
                 evidence about neither rung and must be discarded, not counted"
            );
        }
        assert!(
            !sign_test_says_within_spread(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
            "six unanimous pairs are p = 0.031 and must be significant — without \
             this line the invariance check above would pass on a test that \
             always says the same thing"
        );
    }

    /// The mirror must swap every per-seat array. Otherwise each pair averages
    /// each rung with the other, and the table fills with balanced rows.
    #[test]
    fn mirroring_a_bout_swaps_every_per_seat_reading() {
        let m = bout().mirrored();
        assert_eq!(m.eliminated, [200, 100]);
        assert_eq!(m.stocks, [2, 1]);
        assert_eq!(m.peak_percent, [1.5, 0.5]);
        assert_eq!(m.damage_taken, [30.0, 10.0]);
        // Not swapped: the separation between two bodies is symmetric.
        assert_eq!(m.closest_approach, 48.0);
    }

    /// Mirroring twice is the identity, so the swap is a permutation.
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

    /// A bout where the higher rung dealt `hi`, the lower dealt `lo`, and each
    /// seat ended with the given stocks. `damage_taken[0]` is what the higher
    /// seat absorbed, i.e. what the lower rung dealt.
    fn scored(hi: f32, lo: f32, hi_stocks: u32, lo_stocks: u32) -> Bout {
        Bout {
            eliminated: [100, 100],
            stocks: [hi_stocks, lo_stocks],
            peak_percent: [1.0, 1.0],
            damage_taken: [lo, hi],
            closest_approach: 48.0,
        }
    }

    /// A row may not be significant in a direction it does not report.
    ///
    /// 16 pairs where the higher rung deals `[1000, 0]` against `[400, 400]`,
    /// and 4 pairs where it deals `[0, 0]` against `[1000, 1000]`. Stocks are
    /// level, so damage decides every pair.
    ///
    /// The first assertion checks that the fixture is still adversarial: pooled
    /// medians say `LOWER`. If they agreed with the pairs, the test would test
    /// nothing.
    #[test]
    fn a_row_cannot_be_significant_in_the_direction_it_does_not_report() {
        let mut bouts = Vec::new();
        for _ in 0..16 {
            bouts.push(scored(1000.0, 400.0, 1, 1));
            bouts.push(scored(0.0, 400.0, 1, 1));
        }
        for _ in 0..4 {
            bouts.push(scored(0.0, 1000.0, 1, 1));
            bouts.push(scored(0.0, 1000.0, 1, 1));
        }

        let hi_pooled = median(bouts.iter().map(|b| b.damage_taken[1]).collect());
        let lo_pooled = median(bouts.iter().map(|b| b.damage_taken[0]).collect());
        assert!(
            lo_pooled > hi_pooled,
            "the fixture is supposed to be one where POOLED medians favour the \
             lower rung ({lo_pooled} vs {hi_pooled}); without that it cannot \
             witness the contradiction it exists for"
        );

        let outcomes = paired_outcomes(&bouts);
        let higher = outcomes.iter().filter(|o| **o == PairedOutcome::Higher).count();
        let lower = outcomes.iter().filter(|o| **o == PairedOutcome::Lower).count();
        assert_eq!((higher, lower), (16, 4), "the pairs split 16-4 for the higher rung");

        let (word, overlaps, _split) = paired_verdict(&outcomes);
        assert!(
            !overlaps,
            "16-4 is p = 0.0118, which is significant; the qualifier must be absent"
        );
        assert_eq!(
            word, "higher outfights",
            "the row is significant 16-4 FOR THE HIGHER RUNG, so it may not print \
             LOWER — that pairing of word and qualifier is the defect this exists for"
        );
    }

    /// `(within spread)` alone cannot tell a coin (5:7, p = 0.774) from a near
    /// miss (p = 0.146). Above the threshold, 11:1 and 2:10 are both
    /// significant, and only the first survives a pair flipping. The printed p
    /// separates them.
    #[test]
    fn the_printed_p_separates_a_coin_from_a_near_miss_and_a_sweep_from_a_squeak() {
        let p = |a: usize, b: usize| (sign_test_p(a, b) * 1000.0).round() / 1000.0;

        // Both (within spread), and not remotely the same claim.
        let coin = p(5, 7);
        let near_miss = p(3, 9);
        assert!(coin >= 0.05 && near_miss >= 0.05, "both are within spread");
        assert!(
            coin > near_miss * 4.0,
            "a coin ({coin}) and a near miss ({near_miss}) must be far apart, or \
             printing the tail buys nothing"
        );

        // Both significant, and only one survives losing a pair.
        let sweep = p(11, 1);
        let squeak = p(2, 10);
        assert!(sweep < 0.05 && squeak < 0.05, "both clear the threshold");
        assert!(
            p(3, 9) >= 0.05,
            "one pair off the squeak must fall outside — that is what makes it a \
             squeak rather than a result"
        );
        assert!(
            p(10, 2) >= 0.05 || sweep < squeak,
            "the sweep must read as stronger than the squeak"
        );

        // A larger n accepts a weaker majority, so the row prints the percentage
        // too. Use the same function as the header, so the test cannot drift from
        // it.
        let smallest_clearing = |n: usize| {
            smallest_clearing_majority(n)
                .map(|k| 100.0 * k as f64 / n as f64)
                .expect("some majority clears at these n")
        };
        let at_12 = smallest_clearing(12);
        let at_28 = smallest_clearing(28);
        assert!(
            at_28 < at_12 - 5.0,
            "28 seeds should accept a materially weaker majority than 12 \
             ({at_28:.1}% vs {at_12:.1}%) — if that stops being true this test's \
             premise is gone and the percentage on the row buys nothing"
        );

        // Check the floor that `report_what_this_run_could_report` states: nothing
        // clears below six pairs, and six unanimous pairs clear.
        for n in 1..=5 {
            assert!(
                (n / 2..=n).all(|k| sign_test_p(k, n - k) >= 0.05),
                "{n} pairs can somehow reach significance — the header tells a \
                 run that short that nothing can, and would be lying"
            );
        }
        assert!(
            sign_test_p(6, 0) < 0.05,
            "six unanimous pairs must clear, or `six or more is the floor` names \
             the wrong number"
        );
    }

    /// The row takes its word from the pairs. Tests of `paired_verdict` alone
    /// cannot see `report_row` bypass it, so this asks `row_verdict`. It uses
    /// the adversarial fixture, where pooled medians and pairs disagree.
    #[test]
    fn a_paired_row_takes_its_word_from_the_pairs_not_the_pool() {
        let mut bouts = Vec::new();
        for _ in 0..16 {
            bouts.push(scored(1000.0, 400.0, 1, 1));
            bouts.push(scored(0.0, 400.0, 1, 1));
        }
        for _ in 0..4 {
            bouts.push(scored(0.0, 1000.0, 1, 1));
            bouts.push(scored(0.0, 1000.0, 1, 1));
        }
        let (word, overlaps, split) = row_verdict(&bouts, true);
        assert_eq!(
            (word, overlaps),
            ("higher outfights", false),
            "a properly paired row must read the paired outcomes; the pooled \
             medians on this fixture say LOWER, which is the answer the defect gave"
        );
        // The row carries the split that produced the word, so the printed split
        // cannot drift from the fixture.
        let split = split.expect("a paired row must report its split");
        assert_eq!(
            (split.higher, split.lower, split.tied),
            (16, 4, 0),
            "the reported split does not match the fixture that produced it"
        );
        assert_eq!(
            split.describe(),
            "16:4 = 80%, p=0.012",
            "the split must print its pairs, the MAJORITY they make, and the \
             tail they produced — the percentage is the only one of the three \
             that is comparable between runs of different length"
        );

        // A tie changes the denominator, and the row must name it.
        let tied = PairedSplit { higher: 0, lower: 3, tied: 1, p: sign_test_p(0, 3) };
        assert_eq!(
            tied.describe(),
            "0:3 +1 tied = 100% of 3 usable, p=0.250",
            "a split with ties must name the denominator its percentage is over"
        );
        // An unpaired row keeps the pooled verdict: there are no pairs.
        assert_eq!(
            row_verdict(&bouts, false).0,
            "LOWER outfights",
            "an unpaired row keeps the pooled verdict — the repair narrows what \
             the pooled columns may decide, it does not delete them"
        );
    }

    /// When stocks decide, the inference follows stocks. Every pair is won on
    /// stocks by the higher rung and lost on damage by a wide margin.
    #[test]
    fn the_paired_inference_follows_stocks_when_stocks_decide() {
        let mut bouts = Vec::new();
        for _ in 0..8 {
            // The higher rung takes both of the lower's stocks in each half and
            // is out-damaged ten to one while doing it.
            bouts.push(scored(10.0, 100.0, 2, 0));
            bouts.push(scored(10.0, 100.0, 2, 0));
        }
        let outcomes = paired_outcomes(&bouts);
        assert!(
            outcomes.iter().all(|o| *o == PairedOutcome::Higher),
            "stocks are the primary outcome, so a pair won on stocks is won: {outcomes:?}"
        );
        let (word, overlaps, _split) = paired_verdict(&outcomes);
        assert_eq!(word, "higher outfights");
        assert!(!overlaps, "8-0 is p = 0.0078 and is significant");
    }

    /// A level pair is evidence about neither rung. Five decisive pairs give
    /// p = 0.0625; ten level pairs beside them must not change that.
    #[test]
    fn level_pairs_are_dropped_rather_than_counted() {
        let mut bouts = Vec::new();
        for _ in 0..5 {
            bouts.push(scored(100.0, 10.0, 1, 1));
            bouts.push(scored(100.0, 10.0, 1, 1));
        }
        for _ in 0..10 {
            bouts.push(scored(50.0, 50.0, 1, 1));
            bouts.push(scored(50.0, 50.0, 1, 1));
        }
        let outcomes = paired_outcomes(&bouts);
        assert_eq!(outcomes.iter().filter(|o| **o == PairedOutcome::Even).count(), 10);
        let (word, overlaps, _split) = paired_verdict(&outcomes);
        assert_eq!(word, "higher outfights", "the direction is still the five decisive pairs");
        assert!(
            overlaps,
            "five unanimous pairs are p = 0.0625; the ten level pairs are not evidence \
             and must not push the row over the line"
        );
    }

    /// The paired reading is blind to the seat. A bout decided only by seat,
    /// paired with its own mirror, reduces to `Even`. If `paired_outcomes`
    /// reoriented the mirrored half, the seat term would return.
    #[test]
    fn a_pair_decided_only_by_the_seat_reduces_to_even() {
        let pair = vec![bout(), bout().mirrored()];
        assert_eq!(
            paired_outcomes(&pair),
            vec![PairedOutcome::Even],
            "the mirror cancels the seat, so neither rung won this pair"
        );
    }

    /// `median` is the midpoint for even samples too, and every `--paired` run
    /// is even.
    #[test]
    fn the_median_of_an_even_sample_is_the_midpoint_not_the_upper_middle() {
        assert_eq!(median(vec![0.0, 0.0, 1.0, 1.0]), 0.5);
        assert_eq!(median(vec![1.0, 2.0, 3.0]), 2.0);
        assert_eq!(median(vec![4.0, 1.0]), 2.5, "and it sorts first");
    }

    /// An unpaired row makes no significance claim. The higher rung takes two
    /// stocks in every bout, while damage runs `[0, 100, 0, 100]` against
    /// `[45, 55, 45, 55]`. Stocks decide the word; the damage variance must not
    /// add a qualifier. An unpaired run does not cancel the seat, so no
    /// inference is made.
    #[test]
    fn an_unpaired_rows_qualifier_is_never_authored_by_the_losing_quantity() {
        let mut bouts = Vec::new();
        for damage in [0.0_f32, 100.0, 0.0, 100.0].into_iter().zip([45.0_f32, 55.0, 45.0, 55.0]) {
            let (hi, lo) = damage;
            // Two stocks taken off the lower rung every bout; none taken back.
            bouts.push(scored(hi, lo, 2, 0));
        }

        // The fixture must really disagree, or it witnesses nothing.
        let hi_dealt = median(bouts.iter().map(|b| b.damage_taken[1]).collect());
        let lo_dealt = median(bouts.iter().map(|b| b.damage_taken[0]).collect());
        assert_eq!(
            (hi_dealt, lo_dealt),
            (50.0, 50.0),
            "damage must be a TIE with a wide higher range, so only stocks can \
             decide the word and only damage could have produced the old qualifier"
        );

        let (word, overlaps, _split) = row_verdict(&bouts, false);
        assert_eq!(
            word, "higher outfights",
            "a rung that takes every stock in every bout wins the row"
        );
        assert!(
            !overlaps,
            "the row printed `(within spread)` over a unanimous stock sweep, on \
             the strength of variance in damage — a quantity that did not author \
             the direction. An unpaired row makes NO significance claim."
        );
    }

    /// A pair of mirrored bouts carries no seat advantage: a straight bout and
    /// its mirror give a pure seat effect to each rung once, so the pair's mean
    /// is free of it.
    #[test]
    fn a_mirrored_pair_cancels_a_pure_seat_effect() {
        // Decided only by seat: seat 0 always deals 10, seat 1 always deals 30.
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

    /// Every override moves its own field and nothing else.
    ///
    /// Where it is applied matters more: with an `AuthoredFighterLadder`
    /// installed, the value goes into the rows, because
    /// `project_authored_fighter_ladder` rewrites live profiles every tick
    /// (pinned by `a_profile_written_from_outside_is_reverted_on_the_very_next_tick`).
    /// No unit test here covers that road choice; only a bout shows it, for
    /// example `--ladder <shipped> --rungs 6,6 --seconds 15 --apm 1` against
    /// `--apm 600`.
    #[test]
    fn each_override_moves_its_own_field_and_leaves_the_rest_authored() {
        use ambition_platformer2d::characters::brain::fighter::{
            FighterBrainProfile, UtilityWeights,
        };
        let authored = FighterBrainProfile::for_level(6);
        let authored_kill = authored.utility_weights.kill_potential;
        // Non-vacuity: doubling zero is zero.
        assert_ne!(
            authored_kill, 0.0,
            "the fixture rung authors no kill weight, so scaling it proves nothing"
        );

        // An empty override changes nothing, so each case below is attributable.
        let mut untouched = authored;
        ProfileOverride::NOTHING.apply(&mut untouched);
        assert_eq!(untouched, authored, "an empty override moved a field");

        let mut weights = UtilityWeights::v1();
        weights.reach_fit = 0.0;
        let kill_doubled = [("kill_potential".to_string(), 2.0_f32)];
        // 0.5 set, then doubled, is 1.0: neither flag gives that alone.
        let set_kill_to_half = UtilityWeights {
            kill_potential: 0.5,
            ..UtilityWeights::v1()
        };
        let cases: [(ProfileOverride, &dyn Fn(&FighterBrainProfile) -> bool); 7] = [
            (
                ProfileOverride {
                    weights: Some(weights),
                    ..ProfileOverride::NOTHING
                },
                &|p: &FighterBrainProfile| p.utility_weights.reach_fit == 0.0,
            ),
            (
                ProfileOverride {
                    apm_cap: Some(1.0),
                    ..ProfileOverride::NOTHING
                },
                &|p: &FighterBrainProfile| p.apm_cap == 1.0,
            ),
            (
                ProfileOverride {
                    execution_noise: Some(0.0),
                    ..ProfileOverride::NOTHING
                },
                &|p: &FighterBrainProfile| p.execution_noise == 0.0,
            ),
            (
                ProfileOverride {
                    reaction_ms: Some(2000.0),
                    ..ProfileOverride::NOTHING
                },
                &|p: &FighterBrainProfile| p.reaction_ms == 2000.0,
            ),
            (
                ProfileOverride {
                    no_rollout: true,
                    ..ProfileOverride::NOTHING
                },
                &|p: &FighterBrainProfile| p.rollout_depth == 0 && p.rollout_k == 0,
            ),
            // A scale multiplies the row: rung 6 authors `kill_potential: 0.90`, so
            // x2 is 1.80.
            (
                ProfileOverride {
                    scales: &kill_doubled,
                    ..ProfileOverride::NOTHING
                },
                &|p: &FighterBrainProfile| p.utility_weights.kill_potential == authored_kill * 2.0,
            ),
            // Order is part of the contract: `--weight` replaces, then
            // `--weight-scale` multiplies, so passing both scales the value you set.
            (
                ProfileOverride {
                    weights: Some(set_kill_to_half),
                    scales: &kill_doubled,
                    ..ProfileOverride::NOTHING
                },
                &|p: &FighterBrainProfile| p.utility_weights.kill_potential == 1.0,
            ),
        ];
        for (over, moved) in cases {
            let mut profile = authored;
            over.apply(&mut profile);
            assert!(moved(&profile), "{over:?} did not move the field it names");
            // And nothing else moved. The projection keys on `level`, so a changed
            // level would send the fighter to a different rung.
            assert_eq!(
                profile.level, authored.level,
                "{over:?} moved the level, which is how a rung finds its row"
            );
        }
    }

    /// [`Mirror::Noise`] cancels the stream only if the swap exchanges the two
    /// seats' streams. `seat + 1` would change both streams, add new ones, and
    /// cancel nothing, with no visible change in the columns.
    #[test]
    fn swapping_the_noise_streams_exchanges_them_rather_than_making_new_ones() {
        for seed in [0u64, 1, 7, 12_345, u64::MAX] {
            let (seat0, seat1) = (
                noise_stream(seed, 0, false),
                noise_stream(seed, 1, false),
            );
            // Premise: the seats have different streams.
            assert_ne!(
                seat0, seat1,
                "seed {seed} gave both seats the same stream, so there is \
                 nothing for the null control to cancel"
            );
            assert_eq!(
                noise_stream(seed, 0, true),
                seat1,
                "seed {seed}: seat 0 must receive SEAT 1's stream, not a third one"
            );
            assert_eq!(
                noise_stream(seed, 1, true),
                seat0,
                "seed {seed}: seat 1 must receive SEAT 0's stream, not a fourth one"
            );
        }
    }

    /// Each pairing arm cancels exactly one term. The seat null is the one arm
    /// that must not reorient: its subject is the seat. The poison to check
    /// this against is `reorient: true` on the `(true, true)` arm.
    #[test]
    fn the_seat_null_is_the_one_pairing_that_must_not_reorient() {
        let ladder = Pairing::of(9, 6, false);
        assert_eq!(
            ladder,
            Pairing {
                mirror: Mirror::Straight,
                swap_rungs: true,
                reorient: true
            },
            "unequal rungs: the rung is the variable, so the rungs swap and the \
             columns come back round"
        );
        // Same rungs, two fighters. `same_fighter` is about what the ids resolve
        // to, so this arm is not the null control.
        assert_eq!(
            Pairing::of(6, 6, false),
            Pairing {
                mirror: Mirror::Fighters,
                swap_rungs: false,
                reorient: true
            },
            "one rung, two fighters: swapping the RUNGS there is a tautology"
        );
        let null = Pairing::of(6, 6, true);
        assert_eq!(
            null,
            Pairing {
                mirror: Mirror::Noise,
                swap_rungs: false,
                reorient: false
            },
            "one rung and one fighter is the SEAT null: exchange the noise \
             streams, and do NOT re-orient — mirroring averages each seat with \
             the other and returns `even` for any pair whatsoever"
        );
        // State the property, so it holds for a fourth arm too.
        assert!(
            !null.reorient && ladder.reorient,
            "exactly the seat-null arm reports its seats where it measured them"
        );
    }

    /// The noise arm must not mirror. On the other arms, [`Bout::mirrored`]
    /// restores `[0]` as the higher rung or the `--character` fighter. On the
    /// seat null, index 0 must stay seat 0. Mirroring there averages the seats
    /// and returns `Even` for any pair.
    #[test]
    fn the_seat_null_reports_a_seat_that_wins_both_halves_and_a_mirror_hides_it() {
        // Seat 0 deals more in both halves: `damage_taken[1]` is what seat 1
        // absorbed, which is what seat 0 dealt.
        let half = |taken: [f32; 2]| Bout {
            damage_taken: taken,
            ..bout()
        };
        // Equal stocks, so the verdict falls through to damage.
        let level = |b: Bout| Bout { stocks: [0, 0], ..b };
        // Seat 0 out-deals seat 1 by the same 20 in each half: a seat term.
        let pair = [level(half([10.0, 30.0])), level(half([26.0, 46.0]))];
        assert_eq!(
            paired_outcomes(&pair),
            vec![PairedOutcome::Higher],
            "seat 0 dealt more in both halves, so the raw pair must say so"
        );

        // The same pair with the second half mirrored, as the other arms do.
        let mirrored = [pair[0], pair[1].mirrored()];
        assert_eq!(
            paired_outcomes(&mirrored),
            vec![PairedOutcome::Even],
            "mirroring the seat-null half averaged the two seats and erased the \
             very term the arm exists to measure"
        );
    }
}
