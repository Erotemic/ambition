//! Health restoration requests and their application to bodies.

use bevy::prelude::*;
use ambition_characters::actor::BodyHealth;

/// Restore health on one named body.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyHealRequested {
    pub amount: i32,
    pub target: Entity,
}

impl BodyHealRequested {
    pub fn new(amount: i32, target: Entity) -> Self {
        Self { amount, target }
    }
}

pub fn apply_body_heal_requests(
    mut heals: MessageReader<BodyHealRequested>,
    mut bodies: Query<&mut BodyHealth>,
) {
    for heal in heals.read() {
        if heal.amount > 0 {
            if let Ok(mut health) = bodies.get_mut(heal.target) {
                health.heal(heal.amount);
            }
        }
    }
}


#[cfg(test)]
mod tests {
use super::*;
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;
/// A named body can heal without a player marker.
#[test]
fn healing_changes_only_the_named_body() {
    use ambition_characters::actor::BodyHealth;

    let mut app = App::new();
    app.add_message::<BodyHealRequested>();
    app.add_systems(Update, apply_body_heal_requests);

    let p1 = app
        .world_mut()
        .spawn((
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
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
        ))
        .id();

    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<BodyHealRequested>>()
        .write(BodyHealRequested::new(2, p2));
    app.update();

    let p1_health = app.world().entity(p1).get::<BodyHealth>().unwrap();
    let p2_health = app.world().entity(p2).get::<BodyHealth>().unwrap();
    assert_eq!(p1_health.current(), 1, "primary must not pick up p2's heal");
    assert_eq!(p2_health.current(), 3, "p2 must be healed by 2");
}


}
