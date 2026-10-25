//! The Inugami: Wolfenstein's head, torn off his dead body and risen as a red
//! spirit, a boss of its own (`docs/planning/game/bosses.md`, "The Inugami").
//!
//! Its conductor is the procedural module `ambition_content_modules::inugami`.
//! This is what the game keeps here: its id, its birth, and a view of the
//! module's record for tests and inspectors.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::bosses::hall::Hall;

pub use ambition_content_modules::inugami::{Move, INUGAMI_ID};

/// The state it is built with: the side it faces (its module chooses it from
/// then on) and its drawn row.
pub fn birth(scope: &mut ambition_platformer2d_shared_tangle::construction::EntityScope, body: &ae::BodyKinematics) {
    scope.insert((
        ambition_boss_encounter::conduct::ConductedFacing::of_facing(body.facing),
        ambition_platformer2d::sprite_sheet::character::PinnedRow::default(),
    ));
}

/// What its conductor is doing, read from its module's record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InugamiView {
    /// The move being performed and whether it is striking.
    pub performing: Option<(Move, bool)>,
    /// Its teeth in the floor after the dive.
    pub buried: bool,
    /// Its fire spent, on the floor: the long punish window.
    pub guttered: bool,
    /// The room it measured, once it has.
    pub hall: Option<Hall>,
    /// Seconds since its encounter woke: `None` while it sleeps in his body.
    pub intro: Option<f32>,
    /// Its rise is over: it fights.
    pub woke: bool,
    /// It has called Kagero to your aid.
    pub ally_called: bool,
    /// Its ward of blue fire is lit: no blow hurts it.
    pub warded: bool,
    /// It hunts you to the final hall.
    pub running: bool,
}

/// The conductor's record on `head`, as a view. `None` before the module
/// stored anything.
pub fn conductor_of(world: &World, head: Entity) -> Option<InugamiView> {
    use ambition_content_modules::inugami::Conductor;
    let records = world.get::<ambition_extension_host::BodyRecords>(head)?;
    let c = Conductor::from_record(records.get(&Conductor::KEY)?).ok()?;
    Some(InugamiView {
        performing: c.performing(),
        buried: c.buried(),
        guttered: c.guttered(),
        hall: c.hall_known.then_some(Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right }),
        intro: c.intro,
        woke: c.woke(),
        ally_called: c.ally_called(),
        warded: c.warded(),
        running: c.running(),
    })
}
