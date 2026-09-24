//! The brain's action stream, routed around the moveset.
//!
//! A brain's intent resolves through the body's `ActionSet` into flat
//! `ActorActionMessage`s, except where the body's moveset authors the move: a
//! `ranged` verb in the live moveset owns the ranged press (its timed event
//! re-emits the request with live aim), so the flat emission and the charge
//! stream stand aside. That decision reads the moveset itself, which is why
//! these systems live beside it rather than with the brain.

use bevy::prelude::*;

use ambition_characters::brain::{
    action_set, ActionSet, ActorActionMessage, BrainActionCounter, ChargesProjectiles,
};

use crate::moveset::{routes_ranged, ActorMoveset};

/// Bevy system: walk every actor entity that has a Brain +
/// ActionSet + ambition_characters::control::ActorControl + BodyKinematics and emit one
/// `ActorActionMessage` per resolved action request. Runs after the
/// brain-driver systems (tick_controlled_brains, update_ecs_actors's
/// runtime tick) so the frame is current.
///
/// The origin is the body's own position (`BodyKinematics`), never Bevy
/// `Transform`, which belongs to presentation.
pub fn emit_brain_action_messages(
    actors: Query<(
        Entity,
        &ambition_characters::control::ActorControl,
        &ActionSet,
        &ambition_platformer2d_core::BodyKinematics,
        Option<&ActorMoveset>,
        bevy::prelude::Has<ChargesProjectiles>,
    )>,
    mut writer: MessageWriter<ActorActionMessage>,
) {
    for (entity, control, action_set, kin, moveset, charges) in &actors {
        let moveset_ranged = moveset.is_some_and(routes_ranged);
        for request in action_set::resolve(action_set, &control.0, kin.pos) {
            // A body whose ranged shot is a moveset `"ranged"` move fires through the
            // move's timed event (`MoveEventKind::Ranged`), not this flat
            // `frame.fire → Ranged` path — skip the flat emission so it doesn't fire
            // TWICE (the moveset subsumes ranged just as it did melee/specials). The
            // move's fire event re-emits an identical `Ranged` request downstream.
            //
            // A CHARGE body's ranged intent belongs to the charge path
            // (`emit_player_projectile_tick_messages` hands it over as a press,
            // a release, or an autonomous tap), so it is skipped here for the
            // same reason: one intent, one owner.
            if (moveset_ranged || charges)
                && matches!(request, action_set::ActionRequest::Ranged { .. })
            {
                continue;
            }
            writer.write(ActorActionMessage {
                actor: entity,
                request,
                move_instance: None,
            });
        }
    }
}

/// Bevy system: emit one `ActorActionMessage::PlayerProjectileTick`
/// per charge-capable actor per tick. The charge-projectile input
/// consumer (`charge_projectile_input` in `ambition_platformer2d_actor_monolith`) drives its
/// motion-recognition buffer + Fireball charge state machine from
/// this stream from the already translated `ambition_characters::control::ActorControl` rather than raw slot input.
///
/// Emitted every tick — even on neutral input — because the
/// motion-recognition buffer needs continuous axis samples to detect
/// QCF / half-circle gestures (a "down → down-right → right → press"
/// sequence needs samples from every frame of the rotation, not just
/// the press frame). The consumer cheaply pushes the axis sample
/// into the buffer on idle ticks.
pub fn emit_player_projectile_tick_messages(
    actors: Query<(
        Entity,
        &ambition_characters::control::ActorControl,
        Option<&ChargesProjectiles>,
        Option<&ActorMoveset>,
    )>,
    mut writer: MessageWriter<ActorActionMessage>,
) {
    for (entity, control, charges, moveset) in &actors {
        let moveset_ranged = moveset.is_some_and(routes_ranged);
        // Capability gate, not an identity gate: emit the charge-tick stream for
        // any actor that carries the chargeable-projectile ability — the player
        // today, a possessed body that adopts the player's kit tomorrow. (Was
        // `brain.is_player()`; bosses/enemies carry a `ranged` ActionSet for their
        // OWN projectiles, so this stays a dedicated opt-in marker, pay-for-use.)
        if charges.is_none() {
            continue;
        }
        // ⛔ ONE PRESS, ONE OWNER, DECIDED ON THE EFFECTIVE REPERTOIRE. Charging
        // is a CHARACTER fact, but the hand replaces the ranged slot after it:
        // preparation revokes a charger's own `ranged` verb, so a `ranged` move
        // in the live moveset is the held item's — and a held item is the whole
        // ranged vocabulary. The move answers the press; the charge path does
        // not hear it until the hand lets go.
        if moveset_ranged {
            continue;
        }
        let frame = &control.0;
        writer.write(ActorActionMessage {
            actor: entity,
            request: action_set::ActionRequest::PlayerProjectileTick {
                axis: frame.locomotion.vec(),
                aim: frame.aim.vec(),
                press: frame.projectile_pressed,
                held: frame.projectile_held,
                released: frame.projectile_released,
                intent: frame.fire.is_some()
                    && !frame.projectile_pressed
                    && !frame.projectile_held
                    && !frame.projectile_released,
            },
            move_instance: None,
        });
    }
}

/// Bevy system: observe the `ActorActionMessage` stream and update
/// the counter. Runs after the emitters above. Doesn't consume the
/// messages — other readers still see them.
pub fn observe_brain_action_counter(
    mut counter: ResMut<BrainActionCounter>,
    mut reader: MessageReader<ActorActionMessage>,
) {
    let this_frame = reader.read().count() as u32;
    counter.last_frame = this_frame;
    counter.total = counter.total.wrapping_add(this_frame as u64);
}

#[cfg(test)]
mod tests;
