//! Overflow Crash: a player-wielded lunge strike. Dash forward along the aim
//! and hit everything in the dash corridor. The wielded kit's only offensive
//! mobility attack: [`crate::ranged::shockwave`], [`crate::ranged::beam`],
//! and [`crate::ranged::volley`] are stationary, and
//! [`crate::traversal::blink`] is a defensive reposition. The dive's damage
//! covers the whole path from start to landing.
//!
//! It is the overflow boss's signature, an aerial dive-bomber: defeat it and
//! wield its crash.
//!
//! It reuses [`ambition_platformer2d_core::cast::raycast_solids`] (the blink's
//! wall-stop, so the lunge never lands inside geometry) and a one-shot
//! `Player`-faction [`ambition_combat::events::HitEvent`] over the corridor
//! (`PlayerSlash` source: damages enemies, spares the player). One-shot, not
//! a lingering `Hitbox`, because a dash hits as it crosses.
//!
//! The lunge snaps to the dominant aim axis (default: facing), so the
//! corridor is a thin rectangle.

use bevy::prelude::*;

use ambition_combat::held_items::HeldItem;
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};

/// Held-item id of the dive gauntlet.
pub const DIVE_ID: &str = "dive";

/// Mana per lunge (out of 100), so it cannot be spammed across a room.
const DIVE_MANA_COST: f32 = 26.0;

/// How far (px) the player lunges along the aim, absent a wall.
const DIVE_LUNGE: f32 = 140.0;
/// Half-thickness (px) of the damaging corridor swept by the lunge.
const DIVE_WIDTH: f32 = 48.0;
/// Damage dealt to everything in the corridor.
const DIVE_DAMAGE: i32 = 4;
/// Horizontal shove imparted to struck enemies (signed by the lunge direction).
const DIVE_KNOCKBACK: f32 = 1.4;

/// Snap an aim and facing to a lunge direction (a unit vector on the
/// dominant axis). A null aim uses `facing`, so a plain Attack still lunges;
/// the blink, in contrast, needs an explicit aim.
fn dive_dir(aim: ae::Vec2, facing: f32) -> ae::Vec2 {
    let horizontal = if aim == ae::Vec2::ZERO {
        true
    } else {
        aim.x.abs() >= aim.y.abs()
    };
    if horizontal {
        let s = if aim.x.abs() > 0.001 {
            aim.x.signum()
        } else {
            facing.signum()
        };
        ae::Vec2::new(s, 0.0)
    } else {
        ae::Vec2::new(0.0, aim.y.signum())
    }
}

/// The damaging corridor from `from` to `to`: an axis-aligned box around both
/// endpoints, padded by a body width. For a snapped lunge this is a thin
/// rectangle.
fn dive_corridor(from: ae::Vec2, to: ae::Vec2) -> ae::Aabb {
    let center = (from + to) * 0.5;
    let half = ae::Vec2::new(
        (to.x - from.x).abs() * 0.5 + DIVE_WIDTH * 0.5,
        (to.y - from.y).abs() * 0.5 + DIVE_WIDTH * 0.5,
    );
    ae::Aabb::new(center, half)
}

