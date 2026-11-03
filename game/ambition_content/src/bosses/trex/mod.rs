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
/// And his BODY RIG (`trex_enemy_gameplay_body_rig.ron`, published from the rig that
/// draws him): the parts you can hit him through, head, jaw, neck, torso,
/// tail and legs, posed every tick from the row his module pins, so they are
/// where his art is. His sheet's frame centre is his position (the boss
/// placement law), so the rig's origin, his feet pixel, is offset from it.
pub fn birth(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent, scope: &mut ambition_platformer2d_shared_tangle::construction::EntityScope, body: &ae::BodyKinematics) {
    scope.insert((
        ambition_boss_encounter::conduct::ConductedFacing::of_facing(body.facing),
        ambition_platformer2d::sprite_sheet::character::PinnedRow::default(),
    ));
    let (rig, feet) = body_rig(&sprite_content);
    scope.insert((
        ambition_combat::body_rig::BodyRig(std::sync::Arc::new(rig)),
        ambition_combat::body_rig::BodyRigPose::default(),
        ambition_combat::body_rig::RigFeetOffset(feet),
        ambition_combat::hurtbox_resolution::ResolvedHurtboxes::default(),
    ));
}

/// The sheet his body rig is published beside.
pub const SHEET: &str = "trex_enemy";

/// Read the gameplay rig and its origin in world units.
///
/// # Panics
///
/// The gameplay product must exist and parse. A missing rig must not give the
/// T-rex a simpler hurt shape.
pub fn body_rig(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent) -> (ambition_characters::actor::body_rig::PreparedBodyRig, Vec2) {
    let px = conductor_module::PX;
    let gameplay = format!("{SHEET}_gameplay");
    let text = ambition_sprite_sheet::baked_body_rigs::baked_body_rig(&sprite_content, &gameplay)
        .unwrap_or_else(|| panic!("`{gameplay}_body_rig.ron` is not published; run ./scripts/regen/sprites.sh --target {gameplay}"));
    let (definition, origin) = ambition_characters::actor::BodyRigDefinition::from_published_ron_with_origin(text)
        .unwrap_or_else(|error| panic!("`{gameplay}_body_rig.ron`: {error}"));
    let rig = definition.scaled(px)
        .prepare()
        .unwrap_or_else(|error| panic!("`{gameplay}_body_rig.ron` does not prepare: {error:?}"));
    (rig, origin * px)
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
    /// He holds a body in his jaws, thrashing it.
    pub thrashing: bool,
    /// He rears through the beat between two phases (screaming, into phase 2).
    pub rearing: bool,
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
        thrashing: c.thrashing(),
        rearing: c.rearing(),
        hall: c.hall_known.then_some(Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameplay_geometry_does_not_need_a_sheet_or_images() {
        let text = r#"(
            schema_version: 1, target: "trex_enemy_gameplay",
            origin_from_centre: (3.0, 8.0),
            joints: [(name: "root", parent: None)],
            clips: {"idle": (looping: true, frame_duration_s: 0.1,
                frames: [[(translation: (0.0, 0.0), rotation: 0.0, scale: (1.0, 1.0))]])},
        )"#;
        let content = ambition_sprite_sheet::SpriteContent::from_texts(
            ambition_sprite_sheet::baked_sheet_rons::SheetTexts {
                body_rigs: vec![("content://fixture/sprites/trex_enemy_gameplay", std::sync::Arc::from(text))],
                ..Default::default()
            },
        ).unwrap();
        let (rig, origin) = body_rig(&content);
        assert_eq!(origin, Vec2::new(3.0, 8.0) * conductor_module::PX);
        assert!(rig.solve("idle", 0, &mut Vec::new()));
    }
}
