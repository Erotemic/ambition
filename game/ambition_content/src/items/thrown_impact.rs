//! A thrown item hurts the body it strikes.
//!
//! The engine's item step says only that an item reached a body and how fast
//! (`ItemStruckBody`); what that does is the game's rule. Ambition's rule is
//! this table: an item with a row here hurts what it strikes at or above the
//! row's speed, and bounces off it. An item with no row lands and hurts
//! nobody (the javelin and the axe, today).
//!
//! The rule keeps no state. A struck body is not a wall (the item step does
//! not stop the item on it), so the item's own velocity is what says "this
//! blow is spent": the rebound leaves it under the bar, and it falls from
//! there. A coconut that drops off a palm at speed is the same blow as one
//! that was thrown.

use ambition_characters::actor::BodyHealth;
use ambition_combat::events::{HitMode, HitSource, HitTarget};
use ambition_combat::HitEvent;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::item::{GroundItem, ItemCustody, ItemStruckBody};
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use bevy::prelude::*;

/// What one kind of thrown item does to a body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThrownImpact {
    /// At or above this speed the item hurts what it strikes. It is above the
    /// speed the rebound leaves, so one arrival is one blow.
    pub min_speed: f32,
    pub damage: i32,
    /// How far past the item's own box the blow reaches.
    pub reach: f32,
}

/// The fraction of its velocity an item keeps, reversed, when it strikes, and
/// the most of its own bar that the rebound may be: however fast it arrived,
/// it leaves under the bar.
const REBOUND: f32 = 0.3;
const REBOUND_UNDER_THE_BAR: f32 = 0.8;

/// The velocity an item leaves a body with: back the way it came, slower than
/// its bar.
fn rebound(vel: ae::Vec2, impact: ThrownImpact) -> ae::Vec2 {
    let speed = (vel.length() * REBOUND).min(impact.min_speed * REBOUND_UNDER_THE_BAR);
    -vel.normalize_or_zero() * speed
}

/// Ambition's thrown items that hurt, by held-item id. A throw leaves the hand
/// at about 410 (`THROW_SPEED_X`, `THROW_SPEED_UP`); a fall of a body's height
/// arrives at about 370.
const IMPACTS: &[(&str, ThrownImpact)] =
    &[("coconut", ThrownImpact { min_speed: 240.0, damage: 2, reach: 6.0 })];

/// The impact of the item `id`, if it has one.
pub fn impact_of(id: &str) -> Option<ThrownImpact> {
    IMPACTS.iter().find(|(item, _)| *item == id).map(|(_, impact)| *impact)
}

/// Each loose item that reached a body fast enough hurts it and bounces off.
///
/// The item step says THAT a body was reached and how fast; it does not say
/// which. The body is the one the item's box (grown by the blow's reach) is on,
/// in the item's live room. The blow is a hazard's: it names its victim, it
/// has no attacker, and it asks no question of sides (a coconut hurts whoever
/// it lands on, the one who threw it too).
///
/// ⚠ BODIES ONLY. A boss is not a body to the hit road (`HitTarget::Feature`),
/// so a coconut does not hurt one yet.
pub fn thrown_items_strike(
    mut hits: MessageWriter<HitEvent>,
    mut items: Query<(Entity, &mut GroundItem, &ItemCustody, &ItemStruckBody)>,
    bodies: Query<(Entity, &ae::BodyKinematics, &BodyHealth)>,
    rooms: LiveRooms,
) {
    for (entity, mut item, custody, struck) in &mut items {
        if !custody.in_world() {
            continue;
        }
        let Some(impact) = impact_of(&item.spec.id) else {
            continue;
        };
        if struck.impact_speed < impact.min_speed {
            continue;
        }
        let room = rooms.of(entity);
        let reach = item.half_extent + ae::Vec2::splat(impact.reach);
        for (body, kin, health) in &bodies {
            if !health.alive() || rooms.of(body) != room {
                continue;
            }
            let apart = (kin.pos - item.pos).abs();
            if apart.x > reach.x + kin.size.x * 0.5 || apart.y > reach.y + kin.size.y * 0.5 {
                continue;
            }
            hits.write(HitEvent {
                volume: ae::Aabb::new(item.pos, reach).into(),
                damage: impact.damage,
                source: HitSource::Hazard,
                attacker: None,
                room,
                target: HitTarget::Body(body),
                mode: HitMode::Knockback,
                // The victim's own consumer decides its launch (as for a
                // stage hazard on a body that is not a player's).
                knockback: None,
                ignored_targets: Vec::new(),
                strike_sfx: None,
                attacker_move_instance: None,
            });
        }
        item.vel = rebound(item.vel, impact);
    }
}

pub(crate) fn register(app: &mut App) {
    let sim = ambition_platformer2d::platformer::schedule::SimScheduleExt::sim_schedule(app);
    app.add_systems(
        sim,
        thrown_items_strike
            .after(ambition_platformer2d::platformer::schedule::ItemPickupSet::CoreHeldItems)
            .in_set(ambition_platformer2d::platformer::schedule::CombatSet::Settle),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rebound_of_every_item_is_under_its_own_bar() {
        // The blow is spent when the rebound is slower than the bar: otherwise
        // the item strikes the same body again on the next tick. A throw is
        // about 410; a long fall is as fast as you like.
        for (id, impact) in IMPACTS {
            for speed in [impact.min_speed, 410.0, 900.0, 5000.0] {
                let left = rebound(ae::Vec2::new(0.6 * speed, 0.8 * speed), *impact);
                assert!(left.length() < impact.min_speed, "`{id}` arrives at {speed} and leaves at {}: it strikes twice", left.length());
                assert!(left.x < 0.0 && left.y < 0.0, "`{id}` goes back the way it came, not {left:?}");
            }
        }
    }

    #[test]
    fn a_coconut_hurts_and_a_javelin_has_no_row() {
        assert_eq!(impact_of("coconut").map(|i| i.damage), Some(2));
        assert_eq!(impact_of("javelin"), None);
    }
}
