//! HOW LONG THIS MATCH HAS ACTUALLY BEEN FOUGHT.
//!
//! ⭐⭐ TWO CONSUMERS ASKED THIS AND GOT THE SAME WRONG ANSWER. The timeout and
//! the item cadence both read `ActiveMatch::ticks_since_activation`, which
//! counts from the tick the cast was BUILT — through the opening ceremony, and
//! through every pause. A match paused for thirty seconds was thirty seconds
//! closer to timing out, and each consumer patched around the ceremony its own
//! way: the timeout not at all, the spawner with a hand-written `elapsed == 0`
//! standing in for "not during the countdown", which stops being true the moment
//! an interval is shorter than a countdown.
//!
//! ⛔⛔ AND THIS ONE COSTS WIRE FORMAT, which the thing it replaces did not.
//! `time_remaining` is a pure function of `(activated_on, now)` — its doc says
//! so, and calls it out as the reason a match clock costs no snapshot bytes. A
//! count of ticks the world actually moved cannot be recovered from two numbers,
//! because "how long was this paused" is not written anywhere else. That is the
//! price of the mechanic, paid deliberately: it is registered, and a rewind
//! restores it like any other simulation truth.

use bevy::prelude::{Res, ResMut, Resource};

use super::seating::{ActiveMatch, MatchInstance};
use super::PreparedMatch;
use crate::features::stocks_match::StocksMatchSettled;

/// Ticks THIS match has spent being fought — ceremony and pauses excluded.
///
/// Stamped with the match it counts, the same way [`StocksMatchSettled`] and the
/// sudden-death latch are: the next match reads zero without anybody retracting
/// anything, and a count left over from the previous one cannot time this one
/// out on its first tick.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct LiveMatchTicks {
    of: Option<MatchInstance>,
    /// Live gameplay time in MICROSECONDS.
    ///
    /// ⛔⛔ NOT A COUNT OF FRAMES. It incremented once per unfrozen simulation
    /// step, which is only the same thing while time runs at 1.0 — and it does
    /// not: every impact hitstop RAMPS the scale (0.917, 0.750, 0.983 are all
    /// real values from one match). A ramping tick advanced every fighter timer
    /// by a fraction and this clock by a whole tick, so the match timer and the
    /// item cadence ran fast against the gameplay they were timing.
    ///
    /// ⭐ MICROSECONDS, IN AN INTEGER, so it stays exact under rollback: a float
    /// accumulator would be snapshot state whose replay depends on the order it
    /// was summed in. The snapshot's shape is unchanged — one `u64`, as before.
    micros: u64,
    /// [`Self::micros`] as it stood at the START of this step.
    ///
    /// ⭐⭐ A PERIODIC CONSUMER NEEDS BOTH ENDS OF THE STEP, not a sample of one.
    /// The projection [`Self::of`] returns is no longer guaranteed to advance
    /// every step — at half speed a conceptual tick lands on two consecutive
    /// steps — so `elapsed % interval == 0` fires TWICE on the same conceptual
    /// tick. That is what [`Self::crossed`] exists to answer instead.
    ///
    /// ⛔ NOT SNAPSHOT STATE, and that is a scheduling contract rather than an
    /// oversight: this system runs in `WorldPrep` and every consumer is
    /// downstream of it (the clock's own registration says so), so the value is
    /// written before it is read on every tick it is read on. A rewind restores
    /// `micros`, and the first re-simulated step rebuilds this from it.
    prev_micros: u64,
}

impl LiveMatchTicks {
    /// How long `active` has been fought. Zero for a match this is not counting,
    /// which is the honest answer for one that has not started.
    pub fn of(&self, active: &ActiveMatch) -> u64 {
        if self.of == Some(active.instance()) {
            // Sixtieths of a second of LIVE gameplay, which is what both callers
            // (the match timeout and the item cadence) have always meant by
            // "ticks".
            self.micros * 60 / 1_000_000
        } else {
            0
        }
    }

