//! Health restoration requests and their application to home bodies.

use bevy::prelude::*;
use ambition_characters::actor::BodyHealth;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

/// Typed heal request message for gameplay heal events.
///
/// `target` carries the player entity the heal should land on. `None`
/// keeps the legacy behavior (route to the primary player) — useful
/// for cutscene / quest / dev-tool heals that aren't tied to a
/// specific player. A concrete `Some(entity)` is what per-player
/// pickup collection (#17.6 bridge) sets so a non-primary player's
/// pickup heals them, not the primary.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerHealRequested {
    pub amount: i32,
    pub target: Option<Entity>,
}

impl PlayerHealRequested {
    /// Heal request without a specific player target — applies to the
    /// primary player.
    pub fn new(amount: i32) -> Self {
        Self {
            amount,
            target: None,
        }
    }

    /// Heal request targeting a specific player entity. Use when the
    /// producer already knows which player to heal (pickup overlap,
    /// per-player ability, etc.).
    pub fn for_target(amount: i32, target: Entity) -> Self {
        Self {
            amount,
            target: Some(target),
        }
    }
}


/// Apply heal messages to the authoritative `BodyHealth` ECS component.
///
/// A heal targets either a specific player entity (`heal.target ==
/// Some(entity)`) or the primary player as a fallback (`None`). The
/// fallback path keeps existing call sites — cutscene heals, dev-tool
/// heals — working with no change. Per-player producers like pickup
/// collection should set the target explicitly so a non-primary
/// player who walked into the heart actually gets healed.
pub fn apply_player_heal_requests(
    mut heals: MessageReader<PlayerHealRequested>,
    mut players: Query<&mut BodyHealth, With<PlayerEntity>>,
    primary_q: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let primary = primary_q.single().ok();
    for heal in heals.read() {
        if heal.amount <= 0 {
            continue;
        }
        let target = heal.target.or(primary);
        let Some(target) = target else {
            // No player entity yet (startup or headless): drop the
            // heal silently so the queue still drains.
            continue;
        };
        if let Ok(mut health) = players.get_mut(target) {
            health.heal(heal.amount);
        }
    }
}


#[cfg(test)]
mod tests {
use super::*;
/// A `PlayerHealRequested` carrying `target: Some(p2)` heals p2,
/// not the primary p1. Pins the OVERNIGHT-TODO #17.6 bridge —
/// pickups now route heals to the player who actually overlapped
/// the heart instead of always to primary.
#[test]
fn targeted_heal_routes_to_named_entity_not_primary() {
    use ambition_characters::actor::BodyHealth;

    let mut app = App::new();
    app.add_message::<PlayerHealRequested>();
    app.add_systems(Update, apply_player_heal_requests);

    let p1 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
        ))
        .id();

    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<PlayerHealRequested>>()
        .write(PlayerHealRequested::for_target(2, p2));
    app.update();

    let p1_health = app.world().entity(p1).get::<BodyHealth>().unwrap();
    let p2_health = app.world().entity(p2).get::<BodyHealth>().unwrap();
    assert_eq!(p1_health.current(), 1, "primary must not pick up p2's heal");
    assert_eq!(p2_health.current(), 3, "p2 must be healed by 2");
}

/// `PlayerHealRequested::new` (target = None) keeps legacy
/// behavior: heal lands on the primary player. Pins the
/// backwards-compatible path so cutscene/quest heals don't
/// silently break when other code starts using `for_target`.
#[test]
fn untargeted_heal_routes_to_primary() {
    use ambition_characters::actor::BodyHealth;

    let mut app = App::new();
    app.add_message::<PlayerHealRequested>();
    app.add_systems(Update, apply_player_heal_requests);

    let p1 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
        ))
        .id();

    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<PlayerHealRequested>>()
        .write(PlayerHealRequested::new(3));
    app.update();

    let p1_health = app.world().entity(p1).get::<BodyHealth>().unwrap();
    let p2_health = app.world().entity(p2).get::<BodyHealth>().unwrap();
    assert_eq!(p1_health.current(), 4, "primary picks up untargeted heal");
    assert_eq!(p2_health.current(), 1, "p2 not touched by untargeted heal");
}

}
