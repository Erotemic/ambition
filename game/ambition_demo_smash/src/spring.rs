//! A plate on the floor that throws whoever steps on it.
//!
//! ⭐⭐ THE CAMPAIGN'S "reusable launch object", and the word that matters is
//! REUSABLE: it throws ANY body that touches it, its owner included and its
//! owner's opponent included. A plate that served only the fighter who dropped it
//! would be a second recovery wearing an object's clothes.
//!
//! ⛔ NO OWNER IS RECORDED — the ruling `LiveBomb` makes about itself, for the
//! same reason: a thing on the floor belongs to whoever is standing on it, so
//! "whose plate is this" has no answer anybody would act on. ⇒ It also keeps an
//! `Entity` out of rollback state, which would otherwise need remapping for a
//! fact nothing reads.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_spring::{PlaceSpringParams, PLACE_SPRING};
use ambition_platformer2d::engine_core as ae;

/// A plate somebody dropped, and the two limits that spend it.
///
/// ⛔ ROLLBACK STATE. Both the clock and the remaining uses outlive the tick that
/// made them, so a rewind that restored the plate without them would give the
/// resimulated timeline a launch the confirmed one had already spent — and a
/// launch is a fighter standing somewhere else.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlacedSpring {
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
    /// ⛔⛔ WITHOUT THIS IT FIRES EVERY TICK YOU STAND ON IT. A launch does not
    /// move a body out of the plate's box on the frame it happens — the velocity
    /// is applied and the integrator runs later — so a plate that re-armed
    /// immediately would spend all its uses in three frames and read as one
    /// enormous launch. ⇒ A short re-arm is what makes "three uses" mean three
    /// separate people, or one person three times.
    pub rearm_s: f32,
    /// Seconds before it will answer ANYBODY, counted from the moment it lands.
    ///
    /// ⛔⛔ WITHOUT IT THE DROPPER LAUNCHES HIMSELF ON THE TICK HE DROPS IT, and
    /// a guard found it on the first run. The plate lands at a body-local offset
    /// — 18px below his feet — and the contact tolerance is 32, so **he is inside
    /// his own plate by construction**. ⇒ One of its three uses would be spent
    /// throwing the engineer straight up before anybody saw the plate.
    ///
    /// ⭐⭐ THIRD TIME TODAY THIS EXACT SHAPE HAS BITTEN: the mine needed an
    /// arming delay, the bolt needed a `clear_of_caster` latch, and now this.
    /// **A SPAWN POINT IS INSIDE THE SPAWNER**, and contact logic that does not
    /// say so is wrong on frame one — the only frame that runs before anything
    /// else can.
    pub arm_s: f32,
}

/// Checksum probe: the two spending limits and the re-arm, which are what a peer
/// can disagree about. ⛔ Not the position or the launch — those are constants
/// copied off the move and cannot diverge.
pub fn placed_spring_probe(spring: &PlacedSpring) -> u64 {
    (spring.remaining_s.to_bits() as u64)
        .rotate_left(17)
        ^ (spring.rearm_s.to_bits() as u64)
        ^ (spring.arm_s.to_bits() as u64).rotate_left(31)
        ^ u64::from(spring.uses_left)
}

/// Put a plate on the stage where a move asked for one.
pub fn drop_authored_springs(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    bodies: Query<&ae::BodyKinematics>,
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
        commands.spawn((
            Name::new("Placed spring"),
            PlacedSpring {
                pos: at,
                half_extents: ae::Vec2::new(params.half_extents.0, params.half_extents.1),
                launch: ae::Vec2::new(params.launch.0, params.launch.1),
                remaining_s: params.lifetime_s,
                uses_left: params.uses,
                rearm_s: 0.0,
                // Long enough for the engineer to step off his own plate.
                arm_s: 0.30,
            },
        ));
    }
}

/// Spend the clock, throw whoever is standing on it, and take it away when
/// either limit runs out.
///
/// ⛔⛔ ONE SYSTEM, because the clock, the launch and the removal are one decision
/// about one tick — the same reasoning `burn_fuses_and_answer_impacts` gives for
/// the bomb. Two systems racing to despawn one plate is how a launch happens
/// twice.
pub fn fire_and_expire_springs(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut springs: Query<(Entity, &mut PlacedSpring)>,
    // ⛔ `Entity` IS IN HERE SO THE WINNER CAN BE CHOSEN BEFORE IT IS MOVED.
    // The plate has ONE use to give and two fighters can stand on it, so the
    // candidates are gathered read-only and the seat decides — see below.
    mut bodies: Query<(
        Entity,
        &mut ae::BodyKinematics,
        &ambition_platformer2d::actor::MatchSeat,
    )>,
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
        // ⭐ ANYBODY. The plate does not ask who dropped it — see the module note.
        //
        // ⛔⛔ BUT IT DOES ASK *WHICH*, AND IT USED TO ANSWER BY QUERY ORDER. A
        // plate has ONE use to give; this loop broke on the first overlapping
        // body and ignored the seat entirely (`_seat`). Two fighters standing on
        // it on the same tick meant Bevy's iteration order chose who got launched
        // — which is not a decision anybody authored, is not stable across a
        // rollback resimulation, and is exactly the class `assisted_fire_direction`
        // already solved for aim assist by tie-breaking on a stable id.
        //
        // ⭐ THE LOWEST SEAT WINS, and the rule matters more than the winner: a
        // `MatchSeat` is rollback-registered, so both peers resimulate the same
        // launch. ⚠ It is arbitrary as FAIRNESS — in a tie, seat 0 is favoured —
        // and that is accepted deliberately: two bodies inside one plate on one
        // tick is rare, and a rare unfair outcome both peers agree on is better
        // than a rare desync.
        let mut winner: Option<(usize, Entity)> = None;
        for (entity, kin, seat) in bodies.iter() {
            // ⛔⛔ THE BODY'S OWN HALF-SIZE, NOT A NUMBER I PICKED. This read
            // `+ 14.0 / + 26.0` — invented constants standing in for a
            // fighter's extent, on a component that CARRIES it. ⇒ The plate's
            // catch was sized for one body shape and every other fighter got a
            // different plate. ⭐ The shape, named by a peer: two things agree on
            // a POSITION and disagree on a TOLERANCE.
            let reach = spring.half_extents + kin.size * 0.5;
            let offset = (kin.pos - spring.pos).abs();
            if offset.x > reach.x || offset.y > reach.y {
                continue;
            }
            // ⛔ SET, NOT ADD — see `motion::command_body_velocity`, which owns
            // that argument and the ADR-0024 ownership claim behind it. A plate
            // that added to whatever you arrived with would throw a fast-falling
            // body less far than a walking one.
            if winner.is_none_or(|(best, _)| seat.0 < best) {
                winner = Some((seat.0, entity));
            }
        }
        if let Some((seat, entity)) = winner {
            let Ok((_, mut kin, _)) = bodies.get_mut(entity) else {
                continue;
            };
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