    /// The boundary of `interval_ticks` this STEP crossed, as its ORDINAL —
    /// `1` is the first boundary, one whole interval into the match. `None`
    /// when the step crossed none.
    ///
    /// ⭐⭐ THE QUESTION IS "DID GAMEPLAY TIME CROSS N", not "is the sampled
    /// tick divisible by N". Those were the same question only while the clock
    /// advanced exactly one conceptual tick per step, and it stopped doing that
    /// the moment it started counting SCALED gameplay: under a hitstop ramp the
    /// projection repeats a value across consecutive steps, and a divisibility
    /// test fires on every one of them. The item spawner did exactly that —
    /// two items, and worse, two entities deriving the SAME `SimId` from the
    /// repeated sample.
    ///
    /// ⭐ AND THE ORDINAL IS WHAT AN IDENTITY SHOULD BE DERIVED FROM. It counts
    /// boundaries, so it cannot repeat however time is scaled; the projected
    /// tick can.
    ///
    /// ⚠ ONE BOUNDARY PER STEP. A step long enough to span two intervals
    /// reports only the one it landed past — an interval shorter than a frame
    /// is not a cadence.
    pub fn crossed(&self, active: &ActiveMatch, interval_ticks: u32) -> Option<u64> {
        if self.of != Some(active.instance()) || interval_ticks == 0 {
            return None;
        }
        // `micros * 60 / (interval * 1_000_000)` is `elapsed_ticks / interval`
        // without the intermediate truncation mattering — integer division is
        // monotone, so the two floors agree.
        let period = u64::from(interval_ticks) * 1_000_000;
        let ordinal = |micros: u64| micros * 60 / period;
        let now = ordinal(self.micros);
        (now > ordinal(self.prev_micros)).then_some(now)
    }

    /// Rebuild from a rollback snapshot. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn from_snapshot(of: Option<MatchInstance>, micros: u64) -> Self {
        Self {
            of,
            micros,
            // Rebuilt by `count_the_live_match_ticks` on the first re-simulated
            // step, before any consumer reads it. See the field's own note.
            prev_micros: micros,
        }
    }

    /// The snapshot's view of this counter. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn parts(&self) -> (Option<MatchInstance>, u64) {
        (self.of.clone(), self.micros)
    }
}

