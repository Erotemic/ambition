//! The steered bolt: a thing you fly with the stick while standing still.
//!
//! ⭐⭐ THE STICK IS READ FROM THE CASTER, WHO NEVER STOPS BEING THE CASTER.
//! `ActorControlFrame::steer_axis()` is *"what the PLAYER is HOLDING, as opposed
//! to what this body is ALLOWED to move by"* — it exists because the damped
//! frame is republished after integration, so a rooted move reads `locomotion`
//! as zero. ⇒ No seat is redirected, no brain is masked, nothing owns the
//! player's input but the player. The move COORDINATES two authorities and
//! becomes neither.
//!
//! ⛔ THE OWNER IS A SEAT, for the same reason the mine's is: `MatchSeat` is
//! rollback-registered, survives a rewind unchanged, and needs no entity
//! remapping. "Whose bolt is this" is a `usize` comparison.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_bolt::{SteeredBoltParams, STEERED_BOLT};
use ambition_platformer2d::engine_core as ae;

/// A bolt in flight, and everything about it that moves.
///
/// ⛔ ROLLBACK STATE. Position, velocity and clock all outlive the tick that made
/// them, so a rewind that restored the bolt without them would fly a different
/// bolt from the confirmed one — and since the bolt LAUNCHES ITS CASTER, that is
/// a divergence in where a fighter is standing.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SteeredBolt {
    /// Which seat is flying it.
    pub owner_seat: usize,
    /// Where it is.
    pub pos: ae::Vec2,
    /// Where it is going. Its LENGTH is the authored speed and never changes —
    /// the stick rotates this and cannot lengthen it.
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
    /// Has this bolt got CLEAR of its caster yet?
    ///
    /// ⛔⛔ WITHOUT THIS THE MOVE IS UNPLAYABLE, and a guard found it on the
    /// first run rather than a player finding it in a match: the bolt spawns at
    /// a body-local offset, which is INSIDE its caster's contact box, so it came
    /// home on the very tick it was fired and threw him instantly. ⇒ Every press
    /// was a self-launch and the bolt was never seen.
    ///
    /// ⭐ IT IS ALSO THE GENRE'S RULE, arrived at from the bug rather than from
    /// the reference: the bolt cannot answer its caster until it has left him,
    /// so flying it back is a MANOEUVRE and not an accident of where it starts.
    ///
    /// ⛔ ROLLBACK STATE, and it earns its place in the probe: it is a latch that
    /// flips once and changes what the next contact MEANS, so a restore that lost
    /// it turns a resimulated recovery into a whiff.
    pub clear_of_caster: bool,
}

/// Checksum probe: everything that moves, which for a bolt is most of it.
///
/// ⛔ POSITION AND VELOCITY BOTH, unlike the mine's clock-only probe. A bolt is
/// STEERED, so two peers can disagree about where it is going while agreeing
/// about how long it has left — and the divergence that matters is the one that
/// decides whether it comes home.
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
    casters: Query<(&ae::BodyKinematics, &ambition_platformer2d::actor::MatchSeat)>,
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
        // ⛔ NO SEAT, NO BOLT. A bolt whose owner cannot be named is one nobody
        // can steer and nobody can be launched by — it would fly straight out of
        // the world on its opening velocity forever.
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
        // ⭐ IT LEAVES FORWARD, not upward. The stick takes over on the very next
        // tick, so the opening direction only has to be somewhere the caster can
        // steer FROM — and forward is the one that reads as "he threw it".
        let vel = ae::Vec2::new(facing * params.speed, 0.0);
        info!(
            target: "ambition::moves",
            "bolt fired: seat={} at {at:?} speed={} turn={}deg/s",
            seat.0, params.speed, params.turn_rate_deg,
        );
        commands.spawn((
            Name::new("Steered bolt"),
            SteeredBolt {
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
        ));
    }
}

