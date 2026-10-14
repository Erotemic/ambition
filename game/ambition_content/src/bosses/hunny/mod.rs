//! The Hunny Horror: a stuffed bear the size of a door, and a pattern fight
//! (`docs/planning/game/bosses.md`, "The Hunny Horror").
//!
//! His conductor — it walks him and performs his moves — is the procedural
//! module `ambition_content_modules::hunny`, a conducted boss on the extension
//! host (`ambition.boss.conduct`), like the T-rex. This is what the game keeps
//! here: his id, his birth, and a view of the module's record for tests and
//! inspectors.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::bosses::hall::Hall;

pub use ambition_content_modules::hunny::{Move, HUNNY_ID};
/// The conducted module itself: its measured volumes, for tests that ask
/// where a move should land.
pub use ambition_content_modules::hunny as conductor_module;

/// The state he is built with: the side he faces (his module chooses it from
/// then on) and his drawn row. He is hit through his body box: he has no body
/// rig.
pub fn birth(scope: &mut ambition_platformer2d_shared_tangle::construction::EntityScope, body: &ae::BodyKinematics) {
    scope.insert((
        ambition_boss_encounter::conduct::ConductedFacing::of_facing(body.facing),
        ambition_platformer2d::sprite_sheet::character::PinnedRow::default(),
    ));
}

/// What his conductor is doing, read from its module's record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HunnyView {
    /// The move being performed and whether it is striking.
    pub performing: Option<(Move, bool)>,
    /// Up on his red balloon, rising, before he lets go.
    pub rising: bool,
    /// Pouncing: he has let go and not landed.
    pub pouncing: bool,
    /// Sitting dazed where his pounce landed: the long punish window.
    pub dazed: bool,
    /// He holds a body in his arms, squeezing it.
    pub squeezing: bool,
    /// Honey puddles lie on the floor.
    pub puddles: bool,
    /// The room he measured, once he has.
    pub hall: Option<Hall>,
}

/// The conductor's record on `bear`, as a view. `None` before the module
/// stored anything (it stores nothing until it has measured its room).
pub fn conductor_of(world: &World, bear: Entity) -> Option<HunnyView> {
    use ambition_content_modules::hunny::Conductor;
    let records = world.get::<ambition_extension_host::BodyRecords>(bear)?;
    let c = Conductor::from_record(records.get(&Conductor::KEY)?).ok()?;
    Some(HunnyView {
        performing: c.performing(),
        rising: c.rising(),
        pouncing: c.pouncing(),
        dazed: c.dazed(),
        squeezing: c.squeezing(),
        puddles: c.puddles_live(),
        hall: c.hall_known.then_some(Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right }),
    })
}
