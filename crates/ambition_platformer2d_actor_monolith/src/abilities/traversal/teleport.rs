//! A move that teleports its owner, with the ledge assist a recovery needs.
//!
//! ⭐⭐ THE OTHER HALF OF `ambition_characters::smash_teleport`. The key and its
//! params are what a MOVESET authors; resolving a destination against the
//! collision world is engine work, and it belongs HERE rather than in a game
//! crate because [`blink_target`](super::blink::blink_target) is already the one
//! teleport rule every controller shares. A second implementation of "how far
//! does a teleport actually go" is exactly the shape this repository keeps
//! paying for.
//!
//! ⭐⭐ AND THE LEDGE ASSIST IS THE POINT. Jon, 2026-08-27: *"We need to be sure
//! we have some sort of aim assist when the blinks are aimed at a ledge."* A
//! teleport recovery aimed at a platform edge either lands on it or dies a few
//! pixels under it, and that margin is a stick angle no player can hold. Within
//! the authored radius the arrival is placed STANDING on the ledge instead.

use bevy::prelude::*;

use ambition_characters::brain::{ActionRequest, ActorActionMessage, SpecialActionSpec};
use ambition_characters::smash_teleport::{TeleportParams, TELEPORT};
use ambition_platformer2d_core::{self as ae, AabbExt};

/// Snap an arrival onto a nearby ledge.
///
/// A "ledge" is the TOP FACE of a solid the fighter could stand on. The
/// candidate is that face directly under the destination, and the arrival is
/// placed standing on it.
///
/// ⛔⛔ IT ONLY EVER LIFTS, and getting that sign right is the whole feature.
/// `+y` is gravity-DOWN, so the ledge this catches is the one whose top face is
/// ABOVE the arrival — the fighter fell SHORT and is hanging under the lip, which
/// is the miss a recovery actually makes. The first version of this function had
/// the comparison the other way round and would have snapped a fighter who
/// CLEARED the platform back down onto it: the assist taking the stage away from
/// you, and every "it refuses" test still passing.
///
/// It refuses in three cases that all look like success from inside a naive
/// version:
///
/// - a destination that is ALREADY supported needs no help, and moving it would
///   be the assist choosing a different platform than the player did;
/// - a ledge BELOW the destination is one the fighter cleared, and pulling them
///   down onto it would end a recovery that had already succeeded;
/// - a placement that would EMBED the body in something is worse than the miss
///   it was trying to fix.
///
/// Deterministic by construction: blocks are scanned in world order and ties are
/// broken by the first index, never by "whichever was closer than the last".
pub fn ledge_assisted_arrival(
    world: &ae::World,
    destination: ae::Vec2,
    half: ae::Vec2,
    radius: f32,
) -> ae::Vec2 {
    if radius <= 0.0 {
        return destination;
    }
    let standing = ae::Aabb::new(destination, half);
    let already_supported = world.blocks.iter().any(|b| {
        matches!(b.kind, ae::BlockKind::Solid | ae::BlockKind::OneWay)
            && b.aabb.top() >= standing.bottom() - 2.0
            && b.aabb.top() <= standing.bottom() + 2.0
            && b.aabb.right() > standing.left()
            && b.aabb.left() < standing.right()
    });
    if already_supported {
        return destination;
    }

    let mut best: Option<(f32, ae::Vec2)> = None;
    for block in &world.blocks {
        if !matches!(block.kind, ae::BlockKind::Solid | ae::BlockKind::OneWay) {
            continue;
        }
        // The point on this block's TOP face nearest the destination.
        let x = destination.x.clamp(block.aabb.left(), block.aabb.right());
        let surface = ae::Vec2::new(x, block.aabb.top());
        // ⛔ ABOVE THE DESTINATION ONLY (`+y` is gravity-down, so "above" is a
        // SMALLER y). A surface below the arrival is one the fighter cleared,
        // and dragging them down onto it would end a recovery that worked.
        if surface.y > destination.y {
            continue;
        }
        let offset = surface - destination;
        let distance = offset.length();
        if distance > radius {
            continue;
        }
        let landing = ae::Vec2::new(x, block.aabb.top() - half.y);
        let box_at = ae::Aabb::new(landing, half);
        let embeds = world.blocks.iter().any(|b| {
            matches!(b.kind, ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. })
                && box_at.strict_intersects(b.aabb)
        });
        if embeds {
            continue;
        }
        if best.is_none_or(|(best_distance, _)| distance < best_distance) {
            best = Some((distance, landing));
        }
    }
    best.map_or(destination, |(_, landing)| landing)
}

