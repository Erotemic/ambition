#![cfg(feature = "rl_sim")]
//! The map records where the player has been — in the shipped composition.
//!
//! ⛔⛔ MEASURED (`dev/installer_call_coverage.json`): deleting
//! `ambition_menu::map::install_map_simulation_systems(app, sim)` from
//! `progression_schedule.rs:98` left ALL 583 `app_it` tests green. The map's
//! simulation half is two systems — `track_room_visits` stamps `MapMenuState` AND
//! writes a `room_visited_<id>` save flag, `sync_map_from_save` hydrates that state
//! back out of the save on load — and nothing at app level noticed the composition
//! dropping both.
//!
//! ⚠ THIS IS THE SIMULATION HALF ONLY. The map's INPUT and VIEW systems are a
//! separate installer with a separate contract (that one needs a windowed host;
//! this one deliberately does not), and nothing here says anything about drawing a
//! map.
//!
//! ⭐ THE ASSERTION IS THE PAIR, not either side. `track_room_visits` writes the
//! live state and the durable flag in ONE statement, so a visited room whose flag
//! is missing is not a stale flag — it is that system not having run.

use crate::common::{base, fixed_60hz_sim};

use ambition_platformer2d::menu::map::{room_visited_flag, MapMenuState};
use ambition_platformer2d::persistence::save::AmbitionGameSave;

#[test]
fn every_room_the_map_calls_visited_has_its_visit_on_the_save() {
    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);

    // ⚠ `get_resource`, not `resource`: a MISSING `MapMenuState` and an EMPTY one
    // are different failures, and a panic on the former would report the latter.
    let visited: Vec<String> = sim
        .world()
        .get_resource::<MapMenuState>()
        .expect(
            "this composition holds no `MapMenuState` at all — that is the map \
             plugin being absent, not its simulation half being uninstalled",
        )
        .visited
        .iter()
        .cloned()
        .collect();
    // ⚠ ANTI-VACUITY: an empty visited set satisfies "every visited room is on the
    // save" for free, and an empty set is exactly what the missing installer
    // produces — so this is the assertion that actually witnesses the deletion.
    assert!(
        !visited.is_empty(),
        "the map records NO visited room after 120 frames of play. \
         `track_room_visits` is what stamps one, and \
         `install_map_simulation_systems` is what runs it — deleting that call \
         from `progression_schedule.rs` leaves every other app test green."
    );

    let save = sim.world().resource::<AmbitionGameSave>().data().clone();
    for room in &visited {
        let flag = room_visited_flag(room);
        assert!(
            save.flag(&flag),
            "the map calls `{room}` visited and the save carries no `{flag}`. \
             Both are written by `track_room_visits` in the same statement, so \
             they cannot disagree unless something wrote one of them by hand."
        );
    }
}
