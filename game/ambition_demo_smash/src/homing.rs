//! The homing dash: the fighter is carried at whoever they were pointing at.
//!
//! ⭐⭐ THIS MODULE OWNS NO TARGETING. Every tick it asks
//! `ambition_combat::targeting::assisted_fire_direction` — the same call the
//! pirate's gun-sword uses — and steers on the answer. ⇒ That keeps the one
//! property a homing move must not get wrong: the tie-break is the stable
//! `SimId`, never the `Entity`, because bevy_ggrs recreates rollback entities
//! and a tie decided by a raw id picks a DIFFERENT target mid-resimulation than
//! the confirmed timeline did.
//!
//! ⛔ IT ASKS EVERY TICK RATHER THAN LATCHING A TARGET, and that is the design:
//! a latched target is a homing missile, and re-asking makes the dash follow the
//! cone rather than a person. A foe who leaves the cone stops attracting it, so
//! the move can still be dodged by moving — which is what makes it a read.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_homing::{HomingDashParams, HOMING_DASH};
use ambition_platformer2d::engine_core as ae;

/// A fighter currently being carried at a target.
///
/// ⛔ ROLLBACK STATE. The clock outlives the tick that made it and it decides
/// where a fighter IS, so a rewind that restored the dash without its clock
/// leaves the two peers' fighters in different places.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct HomingDash {
    /// Seconds of homing left.
    pub remaining_s: f32,
    /// How fast it carries.
    pub speed: f32,
    /// The cone, in radians.
    pub cone_rad: f32,
    /// How far a foe may be and still attract it.
    pub max_range: f32,
    /// The direction the fighter committed to when the dash began.
    ///
    /// ⭐ REMEMBERED, NOT RE-READ. The cone is measured from what the player
    /// COMMANDED at the press — re-reading the stick each tick would let them
    /// sweep the cone across the stage and turn a read into a search.
    pub commanded: ae::Vec2,
}

/// Checksum probe: the clock and the committed direction — the two facts a peer
/// can disagree about. ⛔ Speed, cone and range are constants copied off the
/// move and cannot diverge.
pub fn homing_dash_probe(dash: &HomingDash) -> u64 {
    (dash.remaining_s.to_bits() as u64).rotate_left(19)
        ^ (dash.commanded.x.to_bits() as u64)
        ^ (dash.commanded.y.to_bits() as u64).rotate_left(7)
}

/// Begin a homing dash where a move asked for one.
pub fn begin_authored_homing_dashes(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    bodies: Query<&ae::BodyKinematics>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != HOMING_DASH {
            continue;
        }
        let params: HomingDashParams = match params.hydrate() {
            Ok(p) => p,
            Err(err) => {
                warn!("homing-dash params did not hydrate: {err}");
                continue;
            }
        };
        let Ok(kin) = bodies.get(message.actor) else {
            continue;
        };
        // ⭐ THE FACING IS THE COMMAND. A special is authored with a direction
        // the fighter is already committed to — the same reading every
        // body-local `offset` in this repository uses — so the cone opens the
        // way they are pointing rather than the way the stick happens to be.
        let commanded = ae::Vec2::new(kin.facing.signum(), 0.0);
        info!(
            target: "ambition::moves",
            "homing dash: from {:?} toward {commanded:?} for {}s",
            kin.pos, params.duration_s,
        );
        commands.entity(message.actor).try_insert(HomingDash {
            remaining_s: params.duration_s,
            speed: params.speed,
            cone_rad: params.cone_degrees.to_radians(),
            max_range: params.max_range,
            commanded,
        });
    }
}

/// Carry each homing fighter, and stop when its clock runs out.
pub fn carry_homing_dashes(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    // ⛔⛔ ONE QUERY OVER ALL BODIES, NOT A DASHER QUERY BESIDE A CANDIDATE ONE.
    // The dasher IS a body, so two queries both touching `BodyKinematics` — one
    // `&mut`, one `&` — are a `B0001` access conflict. ⇒ Merging is the right
    // fix rather than `Without<HomingDash>`, which would quietly make one
    // homing fighter unable to target another.
    mut bodies: Query<(
        Entity,
        &mut ae::BodyKinematics,
        Option<&mut HomingDash>,
        Option<&ambition_platformer2d::platformer::sim_id::SimId>,
    )>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // ⭐ GATHER FIRST, APPLY SECOND. The candidate set is read from the same
    // query, immutably, before anything moves — so every dash this tick steers
    // against the SAME world rather than against the partially-updated one its
    // predecessors left.
    let candidates: Vec<(
        Entity,
        Option<ambition_platformer2d::platformer::sim_id::SimId>,
        ae::Vec2,
    )> = bodies
        .iter()
        .map(|(entity, kin, _, sim_id)| (entity, sim_id.cloned(), kin.pos))
        .collect();

    for (entity, mut kin, dash, _) in &mut bodies {
        let Some(mut dash) = dash else {
            continue;
        };
        dash.remaining_s -= dt;
        if dash.remaining_s <= 0.0 {
            commands.entity(entity).try_remove::<HomingDash>();
            continue;
        }
        let from = kin.pos;
        let others: Vec<_> = candidates
            .iter()
            .filter(|(other, _, _)| *other != entity)
            .cloned()
            .collect();
        let heading = ambition_platformer2d::combat::targeting::assisted_fire_direction(
            from,
            dash.commanded,
            ambition_platformer2d::characters::brain::action_set::AimAssist {
                max_angle_rad: dash.cone_rad,
                max_range: dash.max_range,
            },
            others,
        );
        // ⛔ SET, NOT ADD — the dash IS the fighter's motion for its duration,
        // and adding would make a running start into a faster homing move.
        kin.vel = heading.normalize_or_zero() * dash.speed;
    }
}

#[cfg(test)]
mod tests;
