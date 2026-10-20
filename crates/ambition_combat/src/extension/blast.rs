//! The adapter of `ambition.combat.blast`: damage where a body is, credited to
//! another body. See the port card in `ambition_combat_port::blast`.
//!
//! Three questions stay separate: who is credited (the body the credit
//! names: kills, grudges, staleness, rage), where the blast is (the scope
//! body, when the request is lowered), and whom it may hurt (every body:
//! `HitSide::Environment`, so the setter next to their own blast is caught).

use ambition_combat_port::BlastPort;
use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

/// Offer a blast in `technique_execution`.
pub fn install(app: &mut App) {
    app.add_message::<ambition_vfx::EffectRequest>();
    app.install_extension_request::<BlastPort, _>(TECHNIQUE_EXECUTION, "ambition_combat", lower_blasts);
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

/// How long a credit stand-in outlives the blast it was spawned for, so every
/// contact the blast makes can still resolve its owner.
pub const CREDIT_STAND_IN_MARGIN_S: f32 = 0.17;

/// The owner of a blast whose credited body has left: an entity that is not
/// the body under the blast and is no participant, so the credit falls to
/// nobody rather than to the victim. It leaves once the blast can no longer
/// land.
///
/// Rollback state: a rewind across the blast must restore it, or the replayed
/// blast has no owner.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct CreditStandIn {
    pub remaining_s: f32,
}

/// The rollback value projection: the clock, in milliseconds.
pub fn credit_stand_in_probe(stand_in: &CreditStandIn) -> u64 {
    (stand_in.remaining_s * 1000.0).round().max(0.0) as u64
}

/// Spend the stand-ins' clocks, then lower each blast a module asked for, in
/// the order asked.
///
/// One system ticks the stand-ins and spawns them, so both use one clock.
pub fn lower_blasts(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    time: Res<ambition_time::WorldTime>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    bodies: Query<&ae::BodyKinematics>,
    identities: Query<(Entity, &SimId)>,
    mut stand_ins: Query<(Entity, &mut CreditStandIn)>,
) {
    let dt = time.sim_dt();
    for (stand_in, mut credit) in &mut stand_ins {
        credit.remaining_s -= dt;
        if credit.remaining_s <= 0.0 {
            commands.entity(stand_in).despawn();
        }
    }
    for submitted in outbox.drain::<BlastPort>(&TECHNIQUE_EXECUTION) {
        let blast = submitted.value;
        let refused = blast.credit == 0
            || !(blast.lifetime_s.is_finite() && blast.lifetime_s > 0.0)
            || !blast.half_extent.iter().all(|extent| extent.is_finite() && *extent >= 0.0)
            || !(blast.knockback.is_finite() && blast.knockback >= 0.0)
            || blast.damage < 0;
        if refused {
            warn!("extension entry {} asked for a blast {blast:?}; refused", submitted.entry);
            continue;
        }
        let Ok(kin) = bodies.get(submitted.scope) else {
            continue;
        };
        let owner = identities
            .iter()
            .find(|(_, identity)| credit_of(identity) == blast.credit)
            .map(|(body, _)| body)
            .unwrap_or_else(|| {
                commands
                    .spawn((
                        CreditStandIn { remaining_s: blast.lifetime_s + CREDIT_STAND_IN_MARGIN_S },
                        Name::new("Credit stand-in"),
                    ))
                    .id()
            });
        effects.write(ambition_vfx::EffectRequest {
            owner,
            effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                center: kin.pos,
                // `Environment`, not `Neutral`: `melee_source` excludes Neutral
                // from the body path, so a Neutral blast damages nobody.
                // `Environment` also waives self-immunity.
                faction: ambition_vfx::HitSide::Environment,
                half_extent: ae::Vec2::from(blast.half_extent),
                damage: blast.damage,
                knockback: blast.knockback,
                lifetime_s: blast.lifetime_s,
                name: Some("blast"),
            }),
        });
    }
}

#[cfg(test)]
mod tests;
