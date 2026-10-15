//! The time of day: a world clock that turns through morning, day, evening
//! and night.
//!
//! [`DayClock`] is simulation state: it advances by the sim clock's seconds
//! ([`crate::WorldTime::sim_dt`]), so it stops while the world is paused and a
//! rewind takes it back with everything else. It is session-scoped, as
//! [`crate::SimTick`] is.
//!
//! [`DayCycle`] is how a game's day turns: its length and where its phases
//! begin. It is authored by each game (what a day is differs per game), and a
//! game that inserts none has no day: the clock stays where it is.
//!
//! What a phase changes (the music, the light) is decided by readers of the
//! clock, never here.

use bevy::prelude::*;

/// The part of the day it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DayPhase {
    Morning,
    Day,
    Evening,
    Night,
}

impl DayPhase {
    pub const ALL: [Self; 4] = [Self::Morning, Self::Day, Self::Evening, Self::Night];

    /// The name content uses for the phase (`"morning"`, `"day"`, ...).
    pub const fn key(self) -> &'static str {
        match self {
            Self::Morning => "morning",
            Self::Day => "day",
            Self::Evening => "evening",
            Self::Night => "night",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|phase| phase.key() == key)
    }
}

/// How a game's day turns. Hours are of a 24 hour day; each phase begins at
/// its hour and lasts until the next one begins, round the clock.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct DayCycle {
    /// Sim seconds in one whole day.
    pub day_length_s: f32,
    pub morning_at: f32,
    pub day_at: f32,
    pub evening_at: f32,
    pub night_at: f32,
    /// The hour a session's first day begins at.
    pub start_hour: f32,
}

impl DayCycle {
    /// The phase at `hour` (0 to 24).
    pub fn phase_at(&self, hour: f32) -> DayPhase {
        let starts = [
            (self.morning_at, DayPhase::Morning),
            (self.day_at, DayPhase::Day),
            (self.evening_at, DayPhase::Evening),
            (self.night_at, DayPhase::Night),
        ];
        // The phase whose start is the latest one not after `hour`; before the
        // first start of the day it is still the last phase of the day before.
        starts
            .iter()
            .filter(|(at, _)| *at <= hour)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .or_else(|| starts.iter().max_by(|a, b| a.0.total_cmp(&b.0)))
            .map(|(_, phase)| *phase)
            .unwrap_or(DayPhase::Day)
    }
}

/// The time of day now. `day` 0 is a clock that has not begun: the first
/// step of a session with a [`DayCycle`] begins day 1 at its start hour.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct DayClock {
    /// The day it is, from 1.
    pub day: u32,
    /// Sim seconds since the day began (midnight), under the cycle's length.
    pub seconds: f32,
}

impl DayClock {
    /// The clock at `hour` of day 1 of `cycle`.
    pub fn at_hour(cycle: &DayCycle, hour: f32) -> Self {
        Self { day: 1, seconds: (hour.rem_euclid(24.0) / 24.0) * cycle.day_length_s }
    }

    /// The hour (0 to 24) under `cycle`.
    pub fn hour(&self, cycle: &DayCycle) -> f32 {
        if cycle.day_length_s <= 0.0 {
            return cycle.start_hour;
        }
        self.seconds / cycle.day_length_s * 24.0
    }

    /// The phase under `cycle`.
    pub fn phase(&self, cycle: &DayCycle) -> DayPhase {
        cycle.phase_at(self.hour(cycle))
    }
}

/// Turn the clock by this step's sim seconds. No [`DayCycle`]: no day.
pub fn advance_day_clock(
    cycle: Option<Res<DayCycle>>,
    time: Res<crate::WorldTime>,
    mut clock: ResMut<DayClock>,
) {
    let Some(cycle) = cycle else { return };
    if cycle.day_length_s <= 0.0 {
        return;
    }
    if clock.day == 0 {
        *clock = DayClock::at_hour(&cycle, cycle.start_hour);
    }
    clock.seconds += time.sim_dt().max(0.0);
    while clock.seconds >= cycle.day_length_s {
        clock.seconds -= cycle.day_length_s;
        clock.day = clock.day.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cycle() -> DayCycle {
        DayCycle { day_length_s: 240.0, morning_at: 5.0, day_at: 10.0, evening_at: 17.0, night_at: 21.0, start_hour: 8.0 }
    }

    #[test]
    fn each_phase_lasts_from_its_hour_to_the_next_round_the_clock() {
        let c = cycle();
        for (hour, phase) in [
            (0.0, DayPhase::Night),
            (4.9, DayPhase::Night),
            (5.0, DayPhase::Morning),
            (9.9, DayPhase::Morning),
            (10.0, DayPhase::Day),
            (16.9, DayPhase::Day),
            (17.0, DayPhase::Evening),
            (21.0, DayPhase::Night),
            (23.9, DayPhase::Night),
        ] {
            assert_eq!(c.phase_at(hour), phase, "at {hour}");
        }
    }

    #[test]
    fn the_clock_begins_at_the_start_hour_and_wraps_into_the_next_day() {
        let c = cycle();
        let mut app = App::new();
        app.insert_resource(c);
        app.init_resource::<DayClock>();
        app.insert_resource(crate::WorldTime::new(1.0, 1.0));
        app.add_systems(Update, advance_day_clock);
        app.update();
        let clock = *app.world().resource::<DayClock>();
        assert_eq!(clock.day, 1);
        // 8:00 is 80 s of a 240 s day; one second has passed.
        assert!((clock.hour(&c) - (81.0 / 240.0 * 24.0)).abs() < 1e-4);
        app.insert_resource(crate::WorldTime::new(1.0, 200.0));
        app.update();
        let clock = *app.world().resource::<DayClock>();
        assert_eq!(clock.day, 2, "81 + 200 s passes midnight of a 240 s day");
        assert!((clock.seconds - 41.0).abs() < 1e-3);
    }

    #[test]
    fn a_paused_world_keeps_its_hour() {
        let c = cycle();
        let mut app = App::new();
        app.insert_resource(c);
        app.insert_resource(DayClock::at_hour(&c, 12.0));
        app.insert_resource(crate::WorldTime::new(1.0, 0.0));
        app.add_systems(Update, advance_day_clock);
        app.update();
        assert_eq!(app.world().resource::<DayClock>().hour(&c), 12.0);
    }
}
