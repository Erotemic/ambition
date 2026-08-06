//! **Does a higher rung beat a lower one?** (FB6e — the ladder rig)
//!
//! `cargo run -p ambition_demo_smash_app --bin ladder_rig`
//!
//! ⭐ **the measurement `ladder_probe` could not make.** That probe seats ONE
//! fighter against a human seat with no controller — a body that never acts — so
//! every stock lost is a self-KO. That makes its number unusually clean (*"did it
//! kill itself"*) and makes a FIGHT impossible to observe. FB6e's
//! `l3_earns_its_depth` and §8's survival/damage ratios both need two fighters,
//! and until `smash_roster_at_levels` (2026-08-06) no roster could express one.
//!
//! ⛔ **THE LADDER IS SPARSE.** `SMASH_ROSTER_RON` registers
//! `duelist_l{1,3,5,6,9}` and nothing between, so "N vs N−1" over the registered
//! rungs is **(3,1), (5,3), (6,5), (9,6)** — four pairs, not eight. Asking for an
//! unregistered rung does not error: `spec_for_brain` hands back a generic row,
//! so the fighter is a statue and the rig would report a landslide as a finding.
//! `a_ladder_roster_seats_two_cpus_at_two_different_levels` is the guard.
//!
//! ## What it reports, and what each column cannot say
//!
//! * **time to elimination** for each seat — the outcome, and the column that
//!   discriminates. The seat that lasts longer won.
//! * **stocks left**, because a seat that survives the clock with three is a
//!   different result from one that survives with one, and time cannot tell
//!   those apart.
//!
//! ⚠ **pair every "it won" with "and it engaged"**, exactly as `ladder_probe`'s
//! own header does. A fighter that stands still beats one that walks off the
//! stage, and this repository has already read that as a 3× improvement once.
//!
//! ⚠ **the median over seeds, never one run.** The brain's noise stream is
//! seeded, and `ladder_probe` reported single samples as answers for a week.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::{FighterStocks, MatchSeat};

/// One minute at 60Hz — the same budget `ladder_probe` uses, so the two are
/// readable against each other.
const TICKS: usize = 3_600;

/// The rungs the demo actually registers. See the sparseness warning above.
const RUNGS: &[u8] = &[1, 3, 5, 6, 9];

const DEFAULT_SEEDS: usize = 3;

/// What one match said.
///
/// ⛔ **TIME, not stocks — and the first draft of this file got it wrong.**
/// `ladder_probe`'s own header records the lesson: *"it was stocks until
/// 2026-07-31, and stocks turned out to be a saturated metric: every level lost
/// all three, so the column read `3 3 3 3 3` and could not have reported an
/// improvement if one had happened."* Run as a fight, both seats lose all three
/// inside a minute, so a stocks column reads `0 : 0` at every rung and says
/// "tie" about matches that were not close.
#[derive(Clone, Copy, Debug)]
struct Bout {
    /// Tick each seat was eliminated on, or `TICKS` for a seat that survived.
    /// The LATER one won.
    eliminated: [usize; 2],
    /// Stocks remaining at the end — kept because a seat that survived with
    /// three is a different result from one that survived with one, and the
    /// time column cannot tell them apart.
    stocks: [u32; 2],
}

fn main() {
    let seeds = seed_count();
    println!(
        "[ladder_rig] higher vs lower   eliminated(hi:lo)     stocks   verdict   \
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
/// ⚠ **distinct, not shared.** Two brains stepping the same stream would make
/// the higher rung's jitter a function of the lower one's, which is a
/// correlation no real match has — and it would hide exactly the kind of
/// difference this rig exists to find.
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

fn seed_count() -> usize {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--seeds" {
            if let Some(value) = args.next().and_then(|v| v.parse().ok()) {
                return value;
            }
        }
    }
    DEFAULT_SEEDS
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
/// ⚠ **the SPREAD is what says whether a difference is a difference.** The two
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
    let hi_all: Vec<f32> = bouts.iter().map(|b| b.eliminated[0] as f32).collect();
    let lo_all: Vec<f32> = bouts.iter().map(|b| b.eliminated[1] as f32).collect();
    let hi_out = median(hi_all.clone());
    let lo_out = median(lo_all.clone());
    let hi_stocks = median(bouts.iter().map(|b| b.stocks[0] as f32).collect());
    let lo_stocks = median(bouts.iter().map(|b| b.stocks[1] as f32).collect());
    // The seat that lasted LONGER won. A tie here is a real tie only when both
    // survived the clock; two eliminations on the same tick is a coincidence
    // worth seeing rather than smoothing away.
    let verdict = if hi_out > lo_out {
        "higher lasts"
    } else if lo_out > hi_out {
        "LOWER lasts"
    } else if hi_out >= TICKS as f32 {
        "both survive"
    } else {
        "both die together"
    };
    // ⚠ **a verdict inside the seeds' own spread is not a verdict.** Reported
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
    println!(
        "[ladder_rig]   {higher:>2} vs {lower:<2}   {:>20} : {:<20} {hi_stocks:>3.0} : {lo_stocks:<3.0}  {verdict}",
        span(&hi_all),
        span(&lo_all)
    );
}

/// Seat the two rungs and run a full match.
///
/// ⚠ the 30 warm-up updates before the roster lands are `ladder_probe`'s, and
/// for its reason: the shell has to reach its stage before a roster means
/// anything.
fn run_bout(higher: u8, lower: u8, seed: u64) -> Bout {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
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
    // ⛔ **a seat that has not ARRIVED yet is not an eliminated one, and the
    // first draft could not tell them apart.** Seating is a transaction that
    // takes frames, so both seats are absent on tick 0 — and reading absence as
    // elimination reported every rung dying at 0.0s, which looks like a finding
    // and is a fixture that never started.
    let mut appeared = [false; 2];
    // ⛔ **the seed has to be WRITTEN, and the first draft took it and dropped
    // it.** `run_bout(_seed)` ignored its argument, so "median of 7 seeds" was
    // one deterministic match reported seven times — and the giveaway was that
    // 3 seeds and 7 seeds printed byte-identical columns. `ladder_probe` seeds
    // the same way: the noise stream lives on the live `FighterState`, so it can
    // only be set once a brain exists, which is after seating.
    let mut seeded = false;
    for tick in 0..TICKS {
        app.update();
        if !seeded {
            seeded = force_noise_seed(&mut app, seed);
        }
        let world = app.world_mut();
        let mut q = world.query::<(&MatchSeat, &FighterStocks)>();
        let mut seen = [false; 2];
        for (seat, remaining) in q.iter(world) {
            if seat.0 < 2 {
                seen[seat.0] = true;
                stocks[seat.0] = remaining.remaining;
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
    Bout { eliminated, stocks }
}
