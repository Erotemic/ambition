//! A move that pays or repays its own mover's health.
//!
//! ⭐⭐ THE OTHER HALF OF `ambition_characters::smash_vitality`. The key and its
//! params are what a MOVESET authors; charging a live body is engine work, and
//! it belongs HERE rather than in a game crate because this crate already owns
//! every other road to a body's health — `strike::apply_effects`, the hit
//! reaction, the death rules. A second place that moves the meter is exactly the
//! shape this repository keeps paying for.
//!
//! ⛔ IT IS NOT A HIT AND MUST NOT LOOK LIKE ONE. No attacker, no hitstun, no
//! knockback, no post-hit invulnerability, no death report — see
//! [`ambition_characters::actor::body::BodyHealth::spend`], which states the
//! whole difference.

use bevy::prelude::*;

use ambition_characters::brain::{ActionRequest, ActorActionMessage, SpecialActionSpec};
use ambition_characters::smash_vitality::{VitalityParams, VITALITY};

/// Recognise an authored health change and apply it to the mover.
///
/// ⛔ IT RUNS WHERE EVERY OTHER `ActorActionMessage` CONSUMER RUNS, so a change
/// authored on a move's timeline lands on the frame the move says and not a
/// phase later — the same placement the authored teleport gets, for the same
/// reason.
pub fn apply_authored_vitality(
    mut actions: MessageReader<ActorActionMessage>,
    mut bodies: Query<&mut ambition_characters::actor::body::BodyHealth>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut sfx: ambition_sfx::BodySfxWriter,
    positions: Query<&ambition_platformer2d_core::BodyKinematics>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != VITALITY {
            continue;
        }
        let params: VitalityParams = match params.hydrate() {
            Ok(params) => params,
            Err(err) => {
                warn!("vitality params did not hydrate: {err}");
                continue;
            }
        };
        let Ok(mut health) = bodies.get_mut(message.actor) else {
            continue;
        };
        // ⭐ THE SIGN IS THE WHOLE BRANCH. `heal` clamps up at the pool's max
        // and repays the meter; `spend` clamps down at the authored floor and
        // charges it. Neither can be reached by the other's number.
        let moved = if params.change > 0 {
            let before = health.current();
            health.heal(params.change);
            health.current() - before
        } else {
            -health.spend(-params.change, params.floor)
        };
        // ⛔ NOTHING IS DRAWN FOR A CHANGE THAT DID NOT HAPPEN. A Medic already
        // at full health, or already down to her floor, gets her move and its
        // frames — but a mend plume over a body that was not mended reads as a
        // heal the player did not receive, which is worse than no feedback.
        if moved == 0 {
            continue;
        }
        let at = positions
            .get(message.actor)
            .map(|kin| kin.pos)
            .unwrap_or_default();
        // The look is the MOVE's, not this system's — the same rule the
        // authored teleport follows, so two characters can spend health and
        // look nothing alike.
        vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
            pos: at,
            fx: ambition_vfx::fx::FxId::new(&params.vfx),
            scale: 1.0,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
        sfx.write_for(
            message.actor,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::SfxId::new(&params.sfx),
                pos: at,
            },
        );
    }
}

#[cfg(test)]
mod tests;
