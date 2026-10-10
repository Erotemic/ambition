//! The combat domain's extension request adapters.
//!
//! A module's damage box enters the effect executor's one road
//! (`EffectRequest` → `Effect::DamageBox`). A module's HELD damage box is an
//! entity this adapter spawns, keeps while the module asks for it, and
//! despawns ([`HeldDamageBoxes`]). Each box is owned by the body the
//! invocation ran for and is on that body's EFFECTIVE faction
//! (`targeting::effective_faction`: a driven body fights as the player). See
//! the port cards on `ambition_combat_port`.

use std::sync::Arc;

use ambition_combat_port::{
    BodyAttachment, BodyAttachments, BodyAttachmentsPort, BodyHold, BodyHoldPort, DamageBoxPort, HeldDamageBoxPort,
    RidingHitboxPort, RidingKnockback, StrikePort,
};
use ambition_extension_host::{
    AdmittedExtensions, ExtensionAppExt, ExtensionOutbox, InBossConduct, InTechniqueExecution, InWieldedUse, LowersIn,
};
use ambition_extension_sdk::phases::{BOSS_CONDUCT, TECHNIQUE_EXECUTION, WIELDED_USE};
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_characters::actor::ActorFaction;

/// Install the request port in `technique_execution`, before the effect
/// executor.
pub fn install(app: &mut App) {
    app.install_extension_request::<DamageBoxPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_damage_boxes::<InTechniqueExecution>,
    );
    app.install_extension_request::<HeldDamageBoxPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_held_damage_boxes::<InTechniqueExecution>,
    );
}

/// Install the held damage box, the riding hitbox and the body hold in
/// `boss_conduct`.
pub fn install_for_boss_conduct(app: &mut App) {
    app.install_extension_request::<HeldDamageBoxPort, _>(
        BOSS_CONDUCT,
        "ambition_combat",
        lower_held_damage_boxes::<InBossConduct>,
    );
    app.install_extension_request::<RidingHitboxPort, _>(BOSS_CONDUCT, "ambition_combat", lower_riding_hitboxes);
    // The hold adapter writes the capture road's requests: a composition with
    // the port has them, whether or not it ever grabs.
    app.add_message::<crate::capture::CaptureCarryRequested>();
    app.add_message::<crate::capture::CapturePummelRequested>();
    app.add_message::<crate::capture::CaptureThrowRequested>();
    app.install_extension_request::<BodyHoldPort, _>(BOSS_CONDUCT, "ambition_combat", lower_body_holds);
    app.install_extension_observation::<BodyAttachmentsPort>(BOSS_CONDUCT, "ambition_combat", body_attachments_of);
}

/// The named points of `scope`'s body rig, from the pose its hurt parts have:
/// a jaw, a saddle, a grip. See `ambition_combat_port::BodyAttachmentsPort`.
///
/// The art package states each point on a joint and the rig's pose places it
/// (`resolve_body_rig_poses`). This gives each one from the body's position,
/// in the body's local frame, which is the frame a module writes a hold
/// offset in. No value for a body with no rig, or one not posed yet.
fn body_attachments_of(world: &World, scope: Entity) -> Option<BodyAttachments> {
    let rig = world.get::<crate::body_rig::BodyRig>(scope)?;
    let pose = world.get::<crate::body_rig::BodyRigPose>(scope)?;
    let feet = crate::body_rig::rig_feet_from_centre(
        world.get::<crate::body_rig::RigFeetOffset>(scope),
        world.get::<ae::BodyKinematics>(scope),
    );
    let points = pose
        .attachments_from_centre(&rig.0, feet)?
        .map(|(name, at)| BodyAttachment {
            name: name.to_owned(),
            offset: at.into(),
        })
        .collect();
    Some(BodyAttachments { points })
}

/// Install the damage box port and the strike port in `wielded_use` (a held
/// item's use), before the effect executor and the hit resolution, as in
/// `technique_execution`.
pub fn install_for_wielded_use(app: &mut App) {
    app.install_extension_request::<DamageBoxPort, _>(
        WIELDED_USE,
        "ambition_combat",
        lower_damage_boxes::<InWieldedUse>,
    );
    // The strike adapter writes the hit message: a composition with the port
    // has it, whether or not a module strikes.
    app.add_message::<crate::events::HitEvent>();
    app.install_extension_request::<StrikePort, _>(WIELDED_USE, "ambition_combat", lower_strikes);
}

