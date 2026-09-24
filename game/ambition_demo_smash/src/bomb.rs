//! A live bomb on the stage: the ruleset half of `smash.drop_bomb`.
//!
//! Jon's design: the Projectile Polygon drops a bomb onto the stage; fighters
//! can pick it up and throw it; it detonates after 4 seconds or on a hit with
//! enough velocity, whichever comes first.
//!
//! It is a ground item, so pickup and throw come free: `GroundItem`,
//! `ItemCustody`, `pickup_held_item_system` and `throw_held_item_system` are
//! installed by `ItemPickupSimulationPlugin` in every game. A summoned body
//! (the shark's road) would need all of them written again.
//!
//! The impact rule uses what item physics publishes: `ground_item_physics`
//! stamps `SettledItem` (with its `impact_speed`) on the tick geometry blocks
//! an item.
//!
//! Any driven body can take and throw one: both systems take `driven:
//! DrivenBodies` (`ControlledSubject` plus every `DrivingParticipant`), and
//! `ItemCustody::Held { holder }` is keyed by the holding body. Whether the
//! fighter brain ever requests a pickup is unmeasured.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_bomb::{DropBombParams, DROP_BOMB};
use ambition_platformer2d::engine_core as ae;

/// How hard a detonation throws what it catches, as the feel multiplier
/// `DamageBoxEffect::knockback` is (shockwave 1.3, beam 1.1, boss blast 1.6).
/// Above the boss's: a bomb you can see and hear for four seconds should hit
/// harder than an unannounced hazard.
const BLAST_FEEL_SCALE: f32 = 2.0;

/// A ground item with a fuse.
///
/// Rollback state: a restore without the fuse gives a different explosion.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct LiveBomb {
    /// Seconds left before it goes off by itself.
    pub fuse_s: f32,
    /// Damage at the centre.
    pub damage: i32,
    /// How far the blast reaches.
    pub blast_radius: f32,
    /// At or above this speed, contact IS the detonation.
    pub impact_speed: f32,
}

// No owner handle: a live bomb changes hands, so the blast is attributed to
// the bomb. It also keeps an `Entity` out of rollback state.

/// Recognise the authored bomb drop and put the object on the stage.
pub fn translate_bomb_drops(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    bodies: Query<&ae::BodyKinematics>,
    // The running match, so what this spawns dies with it (see
    // `crate::match_scope`).
    active_match: Option<Res<ambition_platformer2d::versus_match::ActiveMatch>>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != DROP_BOMB {
            continue;
        }
        let params = match params.hydrate::<DropBombParams>() {
            Ok(params) => params,
            Err(err) => {
                warn!("drop-bomb params did not hydrate: {err}");
                continue;
            }
        };
        let Some(held) = ambition_platformer2d::characters::brain::held_item_by_id(&params.item_id)
        else {
            // An error, not a warning: an unregistered id cannot be picked up,
            // and a bomb nobody can pick up is half the move.
            error!(
                "move drops `{}`, which is not a registered held item — nobody \
                 could pick this bomb up, so it is not dropped at all",
                params.item_id
            );
            continue;
        };
        let Ok(kin) = bodies.get(message.actor) else {
            continue;
        };
        // Body-local, mirrored by facing, like every other authored offset.
        let at = kin.pos + ae::Vec2::new(params.offset.0 * kin.facing.signum(), params.offset.1);
        let half = ae::Vec2::new(params.half_extents.0, params.half_extents.1);
        info!(
            target: "ambition::moves",
            "bomb dropped: owner={:?} item=`{}` at {at:?} fuse={}s",
            message.actor, params.item_id, params.fuse_s,
        );
        let spawned = commands
            .spawn((
                Name::new(format!("Live bomb: {}", params.item_id)),
                ambition_platformer2d::item::GroundItem::at_rest(held, at, half),
                LiveBomb {
                    fuse_s: params.fuse_s,
                    damage: params.damage,
                    blast_radius: params.blast_radius,
                    impact_speed: params.impact_speed,
                },
            ))
            .id();
        // The match owns this object's end; see `crate::match_scope`.
        crate::match_scope::stamp(&mut commands, spawned, active_match.as_deref());
    }
}

/// Burn the fuse, remember the speed, and go off for whichever reason arrives
/// first.
///
/// One system for both reasons, so the bomb explodes once. Two systems could
/// double-blast when a thrown bomb's fuse also runs out.
pub fn burn_fuses_and_answer_impacts(
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut commands: Commands,
    mut effects: MessageWriter<ambition_platformer2d::vfx::EffectRequest>,
    mut bombs: Query<(
        Entity,
        &mut LiveBomb,
        &ambition_platformer2d::item::GroundItem,
        &ambition_platformer2d::item::ItemCustody,
        Option<&ambition_platformer2d::item::SettledItem>,
        // The other hard contact: a bomb that reaches a fighter at speed
        // also counts (Jon's rule names no surface).
        Option<&ambition_platformer2d::item::ItemStruckBody>,
    )>,
    // Where the bomb is, whoever has it. `GroundItem::pos` stops updating once
    // the bomb is picked up; `ItemWorldPos` follows the holder.
    where_it_is: ambition_platformer2d::item::ItemWorldPos,
) {
    let dt = time.sim_dt();
    for (entity, mut bomb, item, custody, settled, struck_body) in &mut bombs {
        let at = where_it_is.of(custody, item);
        // A carried bomb still burns, so the fuse ticks before the custody
        // check.
        bomb.fuse_s -= dt;
        // Impact: geometry or a body stopped it, fast enough to count. The
        // speed is the settle's own (`ground_item_physics` publishes it on the
        // tick it zeroes velocity). One threshold for both surfaces.
        let hardest = settled
            .map(|settled| settled.impact_speed)
            .into_iter()
            .chain(struck_body.map(|hit| hit.impact_speed))
            .fold(0.0_f32, f32::max);
        let struck_hard = custody.in_world() && hardest >= bomb.impact_speed;
        if bomb.fuse_s > 0.0 && !struck_hard {
            continue;
        }
        info!(
            target: "ambition::moves",
            "bomb detonates: entity={entity:?} reason={} at {:?}",
            if struck_hard { "impact" } else { "fuse" },
            at,
        );
        effects.write(ambition_platformer2d::vfx::EffectRequest {
            // The bomb is the owner; see the note on `LiveBomb`.
            owner: entity,
            effect: ambition_platformer2d::vfx::Effect::DamageBox(
                ambition_platformer2d::vfx::DamageBoxEffect {
                    center: at,
                    // `Environment`, not the thrower's side and not `Neutral`:
                    // `melee_source` excludes Neutral from the body path, so a
                    // Neutral blast hurts nobody. The blast hurts everybody in
                    // it, the thrower included.
                    faction: ambition_platformer2d::vfx::HitSide::Environment,
                    half_extent: ae::Vec2::splat(bomb.blast_radius),
                    damage: bomb.damage,
                    // A feel multiplier, not a launch speed (see
                    // `spawn_damage_box`). Its own number, not derived from the
                    // radius: reach and launch are different facts.
                    knockback: BLAST_FEEL_SCALE,
                    lifetime_s: 0.08,
                    name: Some("bomb blast"),
                },
            ),
        });
        commands.entity(entity).despawn();
    }
}

/// The localizer's window on a bomb: the two fields that move.
///
/// The fuse only: damage, radius and threshold are constants copied from the
/// move and cannot diverge.
pub fn live_bomb_probe(bomb: &LiveBomb) -> u64 {
    bomb.fuse_s.to_bits() as u64
}

#[cfg(test)]
mod tests;
