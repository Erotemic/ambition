//! A one-room composition a harness-only consumer supplies (CB4). Its tests
//! step it and observe it through the harness; the closure check in
//! `scripts/tests/test_harness_alone_has_no_renderer.py` reads what it links.

use ambition_platformer2d::sim::ControlFrame;
use ambition_platformer2d::world::{
    prelude::{AuthoredWorld, Vec2},
    rooms::{RoomSet, RoomSpec},
};
use ambition_sim_harness::Platformer2dSimHarnessOptions;

/// The id of the one room this consumer composes.
pub const ROOM: &str = "harness_alone_room";

/// Seed one empty room as the session's room set and register the control
/// frame the harness writes. Only reusable engine APIs; no content crate.
pub fn compose(app: &mut bevy::prelude::App, _options: &Platformer2dSimHarnessOptions) -> Result<(), String> {
    let world = AuthoredWorld::new(ROOM, Vec2::new(800.0, 600.0), Vec2::new(100.0, 100.0), Vec::new());
    let set = RoomSet::from_parts_or_panic(ROOM, vec![RoomSpec::new(ROOM, world)], Vec::new());
    ambition_platformer2d::world::rooms::insert_room_set(app.world_mut(), set);
    app.insert_resource(ControlFrame::default());
    Ok(())
}
