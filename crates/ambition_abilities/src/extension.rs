//! The held-item domain's extension adapters (fast-iteration I7: a wielded
//! ability as a procedural module).
//!
//! The trigger: for each item id an admitted entry is bound to, one
//! [`WieldedUsePort`] invocation for each body that holds that item, each tick,
//! with the body's resolved control (press, aim), kinematics, gravity frame
//! and mana. The observations: the body's mark, and the first wall along its
//! aim. The requests: move the body (a transit) or set its velocity, arm the
//! shared movement cooldown, pay mana from the body's bank, play a sound as
//! the body, and show an effect, a burst or a hit mark. See the port cards in
//! `ambition_combat_port::wielded` and `ambition_combat_port::motion`.

use ambition_characters::control::ActorControl;
use ambition_combat::held_items::HeldItem;
use ambition_combat_port::{
    AimCast, AimCastPort, AimHit, BodySoundPort, BurstPort, EffectPort, EndModuleEntityPort, HitMarkPort, MarkPort,
    MarkView, ModuleEntityTickPort, MovementCooldownPort, PullBodiesPort, SetMarkPort, SetVelocityPort,
    SpawnModuleEntityPort, SpendManaPort, WieldedAlternatePort, WieldedUsePort, Wielder, AIM_CAST_REACH,
};
/// The composition orders this port's lowering as a carry of the travelled
/// path.
pub use ambition_combat_port::TransitPort;
use ambition_extension_host::{
    AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox, InBossConduct, InModuleEntityTick,
    InWieldedUse, LowersIn,
};
use ambition_extension_sdk::phases::{BOSS_CONDUCT, MODULE_ENTITY_TICK, WIELDED_USE};
use ambition_extension_sdk::Port;
use ambition_platformer2d_core::resources::ActorResources;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

/// Install the trigger port and the request ports in `wielded_use`.
pub fn install(app: &mut App) {
    app.install_extension_trigger::<WieldedUsePort, _>(WIELDED_USE, "ambition_abilities", queue_wielded_uses);
    app.install_extension_trigger::<WieldedAlternatePort, _>(
        WIELDED_USE,
        "ambition_abilities",
        queue_wielded_alternates,
    );
    app.install_extension_observation::<MarkPort>(WIELDED_USE, "ambition_abilities", mark_of);
    app.install_extension_observation::<AimCastPort>(WIELDED_USE, "ambition_abilities", aim_cast_of);
    // First of the phase's request ports: the host lowers them in install
    // order, so a `Place::Body` of a later port is the arrival.
    app.install_extension_request::<TransitPort, _>(WIELDED_USE, "ambition_abilities", lower_transits);
    app.install_extension_request::<MovementCooldownPort, _>(
        WIELDED_USE,
        "ambition_abilities",
        lower_movement_cooldowns,
    );
    // The effect adapter writes the room-bound effect message: a composition
    // with the port has it, whether or not a module shows an effect.
    app.add_message::<ambition_vfx::vfx::VfxInRoom>();
    app.install_extension_request::<EffectPort, _>(WIELDED_USE, "ambition_abilities", lower_effects);
    app.install_extension_request::<BurstPort, _>(WIELDED_USE, "ambition_abilities", lower_wielded_bursts);
    app.install_extension_request::<HitMarkPort, _>(WIELDED_USE, "ambition_abilities", lower_hit_marks);
    app.install_extension_request::<SetVelocityPort, _>(WIELDED_USE, "ambition_abilities", lower_velocities);
    app.install_extension_request::<SetMarkPort, _>(WIELDED_USE, "ambition_abilities", lower_set_marks);
    app.install_extension_request::<SpendManaPort, _>(WIELDED_USE, "ambition_abilities", lower_mana_spends);
    app.install_extension_request::<BodySoundPort, _>(
        WIELDED_USE,
        "ambition_abilities",
        lower_body_sounds::<InWieldedUse>,
    );
    app.install_extension_request::<SpawnModuleEntityPort, _>(
        WIELDED_USE,
        "ambition_abilities",
        crate::module_entity::lower_module_entity_spawns,
    );
}

