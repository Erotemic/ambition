//! Ambition's day, as its pack states it (`day` in `data/ambition_rules.ron`):
//! how long it is, where its phases begin, and how its light looks under the
//! sky. The town of the Math dimension is under the sky; its gate hall is
//! under the ground and keeps its lamps.

use ambition_content_schemas::rules::Day;
use ambition_render::rendering::daylight::DaylightStyle;
use ambition_sprite_sheet::game_assets::ParallaxTheme;
use ambition_time::DayCycle;
use bevy::prelude::*;

/// The clock of the pack's `day`.
pub fn day_cycle(day: &Day) -> DayCycle {
    DayCycle {
        day_length_s: day.length_seconds,
        morning_at: day.morning_at,
        day_at: day.day_at,
        evening_at: day.evening_at,
        night_at: day.night_at,
        start_hour: day.start_hour,
    }
}

/// The light of the pack's `day` over a sunlit room: the veil's colour at each
/// hour. The town of the Math dimension is the sunlit theme.
pub fn daylight(day: &Day) -> DaylightStyle {
    DaylightStyle {
        keys: day.daylight.iter().map(|(hour, (r, g, b, a))| (*hour, Color::srgba(*r, *g, *b, *a))).collect(),
        sunlit: vec![ParallaxTheme::MathTown],
    }
}

pub struct AmbitionDayPlugin;

impl Plugin for AmbitionDayPlugin {
    fn build(&self, app: &mut App) {
        let day = app.world().resource::<crate::tuning::AmbitionTuning>().0.day.clone();
        app.insert_resource(day_cycle(&day));
        app.insert_resource(daylight(&day));
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
        let d = day_cycle(&crate::tuning::shipped().day);
        assert!(d.morning_at < d.day_at && d.day_at < d.evening_at && d.evening_at < d.night_at);
        assert_eq!(d.phase_at(d.start_hour), ambition_time::DayPhase::Morning);
    }

    /// The day the game plays is the pack's `day`: an edit to the file is the
    /// next start's clock and light, with no Rust build.
    #[test]
    fn the_day_is_the_pack_files() {
        use ambition_content_schemas::rules::lowered_ambition_rules;
        let mut app = App::new();
        app.insert_resource(crate::tuning::AmbitionTuning(crate::tuning::shipped()));
        app.add_plugins(AmbitionDayPlugin);
        let shipped = crate::tuning::shipped().day;
        assert_eq!(*app.world().resource::<DayCycle>(), day_cycle(&shipped));
        assert_eq!(app.world().resource::<DayCycle>().day_length_s, shipped.length_seconds);
        assert_eq!(app.world().resource::<DaylightStyle>().keys.len(), shipped.daylight.len());

        const START: &str = "start_hour: 8.0,";
        let edited = crate::pack::compile_pack_with(|path, text| {
            if path != "data/ambition_rules.ron" {
                return text;
            }
            assert_eq!(text.matches(START).count(), 1, "the probe edits the start hour");
            text.replace(START, "start_hour: 6.5,")
        })
        .expect("an edited day compiles");
        let mut app = App::new();
        app.insert_resource(crate::tuning::AmbitionTuning(lowered_ambition_rules(&edited).expect("the edited pack lowers")));
        app.add_plugins(AmbitionDayPlugin);
        assert_eq!(app.world().resource::<DayCycle>().start_hour, 6.5, "the game does not start on the file's hour");
    }
}