/// One candidate for [`TeleportParams::behind_nearest_foe`], gathered before the
/// mutable pass so the two queries never alias.
struct FoeCandidate {
    entity: Entity,
    pos: ae::Vec2,
    half: ae::Vec2,
    faction: ambition_combat::components::ActorFaction,
    team: Option<ambition_combat::targeting::MatchTeam>,
    driving: Option<ambition_characters::control::DrivingParticipant>,
    sim: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
}

/// The far side of the nearest foe, or `None` when there is no foe to get
/// behind.
///
/// ⛔⛔ THE SCAN IS ORDERED BY `SimId`, NOT BY QUERY ORDER. Bevy's iteration
/// order is not stable and under rollback the raw `Entity` ids are not stable
/// either, so an exact-distance tie decided by either would resolve differently
/// on a rewind and desync the match. This is the same discipline
/// `select_actor_targets` states for the same reason.
///
/// ⛔ AND "BEHIND" IS THE FAR SIDE FROM THE TELEPORTER, not the far side from
/// the foe's facing. A fighter who turns to meet you does not thereby drag you
/// around to their front; the ambush is decided by where the attacker came
/// from, which is the thing the attacker controls.
fn behind_nearest_foe(
    from: ae::Vec2,
    my_half: ae::Vec2,
    me: Entity,
    my_faction: ambition_combat::components::ActorFaction,
    my_team: Option<&ambition_combat::targeting::MatchTeam>,
    my_driving: Option<&ambition_characters::control::DrivingParticipant>,
    candidates: &[FoeCandidate],
    gap: f32,
) -> Option<ae::Vec2> {
    let mut ordered: Vec<&FoeCandidate> = candidates.iter().filter(|c| c.entity != me).collect();
    ordered.sort_by(|a, b| match (&a.sim, &b.sim) {
        (Some(x), Some(y)) => x.cmp(y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.entity.cmp(&b.entity),
    });
    let mut best: Option<(f32, &FoeCandidate)> = None;
    for candidate in ordered {
        // THE one relationship policy, the same call the damage side makes.
        let relation = ambition_combat::targeting::combat_relation(
            None,
            my_faction,
            my_driving,
            my_team,
            None,
            candidate.entity,
            candidate.faction,
            candidate.driving.as_ref(),
            candidate.team.as_ref(),
        );
        if relation != ambition_combat::targeting::CombatRelation::Foe {
            continue;
        }
        let distance = from.distance_squared(candidate.pos);
        // Strictly nearer wins, so the `SimId` order above breaks an exact tie.
        if best.is_none_or(|(best_distance, _)| distance < best_distance) {
            best = Some((distance, candidate));
        }
    }
    let (_, foe) = best?;
    // Behind = further along the line the attacker was already on.
    let side = if foe.pos.x >= from.x { 1.0 } else { -1.0 };
    Some(ae::Vec2::new(
        foe.pos.x + side * (foe.half.x + my_half.x + gap),
        foe.pos.y,
    ))
}

/// Recognise an authored teleport and move the body.
///
/// ⛔ IT RUNS WHERE EVERY OTHER `ActorActionMessage` CONSUMER RUNS, so a
/// teleport authored on a move's timeline fires on the frame the move says and
/// not a phase later.
pub fn apply_authored_teleports(
    world: ambition_platformer2d_world::collision::CollisionWorld,
    mut actions: MessageReader<ActorActionMessage>,
    // ⛔ A PARAM SET BECAUSE THE TWO QUERIES ALIAS. `BodyClusterQueryData` takes
    // `BodyKinematics` mutably and the foe scan reads it, so Bevy refuses them
    // as two plain queries. Candidates are gathered ONCE up front, which is also
    // the behaviour a teleport wants: every message in a frame sees the same
    // stage rather than one that shifts as bodies move.
    mut set: bevy::ecs::system::ParamSet<(
        Query<(
            ae::BodyClusterQueryData,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &ambition_characters::control::ActorControl,
            &mut ae::movement::MotionModel,
        )>,
        Query<(
            Entity,
            &ae::BodyKinematics,
            Option<&ambition_combat::components::ActorFaction>,
            Option<&ambition_combat::targeting::MatchTeam>,
            Option<&ambition_characters::control::DrivingParticipant>,
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        )>,
    )>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    let mut collision = None;
    // Drained first: the mutable body pass below cannot borrow the reader and
    // the candidate query at the same time.
    let requests: Vec<ActorActionMessage> = actions.read().cloned().collect();
    let candidates: Vec<FoeCandidate> = set
        .p1()
        .iter()
        .map(|(entity, kin, faction, team, driving, sim)| FoeCandidate {
            entity,
            pos: kin.pos,
            half: kin.size * 0.5,
            faction: faction.copied().unwrap_or_default(),
            team: team.cloned(),
            driving: driving.cloned(),
            sim: sim.cloned(),
        })
        .collect();
    let mut bodies = set.p0();
    for message in requests.iter() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != TELEPORT {
            continue;
        }
        let params = match params.hydrate::<TeleportParams>() {
            Ok(params) => params,
            Err(err) => {
                warn!("teleport params did not hydrate: {err}");
                continue;
            }
        };
        let Ok((mut cluster_item, resolved_frame, control, mut motion_model)) =
            bodies.get_mut(message.actor)
        else {
            continue;
        };
        let gravity_dir = resolved_frame.down();
        let facing = cluster_item.kinematics.facing;
        // ⭐ THE SAME AIM EVERY TRAVERSAL ABILITY READS — aim stick, then
        // movement stick, then facing — so a teleport is aimed the way a blink
        // is and a player who knows one knows the other.
        let dir = crate::items::pickup::ability_aim_world(&control.0, facing, gravity_dir)
            .normalize_or_zero();
        let dir = if dir == ae::Vec2::ZERO {
            // ⛔ A TELEPORT WITH NO AIM STILL GOES SOMEWHERE, and it goes UP.
            // This is a recovery: a fighter who lets go of the stick mid-move
            // has not asked to teleport into the floor.
            -gravity_dir
        } else {
            dir
        };

        let mut clusters = cluster_item.as_clusters_mut();
        let from = clusters.kinematics.pos;
        let half = clusters.kinematics.size * 0.5;
        // WHERE, and how far. An ambush aims at a body rather than along the
        // stick, and its reach is the distance to that body -- but it is clamped
        // by exactly the same wall rule, because a teleport that could pass
        // through a stage to reach someone is a different move.
        let (dir, distance) = if !params.behind_nearest_foe {
            (dir, params.distance)
        } else {
            {
                // ⭐ THE TELEPORTER IS IN ITS OWN CANDIDATE LIST, so its
                // allegiance is read from there rather than by widening the
                // MUTABLE body query to carry three more components it does not
                // otherwise touch.
                let me = candidates.iter().find(|c| c.entity == message.actor);
                let Some(behind) = behind_nearest_foe(
                    from,
                    half,
                    message.actor,
                    me.map(|c| c.faction).unwrap_or_default(),
                    me.and_then(|c| c.team.as_ref()),
                    me.and_then(|c| c.driving.as_ref()),
                    &candidates,
                    params.behind_gap,
                ) else {
                    // ⚠ NOBODY TO GET BEHIND, so the fighter stays put. See
                    // `TeleportParams::behind_nearest_foe` -- firing into empty space
                    // spends the move to arrive somewhere nobody asked for.
                    continue;
                };
                let offset = behind - from;
                let length = offset.length();
                if length <= f32::EPSILON {
                    continue;
                }
                // ⛔ CAPPED BY THE AUTHORED DISTANCE. Without it the move is a
                // stage-wide snap to whoever exists, which is not a special --
                // it is a homing missile with the fighter as the payload.
                (offset / length, length.min(params.distance))
            }
        };
        let solids = collision.get_or_insert_with(|| world.solids());
        let target = match solids.as_ref() {
            Some(w) => {
                let clamped = super::blink::blink_target(&**w, from, dir, distance, half);
                ledge_assisted_arrival(&**w, clamped, half, params.ledge_assist)
            }
            // No collision world (a minimal test app) — the full distance, which
            // is what `blink_system` does in the same situation.
            None => from + dir * distance,
        };

        // THE discrete-transit authority: arrive with momentum kept, departure
        // contacts and any attachment reconciled (ADR 0024 authority model).
        //
        // ⛔ `Zero`, NOT `Keep`, and that is the difference between this and the
        // held-item blink. A teleport RECOVERY that kept the fall it was thrown
        // out of would arrive on the ledge already moving down through it; the
        // genre's teleports all stop you dead where you land.
        ae::movement::transit_body(
            &mut motion_model,
            &mut clusters,
            target,
            ae::movement::TransitVelocity::Zero,
        );

        // The look is the MOVE's, not this system's — see `TeleportParams`.
        vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
            pos: from,
            fx: ambition_vfx::fx::FxId::new(&params.depart_vfx),
            scale: 1.0,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
        vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
            pos: target,
            fx: ambition_vfx::fx::FxId::new(&params.arrive_vfx),
            scale: 1.0,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
        sfx.write_for(
            message.actor,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::PLAYER_BLINK,
                pos: target,
            },
        );
    }
}

#[cfg(test)]
mod tests;
