//! The adapter of `ambition.combat.blast`: damage where a body is, credited to
//! another body. See the port card in `ambition_combat_port::blast`.
//!
//! Three questions stay separate: who is credited (the body the credit
//! names: kills, grudges, staleness, rage), where the blast is (where the
//! module says), and whom it may hurt (every body: `HitSide::Environment`,
//! so the setter next to their own blast is caught).

use ambition_combat_port::{BlastPort, CreditPort};
use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox, InModuleEntityTick, InTechniqueExecution, LowersIn};
use ambition_extension_sdk::phases::{MODULE_ENTITY_TICK, TECHNIQUE_EXECUTION};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

/// Offer a blast and the scope's credit in `technique_execution`.
pub fn install(app: &mut App) {
    app.add_message::<crate::damage_box::DamageBoxRequest>();
    app.install_extension_submitted_request::<BlastPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_blasts::<InTechniqueExecution>,
    );
    app.install_extension_observation::<CreditPort>(TECHNIQUE_EXECUTION, "ambition_combat", credit_of_scope);
}

/// Offer a blast and the scope's credit in `module_entity_tick`: an entity
/// that goes off (a bomb) owns its blast.
pub fn install_for_module_entity_tick(app: &mut App) {
    app.add_message::<crate::damage_box::DamageBoxRequest>();
    app.install_extension_submitted_request::<BlastPort, _>(
        MODULE_ENTITY_TICK,
        "ambition_combat",
        lower_blasts::<InModuleEntityTick>,
    );
    app.install_extension_observation::<CreditPort>(MODULE_ENTITY_TICK, "ambition_combat", credit_of_scope);
}

/// The credit of `scope`, absent when it has no simulation identity.
pub fn credit_of_scope(world: &World, scope: Entity) -> Option<u64> {
    world.get::<SimId>(scope).map(credit_of)
}

/// The credit that names `body` durably: its simulation identity, hashed
/// (FNV-1a), so a module can keep it in a record across a rewind and every
/// peer computes the same one. Never `0`, which names nobody.
pub fn credit_of(body: &SimId) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in body.as_str().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash.max(1)
}

/// Emit each validated blast with the emitter's captured context.
pub fn lower_blasts<L: LowersIn>(
    mut outbox: ResMut<ExtensionOutbox>,
    mut effects: crate::damage_box::DamageBoxWriter,
    identities: Query<(Entity, &SimId)>,
) {
    for submitted in outbox.drain::<BlastPort>(&L::PHASE) {
        let blast = submitted.value;
        let refused = blast.credit == 0
            || !blast.center.iter().all(|at| at.is_finite())
            || !(blast.lifetime_s.is_finite() && blast.lifetime_s > 0.0)
            || !blast.half_extent.iter().all(|extent| extent.is_finite() && *extent >= 0.0)
            || !(blast.knockback.is_finite() && blast.knockback >= 0.0)
            || blast.damage < 0;
        if refused {
            warn!("extension entry {} asked for a blast {blast:?}; refused", submitted.entry);
            continue;
        }
        let owner = identities
            .iter()
            .find(|(_, identity)| credit_of(identity) == blast.credit)
            .map(|(body, _)| body)
            .unwrap_or(Entity::PLACEHOLDER);
        // Environmental blasts permit self-damage and use the emission room.
        effects.write(submitted.scope, owner, ambition_vfx::HitSide::Environment, ae::Vec2::from(blast.center), crate::strike::DamageBox {
                half_extent: ae::Vec2::from(blast.half_extent),
                damage: blast.damage,
                knockback: crate::strike::HitboxKnockback::FeelScale(blast.knockback),
                lifetime_s: blast.lifetime_s,
                name: Some("blast"),
                shape: None,
            });
    }
}

#[cfg(test)]
mod tests;