/// Count one tick of fought match, when this tick was one.
///
/// ⭐ THE FOUR REASONS A TICK DOES NOT COUNT, and each is the condition itself
/// rather than a proxy for it:
///
/// ```text
/// no active match      there is nothing to time
/// the ceremony         `opening_phase` is the authority that releases the cast
/// the world is stopped `sim_dt == 0` — a pause, a freeze, a full hitstop
/// already settled      the winner card's four seconds are not match time
/// ```
///
/// ⛔ NOT A PAUSE FLAG. "Is the world moving" is what the clock actually wants,
/// and it is one number the sim already computes; a menu state would be a second
/// opinion about it, and would miss every other reason the world stopped.
pub fn count_the_live_match_ticks(
    mut live: ResMut<LiveMatchTicks>,
    // ⛔ `Option`, and it was a plain `Res` for about an hour. Forty-eight
    // character fixtures compose this plugin without the runtime's time
    // assembly, and a required `WorldTime` failed parameter validation in every
    // one of them — a system that panics a bare App is a system that says
    // "compose the whole engine to test a character".
    //
    // The honest answer for a world with no clock is that no tick counted: this
    // asks whether the world MOVED, and a world with no time did not.
    time: Option<Res<ambition_time::WorldTime>>,
    tick: Option<Res<ambition_time::SimTick>>,
    active: Option<Res<ActiveMatch>>,
    prepared: Option<Res<PreparedMatch>>,
    settled: Option<Res<StocksMatchSettled>>,
) {
    let Some(active) = active else {
        return;
    };
    // A NEW MATCH STARTS THIS COUNTER AT ZERO, and does it here rather than
    // wherever a match is built: the stamp is the retraction, so nothing has to
    // remember to clear a resource it does not own.
    let instance = active.instance();
    if live.of != Some(instance.clone()) {
        live.of = Some(instance);
        live.micros = 0;
    }
    // ⭐ WHERE THIS STEP STARTED, recorded BEFORE every early return below so a
    // step that counted nothing reports no boundary crossed rather than
    // comparing against a stale end. See `LiveMatchTicks::crossed`.
    live.prev_micros = live.micros;

    if settled.is_some_and(|settled| settled.settled(&active)) {
        return;
    }
    let Some(time_dt) = time
        .as_deref()
        .map(|time| time.sim_dt())
        .filter(|dt| *dt > 0.0)
    else {
        return;
    };
    // The ceremony is measured on the RAW clock — it is the thing that has not
    // started yet, so it cannot be measured on the clock it gates.
    let held = prepared
        .as_deref()
        .zip(tick.as_deref())
        .and_then(|(prepared, tick)| {
            active
                .ticks_since_activation(tick.get())
                .map(|raw| prepared.rules().opening_phase(raw))
        })
        .is_some_and(|phase| !matches!(phase, super::prepared_match::OpeningPhase::Live));
    if held {
        return;
    }

    // ⭐ THE SCALED STEP, not one whole tick. `sim_dt` is already the amount of
    // gameplay this step is worth — the same number every fighter timer advances
    // by — so the match clock counts the same thing the match does.
    live.micros += (f64::from(time_dt) * 1_000_000.0).round() as u64;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;
    use bevy::prelude::*;

    fn match_activated_on(tick: u64) -> ActiveMatch {
        ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(tick))
    }

    /// A moving world with a live match and NO prepared plan.
    ///
    /// ⛔ NO `PreparedMatch`, and that is a real limit of this fixture rather
    /// than a shortcut: the type has no constructor outside `prepare_match`, and
    /// giving it one so a unit test could set `opening_countdown_ticks` would be
    /// scaffolding on a production type. The ceremony half is therefore pinned
    /// on the PRODUCTION path instead — see
    /// `smash_in_the_host::the_match_clock_does_not_start_until_the_cast_is_released`,
    /// where a real countdown actually runs. What is testable here is everything
    /// that does not need the plan.
    fn clock_world() -> App {
        let mut app = App::new();
        app.init_resource::<LiveMatchTicks>();
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.insert_resource(ambition_time::SimTick(0));
        app.insert_resource(match_activated_on(0));
        app.init_resource::<StocksMatchSettled>();
        app.add_systems(Update, count_the_live_match_ticks);
        app
    }

    fn step(app: &mut App) {
        let now = app.world().resource::<ambition_time::SimTick>().get();
        app.insert_resource(ambition_time::SimTick(now + 1));
        app.update();
    }

    fn counted(app: &App) -> u64 {
        let active = app.world().resource::<ActiveMatch>().clone();
        app.world().resource::<LiveMatchTicks>().of(&active)
    }

    /// ⛔⛔ AND A HALF-SPEED WORLD SPENDS THE CLOCK AT HALF SPEED.
    ///
    /// The stop case above is the BINARY half, and passing it is what made this
    /// look finished: `sim_dt == 0` was handled, so "the clock follows the
    /// world" seemed true. But time scaling is not binary — every impact hitstop
    /// RAMPS it, and `0.917`, `0.750` and `0.983` are all real values from one
    /// match. On those ticks every fighter timer advanced by a fraction and this
    /// clock advanced by a whole tick, so the match timer and the item cadence
    /// ran fast against the gameplay they were timing.
    ///
    /// ⭐ THE ARM IS A RATIO, NOT A CONSTANT: 120 steps at half speed must buy
    /// the same clock as 60 at full speed, whatever the tick rate is.
    #[test]
    fn a_half_speed_world_spends_the_match_clock_at_half_speed() {
        let mut app = clock_world();
        for _ in 0..60 {
            step(&mut app);
        }
        let full_speed = counted(&app);
        assert!(
            full_speed > 0,
            "the fixture counted nothing at full speed, so the comparison below \
             is between two zeroes"
        );

        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 120.0,
        });
        for _ in 0..120 {
            step(&mut app);
        }
        let after_half = counted(&app) - full_speed;
        // ⚠ ONE TICK OF SLACK, and it is truncation rather than drift: `1/120`
        // in `f32` is 8333.33µs, which rounds to 8333 and leaves 999_960µs after
        // 120 steps — one sixtieth short of a second. The defect this arm exists
        // for was 120 against 60, not 59 against 60.
        assert!(
            after_half.abs_diff(full_speed) <= 1,
            "120 steps of HALF-SPEED gameplay bought {after_half} of clock against \
             {full_speed} for 60 steps at full speed — the match clock is counting \
             FRAMES, not the gameplay those frames were worth"
        );
    }

    /// ⭐⭐ A BOUNDARY IS CROSSED ONCE, HOWEVER SLOWLY TIME IS RUNNING.
    ///
    /// ⛔⛔ THE SIBLING ARM ABOVE FIXED THE CLOCK AND BROKE ITS PERIODIC READER.
    /// Counting scaled gameplay is right, but the projection back to 60 Hz is
    /// then no longer guaranteed to ADVANCE every step: at half speed one
    /// conceptual tick lands on two consecutive steps. The item spawner asked
    /// `elapsed % every == 0`, so it fired on both — two items, and two entities
    /// deriving the SAME `SimId` from the repeated sample, which is a
    /// determinism defect rather than double loot.
    ///
    /// ⭐ THE ASSERTION IS `1, 2, 3, …` WITH NO REPEATS, not a count. A count
    /// cannot tell "crossed four boundaries" from "crossed three and reported
    /// one twice", which is exactly the failure.
    ///
    /// ⛔ AND THE ARMS STRADDLE THE THING THAT BROKE IT: full speed is the only
    /// rate the old sampling was ever correct at, half speed is where it
    /// doubled, and the ramp is what a real match actually runs at — `0.917`,
    /// `0.750` and `0.983` are measured hitstop scales from one match, none of
    /// which divides the frame evenly.
    #[test]
    fn each_interval_is_crossed_exactly_once_at_any_time_scale() {
        // Half a second, so a few seconds of fixture crosses several.
        const EVERY: u32 = 30;

        /// Drive the REAL clock for `steps`, taking each step's scale from
        /// `scales` in turn, and collect every ordinal `crossed` reported.
        fn crossings(scales: &[f32], steps: usize) -> Vec<u64> {
            let mut app = clock_world();
            let mut seen = Vec::new();
            for i in 0..steps {
                app.insert_resource(ambition_time::WorldTime {
                    raw_dt: 1.0 / 60.0,
                    scaled_dt: scales[i % scales.len()] / 60.0,
                });
                step(&mut app);
                let active = app.world().resource::<ActiveMatch>().clone();
                if let Some(ordinal) = app
                    .world()
                    .resource::<LiveMatchTicks>()
                    .crossed(&active, EVERY)
                {
                    seen.push(ordinal);
                }
            }
            seen
        }

        /// `1, 2, 3, …` — every boundary reported once, in order, none skipped.
        fn each_once(seen: &[u64]) -> bool {
            seen.iter().copied().eq(1..=seen.len() as u64)
        }

        let full = crossings(&[1.0], 240);
        assert!(
            full.len() >= 3,
            "the full-speed fixture crossed {} boundaries, so the comparisons \
             below are between empty lists",
            full.len()
        );
        assert!(each_once(&full), "full speed reported {full:?}");

        // ⛔⛔ THE ARM THAT FAILED. Twice the steps at half the speed is the same
        // gameplay, so it must buy the same boundaries — each of them once.
        let half = crossings(&[0.5], 480);
        assert!(
            each_once(&half),
            "a half-speed world reported {half:?} — the projected tick repeats \
             across consecutive steps and a divisibility test fires on every one \
             of them, so this interval dropped two items sharing one SimId"
        );
        assert!(
            half.len().abs_diff(full.len()) <= 1,
            "480 half-speed steps crossed {} boundaries against {} for 240 at \
             full speed",
            half.len(),
            full.len()
        );

        // A real match never runs at a round scale for long.
        let ramped = crossings(&[0.917, 0.750, 0.983, 1.0], 480);
        assert!(
            ramped.len() >= 3,
            "the ramping fixture crossed {} boundaries",
            ramped.len()
        );
        assert!(each_once(&ramped), "a ramping scale reported {ramped:?}");
    }

    /// ⭐⭐ A PAUSED MATCH IS NOT GETTING CLOSER TO TIMING OUT.
    ///
    /// ⛔⛔ AND THIS IS THE HALF THAT COSTS WIRE FORMAT. `SimTick` advances while
    /// paused — deliberately; it is the netcode timeline and its doc says so —
    /// so no function of `(activated_on, now)` can answer this. The count is the
    /// only record of it.
    #[test]
    fn a_stopped_world_does_not_spend_the_match_clock() {
        let mut app = clock_world();
        for _ in 0..10 {
            step(&mut app);
        }
        assert_eq!(counted(&app), 10);

        // The world stops — a pause, a freeze, a full hitstop. `sim_dt` is the
        // condition itself rather than a menu state standing in for it.
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 0.0,
        });
        for _ in 0..600 {
            step(&mut app);
        }
        assert_eq!(
            counted(&app),
            10,
            "ten seconds of paused world came off the match clock"
        );

        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        for _ in 0..5 {
            step(&mut app);
        }
        assert_eq!(
            counted(&app),
            15,
            "the clock did not resume, so a match that was ever paused can never \
             time out"
        );
    }

    /// ⭐ THE WINNER CARD'S FOUR SECONDS ARE NOT MATCH TIME.
    #[test]
    fn a_settled_match_stops_counting() {
        let mut app = clock_world();
        for _ in 0..10 {
            step(&mut app);
        }
        let active = app.world().resource::<ActiveMatch>().clone();
        app.world_mut()
            .resource_mut::<StocksMatchSettled>()
            .settle(&active, ambition_combat::stocks::MatchVerdict::Draw);
        for _ in 0..120 {
            step(&mut app);
        }
        assert_eq!(
            counted(&app),
            10,
            "the results screen kept the match clock running"
        );
    }

    /// ⭐⭐ AND THE NEXT MATCH STARTS AT ZERO WITHOUT ANYBODY RETRACTING
    /// ANYTHING.
    ///
    /// The stamp is the retraction — the same property that makes the verdict
    /// and the sudden-death latch beside it safe. ⛔ a counter carried over
    /// would time the next match out on its first tick.
    #[test]
    fn a_new_match_starts_its_clock_at_zero() {
        let mut app = clock_world();
        for _ in 0..50 {
            step(&mut app);
        }
        assert_eq!(counted(&app), 50);

        // A different activation is a different match.
        app.insert_resource(match_activated_on(900));
        step(&mut app);
        assert_eq!(
            counted(&app),
            1,
            "the previous match's clock carried into this one"
        );
    }
}