/// Install the module-entity trigger port and its request ports in
/// `module_entity_tick`: a sound (a module entity is heard as itself), a pull,
/// and the end of the entity.
pub fn install_module_entities(app: &mut App) {
    app.install_extension_trigger::<ModuleEntityTickPort, _>(
        MODULE_ENTITY_TICK,
        "ambition_abilities",
        crate::module_entity::queue_module_entity_ticks,
    );
    app.install_extension_request::<BodySoundPort, _>(
        MODULE_ENTITY_TICK,
        "ambition_abilities",
        lower_body_sounds::<InModuleEntityTick>,
    );
    app.install_extension_request::<PullBodiesPort, _>(
        MODULE_ENTITY_TICK,
        "ambition_abilities",
        crate::module_entity::lower_body_pulls,
    );
    app.install_extension_request::<EndModuleEntityPort, _>(
        MODULE_ENTITY_TICK,
        "ambition_abilities",
        crate::module_entity::lower_module_entity_ends,
    );
}

/// Install the sound request port in `boss_conduct` (a conducted boss is
/// heard as itself).
pub fn install_for_boss_conduct(app: &mut App) {
    app.install_extension_request::<BodySoundPort, _>(
        BOSS_CONDUCT,
        "ambition_abilities",
        lower_body_sounds::<InBossConduct>,
    );
}

/// The bodies that hold an item an entry of `port` is bound to, in an order
/// a rewind reproduces (the body's simulation identity, then its entity),
/// each with its trigger value.
#[allow(clippy::type_complexity)]
fn wielders_bound_to(
    port: &ambition_extension_sdk::PortKey,
    admitted: &AdmittedExtensions,
    driven: &ambition_held_items::DrivenBodies,
    wielders: &Query<(
        Entity,
        &ActorControl,
        &HeldItem,
        &BodyKinematics,
        &ResolvedMotionFrame,
        Option<&ActorResources>,
        Option<&SimId>,
        Has<ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
        Option<&crate::ability_cooldown::AbilityCooldown>,
        Option<&ambition_platformer2d_core::movement::MotionModel>,
    )>,
) -> Vec<(Entity, String, ambition_characters::actor::control::ActorControlFrame, Wielder)> {
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == *port)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .collect();
    if bound.is_empty() {
        return Vec::new();
    }
    let driven = driven.entities();
    let mut using: Vec<_> = wielders
        .iter()
        .filter(|(_, _, held, ..)| bound.contains(&held.id()))
        .collect();
    using.sort_by(|a, b| {
        (a.6.map(SimId::as_str), a.0.to_bits()).cmp(&(b.6.map(SimId::as_str), b.0.to_bits()))
    });
    using
        .into_iter()
        .map(|(entity, control, held, kin, frame, bank, id, counts, cooldown, model)| {
            let c = control.0;
            let basis = frame.basis();
            let aim = ambition_held_items::ability_aim_local(&c, kin.facing);
            let wielder = Wielder {
                pressed: c.melee_pressed && !c.shield_held,
                driven: driven.contains(&entity),
                position: [kin.pos.x, kin.pos.y],
                size: [kin.size.x, kin.size.y],
                facing: kin.facing,
                frame_side: [basis.side.x, basis.side.y],
                frame_down: [basis.down.x, basis.down.y],
                aim_local: [aim.x, aim.y],
                mana: crate::mana::level(bank).map(|level| level.current),
                names_spawns: id.is_some() && counts,
                cooldown_ready: cooldown.is_none_or(|cooldown| cooldown.ready()),
                swept: matches!(model, Some(ambition_platformer2d_core::movement::MotionModel::AxisSwept(_))),
            };
            (entity, held.id().to_owned(), c, wielder)
        })
        .collect()
}

/// One invocation for each body holding a bound item: IDLE when Attack is
/// not pressed.
#[allow(clippy::type_complexity)]
pub fn queue_wielded_uses(
    admitted: Res<AdmittedExtensions>,
    mut invocations: ResMut<ExtensionInvocations>,
    driven: ambition_held_items::DrivenBodies,
    wielders: Query<(
        Entity,
        &ActorControl,
        &HeldItem,
        &BodyKinematics,
        &ResolvedMotionFrame,
        Option<&ActorResources>,
        Option<&SimId>,
        Has<ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
        Option<&crate::ability_cooldown::AbilityCooldown>,
        Option<&ambition_platformer2d_core::movement::MotionModel>,
    )>,
) {
    for (entity, item, _, wielder) in wielders_bound_to(&WieldedUsePort::KEY, &admitted, &driven, &wielders) {
        let idle = !wielder.pressed;
        invocations.trigger::<WieldedUsePort>(&WIELDED_USE, item, entity, None, idle, wielder);
    }
}

