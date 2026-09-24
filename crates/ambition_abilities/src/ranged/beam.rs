//! Focus Beam: a player-wielded directional line attack. A long, thin, aimed
//! [`Hitbox`] damages every enemy along its length.
//!
//! Beside [`crate::ranged::shockwave`] (a centered AOE) and
//! [`crate::ranged::volley`] (a ranged fan), the beam reaches forward along
//! the aim as one long box. It is the smirking_behemoth's signature (its tell
//! is an eye beam): defeat it and wield the beam.
//!
//! It uses the same faction-tagged [`Hitbox`] as the shockwave: a
//! `Player`-faction box damages enemies and bosses through
//! `apply_hitbox_damage`, not the player. `Hitbox` has no rotation, so the
//! aim snaps to its dominant axis: mostly horizontal fires a horizontal lance,
//! mostly vertical a vertical one.

use ambition_characters::control::ActorControl;
use bevy::prelude::*;

use ambition_combat::held_items::HeldItem;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;

/// Held-item id of the focus-beam gauntlet.
pub const BEAM_ID: &str = "beam";

/// Mana per zap (out of 100). Expensive, because it is a strong,
/// long-reach, line-clearing hit.
const BEAM_MANA_COST: f32 = 30.0;

/// Beam length (px) along the aim axis: how far forward it reaches.
const BEAM_LENGTH: f32 = 300.0;
/// Beam thickness (px) across the aim axis.
const BEAM_WIDTH: f32 = 30.0;
const BEAM_DAMAGE: i32 = 5;
const BEAM_LIFETIME_S: f32 = 0.12;
const BEAM_KNOCKBACK: f32 = 1.1;

/// The beam's axis-aligned geometry from an aim vector. Snaps to the dominant
/// axis and returns `(center_offset_from_player, half_extent)`, reaching
/// `BEAM_LENGTH` forward. A zero aim uses `facing` (a forward horizontal
/// lance), so a plain Attack still fires.
fn beam_geometry(aim: ae::Vec2, facing: f32) -> (ae::Vec2, ae::Vec2) {
    let half_len = BEAM_LENGTH * 0.5;
    let half_wid = BEAM_WIDTH * 0.5;
    // Pick the dominant axis; default to horizontal-facing on a null aim.
    let horizontal = if aim == ae::Vec2::ZERO {
        true
    } else {
        aim.x.abs() >= aim.y.abs()
    };
    if horizontal {
        let dir = if aim.x.abs() > 0.001 {
            aim.x.signum()
        } else {
            facing.signum()
        };
        (
            ae::Vec2::new(dir * half_len, 0.0),
            ae::Vec2::new(half_len, half_wid),
        )
    } else {
        let dir = aim.y.signum();
        (
            ae::Vec2::new(0.0, dir * half_len),
            ae::Vec2::new(half_wid, half_len),
        )
    }
}

/// `Attack` while holding the beam gauntlet fires an aimed `Player`-faction
/// line [`Hitbox`] along the dominant aim axis. Plain Attack only;
/// `Shield + Attack` drops the item (the id is `UseSystem`, excluded from
/// throw-on-plain-Attack in `throw_held_item_system`).
pub fn fire_beam_system(
    // Every driven body, not only the primary seat's `ControlledSubject`, so
    // a possessed body or a second seat can use it.
    driven: ambition_held_items::DrivenBodies,
    mut players: Query<(
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
    for subject in driven.entities() {
        let Ok((entity, control, held, kin, resolved_frame, mut mana)) = players.get_mut(subject)
        else {
            continue;
        };
        let c = control.0;
        if !c.melee_pressed || c.shield_held {
            continue;
        }
        if held.spec.id != BEAM_ID {
            continue;
        }
        // Costs mana; with too little, no beam.
        if !crate::mana::spend(mana.as_deref_mut(), BEAM_MANA_COST) {
            continue;
        }
        // The body's per-tick resolved frame (ADR 0024 frame law).
        let frame = resolved_frame.basis();
        let aim = ambition_held_items::ability_aim_local(&c, kin.facing);
        let (offset_local, half_local) = beam_geometry(aim, kin.facing);
        let offset = frame.to_world(offset_local);
        let half_extent = frame.to_world_half(half_local);
        effects.write(ambition_vfx::EffectRequest {
            owner: entity,
            effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                center: kin.pos + offset,
                faction: ambition_vfx::HitSide::Player,
                half_extent,
                damage: BEAM_DAMAGE,
                knockback: BEAM_KNOCKBACK,
                lifetime_s: BEAM_LIFETIME_S,
                name: Some("Focus Beam"),
            }),
        });
        // G1: the beam is this body's ability, so it speaks in this body's voice.
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
