//! GNU-ton's id, his birth, and a view of his conductor's record.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::bosses::hall::Hall;

pub use ambition_content_modules::gnu_ton::choreography::blows::{Arm, Blow, Exit};
pub use ambition_content_modules::gnu_ton::choreography::Move;
pub use ambition_content_modules::gnu_ton::conductor::GNU_TON_ID;

/// The state the scholar is built with: his drawn row. His conductor's memory
/// is the module's record, which the module writes when it first conducts.
/// See [`ambition_boss_encounter::BossBirthKit`].
pub fn birth(scope: &mut ambition_platformer2d_shared_tangle::construction::EntityScope, _: &ae::BodyKinematics) {
    scope.insert(ambition_platformer2d::sprite_sheet::character::PinnedRow::default());
}

/// What GNU-ton's conductor is doing, read from its module's record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnuTonView {
    /// The move being performed and whether it is striking.
    pub performing: Option<(Move, bool)>,
    /// The hall under the gnu, once it has measured it.
    pub hall: Option<Hall>,
    /// The look lines, where each is: the left shoulder's, the right
    /// shoulder's, the left wrist's, the right wrist's.
    pub look_lines: [Option<ae::Vec2>; 4],
    /// The blow whose portal pair is open, and whether it landed on a body.
    pub blow: Option<(Exit, bool)>,
    /// The limb of that blow (which arm, or the leg), and whether its fist
    /// is in the floor and an opening (a hammer's punish window).
    pub blow_limb: Option<(Blow, bool)>,
}

/// The conductor's record on `scholar`, as a view. `None` before the module
/// stored anything.
pub fn conductor_of(world: &World, scholar: Entity) -> Option<GnuTonView> {
    use ambition_content_modules::gnu_ton::conductor::Conductor;
    let records = world.get::<ambition_extension_host::BodyRecords>(scholar)?;
    let c = Conductor::from_record(records.get(&Conductor::KEY)?).ok()?;
    Some(GnuTonView {
        performing: c.performing(),
        hall: c.hall().map(|hall| Hall { floor: hall.floor, left: hall.left, right: hall.right }),
        look_lines: c.look_lines().map(|line| line.map(|(at, _)| ae::Vec2::new(at.x, at.y))),
        blow: c.blow(),
        blow_limb: c.blow_limb(),
    })
}
