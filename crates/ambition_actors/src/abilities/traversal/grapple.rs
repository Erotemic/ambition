//! Grapple — a held item that yanks the player toward a grappled surface.
//!
//! Canon ability ([`crate::items::Item::Grapple`]): a traversal pull. Implemented
//! as a wired ability (a held item) like Blink / Mark/Recall / Fireball, so it
//! reuses the equip / OoT-menu / throw plumbing. While holding it, `Attack`
//! casts a line along the aim direction; if it lands on a solid wall within
//! [`GRAPPLE_RANGE`], the player is yanked toward the hit at [`GRAPPLE_PULL_SPEED`]
//! (a burst impulse — collision resolution then settles them at the surface).
//! A grapple into empty space fizzles.
//!
//! Stateless, so nothing to clear on reset; opts out of throw-on-attack like the
//! other pure-use abilities.

use bevy::prelude::*;

use crate::actor::BodyKinematics;
use crate::features::HeldItem;
use ambition_characters::brain::ActorControl;
use ambition_engine_core as ae;
use ambition_platformer_primitives::markers::ControlledSubject;

/// The held-item id the Grapple ability grants.
pub const GRAPPLE_ID: &str = "grapple";

/// How far the grapple line reaches for a solid surface.
const GRAPPLE_RANGE: f32 = 300.0;

/// Speed of the burst yank toward a grappled surface.
const GRAPPLE_PULL_SPEED: f32 = 620.0;

/// Cooldown between successful yanks, so grappling reads as deliberate.
const GRAPPLE_COOLDOWN_S: f32 = 0.55;

/// `Attack` while holding the Grapple ability casts along the aim direction; on
/// hitting a solid within [`GRAPPLE_RANGE`] it yanks the player toward the hit.
pub fn grapple_system(
    world: ambition_world::collision::CollisionWorld,
    mut commands: Commands,
    // Ability execution is SUBJECT-GENERIC: acts on the `ControlledSubject`,
    // reading that body's OWN `ActorControl` (brain output) + `HeldItem`. No
    // `With<PlayerEntity>` filter, no `PlayerInputFrame` — works for a possessed
    // actor exactly as for the home avatar.
    controlled: Res<ControlledSubject>,
    mut bodies: Query<(
        Entity,
        &ActorControl,
        &mut BodyKinematics,
        &crate::physics::ResolvedMotionFrame,
        &HeldItem,
        Option<&mut crate::ability_cooldown::AbilityCooldown>,
    )>,
    mut sfx: ambition_sfx::SfxWriter,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((player, control, mut kin, resolved_frame, held, mut cooldown)) =
        bodies.get_mut(subject)
    else {
        return;
    };
    let c = control.0;
    if !c.melee_pressed || c.shield_held {
        return;
    }
    if held.spec.id != GRAPPLE_ID {
        return;
    }
    // The body's per-tick resolved frame (ADR 0024 frame law).
    let gravity_dir = resolved_frame.down();
    let dir =
        crate::items::pickup::ability_aim_world(&c, kin.facing, gravity_dir).normalize_or_zero();
    if dir == ae::Vec2::ZERO {
        return;
    }
    let from = kin.pos;
    // Raycast against the composited collision world so the grapple can latch a
    // moving platform / ECS solid, not just the bare authored room.
    let Some((hit, _normal)) = world.solids().and_then(|w| {
        crate::platformer_runtime::collision::raycast_solids(&*w, from, dir, GRAPPLE_RANGE, false)
    }) else {
        // Grapple into empty space: a dry fizzle, no pull (and no cooldown burned).
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::PLAYER_DASH,
            pos: from,
        });
        return;
    };
    // Only a successful latch is on cooldown — a miss above costs nothing.
    if !crate::ability_cooldown::try_use_ability(
        &mut cooldown,
        &mut commands,
        player,
        GRAPPLE_COOLDOWN_S,
    ) {
        return;
    }
    // Yank toward the latched surface (collision resolution settles the player at
    // it). A burst velocity, not a teleport, so the movement reads as a pull.
    let pull = (hit - from).normalize_or_zero();
    kin.vel = pull * GRAPPLE_PULL_SPEED;
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::PLAYER_DASH,
        pos: from,
    });
    // Draw the grapple LINE as a tan spark trail from the player to the latch
    // point, so the ability READS as a grapple rope being thrown and reeling you
    // in — not just a mysterious sudden yank (#53 "not sure what it does").
    const GRAPPLE_LINE_SEGMENTS: i32 = 8;
    for i in 1..GRAPPLE_LINE_SEGMENTS {
        let p = from.lerp(hit, i as f32 / GRAPPLE_LINE_SEGMENTS as f32);
        vfx.write(ambition_vfx::vfx::VfxMessage::Burst {
            pos: p,
            count: 2,
            speed: 28.0,
            color: [0.86, 0.78, 0.48, 0.95],
            kind: ambition_vfx::vfx::ParticleKind::Spark,
        });
    }
    vfx.write(ambition_vfx::vfx::VfxMessage::Impact { pos: hit });
}

#[cfg(test)]
mod tests;
