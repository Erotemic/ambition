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
//! - `--paired` — run each seed TWICE with ONE TERM swapped between the seats
//!   and test the WITHIN-SEED difference. Every cell of the 15-seed matrix came
//!   back `(within spread)` because seed variance exceeded the effect; pairing
//!   removes that variance rather than out-sampling it, and cancels the
//!   seat/placement confound (7 of the 9 fixtures put SELF, always the higher
//!   rung, offstage). ⇒ It changed 14 of 36 verdicts: a 24:12 skew toward the
//!   lower rung became 16:19. The unpaired reading was measuring the seat.
//!
//!   ⭐⭐ **WHICH TERM DEPENDS ON THE ROW, and the header used to name only one
//!   of the three.** `Pairing::of` is the table: unequal rungs swap the RUNGS;
//!   one rung with two fighters swaps the FIGHTERS; one rung with one fighter
//!   swaps the two seats' NOISE STREAMS, which is the seat null control and the
//!   only row that reports its seats where it measured them. A control that
//!   cancels the wrong term is worse than no control, because the output still
//!   looks symmetric.
//! - `--stage <name>` — which layout to fight on, named as the select screen's
//!   own stage button spells it (today: `flat`, `platforms`, `narrow`; the flag
//!   resolves through `SmashStageChoice::ALL`, so this list cannot go stale).
//!   Every number recorded
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

/// ⛔⛔ **THIS WAS 3_600 — SIXTY SECONDS — AND THE SHIPPED MATCH IS EIGHT
/// MINUTES.** The rig measured the first **12.5%** of a match and called the
/// result a ladder.
///
/// ⇒ The consequence was not subtle and it was invisible: the verdict is *stocks
/// taken, then damage dealt*, and on a clock that short **no bout ever reached a
/// conclusion** — every cell came back with stocks tied, so every verdict in
/// every ladder table ever produced fell through to the damage tiebreak. "Rung 5
/// is weaker than rung 3" silently meant "deals less damage in the first eighth
/// of a match".
///
/// ⚠ Measured, not assumed: at 180 seconds the same `5 vs 3` cell resolves — both
/// fighters eliminated at a median of ~98s. So a match takes about 98 seconds to
/// finish and the instrument was stopping it at 60.
///
/// ⭐ It now reads the demo's own constant rather than choosing a number, which is
/// the whole lesson of the day: a measurement is not of the shipped system until
/// it takes the shipped system's own values. `--seconds` still shortens it for
/// quick iteration, and the header says which clock ran.
const DEFAULT_TICKS: usize = ambition_demo_smash::SMASH_TIME_LIMIT_TICKS as usize;

/// The match budget this run is using, in ticks.
///
/// ⭐⭐ **THE CLOCK IS A PARAMETER BECAUSE THE VERDICT DEPENDS ON IT, and that
/// dependence is a live open question rather than a detail.** The verdict is
/// *stocks taken, then damage dealt* — and on the shipped ladder every inverted
/// cell has stocks TIED at `2 : 2`, so the verdict falls through to damage. ⇒
/// Every "rung N is weaker" result is really "rung N deals less damage per
/// minute", which is a different claim, because a fighter that refuses bad
/// commitments deals less damage and may still be harder to beat.
///
/// ⇒ A longer clock is the one arm that can separate those: give a patient rung
/// three minutes and either it converts patience into stocks (and the ladder is
/// fine, the instrument was too short) or it does not (and the ladder really is
/// inverted). See `awaiting-maintainer-decision.md`.
///
/// ⚠ A run at a non-default clock is NOT comparable to one at the default, and
/// the header says which was used for that reason.
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
    /// Tick each seat was eliminated on, or [`ticks()`] for a seat that survived.
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
    ///
    /// ⛔⛔ **IT REPLACES THE WHOLE WEIGHT SET ON EVERY RUNG WITH
    /// `UtilityWeights::v1()` PLUS YOUR FIELDS, WHICH FLATTENS AN AUTHORED
    /// LADDER'S WEIGHT RAMP.** `v1()` IS the shipped ladder's rung-9 row, so on
    /// a `--ladder` run this hands rung 1 the hardest fighter's scoring and
    /// leaves only reaction/APM/noise/read to separate the rungs. That is a
    /// legitimate controlled arm — one weight set, nine reaction profiles — and
    /// it is NOT "the shipped ladder with one number moved". The header prints
    /// every rung after the override, so a flattened table says so in its own
    /// rows; read them.
    ///
    /// ⇒ For the other question use [`Self::weight_scales`].
    #[arg(long = "weight", value_name = "NAME=VALUE")]
    pub weights: Vec<String>,

    /// Multiply one utility weight on EVERY rung by a factor, as
    /// `NAME=FACTOR`. Repeatable.
    ///
    /// ⭐⭐ **THIS IS THE ONE A REFIT WANTS, and until 2026-09-21 there was no
    /// way to ask it.** A refit moves a weight *relative to what the ladder
    /// authored*, keeping the ramp that makes rung 1 a beginner: `--weight-scale
    /// kill_potential=1.3` gives rung 1 `0.00` (still nothing), rung 5 `0.91`
    /// and rung 9 `1.495`. `--weight kill_potential=1.3` would give all nine
    /// `1.3` and delete the ramp.
    ///
    /// ⚠ Scaling a weight that is authored as ZERO cannot move it, by
    /// construction — rungs 1 and 2 author `kill_potential: 0.00`, so no factor
    /// reaches them. That is the ladder saying those rungs do not price kills,
    /// and a refit that needs to change it wants `--weight`, or an edited
    /// `.ron`.
    ///
    /// ⚠ Applied AFTER `--weight`, so passing both scales the value you set.
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
    /// ⭐⭐ **THE NULL CONTROL, AND `--rungs 6,6` ALONE IS NOT IT.** Nothing here
    /// had ever answered the prior question — *do two IDENTICAL fighters split
    /// evenly?* — and a measurement tool that cannot measure zero cannot be
    /// trusted about small numbers. But equal rungs do not make equal fighters:
    /// with no `--character`/`--opponent` the row still seats the demo's two
    /// DEFAULT ids, and those are two different bodies.
    ///
    /// ⚠ MEASURED 2026-09-21, not inferred from the ids. `smash_duelist_a`
    /// wears `player_robot_v3` — 256px frames, 133 authored animations, body
    /// bbox 57x91 (0.22 x 0.36 of the frame). `smash_duelist_b` wears
    /// `player_robot_v2` — 64px frames, 42 animations, bbox 17x37 (0.27 x
    /// 0.58). They author the SAME eight hitboxes, so the active volumes match;
    /// what differs is the hurtbox the other fighter has to hit and the 91
    /// animations one of them does not author at all.
    ///
    /// ⇒ So `--rungs 6,6 --paired` on the defaults is a FIGHTER comparison
    /// wearing a null control's clothes, and it returns a real result:
    /// `LOWER outfights [3:12 = 80%, p=0.035]` on the shipped ladder — Robot v2
    /// beating Robot v3 at one rung, printed unqualified. Read as the null it
    /// claimed to be, that number would have condemned the instrument.
    ///
    /// ⇒ **THE NULL CONTROL IS `--rungs X,X --character F --opponent F
    /// --paired`**, which pairs by swapping the two seats' NOISE STREAMS (see
    /// [`Mirror::Noise`]) because with one rung and one fighter the stream is
    /// the only thing left that tells the seats apart. Anything but `even`
    /// there is the seat, and every ladder verdict carries it.
    ///
    /// ⛔⛔ **AND THE FIRST RUN OF IT FAILED.** Rung 6 against itself, shipped
    /// ladder, 40 paired seeds each:
    ///
    /// ```text
    /// smash_duelist_a     seat0 17 : 6  seat1   (+17 tied)   p = 0.035
    /// smash_duelist_b     seat0 11 : 4  seat1   (+25 tied)   p = 0.118  (within spread)
    /// smash_george_booul  seat0 12 : 8  seat1   (+20 tied)   p = 0.503  (within spread)
    /// pooled              seat0 40 : 18 seat1   (+62 tied)   p = 0.0054
    /// ```
    ///
    /// ⇒ **Seat 0 takes 69% of decided pairs, and all three fighters lean the
    /// same way.** So this rig has a seat term worth roughly 69:31 on a decided
    /// pair, and an UNPAIRED row — which is the DEFAULT, and which every number
    /// recorded before `--paired` existed used — carries it undiscounted. The
    /// paired rung and fighter arms cancel it, which is what they are for; they
    /// now also have its size.
    ///
    /// ⚠ NOT THE PLACEMENT, checked rather than assumed:
    /// `ambition_demo_smash::respawn_placement` alternates the seats outward
    /// from the stage centre, so seats 0 and 1 sit at ±32px of a symmetric
    /// platform and the initial seating is the same call. The cause is
    /// somewhere else — decision order within a tick is the obvious candidate
    /// and has not been measured. ⇒ Named, not chased.
    ///
    /// ⭐ The TIES are the arm's own evidence that it works: 62 of 174 pairs
    /// (36%) came out exactly level, which is what exchanging a term and
    /// nothing else should do to a third of seeds.
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
    /// Match budget in SECONDS. Absent means the demo's own
    /// `SMASH_TIME_LIMIT_TICKS` — the shipped eight minutes.
    ///
    /// See [`ticks()`] for why this is a
    /// parameter: the verdict falls through to damage whenever stocks tie, so a
    /// longer clock is the arm that separates "this rung is weaker" from "this
    /// rung is patient and the clock was too short".
    #[arg(long, value_name = "SECONDS")]
    pub seconds: Option<usize>,
    /// Load an authored difficulty ladder from a `.ron` file and install it, so
    /// the rig measures THAT ladder instead of the engine floor.
    ///
    /// ⭐⭐ **THIS IS THE FLAG THAT LETS THE RIG MEASURE THE SHIPPED FIGHTER.**
    ///
    /// ⛔⛔ AND UNTIL 2026-09-21 IT SILENTLY DISABLED EVERY OTHER TUNING FLAG.
    /// Installing the resource hands `cfg.profile` to
    /// `project_authored_fighter_ladder`, which rewrites any live profile that
    /// differs from its rung on every tick — so `--weight`, `--apm`, `--noise`
    /// and `--reaction-ms`, all of which wrote live brains, were reverted within
    /// a tick. MEASURED: `--apm 1` and `--apm 600` produced byte-identical bouts
    /// on this road; on the floor they are `0% : 0%` and `21% : 9%`. The
    /// override now goes into the ROWS this flag reads, so it survives. See
    /// `ProfileOverride`.
    ///
    /// Every number this tool has ever produced was taken on the engine floor:
    /// the demo app installs no `AuthoredFighterLadder`, so `profile_for_level`
    /// falls back to `FighterBrainProfile::for_level`. That floor differs from
    /// the shipped ladder in two ways that matter — it gives every rung the
    /// level-9 utility weights (`UtilityWeights::default()` IS `v1()`), and it
    /// switches the L3 rollout ON at level 6, which the authored ladder
    /// deliberately disables on all nine rows.
    ///
    /// ⇒ Point it at `game/ambition_content/assets/data/fighter_brain_ladder.ron`
    /// to measure what a player fights. ⚠ Reading a file is a MEASUREMENT-tool
    /// choice and deliberately not a composition change: whether the demo app
    /// itself should compose `ambition_content` is a product decision that
    /// belongs to Jon (`awaiting-maintainer-decision.md`), and this flag settles
    /// the measurement question without pre-empting it.
    #[arg(long, value_name = "PATH")]
    pub ladder: Option<String>,
    /// Print one line per BOUT beneath each row, not just the medians.
    ///
    /// ⭐ Added 2026-09-04 because a summary row could not settle a question its
    /// own numbers raised: the `6 vs 5` survival gap is a constant +4.5s with the
    /// rollout on and exactly +0.0 in all nine fixtures with it off, and a median
    /// cannot say whether "+0.0" means the two bodies died together or neither
    /// died before the match resolved. Those are different claims about the
    /// engine and the table cannot separate them.
    #[arg(long)]
    pub per_bout: bool,
    /// Stage to fight on. The names are the stage button's own labels,
    /// lowercased — `flat` (the demo's default), `platforms`, `narrow` — and an
    /// unknown one is refused with the live list rather than defaulted.
    ///
    /// ⚠ THIS LINE SAID "`flat` (default) or `platforms`" WHILE THE RIG ALREADY
    /// ACCEPTED `narrow`, which is the same defect one layer up from the stage
    /// itself: a third stage was added, the resolver was derived from
    /// `SmashStageChoice::ALL` so it needed no edit, and the HELP was the one
    /// place still hand-listing two.
    ///
    /// ⭐ Every ladder number recorded before 2026-09-04 was measured on `flat`,
    /// which was the only stage there was. That makes the stage a CONFOUNDER
    /// sitting under the whole corpus — spacing, recovery and edgeguard results
    /// were all taken on one layout — and this flag is what turns it into a
    /// variable that can be compared instead of a constant nobody chose.
    ///
    /// ⛔ **THE DEFAULT IS EMPTY, NOT `"flat"`, AND THAT IS DELIBERATE.** It used
    /// to be the literal `"flat"`, which happened to match
    /// `SmashStageChoice::default()` — and a default that is right by coincidence
    /// is the shape that produced five separate wrong-configuration measurements
    /// in this rig on 2026-09-04 (weights, ladder source, rollout, fighters,
    /// clock). ⇒ Empty resolves to the demo's OWN default at the point of use, so
    /// changing `SmashStageChoice::default()` moves the rig with it instead of
    /// silently leaving it behind.
    #[arg(long, default_value = "")]
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

    // ⭐⭐ THE HEADER IS PRINTED ONCE, HERE, BEFORE THE MODE IS CHOSEN — and that
    // placement is the point rather than a tidy-up.
    //
    // ⛔ It used to be called by each mode, and `--sweep-below` never called it:
    // that mode returns before either of the other two call sites, so it printed
    // its numbers with no ladder line, no fighters line and no clock line at all.
    // ⚠ Found while checking whether the CLOCK fix had reached every mode, which
    // is the same shape as the five configuration defects this rig has already
    // produced — each fix reached the callers somebody remembered.
    //
    // ⇒ Above the branch, a fourth mode cannot be added without a header. That is
    // a structural guarantee where three call sites were a habit.
    report_which_ladder_is_in_play();

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
        // ⚠ `--weight-scale` is an override too, and this arm used to call the
        // run "not overridden" while a factor was multiplying every rung. It
        // does not print the factors itself: they are on the `ladder:` line,
        // beside the rows they modified, which is where a reader can see what
        // they did.
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
    // ⭐ THE BAR SITS WITH THE STATISTICS rather than among the
    // configuration lines: it is the last thing a reader passes before the
    // column header, because it is what they need in hand while reading the
    // rows underneath.
    report_what_this_run_could_report();
    println!(
        // ⛔ "stocks" ALONE IS AMBIGUOUS AND WAS MISREAD. The column is stocks
        // REMAINING, so `0 : 0` means BOTH fighters were fully eliminated — the
        // opposite of the "nobody lost a stock" it reads as at a glance. Say
        // LEFT in the header, where the reader is.
        "[ladder_rig] higher vs lower   survived(hi:lo)   stocks LEFT(hi:lo)   dealt%(hi:lo)   peak%(hi:lo)   \
         verdict = who OUTFOUGHT. ⚠ PAIRED rows decide it per SEED (stocks, then \
         damage on a stock tie) and the columns beside it are pooled medians, \
         DESCRIPTIVE ONLY; UNPAIRED rows decide it from those medians   \
         (median of {seeds} seeds, {}s each, {})",
        ticks() / 60,
        // The design belongs in EVERY table's header, not just the scenario
        // one. This mode had no such line while `--paired` silently did nothing
        // here, so a reader had two reasons to be misled and no way to see either.
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
/// The `--ladder` file, parsed and wrapped, or `None` when the flag is absent.
///
/// ⛔ A parse failure EXITS rather than falling back to the floor. Falling back
/// would produce a run whose header says one thing and whose fighters carry
/// another, which is the failure this file has spent a day removing.
/// The profile fields this run overrides, as ONE value applied wherever the
/// profile is owned.
///
/// ⛔⛔ **THE OVERRIDES USED TO POKE LIVE BRAINS, AND ON THE SHIPPED LADDER THAT
/// LOST EVERY TICK.** `project_authored_fighter_ladder` carries no change filter
/// — it cannot, because no tick-based filter composes with the disabling
/// component a candidate session builds behind — so it re-reads every fighter
/// every tick and rewrites any `cfg.profile` that differs from its rung,
/// rebuilding `FighterState` with it. ⇒ A `--weight` written onto a live brain
/// was reverted within one tick, and the only lasting effect was the state
/// rebuild it provoked.
///
/// ⚠ MEASURED 2026-09-21, which is how it was found. On `--ladder <shipped>`,
/// `--apm 1` and `--apm 600` produced byte-identical bouts, as did
/// `--weight reach_fit=0` and `--weight reach_fit=999`, and all four equalled
/// each other — the value never mattered, only whether a force had happened at
/// all. The same flags on the ENGINE FLOOR, where no ladder resource exists and
/// the projection returns early, moved every number: `--apm 1` took the fight
/// from `21% : 9%` to `0% : 0%`. **The rig's entire reason to exist on the
/// shipped ladder was a no-op, and every header it printed claimed otherwise.**
///
/// ⇒ ONE OWNER. With a ladder installed the override goes into the ROWS before
/// the resource is inserted, so the projection projects it; with no ladder the
/// floor owns the profile and the override goes onto the live brains. Same
/// value, same function, and the road is chosen by who owns the fact.
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
    /// ⚠ A borrowed slice rather than fields, because these multiply the
    /// AUTHORED row: `apply` never sees the ladder, only one profile at a time,
    /// so the factor has to travel with the override rather than be folded into
    /// a value up front.
    scales: &'a [(String, f32)],
}

