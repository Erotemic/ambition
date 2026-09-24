//! Shockwave Slam: a boss-style ground-slam AOE the player can wield.
//!
//! `Attack` while holding the shockwave gauntlet emits an
//! [`ambition_vfx::EffectRequest`] with a `DamageBox` effect anchored at the
//! emitter. [`ambition_combat::strike::apply_effects`] spawns the
//! world-anchored, faction-tagged AOE, so the same path serves the player
//! (Player faction damages enemies) and a boss (Boss faction damages the
//! player; see the phase-transition slam in `boss_encounter::systems`). The
//! technique only emits an effect.

use bevy::prelude::*;

use ambition_combat::held_items::HeldItem;
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;

/// Held-item id of the shockwave gauntlet.
pub const SHOCKWAVE_ID: &str = "shockwave";

/// Mana per use (out of 100).
const SHOCKWAVE_MANA_COST: f32 = 25.0;

/// Player gauntlet tuning. A boss authors its own `DamageBox` values where it
/// emits.
const SHOCKWAVE_HALF: ae::Vec2 = ae::Vec2::new(120.0, 52.0);
const SHOCKWAVE_DAMAGE: i32 = 4;
const SHOCKWAVE_LIFETIME_S: f32 = 0.18;
const SHOCKWAVE_KNOCKBACK: f32 = 1.3;

/// `Attack` while holding the shockwave gauntlet emits a `DamageBox` effect
/// from the wielding body. Plain Attack only; `Shield + Attack` is the
/// throw/drop gesture (`item_pickup::throw_held_item_system` excludes this id
/// from throw-on-plain-Attack).
///
/// Body-generic: reads the body's resolved intent ([`ActorControl`], the same
/// frame an NPC brain writes), not raw input, for every wielder. Mana is the
/// gate, and a body has Mana only when its experience declared the pool.
pub fn fire_shockwave_system(
    mut wielders: Query<(
        Entity,
        &ActorControl,
        &HeldItem,
        &BodyKinematics,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        Option<&mut ambition_platformer2d_core::resources::ActorResources>,
    )>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    for (entity, control, held, kin, resolved_frame, mut mana) in &mut wielders {
        if !control.0.melee_pressed || control.0.shield_held {
            continue;
        }
        if held.spec.id != SHOCKWAVE_ID {
            continue;
        }
        // Costs mana; with too little, no slam.
        if !crate::mana::spend(mana.as_deref_mut(), SHOCKWAVE_MANA_COST) {
            continue;
        }
        // The body's per-tick resolved frame (ADR 0024 frame law).
        let half_extent = resolved_frame.basis().to_world_half(SHOCKWAVE_HALF);
        effects.write(ambition_vfx::EffectRequest {
            owner: entity,
            effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                center: kin.pos,
                faction: ambition_vfx::HitSide::Player,
                half_extent,
                damage: SHOCKWAVE_DAMAGE,
                knockback: SHOCKWAVE_KNOCKBACK,
                lifetime_s: SHOCKWAVE_LIFETIME_S,
                name: Some("Shockwave AOE"),
            }),
        });
        sfx.write_for(
            entity,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_ROCK_HIT,
                pos: kin.pos,
            },
        );
    }
}

#[cfg(test)]
mod tests;
