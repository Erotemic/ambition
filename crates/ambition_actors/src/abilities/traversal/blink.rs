//! Blink — a held item granting a short-range directional teleport.
//!
//! Canon ability ([`crate::items::Item::Blink`]): Jon's note — "Short-range
//! teleport. Your favorite, and high-skill." Implemented as a wired ability (a
//! held item) like Mark/Recall and Fireball, so it reuses the equip / OoT-menu /
//! throw plumbing. While holding it, `Attack` blinks the player a fixed distance
//! along the aim direction, **collision-clamped**: a `raycast_solids` stops the
//! teleport just short of the first wall so you can't blink through or embed in a
//! solid (the "collision safety policy" the blink design calls for).
//!
//! Stateless (no mark to store), so there's nothing to clear on reset. Like the
//! other pure-use held items it has no melee/ranged verb and opts out of
//! throw-on-attack via `throw_held_item_system`'s `use_on_attack` id check.

use bevy::prelude::*;

use crate::features::HeldItem;
use ambition_characters::brain::ActorControl;
use ambition_engine_core::{self as ae, AabbExt};
use ambition_platformer_primitives::class_b::{ClassBRemap, ClassBRemapLog};
use ambition_platformer_primitives::markers::ControlledSubject;

/// The held-item id the Blink ability grants.
pub const BLINK_ID: &str = "blink";

/// How far a blink carries the player along the aim direction, walls permitting.
const BLINK_DISTANCE: f32 = 150.0;

/// Cooldown between blinks, so it reads as a deliberate reposition (not spam).
const BLINK_COOLDOWN_S: f32 = 0.45;

/// Half-extent of the arrival shockwave that lets you blink offensively into a
/// cluster of enemies.
const BLINK_SHOCKWAVE_HALF: f32 = 36.0;
/// Shockwave damage — modest; Blink is mobility first, a light strike second.
const BLINK_SHOCKWAVE_DAMAGE: i32 = 2;

/// Resolve a blink destination over `world`: teleport up to `distance` along the
/// unit `dir`, stopping a body-half (`half`, measured in the blink direction)
/// short of the first solid so the body never embeds, with a safety net that
/// falls back to `from` if the landing box would still overlap a solid.
///
/// This is the **one teleport rule** shared by every controller: the player's
/// held-item blink and any actor body that resolves a `blink` intent from its
/// `ActorControlFrame` call the same function (invariants I2/I7 — a possessed or
/// AI body blinks exactly as the player does, against the same collision world it
/// physically occupies).
pub fn blink_target(
    world: &ae::World,
    from: ae::Vec2,
    dir: ae::Vec2,
    distance: f32,
    half: ae::Vec2,
) -> ae::Vec2 {
    // Pull-back must use the body's extent IN the blink direction — a vertical
    // blink needs half-height, not half-width — or a diagonal blink embeds.
    let margin = (half.x * dir.x.abs() + half.y * dir.y.abs()) + 2.0;
    let mut target = match crate::platformer_runtime::collision::raycast_solids(
        world,
        from,
        dir,
        distance + margin,
        false,
    ) {
        Some((hit, _normal)) => hit - dir * margin,
        None => from + dir * distance,
    };
    // Safety net: the center-ray can miss a wall the body's perpendicular extent
    // would clip (corners, grazing). If the landing box still overlaps a solid,
    // fall back to the start so a blink never lands inside geometry.
    let landing = ae::Aabb::new(target, half);
    let embeds = world.blocks.iter().any(|b| {
        matches!(
            b.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
        ) && landing.strict_intersects(b.aabb)
    });
    if embeds {
        target = from;
    }
    target
}

