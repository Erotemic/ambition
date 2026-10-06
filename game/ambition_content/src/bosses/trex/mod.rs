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
pub fn birth(scope: &mut ambition_platformer2d_shared_tangle::construction::EntityScope, body: &ae::BodyKinematics) {
    scope.insert((
        ambition_boss_encounter::conduct::ConductedFacing::of_facing(body.facing),
        ambition_platformer2d::sprite_sheet::character::PinnedRow::default(),
    ));
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