/// `Attack` while holding the dive gauntlet lunges the player along the aim
/// and emits a one-shot `Player`-faction hit over the corridor. Plain Attack
/// only; `Shield + Attack` drops the item (the id is `UseSystem`, excluded from
/// throw-on-plain-Attack in `throw_held_item_system`).
pub fn fire_dive_system(
    world: ambition_platformer2d_world::collision::CollisionWorld,
    // Every driven body, not only the primary seat's `ControlledSubject`, so
    // a possessed body or a second seat can act.
    driven: ambition_held_items::DrivenBodies,
    mut players: Query<(
        Entity,
        &ActorControl,
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d_core::movement::MotionModel,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &HeldItem,
    )>,
    mut sfx: ambition_sfx::BodySfxWriter,
    mut hits: MessageWriter<ambition_combat::events::HitEvent>,
    // Optional diagnostic Class-B ledger (§3.2), so a minimal test app still
    // dives.
    mut class_b: Option<ResMut<ClassBRemapLog>>,
) {
    for subject in driven.entities() {
        let Ok((player, control, mut cluster_item, mut motion_model, resolved_frame, held)) =
            players.get_mut(subject)
        else {
            continue;
        };
        let mut clusters = cluster_item.as_clusters_mut();
        let c = control.0;
        if !c.melee_pressed || c.shield_held {
            continue;
        }
        if held.spec.id != DIVE_ID {
            continue;
        }
        if !crate::mana::spend(clusters.resources.as_deref_mut(), DIVE_MANA_COST) {
            continue;
        }
        // The body's per-tick resolved frame (ADR 0024 frame law).
        let frame = resolved_frame.basis();
        let facing = clusters.kinematics.facing;
        let local_aim = ambition_held_items::ability_aim_local(&c, facing);
        let local_dir = dive_dir(local_aim, facing).normalize_or_zero();
        let dir = frame.to_world(local_dir).normalize_or_zero();
        let from = clusters.kinematics.pos;
        // Stop a body-half short of the wall so the lunge never embeds. Use
        // the body's extent in the lunge direction (half-height for a vertical
        // dive), as the blink does, or a downward dive embeds in the floor
        // and trips the OOB detector.
        let half = clusters.kinematics.size * 0.5;
        let margin = (half.x * dir.x.abs() + half.y * dir.y.abs()) + 2.0;
        // One collision view for the clamp raycast and the embed check, so
        // moving platforms and ECS solids also stop the lunge.
        let collision = world.solids();
        let mut target = match collision.as_ref().and_then(|w| {
            ambition_platformer2d_core::cast::raycast_solids(
                &**w,
                from,
                dir,
                DIVE_LUNGE + margin,
                false,
            )
        }) {
            Some((hit, _normal)) => hit - dir * margin,
            None => from + dir * DIVE_LUNGE,
        };
        // Safety net: if the landing AABB still overlaps a solid (a corner
        // the center ray missed), stay at the start instead of embedding.
        if let Some(w) = collision.as_ref() {
            let landing = ae::Aabb::new(target, half);
            let embeds = w.blocks.iter().any(|b| {
                matches!(
                    b.kind,
                    ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
                ) && landing.strict_intersects(b.aabb)
            });
            if embeds {
                target = from;
            }
        }
        // The discrete-transit authority: arrive with momentum kept, and
        // reconcile departure contacts and attachment (ADR 0024).
        ae::movement::transit_body(
            &mut motion_model,
            &mut clusters,
            target,
            ae::movement::TransitVelocity::Keep,
        );
        // Class-B transit (`docs/concepts/movement-collision.md`): a
        // traversal ability that moves a body is a scripted teleport, ranked
        // weakest, so dying mid-dive is a death, not a dive.
        if let Some(log) = class_b.as_mut() {
            log.record(player, ClassBRemap::ScriptedTeleport);
        }
        if local_dir.x.abs() > 0.001 {
            clusters.kinematics.facing = local_dir.x.signum();
        }
        // The corridor hits everything between start and landing: a one-shot
        // PlayerSlash volume that spares the player and pushes enemies along
        // the dash. The push uses `DIVE_KNOCKBACK` above.
        let corridor: ambition_platformer2d_core::CombatVolume = dive_corridor(from, target).into();
        let corridor_center = corridor.center();
        hits.write(ambition_combat::events::HitEvent {
            strike_sfx: None,
            volume: corridor,
            damage: DIVE_DAMAGE,
            source: ambition_combat::events::HitSource::Melee,
            attacker: Some(player),
            target: ambition_combat::events::HitTarget::Volume,
            mode: ambition_combat::events::HitMode::Knockback,
            knockback: Some(ambition_combat::events::HitKnockback {
                // An ordinary hit: it stuns.
                reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
                dir: local_dir.x.signum(),
                magnitude: ambition_combat::events::HitKnockbackMagnitude::FeelScale(
                    DIVE_KNOCKBACK,
                ),
                source_pos: corridor_center,
                impact_pos: corridor_center,
                launch_dir: None,
                follow: None,
            }),
            ignored_targets: Vec::new(),
                    attacker_move_instance: None,
        });
        sfx.write_for(
            player,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::PLAYER_BLINK,
                pos: target,
            },
        );
    }
}

#[cfg(test)]
mod tests;
