//! The held-item domain's extension adapters (fast-iteration I7: a wielded
//! ability as a procedural module).
//!
//! The trigger: for each item id an admitted entry is bound to, one
//! [`WieldedUsePort`] invocation for each body that holds that item, each tick,
//! with the body's resolved control (press, aim), kinematics, gravity frame
//! and mana. The requests: move the body (a transit), arm the shared movement
//! cooldown, pay mana from the body's bank, play a sound as the body, and
//! show an effect. See the port cards in `ambition_combat_port::wielded` and
//! `ambition_combat_port::motion`.

use ambition_characters::control::ActorControl;
use ambition_combat::held_items::HeldItem;
use ambition_combat_port::{
    BodySoundPort, EffectPort, EndModuleEntityPort, ModuleEntityTickPort, MovementCooldownPort, PullBodiesPort,
    SpawnModuleEntityPort, SpendManaPort, WieldedUsePort, Wielder,
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

/// One invocation for each body holding a bound item, in an order a rewind
/// reproduces (the body's simulation identity, then its entity).
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
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == WieldedUsePort::KEY)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .collect();
    if bound.is_empty() {
        return;
    }
    let driven = driven.entities();
    let mut using: Vec<_> = wielders
        .iter()
        .filter(|(_, _, held, ..)| bound.contains(&held.id()))
        .collect();
    using.sort_by(|a, b| {
        (a.6.map(SimId::as_str), a.0.to_bits()).cmp(&(b.6.map(SimId::as_str), b.0.to_bits()))
    });
    for (entity, control, held, kin, frame, bank, id, counts, cooldown, model) in using {
        let c = control.0;
        let pressed = c.melee_pressed && !c.shield_held;
        let basis = frame.basis();
        let aim = ambition_held_items::ability_aim_local(&c, kin.facing);
        invocations.trigger::<WieldedUsePort>(
            &WIELDED_USE,
            held.id().to_owned(),
            entity,
            None,
            // The port's IDLE: the item is not used this tick.
            !pressed,
            Wielder {
                pressed,
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
                transits: transits(model),
            },
        );
    }
}

/// A transit moves a body that moves by the swept kernel, and no other.
fn transits(model: Option<&ambition_platformer2d_core::movement::MotionModel>) -> bool {
    matches!(model, Some(ambition_platformer2d_core::movement::MotionModel::AxisSwept(_)))
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
        if !transits(Some(&model)) {
            warn!(
                "extension entry {} asked {:?}, which does not move by the swept kernel, to transit; refused",
                submitted.entry, submitted.scope
            );
            continue;
        }
        let transit = submitted.value;
        let dir = ae::Vec2::from(transit.direction);
        let mut clusters = cluster_item.as_clusters_mut();
        let from = clusters.kinematics.pos;
        // The box the body has: turned to the DOWN of its resolved frame, as
        // the kernel turns it for the step.
        let half = clusters.kinematics.half_oriented(frame.down());
        let collision = world.room(room).and_then(|room| room.solids());
        let target = match collision.as_ref() {
            Some(w) => crate::traversal::blink::blink_target(&**w, from, dir, transit.distance, half),
            // No collision world (a minimal test app): the full distance.
            None => from + dir * transit.distance,
        };
        ae::movement::transit_body(&mut model, &mut clusters, target, ae::movement::TransitVelocity::Keep);
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
