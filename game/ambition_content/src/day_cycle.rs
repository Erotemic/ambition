//! Ambition's day: how long it is, where its phases begin, and how its light
//! looks under the sky.
//!
//! A day is 24 minutes of play (an hour of the day a minute), and a session
//! begins at 8 in the morning. The town of the Math dimension is under the
//! sky; its gate hall is under the ground and keeps its lamps. The music of a
//! room by the time of day is authored in `music_cues.ron`
//! (`room_phase_tracks`).

use ambition_render::rendering::daylight::DaylightStyle;
use ambition_sprite_sheet::game_assets::ParallaxTheme;
use ambition_time::DayCycle;
use bevy::prelude::*;

/// Ambition's day.
pub const AMBITION_DAY: DayCycle = DayCycle {
    day_length_s: 24.0 * 60.0,
    morning_at: 5.0,
    day_at: 10.0,
    evening_at: 17.0,
    night_at: 21.0,
    start_hour: 8.0,
};

/// The light of Ambition's day over a sunlit room: the veil's colour at each
/// hour. Clear from 10 to 16; gold at 7:30, violet at dawn and dusk, deep
/// blue at night.
pub fn ambition_daylight() -> DaylightStyle {
    DaylightStyle {
        // Alphas are of a veil blended in linear light, which darkens much
        // less than its alpha reads in sRGB: 0.86 takes a white wall to about
        // a third of its brightness (measured in a capture at 22:00).
        keys: vec![
            (0.0, Color::srgba(0.05, 0.08, 0.26, 0.86)),
            (4.5, Color::srgba(0.05, 0.08, 0.26, 0.86)),
            (6.0, Color::srgba(0.32, 0.26, 0.52, 0.55)),
            (7.5, Color::srgba(1.0, 0.72, 0.42, 0.20)),
            (10.0, Color::srgba(1.0, 1.0, 1.0, 0.0)),
            (16.0, Color::srgba(1.0, 1.0, 1.0, 0.0)),
            (18.0, Color::srgba(1.0, 0.48, 0.12, 0.32)),
            (19.5, Color::srgba(0.45, 0.20, 0.42, 0.58)),
            (21.0, Color::srgba(0.06, 0.09, 0.28, 0.82)),
        ],
        sunlit: vec![ParallaxTheme::MathTown],
    }
}

pub struct AmbitionDayPlugin;

impl Plugin for AmbitionDayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AMBITION_DAY);
        app.insert_resource(ambition_daylight());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The music file names phases by the keys the audio crate accepts; those
    /// are the clock's own.
    #[test]
    fn the_phase_keys_the_music_accepts_are_the_clocks() {
        let clock: Vec<&str> = ambition_time::DayPhase::ALL.iter().map(|phase| phase.key()).collect();
        assert_eq!(clock, ambition_audio::content_schema::DAY_PHASE_KEYS.to_vec());
    }

    /// Each phase of Ambition's day lasts a while, in its order round the clock.
    #[test]
    fn the_phases_begin_in_order_and_the_session_begins_in_the_morning() {
        let d = AMBITION_DAY;
        assert!(d.morning_at < d.day_at && d.day_at < d.evening_at && d.evening_at < d.night_at);
        assert_eq!(d.phase_at(d.start_hour), ambition_time::DayPhase::Morning);
    }
}
