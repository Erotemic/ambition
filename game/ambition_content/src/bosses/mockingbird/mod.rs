//! The Mockingbird: the air chase (`docs/planning/game/bosses.md`, "The
//! Mockingbird").
//!
//! Its conductor (it flies the jet, fires its shots and drops its guard) is
//! the procedural module `ambition_content_modules::mockingbird`, a conducted
//! boss on the extension host like the T-rex. This is what the game keeps
//! here: its id, its birth, the burning sharks its sky is fought on, and a
//! view of the module's record for tests and inspectors.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

pub use ambition_content_modules::mockingbird::{Move, MOCKINGBIRD_ID};
/// The conducted module itself: its sockets and scale, for tests.
pub use ambition_content_modules::mockingbird as conductor_module;

/// The sheet its sky's sharks are drawn with: a moving platform's `visual`
/// and a room's `fall_rescue` name it.
pub const SHARK_SHEET: &str = "burning_flying_shark";

/// The state it is built with: the side it faces (its module chooses it from
/// then on) and its drawn row. See [`ambition_boss_encounter::BossBirthKit`].
pub fn birth(scope: &mut ambition_platformer2d_shared_tangle::construction::EntityScope, body: &ae::BodyKinematics) {
    scope.insert((
        ambition_boss_encounter::conduct::ConductedFacing::of_facing(body.facing),
        ambition_platformer2d::sprite_sheet::character::PinnedRow::default(),
    ));
}

/// The burning shark as a platform: its sheet loads with the rest of the art,
/// and it is drawn so the platform is its back. It loops its `idle` row, and
/// it faces the way it flies through the scrolling sky: away from the
/// Mockingbird. Its body box spans frame
/// pixels 9..194 and its back, behind the saddle, is at row 60
/// (`burning_flying_shark_spritesheet.ron`); the platform covers the back
/// from the tail fin to the head (x 40..176), so a body stands on it.
pub fn register_shark_platforms(app: &mut App) {
    use ambition_platformer2d_actor_monolith::assets::game_assets::{PropSheetSource, PropSheetsAppExt};
    use ambition_render::rendering::moving_platforms::{PlatformLook, PlatformLooksAppExt};
    app.register_prop_sheet(
        SHARK_SHEET,
        PropSheetSource::SpriteFolder {
            target: SHARK_SHEET.to_string(),
            tuning: ambition_platformer2d::sprite_sheet::character::SheetTuning::new(1.0, 0),
        },
    );
    app.register_platform_look(
        SHARK_SHEET,
        PlatformLook { row: "idle".into(), span_px: [40.0, 176.0], top_px: 60.0, faces_right: true },
    );
}

/// What its conductor is doing, read from its module's record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MockingbirdView {
    /// The move being performed and whether it is striking.
    pub performing: Option<(Move, bool)>,
    /// Winded after a dive: the punish window.
    pub stunned: bool,
    /// Its guard is down (diving, winded, flying home, passing low).
    pub open: bool,
    /// Screeching between two phases.
    pub screeching: bool,
    /// The room it measured.
    pub room: Option<Vec2>,
}

/// The conductor's record on `bird`, as a view. `None` before the module
/// stored anything (it stores nothing until it has measured its room).
pub fn conductor_of(world: &World, bird: Entity) -> Option<MockingbirdView> {
    use ambition_content_modules::mockingbird::Conductor;
    let records = world.get::<ambition_extension_host::BodyRecords>(bird)?;
    let c = Conductor::from_record(records.get(&Conductor::KEY)?).ok()?;
    Some(MockingbirdView {
        performing: c.performing(),
        stunned: c.is_stunned(),
        open: c.is_open(),
        screeching: c.is_screeching(),
        room: c.room().map(|room| Vec2::new(room.x, room.y)),
    })
}
