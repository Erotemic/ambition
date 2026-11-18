//! Native responses and shared samples for body contact rules.

use ambition_characters::actor::BodyHealth;
use ambition_combat_port::{BodyContact, BodyContactPort, BodyDefeatPort, ContactBody, ContactBouncePort};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::BODY_CONTACT;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

mod facts;
pub use facts::{ContactFactsLimits, queue_body_contacts};

/// Install the trigger port, the bounce and the defeat in `body_contact`.
pub fn install_for_body_contact(app: &mut App) {
    app.init_resource::<ContactTouchers>();
    facts::install_status_requests(app);
    app.install_extension_trigger::<BodyContactPort, _>(
        BODY_CONTACT,
        "ambition_combat",
        queue_body_contacts.run_if(ambition_extension_host::trigger_is_bound::<BodyContactPort>),
    );
    app.install_extension_submitted_request::<ContactBouncePort, _>(BODY_CONTACT, "ambition_combat", lower_contact_bounces);
    app.install_extension_retirement_request::<BodyDefeatPort, _>(BODY_CONTACT, "ambition_combat", lower_body_defeats);
}

/// The touchers of each touched body of this tick, in the order its
/// invocation gave them: a bounce names a toucher by its index here.
///
/// Derived: written again in each tick's collect from the bodies, and read
/// only by the lowering of the same tick.
#[derive(Resource, Default, Debug)]
pub struct ContactTouchers(pub Vec<(Entity, ae::Vec2, Vec<Entity>)>);

/// The current physical envelope in the contact wire format.
fn contact_body(kin: &ae::BodyKinematics) -> ContactBody {
    let reach = kin.aabb();
    let (centre, size) = ((reach.min + reach.max) * 0.5, reach.max - reach.min);
    ContactBody { position: [centre.x, centre.y], size: [size.x, size.y], velocity: [kin.vel.x, kin.vel.y] }
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
    // The bounce is heard in the toucher's live room.
    live: LiveRooms,
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
            sfx.in_space(ambition_vfx::sfx_space(live.of(toucher))).write_for(toucher, ambition_sfx::SfxMessage::Pogo { pos: at });
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
