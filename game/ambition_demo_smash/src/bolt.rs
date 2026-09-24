//! The steered bolt: a thing you fly with the same stick you walk with.
//!
//! The stick is read from the caster through `ActorControlFrame::steer_axis()`
//! ("what the player is holding", not what the body may move by). The damped
//! frame is republished after integration, so a rooted move reads `locomotion`
//! as zero. No seat is redirected and no brain is masked.
//!
//! The owner is a seat, as for the mine: `MatchSeat` is rollback-registered
//! and needs no entity remapping.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_bolt::{SteeredBoltParams, STEERED_BOLT};
use ambition_platformer2d::engine_core as ae;

/// A bolt in flight, and everything about it that moves.
///
/// Rollback state. Position, velocity and clock outlive the tick that made
/// them, and the bolt launches its caster, so a bad restore moves a fighter.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SteeredBolt {
    /// The cosmetic row drawn along its path; see
    /// `SteeredBoltParams::trail_vfx`. Without one the caster cannot see it.
    pub trail_vfx: String,
    /// Seconds between trail marks, and the countdown to the next one.
    pub trail_every_s: f32,
    pub trail_in_s: f32,
    /// Which seat is flying it.
    pub owner_seat: usize,
    /// Where it is.
    pub pos: ae::Vec2,
    /// Where it is going. Its length is the authored speed and never changes:
    /// the stick rotates it.
    pub vel: ae::Vec2,
    /// Seconds before it fades on its own.
    pub remaining_s: f32,
    /// Radians per second the stick may turn it.
    pub turn_rate: f32,
    /// Contact half-extent.
    pub radius: f32,
    pub damage: i32,
    pub knockback: f32,
    /// How hard it throws the caster when it comes home.
    pub self_launch: f32,
    /// Has this bolt got clear of its caster yet?
    ///
    /// The bolt spawns inside its caster's contact box. Without this latch it
    /// would come home on the firing tick and launch him at once. It cannot
    /// answer its caster until it has left him, so flying it back is a
    /// manoeuvre. Rollback state: it changes what the next contact means.
    pub clear_of_caster: bool,
}

/// Checksum probe: everything that moves, which for a bolt is most of it.
///
/// Position and velocity both, unlike the mine's clock-only probe: a steered
/// bolt's heading can diverge while its lifetime agrees.
pub fn steered_bolt_probe(bolt: &SteeredBolt) -> u64 {
    let mut h = (bolt.remaining_s.to_bits() as u64) ^ u64::from(bolt.clear_of_caster);
    for f in [bolt.pos.x, bolt.pos.y, bolt.vel.x, bolt.vel.y] {
        h = h.rotate_left(13) ^ (f.to_bits() as u64);
    }
    h
}

/// Put a bolt on the stage where a move asked for one.
pub fn fire_authored_bolts(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    casters: Query<(
        &ae::BodyKinematics,
        &ambition_platformer2d::actor::MatchSeat,
    )>,
    // The running match, so what this spawns dies with it (see
    // `crate::match_scope`).
    active_match: Option<Res<ambition_platformer2d::versus_match::ActiveMatch>>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != STEERED_BOLT {
            continue;
        }
        let params: SteeredBoltParams = match params.hydrate() {
            Ok(p) => p,
            Err(err) => {
                warn!("steered-bolt params did not hydrate: {err}");
                continue;
            }
        };
        // No seat, no bolt: nobody could steer it or be launched by it.
        let Ok((kin, seat)) = casters.get(message.actor) else {
            error!(
                "a bolt was fired by {:?}, which has no MatchSeat — nobody could \
                 steer it, so it is not fired at all",
                message.actor
            );
            continue;
        };
        let facing = kin.facing.signum();
        let at = kin.pos + ae::Vec2::new(params.offset.0 * facing, params.offset.1);
        // It leaves forward; the stick takes over on the next tick.
        let vel = ae::Vec2::new(facing * params.speed, 0.0);
        info!(
            target: "ambition::moves",
            "bolt fired: seat={} at {at:?} speed={} turn={}deg/s",
            seat.0, params.speed, params.turn_rate_deg,
        );
        let spawned = commands
            .spawn((
                Name::new("Steered bolt"),
                SteeredBolt {
                    trail_vfx: params.trail_vfx.clone(),
                    trail_every_s: params.trail_every_s,
                    // Zero, so the first mark lands on the spawn tick and the
                    // bolt is visible at once.
                    trail_in_s: 0.0,
                    owner_seat: seat.0,
                    pos: at,
                    vel,
                    remaining_s: params.lifetime_s,
                    turn_rate: params.turn_rate_deg.to_radians(),
                    radius: params.radius,
                    damage: params.damage,
                    knockback: params.knockback,
                    self_launch: params.self_launch,
                    // It starts inside him, by construction.
                    clear_of_caster: false,
                },
            ))
            .id();
        // The match owns this object's end; see `crate::match_scope`.
        crate::match_scope::stamp(&mut commands, spawned, active_match.as_deref());
    }
}

