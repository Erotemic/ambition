//! A plate on the floor that throws whoever steps on it.
//!
//! A reusable launch object: it throws any body that touches it, owner and
//! opponent alike. A plate that served only its dropper would be a second
//! recovery.
//!
//! No owner is recorded, as for `LiveBomb`: whoever stands on it is the only
//! owner that matters. It also keeps an `Entity` out of rollback state.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_spring::{PlaceSpringParams, PLACE_SPRING};
use ambition_platformer2d::engine_core as ae;

/// A plate somebody dropped, and the two limits that spend it.
///
/// Rollback state: a restore without the clock and uses could replay a launch
/// the confirmed timeline already spent.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlacedSpring {
    /// The cosmetic row this plate draws when it lands and fires; see
    /// `PlaceSpringParams::vfx`.
    pub vfx: String,
    /// Where it sits.
    pub pos: ae::Vec2,
    /// Its size on the floor.
    pub half_extents: ae::Vec2,
    /// What it does to a body that touches it.
    pub launch: ae::Vec2,
    /// Seconds before it is taken away.
    pub remaining_s: f32,
    /// Launches left in it.
    pub uses_left: u8,
    /// Re-arm delay. A launch does not move a body out of the box on the same
    /// frame (the integrator runs later), so without this a plate would spend
    /// all its uses in three frames. With it, "three uses" means three separate
    /// launches.
    pub rearm_s: f32,
    /// Seconds before it will answer anybody, counted from the moment it lands.
    ///
    /// The plate lands 18px below the dropper's feet and the contact tolerance
    /// is 32, so the dropper is inside his own plate by construction. Without
    /// this he would launch himself on the drop tick. (The mine's arming delay
    /// and the bolt's `clear_of_caster` solve the same problem: a spawn point is
    /// inside the spawner.)
    pub arm_s: f32,
}

/// Checksum probe: the two spending limits and the re-arm. Position and launch
/// are constants copied from the move and cannot diverge.
pub fn placed_spring_probe(spring: &PlacedSpring) -> u64 {
    (spring.remaining_s.to_bits() as u64).rotate_left(17)
        ^ (spring.rearm_s.to_bits() as u64)
        ^ (spring.arm_s.to_bits() as u64).rotate_left(31)
        ^ u64::from(spring.uses_left)
}

/// Put a plate on the stage where a move asked for one.
pub fn drop_authored_springs(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    bodies: Query<&ae::BodyKinematics>,
    // The plate's announcement; see `PlaceSpringParams::vfx`.
    mut cues: MessageWriter<ambition_platformer2d::vfx::vfx::VfxMessage>,
    // The running match, so what this spawns dies with it (see
    // `crate::match_scope`).
    active_match: Option<Res<ambition_platformer2d::versus_match::ActiveMatch>>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != PLACE_SPRING {
            continue;
        }
        let params: PlaceSpringParams = match params.hydrate() {
            Ok(p) => p,
            Err(err) => {
                warn!("place-spring params did not hydrate: {err}");
                continue;
            }
        };
        let Ok(kin) = bodies.get(message.actor) else {
            continue;
        };
        // Body-local, mirrored by facing, like every other authored offset.
        let at = kin.pos + ae::Vec2::new(params.offset.0 * kin.facing.signum(), params.offset.1);
        info!(
            target: "ambition::moves",
            "plate dropped at {at:?} launch={:?} uses={}",
            params.launch, params.uses,
        );
        // Announce it: a plate nobody saw arrive is an ambush. See
        // `PlaceSpringParams::vfx`.
        if !params.vfx.trim().is_empty() {
            let row = &params.vfx;
            cues.write(ambition_platformer2d::vfx::vfx::VfxMessage::Effect {
                pos: at,
                fx: ambition_platformer2d::vfx::fx::FxId::new(row),
                scale: 1.0,
                pose: ambition_platformer2d::vfx::FxPose::UPRIGHT,
            });
        }
        let spawned = commands
            .spawn((
                Name::new("Placed spring"),
                PlacedSpring {
                    vfx: params.vfx.clone(),
                    pos: at,
                    half_extents: ae::Vec2::new(params.half_extents.0, params.half_extents.1),
                    launch: ae::Vec2::new(params.launch.0, params.launch.1),
                    remaining_s: params.lifetime_s,
                    uses_left: params.uses,
                    rearm_s: 0.0,
                    // Long enough for the engineer to step off his own plate.
                    arm_s: 0.30,
                },
            ))
            .id();
        // The match owns this object's end; see `crate::match_scope`.
        crate::match_scope::stamp(&mut commands, spawned, active_match.as_deref());
    }
}

