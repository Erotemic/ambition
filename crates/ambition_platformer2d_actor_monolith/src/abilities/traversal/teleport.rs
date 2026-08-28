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
    gravity_dir: ae::Vec2,
) -> ae::Vec2 {
    if radius <= 0.0 {
        return destination;
    }
    // The two axes of the frame this arrival is happening in. `along` is the
    // support axis (gravity), `across` the face it lands on.
    let down = gravity_dir.normalize_or_zero();
    let down = if down == ae::Vec2::ZERO {
        ae::Vec2::new(0.0, 1.0)
    } else {
        down
    };
    let across = ae::Vec2::new(-down.y, down.x);

    let standing = ae::Aabb::new(destination, half);
    let overlaps_across = |b: &ae::Aabb| {
        let (b_lo, b_hi) = span_across(*b, across);
        let (s_lo, s_hi) = span_across(standing, across);
        b_hi > s_lo && b_lo < s_hi
    };
    let already_supported = world.blocks.iter().any(|b| {
        matches!(b.kind, ae::BlockKind::Solid | ae::BlockKind::OneWay)
            && ae::collision_semantics::support_face_separation(standing, b.aabb, down).abs() <= 2.0
            && overlaps_across(&b.aabb)
    });
    if already_supported {
        return destination;
    }

    let mut best: Option<(f32, ae::Vec2)> = None;
    for block in &world.blocks {
        if !matches!(block.kind, ae::BlockKind::Solid | ae::BlockKind::OneWay) {
            continue;
        }
        // The point on this block's SUPPORT face nearest the destination: its
        // head face (the anti-gravity one a falling body lands on), clamped to
        // the block's own extent across that face.
        let (b_lo, b_hi) = span_across(block.aabb, across);
        let want = destination.dot(across).clamp(b_lo, b_hi);
        let surface = across * want + down * block.aabb.head_coord(down);
        // ⛔ TOWARD THE ANTI-GRAVITY SIDE ONLY. A surface further along gravity
        // than the arrival is one the fighter CLEARED, and dragging them onto it
        // would end a recovery that had already worked. Written against the
        // gravity axis rather than `y`: the teleport itself has always aimed in
        // the resolved frame, and this half searched the world's `+y` faces —
        // so under flipped or sideways gravity the ability aimed one way and
        // its assist looked the other.
        if surface.dot(down) > destination.dot(down) {
            continue;
        }
        let distance = (surface - destination).length();
        if distance > radius {
            continue;
        }
        // Feet exactly on that face, body centred across it.
        let landing =
            across * want + down * (block.aabb.head_coord(down) - half.dot(down.abs()).abs());
        let box_at = ae::Aabb::new(landing, half);
        let embeds = world.blocks.iter().any(|b| {
            matches!(
                b.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            ) && box_at.strict_intersects(b.aabb)
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

/// A box's extent projected onto the ACROSS axis, low first.
///
/// ⛔ BOTH CORNERS, because `across` may point either way along a world axis
/// and a min/max read off `.left()`/`.right()` is only correct for one of them.
fn span_across(b: ae::Aabb, across: ae::Vec2) -> (f32, f32) {
    let c = b.center().dot(across);
    let half = (b.half_size().x * across.x).abs() + (b.half_size().y * across.y).abs();
    (c - half, c + half)
}

/// One body the ambush can see: everything [`ambush_arrival`] needs to decide
/// whether it may be targeted and where behind it lands.
///
/// ⭐ THE TELEPORTER IS IN THIS LIST TOO, and that is why the chooser below
/// takes two of these rather than eight loose arguments. Its faction, team and
/// driver are read from its own row instead of by widening the MUTABLE body
/// query to carry three components a teleport does not otherwise touch, and
/// "who may I ambush" then reads as one call between two peers.
struct FoeCandidate {
    entity: Entity,
    pos: ae::Vec2,
    half: ae::Vec2,
    faction: ambition_combat::components::ActorFaction,
    team: Option<ambition_combat::targeting::MatchTeam>,
    driving: Option<ambition_characters::control::DrivingParticipant>,
    sim: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
}

impl FoeCandidate {
    /// May this body ambush `other`?
    ///
    /// ⛔ THE ONE RELATIONSHIP POLICY — the same `combat_relation` call the
    /// damage side makes, so a teammate cannot become a target here after
    /// ceasing to be one there. `Neutral` is not a foe: a body that is merely
    /// *hittable* is not someone this move was aimed at.
    fn may_ambush(&self, other: &FoeCandidate) -> bool {
        other.entity != self.entity
            && ambition_combat::targeting::combat_relation(
                None,
                self.faction,
                self.driving.as_ref(),
                self.team.as_ref(),
                None,
                other.entity,
                other.faction,
                other.driving.as_ref(),
                other.team.as_ref(),
            ) == ambition_combat::targeting::CombatRelation::Foe
    }

    /// The world y of this body's FEET (`+y` is gravity-down).
    fn feet_y(&self) -> f32 {
        self.pos.y + self.half.y
    }
}

/// Where an ambush arrives, and which way it looks once there.
struct Ambush {
    arrival: ae::Vec2,
    /// ⭐ TURNED TO FACE HIM. Arriving behind someone still looking the way you
    /// were thrown is an ambush that lands with its back to the fight, and every
    /// follow-up the move exists to set up would come out backwards.
    facing: f32,
}

/// Get behind the nearest foe within `reach`, or `None` when there is nobody to
/// get behind.
///
/// ⛔⛔ `reach` IS A RANGE, NOT A LEASH. It used to be a cap on how far the
/// teleport travelled, which sounds like the same rule and is not: a foe 900px
/// away with a 320px reach put the fighter 320px along the line to him —
/// *in front of* him, or inside him, having spent the move to walk into the
/// worst position on the stage. A foe past the reach is not a target, and the
/// move refuses exactly as it refuses when the stage is empty.
///
/// ⛔ EDGES, NOT CENTRES, ON BOTH AXES. `gap` is measured between the two
/// bodies' near faces, so the same authored number reads the same behind a
/// small body and a large one; and the arrival's FEET are placed at the foe's
/// feet rather than its centre, so ambushing someone twice her height does not
/// leave her buried to the waist or standing on his shoulders.
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
fn ambush_arrival(
    me: &FoeCandidate,
    candidates: &[FoeCandidate],
    reach: f32,
    gap: f32,
    facing: f32,
) -> Option<Ambush> {
    let mut ordered: Vec<&FoeCandidate> = candidates.iter().filter(|c| me.may_ambush(c)).collect();
    ordered.sort_by(|a, b| match (&a.sim, &b.sim) {
        (Some(x), Some(y)) => x.cmp(y),
        // A body with no `SimId` is not in the rollback sweep at all, so it
        // cannot be the source of a desync — but it still has to sort somewhere,
        // and last is the choice that leaves the tracked bodies' order untouched.
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.entity.cmp(&b.entity),
    });
    let mut best: Option<(f32, &FoeCandidate)> = None;
    for foe in ordered {
        let distance = me.pos.distance_squared(foe.pos);
        if distance > reach * reach {
            continue;
        }
        // Strictly nearer wins, so the `SimId` order above breaks an exact tie.
        if best.is_none_or(|(nearest, _)| distance < nearest) {
            best = Some((distance, foe));
        }
    }
    let (_, foe) = best?;
    // Behind = further along the line the attacker was already on. Directly
    // above or below him there is no such line, and the tiebreak is where she is
    // LOOKING: an ambush carries past him the way she was already facing.
    let side = if foe.pos.x > me.pos.x {
        1.0
    } else if foe.pos.x < me.pos.x {
        -1.0
    } else if facing != 0.0 {
        facing.signum()
    } else {
        1.0
    };
    Some(Ambush {
        arrival: ae::Vec2::new(
            foe.pos.x + side * (foe.half.x + me.half.x + gap),
            foe.feet_y() - me.half.y,
        ),
        facing: -side,
    })
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
    // THE CLASS-B LEDGER. `Option` for the same reason blink's is: a bare
    // fixture installs no log, and a teleport is still a teleport without one.
    mut class_b: Option<ResMut<ambition_platformer2d_shared_tangle::class_b::ClassBRemapLog>>,
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
        // WHERE, and how far. An ambush aims at a BODY rather than along the
        // stick, and it travels the whole way to the far side of him — but it is
        // clamped by exactly the same wall rule, because a teleport that could
        // pass through a stage to reach someone is a different move.
        let (dir, distance, turn_to) = if !params.behind_nearest_foe {
            (dir, params.distance, None)
        } else {
            // ⭐ HER OWN ROW IN THE CANDIDATE LIST carries the allegiance the
            // relation check needs; see [`FoeCandidate`].
            let Some(me) = candidates.iter().find(|c| c.entity == message.actor) else {
                continue;
            };
            // ⚠ NOBODY IN REACH, so the fighter stays put. See
            // `TeleportParams::behind_nearest_foe` — firing into empty space
            // spends the move to arrive somewhere nobody asked for.
            let Some(ambush) =
                ambush_arrival(me, &candidates, params.distance, params.behind_gap, facing)
            else {
                continue;
            };
            let offset = ambush.arrival - from;
            let length = offset.length();
            if length <= f32::EPSILON {
                continue;
            }
            (offset / length, length, Some(ambush.facing))
        };
        let solids = collision.get_or_insert_with(|| world.solids());
        let target = match solids.as_ref() {
            Some(w) => {
                let clamped = super::blink::blink_target(&**w, from, dir, distance, half);
                ledge_assisted_arrival(&**w, clamped, half, params.ledge_assist, gravity_dir)
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
        // ⭐ AND SHE TURNS. An ambush is a setup for the move that follows it;
        // landing behind him still facing the way she left would point every one
        // of those the wrong way. Only an ambush turns — an aimed recovery keeps
        // the facing the player was holding.
        if let Some(facing) = turn_to {
            clusters.kinematics.facing = facing;
        }
        // ⛔⛔ AND THE LEDGER HEARS ABOUT IT, at the write rather than near it.
        // A body that moves discontinuously without an entry reads to the
        // collision oracle as unexplained clipping, and a SECOND Class-B
        // authority remapping this body on the same frame becomes invisible to
        // the contention check — the two things that ledger exists for. Blink
        // has always recorded here; this road was added without it
        //.
        if let Some(log) = class_b.as_mut() {
            log.record(
                message.actor,
                ambition_platformer2d_shared_tangle::class_b::ClassBRemap::ScriptedTeleport,
            );
        }

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
