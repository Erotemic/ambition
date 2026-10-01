//! The held-item domain's extension adapters (fast-iteration I7: a wielded
//! ability as a procedural module).
//!
//! The trigger: for each item id an admitted entry is bound to, one
//! [`WieldedUsePort`] invocation for each body that holds that item, each tick,
//! with the body's resolved control (press, aim), kinematics, gravity frame
//! and mana. The requests: pay mana from the body's bank, and play a sound as
//! the body. See the port cards in `ambition_combat_port::wielded`.

use ambition_characters::control::ActorControl;
use ambition_combat::held_items::HeldItem;
use ambition_combat_port::{
    BodySoundPort, EndModuleEntityPort, ModuleEntityTickPort, PullBodiesPort, SpawnModuleEntityPort, SpendManaPort,
    WieldedUsePort, Wielder,
};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::{MODULE_ENTITY_TICK, WIELDED_USE};
use ambition_extension_sdk::Port;
use ambition_platformer2d_core::resources::ActorResources;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

/// Install the trigger port and the three request ports in `wielded_use`.
pub fn install(app: &mut App) {
    app.install_extension_trigger::<WieldedUsePort, _>(WIELDED_USE, "ambition_abilities", queue_wielded_uses);
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

/// The phase a request adapter instance lowers for: one port offered in two
/// phases has two named adapter systems, not one system registered twice.
pub struct InWieldedUse;
/// See [`InWieldedUse`].
pub struct InModuleEntityTick;

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
    for (entity, control, held, kin, frame, bank, id, counts) in using {
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
            },
        );
    }
}

fn lower_mana_spends(mut outbox: ResMut<ExtensionOutbox>, mut banks: Query<Option<&mut ActorResources>>) {
    for submitted in outbox.drain::<SpendManaPort>() {
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

fn lower_body_sounds<Phase: Send + Sync + 'static>(mut outbox: ResMut<ExtensionOutbox>, mut sfx: ambition_sfx::BodySfxWriter) {
    for submitted in outbox.drain::<BodySoundPort>() {
        let sound = submitted.value;
        sfx.write_for(
            submitted.scope,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::SfxId::new(&sound.cue),
                pos: ambition_platformer2d_core::Vec2::from(sound.at),
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
