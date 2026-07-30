//! The one experience this game registers: a floor, and a body that can walk on
//! it.
//!
//! Everything here is the answer to "what is the SMALLEST thing that is still a
//! game?" — and every line that turned out to be mandatory is a line the
//! campaign gets to look at, because a mandatory line in a minimal game is a
//! tax on every game.

use ambition::app::prelude::App;
use ambition::engine_core as ae;
use ambition::world::rooms::RoomSpec;

use crate::MINIMAL_EXPERIENCE;

/// The one body in the game.
pub const MINIMAL_CHARACTER_ID: &str = "minimal_walker";
pub const MINIMAL_ROOM_ID: &str = "minimal_room";

/// One character, no combat, no art.
///
/// ⚠ **`playable_kit: HostCode` and a `spritesheet` are both required fields**,
/// and neither is a thing a movement-only game has an opinion about. The sheet
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
            playable_kit: HostCode,
            tags: ["player"],
        ),
    },
)"#;

/// One rectangular room with a floor. Nothing to fight, nowhere to go.
pub fn minimal_room() -> RoomSpec {
    let size = ae::Vec2::new(640.0, 360.0);
    let floor_top = 320.0;
    let world = ae::World::new(
        "Minimal Room",
        size,
        ae::Vec2::new(64.0, floor_top - 64.0),
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(size.x * 0.5, floor_top + 20.0),
            ae::Vec2::new(size.x, 40.0),
        )],
    );
    RoomSpec::new(MINIMAL_ROOM_ID, world)
}

pub fn register(app: &mut App) {
    // ⚠ DELIBERATE SILENCE, DECLARED — and a movement-only game has to declare
    // it too. Preparation validation refuses an experience whose provider
    // registered no explicit audio fragment.
    //
    // Found the way the campaign is supposed to find things: the host sat in
    // `HostStatus::Activating` for 600 ticks and never started. Outlander's own
    // comment already knew — "a good message that a headless host surfaced
    // NOWHERE" — so a KNOWN error-quality gap was sitting there and a second
    // consumer walked straight into it. `HostStatus` names the stuck state now;
    // the REASON is still swallowed, which is slice-C material.
    //
    // This is the LAST thing this game registers by hand. Everything else moved
    // onto `ModuleDraft::playable`.
    use ambition::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
    app.register_audio_catalog_fragment(
        AudioCatalogFragment::new(MINIMAL_EXPERIENCE, None, None)
            .expect("the silent minimal-game audio fragment is valid"),
    );
}
