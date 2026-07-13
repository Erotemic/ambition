//! Overflow Crash — a player-wielded **lunge strike**: dash forward along the
//! aim and skewer everything in the dash corridor. The wielded kit's only
//! *offensive mobility* attack — [`crate::abilities::ranged::shockwave`] / [`crate::abilities::ranged::beam`] /
//! [`crate::abilities::ranged::volley`] are all stationary, and while [`crate::abilities::traversal::blink`] also
//! teleports, blink is a *defensive* reposition (a tiny poof at the arrival
//! point); the dive is an *offensive* gap-closer whose damage is the whole
//! **path** from start to landing. Close the distance and cut a line through
//! the mob in one commit.
//!
//! It is the **overflow** boss signature gauntlet — an aerial dive-bomber that
//! bursts past its bounds and crashes into you. Defeat it, wield its crash
//! yourself ("every boss a failed objective function, learn its attack").
//!
//! Mechanically it reuses two proven primitives: [`crate::platformer_runtime::collision::raycast_solids`]
//! (the same wall-stop the blink uses, so the lunge never lands inside geometry)
//! and a one-shot `Player`-faction [`crate::features::HitEvent`] over the dash
//! corridor (a `PlayerSlash` source, so it damages enemies and spares the
//! player). A *one-shot* event — not a lingering `Hitbox` — because a dash hits
//! at the instant it crosses, it doesn't leave a damaging box behind.
//!
//! The lunge axis-snaps (dominant aim axis, defaulting to facing) so the
//! corridor stays a clean thin rectangle; a rotated dash is a feel follow-up.

use bevy::prelude::*;

use crate::features::HeldItem;
use ambition_characters::brain::ActorControl;
use ambition_engine_core::{self as ae, AabbExt};
use ambition_platformer_primitives::class_b::{ClassBRemap, ClassBRemapLog};
use ambition_platformer_primitives::markers::ControlledSubject;

/// Held-item id of the dive gauntlet.
pub const DIVE_ID: &str = "dive";

/// Mana the dive spends per lunge (out of 100). A committed gap-closer — gated
/// like the rest of the wielded kit so it can't be spammed across a room.
const DIVE_MANA_COST: f32 = 26.0;

/// How far (px) the player lunges along the aim, absent a wall.
const DIVE_LUNGE: f32 = 140.0;
/// Half-thickness (px) of the damaging corridor swept by the lunge.
const DIVE_WIDTH: f32 = 48.0;
/// Damage dealt to everything in the corridor.
const DIVE_DAMAGE: i32 = 4;
/// Horizontal shove imparted to struck enemies (signed by the lunge direction).
const DIVE_KNOCKBACK: f32 = 1.4;

/// Axis-snap an aim + facing to the lunge direction (a unit vector along the
/// dominant axis). A null aim falls back to `facing` (a forward dash), so a
/// plain Attack with no directional hold still lunges — it's an attack, not a
/// precise teleport like the blink (which needs an explicit aim).
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

/// The damaging corridor swept from `from` to `to` — an axis-aligned box that
/// bounds both endpoints, padded by a body-width so the dash has thickness. For
/// an axis-snapped lunge this is a clean thin rectangle along the dash.
fn dive_corridor(from: ae::Vec2, to: ae::Vec2) -> ae::Aabb {
    let center = (from + to) * 0.5;
    let half = ae::Vec2::new(
        (to.x - from.x).abs() * 0.5 + DIVE_WIDTH * 0.5,
        (to.y - from.y).abs() * 0.5 + DIVE_WIDTH * 0.5,
    );
    ae::Aabb::new(center, half)
}

/// `Attack` while holding the dive gauntlet lunges the player along the aim and
/// emits a one-shot `Player`-faction hit over the dash corridor. Plain Attack
/// only — `Shield + Attack` drops the item (the id is `UseSystem`, excluded from
/// throw-on-plain-Attack in `throw_held_item_system`).
pub fn fire_dive_system(
    world: ambition_world::collision::CollisionWorld,
    // Ability ORIGIN = the controlled subject, not a `PrimaryPlayer` filter.
    controlled: Res<ControlledSubject>,
    mut players: Query<(
        Entity,
        &ActorControl,
        ae::BodyClusterQueryData,
        &mut crate::features::MotionModel,
        &crate::physics::ResolvedMotionFrame,
        &HeldItem,
    )>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
    mut hits: MessageWriter<crate::features::HitEvent>,
    // Optional: the diagnostic-only Class-B ledger (§3.2). A minimal test app
    // that never added the engine's schedule plugin still dives.
    mut class_b: Option<ResMut<ClassBRemapLog>>,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((player, control, mut cluster_item, mut motion_model, resolved_frame, held)) =
        players.get_mut(subject)
    else {
        return;
    };
    let mut clusters = cluster_item.as_clusters_mut();
    let c = control.0;
    if !c.melee_pressed || c.shield_held {
        return;
    }
    if held.spec.id != DIVE_ID {
        return;
    }
    if !clusters.mana.meter.try_spend(DIVE_MANA_COST) {
        return;
    }
    // The body's per-tick resolved frame (ADR 0024 frame law).
    let frame = resolved_frame.basis();
    let facing = clusters.kinematics.facing;
    let local_aim = crate::items::pickup::ability_aim_local(&c, facing);
    let local_dir = dive_dir(local_aim, facing).normalize_or_zero();
    let dir = frame.to_world(local_dir).normalize_or_zero();
    let from = clusters.kinematics.pos;
    // Stop a body-half short of the wall so the lunge never embeds. The pull-back
    // must use the body's extent IN THE LUNGE DIRECTION -- half-height for a
    // vertical dive, not half-width -- the same direction-aware clamp the blink
    // uses (or a down/diagonal dive embeds in the floor and trips the OOB detector).
    let half = clusters.kinematics.size * 0.5;
    let margin = (half.x * dir.x.abs() + half.y * dir.y.abs()) + 2.0;
    // One composited collision view, shared by the clamp raycast and the embed
    // safety net, so the lunge is stopped by moving platforms / ECS solids too.
    let collision = world.solids();
    let mut target = match collision.as_ref().and_then(|w| {
        crate::platformer_runtime::collision::raycast_solids(
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
    // Safety net: if the landing AABB still overlaps a solid (a corner / grazing
    // the center-ray missed), fall back to the start instead of embedding.
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
    // mid-dive is a death, not a dive.
    if let Some(log) = class_b.as_mut() {
        log.record(player, ClassBRemap::ScriptedTeleport);
    }
    if local_dir.x.abs() > 0.001 {
        clusters.kinematics.facing = local_dir.x.signum();
    }
    // The dash corridor cuts everything between start and landing — a one-shot
    // PlayerSlash volume (spares the player, shoves enemies along the dash).
    hits.write(crate::features::HitEvent {
        volume: dive_corridor(from, target).into(),
        damage: DIVE_DAMAGE,
        source: crate::features::HitSource::PlayerSlash {
            knock_x: local_dir.x * DIVE_KNOCKBACK,
        },
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
}

#[cfg(test)]
mod tests;
