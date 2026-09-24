//! Blink: a held item that grants a short-range directional teleport.
//!
//! A held-item ability like Mark/Recall and Fireball, so it reuses the equip,
//! menu, and throw plumbing. Stateless (no mark), so nothing to clear on
//! reset. Like other pure-use held items it has no melee/ranged verb and opts
//! out of throw-on-attack through `throw_held_item_system`'s `use_on_attack`
//! id check.

use bevy::prelude::*;

use ambition_combat::held_items::HeldItem;
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};

/// The held-item id the Blink ability grants.
pub const BLINK_ID: &str = "blink";

/// How far a blink carries the player along the aim direction, walls permitting.
const BLINK_DISTANCE: f32 = 150.0;

/// Cooldown between blinks, so it is a deliberate reposition, not spam.
const BLINK_COOLDOWN_S: f32 = 0.45;

/// Half-extent of the arrival shockwave that lets you blink offensively into a
/// cluster of enemies.
const BLINK_SHOCKWAVE_HALF: f32 = 36.0;
/// Shockwave damage: modest; Blink is mobility first, a light strike second.
const BLINK_SHOCKWAVE_DAMAGE: i32 = 2;

/// Resolve a blink destination over `world`: teleport up to `distance` along the
/// unit `dir`, stopping a body-half (`half`, measured in the blink direction)
/// short of the first solid so the body never embeds, with a safety net that
/// falls back to `from` if the landing box would still overlap a solid.
///
/// The one teleport rule for every controller: the player's held-item blink
/// and any actor body that resolves a `blink` intent from its
/// `ActorControlFrame` call this (I2/I7), against the collision world it
/// occupies.
pub fn blink_target(
    world: &ae::World,
    from: ae::Vec2,
    dir: ae::Vec2,
    distance: f32,
    half: ae::Vec2,
) -> ae::Vec2 {
    // The pull-back uses the body's extent in the blink direction (half-height
    // for a vertical blink), or a diagonal blink embeds.
    let margin = (half.x * dir.x.abs() + half.y * dir.y.abs()) + 2.0;
    let mut target = match ambition_platformer2d_core::cast::raycast_solids(
        world,
        from,
        dir,
        distance + margin,
        false,
    ) {
        Some((hit, _normal)) => hit - dir * margin,
        None => from + dir * distance,
    };
    // Safety net: the center ray can miss a wall the body's width would clip
    // (corners, grazing). If the landing box still overlaps a solid, stay at
    // the start.
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
    world: ambition_platformer2d_world::collision::CollisionWorld,
    mut commands: Commands,
    // Every driven body, not only the primary seat's `ControlledSubject`, so
    // a possessed body or a second seat can use it.
    driven: ambition_held_items::DrivenBodies,
    mut bodies: Query<(
        Entity,
        ae::BodyClusterQueryData,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &HeldItem,
        &ActorControl,
        Option<&mut crate::ability_cooldown::AbilityCooldown>,
        &mut ambition_platformer2d_core::movement::MotionModel,
    )>,
    mut sfx: ambition_sfx::BodySfxWriter,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut hits: MessageWriter<ambition_combat::events::HitEvent>,
    // Optional diagnostic Class-B ledger (§3.2), so a minimal test app still
    // blinks.
    mut class_b: Option<ResMut<ClassBRemapLog>>,
) {
    for subject in driven.entities() {
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
            continue;
        };
        if !matches!(
            *motion_model,
            ambition_platformer2d_core::movement::MotionModel::AxisSwept(_)
        ) {
            continue;
        }
        let c = control.0;
        // Plain Attack blinks; Shield+Attack throws the item away.
        if !c.melee_pressed || c.shield_held {
            continue;
        }
        if held.spec.id != BLINK_ID {
            continue;
        }
        // Aim from the brain-resolved frame (aim stick, then movement stick,
        // then facing), rotated to world. Uses the body's per-tick resolved
        // frame (ADR 0024).
        let gravity_dir = resolved_frame.down();
        let facing = cluster_item.kinematics.facing;
        let dir =
            ambition_held_items::ability_aim_world(&c, facing, gravity_dir).normalize_or_zero();
        if dir == ae::Vec2::ZERO {
            continue;
        }
        // Check the shared movement-ability cooldown only after a real blink
        // is confirmed, so an aimless press does not use it.
        if !crate::ability_cooldown::try_use_ability(
            &mut cooldown,
            &mut commands,
            player,
            BLINK_COOLDOWN_S,
        ) {
            continue;
        }
        let mut clusters = cluster_item.as_clusters_mut();
        let from = clusters.kinematics.pos;
        let half = clusters.kinematics.size * 0.5;
        // One collision view (moving platforms and ECS solids included) for
        // the clamp raycast and the embed check in `blink_target`.
        let collision = world.solids();
        let target = match collision.as_ref() {
            Some(w) => blink_target(&**w, from, dir, BLINK_DISTANCE, half),
            // No collision world (tests): blink the full distance.
            None => from + dir * BLINK_DISTANCE,
        };
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
        // weakest, so dying mid-blink is a death, not a blink.
        if let Some(log) = class_b.as_mut() {
            log.record(player, ClassBRemap::ScriptedTeleport);
        }
        // Offensive blink: a small player-side shockwave at the arrival point,
        // so you can blink into enemies to hit them (PlayerSlash spares the
        // player).
        hits.write(ambition_combat::events::HitEvent {
            strike_sfx: None,
            volume: ae::CombatVolume::circle(target, BLINK_SHOCKWAVE_HALF),
            damage: BLINK_SHOCKWAVE_DAMAGE,
            source: ambition_combat::events::HitSource::Melee,
            attacker: Some(player),
            target: ambition_combat::events::HitTarget::Volume,
            mode: ambition_combat::events::HitMode::Knockback,
            knockback: None,
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
        // A wisp where you left, a flash where you arrive.
        vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
            pos: from,
            fx: ambition_vfx::fx::ids::CLASSIC_BURST,
            scale: 0.35,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
        vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
            pos: target,
            fx: ambition_vfx::fx::ids::CLASSIC_BURST,
            scale: 0.5,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
    }
}

#[cfg(test)]
mod tests;
