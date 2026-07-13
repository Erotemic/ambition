//! Focus Beam — a player-wielded **directional line attack**: a long, thin,
//! aimed [`Hitbox`] that damages every enemy along its length.
//!
//! This is the third wielded boss-style attack, alongside [`crate::abilities::ranged::shockwave`]
//! (a centered AOE) and [`crate::abilities::ranged::volley`] (a ranged fan). Where the shockwave
//! slams a compact box at the player's feet, the beam reaches *forward* along
//! the aim as a long thin box — a single readable lance that skewers a line of
//! enemies. It is the **smirking_behemoth** (the eye-beam boss) signature
//! gauntlet: defeat the boss whose tell is a focused eye beam, wield the beam
//! yourself ("every boss a failed objective function, learn its attack").
//!
//! Mechanically it rides the same faction-tagged [`Hitbox`] primitive the
//! shockwave uses — a `Player`-faction box damages enemies/bosses through the
//! `apply_hitbox_damage` player branch, not the player. The box is axis-aligned
//! (the `Hitbox` primitive carries no rotation), so the aim snaps to its
//! dominant axis: a mostly-horizontal aim fires a wide horizontal lance, a
//! mostly-vertical aim fires a tall vertical one. Diagonal aim resolves to
//! whichever axis dominates — good enough for a first pass; a rotated beam is a
//! feel/visual follow-up.

use ambition_characters::brain::ActorControl;
use bevy::prelude::*;

use crate::actor::BodyKinematics;
use crate::actor::BodyMana;
use crate::features::HeldItem;
use ambition_engine_core as ae;
use ambition_platformer_primitives::markers::ControlledSubject;

/// Held-item id of the focus-beam gauntlet.
pub const BEAM_ID: &str = "beam";

/// Mana the beam spends per zap (out of 100). The priciest of the three wielded
/// attacks — it's a strong, long-reach, line-clearing hit, so it's gated harder.
const BEAM_MANA_COST: f32 = 30.0;

/// Beam length (px) along the aim axis — how far forward it reaches.
const BEAM_LENGTH: f32 = 300.0;
/// Beam thickness (px) across the aim axis.
const BEAM_WIDTH: f32 = 30.0;
const BEAM_DAMAGE: i32 = 5;
const BEAM_LIFETIME_S: f32 = 0.12;
const BEAM_KNOCKBACK: f32 = 1.1;

/// Resolve the beam's axis-aligned geometry from an aim vector. Snaps to the
/// dominant axis and returns `(center_offset_from_player, half_extent)` so the
/// box reaches `BEAM_LENGTH` forward along that axis. A zero aim falls back to
/// `facing` (a forward horizontal lance), so a plain Attack with no directional
/// hold still fires.
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

/// `Attack` while holding the beam gauntlet fires an aimed line [`Hitbox`] of
/// **Player** faction along the dominant aim axis. Plain Attack only —
/// `Shield + Attack` drops the item (the id is `UseSystem`, excluded from
/// throw-on-plain-Attack in `throw_held_item_system`).
pub fn fire_beam_system(
    // Ability ORIGIN = the controlled subject, not a `PrimaryPlayer` filter.
    controlled: Res<ControlledSubject>,
    mut players: Query<(
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
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((entity, control, held, kin, resolved_frame, mut mana)) = players.get_mut(subject)
    else {
        return;
    };
    let c = control.0;
    if !c.melee_pressed || c.shield_held {
        return;
    }
    if held.spec.id != BEAM_ID {
        return;
    }
    // Costs mana — out of mana, no beam (the sandbox's fast regen tops it back up).
    if !mana.meter.try_spend(BEAM_MANA_COST) {
        return;
    }
    // The body's per-tick resolved frame (ADR 0024 frame law).
    let frame = resolved_frame.basis();
    let aim = crate::items::pickup::ability_aim_local(&c, kin.facing);
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
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        pos: kin.pos,
    });
}

#[cfg(test)]
mod tests;