/// One hit, this tick, by the body, on what is in the circle. The body is the
/// attacker, so the hit resolution reads its side.
fn lower_strikes(
    mut outbox: ResMut<ExtensionOutbox>,
    mut hits: MessageWriter<crate::events::HitEvent>,
    bodies: Query<&ae::BodyKinematics>,
) {
    for submitted in outbox.drain::<StrikePort>(&WIELDED_USE) {
        let strike = submitted.value;
        let body = bodies.get(submitted.scope).ok().map(|kin| [kin.pos.x, kin.pos.y]);
        let at = match (strike.at, body) {
            (ambition_combat_port::Place::Body, None) => {
                warn!("extension entry {} asked for a strike at a body with no position; refused", submitted.entry);
                continue;
            }
            (place, body) => place.resolve(body.unwrap_or_default()),
        };
        hits.write(crate::events::HitEvent {
            strike_sfx: None,
            volume: ae::CombatVolume::circle(ae::Vec2::from(at), strike.radius),
            damage: strike.damage,
            source: crate::events::HitSource::Melee,
            attacker: Some(submitted.scope),
            room: None,
            target: crate::events::HitTarget::Volume,
            mode: crate::events::HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
            attacker_move_instance: None,
        });
    }
}


fn lower_damage_boxes<L: LowersIn>(
    mut outbox: ResMut<ExtensionOutbox>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    factions: Query<(
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
    )>,
) {
    for submitted in outbox.drain::<DamageBoxPort>(&L::PHASE) {
        // ⛔ SUBMITTED IS NOT APPLIED. A body with no faction cannot say whom
        // its box hurts, and the module may not say it either.
        let Ok((authored, driver)) = factions.get(submitted.scope) else {
            warn!(
                "extension entry {} asked for a damage box for {:?}, which has no faction; refused",
                submitted.entry, submitted.scope
            );
            continue;
        };
        let faction = crate::targeting::effective_faction(*authored, driver);
        let b = submitted.value;
        effects.write(ambition_vfx::EffectRequest {
            owner: submitted.scope,
            effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                center: ae::Vec2::from(b.center),
                faction: crate::hit_side_from_actor_faction(faction),
                half_extent: ae::Vec2::from(b.half_extent),
                damage: b.damage,
                knockback: b.knockback,
                lifetime_s: b.lifetime_s,
                name: None,
            }),
        });
    }
}

/// The held damage boxes of one body: the box entities the held-box adapter
/// spawned for it, by entry and slot. The module never sees these entities.
///
/// Rollback state: a rewind must restore which box each slot holds, or the
/// replay spawns a second box. The entity references are mapped
/// (`map.combat.held_damage_boxes`); the checksum reads the slots and
/// generations, not the allocator-local entities.
#[derive(Component, Clone, Debug, Default)]
pub struct HeldDamageBoxes(pub Vec<HeldDamageBoxRecord>);

#[derive(Clone, Debug)]
pub struct HeldDamageBoxRecord {
    /// `provider::module/entry`: slots are per entry.
    pub entry: Arc<str>,
    pub slot: u32,
    pub generation: u32,
    /// The box. It can be gone already: a box that ran out of lifetime is not
    /// spawned again for the same generation.
    pub entity: Entity,
}

impl bevy::ecs::entity::MapEntities for HeldDamageBoxes {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        for record in &mut self.0 {
            record.entity = mapper.get_mapped(record.entity);
        }
    }
}

impl ae::snapshot::SnapshotCursor for HeldDamageBoxes {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        ae::snapshot::put_u32(out, self.0.len() as u32);
        for record in &self.0 {
            ae::snapshot::put_str(out, &record.entry);
            ae::snapshot::put_u32(out, record.slot);
            ae::snapshot::put_u32(out, record.generation);
        }
    }
}

