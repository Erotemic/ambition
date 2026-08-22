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

/// One character, no combat, no art.
///
/// The sheet named here does not exist; the engine degrades to a placeholder
/// body, which is the correct behaviour for a game that ships no art.
///
/// The sibling fixture `external_consumer` hit the identical failure and was fixed alone, which is
/// how this one kept it.
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
        // `Block::solid(name, MIN, size)` — a MIN CORNER, not a centre.
        //
        // Found by blind run 3, which copied this fixture verbatim because
        // `docs/sdk/README.md` says to, and spent its longest debugging episode
        // on a bug it had inherited from the reference.
        vec![Block::solid(
            "floor",
            Vec2::new(0.0, floor_top),
            Vec2::new(size.x, 40.0),
        )],
    );
    RoomSpec::new(MINIMAL_ROOM_ID, world)
}

// `register()` stood here. It hand-registered an empty `AudioCatalogFragment`
// because preparation validation refuses an experience that declares no audio —
// mandatory paperwork with no word for it on the public surface.
//
// `ModuleDraft::no_audio()` is the word. That was this game's LAST hand-registration; it now
// declares itself entirely through the draft and installs no plugin of its own.
