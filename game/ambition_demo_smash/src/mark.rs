//! The delayed mark: a clock riding on the body that was hit.
//!
//! ⭐⭐ NOTHING HERE IS COMBAT BEHAVIOUR, which is the campaign's thesis for the
//! sixth time. `OnHitEffectMessage` owns "who was struck by what"; `BodyKinematics`
//! owns where that body is; `DamageBoxEffect` owns the blast. This module
//! contributes ONE CLOCK and ONE DECISION — when the clock runs out, ask for a
//! blast where the marked body currently is. That is the whole of it.
//!
//! ⛔ THE MARK RIDES THE BODY. A mine asks the opponent to avoid a SPOT; a mark
//! travels with them, so running away does not help and the pressure is on the
//! clock. That is the only reason this is a different technique rather than a
//! re-tuned mine.
//!
//! ⚠ AND A SECOND MARK REFRESHES RATHER THAN STACKS, which the authored params
//! say too: stacking turns one read ("how long have I got") into arithmetic the
//! player cannot do mid-match, and the read is what the move sells.

use bevy::prelude::*;

use ambition_platformer2d::characters::smash_mark::{MarkBodyParams, MARK_BODY};

/// A live mark on one body.
///
/// ⛔ ROLLBACK-REGISTERED, and it must be: this is sim state that decides whether
/// a hitbox appears, so a rewind that lost it would drop a detonation the peer
/// still expects — `PlacedMine` carries the same registration for the same
/// reason. The probe is the clock, because the clock is what a desync would
/// disagree about first.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BodyMark {
    /// Seconds remaining before this mark goes off.
    pub fuse_s: f32,
    pub damage: i32,
    pub blast_radius: f32,
    pub knockback: f32,
}

/// Value projection for the rollback checksum: the clock, in milliseconds.
///
/// ⛔ NOT A PRESENCE PROBE. A mark whose PRESENCE survived a rewind while its
/// clock did not would detonate on a different frame on the two peers, which is
/// exactly the desync a presence-only probe cannot see.
pub fn body_mark_probe(mark: &BodyMark) -> u64 {
    (mark.fuse_s * 1000.0).round().max(0.0) as u64
}

/// Attach or refresh a mark on every body struck by a move that authors one.
pub fn apply_authored_body_marks(
    mut commands: Commands,
    mut hits: MessageReader<ambition_platformer2d::combat::on_hit::OnHitEffectMessage>,
) {
    for hit in hits.read() {
        if hit.effect.key != MARK_BODY {
            continue;
        }
        let Ok(params) = hit.effect.params.hydrate::<MarkBodyParams>() else {
            // An unhydratable payload is an AUTHORING error, not a runtime one:
            // the key matched, so somebody meant this. Left loud rather than
            // silent for the same reason the mine's is.
            warn!(
                target: "ambition::moves",
                "a `{MARK_BODY}` payload did not hydrate; the mark was not applied",
            );
            continue;
        };
        if let Ok(mut victim) = commands.get_entity(hit.victim) {
            victim.insert(BodyMark {
                fuse_s: params.fuse_s,
                damage: params.damage,
                blast_radius: params.blast_radius,
                knockback: params.knockback,
            });
        }
    }
}

/// Tick every live mark and detonate the ones that reach zero.
pub fn detonate_body_marks(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut effects: MessageWriter<ambition_platformer2d::vfx::EffectRequest>,
    mut marked: Query<(
        Entity,
        &mut BodyMark,
        &ambition_platformer2d::engine_core::BodyKinematics,
    )>,
) {
    let dt = time.sim_dt();
    for (entity, mut mark, kin) in &mut marked {
        mark.fuse_s -= dt;
        if mark.fuse_s > 0.0 {
            continue;
        }
        effects.write(ambition_platformer2d::vfx::EffectRequest {
            // ⛔ THE MARKED BODY OWNS THE BLAST, not the fighter who applied it,
            // and the mine's own comment argues the same shape from the other
            // side: the thing that goes off is what exploded. It also keeps the
            // blast honest under `Environment` below — a marked player standing
            // next to their opponent catches them too, which is the counterplay.
            owner: entity,
            effect: ambition_platformer2d::vfx::Effect::DamageBox(
                ambition_platformer2d::vfx::DamageBoxEffect {
                    center: kin.pos,
                    // ⛔ `Environment`, the ruling the mine had to be CORRECTED
                    // to: its comment records that `Neutral` reads as "hurts
                    // everybody" and the resolver does the opposite — `melee_source`
                    // excludes Neutral from the body path, so a Neutral blast
                    // damages nobody and only a test asking about the REQUEST
                    // would call that working.
                    faction: ambition_platformer2d::vfx::HitSide::Environment,
                    half_extent: ambition_platformer2d::engine_core::Vec2::splat(
                        mark.blast_radius,
                    ),
                    damage: mark.damage,
                    knockback: mark.knockback,
                    lifetime_s: 0.08,
                    name: Some("mark detonation"),
                },
            ),
        });
        commands.entity(entity).remove::<BodyMark>();
    }
}

#[cfg(test)]
mod tests;
