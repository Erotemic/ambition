//! The adapter of `ambition.combat.foes`: who a body may aim at. See the port
//! card in `ambition_combat_port::targets`.
//!
//! This answers the relation a module must not restate (who is a foe) and
//! nothing about aiming: a cone, a nearest or a lock-on is the module's rule.

use ambition_combat_port::{Foe, Foes, FoesPort};
use ambition_extension_host::ExtensionAppExt;
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

/// Offer the foes of a body in `technique_execution`.
pub fn install(app: &mut App) {
    app.install_extension_observation::<FoesPort>(TECHNIQUE_EXECUTION, "ambition_combat", foes_of);
}

/// The live room of `entity`, as `LiveRooms::of` answers it: its stamp, or
/// the sole live room.
fn room_of(world: &World, entity: Entity, sole: Option<LiveRoomInstance>) -> Option<LiveRoomInstance> {
    world.get::<InRoomInstance>(entity).map(|stamp| stamp.0).or(sole)
}

/// The foes of `scope`: each other body of its live room that it targets and
/// can hit, in the order of simulation identity.
pub fn foes_of(world: &World, scope: Entity) -> Option<Foes> {
    world.get::<ae::BodyKinematics>(scope)?;
    let mut roots = world
        .iter_entities()
        .filter(|entity| entity.contains::<RoomInstanceRoot>())
        .filter_map(|entity| entity.get::<LiveRoomInstance>().copied());
    let sole = match (roots.next(), roots.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    };
    let room = room_of(world, scope, sole);
    let faction = world.get::<ambition_characters::actor::ActorFaction>(scope).copied().unwrap_or_default();
    let team = world.get::<crate::targeting::MatchTeam>(scope);
    let relations = world.get_resource::<crate::targeting::FactionRelations>();
    let mut foes: Vec<(Option<&SimId>, Entity, ae::Vec2)> = world
        .iter_entities()
        .filter(|other| other.id() != scope)
        .filter_map(|other| Some((other, other.get::<ae::BodyKinematics>()?)))
        .filter(|(other, _)| {
            !crate::util::body_is_untouchable(
                other.get::<ambition_characters::actor::BodyHealth>(),
                other.contains::<crate::death_rules::OutOfPlay>(),
                other.get::<ae::DepthPlane>(),
            )
        })
        .filter(|(other, _)| room_of(world, other.id(), sole) == room)
        .filter(|(other, _)| {
            crate::targeting::combat_relation(
                relations,
                faction,
                None,
                team,
                None,
                other.id(),
                other.get::<ambition_characters::actor::ActorFaction>().copied().unwrap_or_default(),
                None,
                other.get::<crate::targeting::MatchTeam>(),
            )
            .is_target()
        })
        .map(|(other, kin)| (other.get::<SimId>(), other.id(), kin.pos))
        .collect();
    // Identity order; a body with no identity after every body with one.
    foes.sort_by(|a, b| match (a.0, b.0) {
        (Some(x), Some(y)) => x.cmp(y).then(a.1.cmp(&b.1)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.1.cmp(&b.1),
    });
    Some(Foes { foes: foes.into_iter().map(|(_, _, at)| Foe { position: [at.x, at.y] }).collect() })
}

#[cfg(test)]
mod tests;