/// One invocation for each body holding a bound item: IDLE when Blink is not
/// pressed.
#[allow(clippy::type_complexity)]
pub fn queue_wielded_alternates(
    admitted: Res<AdmittedExtensions>,
    mut invocations: ResMut<ExtensionInvocations>,
    driven: ambition_held_items::DrivenBodies,
    wielders: Query<(
        Entity,
        &ActorControl,
        &HeldItem,
        &BodyKinematics,
        &ResolvedMotionFrame,
        Option<&ActorResources>,
        Option<&SimId>,
        Has<ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
        Option<&crate::ability_cooldown::AbilityCooldown>,
        Option<&ambition_platformer2d_core::movement::MotionModel>,
    )>,
) {
    for (entity, item, control, wielder) in wielders_bound_to(&WieldedAlternatePort::KEY, &admitted, &driven, &wielders) {
        let idle = !control.blink_pressed;
        invocations.trigger::<WieldedAlternatePort>(&WIELDED_USE, item, entity, None, idle, wielder);
    }
}

/// The body's mark, if it is in the live room the body is in now: a mark is
/// a place in one live room.
fn mark_of(world: &World, scope: Entity) -> Option<MarkView> {
    let room = world
        .get::<ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>(scope)
        .map(|stamp| stamp.0);
    let mark = world.get::<crate::traversal::mark_recall::PlayerMark>(scope);
    Some(MarkView {
        at: mark.filter(|m| m.room == room).and_then(|m| m.pos).map(|at| [at.x, at.y]),
    })
}

/// The first solid of the body's live room along its aim, within
/// [`AIM_CAST_REACH`]. The aim is the one a native held item reads
/// (`ability_aim_world`), and the walls are the composed walls of the room.
fn aim_cast_of(world: &World, scope: Entity) -> Option<AimCast> {
    let (Some(control), Some(kin), Some(frame)) = (
        world.get::<ActorControl>(scope),
        world.get::<BodyKinematics>(scope),
        world.get::<ResolvedMotionFrame>(scope),
    ) else {
        return Some(AimCast { hit: None });
    };
    let dir = ambition_held_items::ability_aim_world(&control.0, kin.facing, frame.down()).normalize_or_zero();
    let room = world.get::<ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>(scope);
    let hit = ambition_platformer2d_world::collision::with_room_in_world(world, room, |room| {
        room.solids().and_then(|walls| {
            ambition_platformer2d_core::cast::raycast_solids_far(&*walls, kin.pos, dir, AIM_CAST_REACH, false)
        })
    })
    .flatten()
    .map(|(distance, at, _normal)| AimHit { at: [at.x, at.y], distance });
    Some(AimCast { hit })
}

/// Put each body's mark where the module asked, in the live room the body is
/// in, in place of any mark it had.
fn lower_set_marks(
    mut outbox: ResMut<ExtensionOutbox>,
    mut commands: Commands,
    mut bodies: Query<(
        &BodyKinematics,
        Option<&mut crate::traversal::mark_recall::PlayerMark>,
        Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
    )>,
) {
    for submitted in outbox.drain::<SetMarkPort>(&WIELDED_USE) {
        let Ok((kin, mark, room)) = bodies.get_mut(submitted.scope) else {
            continue;
        };
        let at = submitted.value.at.resolve([kin.pos.x, kin.pos.y]);
        let dropped = crate::traversal::mark_recall::PlayerMark {
            pos: Some(at.into()),
            room: room.map(|stamp| stamp.0),
        };
        match mark {
            Some(mut existing) => *existing = dropped,
            None => {
                commands.entity(submitted.scope).insert(dropped);
            }
        }
    }
}