/// Fly every bolt: turn it by its owner's stick, move it, and answer whatever it
/// reaches.
///
/// One system, because steering, flight and contact are one decision per tick.
/// Split, contact would use a position no frame drew.
pub fn steer_and_fly_bolts(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut effects: MessageWriter<ambition_platformer2d::vfx::EffectRequest>,
    mut bolts: Query<(Entity, &mut SteeredBolt)>,
    // Contact does not require a control frame: a query wanting `ActorControl`
    // skips bodies without one, so a seated fighter without a frame would be
    // invisible to the bolt. Two queries: who stands here, and what that seat
    // holds.
    mut bodies: Query<(
        Entity,
        &mut ae::BodyKinematics,
        &ambition_platformer2d::actor::MatchSeat,
    )>,
    steering: Query<(
        &ambition_platformer2d::actor::MatchSeat,
        &ambition_platformer2d::characters::control::ActorControl,
    )>,
    // The bolt's trail; see `SteeredBoltParams::trail_vfx`.
    mut cues: MessageWriter<ambition_platformer2d::vfx::vfx::VfxMessage>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (entity, mut bolt) in &mut bolts {
        bolt.remaining_s -= dt;
        if bolt.remaining_s <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // The live stick. `steer_axis()` returns the pre-damp value while the
        // move damps him, and the live axis after the move ends (well before
        // the bolt fades). `locomotion` would be zero in the first case.
        //
        // Once he is free, one stick does both: walking right also steers the
        // bolt right. That is the move's cost.
        let steer = steering
            .iter()
            .find(|(seat, _)| seat.0 == bolt.owner_seat)
            .map(|(_, control)| control.0.steer_axis().vec())
            .unwrap_or(ae::Vec2::ZERO);
        if steer.length() > 0.2 {
            // Rotate toward the stick, never snap: a turn costs distance.
            let want = steer.normalize();
            let have = bolt.vel.normalize_or_zero();
            if have != ae::Vec2::ZERO {
                let cross = have.x * want.y - have.y * want.x;
                let dot = have.dot(want).clamp(-1.0, 1.0);
                let step = (bolt.turn_rate * dt).min(dot.acos());
                let (sin, cos) = (step * cross.signum()).sin_cos();
                let speed = bolt.vel.length();
                bolt.vel =
                    ae::Vec2::new(have.x * cos - have.y * sin, have.x * sin + have.y * cos) * speed;
            }
        }
        let step = bolt.vel * dt;
        bolt.pos += step;

        // Who did it reach? The caster's clearance is checked first, every
        // tick, whether or not anything else is in range.
        let mut spent = false;
        if !bolt.clear_of_caster {
            let still_inside = bodies.iter().any(|(_, kin, seat)| {
                if seat.0 != bolt.owner_seat {
                    return false;
                }
                // The same reach the contact test uses, so the bolt cannot
                // clear and then at once report a hit.
                let reach = ae::Vec2::splat(bolt.radius) + kin.size * 0.5;
                let offset = (kin.pos - bolt.pos).abs();
                offset.x <= reach.x && offset.y <= reach.y
            });
            if !still_inside {
                bolt.clear_of_caster = true;
            }
        }
        // A rival beats the caster: a bolt that can connect does the offensive
        // thing, and the Thunder Jacket happens when it finds nobody else.
        // Rivals tie-break on the lowest `MatchSeat` (rollback-registered), so
        // the choice does not depend on query order.
        let mut caster_hit: Option<Entity> = None;
        let mut rival_hit: Option<(usize, Entity)> = None;
        // Draw the trail; see `SteeredBoltParams::trail_vfx`.
        bolt.trail_in_s -= dt;
        if bolt.trail_in_s <= 0.0 {
            bolt.trail_in_s = bolt.trail_every_s;
            cues.write(ambition_platformer2d::vfx::vfx::VfxMessage::Effect {
                pos: bolt.pos,
                fx: ambition_platformer2d::vfx::fx::FxId::new(&bolt.trail_vfx),
                scale: 1.0,
                pose: ambition_platformer2d::vfx::FxPose::UPRIGHT,
            });
        }
        for (body, kin, seat) in bodies.iter() {
            let reach = ae::Vec2::splat(bolt.radius) + kin.size * 0.5;
            let offset = (kin.pos - bolt.pos).abs();
            if offset.x > reach.x || offset.y > reach.y {
                continue;
            }
            if seat.0 == bolt.owner_seat {
                if bolt.clear_of_caster {
                    caster_hit = Some(body);
                }
            } else if rival_hit.is_none_or(|(best, _)| seat.0 < best) {
                rival_hit = Some((seat.0, body));
            }
        }
        let chosen = rival_hit.map(|(_, e)| e).or(caster_hit);
        // The caster owns the blast, not the body struck: kills, grudges and
        // staleness key on the owner. Resolved by seat, because bevy_ggrs
        // recreates entities across a rollback.
        let caster_entity = bodies
            .iter()
            .find(|(_, _, seat)| seat.0 == bolt.owner_seat)
            .map(|(e, _, _)| e);
        for (body, mut kin, seat) in &mut bodies {
            if Some(body) != chosen {
                continue;
            }
            // The body's own half-size, not a fixed constant, so fighters of
            // any size get a matching contact box.
            let reach = ae::Vec2::splat(bolt.radius) + kin.size * 0.5;
            let offset = (kin.pos - bolt.pos).abs();
            if offset.x > reach.x || offset.y > reach.y {
                continue;
            }
            if seat.0 == bolt.owner_seat {
                // Not yet: it has not left him, so this is the firing frame.
                if !bolt.clear_of_caster {
                    continue;
                }
                // The Thunder Jacket: he flies his own bolt into his back and it
                // carries him, so the move is also a recovery. `self_launch`
                // decides its worth offstage. It uses the bolt's direction, not
                // the stick: the player already aimed by flying it.
                let push = bolt.vel.normalize_or_zero() * bolt.self_launch;
                crate::motion::command_body_velocity(&mut kin, push, "thunder jacket");
                info!(target: "ambition::moves", "bolt came home: seat={} push={push:?}", seat.0);
            } else {
                effects.write(ambition_platformer2d::vfx::EffectRequest {
                    // The caster, not the body struck; see `caster_entity`. The
                    // victim only if the caster has left the match.
                    owner: caster_entity.unwrap_or(body),
                    effect: ambition_platformer2d::vfx::Effect::DamageBox(
                        ambition_platformer2d::vfx::DamageBoxEffect {
                            center: bolt.pos,
                            // A caster standing in the blast is caught by it.
                            // `Environment`, not `Neutral`: `Environment` has no
                            // self-exclusion, and `Neutral` never spawns a
                            // damaging hitbox.
                            faction: ambition_platformer2d::vfx::HitSide::Environment,
                            half_extent: ae::Vec2::splat(bolt.radius),
                            damage: bolt.damage,
                            knockback: bolt.knockback,
                            lifetime_s: 0.06,
                            name: Some("bolt"),
                        },
                    ),
                });
            }
            spent = true;
            break;
        }
        if spent {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests;
