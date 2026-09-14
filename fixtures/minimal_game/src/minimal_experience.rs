//! The one experience this game registers: a floor, and a body that can walk on
//! it.
//!
//! Keep this experience minimal: every required declaration is part of the
//! baseline cost paid by all games.

use ambition_platformer2d::world::prelude::*;

use crate::MINIMAL_EXPERIENCE;

/// The one body in the game.
pub const MINIMAL_CHARACTER_ID: &str = "minimal_walker";
pub const MINIMAL_ROOM_ID: &str = "minimal_room";

/// One character, no combat, no art. The missing sheet intentionally exercises
/// the engine's placeholder-body fallback.
pub const MINIMAL_ROSTER_RON: &str = r#"(
    brain_presets: { "still": StandStill },
    action_set_presets: {
        "walk_only": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "minimal_walker": (
            display_name: "Walker",
            spritesheet: "minimal_walker.png",
            manifest: "minimal_walker_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "still",
            default_action_set: "walk_only",
            tags: ["player"],
        ),
    },
)"#;

/// One rectangular room with a floor. Nothing to fight, nowhere to go.
pub fn minimal_room() -> RoomSpec {
    let size = Vec2::new(640.0, 360.0);
    let floor_top = 320.0;
    let world = AuthoredWorld::new(
        "Minimal Room",
        size,
        Vec2::new(64.0, floor_top - 64.0),
        // `Block::solid` takes a minimum corner and size, not a centre.
        vec![Block::solid(
            "floor",
            Vec2::new(0.0, floor_top),
            Vec2::new(size.x, 40.0),
        )],
    );
    RoomSpec::new(MINIMAL_ROOM_ID, world)
}
