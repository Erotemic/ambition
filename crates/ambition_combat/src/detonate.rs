//! A move that ends its own mover: the handler of
//! [`ambition_entity_catalog::detonate::DETONATE`].
//!
//! The detonation is a lethal hit the body deals itself, the road a charging
//! shark that hits a wall already takes, so the death is an ordinary death:
//! its death traits run (an `explodes_on_death` body leaves its blast), its
//! death cue plays and its fate is recorded.

use bevy::prelude::*;

use ambition_characters::brain::{ActionRequest, ActorActionMessage, SpecialActionSpec};
use ambition_entity_catalog::detonate::DETONATE;

use crate::events::{HitEvent, HitMode, HitSource, HitTarget};

/// End each body whose move reached its authored detonation.
pub fn apply_authored_detonations(
    mut actions: MessageReader<ActorActionMessage>,
    bodies: Query<(
        &ambition_characters::actor::body::BodyHealth,
        &ambition_platformer2d_core::BodyKinematics,
    )>,
    mut hits: MessageWriter<HitEvent>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec: SpecialActionSpec::Special(key), .. } = &message.request else {
            continue;
        };
        if key.as_str() != DETONATE {
            continue;
        }
        let Ok((health, kin)) = bodies.get(message.actor) else {
            continue;
        };
        if !health.alive() {
            continue;
        }
        hits.write(HitEvent {
            contact: None,
            pieces: Vec::new(),
            strike_sfx: None,
            volume: kin.aabb().into(),
            // Its whole pool: a detonation is not a hit it can survive.
            damage: health.current().max(1),
            source: HitSource::Contact,
            attacker: Some(message.actor),
            room: None,
            target: HitTarget::Body(message.actor),
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
            attacker_move_instance: message.move_instance,
        });
    }
}
