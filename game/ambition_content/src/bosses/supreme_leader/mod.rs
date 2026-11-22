//! The Supreme Leader: the ruler of the Nazi Dimension's regime, a pattern
//! fight in the War Room (`docs/planning/game/bosses.md`, "The Supreme
//! Leader").
//!
//! His conductor — it walks him, runs his Command Uplink and performs his
//! moves — is the procedural module `ambition_content_modules::
//! supreme_leader`, a conducted boss on the extension host
//! (`ambition.boss.conduct`). This is what the game keeps here: his id, his
//! birth, and a view of the module's record for tests and inspectors.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::bosses::hall::Hall;

pub use ambition_content_modules::supreme_leader::{Move, SUPREME_LEADER_ID};
/// The conducted module itself, for tests that ask where a move lands.
pub use ambition_content_modules::supreme_leader as conductor_module;

/// The state he is built with: the side he faces (his module chooses it from
/// then on) and his drawn row. He is hit through his body box.
pub fn birth(_content: &ambition_platformer2d::sprite_sheet::SpriteContent, scope: &mut ambition_platformer2d_shared_tangle::construction::EntityScope, body: &ae::BodyKinematics) {
    scope.insert((
        ambition_boss_encounter::conduct::ConductedFacing::of_facing(body.facing),
        ambition_platformer2d::sprite_sheet::character::PinnedRow::default(),
    ));
}

/// What his conductor is doing, read from its module's record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupremeLeaderView {
    /// The move being performed and whether it is striking.
    pub performing: Option<(Move, bool)>,
    /// An uplink stands for him.
    pub uplink_up: bool,
    /// How many uplinks he has called down.
    pub uplinks_called: u32,
    /// Reporting a failed order: open to every blow.
    pub reporting: bool,
    /// The sceptre overheated after the Mandate.
    pub overheated: bool,
    /// The Emergency Powers are his.
    pub emergency: bool,
    /// The guard he asked for last.
    pub guarded: bool,
    /// The phase beats he has begun.
    pub transitions: u32,
    /// The room he measured, once he has.
    pub hall: Option<Hall>,
    /// His intro is over: he fights.
    pub woke: bool,
}

/// The conductor's record on `leader`, as a view. `None` before the module
/// stored anything (it stores nothing until it has measured its room).
pub fn conductor_of(world: &World, leader: Entity) -> Option<SupremeLeaderView> {
    use ambition_content_modules::supreme_leader::Conductor;
    let records = world.get::<ambition_extension_host::BodyRecords>(leader)?;
    let c = Conductor::from_record(records.get(&Conductor::KEY)?).ok()?;
    Some(SupremeLeaderView {
        performing: c.performing(),
        uplink_up: c.uplink_up(),
        uplinks_called: c.uplinks_called(),
        reporting: c.reporting(),
        overheated: c.overheated(),
        emergency: c.emergency(),
        guarded: c.guarded(),
        transitions: c.transitions(),
        hall: c.hall_known.then_some(Hall { floor: c.hall_floor, left: c.hall_left, right: c.hall_right }),
        woke: c.woke,
    })
}