/// Fly every bolt: turn it by its owner's stick, move it, and answer whatever it
/// reaches.
///
/// ⛔⛔ ONE SYSTEM, because steering, flight and contact are one decision about
/// one tick. A bolt that turned in one system and moved in another would answer
/// contact against a position no frame ever drew.
pub fn steer_and_fly_bolts(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut effects: MessageWriter<ambition_platformer2d::vfx::EffectRequest>,
    mut bolts: Query<(Entity, &mut SteeredBolt)>,
    // ⛔⛔ CONTACT DOES NOT REQUIRE A CONTROL FRAME, and joining them would have
    // been a silent no-op waiting to happen: a query wanting `ActorControl` skips
    // any body that lacks one ENTIRELY, so a seated fighter without a control
    // frame would be invisible to the bolt rather than merely unable to steer it.
    // ⇒ Two queries, because they answer two questions — "who is standing here"
    // and "what is that seat holding" — and only the second needs the frame.
    mut bodies: Query<(
        Entity,
        &mut ae::BodyKinematics,
        &ambition_platformer2d::actor::MatchSeat,
    )>,
    steering: Query<(
        &ambition_platformer2d::actor::MatchSeat,
        &ambition_platformer2d::characters::control::ActorControl,
    )>,
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

        // ⭐ THE LIVE STICK, from the caster who is still standing there. Reading
        // `locomotion` here would find zero every tick, because the move that
        // fired this is rooted — which is exactly the trap `steer_axis` exists
        // to keep techniques out of.
        let steer = steering
            .iter()
            .find(|(seat, _)| seat.0 == bolt.owner_seat)
            .map(|(_, control)| control.0.steer_axis().vec())
            .unwrap_or(ae::Vec2::ZERO);
        if steer.length() > 0.2 {
            // ⛔ ROTATE TOWARD, NEVER SNAP TO. A bolt that took the stick's
            // direction outright would be a cursor, and the whole tension of the
            // move is that a turn costs distance.
            let want = steer.normalize();
            let have = bolt.vel.normalize_or_zero();
            if have != ae::Vec2::ZERO {
                let cross = have.x * want.y - have.y * want.x;
                let dot = have.dot(want).clamp(-1.0, 1.0);
                let step = (bolt.turn_rate * dt).min(dot.acos());
                let (sin, cos) = (step * cross.signum()).sin_cos();
                let speed = bolt.vel.length();
                bolt.vel = ae::Vec2::new(
                    have.x * cos - have.y * sin,
                    have.x * sin + have.y * cos,
                ) * speed;
            }
        }
        let step = bolt.vel * dt;
        bolt.pos += step;

        // WHO DID IT REACH?
        //
        // ⭐ THE CASTER'S OWN CLEARANCE IS CHECKED FIRST AND SEPARATELY, because
        // "is it still inside me" has to be answered every tick whether or not
        // anything else is in range.
        let mut spent = false;
        if !bolt.clear_of_caster {
            let still_inside = bodies.iter().any(|(_, kin, seat)| {
                if seat.0 != bolt.owner_seat {
                    return false;
                }
                let offset = (kin.pos - bolt.pos).abs();
                offset.x <= bolt.radius + 16.0 && offset.y <= bolt.radius + 24.0
            });
            if !still_inside {
                bolt.clear_of_caster = true;
            }
        }
        for (body, mut kin, seat) in &mut bodies {
            let offset = (kin.pos - bolt.pos).abs();
            if offset.x > bolt.radius + 16.0 || offset.y > bolt.radius + 24.0 {
                continue;
            }
            if seat.0 == bolt.owner_seat {
                // ⛔ NOT YET. It has not left him, so this is the frame it was
                // fired on rather than the frame it came home.
                if !bolt.clear_of_caster {
                    continue;
                }
                // ⭐⭐ THE THUNDER JACKET. He flies his own bolt into his back and
                // it carries him — which is why this move is a recovery as much
                // as an attack, and why `self_launch` is the number that decides
                // whether it is worth using offstage.
                //
                // ⛔ THE BOLT'S OWN DIRECTION, not the stick's: the player aimed
                // by flying it, and re-reading the stick at the impact would let
                // them aim twice.
                let push = bolt.vel.normalize_or_zero() * bolt.self_launch;
                kin.vel = push;
                info!(target: "ambition::moves", "bolt came home: seat={} push={push:?}", seat.0);
            } else {
                effects.write(ambition_platformer2d::vfx::EffectRequest {
                    owner: body,
                    effect: ambition_platformer2d::vfx::Effect::DamageBox(
                        ambition_platformer2d::vfx::DamageBoxEffect {
                            center: bolt.pos,
                            // ⛔ THE CASTER/FOE DISTINCTION IS MADE ABOVE, NOT
                            // HERE. `HitSide` has no "everyone but my owner" —
                            // its arms are Player/Enemy/Npc/Boss/Neutral — and it
                            // does not need one: the branch this box sits in only
                            // runs for a body that is NOT the owner's seat, and
                            // the owner's contact returns the launch instead. ⇒ A
                            // caster standing inside their own bolt's blast is
                            // caught by it, which is the honest reading of being
                            // there.
                            faction: ambition_platformer2d::vfx::HitSide::Neutral,
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
