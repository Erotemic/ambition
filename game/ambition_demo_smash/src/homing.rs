//! The homing dash: the fighter is carried at whoever they were pointing at.
//!
//! This module owns no targeting. Every tick it asks
//! `ambition_combat::targeting::assisted_fire_direction` (as the pirate's
//! gun-sword does) and steers on the answer. That keeps ties broken by the
//! stable `SimId`, never `Entity`: bevy_ggrs recreates entities, so an `Entity`
//! tie-break could pick a different target mid-resimulation.
//!
//! It asks every tick instead of latching a target: the dash follows the cone,
//! not a person. A foe who leaves the cone stops attracting it, so the move can
//! be dodged.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_homing::{HomingDashParams, HOMING_DASH};
use ambition_platformer2d::engine_core as ae;

/// A fighter currently being carried at a target.
///
/// Rollback state: the clock decides where a fighter is.
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
    /// Remembered, not re-read: the cone is measured from the press. Re-reading
    /// the stick each tick would let the player sweep the cone across the stage.
    pub commanded: ae::Vec2,
}

/// Checksum probe: the clock and the committed direction. Speed, cone and range
/// are constants copied from the move and cannot diverge.
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
        // The facing is the command, as for every body-local `offset`: the cone
        // opens the way they point, not the way the stick happens to be.
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
    // One query over all bodies: a dasher query beside a candidate query would
    // both touch `BodyKinematics` (`B0001`). `Without<HomingDash>` would stop
    // one homing fighter from targeting another.
    mut bodies: Query<(
        Entity,
        &mut ae::BodyKinematics,
        Option<&mut HomingDash>,
        Option<&ambition_platformer2d::platformer::sim_id::SimId>,
        // Eligibility: "not me" is not "a foe". `assisted_fire_direction` is
        // geometric and assumes the caller supplies foes, so KO'd (`OutOfPlay`)
        // bodies and teammates must be filtered here.
        Option<&ambition_platformer2d::characters::actor::BodyHealth>,
        (
            bevy::prelude::Has<ambition_platformer2d::combat::death_rules::OutOfPlay>,
            Option<&ambition_platformer2d::engine_core::DepthPlane>,
        ),
        Option<&ambition_platformer2d::combat::components::ActorFaction>,
        Option<&ambition_platformer2d::combat::targeting::MatchTeam>,
    )>,
    // The faction matrix: targeting is relational and wants it. The damage
    // side passes `None` to `combat_relation` because damage is physical.
    relations: Option<Res<ambition_platformer2d::combat::targeting::FactionRelations>>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // Gather first, apply second: every dash this tick steers against the same
    // world, not one its predecessors partly updated.
    #[allow(clippy::type_complexity)]
    let candidates: Vec<(
        Entity,
        Option<ambition_platformer2d::platformer::sim_id::SimId>,
        ae::Vec2,
        // The eligibility inputs, captured with the position on the same tick.
        bool,
        Option<ambition_platformer2d::combat::components::ActorFaction>,
        Option<ambition_platformer2d::combat::targeting::MatchTeam>,
    )> = bodies
        .iter()
        .map(
            |(entity, kin, _, sim_id, health, (out_of_play, plane), faction, team)| {
                (
                    entity,
                    sim_id.cloned(),
                    kin.pos,
                    ambition_platformer2d::combat::util::body_is_untouchable(health, out_of_play, plane),
                    faction.copied(),
                    team.cloned(),
                )
            },
        )
        .collect();
    for (entity, mut kin, dash, _, _, _, self_faction, self_team) in &mut bodies {
        let Some(mut dash) = dash else {
            continue;
        };
        dash.remaining_s -= dt;
        if dash.remaining_s <= 0.0 {
            commands.entity(entity).try_remove::<HomingDash>();
            continue;
        }
        let from = kin.pos;
        // A foe, and "foe" is the targeting question, not the damage one.
        // `CombatRelation::damage_lands` is true for `Foe` and `Neutral`;
        // `is_target` is true for `Foe` only. A neutral bystander is damageable
        // but not a target, so it must not win the cone over the opponent.
        //
        // Seated fighters are unaffected: each seat has its own `MatchTeam`
        // (`prepared::team_for`), and team relation outranks faction, so
        // opponents are `Foe` and teammates are `Ally`.
        //
        // Friendly fire does not participate: it decides whether a swing hurts
        // an ally, not whether a dash hunts one.
        let self_faction = self_faction.copied().unwrap_or_default();
        let others: Vec<_> = candidates
            .iter()
            .filter(|(other, _, _, untouchable, faction, team)| {
                *other != entity
                    && !*untouchable
                    && ambition_platformer2d::combat::targeting::combat_relation(
                        relations.as_deref(),
                        self_faction,
                        None,
                        self_team,
                        None,
                        *other,
                        faction.unwrap_or_default(),
                        None,
                        team.as_ref(),
                    )
                    .is_target()
            })
            .map(|(other, sim_id, pos, _, _, _)| (*other, sim_id.clone(), *pos))
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
        // Set, not add: the dash is the fighter's motion while it lasts, so a
        // running start does not make it faster. See
        // `motion::command_body_velocity`.
        crate::motion::command_body_velocity(
            &mut kin,
            heading.normalize_or_zero() * dash.speed,
            "homing dash",
        );
    }
}

#[cfg(test)]
mod tests;
