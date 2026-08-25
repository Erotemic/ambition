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

use super::PreparedMatch;
use super::seating::{ActiveMatch, MatchInstance};
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
    ticks: u64,
}

impl LiveMatchTicks {
    /// How long `active` has been fought. Zero for a match this is not counting,
    /// which is the honest answer for one that has not started.
    pub fn of(&self, active: &ActiveMatch) -> u64 {
        if self.of == Some(active.instance()) {
            self.ticks
        } else {
            0
        }
    }

    /// Rebuild from a rollback snapshot. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn from_snapshot(of: Option<MatchInstance>, ticks: u64) -> Self {
        Self { of, ticks }
    }

    /// The snapshot's view of this counter. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn parts(&self) -> (Option<MatchInstance>, u64) {
        (self.of.clone(), self.ticks)
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
        live.ticks = 0;
    }

    if settled.is_some_and(|settled| settled.settled(&active)) {
        return;
    }
    if time.is_none_or(|time| time.sim_dt() <= 0.0) {
        return;
    }
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

    live.ticks += 1;
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
            .settle(&active);
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
