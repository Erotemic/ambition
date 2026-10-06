//! The Tyrant King: a T-rex, and a pattern fight
//! (`docs/planning/game/bosses.md`, "The Tyrant King").
//!
//! His conductor — it walks him and performs his moves — is the procedural
//! module `ambition_content_modules::trex`, a conducted boss on the extension
//! host (`ambition.boss.conduct`), like the Flying Spaghetti Monster. This is
//! what the game keeps here: his id, his birth, and a view of the module's
//! record for tests and inspectors.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::bosses::hall::Hall;

pub use ambition_content_modules::trex::{Move, TREX_ID};
/// The conducted module itself: its measured volumes, for tests that ask
/// where a move should land.
pub use ambition_content_modules::trex as conductor_module;

/// The state he is built with: the side he faces (the side the body was built
/// facing; his module chooses it from then on) and his drawn row. See
/// [`ambition_boss_encounter::BossBirthKit`].
///
/// And his BODY RIG (`trex_enemy_body_rig.ron`, published from the rig that
/// draws him): the parts you can hit him through, head, jaw, neck, torso,
/// tail and legs, posed every tick from the row his module pins, so they are
/// where his art is. His sheet's frame centre is his position (the boss
/// placement law), so the rig's origin, his feet pixel, is offset from it.
pub fn birth(scope: &mut ambition_platformer2d_shared_tangle::construction::EntityScope, body: &ae::BodyKinematics) {
    scope.insert((
        ambition_boss_encounter::conduct::ConductedFacing::of_facing(body.facing),
        ambition_platformer2d::sprite_sheet::character::PinnedRow::default(),
    ));
    let (rig, feet) = body_rig();
    scope.insert((
        ambition_combat::body_rig::BodyRig(std::sync::Arc::new(rig)),
        ambition_combat::body_rig::BodyRigPose::default(),
        ambition_combat::body_rig::RigFeetOffset(feet),
        ambition_combat::hurtbox_resolution::ResolvedHurtboxes::default(),
    ));
}

/// The sheet his body rig is published beside.
pub const SHEET: &str = "trex_enemy";

/// His prepared body rig in world units, and where its origin (his feet) is
/// from his position.
///
/// # Panics
///
/// When the sheet or its rig is not published, or does not parse: a T-rex
/// with no hurt parts would be hit through nothing, or fall back to a box the
/// rework exists to replace, and say nothing.
pub fn body_rig() -> (ambition_characters::actor::body_rig::PreparedBodyRig, Vec2) {
    let px = conductor_module::PX;
    let text = ambition_sprite_sheet::baked_body_rigs::baked_body_rig(SHEET)
        .unwrap_or_else(|| panic!("`{SHEET}_body_rig.ron` is not published: regenerate the T-rex sheet"));
    let rig = ambition_characters::actor::BodyRigDefinition::from_published_ron(text)
        .unwrap_or_else(|error| panic!("`{SHEET}_body_rig.ron`: {error}"))
        .scaled(px)
        .prepare()
        .unwrap_or_else(|error| panic!("`{SHEET}_body_rig.ron` does not prepare: {error:?}"));
    let record = ambition_sprite_sheet::character::sheets::record_for_sheet_key(SHEET)
        .unwrap_or_else(|| panic!("`{SHEET}` has no sheet record"));
    let feet = record
        .body_metrics
        .as_ref()
        .and_then(|metrics| metrics.feet_pixel)
        .unwrap_or_else(|| panic!("`{SHEET}` publishes no feet pixel"));
    let centre = Vec2::new(record.frame_width as f32, record.frame_height as f32) * 0.5;
    (rig, (Vec2::new(feet.x as f32, feet.y as f32) - centre) * px)
}

/// What his conductor is doing, read from its module's record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrexView {
    /// The move being performed and whether it is striking.
    pub performing: Option<(Move, bool)>,
    /// Running the hall after a charge, before the wall.
    pub charging: bool,
    /// Lying stunned against the wall: the long punish window.
    pub stunned: bool,
    /// The ceiling is shedding rocks.
    pub rocks_falling: bool,
    /// A shock is rolling along the floor.
    pub shock_rolling: bool,
    /// The hall he measured, once he has.
    pub hall: Option<Hall>,
}

/// The conductor's record on `rex`, as a view. `None` before the module stored
/// anything (it stores nothing until it has measured its hall).
pub fn conductor_of(world: &World, rex: Entity) -> Option<TrexView> {
    use ambition_content_modules::trex::Conductor;
    let records = world.get::<ambition_extension_host::BodyRecords>(rex)?;
    let c = Conductor::from_record(records.get(&Conductor::KEY)?).ok()?;
    Some(TrexView {
        performing: c.performing(),
        charging: c.charging.is_some(),
        stunned: c.stunned.is_some(),
        rocks_falling: c.rocks_falling(),
        shock_rolling: c.shock_rolling(),
        hall: c.hall_known.then_some(Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right }),
    })
}
