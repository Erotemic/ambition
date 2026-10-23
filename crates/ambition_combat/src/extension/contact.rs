//! The adapters of a contact rule: `ambition.body.contact`,
//! `ambition.body.contact_bounce` and `ambition.body.defeat`. See the port
//! cards in `ambition_combat_port::contact`.
//!
//! A body whose placement authors a `CharacterBrain::Custom` key that a
//! contact module binds decides what a touch by a player body does, after the
//! bodies integrated and before the shared contact damage. A stomp or a roll
//! is a rule of its game, and it can be a module and not Rust in the demo
//! crate.

use ambition_characters::actor::BodyHealth;
use ambition_characters::control::DrivingParticipant;
use ambition_combat_port::{BodyContact, BodyContactPort, BodyDefeatPort, ContactBody, ContactBouncePort};
use ambition_entity_catalog::placements::CharacterBrain;
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::BODY_CONTACT;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

use crate::actor_tuning::ActorConfig;

/// Install the trigger port, the bounce and the defeat in `body_contact`.
pub fn install_for_body_contact(app: &mut App) {
    app.init_resource::<ContactTouchers>();
    app.install_extension_trigger::<BodyContactPort, _>(
        BODY_CONTACT,
        "ambition_combat",
        queue_body_contacts.run_if(ambition_extension_host::trigger_is_bound::<BodyContactPort>),
    );
    app.install_extension_submitted_request::<ContactBouncePort, _>(BODY_CONTACT, "ambition_combat", lower_contact_bounces);
    app.install_extension_submitted_request::<BodyDefeatPort, _>(BODY_CONTACT, "ambition_combat", lower_body_defeats);
}

/// The touchers of each touched body of this tick, in the order its
/// invocation gave them: a bounce names a toucher by its index here.
///
/// Derived: written again in each tick's collect from the bodies, and read
/// only by the lowering of the same tick.
#[derive(Resource, Default, Debug)]
pub struct ContactTouchers(pub Vec<(Entity, ae::Vec2, Vec<Entity>)>);

/// Whether the box `toucher` touches the box `body`: they overlap side to
/// side, and top to bottom they overlap or meet (a body that stands on the
/// other's head touches it).
pub fn touches(body: ae::Aabb, toucher: ae::Aabb) -> bool {
    toucher.min.x < body.max.x
        && toucher.max.x > body.min.x
        && toucher.min.y <= body.max.y
        && toucher.max.y >= body.min.y
}

/// The box of a body as the wire gives it.
fn box_of(body: &ContactBody) -> ae::Aabb {
    let centre = ae::Vec2::from(body.position);
    let half = ae::Vec2::from(body.size) * 0.5;
    ae::Aabb { min: centre - half, max: centre + half }
}

/// The body as the wire gives it: the box it collides with under its own
/// DOWN (`collision_box`), not its level box. A body in turned gravity lies
/// along its support, so its box is turned too.
fn contact_body(kin: &ae::BodyKinematics, last_step: Option<&ae::SweepSample>) -> ContactBody {
    let reach = kin.collision_box(last_step);
    let (centre, size) = ((reach.min + reach.max) * 0.5, reach.max - reach.min);
    ContactBody { position: [centre.x, centre.y], size: [size.x, size.y], velocity: [kin.vel.x, kin.vel.y] }
}

