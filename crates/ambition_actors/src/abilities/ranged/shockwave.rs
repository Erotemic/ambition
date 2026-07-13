//! Shockwave Slam — a boss-style ground-slam AOE the **player** can wield.
//!
//! The first "player wields a boss attack" slice, now expressed on the effect
//! seam: `Attack` while holding the shockwave gauntlet emits a generic
//! [`ambition_vfx::EffectRequest`] carrying a `DamageBox` effect anchored at
//! the emitter. The generic [`ambition_vfx::apply_effects`] consumer spawns
//! the World-anchored, faction-tagged AOE — so the SAME path serves the player
//! (Player faction → damages enemies) and a boss (Boss faction → damages the
//! player, see `boss_encounter::systems` phase-transition slam). No bespoke
//! per-attack consumer: the technique just emits an effect.

use bevy::prelude::*;

use crate::actor::BodyKinematics;
use crate::actor::BodyMana;
use crate::features::HeldItem;
use ambition_characters::brain::ActorControl;
use ambition_engine_core as ae;

/// Held-item id of the shockwave gauntlet.
pub const SHOCKWAVE_ID: &str = "shockwave";

/// Mana the shockwave slam spends per use (out of 100). With the sandbox's fast
/// regen this is feedback (the bar visibly drops), not a hard gate — feel-tune.
const SHOCKWAVE_MANA_COST: f32 = 25.0;

/// Player-wielded shockwave tunings. (A boss authors its own `DamageBox` values
/// at its emit site; these are the player gauntlet's.)
const SHOCKWAVE_HALF: ae::Vec2 = ae::Vec2::new(120.0, 52.0);
const SHOCKWAVE_DAMAGE: i32 = 4;
const SHOCKWAVE_LIFETIME_S: f32 = 0.18;
const SHOCKWAVE_KNOCKBACK: f32 = 1.3;

/// `Attack` while holding the shockwave gauntlet emits a `DamageBox` effect from
/// the **wielding body**. Plain Attack only — `Shield + Attack` is the throw/drop
/// gesture (handled by `item_pickup::throw_held_item_system`, which excludes
/// this id from throw-on-plain-Attack).
///
/// Body-generic: the trigger reads the body's own resolved intent
/// ([`ActorControl`], the same frame an NPC brain writes) rather than the
/// player's raw input, and iterates every wielder. `BodyMana` is the implicit
/// gate (player-only today), so a possessed/robot body that gains mana + this
/// gauntlet slams through this exact path — no player-casing.
pub fn fire_shockwave_system(
    mut wielders: Query<(
        Entity,
        &ActorControl,
        &HeldItem,
        &BodyKinematics,
        &crate::physics::ResolvedMotionFrame,
        &mut BodyMana,
    )>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
) {
    for (entity, control, held, kin, resolved_frame, mut mana) in &mut wielders {
        if !control.0.melee_pressed || control.0.shield_held {
            continue;
        }
        if held.spec.id != SHOCKWAVE_ID {
            continue;
        }
        // Costs mana — out of mana, no slam (the sandbox's fast regen tops it back up).
        if !mana.meter.try_spend(SHOCKWAVE_MANA_COST) {
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
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_ROCK_HIT,
            pos: kin.pos,
        });
    }
}

#[cfg(test)]
mod tests;
