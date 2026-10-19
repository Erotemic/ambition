//! Wolfenstein: Frankenstein's monster with the head of a mad wolf, the boss
//! of Castle Godwin, and a pattern fight (`docs/planning/game/bosses.md`,
//! "Wolfenstein — the leash").
//!
//! His conductor — it walks him and performs his moves — is the procedural
//! module `ambition_content_modules::wolfenstein`, a conducted boss on the
//! extension host (`ambition.boss.conduct`), like the Hunny Horror. This is
//! what the game keeps here: his id, his birth, and a view of the module's
//! record for tests and inspectors.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::bosses::hall::Hall;

pub use ambition_content_modules::wolfenstein::{Move, WOLFENSTEIN_ID};
/// The conducted module itself: its measured volumes, for tests that ask
/// where a move should land.
pub use ambition_content_modules::wolfenstein as conductor_module;

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
pub struct WolfensteinView {
    /// The move being performed and whether it is striking.
    pub performing: Option<(Move, bool)>,
    /// Flat after the galvanize, his charge spent: the long punish window.
    pub flat: bool,
    /// Winded after the bound.
    pub winded: bool,
    /// His fists in the floor after the hammer.
    pub stuck: bool,
    /// Overreached after the stab.
    pub overreached: bool,
    /// A shock rolls the floor.
    pub shock_rolling: bool,
    /// The leash is broken: he has been through a transition.
    pub unleashed: bool,
    /// The room he measured, once he has.
    pub hall: Option<Hall>,
    /// Seconds into his intro: `None` while he sleeps on his throne.
    pub intro: Option<f32>,
    /// His intro is over: he fights.
    pub woke: bool,
}

/// The conductor's record on `wolf`, as a view. `None` before the module
/// stored anything (it stores nothing until it has measured its room).
pub fn conductor_of(world: &World, wolf: Entity) -> Option<WolfensteinView> {
    use ambition_content_modules::wolfenstein::Conductor;
    let records = world.get::<ambition_extension_host::BodyRecords>(wolf)?;
    let c = Conductor::from_record(records.get(&Conductor::KEY)?).ok()?;
    Some(WolfensteinView {
        performing: c.performing(),
        flat: c.flat(),
        winded: c.winded(),
        stuck: c.stuck(),
        overreached: c.overreached(),
        shock_rolling: c.shock_rolling(),
        unleashed: c.unleashed(),
        hall: c.hall_known.then_some(Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right }),
        intro: c.intro,
        woke: c.woke,
    })
}