/// See the port card on `HeldDamageBoxPort`: release what was not asked for
/// this tick, then spawn what is asked for and not held.
fn lower_held_damage_boxes<P: LowersIn>(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    admitted: Res<AdmittedExtensions>,
    mut holders: Query<(Entity, &mut HeldDamageBoxes)>,
    mut hitboxes: Query<&mut crate::strike::Hitbox>,
    factions: Query<(
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
    )>,
) {
    let submitted = outbox.drain::<HeldDamageBoxPort>(&P::PHASE);
    // Only this phase's entries' boxes: the other phase's adapter releases
    // its own.
    let mine: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.phase == P::PHASE)
        .map(|e| e.path.as_str())
        .collect();
    let asked = |owner: Entity, r: &HeldDamageBoxRecord| {
        !mine.contains(&&*r.entry)
            || submitted.iter().any(|s| {
                s.scope == owner
                    && *s.entry == *r.entry
                    && s.value.slot == r.slot
                    && s.value.generation == r.generation
            })
    };
    for (owner, mut held) in &mut holders {
        if held.0.iter().all(|r| asked(owner, r)) {
            continue;
        }
        held.0.retain(|r| {
            let keep = asked(owner, r);
            if !keep {
                if let Ok(mut e) = commands.get_entity(r.entity) {
                    e.try_despawn();
                }
            }
            keep
        });
    }

    // New boxes, per owner, in submit order. An owner that holds nothing yet
    // gets the component with them.
    let mut new: Vec<(Entity, Vec<HeldDamageBoxRecord>)> = Vec::new();
    for s in &submitted {
        let held = holders.get(s.scope).ok().map(|(_, h)| h);
        let pending = new.iter().find(|(o, _)| *o == s.scope).map(|(_, r)| r);
        let is_held = held
            .into_iter()
            .flat_map(|h| h.0.iter())
            .chain(pending.into_iter().flatten())
            .any(|r| *r.entry == *s.entry && r.slot == s.value.slot && r.generation == s.value.generation);
        if is_held {
            // Held: it follows its submitted centre (version 2).
            let entity = held
                .into_iter()
                .flat_map(|h| h.0.iter())
                .find(|r| *r.entry == *s.entry && r.slot == s.value.slot && r.generation == s.value.generation)
                .map(|r| r.entity);
            if let Some(mut hitbox) = entity.and_then(|e| hitboxes.get_mut(e).ok()) {
                hitbox.anchor = crate::strike::HitboxAnchor::World {
                    center: ae::Vec2::from(s.value.center),
                };
            }
            continue;
        }
        // ⛔ SUBMITTED IS NOT APPLIED: a body with no faction cannot say whom
        // its box hurts.
        let Ok((authored, driver)) = factions.get(s.scope) else {
            warn!(
                "extension entry {} asked for a held damage box for {:?}, which has no faction; refused",
                s.entry, s.scope
            );
            continue;
        };
        let faction = crate::targeting::effective_faction(*authored, driver);
        let b = &s.value;
        let entity = crate::strike::spawn_damage_box(
            &mut commands,
            s.scope,
            crate::hit_side_from_actor_faction(faction),
            ae::Vec2::from(b.center),
            crate::strike::DamageBox {
                half_extent: ae::Vec2::from(b.half_extent),
                shape: None,
                damage: b.damage,
                knockback: b.knockback,
                lifetime_s: b.lifetime_s,
                name: None,
            },
        );
        let record = HeldDamageBoxRecord {
            entry: s.entry.clone(),
            slot: b.slot,
            generation: b.generation,
            entity,
        };
        match new.iter_mut().find(|(o, _)| *o == s.scope) {
            Some((_, records)) => records.push(record),
            None => new.push((s.scope, vec![record])),
        }
    }
    for (owner, records) in new {
        // A same-slot box of another generation was released above.
        if let Ok((_, mut held)) = holders.get_mut(owner) {
            held.0.extend(records);
        } else {
            commands.entity(owner).insert(HeldDamageBoxes(records));
        }
    }
}

/// Lower `ambition.combat.riding_hitbox`: a hitbox that follows its owner, on
/// the owner's effective side, shown by the owner's own art.
fn lower_riding_hitboxes(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    factions: Query<(
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
    )>,
) {
    for s in outbox.drain::<RidingHitboxPort>(&BOSS_CONDUCT) {
        // ⛔ SUBMITTED IS NOT APPLIED: a body with no faction cannot say whom
        // its hitbox hurts.
        let Ok((authored, driver)) = factions.get(s.scope) else {
            warn!(
                "extension entry {} asked for a riding hitbox for {:?}, which has no faction; refused",
                s.entry, s.scope
            );
            continue;
        };
        let h = s.value;
        let side = crate::hit_side_from_actor_faction(crate::targeting::effective_faction(*authored, driver));
        commands.spawn((
            crate::strike::Hitbox {
                strike_sfx: None,
                owner: s.scope,
                source: side,
                anchor: crate::strike::HitboxAnchor::FollowOwner {
                    local_offset: ae::Vec2::from(h.offset),
                },
                half_extent: ae::Vec2::from(h.half_extent),
                shape: h.circle_radius.map(|radius| ae::VolumeShape::Circle { radius }),
                facing: 1.0,
                damage: h.damage,
                knockback: match h.knockback {
                    RidingKnockback::FeelScale(f) => crate::strike::HitboxKnockback::FeelScale(f),
                    RidingKnockback::LaunchSpeed { base, growth } => {
                        crate::strike::HitboxKnockback::LaunchSpeed { base, growth }
                    }
                },
                // The port's card: world units, +Y down, `x` away from the
                // source. The owner's facing is not asked.
                launch_dir: h
                    .launch_dir
                    .map(|dir| crate::strike::HitboxLaunch::AwayFromSource(ae::Vec2::from(dir))),
                frame_down: ae::Vec2::new(0.0, 1.0),
                reaction: None,
            },
            crate::strike::HitboxLifetime { remaining_s: h.lifetime_s },
            crate::strike::HitboxHits::default(),
            crate::strike::DepictedByOwner,
            Name::new(h.name),
        ));
    }
}