/// `Attack` while holding the Blink ability teleports the player up to
/// [`BLINK_DISTANCE`] along the aim direction, stopping a body-half short of the
/// first solid wall so the teleport never lands inside geometry.
pub fn blink_system(
    world: ambition_world::collision::CollisionWorld,
    mut commands: Commands,
    // Ability execution is SUBJECT-GENERIC: it acts on the CONTROLLED SUBJECT (the
    // body carrying `Brain::Player`) reading that body's OWN `ActorControl` (the
    // brain output — present on ANY controlled body, player or possessed actor)
    // and its OWN `HeldItem`. No `With<PlayerEntity>` filter, no `PlayerInputFrame`
    // (a possessed actor has neither). A possessed body blinks iff IT holds the
    // blink item; the vacated home avatar is not the subject, so it never blinks.
    //
    // The box-traversal kit belongs to the explicit axis-swept policy. A
    // surface-momentum body's traversal IS its kernel (the S1 v1 ruling:
    // blink/dash machinery absent) — a worn speedster must not teleport with
    // the robot's blink. Absence is never interpreted as an axis policy.
    controlled: Res<ControlledSubject>,
    mut bodies: Query<(
        Entity,
        ae::BodyClusterQueryData,
        &crate::physics::ResolvedMotionFrame,
        &HeldItem,
        &ActorControl,
        Option<&mut crate::ability_cooldown::AbilityCooldown>,
        &mut crate::features::MotionModel,
    )>,
    mut sfx: ambition_sfx::SfxWriter,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut hits: MessageWriter<crate::features::HitEvent>,
    // Optional: the diagnostic-only Class-B ledger (§3.2). A minimal test app
    // that never added the engine's schedule plugin still blinks.
    mut class_b: Option<ResMut<ClassBRemapLog>>,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((
        player,
        mut cluster_item,
        resolved_frame,
        held,
        control,
        mut cooldown,
        mut motion_model,
    )) = bodies.get_mut(subject)
    else {
        return;
    };
    if !matches!(*motion_model, crate::features::MotionModel::AxisSwept(_)) {
        return;
    }
    let c = control.0;
    // Plain Attack blinks; Shield+Attack is the generic "throw the item away".
    if !c.melee_pressed || c.shield_held {
        return;
    }
    if held.spec.id != BLINK_ID {
        return;
    }
    // Aim from the brain-resolved frame (aim stick → movement stick → facing),
    // rotated to world for the raycast/teleport. Body-generic — no per-ability
    // re-reading of raw input.
    // The body's per-tick resolved frame (ADR 0024 frame law).
    let gravity_dir = resolved_frame.down();
    let facing = cluster_item.kinematics.facing;
    let dir = crate::items::pickup::ability_aim_world(&c, facing, gravity_dir).normalize_or_zero();
    if dir == ae::Vec2::ZERO {
        return;
    }
    // Gate on the shared movement-ability cooldown (after confirming a real blink
    // so an aimless press doesn't burn it).
    if !crate::ability_cooldown::try_use_ability(
        &mut cooldown,
        &mut commands,
        player,
        BLINK_COOLDOWN_S,
    ) {
        return;
    }
    let mut clusters = cluster_item.as_clusters_mut();
    let from = clusters.kinematics.pos;
    let half = clusters.kinematics.size * 0.5;
    // One composited collision view (moving platforms + ECS solids included),
    // shared by the clamp raycast and the embed safety net inside `blink_target`.
    let collision = world.solids();
    let target = match collision.as_ref() {
        Some(w) => blink_target(&**w, from, dir, BLINK_DISTANCE, half),
        // No collision world (tests / degenerate) — blink the full distance.
        None => from + dir * BLINK_DISTANCE,
    };
    // THE discrete-transit authority: arrive with momentum kept, departure
    // contacts and any attachment reconciled (ADR 0024 authority model).
    ae::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        target,
        ae::movement::TransitVelocity::Keep,
    );
    // Class-B transit authority (`collision-and-ccd.md` §3.2): a traversal
    // ability that JUMPS a body is a scripted teleport, ranked weakest — dying
    // mid-blink is a death, not a blink.
    if let Some(log) = class_b.as_mut() {
        log.record(player, ClassBRemap::ScriptedTeleport);
    }
    // Offensive blink: a small player-side shockwave at the arrival point, so you
    // can blink *into* enemies to strike them (and the PlayerSlash source spares
    // the player). Composes nicely with a gravity well — blink in, sweep them up.
    hits.write(crate::features::HitEvent {
        strike_sfx: None,
        volume: ae::CombatVolume::circle(target, BLINK_SHOCKWAVE_HALF),
        damage: BLINK_SHOCKWAVE_DAMAGE,
        source: crate::features::HitSource::PlayerSlash { knock_x: 0.0 },
        attacker: Some(player),
        target: crate::features::HitTarget::Volume,
        mode: crate::features::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::PLAYER_BLINK,
        pos: target,
    });
    // A wisp where you left, a flash where you arrive.
    vfx.write(ambition_vfx::vfx::VfxMessage::Explosion {
        pos: from,
        kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
        scale: 0.35,
    });
    vfx.write(ambition_vfx::vfx::VfxMessage::Explosion {
        pos: target,
        kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
        scale: 0.5,
    });
}

#[cfg(test)]
mod tests;
