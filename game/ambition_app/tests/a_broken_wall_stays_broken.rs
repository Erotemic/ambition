//! A secret wall, as in Hollow Knight: the player hits a cracked wall until it
//! breaks, walks through to the room behind it, and the wall stays broken for
//! good in that save (`HazardRespawn::Persistent`, a `Spent` row in the
//! occurrence ledger). Leaving and coming back finds it broken, and so does a
//! fresh session loaded from the save file.
//!
//! The rooms are the sandbox's test bed
//! (`tools/ambition_ldtk_tools/specs/breakable_wall_lab/make_specs.py`): a
//! corridor 160 high shut by the wall, and the room behind it.

use ambition_platformer2d::breakables::BreakableFeature;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::persistence::save::AmbitionGameSave;
use ambition_platformer2d::persistence::save_data::AmbitionGameSaveData;
use bevy::prelude::App;

use crate::common::walk::{Keys, Press, Walker};
use crate::the_pirate_port::{a_session_in, go, go_to};

const LAB: &str = "breakable_wall_lab";
const SECRET: &str = "breakable_wall_lab_secret";
const WALL: &str = "breakable_wall_lab_wall";
/// West of the wall's face (x 640), on the corridor's floor (y 224).
const BEFORE_THE_WALL: ae::Vec2 = ae::Vec2::new(600.0, 200.0);

/// Whether the wall is broken, or `None` when no live room builds it.
fn the_wall(app: &mut App) -> Option<bool> {
    let world = app.world_mut();
    let mut walls = world.query::<(&FeatureId, &BreakableFeature)>();
    walls.iter(world).find(|(id, _)| id.as_str() == WALL).map(|(_, wall)| wall.broken())
}

/// Walk by keys to the wall and hit it, facing it, until it breaks.
fn break_the_wall(app: &mut App) {
    go_to(app, BEFORE_THE_WALL, "the cracked wall");
    let mut swings = 0;
    while the_wall(app) == Some(false) {
        assert!(swings < 12, "the wall did not break after {swings} swings");
        let mut keys = Keys::new(app);
        // Face the wall: a push into it does not move the body.
        keys.step(Press { axis: 1.0, ..Press::default() });
        keys.step(Press { attack: true, ..Press::default() });
        for _ in 0..24 {
            keys.step(Press::default());
        }
        drop(keys);
        swings += 1;
    }
    assert!(swings >= 2, "control: the wall takes more than one hit ({swings})");
}

/// The wall broken by hits stays broken: the player walks through where it
/// stood to the room behind it, and back, and it is still broken.
#[test]
fn a_wall_broken_by_hits_lets_the_player_through_and_stays_broken() {
    let mut app = a_session_in(LAB);
    assert_eq!(the_wall(&mut app), Some(false), "control: the wall stands");
    break_the_wall(&mut app);
    go(&mut app, SECRET, Some("lab_east"));
    go(&mut app, LAB, Some("secret_west"));
    assert_eq!(the_wall(&mut app), Some(true), "the wall was whole again when the player came back");
}

/// The save file of a session in the lab, read back as a fresh process reads
/// it; the wall broken first when `broken`.
fn the_save_file_written_after(broken: bool) -> AmbitionGameSaveData {
    use ambition_platformer2d::persistence::save::{load_save, write_save};
    let mut app = a_session_in(LAB);
    if broken {
        break_the_wall(&mut app);
    }
    // The durable horizon is mirrored into the save by the simulation.
    for _ in 0..4 {
        app.update();
    }
    let data = app.world().resource::<AmbitionGameSave>().0.clone();
    let path = std::env::temp_dir().join(format!(
        "ambition_wall_save_{}_{}_{broken}.ron",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_nanos())
    ));
    write_save(&path, &data).expect("the save file is written");
    let loaded = load_save(&path);
    let _ = std::fs::remove_file(&path);
    assert!(loaded.present && loaded.writable, "precondition: the file reads back as a present, writable save");
    loaded.data
}

/// The wall after a load of `file` in a fresh session in the lab.
fn the_wall_after_a_load(file: &AmbitionGameSaveData) -> Option<bool> {
    let mut app = a_session_in(LAB);
    app.world_mut().resource_mut::<AmbitionGameSave>().0 = file.clone();
    app.world_mut().resource_mut::<ambition_platformer2d::platformer::lifecycle::SaveRestored>().0 = false;
    for _ in 0..90 {
        app.update();
    }
    assert!(
        app.world().resource::<ambition_platformer2d::platformer::lifecycle::SaveRestored>().0,
        "precondition: the load landed"
    );
    the_wall(&mut app)
}

/// Save, quit, load: a fresh session loaded from a file written after the
/// wall broke builds it broken. The control is a file written with the wall
/// whole, which builds it whole.
#[test]
fn a_wall_broken_in_play_is_broken_after_its_save_file_is_loaded() {
    assert_eq!(
        (
            the_wall_after_a_load(&the_save_file_written_after(false)),
            the_wall_after_a_load(&the_save_file_written_after(true)),
        ),
        (Some(false), Some(true)),
        "(a file written with the wall whole, one written after it broke): the wall after the load"
    );
}