impl<'a> ProfileOverride<'a> {
    /// An override that changes nothing — the reference every `from_args`
    /// answer is compared against, and the fixture a test starts from.
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
    /// ⚠ `None` and an all-default `Some` are different: an override that
    /// changes no field still costs a profile write, and on the ladder road that
    /// write is what a reader would mistake for the flag working.
    fn from_args() -> Option<Self> {
        // Parsed once per process and leaked so the override can borrow it: the
        // scales live as long as `args()` does, and threading a lifetime from a
        // `OnceLock` through every call site buys nothing a leak of one small
        // `Vec` does not.
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
        // ⛔ LAST, AND MULTIPLYING WHATEVER IS THERE. The point of a scale is to
        // move a weight relative to what the ROW authored, so it must see the
        // row (or the `--weight` value that replaced it), not a constant.
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
    // ⛔ THE OVERRIDE GOES IN HERE, NOT ONTO THE LIVE BRAINS, because with this
    // resource installed the projection owns `cfg.profile` and rewrites it every
    // tick. See `ProfileOverride` for the measurement that found it.
    if let Some(over) = ProfileOverride::from_args() {
        for rung in ladder.rungs_mut() {
            over.apply(rung);
        }
        // ⚠ A SWEEP CAN MAKE A LADDER THAT IS NO LONGER A LADDER — `--apm 1`
        // flattens every rung's cap onto one number — and the caller is entitled
        // to know before reading the rows as a difficulty curve. Reported, not
        // refused: flattening a field deliberately is exactly what a controlled
        // arm does.
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

/// A short, stable digest of the ladder FILE's bytes, so two runs can be shown
/// to have read the same rows rather than the same path.
///
/// ⚠ Deliberately not a cryptographic hash and deliberately not `Hash` on the
/// parsed rows: the parsed form drops comments and formatting, so two files that
/// differ visibly could digest alike, and the reader comparing two logs is
/// asking about the INPUT they were handed, not about a canonical form of it.
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
/// ⭐ Prints ALL of them rather than the pair currently under investigation.
/// A summary narrowed to today's question is a summary that silently agrees
/// with tomorrow's different arm.
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

/// Two-sided SIGN TEST on paired differences: is this split surprising for a
/// fair coin?
///
/// Returns `true` for "within spread" — i.e. NOT significant at p < 0.05, so the
/// verdict should be read with its qualifier and discounted.
///
/// ⭐ The sign test is the right instrument for this data specifically because it
/// throws information away. Each pair contributes only WHICH rung dealt more
/// damage, never by how much, so a single lopsided bout cannot carry a cell —
/// and bout damage here is bounded, skewed and bimodal, which is exactly where
/// tests that trust magnitudes go wrong.
///
/// ⛔ TIES ARE DROPPED, not counted for either side. A pair whose two halves deal
/// identical damage is evidence about neither rung, and folding it in as half a
/// success would manufacture confidence out of a non-result.
/// ⚠ TEST-ONLY SINCE THE REPAIR, and labelled rather than deleted. Production
/// no longer turns paired DIFFERENCES into signs — `paired_outcomes` produces
/// the outcomes directly, ordered stocks-first, and `paired_verdict` reads the
/// same split for both the word and the test. What survives here is the
/// sign-conversion half of the old road, kept because the properties its tests
/// state (ties discarded, more agreeing evidence never less significant) are
/// properties of the shared core below and are cheapest to state this way.
/// ⇒ The PRODUCTION tie path is covered separately, at the real entry point, by
/// `level_pairs_are_dropped_rather_than_counted`.
#[cfg(test)]
fn sign_test_says_within_spread(diffs: &[f32]) -> bool {
    let positives = diffs.iter().filter(|d| **d > 0.0).count();
    let negatives = diffs.iter().filter(|d| **d < 0.0).count();
    // ⭐ The threshold spelled out here rather than hidden behind a wrapper.
    // The wrapper existed only for this helper, so in a non-test build it was
    // dead — which `cargo test -p` cannot see and `check_no_warnings` did.
    sign_test_p(positives, negatives) >= 0.05
}

/// The sign test on the COUNTS, so the direction and the inference are two
/// readings of one split rather than two computations.
///
/// ⛔⛔ SPLITTING THIS OUT IS THE WHOLE REPAIR, not a tidy-up. While the only
/// entry point took `&[f32]` differences and returned a bare bool, the caller
/// had no way to learn WHICH side the significant split favoured — `k =
/// positives.max(negatives)` throws it away — so the reported direction had to
/// come from somewhere else, and it did: pooled medians over every bout. Two
/// authors of one row's meaning, free to disagree, and they did.
/// ⛔⛔ AND THE TAIL IS RETURNED, NOT A BOOLEAN, BECAUSE `(within spread)` WAS
/// DOING TWO JOBS AND NOTHING TOLD THEM APART.
///
/// Measured 2026-09-04 on the shipped ladder: `6 vs 5` is 5:7 and `9 vs 6` is
/// 7:5 — **p = 0.774, a coin** — while a cell one pair short of clearing is
/// p = 0.146. Both printed the identical qualifier, so the page reading them had
/// no way to separate *"nearly"* from *"not at all"*, and it treated them as one
/// kind of near miss for weeks. The same collapse happens above the threshold:
/// 11:1 and 2:10 are both *significant* and only the first survives a pair
/// flipping.
///
/// ⇒ The threshold is unchanged; the caller compares against `0.05` and the row
/// prints the number, so the distinction is readable rather than recomputable.
/// The smallest majority that clears p < 0.05 at `pairs` usable pairs, or
/// `None` when no split can.
///
/// ⛔ ONE AUTHORITY, because there were two: the header line that tells a reader
/// what this run could report, and the test asserting a longer run accepts a
/// weaker majority, each ran their own `find` over `sign_test_p`. Two
/// computations of one fact — and the fact is the bar the whole table is read
/// against.
fn smallest_clearing_majority(pairs: usize) -> Option<usize> {
    (pairs / 2..=pairs).find(|&k| sign_test_p(k, pairs - k) < 0.05)
}

fn sign_test_p(positives: usize, negatives: usize) -> f64 {
    let n = positives + negatives;
    // ⛔ THERE WAS AN EXPLICIT `if n < 6 { return true }` HERE AND IT WAS DEAD
    // CODE. Removing it changed no test, which is how it was found: the poison
    // arm that deleted it stayed GREEN while the other two reddened.
    //
    // ⇒ The exact tail already covers it. Five unanimous pairs are
    // 2 * 0.5^5 = 0.0625, which is not below 0.05, so an underpowered run
    // reports `(within spread)` by the arithmetic rather than by a special case.
    // ⭐ Keeping the branch would have meant a line no test could distinguish
    // from its absence, guarding a case the formula already handles — so the
    // FACT it documented is worth keeping and the code was not.
    //
    // ⚠ That fact, for the reader of a small run: fewer than six usable pairs
    // cannot reach significance no matter how unanimous they are. Such a cell is
    // `(within spread)` because the run is too short, not because the rungs are
    // alike, and those are different statements about the fighters.
    let k = positives.max(negatives);
    // Two-sided exact binomial tail: 2 * P(X >= k) for X ~ Binomial(n, 0.5).
    // Computed by summing terms rather than via a normal approximation, because
    // n is small enough that the approximation is the sloppier of the two and
    // the sum is a dozen multiplications.
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



/// Say WHICH TWO FIGHTERS the run is about, including when nobody chose them.
///
/// ⭐⭐ **A DEFAULT THAT APPEARS ONLY IN THE SOURCE IS THE ONE THAT SURVIVES FOUR
/// INVESTIGATIONS.** `--character` and `--opponent` have always existed, so the
/// fighters were nameable the whole time — the runs simply defaulted, and the
/// header never said to what. It took four separate findings before anyone
/// checked, and the answer was that every ladder number ever taken measured two
/// STAND-INS: `smash_duelist_a` and `smash_duelist_b` get `fighter_moveset()`,
/// which bound 18 verbs to George's 26 and had no special button at all until
/// 2026-09-04. ⇒ Printing a default costs one line and is the only thing that
/// lets a reader notice it is wrong.
fn report_which_fighters_are_in_play() {
    let [higher, lower] = fighters();
    let chosen = flag_value("--character").is_some() || flag_value("--opponent").is_some();
    let george = ambition_demo_smash::SMASH_GEORGE_BOOUL;
    let stand_ins = higher != george && lower != george;
    println!(
        "[ladder_rig] fighters: `{higher}` (higher rung) vs `{lower}` (lower rung){}{}",
        if chosen { "" } else { " — DEFAULTED, nobody passed --character/--opponent" },
        if stand_ins {
            // ⚠ Not phrased as a defect in the fighters. It is a statement about
            // what the run is ABOUT, which is the thing a reader needs in order
            // to know whether the number answers their question.
            format!(
                ". ⛔ Neither is `{george}`, the demo's one fully authored fighter — \
                 these carry `fighter_moveset()`, so this measures the STAND-INS. \
                 Concretely: their unanswered presses are George's plus EIGHT MORE, \
                 every one a `special` (only `special_forward` answers), because \
                 `fighter_moveset()` is the one contract that does not go through \
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
/// ⛔ **ONE OWNER, BECAUSE THE PARSE AND THE HEADER USED TO BE TWO.** The header
/// printed `args().stage` — the RAW flag — while the world was built from a
/// separate `match` over the same string. They agreed only because both spelled
/// `"flat"`. ⇒ Making the flag's default empty (so it can defer to
/// `SmashStageChoice::default()`) would have made the header print an empty
/// stage name while the run measured the real one: a header and a run
/// disagreeing, which is the exact failure this file has spent a day removing.
///
/// ⭐ So both go through here, and the header prints `label()` — the same string
/// the game's own stage button shows.
fn resolved_stage() -> ambition_demo_smash::SmashStageChoice {
    // ⛔⛔ RESOLVED FROM `SmashStageChoice::ALL`, NOT FROM STRING LITERALS. This
    // was a `match` over `"flat"` and `"platforms"`, and when a THIRD stage was
    // authored it stayed a two-arm match: the stage existed, the select screen
    // cycled to it, and the one instrument that measures stages could not be
    // pointed at it. A stage nobody can take a number on cannot do the job a
    // third stage was added for.
    //
    // ⭐ The names come from `label()`, which is what the game's own stage
    // button shows — so the flag a reader types is the word they saw on screen,
    // and a renamed stage renames its flag rather than orphaning it.
    let asked = args().stage.trim().to_ascii_lowercase();
    if asked.is_empty() {
        // ⭐ Nobody passed `--stage`: take the demo's OWN default rather than
        // naming one here, so changing it moves the rig with it.
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
            // ⛔ `{:.0}%` PRINTED "0%" FOR A TWO-SECOND RUN, which reads as
            // "measures nothing" and is the same rounding collapse that made two
            // p-values a hundred times apart both print as 0.000. A fraction
            // this small wants a scale, not a rounded percent.
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

/// Say what majority this run's SEED COUNT could even report, before any row.
///
/// ⛔⛔ THE BAR IS A PROPERTY OF THE RUN LENGTH AND NOTHING SAID SO. A cell reads
/// `(within spread)` for two unrelated reasons — the rungs are alike, or the run
/// is too short for any split to clear — and a reader met both wearing the same
/// words. Below six usable pairs NOTHING can clear, not even a unanimous sweep;
/// at four seeds a `4:0 = 100%` row still prints `(within spread)`.
///
/// ⚠ AND THE BAR FALLS AS SEEDS RISE, which is the trap in comparing two runs:
/// 83.3% at 12 pairs, 80.0% at 15, 71.4% at 28. A longer run can clear the same
/// line with a materially weaker majority, so "significant at 12 and at 28" is
/// two different claims. Printing the bar per run is what lets a reader see
/// which one they are holding.
fn report_what_this_run_could_report() {
    // Ties are dropped by the sign test, so this is the CEILING on usable pairs
    // — a run with ties has fewer, and a harsher bar than this line states.
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
        // ⛔⛔ THIS LINE SAID ONLY "the AUTHORED rows" AND THAT MADE TWO ARMS
        // INDISTINGUISHABLE IN THEIR OWN OUTPUT. Every `--ladder <path>` run
        // printed the identical header, so a shipped-ladder arm and a candidate
        // -tuning arm — whose ONLY difference is the file — produced logs a
        // reader cannot tell apart. The neighbouring `weights:` line even says
        // "see the ladder line below", pointing at a line that did not carry the
        // rows. ⇒ A header that cannot name the configuration it measured is the
        // defect this tool's own documentation keeps recording, one level up.
        //
        // ⭐ The path alone is not enough: paths get reused and edited in place.
        // The digest is over the file TEXT actually parsed, so two runs agree if
        // and only if they read the same bytes.
        let rungs = ladder_rungs_summary();
        // ⛔ AND IT SAYS WHETHER THE ROWS BELOW ARE STILL THE FILE'S. The
        // override now goes INTO the rows (`ProfileOverride`), so the rungs
        // printed underneath can differ from the bytes the digest covers — and
        // a reader comparing two logs by digest would conclude they used the
        // same rows. The digest still describes the FILE, which is what it is
        // for; this clause describes what happened to it afterwards.
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
/// ⛔ `displacement_value` USED TO BE MISSING FROM THE MATCH and the arm said
/// *"no weight named 'displacement_value'"*, so the one weight a reader might
/// reach for after the knockback work was the one the rig refused. A field
/// added to `UtilityWeights` and not added here is invisible to the only tool
/// that can sweep it.
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
            // Refused here rather than silently ignored, so a typo is not a run
            // that measured the unmodified weights under a header claiming
            // otherwise.
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
            // ⛔ THE STREAM IS KEYED ON THE SEAT, WHICH IS WHY IT CAN BE THE
            // CONTROL AND WHY IT HAD TO BECOME ONE. A zero stream is a
            // legitimate SplitMix64 state but an unhelpful one to start every
            // seat on, so the seat index separates them — and that makes the
            // stream a per-seat term indistinguishable from placement until
            // something swaps it. `Mirror::Noise` is that something: XOR with 1
            // exchanges the two seats' streams and leaves every other seat fact
            // where it was.
            state.noise = noise_stream(seed, seat.0, swap_streams);
            applied = true;
        }
    }
    applied
}

/// The SplitMix64 state one seat starts on, for one seed and one pairing.
///
/// ⭐⭐ **THE PROPERTY IS AN EXCHANGE, NOT A DIFFERENCE, and the two look alike
/// at the call site.** [`Mirror::Noise`] cancels the stream term by giving each
/// seat the OTHER seat's stream — so `swap_streams` must permute the two
/// streams, not derive two fresh ones. `seat + 1` would also "swap" in the
/// sense of changing both, and would put a third and fourth stream into the
/// pair with nothing cancelled, leaving a null control that still measures
/// noise. `^ 1` is an involution on the two seats and is the whole reason this
/// is one line with a name.
fn noise_stream(seed: u64, seat: usize, swap_streams: bool) -> u64 {
    // A zero stream is a legitimate state but an unhelpful one to start every
    // seat on, hence the `+ 1`.
    let stream_of = seat ^ usize::from(swap_streams);
    seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (stream_of as u64 + 1)
}

/// Apply this run's [`ProfileOverride`] to every live fighter — the FLOOR road.
///
/// ⛔ Four functions used to do this, one per flag, each with its own
/// `world.query::<&mut Brain>()` and its own `found` nobody read. They are one
/// function because they are one decision: *this run measures a modified
/// profile*. Splitting it by flag is what let the ladder road keep three of
/// them while the fourth was the only one anybody re-checked.
///
/// ⚠ It pokes the LIVE cfg rather than going through the published policy,
/// which is the point of a sweep and deliberately not a model of how a fighter
/// gets its weights. Do not "fix" it to match the builder. ⛔ But it is only
/// correct where the floor owns the profile: with an `AuthoredFighterLadder`
/// installed the override belongs in the ROWS, and `ProfileOverride` says why.
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
    // ⛔⛔ THE LEDGE-HANG FIXTURE IS ANCHORED TO FLAT'S PLATFORM, WHATEVER
    // `--stage` SAYS, so this mode REFUSES on any other stage rather than
    // reporting a number taken against the wrong geometry.
    //
    // `place_at` reads `smash_stage().world.blocks[0].aabb` — always Flat,
    // `x = 80..560` — and installs `LedgeGrabState::hanging` on that contact.
    // Narrow's real platform is `x = 160..480`, so a Narrow ledge hang stages the
    // fighter EIGHTY PIXELS past the ledge it claims to be holding, and the
    // scenario then measures an edgeguard against a body hanging in mid-air.
    // ⚠ The comment above that line says the anchor "comes from the REAL
    // platform", which was true when Flat was the only stage.
    //
    // ⇒ REFUSED RATHER THAN FILTERED. Dropping the ledge-hang scenarios on a
    // non-Flat stage would change the suite's denominator without saying so, and
    // a table with a quietly different population is worse than no table — this
    // rig has already produced five configuration defects of exactly that shape.
    // ⇒ REFUSED RATHER THAN FIXED, for now: deriving the anchor from the live
    // session's `RoomGeometry` (the seam `stage_bounds()` already uses) is the
    // repair, and it is correctness work that has not been done yet. A refusal
    // closes the wrong-data window COMPLETELY and immediately; a repair that has
    // not landed does not.
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
             profile source gave it (the `ladder:` line ABOVE names the file it \
             read and prints every rung)"
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
        ticks() / 60,
        resolved_stage().label(),
        // ⛔ THE DESIGN IS PART OF THE NUMBER. A paired table and an unpaired one
        // answer the same question with different controls, and two runs whose
        // headers do not say which cannot be compared.
        if args().paired {
            pairing_axis()
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
    // ⛔ THIS MODE PRINTED NO HEADER AT ALL, which is the same defect the other
    // two modes were fixed for on 2026-09-04 — a run that does not name its
    // ladder, its fighters or its clock is a number nobody can compare with
    // another number. ⚠ It was missed because the fix was applied to
    // `report_which_ladder_is_in_play`'s CALLERS and this mode returns before
    // reaching either of them. ⇒ Third mode, same rule: say what was resolved,
    // including what nobody passed.
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
/// The two fighters with the seats EXCHANGED — the fighter-comparison twin of
/// swapping the rungs.
///
/// ⭐⭐ **WHY THIS EXISTS.** `--paired` cancels the seat term by running each seed
/// twice with the RUNGS swapped. That is the right control when the rungs are
/// what differ — and it is a tautology when they are the same, which the guard in
/// `bouts_for_seed` says out loud. ⇒ But `--rungs 5,5 --character A --opponent B`
/// is a perfectly good question ("is fighter A stronger than B at one rung?") with
/// a real variable in it; the variable is simply the FIGHTER, not the rung. So the
/// pairing swaps that instead, and the seat term cancels exactly as it does for a
/// rung comparison.
///
/// ⚠ Measured need, not a generalisation: an unpaired `5 vs 5` George-against-a-
/// stand-in run produced a 329% : 225% damage gap and still came back `(within
/// spread)`, because unpaired seed variance is what `--paired` exists to remove.
/// The question could be ASKED and could not be ANSWERED.
/// Refuse a `--character`/`--opponent` this app cannot seat, naming what it can.
///
/// ⛔ AN ABSENT OR EMPTY REGISTRY IS A REFUSAL, NOT A PASS. Skipping the check
/// when the vocabulary is missing would accept every id including the typos, and
/// "no registry" is itself worth saying out loud — it means the warm-up updates
/// did not prepare the cast this bout is about to seat.
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

/// What `--paired` actually swaps for THIS run, in the words the header prints.
///
/// ⛔⛔ **THE HEADER SAID "the rungs swapped between seats" ON EVERY RUN, AND ON
/// AN EQUAL-RUNG ROW THAT IS THE ONE THING IT DOES NOT SWAP.** `bouts_for_seed`
/// has three pairings and the design line named one of them, so a reader of a
/// `--rungs 6,6` table was told the control cancelled a term the row does not
/// contain. ⇒ The axis is derived from the same two inputs the pairing branches
/// on, so a fourth arm cannot leave this line behind.
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

/// The row's word and whether to qualify it — the ONE place a row's meaning is
/// decided, so a test can ask the ROW and not only its parts.
///
/// ⛔⛔ EXTRACTED BECAUSE THE UNIT TESTS COULD NOT SEE THE DEFECT. With the
/// paired authority written and five regressions green, `report_row` was
/// deliberately re-wired back to the broken shape — the word from pooled
/// medians, the qualifier from the pairs — and **all ten tests still passed.**
/// They pinned `paired_verdict`, which was never what was wrong: the bug lived
/// in which authority the ROW consulted. ⇒ A test that constructs its subject
/// cannot witness that subject being bypassed, and the fix is to give the row's
/// decision a name something can call.
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
    // ⚠ DESCRIPTIVE ONLY ON A PAIRED ROW. These pooled medians used to AUTHOR
    // the verdict outright; on a paired row the paired outcomes do, and these
    // stay as the columns a reader compares. On an unpaired row there are no
    // pairs to reduce, so they are still the best available answer.
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
        // ⛔⛔ THIS WAS `mid.abs() < 0.5 * (hi - lo)` AND THAT TEST RAN BACKWARDS.
        //
        // `hi - lo` is the RANGE of the paired differences, and a range only
        // GROWS as you add seeds — every new pair can widen it and none can
        // narrow it. Meanwhile the median converges. ⇒ So the old criterion got
        // strictly HARDER to pass the more evidence you collected, which is the
        // exact opposite of what a significance test does.
        //
        // ⚠ CAUGHT BY IT ACTUALLY HAPPENING, 2026-09-04, not by reading: the
        // `3 vs 1` cell of the shipped-ladder arm was the ONE cell in sixteen
        // that printed without `(within spread)` at 12 seeds, and re-running the
        // identical arm at 40 seeds made it `(within spread)`. More power, less
        // significance. A single outlier pair also sets the range outright,
        // making it the least robust statistic available for the job.
        //
        // ⇒ REPLACED BY A SIGN TEST, which is the standard non-parametric test
        // for paired data and has none of those properties: count how many pairs
        // favour the higher rung, and ask how surprising that split is under a
        // fair coin. It gains power with seeds, ignores the magnitude of
        // outliers entirely, and assumes nothing about the distribution — which
        // matters here because bout damage is bounded, skewed and bimodal.
        let (word, within, split) = paired_verdict(&paired_outcomes(bouts));
        (word, within, Some(split))
    } else {
        // ⛔⛔ AN UNPAIRED ROW MAKES NO INFERENCE AT ALL, and printing one was the
        // paired road's defect surviving on the road that is the DEFAULT.
        //
        // The qualifier here was a range test over DAMAGE, computed whatever
        // decided the word — so a unanimous STOCK outcome was discounted by
        // variance in a quantity that had not authored it. The reviewer's
        // fixture: higher takes 2 stocks in every bout and lower takes 0, while
        // damage runs `[0, 100, 0, 100]` against `[45, 55, 45, 55]`. Median
        // stocks 2 : 0 say `higher outfights`; damage medians are 50 : 50 with a
        // wide higher range, so the row printed `(within spread)` over a sweep.
        //
        // ⭐ AND THE FIX IS NOT A BETTER THRESHOLD OVER STOCKS. An unpaired run
        // does not cancel the seat — this tool's own header says *"unpaired —
        // seat 0 is always the higher rung"*, and 7 of the 9 fixtures place seat
        // 0 offstage. A significance statement over seat-confounded samples is a
        // confident answer to a question the DESIGN cannot answer, however it is
        // computed. `--paired` exists precisely to buy the inference.
        //
        // ⇒ The word stays — pooled medians are the best DESCRIPTION available —
        // and the inferential qualifier is replaced at the print site by the
        // design fact, which is true and is what a reader should discount by.
        // ⚠ No split: an unpaired row HAS no per-seed pairs to report, and
        // inventing one from pooled medians is the two-authorities defect this
        // file spent a day removing.
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
/// ⭐ THE HALVES ARE ALREADY ORIENTED. `bouts_for_seed` calls `.mirrored()` on
/// the swapped half, so `[0]` means the higher rung in BOTH bouts of a pair and
/// this function must not swap anything itself. Re-orienting here would undo
/// the mirror and average each rung with the other — the failure
/// `the_mirror_puts_the_seats_back` exists to catch, which is why that test is
/// load-bearing for this one.
///
/// ⚠ SUMMED ACROSS THE PAIR, not compared bout by bout. The pair is the unit
/// `--paired` buys: the seat term appears once on each side and cancels in the
/// sum. Comparing the two halves separately would put the seat term back.
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

/// The row's word and its qualifier, BOTH read off the same split.
///
/// ⛔⛔ THE DEFECT THIS REPLACES COULD PRINT A DIRECTION ITS OWN EVIDENCE
/// CONTRADICTED. The displayed verdict came from pooled medians over every
/// bout; the qualifier came from a sign test on per-pair DAMAGE differences;
/// and the sign test's answer was reduced to `p >= 0.05`, discarding which side
/// had won. So a row could print `LOWER outfights`, unqualified, while its own
/// significance evidence favoured HIGHER 16 pairs to 4. Reproduced as
/// `a_row_cannot_be_significant_in_the_direction_it_does_not_report`.
///
/// ⚠ AND THE QUALIFIER TESTED THE WRONG QUANTITY WHENEVER STOCKS DECIDED. The
/// old comment claimed it was "measured on the DECIDING quantity" — true only
/// while damage decided, false on every row where `hi_took != lo_took`. A
/// comment asserting a requirement the code misses by one condition is read by
/// exactly the person who would otherwise check.
///
/// ⇒ There is now ONE authority. The direction is whichever side more pairs
/// favoured; the significance is the same split's exact two-sided sign test.
/// They cannot disagree, because there is nothing left to disagree with.
/// The per-seed split a paired verdict was computed from.
///
/// ⛔⛔ IT IS RETURNED BECAUSE A VERDICT NOBODY CAN RE-DERIVE IS A VERDICT YOU
/// MUST TRUST. `paired_verdict` used to hand back only `(word, within_spread)`,
/// so a row printed `higher outfights` beside pooled-median columns that no
/// longer decide anything, and the 10-versus-2 that actually produced it existed
/// for one stack frame and was gone. A reader who wanted to check the sign test
/// had no numbers to check it with — `fighter-brain.md` recorded that as the
/// repaired tool's one remaining limitation.
// ⚠ No `Eq`: it carries an `f64`. Tests compare the integer counts and the
// rendered string, which is what a reader sees anyway.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PairedSplit {
    higher: usize,
    lower: usize,
    tied: usize,
    /// The exact two-sided sign-test tail this split produced.
    p: f64,
}

impl PairedSplit {
    /// `10:2` — and `+1 tied` only when there IS one, so the common row stays
    /// narrow and a dropped pair is never silently invisible.
    fn describe(self) -> String {
        let pairs = if self.tied == 0 {
            format!("{}:{}", self.higher, self.lower)
        } else {
            format!("{}:{} +{} tied", self.higher, self.lower, self.tied)
        };
        // ⭐ THE p TRAVELS WITH THE SPLIT. `0.0386` and `0.0063` are both
        // "significant" and only one of them survives a pair flipping; `0.774`
        // and `0.146` are both "(within spread)" and only one is a near miss.
        //
        // ⛔ AND IT NEEDS A SCALE, NOT THREE DECIMALS. The first version printed
        // `{:.3}`, so a 28-seed run showed `p=0.000` for BOTH 3.0e-6 and 1.8e-4
        // — two results a hundred times apart, rendered identically, by the very
        // change that existed to stop a token collapsing two states.
        let p = if self.p < 0.001 {
            format!("{:.1e}", self.p)
        } else {
            format!("{:.3}", self.p)
        };
        // ⛔⛔ AND THE MAJORITY AS A PERCENTAGE, because comparing two runs by
        // their p is the trap. A LARGER n accepts a WEAKER majority — the
        // smallest reportable split is 83.3% at n=12 and 71.4% at n=28 — so a
        // bigger run can clear the same line with a materially smaller effect
        // and print no differently. The proportion is what is comparable across
        // run lengths; the p is not.
        let usable = self.higher + self.lower;
        let pct = if usable == 0 {
            String::new()
        } else {
            let share = 100.0 * self.higher.max(self.lower) as f64 / usable as f64;
            // ⛔ NAME THE DENOMINATOR WHEN A TIE MOVED IT. `0:3 +1 tied = 100%`
            // reads as 100% of four to anyone who has not memorised that the
            // sign test drops ties — and it is 100% of THREE. The bar the header
            // quotes is against the seed count, so a row whose denominator is
            // smaller must say so or the two cannot be compared.
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
    // ⛔ TIES ARE DROPPED rather than split, the same rule the sign test uses: a
    // pair that came out level is evidence about neither rung.
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

/// The midpoint of a sample.
///
/// ⛔ THIS RETURNED `values[len / 2]`, THE UPPER MIDDLE ORDER STATISTIC, for a
/// decade of even-sized runs — and every ladder run is even-sized under
/// `--paired`. On stock summaries, which are small integers, that is the
/// difference between a row reading `0` and `1`: a 20-bout sample split
/// 10 zeroes / 10 ones reported ONE, the more flattering half, for both seats.
/// ⚠ A function named `median` with hidden even-N semantics is worse than an
/// honestly-named one, because every caller reads the name and not the body.
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
    // ⚠ The parameter was called `ticks`, which now collides with the run's
    // clock function of that name. Renamed rather than shadowed: a `secs` that
    // silently compared a bout's elapsed ticks against ITSELF would print every
    // bout as ">Ns" and nothing would fail.
    if elapsed >= ticks() as f32 {
        format!(">{}s", ticks() / 60)
    } else {
        format!("{:.1}s", elapsed / 60.0)
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
    let (hi_dealt, lo_dealt) = (dealt(0), dealt(1));
    // a verdict inside the seeds' own spread is not a verdict. Reported
    // rather than suppressed: the reader should see the overlap and discount the
    // word, not be handed a cleaner-looking table.
    //
    // ⚠ Measured on the DECIDING quantity. It used to test the survival times
    // while the word above described survival; now the word describes damage
    // dealt, so the spread that matters is damage's. Leaving it on the old
    // column would have marked a decisive damage gap "within spread" whenever
    // the two seats happened to die at similar times.
    // ⭐ PAIRED RUNS ARE TESTED ON PER-SEED OUTCOMES, NOT ON TWO POOLED MEDIANS.
    // ⚠ This said "on the DIFFERENCES" and described the pre-`36dd9a248` road:
    // paired inference consumed per-pair DAMAGE differences then. It consumes
    // categorical stocks-first `PairedOutcome`s now, and the sentence outlived
    // the mechanism it described — the same way the "DECIDING quantity" comment
    // did, two lines from the code that contradicted it.
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
    let (verdict, overlaps, split) = row_verdict(bouts, properly_paired);
    // ⭐ THE SPLIT TRAVELS WITH THE WORD. A reader can now re-derive the sign
    // test from the row instead of trusting it — `10:2` against 12 pairs is
    // checkable by hand, and it is the only number on the line that DECIDED
    // anything, the medians beside it being descriptive.
    let seen = split.map(|s| format!(" [{}]", s.describe())).unwrap_or_default();
    let verdict = if overlaps {
        format!("{verdict}{seen} (within spread)")
    } else if properly_paired {
        format!("{verdict}{seen}")
    } else {
        // ⚠ NOT a significance claim, and deliberately not shaped like one: it
        // names the DESIGN that produced the row, so a reader discounts it for
        // the right reason instead of reading an unqualified word as resolved.
        format!("{verdict} (unpaired — seat not cancelled)")
    };
    if args().per_bout {
        // ⚠ RAW, and in the order the bouts were run — a paired run emits each
        // seed's straight bout and then its mirror, so the pairs are adjacent
        // and a reader can see the swap rather than trust it.
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

/// What the SECOND bout of a `--paired` seed exchanges between the two seats.
///
/// ⭐⭐ **THE AXIS IS THE WHOLE DESIGN, because a control that cancels the wrong
/// term produces symmetric-looking output that reads as rigour.** Each variant
/// is the control for exactly one question, and picking the wrong one answers a
/// different question in the same columns.
///
/// ⚠ `Rungs` is absent on purpose: the rung swap is expressed by handing
/// `run_bout_at` its two rungs the other way round, so it never reaches here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mirror {
    /// The first bout of a pair, and every unpaired bout.
    Straight,
    /// The two fighter ids change seats. The control for *is A stronger than B*.
    Fighters,
    /// The two seats' NOISE STREAMS change places, and nothing else does.
    ///
    /// ⭐⭐ **THIS IS THE ONE THAT MAKES THE NULL CONTROL RUNNABLE.** One rung
    /// against itself with one fighter in both seats has no variable left except
    /// the seat — placement, and the stream `force_noise_seed` derives from the
    /// seat index. Swapping the streams cancels the stream, and what survives the
    /// pair is the SEAT TERM alone: the number every ladder verdict carries and
    /// none of them had ever been measured against.
    Noise,
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
    let straight = run_bout_at(higher, lower, seed, start.cloned(), Mirror::Straight);
    if !args().paired {
        return vec![straight];
    }

    // ⛔ THE SAME SEED, THE ROLES SWAPPED, AND — ON TWO ARMS OF THREE — THE
    // RESULT PUT BACK THE RIGHT WAY ROUND. `run_bout_at(lower, higher, ..)`
    // seats the LOWER rung where the fixture puts SELF, so `mirrored` swaps the
    // pair back and every `[0]` stays "the higher rung"; reporting the raw
    // mirror would average each rung with the other one. The seat null is the
    // exception and `Pairing::reorient` is where that is decided, ONCE.
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
/// ⛔⛔ **THIS IS A TABLE BECAUSE A TEST COULD NOT OTHERWISE SEE THE CHOICE.**
/// The three arms each ended in their own `return vec![straight, ...]`, so the
/// decision "does the seat null mirror?" lived at a call site and nothing could
/// ask it. Poisoning that site — adding `.mirrored()` to the noise arm, which
/// averages each seat with the other and returns `Even` for any pair whatsoever
/// — left **every test in this file green**, because the tests pin
/// `paired_outcomes` and the defect was in which orientation the row handed it.
/// That is this file's own recorded failure mode: *a test that constructs its
/// subject cannot witness that subject being bypassed*.
///
/// ⇒ One road now, and `the_seat_null_is_the_one_pairing_that_must_not_reorient`
/// reads the table rather than a hand-built pair.
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
    /// ⚠ `same_fighter` is whether the two IDS RESOLVE to one fighter, which is
    /// not the same as the ids being equal in spelling — and the distinction is
    /// what made the fighter arm masquerade as a seat null for a week. The
    /// caller compares the ids; see `Mirror::Fighters`' arm for what that buys
    /// and what it does not.
    fn of(higher: u8, lower: u8, same_fighter: bool) -> Self {
        match (higher == lower, same_fighter) {
            // Unequal rungs: the ladder's own question. The rung is the variable.
            (false, _) => Self {
                mirror: Mirror::Straight,
                swap_rungs: true,
                reorient: true,
            },
            // ⭐⭐ EQUAL RUNGS, DIFFERENT FIGHTERS: pair on the FIGHTER instead.
            //
            // `--rungs 5,5 --character A --opponent B` asks a real question — is
            // A stronger than B at one rung — and its variable is the fighter,
            // not the rung, so swapping the rungs would cancel a term the row
            // does not contain. Swapping the SEATS the two fighters occupy is
            // the same control applied to the actual variable.
            //
            // ⛔⛔ THE ABSENCE OF THIS WAS A DEFECT, NOT A MISSING FEATURE.
            // `--paired` swapped the RUNGS, so a fighter comparison got a
            // control that cancelled the wrong term — and **a control that
            // cancels the wrong term is worse than no control, because it
            // produces symmetric-looking output that reads as rigour.** The old
            // equal-rung arm printed perfectly equal columns and an `even`
            // verdict, which is what a careful null control looks like.
            //
            // ⚠ Measured cost, not a hypothetical: an UNPAIRED `5 vs 5` run of
            // George against a stand-in gave a **329% : 225%** damage gap and
            // still reported `(within spread)`, because unpaired seed variance
            // is exactly what `--paired` removes. ⇒ The question could be ASKED
            // and could not be ANSWERED, and nothing in the output said so.
            //
            // ⛔⛔ AND THIS ARM IS NOT A SEAT NULL CONTROL, THOUGH THE COMMENT
            // HERE SAID IT WAS. It claimed the demo's default pair —
            // `smash_duelist_a` and `smash_duelist_b` — both receive
            // `fighter_moveset()` and so "swapping them exchanges the SEATS and
            // nothing else". The moveset half is true. The "nothing else" is
            // not: the two ids wear different sheets and the sheet carries the
            // body. MEASURED 2026-09-21 — v3 is 256px frames with 133 authored
            // animations and a 57x91 body bbox, v2 is 64px with 42 and 17x37.
            // The same eight hitboxes, so the same active volumes; a different
            // hurtbox, and 91 animations on one side the other does not author.
            // ⇒ It is a real and useful FIGHTER comparison and it answers
            // `LOWER outfights [3:12 = 80%, p=0.035]` at rung 6 on the shipped
            // ladder. It is simply not the question a reader of `--rungs 6,6`
            // was told it was. **Read the arm by what the ids RESOLVE to, not
            // by the fact that they differ.**
            (true, false) => Self {
                mirror: Mirror::Fighters,
                swap_rungs: false,
                reorient: true,
            },
            // ⭐⭐ ONE RUNG, ONE FIGHTER: THE SEAT NULL CONTROL, AND IT IS NOT
            // DEGENERATE — which is what the arm it replaced believed.
            //
            // That arm warned and fell through, reasoning that with both rungs
            // and both ids equal the swapped call is the SAME call, so the pair
            // is `[B, B.mirrored()]` — equal columns by construction, on a
            // biased instrument as readily as an unbiased one. Right about the
            // FIGHTER swap, wrong about the row: the two seats are still not
            // interchangeable, because `noise_stream` derives each brain's
            // stream from its seat index. ⇒ Exchange the STREAMS and the pair
            // cancels the one term that tells the seats apart, leaving the
            // seat's own placement — the term every ladder verdict carries.
            //
            // ⛔ AND IT MUST NOT REORIENT. Everywhere else `[0]` means "the
            // higher rung" or "the `--character` fighter" and the mirror puts it
            // back. Here the subject IS the seat, so index 0 has to keep meaning
            // seat 0 in both halves; mirroring would average each seat with the
            // other and hand back the equal columns this arm exists to stop
            // manufacturing.
            (true, true) => Self {
                mirror: Mirror::Noise,
                swap_rungs: false,
                reorient: false,
            },
        }
    }
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

/// One bout, optionally started from a scenario's positions.
fn run_bout_at(
    higher: u8,
    lower: u8,
    seed: u64,
    start: Option<ambition_platformer2d::combat::brain::fighter::scenarios::Scenario>,
    mirror: Mirror,
) -> Bout {
    let mut app = build_demo_app();
    // ⛔ BEFORE the warm-up updates, because `project_authored_fighter_ladder`
    // applies the rows to brains with `Added<Brain>` — a ladder installed after
    // the fighters exist would never reach them, and the run would silently
    // measure the floor while its header claimed the authored rows.
    if let Some(ladder) = authored_ladder() {
        app.world_mut().insert_resource(ladder);
    }
    // ⛔ BEFORE the route below, because the route is what prepares the session:
    // the preparation source reads this resource once, when the match is asked
    // for. Setting it afterwards would change nothing and look like it worked.
    app.world_mut()
        .insert_resource(resolved_stage());
    for _ in 0..30 {
        app.update();
    }
    // ⛔⛔ **THE IDS ARE CHECKED HERE, AGAINST THE APP THAT WILL SEAT THEM.**
    // `--character`/`--opponent` were taken verbatim: `--character __nope__`
    // printed *"`__nope__` (higher rung)"* in the header as though it were a
    // fighter, ran, and died a thousand lines later inside the noise-seed
    // anti-vacuity guard — *"no fighter brain ever took the noise seed"* — a
    // sighted guard firing for the right reason and naming the wrong party. A
    // reader debugging that goes looking at seeds.
    //
    // ⚠ AND IT IS NOT AN EDGE CASE IN THIS APP. `ambition_demo_smash_app` has no
    // `ambition_content` edge, so Ambition's authored cast is NOT seatable here
    // — the first person to pass a shipped fighter's name gets that message.
    //
    // ⭐ VALIDATED AGAINST THE APP'S OWN REGISTRY rather than a list kept in this
    // tool: `PreparedCharacterRegistry` is what the composition actually
    // prepared, so it cannot drift from what can be seated, and a fighter added
    // to the demo needs no edit here.
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

    // A seat that is ELIMINATED stops existing, so the last value seen is the
    // answer — reading only at the end would report zero for both.
    let mut stocks = [ambition_demo_smash::STARTING_STOCKS; 2];
    let mut eliminated = [ticks(); 2];
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
    let overrides = ProfileOverride::from_args();
    // Who owns `cfg.profile` for this run: the installed ladder, or the floor.
    let ladder_owns_profile = args().ladder.is_some();
    let mut placed = start.is_none();
    for tick in 0..ticks() {
        app.update();
        if !seeded {
            seeded = force_noise_seed(&mut app, seed, mirror == Mirror::Noise);
            if seeded {
                // ⛔ ONLY ON THE FLOOR ROAD. With a ladder installed the rows
                // already carry this override and the projection would revert
                // anything written here within one tick — see `ProfileOverride`.
                // Only when the caller asked, too: forcing unconditionally is
                // what flattened the ladder; see `weights_from_args`.
                if let Some(over) = overrides.filter(|_| !ladder_owns_profile) {
                    force_profile(&mut app, over);
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
            if appeared[slot] && !seen[slot] && eliminated[slot] == ticks() {
                eliminated[slot] = tick;
                stocks[slot] = 0;
            }
        }

        // ⭐⭐ STOP WHEN THE MATCH IS OVER. Both seats eliminated means an empty
        // stage, and every further tick simulates nothing at real cost: at the
        // shipped 480s clock a bout that resolves at ~98s was spending **four
        // fifths of its time** on a stage with no fighters on it.
        //
        // ⛔ It changes no measurement, and that is asserted rather than
        // reasoned: every column is recorded above before this runs —
        // `eliminated` is a tick already stamped, `stocks` are already zero,
        // `damage_taken` cannot grow for a body that is gone, and
        // `closest_approach` has no pair to measure. ⚠ Verified by running a cell
        // before and after and diffing: **byte-identical in BOTH modes** — the
        // ladder's `3 vs 1` at 12 seeds paired, and the scenario matrix's
        // `5 vs 3` across four fixtures. ⚠ The second run was the point: the
        // change lives in `run_bout_at`, which every mode shares, and verifying
        // only the mode I was looking at would have been a claim about one
        // caller offered as a property of the function.
        //
        // ⚠⚠ AND THE SPEEDUP IS 1.75x, NOT THE ~5x THE ARITHMETIC PREDICTS —
        // 126s → 72s on that cell. A bout resolving at 85s of a 480s budget
        // should have saved four fifths of its ticks, so the shortfall is itself
        // a measurement: **a large share of a bout's cost is FIXED** (building
        // the app, the warm-up updates, the route) rather than simulated ticks.
        // ⇒ Worth knowing before anyone optimises this loop further — the next
        // win is in the setup, not here.
        //
        // ⭐ SOLVED FOR, from the same two timings rather than a new run. With
        // 24 bouts, a 480s budget and an ~85s resolve: 126s → 72s gives
        //     sim ≈ 5.7 ms per simulated second (~176x realtime)
        //     fixed setup ≈ 2.5 s per bout
        // ⇒ So an 85-second bout costs **0.5s of simulation and 2.5s of setup —
        // 84% fixed**. Building the app, its warm-up updates and the route
        // dominate, and no tick-loop work can reach them. ⚠ The lever is reusing
        // one app across bouts, which is a determinism question (each bout wants
        // a clean world) and therefore not a free win.
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
    ///
    /// ⛔⛔ THIS DOC AND ITS `#[test]` SPENT THEIR WHOLE LIFE ON THE WRONG
    /// FUNCTION. A second doc comment and a second `#[test]` followed
    /// immediately, so both attributes bound to
    /// `adding_agreeing_evidence_never_makes_a_result_less_significant` and the
    /// mirror check below became an ordinary private fn nothing called — dead
    /// code wearing a test's name, reported only as `function ... is never
    /// used` among the crate's other unused-function warnings. ⇒ The guard that
    /// protects the orientation every paired reading depends on had never once
    /// run.
    /// ⭐⭐ THE PROPERTY THE OLD TEST VIOLATED: more evidence must not make a
    /// result LESS significant.
    ///
    /// The replaced criterion was `|median| < 0.5 * (max - min)` over the paired
    /// differences. A range only grows with n, so lengthening a run of unanimous
    /// pairs could flip a cell from significant to `(within spread)` — which is
    /// what happened to the `3 vs 1` cell between 12 and 40 seeds and is what
    /// sent me looking. This pins the direction rather than any single verdict.
    #[test]
    fn adding_agreeing_evidence_never_makes_a_result_less_significant() {
        // Unanimous pairs, with one deliberately huge outlier so a
        // magnitude-sensitive test would be dragged around by it.
        let mut diffs = vec![1.0f32, 2.0, 1.5, 0.5, 3.0, 1.0, 900.0];
        assert!(
            !sign_test_says_within_spread(&diffs),
            "seven unanimous pairs should be significant (p = 2 * 0.5^7 = 0.016)"
        );
        // Every further pair AGREES. Significance must not evaporate.
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

    /// ⛔ AND IT MUST STILL SAY "within spread" WHEN IT SHOULD.
    ///
    /// A test that never withholds its qualifier is not a test. Three cases the
    /// sign test has to get right, and the third is the one a magnitude test
    /// fails: one colossal difference against a majority of small opposing ones
    /// is NOT evidence, and the sign test refuses it by construction.
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

        // ⛔ Underpowered: five unanimous pairs cannot reach p < 0.05 (2 * 0.5^5
        // = 0.0625), and the run should say so rather than claim an effect.
        assert!(
            sign_test_says_within_spread(&[1.0, 1.0, 1.0, 1.0, 1.0]),
            "five pairs cannot be significant at any effect size, so a five-pair \
             run must report within spread — underpowered, not null"
        );

        // ⭐ The magnitude trap. One pair favours the higher rung by 5000; eight
        // favour the lower by a little. A mean or a range-scaled median would be
        // dominated by the outlier; the sign test sees 1 against 8.
        //
        // ⚠ THE 8 IS NOT ARBITRARY AND I GOT IT WRONG FIRST. I wrote this with
        // seven opposing pairs, expecting significance; the exact test refused,
        // and it was right — 7 of 8 is p = 2 * (8 + 1) / 256 = 0.070, which is
        // not below 0.05. 8 of 9 is 2 * (9 + 1) / 512 = 0.039, which is. ⇒ Worth
        // recording because it is the whole argument for computing the exact
        // tail instead of eyeballing "nearly unanimous": my intuition was off by
        // one pair, in the direction of claiming an effect.
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
        // ⛔ And one pair fewer is NOT significant, which is the line that makes
        // the assertion above a claim about the threshold rather than about
        // "lots of pairs agreeing".
        assert!(
            sign_test_says_within_spread(&outlier[..8]),
            "7 of 8 is p = 0.070 and must carry the qualifier — if this passes \
             without the qualifier the threshold has drifted"
        );

        // ⛔ TIES ARE DROPPED, and the property that states is INVARIANCE: ties
        // must not change the answer in either direction.
        //
        // ⚠ I first wrote this as "six unanimous pairs plus twenty ties must be
        // within spread" and it was wrong for the same reason as the fixture
        // above — I was reasoning about the padding instead of computing. Ties
        // are discarded, so six unanimous pairs are six unanimous pairs
        // (p = 0.031) whether or not twenty ties sit beside them. ⇒ The real
        // claim is that the twenty make NO difference, which is both stronger
        // and the thing that would actually break if ties were folded in as half
        // a success each.
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

    /// A bout where the higher rung dealt `hi`, the lower dealt `lo`, and each
    /// seat ended with the given stocks. `damage_taken[0]` is what the HIGHER
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

    /// ⛔⛔ THE ROW MAY NOT BE SIGNIFICANT IN A DIRECTION IT DOES NOT REPORT.
    ///
    /// This is the reviewer's fixture, kept exactly: 16 pairs where the higher
    /// rung deals `[1000, 0]` against the lower's `[400, 400]`, and 4 pairs
    /// where it deals `[0, 0]` against `[1000, 1000]`. Stocks are level
    /// throughout, so damage decides every pair.
    ///
    /// ⭐ THE FIRST ASSERTION IS THAT THE FIXTURE IS STILL ADVERSARIAL. Pooled
    /// medians over these 40 bouts say `LOWER`, because 24 of the higher rung's
    /// 40 per-bout figures are zero while the lower's sit at 400 — the old
    /// verdict authority. If a later change made the pooled reading agree with
    /// the paired one, this test would still pass while testing nothing, so the
    /// disagreement is pinned before the repair is checked.
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

    /// ⛔⛔ THE ROW ITSELF TAKES ITS WORD FROM THE PAIRS — the assertion the
    /// other tests here CANNOT make.
    ///
    /// Every regression above calls `paired_verdict` directly, and
    /// `paired_verdict` was never the broken part. When this repair was first
    /// written, `report_row` was deliberately wired back to the defect — word
    /// from pooled medians, qualifier from the pairs — and **all ten tests
    /// passed.** A test that constructs its subject cannot witness that subject
    /// being bypassed, so the row's decision was given a name and this asks it
    /// by that name.
    ///
    /// ⚠ IT USES THE SAME ADVERSARIAL FIXTURE ON PURPOSE. Pooled medians say
    /// `LOWER` here and the pairs say `higher` 16-4, so the two authorities give
    /// different answers and the test can only pass if the row consults the
    /// right one. On a fixture where they agree it would prove nothing.
    /// ⛔⛔ `(within spread)` DESCRIBED TWO DIFFERENT SITUATIONS AND THE ROW
    /// COULD NOT TELL THEM APART.
    ///
    /// Measured on the shipped ladder 2026-09-04: `6 vs 5` came out 5:7 and
    /// `9 vs 6` came out 7:5 — **a coin, p = 0.774** — while a cell one pair
    /// short of clearing is p = 0.146. Both printed the identical qualifier, so
    /// `fighter-brain.md` read them as the same kind of near miss for weeks.
    ///
    /// ⭐ AND THE SAME COLLAPSE HAPPENS ON THE OTHER SIDE OF THE THRESHOLD:
    /// 11:1 and 2:10 are both "significant", and only the first survives a pair
    /// flipping. The page treated them as one class of result until the number
    /// was on the row.
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

        // ⛔⛔ AND A LARGER n ACCEPTS A WEAKER MAJORITY, which is why the row
        // prints the percentage and not only the tail. A reader comparing two
        // runs by p alone reads a bigger, weaker result as confirming a smaller,
        // stronger one.
        // ⭐ THE SAME FUNCTION THE HEADER PRINTS FROM. The test used to run its
        // own `find`, so a change to the threshold could have moved the header
        // and left this asserting the old rule — a test agreeing with itself.
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

        // ⛔ THE FLOOR THE HEADER CLAIMS. `report_what_this_run_could_report`
        // tells a short run that NOTHING can clear, and says six is the floor.
        // Both halves asserted, because a header stating a bound nobody checks
        // is the shape this file has spent a day removing.
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
        // ⭐ AND THE ROW NOW CARRIES THE SPLIT THAT PRODUCED THAT WORD, so a
        // reader can re-derive the sign test instead of trusting it. This is
        // the fixture's own 10-0 sweep, and asserting it here is what stops the
        // printed `[10:0]` drifting away from the numbers it claims to report.
        let split = split.expect("a paired row must report its split");
        // ⭐ 16:4 IS THE EXACT SPLIT `fighter-brain.md` NAMED AS UNCHECKABLE —
        // *"no way to check the 16-versus-4 that produced it"*. It is this
        // fixture's, and it is now on the printed row.
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

        // ⛔ A TIE MOVES THE DENOMINATOR AND THE ROW MUST SAY SO. `0:3 +1 tied
        // = 100%` reads as 100% of four to anyone who has not memorised that
        // the sign test drops ties; it is 100% of THREE, and the header's bar
        // is quoted against the seed count, so the two cannot be compared
        // unless the smaller denominator is named.
        let tied = PairedSplit { higher: 0, lower: 3, tied: 1, p: sign_test_p(0, 3) };
        assert_eq!(
            tied.describe(),
            "0:3 +1 tied = 100% of 3 usable, p=0.250",
            "a split with ties must name the denominator its percentage is over"
        );
        // ⭐ AND THE UNPAIRED ROW IS DELIBERATELY UNCHANGED: with no pairs to
        // reduce there is no second authority to prefer, so the pooled reading
        // is still the honest one and still says LOWER here.
        assert_eq!(
            row_verdict(&bouts, false).0,
            "LOWER outfights",
            "an unpaired row keeps the pooled verdict — the repair narrows what \
             the pooled columns may decide, it does not delete them"
        );
    }

    /// ⛔ WHEN STOCKS DECIDE, THE INFERENCE FOLLOWS STOCKS.
    ///
    /// The old qualifier was computed from damage differences whatever decided
    /// the verdict, so a stocks-decided row was qualified by a quantity it had
    /// not used. Here every pair is won on stocks by the higher rung and lost on
    /// damage by a wide margin; the row must report — and test — the higher rung.
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

    /// ⛔ A LEVEL PAIR IS EVIDENCE ABOUT NEITHER RUNG, and folding it in would
    /// manufacture confidence.
    ///
    /// Five decisive pairs cannot reach significance — `2 * 0.5^5 = 0.0625`. Ten
    /// level pairs alongside them must not change that. If ties were counted for
    /// either side, or merely inflated `n`, this row would flip.
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

    /// ⭐ THE PAIRED READING IS BLIND TO THE SEAT, which is the property
    /// `--paired` is bought for — asserted on the outcome authority itself
    /// rather than on the damage arithmetic alone.
    ///
    /// A bout decided entirely by seat, paired with its own mirror, must reduce
    /// to `Even`. If `paired_outcomes` re-oriented the already-mirrored half, the
    /// seat term would come back and this pair would read as a win.
    #[test]
    fn a_pair_decided_only_by_the_seat_reduces_to_even() {
        let pair = vec![bout(), bout().mirrored()];
        assert_eq!(
            paired_outcomes(&pair),
            vec![PairedOutcome::Even],
            "the mirror cancels the seat, so neither rung won this pair"
        );
    }

    /// ⛔ `median` IS THE MIDPOINT, INCLUDING FOR EVEN SAMPLES — and every
    /// `--paired` run is even.
    ///
    /// The old body returned `values[len / 2]`, the upper middle. On a small
    /// integer column like stocks that is the difference between reporting 0 and
    /// reporting 1 for an evenly split sample.
    #[test]
    fn the_median_of_an_even_sample_is_the_midpoint_not_the_upper_middle() {
        assert_eq!(median(vec![0.0, 0.0, 1.0, 1.0]), 0.5);
        assert_eq!(median(vec![1.0, 2.0, 3.0]), 2.0);
        assert_eq!(median(vec![4.0, 1.0]), 2.5, "and it sorts first");
    }

    /// ⛔⛔ AN UNPAIRED ROW'S QUALIFIER MAY NOT BE AUTHORED BY A QUANTITY THAT
    /// DID NOT AUTHOR ITS DIRECTION — the paired road's defect, which survived
    /// on the road that is the DEFAULT.
    ///
    /// The reviewer's fixture, kept exactly: the higher rung takes two stocks in
    /// every bout and the lower takes none, while damage runs `[0, 100, 0, 100]`
    /// against `[45, 55, 45, 55]`. Median stocks 2 : 0 decide the word; damage
    /// medians are 50 : 50 with a wide higher range, and the old range test read
    /// that variance as "within spread" — **discounting a clean sweep on the
    /// strength of a quantity that had not decided anything.**
    ///
    /// ⭐ THE ASSERTION IS THAT NO INFERENCE IS MADE AT ALL, not that a better
    /// one is. An unpaired run does not cancel the seat, and 7 of the 9 fixtures
    /// place seat 0 offstage, so a significance statement over these samples is
    /// a confident answer to a question the design cannot answer however it is
    /// computed. `--paired` is what buys the inference.
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

    /// ⭐⭐ EVERY OVERRIDE MOVES ITS OWN FIELD AND NOTHING ELSE, because the
    /// four separate force functions this replaced could not be compared.
    ///
    /// ⚠ The `apply`-moves-one-field property is the cheap half. The half that
    /// mattered is WHERE it is applied: with an `AuthoredFighterLadder`
    /// installed the same value has to go into the ladder ROWS, because
    /// `project_authored_fighter_ladder` rewrites any live profile that differs
    /// from its rung, every tick. That is the engine's fact and it is pinned
    /// where it lives, by
    /// `a_profile_written_from_outside_is_reverted_on_the_very_next_tick`.
    ///
    /// ⚠ AND NO UNIT TEST HERE WITNESSES THE ROAD CHOICE — checked, not assumed.
    /// Poisoning `run_bout_at` to poke live brains on the ladder road as well
    /// leaves this whole file green, because with the override already in the
    /// rows the extra write is REDUNDANT rather than wrong. The defect was
    /// writing the brains INSTEAD of the rows, and only a bout can witness
    /// that: `--ladder <shipped> --rungs 6,6 --seconds 15 --apm 1` deals
    /// `0% : 0%` where `--apm 600` deals `68% : 43%`, and before this change
    /// both dealt `54% : 56%`.
    #[test]
    fn each_override_moves_its_own_field_and_leaves_the_rest_authored() {
        use ambition_platformer2d::characters::brain::fighter::{
            FighterBrainProfile, UtilityWeights,
        };
        let authored = FighterBrainProfile::for_level(6);
        let authored_kill = authored.utility_weights.kill_potential;
        // Non-vacuity for the scale case: doubling zero is zero, and the
        // assertion would pass without the scale ever being read.
        assert_ne!(
            authored_kill, 0.0,
            "the fixture rung authors no kill weight, so scaling it proves nothing"
        );

        // An override that asks for nothing changes nothing — the premise that
        // makes each single-field case below attributable.
        let mut untouched = authored;
        ProfileOverride::NOTHING.apply(&mut untouched);
        assert_eq!(untouched, authored, "an empty override moved a field");

        let mut weights = UtilityWeights::v1();
        weights.reach_fit = 0.0;
        let kill_doubled = [("kill_potential".to_string(), 2.0_f32)];
        // 0.5 set, then doubled, is 1.0 — a value neither flag produces alone,
        // so the assertion cannot pass on either one being ignored.
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
            // ⭐ A SCALE MULTIPLIES THE ROW, which is the whole difference from
            // `--weight`: rung 6 authors `kill_potential: 0.90`, so x2 is 1.80
            // and not the `v1` row's 1.15 with a 2 in it.
            (
                ProfileOverride {
                    scales: &kill_doubled,
                    ..ProfileOverride::NOTHING
                },
                &|p: &FighterBrainProfile| p.utility_weights.kill_potential == authored_kill * 2.0,
            ),
            // ⛔ AND THE ORDER BETWEEN THEM IS PART OF THE CONTRACT. `--weight`
            // replaces the set, `--weight-scale` multiplies what is there — so
            // passing both must scale the value you SET. Applying the scale
            // first makes `--weight` silently discard it, which is a run whose
            // header names a factor that never reached a fighter.
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
            // And nothing else did. `level` is the one field no flag touches and
            // the one the projection keys on, so losing it would send the
            // fighter to a different rung entirely.
            assert_eq!(
                profile.level, authored.level,
                "{over:?} moved the level, which is how a rung finds its row"
            );
        }
    }

    /// ⭐⭐ THE SEAT NULL CONTROL CANCELS THE STREAM ONLY IF THE SWAP IS AN
    /// EXCHANGE, and a swap that merely CHANGES both streams passes every
    /// eyeball check.
    ///
    /// [`Mirror::Noise`] is the arm that made the null control runnable: one
    /// rung, one fighter, and the pair differs only in which seat holds which
    /// noise stream. That cancels the stream across the pair **only** if the two
    /// halves hold the same two streams in the other order. `seat + 1` would
    /// also make both halves differ from each other and from the straight bout —
    /// and would put streams 1,2 in one half and 2,3 in the other, so the pair
    /// would carry three streams, cancel nothing, and report the noise it was
    /// built to remove. The columns would look no different.
    #[test]
    fn swapping_the_noise_streams_exchanges_them_rather_than_making_new_ones() {
        for seed in [0u64, 1, 7, 12_345, u64::MAX] {
            let (seat0, seat1) = (
                noise_stream(seed, 0, false),
                noise_stream(seed, 1, false),
            );
            // The premise: the seats are separated at all. Without this the
            // exchange below is vacuously satisfied.
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

    /// ⛔⛔ THE THREE PAIRINGS, READ OFF THE TABLE THAT DECIDES THEM — and the
    /// reason the table exists is that the version of this test which built its
    /// own pair could not see the defect.
    ///
    /// Each arm cancels exactly one term, and picking the wrong one answers a
    /// different question in the same columns. The seat null is the arm that
    /// must NOT re-orient: its subject is the seat, so index 0 has to keep
    /// meaning seat 0 in both halves. ⚠ The poison to run against this is
    /// `reorient: true` on the `(true, true)` arm; with the old shape the
    /// equivalent poison — `.mirrored()` at the call site — left every test in
    /// this file green.
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
        // Same rungs, two fighters — and `same_fighter` is about what the ids
        // RESOLVE to, which is why this arm is not the null control.
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
        // Stated as the property rather than the triple, so the reason survives
        // a fourth arm: only the arm whose subject is the seat keeps its columns.
        assert!(
            !null.reorient && ladder.reorient,
            "exactly the seat-null arm reports its seats where it measured them"
        );
    }

    /// ⛔⛔ AND THE NOISE ARM MUST NOT MIRROR, WHICH IS THE OPPOSITE OF EVERY
    /// OTHER ARM'S REQUIREMENT.
    ///
    /// On the rung and fighter arms `[0]` means "the higher rung" / "the
    /// `--character` fighter" and [`Bout::mirrored`] is what puts it back. On
    /// the seat null the SUBJECT IS THE SEAT, so index 0 has to keep meaning
    /// seat 0 in both halves. Mirroring the second half there averages each seat
    /// with the other and returns `Even` for any pair whatsoever — the
    /// equal-columns-by-construction failure the arm was written to stop
    /// manufacturing, and it would look like a clean null.
    #[test]
    fn the_seat_null_reports_a_seat_that_wins_both_halves_and_a_mirror_hides_it() {
        // Seat 0 deals more in both halves: `damage_taken[1]` is what seat 1
        // absorbed, which is what seat 0 dealt.
        let half = |taken: [f32; 2]| Bout {
            damage_taken: taken,
            ..bout()
        };
        // Stocks equal so the verdict falls through to damage, as every real
        // 6-vs-6 bout on the shipped ladder does.
        let level = |b: Bout| Bout { stocks: [0, 0], ..b };
        // Seat 0 out-deals seat 1 by the SAME 20 in each half — the signature
        // of a term that belongs to the seat rather than to the bout — while
        // the halves are otherwise different bouts.
        let pair = [level(half([10.0, 30.0])), level(half([26.0, 46.0]))];
        assert_eq!(
            paired_outcomes(&pair),
            vec![PairedOutcome::Higher],
            "seat 0 dealt more in both halves, so the raw pair must say so"
        );

        // The same pair with the second half mirrored, which is what every
        // other arm does and what this one must not.
        let mirrored = [pair[0], pair[1].mirrored()];
        assert_eq!(
            paired_outcomes(&mirrored),
            vec![PairedOutcome::Even],
            "mirroring the seat-null half averaged the two seats and erased the \
             very term the arm exists to measure"
        );
    }
}
