//! The one experience this game registers: a floor, and a body that can walk on
//! it.
//!
//! Everything here is the answer to "what is the SMALLEST thing that is still a
//! game?" — and every line that turned out to be mandatory is a line the
//! campaign gets to look at, because a mandatory line in a minimal game is a
//! tax on every game.

// ONE import for room authoring. This used to be `ambition_platformer2d::engine_core as ae`
// plus `ambition_platformer2d::world::rooms::RoomSpec` — two modules, one of them an
// implementation crate named `engine_core`, to place a floor.
use ambition_platformer2d::world::prelude::*;

use crate::MINIMAL_EXPERIENCE;

/// The one body in the game.
pub const MINIMAL_CHARACTER_ID: &str = "minimal_walker";
pub const MINIMAL_ROOM_ID: &str = "minimal_room";

/// One character, no combat, no art.
///
/// ⚠ **`playable_kit` is a required field whose two values mean opposite
/// things, and the wrong one ARMS YOUR PEACEFUL CHARACTER.**
///
/// `HostCode` means "give this body the host protagonist's kit", so the engine
/// calls `default_player_action_set` and the authored `melee: None` is ignored
/// entirely. `Authored` means "my action set is the authority".
///
/// This game shipped `HostCode` for a day because it was copied out of
/// Outlander — which is a protagonist and wants it — and blind run 2 recorded
/// doing exactly the same copy for exactly the same reason. Nothing documents
/// the difference. The consumer-matrix test
/// `a_noncombat_character_gets_no_combat_state` is what caught it, by asking
/// the live body rather than the RON what it could do. The sheet
/// named here does not exist; the engine degrades to a placeholder body, which
/// is the correct behaviour for a game that ships no art and is also exactly
/// the "declared image indistinguishable from an unskinned bolt" hazard the
/// repo has been bitten by. Recorded rather than worked around — a required
/// field with no meaningful value is API pressure, and slice C's evidence
/// should see it.
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
            playable_kit: Authored,
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
        // ⚠ `Block::solid(name, MIN, size)` — a MIN CORNER, not a centre.
        //
        // This passed a centre until 2026-07-30, so the floor sat at x 320..960
        // in a 640-wide room while the walker spawned at x=64. The walker fell
        // straight past it, blast-died, respawned and fell again, forever —
        // and `host_status` reported `Running { prepared: true }` the whole
        // time, because the host WAS running. It was the game that was broken.
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
// DELETED 2026-07-30. `ModuleDraft::no_audio()` is the word. That was this
// game's LAST hand-registration; it now declares itself entirely through the
// draft and installs no plugin of its own.