/// The reach volume of a seize in world space: its centre and its half.
///
/// `reach_offset` and `reach_half` are on the captor's own axes (`+x` the way
/// it faces, `+y` toward its feet), the frame the port card states for every
/// geometry of a hold. They are lowered as a body-local volume of a move is:
/// mirror by the facing, then turn into the frame whose DOWN is `down`.
fn seize_reach(
    captor: &ae::BodyKinematics,
    down: ae::Vec2,
    reach_offset: ae::Vec2,
    reach_half: ae::Vec2,
) -> (ae::Vec2, ae::Vec2) {
    let frame = ae::AccelerationFrame::new(down);
    let facing = if captor.facing < 0.0 { -1.0 } else { 1.0 };
    let centre = captor.pos + frame.to_world(ae::Vec2::new(reach_offset.x * facing, reach_offset.y));
    (centre, frame.to_world_half(reach_half))
}

/// A module's hold on another body: the engine's capture relation
/// (`crate::capture`), the same grant, carry, pummel, throw and release a
/// fighter's grab uses. See `ambition_combat_port::BodyHoldPort`.
///
/// A seize reaches like a grab (one victim, the nearest to the reach's
/// centre, ties by `SimId`; never a body already held or holding, a corpse,
/// an intangible body, a body in another live room, or one its damage does
/// not land on) without a grab's two FIGHTER requirements: the captor need
/// not stand on the floor (a boss's pose is its module's), and the victim
/// need not carry a fighter's surface state (the Ambition player does not).
/// A victim in hitstun is not seized: the capture road releases a captive
/// that reacts to a hit (`release_interrupted_captures`), so it would be let
/// go on the tick it was caught.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
/// The attachment of `captor`'s body rig that a hold names, as the capture
/// relation keeps it (`CapturedBy::hold_attachment`). `Ok(None)`: the hold
/// names no point. `Err`: it names a point this body's rig does not state (or
/// the body has no rig), and the hold is refused: a held body at the wrong
/// point is worse than no hold.
fn hold_attachment(
    rigs: &Query<&crate::body_rig::BodyRig>,
    captor: Entity,
    hold_at: Option<&str>,
) -> Result<Option<u16>, ()> {
    let Some(name) = hold_at else {
        return Ok(None);
    };
    rigs.get(captor)
        .ok()
        .and_then(|rig| rig.0.attachment_index(name))
        .and_then(|index| u16::try_from(index).ok())
        .map(Some)
        .ok_or(())
}

