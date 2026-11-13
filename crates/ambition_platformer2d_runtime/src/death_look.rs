//! A participant's death, drawn: the game's [`DeathLook`] effect where the
//! body is taken when its death window opens, and the other where it comes
//! back into play (PLAYER-DEATH).
//!
//! Presentation only. Each reads a fact the death systems already wrote (the
//! death message, `OutOfPlay` coming off) and writes nothing the simulation
//! reads.

use bevy::prelude::*;

use ambition_combat::death_rules::{ActorDiedMessage, DeathLook, OutOfPlay};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_vfx::{FxPose, VfxMessage, VfxWriter};

const SCALE: f32 = 1.0;

/// The departure, where a participant died, in its own room's look.
pub fn draw_death_departures(
    mut deaths: MessageReader<ActorDiedMessage>,
    looks: ambition_platformer2d_world::governing_rules::RulesOf<DeathLook>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    // Out of play already: the window opened on this death before this ran.
    players: Query<&BodyKinematics, With<PlayerEntity>>,
    mut vfx: VfxWriter,
) {
    for death in deaths.read() {
        let (Ok(kin), Some(look)) = (players.get(death.victim), looks.of(death.victim)) else { continue };
        vfx.write_in(rooms.of(death.victim), VfxMessage::Effect { pos: kin.pos, fx: look.departure, scale: SCALE, pose: FxPose::UPRIGHT });
    }
}

/// The arrival, where a participant came back into play.
pub fn draw_death_arrivals(
    mut returned: RemovedComponents<OutOfPlay>,
    looks: ambition_platformer2d_world::governing_rules::RulesOf<DeathLook>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    players: Query<&BodyKinematics, (With<PlayerEntity>, Without<OutOfPlay>)>,
    mut vfx: VfxWriter,
) {
    for body in returned.read() {
        let (Ok(kin), Some(look)) = (players.get(body), looks.of(body)) else { continue };
        vfx.write_in(rooms.of(body), VfxMessage::Effect { pos: kin.pos, fx: look.arrival, scale: SCALE, pose: FxPose::UPRIGHT });
    }
}
