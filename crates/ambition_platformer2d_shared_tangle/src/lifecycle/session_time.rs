//! The clock of the live session root.

use crate::schedule::SimScheduleExt;
use ambition_time::GameplayElapsed;
use bevy::prelude::*;

/// The session clock is current before a brain reads it.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GameplayElapsedAdvanced;

/// Install the session clock and advance it on the simulation schedule.
pub struct SessionElapsedPlugin;

impl Plugin for SessionElapsedPlugin {
    fn build(&self, app: &mut App) {
        super::require_on_session_root::<GameplayElapsed>(app);
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            advance_gameplay_elapsed
                .in_set(GameplayElapsedAdvanced)
                .in_set(crate::schedule::Platformer2dSimulationPhase::WorldPrep),
        );
    }
}

/// Advance [`GameplayElapsed`] by the scaled gameplay dt each frame. Runs at the
/// head of `WorldPrep`, before any actor brain reads the snapshot.
fn advance_gameplay_elapsed(
    mut elapsed: super::SessionWorldMut<GameplayElapsed>,
    world_time: bevy::prelude::Res<ambition_time::WorldTime>,
) {
    elapsed.0 += world_time.sim_dt();
}

#[cfg(test)]
mod sim_clock_tests {
    use super::{GameplayElapsed, advance_gameplay_elapsed};
    use crate::lifecycle::{
        SessionRoot, SessionScopeId, require_on_session_root, session_world_component,
    };
    use bevy::prelude::*;

    fn clock(app: &App) -> f32 {
        session_world_component::<GameplayElapsed>(app.world())
            .expect("the session root carries the clock")
            .0
    }

    #[test]
    fn gameplay_clock_accumulates_scaled_dt() {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 1.0 / 60.0));
        require_on_session_root::<GameplayElapsed>(&mut app);
        app.world_mut().spawn(SessionRoot(SessionScopeId(1)));
        app.add_systems(Update, advance_gameplay_elapsed);

        app.update();
        app.update();
        app.update();
        let elapsed = clock(&app);
        assert!(
            (elapsed - 3.0 / 60.0).abs() < 1e-6,
            "three ticks at 1/60 s must accumulate 3/60 s; got {elapsed}"
        );

        // Paused (sim_dt == 0) the clock freezes — reaction latency, hitstun,
        // and every other sim timer that reads it stop together.
        app.insert_resource(ambition_time::WorldTime::new(1.0 / 60.0, 0.0));
        app.update();
        let after_pause = clock(&app);
        assert_eq!(
            elapsed, after_pause,
            "a paused frame must not advance sim-time"
        );
    }
}
