//! The god's id, its birth, and a view of its conductor's record.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::bosses::hall::Hall;

pub use ambition_content_modules::fsm::{Move, FSM_ID};

/// The state the god is built with: the side it faces (the side the body was
/// built facing; its module chooses it from then on) and its drawn row. See
/// [`ambition_boss_encounter::BossBirthKit`].
pub fn birth(scope: &mut ambition_platformer2d_shared_tangle::construction::EntityScope, body: &ae::BodyKinematics) {
    scope.insert((
        ambition_boss_encounter::conduct::ConductedFacing::of_facing(body.facing),
        ambition_platformer2d::sprite_sheet::character::PinnedRow::default(),
    ));
}

/// What the god's conductor is doing, read from its module's record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FsmView {
    /// The move being performed and whether it is striking.
    pub performing: Option<(Move, bool)>,
    /// Lying on the floor after a dive: the punish window.
    pub stranded: bool,
    /// The hall it measured, once it has.
    pub hall: Option<Hall>,
}

/// The conductor's record on `god`, as a view. `None` before the module
/// stored anything (it stores nothing until it has measured its hall).
pub fn conductor_of(world: &World, god: Entity) -> Option<FsmView> {
    use ambition_content_modules::fsm::Conductor;
    let records = world.get::<ambition_extension_host::BodyRecords>(god)?;
    let c = Conductor::from_record(records.get(&Conductor::KEY)?).ok()?;
    Some(FsmView {
        performing: c.performing(),
        stranded: c.stranded.is_some(),
        hall: c.hall_known.then_some(Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right }),
    })
}