/// Spend the clock, throw whoever is standing on it, and take it away when
/// either limit runs out.
///
/// One system, because clock, launch and removal are one decision per tick
/// (as `burn_fuses_and_answer_impacts` does for the bomb). Two systems could
/// launch twice.
pub fn fire_and_expire_springs(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut springs: Query<(Entity, &mut PlacedSpring)>,
    // `Entity` is here so the winner is chosen before anything moves: the
    // plate has one use, and two fighters can stand on it.
    mut bodies: Query<(
        Entity,
        &mut ae::BodyKinematics,
        &ambition_platformer2d::actor::MatchSeat,
    )>,
    // The plate's announcement when it throws somebody.
    mut cues: MessageWriter<ambition_platformer2d::vfx::vfx::VfxMessage>,
) {
    let dt = time.sim_dt();
    for (entity, mut spring) in &mut springs {
        spring.remaining_s -= dt;
        if spring.rearm_s > 0.0 {
            spring.rearm_s = (spring.rearm_s - dt).max(0.0);
        }
        if spring.arm_s > 0.0 {
            spring.arm_s = (spring.arm_s - dt).max(0.0);
        }
        if spring.remaining_s <= 0.0 || spring.uses_left == 0 {
            commands.entity(entity).despawn();
            continue;
        }
        if spring.rearm_s > 0.0 || spring.arm_s > 0.0 {
            continue;
        }
        // Anybody: the plate does not ask who dropped it (see the module note).
        //
        // It does ask which: the lowest seat wins. `MatchSeat` is
        // rollback-registered, so both peers resimulate the same launch, not a
        // query-order choice. Seat 0 is favoured in a tie; a rare unfair
        // outcome both peers agree on is better than a rare desync.
        let mut winner: Option<(usize, Entity)> = None;
        for (entity, kin, seat) in bodies.iter() {
            // The body's own half-size, so every fighter shape gets a matching
            // catch.
            let reach = spring.half_extents + kin.size * 0.5;
            let offset = (kin.pos - spring.pos).abs();
            if offset.x > reach.x || offset.y > reach.y {
                continue;
            }
            if winner.is_none_or(|(best, _)| seat.0 < best) {
                winner = Some((seat.0, entity));
            }
        }
        if let Some((seat, entity)) = winner {
            let Ok((_, mut kin, _)) = bodies.get_mut(entity) else {
                continue;
            };
            // Announce the firing, so the launched player can attribute the
            // throw.
            if !spring.vfx.is_empty() {
                cues.write(ambition_platformer2d::vfx::vfx::VfxMessage::Effect {
                    pos: spring.pos,
                    fx: ambition_platformer2d::vfx::fx::FxId::new(&spring.vfx),
                    scale: 1.0,
                    pose: ambition_platformer2d::vfx::FxPose::UPRIGHT,
                });
            }
            // Set, not add (see `motion::command_body_velocity`): an additive
            // plate would throw a fast-falling body less far than a walking one.
            crate::motion::command_body_velocity(&mut kin, spring.launch, "plate fired");
            spring.uses_left = spring.uses_left.saturating_sub(1);
            spring.rearm_s = 0.25;
            info!(
                target: "ambition::moves",
                "plate fired: seat={seat} {} use(s) left", spring.uses_left
            );
        }
    }
}

#[cfg(test)]
mod tests;
