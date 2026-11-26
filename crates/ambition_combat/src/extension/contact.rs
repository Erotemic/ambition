//! Native responses and shared samples for body contact rules.

use ambition_characters::actor::BodyHealth;
use ambition_combat_port::{
    BodyContact, BodyContactPort, BodyDefeatPort, ContactBody, ContactBouncePort, ContactCuePort, ContactThreatPort,
    ControlHoldPort, KineticHazardPort, PosePinPort,
};
use ambition_extension_host::{AdmittedEntry, AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::BODY_CONTACT;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::ecs::{entity_disabling::DefaultQueryFilters, query::QueryState};
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
    app.install_extension_lowering_maintenance(BODY_CONTACT, withdraw_unbound_attachment_claims);
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

/// The live occurrence of `entity`, allocated on first need (the counter
/// and the stamp rewind with its room). `None` when it has no session or no
/// identity to allocate from.
fn live_occurrence(world: &mut World, entity: Entity) -> Option<ambition_platformer2d_core::applications::OccurrenceKey> {
    let session = world.get::<ambition_platformer2d_shared_tangle::lifecycle::SessionScopedEntity>(entity)?.0;
    let reference = ambition_platformer2d_shared_tangle::lifecycle::ensure_object_occurrence(world, entity)?.reference(session);
    Some(ambition_platformer2d_core::applications::OccurrenceKey {
        session: reference.session,
        room: reference.allocation_room,
        sequence: reference.sequence,
    })
}

/// The application a submission belongs to: the attachment that submitted
/// it, by the live occurrence of the state whose records it keeps and its
/// module. Two entries of one attachment share it; another attachment, or
/// the same module on a remade object, never does. `None` (a refusal) when
/// the state has no live occurrence: no empty issuer is made up.
fn submitter_key<T>(
    world: &mut World,
    submitted: &ambition_extension_host::Submitted<T>,
) -> Option<ambition_platformer2d_core::applications::ApplicationKey> {
    let occurrence = live_occurrence(world, submitted.state_scope)?;
    Some(ambition_platformer2d_core::applications::ApplicationKey::attachment(
        occurrence,
        ambition_platformer2d_core::applications::module_of_entry(&submitted.entry),
    ))
}

/// The body holds its control under the submitting attachment's claim, or
/// that attachment lets its claim go. Another attachment's claim, and every
/// native hold, stay.
pub fn lower_control_holds(world: &mut World) {
    use ambition_characters::control::{claim_hold_as, release_hold_as};
    let submitted: Vec<_> = world.resource_mut::<ExtensionOutbox>().drain::<ControlHoldPort>(&BODY_CONTACT);
    for submitted in submitted {
        if world.get_entity(submitted.scope).is_err() {
            continue;
        }
        let Some(key) = submitter_key(world, &submitted) else {
            warn!("extension entry {} asked to hold a body, and its state has no live occurrence; refused", submitted.entry);
            continue;
        };
        let mut commands = world.commands();
        if submitted.value.held {
            claim_hold_as(&mut commands, submitted.scope, key);
        } else {
            release_hold_as(&mut commands, submitted.scope, key);
        }
        world.flush();
    }
}

/// The body is drawn in one row of its sheet under the submitting
/// attachment's pose claim, or that attachment withdraws its pose. A row the
/// game does not know is refused.
pub fn lower_pose_pins(world: &mut World) {
    use ambition_sprite_sheet::character::{claim_pose, release_pose, CharacterAnim, PoseTerm, SCRIPT_POSE_PRIORITY};
    let submitted: Vec<_> = world.resource_mut::<ExtensionOutbox>().drain::<PosePinPort>(&BODY_CONTACT);
    for submitted in submitted {
        if world.get_entity(submitted.scope).is_err() {
            continue;
        }
        let Some(key) = submitter_key(world, &submitted) else {
            warn!("extension entry {} asked to pose a body, and its state has no live occurrence; refused", submitted.entry);
            continue;
        };
        let mut commands = world.commands();
        match submitted.value.row.as_deref() {
            None => release_pose(&mut commands, submitted.scope, key),
            Some(row) => match CharacterAnim::from_name(row) {
                Some(anim) => claim_pose(&mut commands, submitted.scope, key, PoseTerm { anim, priority: SCRIPT_POSE_PRIORITY }),
                None => warn!("extension entry {} asked to pin the row {row:?}, which no sheet has; refused", submitted.entry),
            },
        }
        world.flush();
    }
}

/// Withdraw each hold and pose a body-contact attachment applied to its body
/// once that attachment is no longer bound there (its status ended, its
/// brain changed). A body-contact attachment keeps its records on the body
/// it is attached to, so its claims name the body's own live occurrence.
/// Runs on every tick of the phase, whether or not anything was submitted,
/// over the bodies that carry a claim. A claim another object issued is not
/// this phase's to judge, and a native claim is its owner's.
fn withdraw_unbound_attachment_claims(world: &mut World, mut state: Local<Option<AttachmentClaimsQuery>>) {
    let state = state.get_or_insert_with(|| world.query_filtered());
    let entries = &world.resource::<AdmittedExtensions>().0.entries;
    let withdrawn = unbound_claims(state.iter(world), entries);
    withdraw_claims(world, withdrawn);
}

/// The same withdrawal for one body, at the moment its statuses dropped a
/// kind: the ended application's claims go then, so a re-application later
/// on the same tick (another phase holds the kind again) starts with none of
/// them under its identical issuer key.
pub fn withdraw_unbound_attachment_claims_of(world: &mut World, body: Entity) {
    let Ok(body) = world.get_entity(body) else {
        return;
    };
    // The direct read uses the same visibility rules as the scheduled query.
    if world.get_resource::<DefaultQueryFilters>().is_some_and(|filters| {
        filters.disabling_ids().any(|id| body.contains_id(id))
    }) {
        return;
    }
    let Ok(row) = body.get_components::<AttachmentClaims>() else {
        return;
    };
    let entries = &world.resource::<AdmittedExtensions>().0.entries;
    let withdrawn = unbound_claims([row], entries);
    withdraw_claims(world, withdrawn);
}

type AttachmentClaims = (
    Entity,
    Option<&'static ambition_characters::control::HoldClaims>,
    Option<&'static ambition_sprite_sheet::character::PoseClaims>,
    &'static ambition_platformer2d_shared_tangle::lifecycle::ObjectOccurrence,
    &'static ambition_platformer2d_shared_tangle::lifecycle::SessionScopedEntity,
    Option<&'static super::status::BodyStatuses>,
    Option<&'static crate::actor_tuning::ActorConfig>,
);

type AttachmentClaimsQuery = QueryState<
    AttachmentClaims,
    Or<(
        With<ambition_characters::control::HoldClaims>,
        With<ambition_sprite_sheet::character::PoseClaims>,
    )>,
>;

type Withdrawal = (Entity, ambition_platformer2d_core::applications::ApplicationKey, bool);

fn unbound_claims<'w, 's>(
    rows: impl IntoIterator<Item = bevy::ecs::query::QueryItem<'w, 's, AttachmentClaims>>,
    entries: &[AdmittedEntry],
) -> Vec<Withdrawal> {
    use ambition_platformer2d_core::applications::module_of_entry;

    let mut withdrawn = Vec::new();
    for (body, holds, poses, occurrence, session, statuses, config) in rows {
        let own = occurrence.reference(session.0);
        let brain = config.and_then(|config| match &config.brain {
            ambition_entity_catalog::placements::CharacterBrain::Custom(key) => Some(key.as_str()),
            _ => None,
        });
        let bound = |module: &str| {
            entries.iter().any(|entry| {
                let trigger = &entry.descriptor.trigger;
                trigger.port == BodyContactPort::KEY
                    && module_of_entry(&entry.path) == module
                    && (brain == Some(trigger.selector.as_ref())
                        || statuses.is_some_and(|statuses| statuses.holds(trigger.selector.as_ref())))
            })
        };
        let claims = holds
            .into_iter()
            .flat_map(|claims| claims.0.iter().map(|(key, _)| (key, true)))
            .chain(poses.into_iter().flat_map(|claims| claims.0.iter().map(|(key, _)| (key, false))));
        for (key, hold) in claims {
            let Some((issuer, module)) = key.attachment_issuer() else {
                continue;
            };
            let mine = issuer.session == own.session && issuer.room == own.allocation_room && issuer.sequence == own.sequence;
            if mine && !bound(module) {
                withdrawn.push((body, key.clone(), hold));
            }
        }
    }
    withdrawn
}

fn withdraw_claims(world: &mut World, withdrawn: Vec<Withdrawal>) {
    use ambition_characters::control::release_hold_as;
    use ambition_sprite_sheet::character::release_pose;

    if withdrawn.is_empty() {
        return;
    }
    let mut commands = world.commands();
    for (body, key, hold) in withdrawn {
        if hold {
            release_hold_as(&mut commands, body, key);
        } else {
            release_pose(&mut commands, body, key);
        }
    }
    world.flush();
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