/// Move each body along its line, walls of its own live room permitting, by
/// the discrete-transit authority (ADR 0024): its velocity is kept, and its
/// departure contacts and attachment are reconciled.
pub fn lower_transits(
    mut outbox: ResMut<ExtensionOutbox>,
    world: ambition_platformer2d_world::collision::CollisionWorld,
    mut bodies: Query<(
        ambition_platformer2d_core::BodyClusterQueryData,
        &ResolvedMotionFrame,
        &mut ambition_platformer2d_core::movement::MotionModel,
        Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
    )>,
    // Optional diagnostic Class-B ledger, so a minimal test app still moves.
    mut class_b: Option<ResMut<ambition_platformer2d_shared_tangle::class_b::ClassBRemapLog>>,
) {
    use ambition_platformer2d_core as ae;
    for submitted in outbox.drain::<TransitPort>(&WIELDED_USE) {
        let Ok((mut cluster_item, frame, mut model, room)) = bodies.get_mut(submitted.scope) else {
            continue;
        };
        let transit = submitted.value;
        let mut clusters = cluster_item.as_clusters_mut();
        let from = clusters.kinematics.pos;
        let target = match transit.to {
            ambition_combat_port::Destination::To(at) => ae::Vec2::from(at),
            ambition_combat_port::Destination::Along { direction, distance } => {
                let dir = ae::Vec2::from(direction);
                // The box the body has: turned to the DOWN of its last step.
                // For a body the axis arm moves that is the DOWN of its
                // resolved frame; a crawler on a wall lies along the wall,
                // and only the record of its step says so.
                let down = ae::SweepSample::down_or(clusters.sweep.as_deref(), frame.down());
                let half = clusters.kinematics.half_oriented(down);
                match world.room(room).and_then(|room| room.solids()).as_ref() {
                    Some(w) => crate::traversal::blink::blink_target(&**w, from, dir, distance, half),
                    // No collision world (a minimal test app): the full distance.
                    None => from + dir * distance,
                }
            }
        };
        ae::movement::transit_body(&mut model, &mut clusters, target, ae::movement::TransitVelocity::Keep);
        if let Some(facing) = transit.facing {
            clusters.kinematics.facing = facing;
        }
        // A transit is a scripted teleport, ranked weakest
        // (`docs/concepts/movement-collision.md`): dying in one is a death.
        if let Some(log) = class_b.as_mut() {
            log.record(submitted.scope, ambition_platformer2d_shared_tangle::class_b::ClassBRemap::ScriptedTeleport);
        }
    }
}

fn lower_movement_cooldowns(
    mut outbox: ResMut<ExtensionOutbox>,
    mut commands: Commands,
    mut cooldowns: Query<Option<&mut crate::ability_cooldown::AbilityCooldown>>,
) {
    for submitted in outbox.drain::<MovementCooldownPort>(&WIELDED_USE) {
        let Ok(mut cooldown) = cooldowns.get_mut(submitted.scope) else {
            continue;
        };
        if !crate::ability_cooldown::try_use_ability(
            &mut cooldown,
            &mut commands,
            submitted.scope,
            submitted.value.seconds,
        ) {
            warn!(
                "extension entry {} asked {:?} to arm a movement cooldown that runs; refused",
                submitted.entry, submitted.scope
            );
        }
    }
}

/// Where the body is now: a `Place::Body` request lowered after a transit is
/// at the arrival.
fn resolve_place(
    place: ambition_combat_port::Place,
    body: Option<&BodyKinematics>,
) -> Option<ambition_platformer2d_core::Vec2> {
    match (place, body) {
        (ambition_combat_port::Place::Body, None) => None,
        (place, body) => Some(place.resolve(body.map_or([0.0; 2], |kin| [kin.pos.x, kin.pos.y])).into()),
    }
}

