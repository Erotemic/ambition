//! The one experience this game registers: a floor, and a body that can walk on
//! it.
//!
//! Keep this experience minimal: every required declaration is part of the
//! baseline cost paid by all games.

use ambition_platformer2d::world::prelude::*;

/// The one body in the game.
pub const CHARACTER_ID: &str = "headless_walker";
pub const ROOM_ID: &str = "headless_room";

/// One character, no combat, no art.
///
/// The sheet named here does not exist; the engine degrades to a placeholder
/// body, which is the correct behaviour for a game that ships no art.
///
/// The sibling fixture `external_consumer` hit the identical failure and was fixed alone, which is
/// how this one kept it.
pub const ROSTER_RON: &str = r#"(
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
        "headless_walker": (
            display_name: "Walker",
            spritesheet: "headless_walker.png",
            manifest: "headless_walker_spritesheet.ron",
            tier: MainHall,
            body_kind: Standard,
            composition: None,
            default_brain: "still",
            default_action_set: "walk_only",
            tags: ["player"],
        ),
    },
)"#;

/// One room with a RAISED floor, and the raise is what makes the floor testable.
///
/// ⛔⛔ **A FLOOR AT THE ROOM'S OWN BOTTOM CANNOT BE TOLD FROM ANY OTHER STOP.**
/// The reference fixture this was copied from puts its floor block at the bottom
/// of a 640x360 room, where a body comes to rest at y=296 — which is also
/// `room_height - body_height`. A test written against that geometry can assert
/// the body moved and then stopped, and BOTH arms survive deleting the floor:
/// measured 2026-09-09, a body with the block moved 100,000px away fell to
/// y=583 and settled there, satisfying "it moved" and "it stopped" while
/// touching nothing the room authored.
///
/// ⇒ Raised clear, the resting height NAMES the surface. 176 is the floor;
/// anything past 200 is a body that fell through it.
pub fn room() -> RoomSpec {
    let size = Vec2::new(640.0, 360.0);
    let world = AuthoredWorld::new(
        "Headless Room",
        size,
        // Above the floor, so the body has to fall to reach it.
        Vec2::new(64.0, FLOOR_TOP - 80.0),
        // `Block::solid(name, MIN, size)` — a MIN CORNER, not a centre.
        vec![Block::solid(
            "floor",
            Vec2::new(0.0, FLOOR_TOP),
            Vec2::new(size.x, 40.0),
        )],
    );
    RoomSpec::new(ROOM_ID, world)
}

/// The top of [`room`]'s floor. A body at rest sits its own half-height above
/// this; a body past it fell through. Shared with the test so the geometry and
/// the assertion cannot disagree.
pub const FLOOR_TOP: f32 = 200.0;
