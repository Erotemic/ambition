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
    HeldRay, HeldRayPort, RidingHitboxPort, RidingKnockback, StrikePort,
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
    // A ray is lowered with the held damage boxes (one record set): its own
    // install only offers the port.
    app.install_extension_request::<HeldRayPort, _>(TECHNIQUE_EXECUTION, "ambition_combat", || {});
    app.install_extension_request::<HeldDamageBoxPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_held_damage_boxes::<InTechniqueExecution>,
    );
}

/// Install the held damage box, the riding hitbox and the body hold in
/// `boss_conduct`.
pub fn install_for_boss_conduct(app: &mut App) {
    app.install_extension_request::<HeldRayPort, _>(BOSS_CONDUCT, "ambition_combat", || {});
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
    app.install_extension_request::<ambition_combat_port::SlowBodiesPort, _>(
        BOSS_CONDUCT,
        "ambition_combat",
        lower_slows,
    );
    app.install_extension_observation::<BodyAttachmentsPort>(BOSS_CONDUCT, "ambition_combat", body_attachments_of);
}

/// See the port card on `SlowBodiesPort`: each body in the box, in the
/// requester's live room, of any side, slowed from the next body step on.
#[allow(clippy::type_complexity)]
fn lower_slows(
    mut outbox: ResMut<ExtensionOutbox>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut bodies: Query<(
        Entity,
        &ae::BodyKinematics,
        &mut ambition_characters::actor::BodyCombat,
        Option<&ambition_characters::actor::BodyHealth>,
        Has<crate::death_rules::OutOfPlay>,
        Option<&ae::DepthPlane>,
    )>,
) {
    for submitted in outbox.drain::<ambition_combat_port::SlowBodiesPort>(&BOSS_CONDUCT) {
        let slow = submitted.value;
        let (centre, half) = (ae::Vec2::from(slow.center), ae::Vec2::from(slow.half_extent));
        let room = rooms.of(submitted.scope);
        for (body, kin, mut combat, health, out_of_play, plane) in &mut bodies {
            if body == submitted.scope
                || rooms.of(body) != room
                || crate::util::body_is_untouchable(health, out_of_play, plane)
            {
                continue;
            }
            let gap = (kin.pos - centre).abs() - (half + kin.size * 0.5);
            if gap.x < 0.0 && gap.y < 0.0 {
                combat.slow(slow.speed_scale, slow.seconds);
            }
        }
    }
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

/// One hit, this tick, by the body, on what is in the volume. The body is the
/// attacker, so the hit resolution reads its side.
fn lower_strikes(
    mut outbox: ResMut<ExtensionOutbox>,
    mut hits: MessageWriter<crate::events::HitEvent>,
    bodies: Query<&ae::BodyKinematics>,
) {
    use ambition_combat_port::{Place, StrikeVolume};
    for submitted in outbox.drain::<StrikePort>(&WIELDED_USE) {
        let strike = submitted.value;
        let body = bodies.get(submitted.scope).ok().map(|kin| [kin.pos.x, kin.pos.y]);
        // Where a place is: `Place::Body` is where the body is now, after a
        // transit of this tick.
        let at = |place: Place| match (place, body) {
            (Place::Body, None) => None,
            (place, body) => Some(ae::Vec2::from(place.resolve(body.unwrap_or_default()))),
        };
        let volume = match strike.volume {
            StrikeVolume::Circle { at: place, radius } => at(place).map(|c| ae::CombatVolume::circle(c, radius)),
            StrikeVolume::Span { from, to, pad } => at(from).zip(at(to)).map(|(from, to)| {
                let half = ae::Vec2::new((to.x - from.x).abs() * 0.5 + pad * 0.5, (to.y - from.y).abs() * 0.5 + pad * 0.5);
                ae::CombatVolume::from(ae::Aabb::new((from + to) * 0.5, half))
            }),
        };
        let Some(volume) = volume else {
            warn!("extension entry {} asked for a strike at a body with no position; refused", submitted.entry);
            continue;
        };
        let center = volume.center();
        hits.write(crate::events::HitEvent {
            pieces: Vec::new(),
            strike_sfx: None,
            volume,
            damage: strike.damage,
            source: crate::events::HitSource::Melee,
            attacker: Some(submitted.scope),
            room: None,
            target: crate::events::HitTarget::Volume,
            mode: crate::events::HitMode::Knockback,
            knockback: strike.knockback.map(|k| crate::events::HitKnockback {
                // An ordinary hit: it stuns.
                reaction: ae::hit_response::HitReaction::Strike,
                dir: k.dir,
                magnitude: crate::events::HitKnockbackMagnitude::FeelScale(k.feel_scale),
                source_pos: center,
                impact_pos: center,
                launch_dir: None,
                follow: None,
                guard: Default::default(),
            }),
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

/// One held volume a module asked for this tick: a box, or a ray cast now.
struct AskedHeld {
    scope: Entity,
    entry: Arc<str>,
    slot: u32,
    generation: u32,
    center: ae::Vec2,
    half_extent: ae::Vec2,
    shape: Option<ae::VolumeShape>,
    ray: Option<crate::strike::RayStrike>,
    damage: i32,
    knockback: f32,
    lifetime_s: f32,
}

/// Where `ray`, asked for by a body at `body`, goes: from its start along its
/// direction to the first wall, through the pairs it enters as far as the
/// body's portal policy lets a volume go, as a volume from its start (so the
/// pairs cut it where it goes through them). `None`: a body ray of a body
/// with no position, or no direction.
fn cast_ray(
    ray: &HeldRay,
    body: Option<ae::Vec2>,
    walls: Option<&ae::World>,
    passages: Option<&ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>,
    owner: Entity,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
) -> Option<(ae::Vec2, crate::strike::RayStrike, ae::VolumeShape)> {
    let origin = match ray.from {
        ambition_combat_port::Spot::World(at) => ae::Vec2::from(at),
        ambition_combat_port::Spot::Body(offset) => body? + ae::Vec2::from(offset),
    };
    let dir = ae::Vec2::from(ray.toward).normalize_or_zero();
    if dir == ae::Vec2::ZERO {
        return None;
    }
    let reach = ray.reach.max(0.0);
    let length = match walls {
        Some(walls) => {
            let (pairs, hops, convention) = match passages {
                Some(p) => (
                    p.pairs_in(room),
                    p.budget_for(Some(owner), room).map_or(0, |budget| u32::from(budget.max_hops)),
                    p.convention(),
                ),
                None => (Vec::new(), 0, Default::default()),
            };
            ae::cast::trace_through_apertures(walls, &pairs, origin, dir, reach, false, hops, convention).distance
        }
        None => reach,
    };
    let strike = crate::strike::RayStrike { dir, length, width: ray.width.max(1.0), look: Arc::from(ray.look.as_str()) };
    Some((origin, strike.clone(), strike.shape()))
}

/// See the port cards on `HeldDamageBoxPort` and `HeldRayPort`: release what
/// was not asked for this tick, move what is held and asked again, then spawn
/// what is asked for and not held. A ray is a held volume like a box, cast
/// again each tick it is asked for.
#[allow(clippy::too_many_arguments)]
fn lower_held_damage_boxes<P: LowersIn>(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    admitted: Res<AdmittedExtensions>,
    mut holders: Query<(Entity, &mut HeldDamageBoxes)>,
    mut hitboxes: Query<(&mut crate::strike::Hitbox, Option<&mut crate::strike::RayStrike>)>,
    factions: Query<(
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
    )>,
    bodies: Query<(&ae::BodyKinematics, Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>)>,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    passages: Option<Res<ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>>,
) {
    let mut asked: Vec<AskedHeld> = outbox
        .drain::<HeldDamageBoxPort>(&P::PHASE)
        .into_iter()
        .map(|s| AskedHeld {
            scope: s.scope,
            entry: s.entry.clone(),
            slot: s.value.slot,
            generation: s.value.generation,
            center: ae::Vec2::from(s.value.center),
            half_extent: ae::Vec2::from(s.value.half_extent),
            shape: None,
            ray: None,
            damage: s.value.damage,
            knockback: s.value.knockback,
            lifetime_s: s.value.lifetime_s,
        })
        .collect();
    for s in outbox.drain::<HeldRayPort>(&P::PHASE) {
        let (body, room) = bodies.get(s.scope).map_or((None, None), |(kin, room)| (Some(kin.pos), room));
        let walls = collision.room(room).and_then(|room| room.solids());
        let Some((origin, ray, shape)) =
            cast_ray(&s.value, body, walls.as_deref(), passages.as_deref(), s.scope, room.map(|r| r.0))
        else {
            warn!("extension entry {} asked for a ray from {:?} that has no start or no direction; refused", s.entry, s.scope);
            continue;
        };
        asked.push(AskedHeld {
            scope: s.scope,
            entry: s.entry.clone(),
            slot: s.value.slot,
            generation: s.value.generation,
            center: origin,
            half_extent: shape_half(&shape),
            shape: Some(shape),
            ray: Some(ray),
            damage: s.value.damage,
            knockback: s.value.knockback,
            lifetime_s: s.value.lifetime_s,
        });
    }
    // Only this phase's entries' boxes: the other phase's adapter releases
    // its own.
    let mine: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.phase == P::PHASE)
        .map(|e| e.path.as_str())
        .collect();
    let is_asked = |owner: Entity, r: &HeldDamageBoxRecord| {
        !mine.contains(&&*r.entry)
            || asked.iter().any(|s| {
                s.scope == owner && *s.entry == *r.entry && s.slot == r.slot && s.generation == r.generation
            })
    };
    for (owner, mut held) in &mut holders {
        if held.0.iter().all(|r| is_asked(owner, r)) {
            continue;
        }
        held.0.retain(|r| {
            let keep = is_asked(owner, r);
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
    for s in &asked {
        let held = holders.get(s.scope).ok().map(|(_, h)| h);
        let pending = new.iter().find(|(o, _)| *o == s.scope).map(|(_, r)| r);
        let is_held = held
            .into_iter()
            .flat_map(|h| h.0.iter())
            .chain(pending.into_iter().flatten())
            .any(|r| *r.entry == *s.entry && r.slot == s.slot && r.generation == s.generation);
        if is_held {
            // Held: it follows its submitted centre (version 2), and a ray
            // its new cast.
            let entity = held
                .into_iter()
                .flat_map(|h| h.0.iter())
                .find(|r| *r.entry == *s.entry && r.slot == s.slot && r.generation == s.generation)
                .map(|r| r.entity);
            if let Some((mut hitbox, ray)) = entity.and_then(|e| hitboxes.get_mut(e).ok()) {
                hitbox.anchor = crate::strike::HitboxAnchor::World { center: s.center };
                if let (Some(mut held_ray), Some(cast)) = (ray, &s.ray) {
                    hitbox.shape = s.shape.clone();
                    hitbox.half_extent = s.half_extent;
                    *held_ray = cast.clone();
                }
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
        let entity = crate::strike::spawn_damage_box(
            &mut commands,
            s.scope,
            crate::hit_side_from_actor_faction(faction),
            s.center,
            crate::strike::DamageBox {
                half_extent: s.half_extent,
                shape: s.shape.clone(),
                damage: s.damage,
                knockback: s.knockback,
                lifetime_s: s.lifetime_s,
                name: None,
            },
        );
        if let Some(ray) = &s.ray {
            commands.entity(entity).insert(ray.clone());
        }
        let record = HeldDamageBoxRecord {
            entry: s.entry.clone(),
            slot: s.slot,
            generation: s.generation,
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

/// The half extent of a shape's bounds about its origin: what a box of the
/// shape reports where a reader wants a box.
fn shape_half(shape: &ae::VolumeShape) -> ae::Vec2 {
    let bounds = shape.place_at(ae::Vec2::ZERO, 1.0, ae::Vec2::new(0.0, 1.0)).bounds();
    bounds.max.abs().max(bounds.min.abs())
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
                guard: Default::default(),
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
                    if combat.get(victim.entity).is_ok_and(|c| c.is_knocked()) {
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

pub mod contact;
pub mod special;
pub mod status;
pub mod targets;
pub mod think;
pub use contact::install_for_body_contact;
pub use special::install_for_specials;
pub use think::install_for_actor_think;

#[cfg(test)]
mod tests;