fn lower_effects(
    mut outbox: ResMut<ExtensionOutbox>,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    bodies: Query<(Option<&BodyKinematics>, Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>)>,
) {
    for submitted in outbox.drain::<EffectPort>(&WIELDED_USE) {
        let effect = submitted.value;
        let (kin, room) = bodies.get(submitted.scope).unwrap_or((None, None));
        let Some(pos) = resolve_place(effect.at, kin) else {
            warn!("extension entry {} asked for an effect at a body with no position; refused", submitted.entry);
            continue;
        };
        // Drawn in the live room of the body.
        vfx.for_room(room.map(|stamp| stamp.0)).write(ambition_vfx::vfx::VfxMessage::Effect {
            pos,
            fx: ambition_vfx::fx::FxId::new(&effect.fx),
            scale: effect.scale,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
    }
}

fn lower_wielded_bursts(
    mut outbox: ResMut<ExtensionOutbox>,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    rooms: Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
) {
    for submitted in outbox.drain::<BurstPort>(&WIELDED_USE) {
        let burst = submitted.value;
        let Some(kind) = ambition_vfx::vfx::ParticleKind::named(&burst.kind) else {
            warn!("extension entry {} asked for a burst of {:?}; refused", submitted.entry, burst.kind);
            continue;
        };
        // Drawn in the live room of the body.
        vfx.for_room(rooms.get(submitted.scope).ok().map(|stamp| stamp.0)).write(
            ambition_vfx::vfx::VfxMessage::Burst {
                pos: Vec2::from(burst.at),
                count: burst.count,
                speed: burst.speed,
                color: burst.color,
                kind,
            },
        );
    }
}

fn lower_hit_marks(
    mut outbox: ResMut<ExtensionOutbox>,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    bodies: Query<(Option<&BodyKinematics>, Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>)>,
) {
    for submitted in outbox.drain::<HitMarkPort>(&WIELDED_USE) {
        let (kin, room) = bodies.get(submitted.scope).unwrap_or((None, None));
        let Some(pos) = resolve_place(submitted.value.at, kin) else {
            warn!("extension entry {} asked for a hit mark at a body with no position; refused", submitted.entry);
            continue;
        };
        // Drawn in the live room of the body.
        vfx.for_room(room.map(|stamp| stamp.0)).write(ambition_vfx::vfx::VfxMessage::Impact { pos });
    }
}

fn lower_velocities(mut outbox: ResMut<ExtensionOutbox>, mut bodies: Query<&mut BodyKinematics>) {
    for submitted in outbox.drain::<SetVelocityPort>(&WIELDED_USE) {
        let Ok(mut kin) = bodies.get_mut(submitted.scope) else {
            warn!("extension entry {} asked to set the velocity of a body with none; refused", submitted.entry);
            continue;
        };
        kin.vel = Vec2::from(submitted.value.velocity);
    }
}

fn lower_mana_spends(mut outbox: ResMut<ExtensionOutbox>, mut banks: Query<Option<&mut ActorResources>>) {
    for submitted in outbox.drain::<SpendManaPort>(&WIELDED_USE) {
        let Ok(bank) = banks.get_mut(submitted.scope) else {
            continue;
        };
        // ⛔ SUBMITTED IS NOT APPLIED: a bank that cannot pay pays nothing.
        if !crate::mana::spend(bank.map(|b| b.into_inner()), submitted.value.amount) {
            warn!(
                "extension entry {} asked {:?} to pay {} mana it does not have; refused",
                submitted.entry, submitted.scope, submitted.value.amount
            );
        }
    }
}

fn lower_body_sounds<L: LowersIn>(
    mut outbox: ResMut<ExtensionOutbox>,
    mut sfx: ambition_sfx::BodySfxWriter,
    bodies: Query<&BodyKinematics>,
) {
    for submitted in outbox.drain::<BodySoundPort>(&L::PHASE) {
        let sound = submitted.value;
        let Some(pos) = resolve_place(sound.at, bodies.get(submitted.scope).ok()) else {
            warn!("extension entry {} asked for a sound at a body with no position; refused", submitted.entry);
            continue;
        };
        sfx.write_for(
            submitted.scope,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::SfxId::new(&sound.cue),
                pos,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use ambition_combat_port::wielded::PAY_EPSILON;

    /// The port's `can_pay_mana` and the bank's `pay` are one rule: a module
    /// that asked the port is never refused by the bank.
    #[test]
    fn the_ports_mana_rule_is_the_banks() {
        for (level, cost) in [(30.0, 30.0), (29.9999995, 30.0), (29.99, 30.0), (0.0, 0.0), (100.0, 25.0)] {
            let mut bank = crate::mana::bank();
            let spent = 100.0 - level;
            assert!(crate::mana::spend(Some(&mut bank), spent), "set up {level}");
            let port = level + PAY_EPSILON >= cost;
            assert_eq!(crate::mana::spend(Some(&mut bank), cost), port, "level {level}, cost {cost}");
        }
    }
}
