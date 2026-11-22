//! Native responses and shared samples for body contact rules.

use ambition_characters::actor::BodyHealth;
use ambition_combat_port::{
    BodyContact, BodyContactPort, BodyDefeatPort, ContactBody, ContactBouncePort, ContactCuePort, ContactThreatPort,
    ControlHoldPort, KineticHazardPort, PosePinPort,
};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::BODY_CONTACT;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

mod facts;
mod geometry;
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
    app.install_extension_submitted_request::<ContactCuePort, _>(BODY_CONTACT, "ambition_combat", lower_contact_cues);
    app.install_extension_submitted_request::<ContactThreatPort, _>(BODY_CONTACT, "ambition_combat", lower_contact_threats);
    app.install_extension_submitted_request::<ControlHoldPort, _>(BODY_CONTACT, "ambition_combat", lower_control_holds);
    app.install_extension_submitted_request::<PosePinPort, _>(BODY_CONTACT, "ambition_combat", lower_pose_pins);
    app.install_extension_submitted_request::<KineticHazardPort, _>(BODY_CONTACT, "ambition_combat", lower_kinetic_hazards);
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

/// A toucher plays its pogo cue at the touched body: the touch was its verb.
pub fn lower_contact_cues(
    mut outbox: ResMut<ExtensionOutbox>,
    table: Res<ContactTouchers>,
    mut sfx: ambition_sfx::BodySfxWriter,
    live: LiveRooms,
) {
    for submitted in outbox.drain::<ContactCuePort>(&BODY_CONTACT) {
        let cue = submitted.value;
        let toucher = table
            .0
            .iter()
            .find(|(body, ..)| *body == submitted.scope)
            .and_then(|(_, at, touchers)| Some((*at, *touchers.get(cue.toucher as usize)?)));
        let Some((at, toucher)) = toucher else {
            warn!(
                "extension entry {} asked toucher {} of {:?} for its cue, which is not a toucher of its contact; refused",
                submitted.entry, cue.toucher, submitted.scope
            );
            continue;
        };
        sfx.in_space(ambition_vfx::sfx_space(live.of(toucher))).write_for(toucher, ambition_sfx::SfxMessage::Pogo { pos: at });
    }
}

/// The body's touch hurts nobody, or hurts again. The shared contact damage
/// of this tick reads it.
pub fn lower_contact_threats(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    mut bodies: Query<Option<&mut crate::actor_tuning::ContactThreatWithdrawn>>,
) {
    for submitted in outbox.drain::<ContactThreatPort>(&BODY_CONTACT) {
        let withdrawn = submitted.value.withdrawn;
        match bodies.get_mut(submitted.scope) {
            Ok(Some(mut threat)) => threat.0 = withdrawn,
            Ok(None) => {
                commands.entity(submitted.scope).try_insert(crate::actor_tuning::ContactThreatWithdrawn(withdrawn));
            }
            Err(_) => {}
        }
    }
}

/// The application a submission belongs to: the attachment that submitted
/// it (the occurrence whose records it keeps, and its module). Two entries of
/// one attachment share it; another attachment never does.
fn submitter_key<T>(
    submitted: &ambition_extension_host::Submitted<T>,
    ids: &Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
) -> ambition_platformer2d_core::applications::ApplicationKey {
    let occurrence = ids.get(submitted.state_scope).map_or("", |id| id.as_str());
    ambition_platformer2d_core::applications::ApplicationKey::attachment(
        occurrence,
        ambition_platformer2d_core::applications::module_of_entry(&submitted.entry),
    )
}

/// The body holds its control under the submitting attachment's claim, or
/// that attachment lets its claim go. Another attachment's claim, and every
/// native hold, stay.
pub fn lower_control_holds(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    bodies: Query<Entity>,
    ids: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
) {
    use ambition_characters::control::{claim_hold_as, release_hold_as};
    for submitted in outbox.drain::<ControlHoldPort>(&BODY_CONTACT) {
        if !bodies.contains(submitted.scope) {
            continue;
        }
        let key = submitter_key(&submitted, &ids);
        if submitted.value.held {
            claim_hold_as(&mut commands, submitted.scope, key);
        } else {
            release_hold_as(&mut commands, submitted.scope, key);
        }
    }
}

/// The body is drawn in one row of its sheet under the submitting
/// attachment's pose claim, or that attachment withdraws its pose. A row the
/// game does not know is refused.
pub fn lower_pose_pins(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    ids: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
) {
    use ambition_sprite_sheet::character::{claim_pose, release_pose, CharacterAnim, PoseTerm, SCRIPT_POSE_PRIORITY};
    for submitted in outbox.drain::<PosePinPort>(&BODY_CONTACT) {
        let key = submitter_key(&submitted, &ids);
        match submitted.value.row.as_deref() {
            None => release_pose(&mut commands, submitted.scope, key),
            Some(row) => match CharacterAnim::from_name(row) {
                Some(anim) => claim_pose(&mut commands, submitted.scope, key, PoseTerm { anim, priority: SCRIPT_POSE_PRIORITY }),
                None => warn!("extension entry {} asked to pin the row {row:?}, which no sheet has; refused", submitted.entry),
            },
        }
    }
}

/// The body is a hazard this tick: a contact hit of its own extent where it
/// is, on no side, credited to the body named or to itself, sparing itself
/// and the bodies named.
pub fn lower_kinetic_hazards(
    mut outbox: ResMut<ExtensionOutbox>,
    mut strikes: crate::damage_box::DamageBoxSpawner,
    bodies: Query<&ae::BodyKinematics>,
    identities: Query<(Entity, &SimId)>,
) {
    for submitted in outbox.drain::<KineticHazardPort>(&BODY_CONTACT) {
        let hazard = submitted.value;
        if hazard.damage < 0 || !(hazard.knockback.is_finite() && hazard.knockback >= 0.0) {
            warn!("extension entry {} asked for a kinetic hazard {hazard:?}; refused", submitted.entry);
            continue;
        }
        let Ok(kin) = bodies.get(submitted.scope) else {
            continue;
        };
        let named = |id: &str| identities.iter().find(|(_, identity)| identity.as_str() == id).map(|(body, _)| body);
        let owner = hazard.credit.as_deref().and_then(named).unwrap_or(submitted.scope);
        let mut excluded = crate::strike::HitboxExclusions::default();
        excluded.bodies.insert(submitted.scope);
        excluded.bodies.extend(hazard.spare.iter().filter_map(|id| named(id)));
        if let Some(mut strike) = strikes.spawn(
            submitted.scope,
            owner,
            ambition_vfx::HitSide::Environment,
            kin.pos,
            crate::strike::DamageBox {
                cause: crate::events::HitSource::Contact,
                move_instance: None,
                half_extent: kin.size() * 0.5,
                shape: None,
                damage: hazard.damage,
                knockback: crate::strike::HitboxKnockback::FeelScale(hazard.knockback),
                lifetime_s: 0.0,
                name: Some("kinetic hazard"),
            },
        ) {
            strike.insert(excluded);
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