/// One invocation for each live body whose custom brain key a contact entry
/// binds and that a player body of its own live room touches, each tick with
/// a gameplay step. Bodies and touchers are in the order of their
/// simulation identity, so a rewind invokes them in the same order.
#[allow(clippy::type_complexity)]
pub fn queue_body_contacts(
    admitted: Res<AdmittedExtensions>,
    time: Res<ambition_time::WorldTime>,
    live: LiveRooms,
    mut invocations: ResMut<ExtensionInvocations>,
    mut table: ResMut<ContactTouchers>,
    bodies: Query<
        (Entity, &ActorConfig, &ae::BodyKinematics, Option<&ae::SweepSample>, &BodyHealth, Option<&SimId>),
        (Without<PlayerEntity>, Without<DrivingParticipant>),
    >,
    players: Query<(Entity, &ae::BodyKinematics, Option<&ae::SweepSample>, Option<&SimId>), With<PlayerEntity>>,
) {
    table.0.clear();
    if time.sim_dt() <= 0.0 {
        return;
    }
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == BodyContactPort::KEY)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .collect();
    if bound.is_empty() {
        return;
    }
    let mut everyone: Vec<_> = players
        .iter()
        .map(|(player, kin, last_step, id)| (player, live.of(player), contact_body(kin, last_step), id))
        .collect();
    everyone.sort_by(|a, b| (a.3.map(SimId::as_str), a.0).cmp(&(b.3.map(SimId::as_str), b.0)));
    let mut touched: Vec<_> = bodies
        .iter()
        .filter_map(|(entity, config, kin, last_step, health, id)| match &config.brain {
            CharacterBrain::Custom(key) if bound.contains(&key.as_str()) && health.alive() => {
                Some((entity, key.clone(), kin.pos, contact_body(kin, last_step), id))
            }
            _ => None,
        })
        .collect();
    touched.sort_by(|a, b| (a.4.map(SimId::as_str), a.0).cmp(&(b.4.map(SimId::as_str), b.0)));
    for (entity, key, pos, body, _) in touched {
        let room = live.of(entity);
        let touchers: Vec<_> = everyone
            .iter()
            .filter(|(_, player_room, player, _)| *player_room == room && touches(box_of(&body), box_of(player)))
            .collect();
        if touchers.is_empty() {
            continue;
        }
        invocations.trigger::<BodyContactPort>(
            &BODY_CONTACT,
            key,
            entity,
            None,
            false,
            BodyContact { body, touchers: touchers.iter().map(|(_, _, player, _)| *player).collect() },
        );
        table.0.push((entity, pos, touchers.iter().map(|(player, ..)| *player).collect()));
    }
}

/// A toucher bounces off: it jumps up off the body at the asked speed,
/// against its own gravity (its resolved frame's DOWN, which a gravity zone
/// or a portal can turn), and with the cue its pogo cue plays at the body.
/// A toucher with no resolved frame has no gravity to jump against: refused.
pub fn lower_contact_bounces(
    mut outbox: ResMut<ExtensionOutbox>,
    table: Res<ContactTouchers>,
    mut bodies: Query<(&mut ae::BodyKinematics, Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>)>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    for submitted in outbox.drain::<ContactBouncePort>(&BODY_CONTACT) {
        let bounce = submitted.value;
        let toucher = table
            .0
            .iter()
            .find(|(body, ..)| *body == submitted.scope)
            .and_then(|(_, at, touchers)| Some((*at, *touchers.get(bounce.toucher as usize)?)));
        let Some((at, toucher)) = toucher else {
            warn!(
                "extension entry {} asked toucher {} of {:?} to bounce, which is not a toucher of its contact; refused",
                submitted.entry, bounce.toucher, submitted.scope
            );
            continue;
        };
        match bodies.get_mut(toucher) {
            Ok((mut kin, Some(frame))) => ae::movement::set_jump_velocity(&mut kin.vel, frame.down(), bounce.speed),
            Ok((_, None)) => {
                warn!(
                    "extension entry {} asked {:?} to bounce, which has no resolved gravity frame; refused",
                    submitted.entry, toucher
                );
                continue;
            }
            Err(_) => continue,
        }
        if bounce.pogo_cue {
            sfx.write_for(toucher, ambition_sfx::SfxMessage::Pogo { pos: at });
        }
    }
}

/// The touched body is defeated now: its health is zero before the shared
/// contact damage reads it, and it is removed from the world.
pub fn lower_body_defeats(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: Query<&mut BodyHealth>,
) {
    for submitted in outbox.drain::<BodyDefeatPort>(&BODY_CONTACT) {
        let Ok(mut health) = bodies.get_mut(submitted.scope) else {
            warn!("extension entry {} asked to defeat {:?}, which has no health; refused", submitted.entry, submitted.scope);
            continue;
        };
        health.health.current = 0;
        commands.entity(submitted.scope).try_despawn();
    }
}

#[cfg(test)]
mod tests;
