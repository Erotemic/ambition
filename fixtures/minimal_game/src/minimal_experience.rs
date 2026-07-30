//! The one experience this game registers: a floor, and a body that can walk on
//! it.
//!
//! Everything here is the answer to "what is the SMALLEST thing that is still a
//! game?" — and every line that turned out to be mandatory is a line the
//! campaign gets to look at, because a mandatory line in a minimal game is a
//! tax on every game.

use ambition::app::prelude::App;
use ambition::engine_core as ae;
use ambition::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition::runtime::PreparedPlatformerSource;
use ambition::world::rooms::RoomSpec;

use crate::{MINIMAL_EXPERIENCE, MINIMAL_GAMEPLAY_ROUTE};

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

fn prepared_session_world() -> PreparedPlatformerSource {
    let room = minimal_room();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    PreparedPlatformerSource::new(
        MINIMAL_EXPERIENCE,
        RoomSet::from_parts(MINIMAL_ROOM_ID, vec![room], Vec::new()),
        geometry,
        metadata,
        StartingCharacter::new(MINIMAL_CHARACTER_ID),
        LdtkRuntimeIndex::default(),
    )
}

pub fn register(app: &mut App) {
    PlatformerExperienceAuthoring::new(
        MINIMAL_EXPERIENCE,
        MINIMAL_GAMEPLAY_ROUTE,
        "Minimal Game",
        "Movement only — the smallest thing that is still a game",
        "Prepare the minimal game",
        AuthoredCatalogFragments::new(MINIMAL_CHARACTER_ID, MINIMAL_EXPERIENCE),
    )
    .install(app, prepared_session_world);
}