fn lower_body_holds(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    captors: Query<(
        &ae::BodyKinematics,
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
        Option<&crate::targeting::MatchTeam>,
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    )>,
    rigs: Query<&crate::body_rig::BodyRig>,
    victims: Query<crate::hitbox::StrikeVictim, Without<ambition_characters::control::ControlHolds>>,
    captives: Query<(Entity, &crate::capture::CapturedBy)>,
    combat: Query<&ambition_characters::actor::BodyCombat>,
    identities: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    mut playbacks: Query<&mut crate::moveset::MovePlayback>,
    mut budgets: Query<(
        &ae::BodyAbilities,
        &mut ae::BodyJumpState,
        &mut ae::BodyDodgeState,
        &ae::MotionModel,
    )>,
    mut grounds: Query<&mut ae::BodyGroundState>,
    mut holds: Query<&mut ambition_characters::control::ControlHolds>,
    mut carries: MessageWriter<crate::capture::CaptureCarryRequested>,
    mut pummels: MessageWriter<crate::capture::CapturePummelRequested>,
    mut throws: MessageWriter<crate::capture::CaptureThrowRequested>,
    tuning: crate::rules::CombatTuningOf,
) {
    // One seize per captor per tick, and a captor holds one body.
    let mut seized: Vec<Entity> = Vec::new();
    for s in outbox.drain::<BodyHoldPort>(&BOSS_CONDUCT) {
        let captor = s.scope;
        let held = crate::capture::captive_of(captor, &captives).or_else(|| {
            seized.contains(&captor).then_some(Entity::PLACEHOLDER)
        });
        match s.value {
            BodyHold::Seize { reach_offset, reach_half, hold_at, hold_offset, hold_s } => {
                if held.is_some() {
                    continue;
                }
                let Ok(hold_attachment) = hold_attachment(&rigs, captor, hold_at.as_deref()) else {
                    warn!(
                        "extension entry {} asked to seize at `{}`, a point the body rig of {captor:?} does not state; refused",
                        s.entry,
                        hold_at.as_deref().unwrap_or_default()
                    );
                    continue;
                };
                let Ok((kin, faction, driver, team, captor_frame)) = captors.get(captor) else {
                    warn!("extension entry {} asked {:?}, which is no body, to seize; refused", s.entry, captor);
                    continue;
                };
                let room = tuning.room_of(captor);
                let friendly_fire = tuning.in_room(room).unwrap_or_default().friendly_fire();
                let attacker = crate::targeting::effective_faction(*faction, driver);
                // The reach is a box on the captor's own axes, as the hold
                // point is (`constrain_captives`): mirror by its facing, then
                // turn into its frame. A captor under sideways gravity
                // reaches along its own floor, and its box lies along it.
                let (centre, half) = seize_reach(
                    kin,
                    captor_frame.map_or(ae::DEFAULT_GRAVITY_DIR, |frame| frame.down()),
                    ae::Vec2::from(reach_offset),
                    ae::Vec2::from(reach_half),
                );
                let reach = ae::CenteredAabb::new(centre, half).aabb();
                let already: std::collections::HashSet<Entity> = captives
                    .iter()
                    .flat_map(|(victim, held)| [victim, held.captor])
                    .chain(seized.iter().copied())
                    .collect();
                let mut candidates: Vec<(f32, &ambition_platformer2d_shared_tangle::sim_id::SimId, Entity)> =
                    Vec::new();
                for victim in &victims {
                    if victim.entity == captor || already.contains(&victim.entity) || tuning.room_of(victim.entity) != room {
                        continue;
                    }
                    if victim.is_corpse() || victim.is_intangible() {
                        continue;
                    }
                    if combat.get(victim.entity).is_ok_and(|c| c.hitstun_timer > 0.0 || c.recoil_lock_timer > 0.0) {
                        continue;
                    }
                    if !crate::targeting::damage_lands_between(
                        attacker,
                        victim.effective_faction(),
                        team,
                        victim.team,
                        friendly_fire,
                        None,
                        victim.entity,
                    ) {
                        continue;
                    }
                    let body = victim.aabb.aabb();
                    if !bevy::math::bounding::IntersectsVolume::intersects(&reach, &body) {
                        continue;
                    }
                    let Ok(id) = identities.get(victim.entity) else {
                        continue;
                    };
                    candidates.push((ae::AabbExt::center(body).distance_squared(centre), id, victim.entity));
                }
                candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)));
                let Some(&(_, _, victim)) = candidates.first() else {
                    continue;
                };
                seized.push(captor);
                crate::capture::systems::begin_capture(
                    &mut commands,
                    captor,
                    victim,
                    ae::Vec2::from(hold_offset),
                    hold_attachment,
                    hold_s,
                    playbacks.get_mut(victim).ok(),
                    budgets.get_mut(victim).ok(),
                );
            }
            BodyHold::Carry { hold_at, hold_offset } => {
                if held.is_none() {
                    continue;
                }
                let Ok(hold_attachment) = hold_attachment(&rigs, captor, hold_at.as_deref()) else {
                    warn!(
                        "extension entry {} asked to carry at `{}`, a point the body rig of {captor:?} does not state; refused",
                        s.entry,
                        hold_at.as_deref().unwrap_or_default()
                    );
                    continue;
                };
                carries.write(crate::capture::CaptureCarryRequested {
                    captor,
                    hold_offset: ae::Vec2::from(hold_offset),
                    hold_attachment,
                });
            }
            BodyHold::Pummel { damage } => {
                if held.is_some() {
                    pummels.write(crate::capture::CapturePummelRequested { captor, damage });
                }
            }
            BodyHold::Throw { damage, knockback, growth, launch_dir } => {
                if held.is_some() {
                    throws.write(crate::capture::CaptureThrowRequested {
                        captor,
                        damage,
                        knockback,
                        knockback_growth: growth,
                        launch_dir: ae::Vec2::from(launch_dir),
                        move_instance: None,
                    });
                }
            }
            BodyHold::Release => {
                if let Some(victim) = crate::capture::captive_of(captor, &captives) {
                    crate::capture::systems::release_capture(
                        &mut commands,
                        victim,
                        grounds.get_mut(victim).ok().as_deref_mut(),
                        holds.get_mut(victim).ok().as_deref_mut(),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
