//! A thrown item hurts the body it strikes.
//!
//! The engine's item step says which bodies an item reached and how fast
//! (`ItemStruckBody`); what that does is the game's rule. Ambition's rule is
//! `thrown_impacts` in its tuning (`data/ambition_rules.ron`): an item with a
//! row hurts what it strikes at or above the row's speed, and bounces off it.
//! An item with no row lands and hurts nobody (the javelin and the axe,
//! today).
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

use ambition_content_schemas::rules::ThrownImpact;

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

/// Each loose item that reached a body fast enough hurts it and bounces off.
///
/// The victims are the bodies the item step's swept path entered. They are not
/// found again here from where the item is now: a fast item is past the body
/// it crossed by the end of the tick. The blow is a hazard's: it names its
/// victim, it has no attacker, and it asks no question of sides (a coconut
/// hurts whoever it lands on, the one who threw it too).
///
/// ⚠ BODIES ONLY. A boss is not a body to the hit road (`HitTarget::Feature`),
/// so a coconut does not hurt one yet.
pub fn thrown_items_strike(
    tuning: Res<crate::tuning::AmbitionTuning>,
    mut hits: MessageWriter<HitEvent>,
    mut items: Query<(Entity, &mut GroundItem, &ItemCustody, &ItemStruckBody)>,
    bodies: Query<&BodyHealth>,
    rooms: LiveRooms,
) {
    for (entity, mut item, custody, struck) in &mut items {
        if !custody.in_world() {
            continue;
        }
        let Some(impact) = tuning.0.thrown_impacts.get(&item.spec.id).copied() else {
            continue;
        };
        if struck.impact_speed < impact.min_speed {
            continue;
        }
        let room = rooms.of(entity);
        let mut struck_any = false;
        for &body in &struck.bodies {
            if !bodies.get(body).is_ok_and(|health| health.alive()) {
                continue;
            }
            struck_any = true;
            hits.write(HitEvent {
                volume: ae::Aabb::new(item.pos, item.half_extent).into(),
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
        if struck_any {
            item.vel = rebound(item.vel, impact);
        }
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
        for (id, impact) in &crate::tuning::shipped().thrown_impacts {
            for speed in [impact.min_speed, 410.0, 900.0, 5000.0] {
                let left = rebound(ae::Vec2::new(0.6 * speed, 0.8 * speed), *impact);
                assert!(left.length() < impact.min_speed, "`{id}` arrives at {speed} and leaves at {}: it strikes twice", left.length());
                assert!(left.x < 0.0 && left.y < 0.0, "`{id}` goes back the way it came, not {left:?}");
            }
        }
    }

    /// The blow lands on the body the item's path entered, not on a body the
    /// item stands on at the end of the tick: a fast item is past the body it
    /// crossed, and may stand on another one.
    #[test]
    fn the_blow_lands_on_the_bodies_the_item_struck_and_no_other() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(crate::tuning::AmbitionTuning(crate::tuning::shipped()));
        world.init_resource::<Messages<HitEvent>>();
        let health = || BodyHealth::new(ambition_characters::actor::Health::new(10));
        let at = ae::Vec2::new(100.0, 100.0);
        let crossed = world.spawn((ae::BodyKinematics::default(), health())).id();
        let mut beside = ae::BodyKinematics::default();
        beside.pos = at;
        beside.size = ae::Vec2::new(24.0, 48.0);
        let beside = world.spawn((beside, health())).id();
        let spec = ambition_characters::brain::held_item_by_id("coconut").expect("the coconut is a held item");
        let mut item = GroundItem::at_rest(spec, at, ae::Vec2::new(9.0, 7.5));
        item.vel = ae::Vec2::new(600.0, 0.0);
        world.spawn((item, ItemCustody::InWorld, ItemStruckBody { impact_speed: 600.0, bodies: vec![crossed] }));
        world.run_system_once(thrown_items_strike).expect("the rule runs");
        let mut cursor = world.resource::<Messages<HitEvent>>().get_cursor();
        let victims: Vec<_> = cursor
            .read(world.resource::<Messages<HitEvent>>())
            .map(|hit| hit.target)
            .collect();
        assert_eq!(victims, vec![HitTarget::Body(crossed)], "the struck body, and not {beside:?} under the item");
    }

    /// The impacts are the pack's rows: the coconut hurts and the javelin
    /// has no row, an edit is the lowered row, and a row that cannot hurt is
    /// refused at admission with its field named.
    #[test]
    fn the_thrown_impacts_are_the_packs_rows() {
        use ambition_content_schemas::rules::lowered_ambition_rules;
        let shipped = crate::tuning::shipped();
        assert_eq!(shipped.thrown_impacts.get("coconut").map(|impact| impact.damage), Some(2));
        assert_eq!(shipped.thrown_impacts.get("javelin"), None);

        let row = r#""coconut": (min_speed: 240.0, damage: 2),"#;
        let with = |replacement: &'static str| {
            crate::pack::compile_pack_with(move |path, text| {
                if path != "data/ambition_rules.ron" {
                    return text;
                }
                assert_eq!(text.matches(row).count(), 1, "the probe edits one row");
                text.replace(row, replacement)
            })
        };
        let edited = with(r#""coconut": (min_speed: 240.0, damage: 3),"#).expect("an edited row compiles");
        let rules = lowered_ambition_rules(&edited).expect("the edited pack lowers its tuning");
        assert_eq!(rules.thrown_impacts.get("coconut").map(|impact| impact.damage), Some(3));

        let refused = format!("{:?}", with(r#""coconut": (min_speed: 240.0, damage: 0),"#).expect_err("a row that cannot hurt"));
        assert!(refused.contains("`thrown_impacts.coconut.damage`"), "{refused}");
    }
}
