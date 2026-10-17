//! The primeval trail: the level of the dinosaurs.
//!
//! Six rooms in a row in the `primeval` biome
//! (`tools/ambition_ldtk_tools/specs/primeval/make_specs.py`): the T-rex's kin
//! are its enemies and the T-rex is at its end. Its rooms, its enemies and its
//! coconuts are authored in the map. What the map cannot author is here: the
//! sheet of the scenery the rooms name, and what an African swallow does with
//! a coconut.

use ambition_characters::control::ActorControl;
use ambition_combat::components::ActorTarget;
use ambition_combat::hand::RepertoireQuery;
use ambition_held_items::{release_carried_item, take_ground_item, GroundItem, ItemCustody, Release, SettledItem};
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_platformer2d_shared_tangle::schedule::{ActorDecisionSet, GameplayGated, PlayerInputSet, SimScheduleExt};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ae::AabbExt;
use bevy::prelude::*;
use ambition_content_schemas::rules::SwallowFetch;

/// The catalog row of the bird that carries coconuts, and what it carries.
pub const COCONUT_CARRIER: &str = "african_swallow";
pub const COCONUT: &str = "coconut";

/// What a swallow with a coconut does about its target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Laden {
    /// Fly at this velocity, to the place over the target.
    Carry(ae::Vec2),
    /// It is over the target: let the coconut go.
    LetGo,
}

/// What a swallow at `bird` with a coconut does about a target at `target`
/// that moves at `target_vel` (world units, +Y down), by the pack's
/// `african_swallow` tuning.
pub fn laden(fetch: &SwallowFetch, bird: ae::Vec2, target: ae::Vec2, target_vel: ae::Vec2) -> Laden {
    let over = ae::Vec2::new(target.x + target_vel.x * fetch.lead_seconds, target.y - fetch.carry_over);
    if (bird.x - over.x).abs() <= fetch.let_go_within && target.y - bird.y >= fetch.let_go_over {
        return Laden::LetGo;
    }
    Laden::Carry((over - bird).normalize_or_zero() * fetch.laden_speed)
}

/// An African swallow picks up a coconut and lets it go over what it hunts.
///
/// With an empty hand it flies to the nearest coconut at rest in its room and
/// takes it (the one take of a ground item). With a coconut and a target it
/// carries the coconut over the target and lets it go (the one release of a
/// carried item): the coconut is a thing in the world again, it falls, and the
/// thrown-item rule decides what it does to what it lands on. Then the coconut
/// is at rest, and the bird goes for it again.
///
/// It keeps no state: what a bird does is a function of what it holds, where
/// the coconuts are and where its target is. So a rewind has nothing of this
/// system's to restore. It runs where a custom brain decides (after the
/// brains publish their control frames, before the control gate), and what it
/// writes is the velocity of the bird's own control frame: a bird that is
/// held or captured is restricted as any body is.
///
/// A bird with no coconut to go for, and a bird with a coconut and no target,
/// does what its brain decided.
#[allow(clippy::type_complexity)]
pub fn swallows_carry_coconuts(
    mut commands: Commands,
    mut birds: Query<
        (
            &WornCharacter,
            &BodyHealth,
            &ae::BodyKinematics,
            Option<&ae::SweepSample>,
            &mut ActorControl,
            Option<&ActorTarget>,
            RepertoireQuery,
        ),
        Without<PlayerEntity>,
    >,
    order: Query<(Entity, &WornCharacter, Option<&SimId>), Without<PlayerEntity>>,
    mut items: Query<(Entity, &mut GroundItem, &mut ItemCustody)>,
    at_rest: Query<(), With<SettledItem>>,
    ids: Query<&SimId>,
    hunted: Query<&ae::BodyKinematics>,
    rooms: LiveRooms,
    tuning: Res<crate::tuning::AmbitionTuning>,
) {
    let fetch = &tuning.0.african_swallow;
    // In the order of the birds' simulation identities: two birds that go for
    // one coconut are decided the same way after a rewind (which makes new
    // entities).
    let mut carriers: Vec<(Entity, Option<&SimId>)> =
        order.iter().filter(|(_, worn, _)| worn.id() == COCONUT_CARRIER).map(|(bird, _, id)| (bird, id)).collect();
    carriers.sort_by(|a, b| a.1.map(SimId::as_str).cmp(&b.1.map(SimId::as_str)));
    for (bird, _) in carriers {
        let Ok((_, health, kin, last_step, mut control, target, mut repertoire)) = birds.get_mut(bird) else {
            continue;
        };
        let holds = repertoire.held.map(|held| held.id() == COCONUT);
        if !health.alive() {
            // A dead bird holds nothing: its coconut falls from where it was.
            if holds == Some(true) {
                release_carried_item(&mut commands, bird, &mut repertoire, &mut items, kin.pos, ae::Vec2::ZERO, Release::Drop);
            }
            continue;
        }
        match holds {
            Some(true) => {
                let Some(prey) = target.and_then(|target| target.entity).and_then(|prey| hunted.get(prey).ok()) else {
                    continue;
                };
                match laden(fetch, kin.pos, prey.pos, prey.vel) {
                    Laden::LetGo => {
                        // From under its feet, with a little of its own way.
                        let from = kin.pos + ae::Vec2::new(0.0, kin.size.y * 0.5 + 8.0);
                        let with = ae::Vec2::new(kin.vel.x * 0.25, 0.0);
                        release_carried_item(&mut commands, bird, &mut repertoire, &mut items, from, with, Release::Throw);
                    }
                    Laden::Carry(velocity) => control.0.velocity_target = ae::WorldVec2(velocity),
                }
                // A bird with a coconut in its feet does not dart.
                control.0.melee_pressed = false;
                control.0.melee_held = false;
                control.0.fire = None;
            }
            // It holds something that is not a coconut: not this system's.
            Some(false) => {}
            None => {
                let room = rooms.of(bird);
                let mut nearest: Option<(f32, &str, Entity)> = None;
                for (item, ground, custody) in items.iter() {
                    if ground.spec.id != COCONUT || !custody.in_world() || !at_rest.contains(item) || rooms.of(item) != room {
                        continue;
                    }
                    let far = ground.pos.distance(kin.pos);
                    let name = ids.get(item).map_or("", SimId::as_str);
                    if far <= fetch.fetch_reach && nearest.is_none_or(|(best, best_name, _)| (far, name) < (best, best_name)) {
                        nearest = Some((far, name, item));
                    }
                }
                let Some((_, _, item)) = nearest else {
                    continue;
                };
                let Ok((item, mut ground, mut custody)) = items.get_mut(item) else {
                    continue;
                };
                // The reach of the bird is its collision box, as for a press.
                if kin.collision_box(last_step).strict_intersects(ae::Aabb::new(ground.pos, ground.half_extent)) {
                    take_ground_item(&mut commands, bird, &mut repertoire, item, &mut ground, &mut custody);
                } else {
                    control.0.velocity_target = ae::WorldVec2((ground.pos - kin.pos).normalize_or_zero() * fetch.fetch_speed);
                    control.0.melee_pressed = false;
                    control.0.melee_held = false;
                }
            }
        }
    }
}

pub struct PrimevalPlugin;

impl Plugin for PrimevalPlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d::runtime::ScopedSystemsAppExt as _;
        let sim = SimScheduleExt::sim_schedule(app);
        app.add_scoped_systems(
            sim,
            crate::AMBITION_RULES,
            swallows_carry_coconuts.in_set(GameplayGated).after(ActorDecisionSet::Publish).before(PlayerInputSet::ControlGate),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fetch() -> SwallowFetch {
        crate::tuning::shipped().african_swallow
    }

    /// A swallow beside its target climbs to the place over it; over it, it
    /// lets go; and it does not let go from so low that the coconut arrives
    /// slower than it hurts at.
    #[test]
    fn a_laden_swallow_goes_over_its_target_and_lets_go_there() {
        let target = ae::Vec2::new(500.0, 400.0);
        let Laden::Carry(velocity) = laden(&fetch(), ae::Vec2::new(200.0, 380.0), target, ae::Vec2::ZERO) else {
            panic!("a swallow beside its target does not let go");
        };
        assert!(velocity.x > 0.0 && velocity.y < 0.0, "it flies toward the target and up, not {velocity:?}");
        assert!((velocity.length() - fetch().laden_speed).abs() < 1e-3);
        assert_eq!(laden(&fetch(), ae::Vec2::new(505.0, 400.0 - fetch().carry_over), target, ae::Vec2::ZERO), Laden::LetGo);
        assert!(matches!(laden(&fetch(), ae::Vec2::new(505.0, 400.0 - fetch().let_go_over + 20.0), target, ae::Vec2::ZERO), Laden::Carry(_)));
    }

    /// The coconut is let go where a walking target will be, not where it is.
    #[test]
    fn it_leads_a_target_that_moves() {
        let target = ae::Vec2::new(500.0, 400.0);
        let walking = ae::Vec2::new(300.0, 0.0);
        let over_now = ae::Vec2::new(500.0, 400.0 - fetch().carry_over);
        assert!(matches!(laden(&fetch(), over_now, target, walking), Laden::Carry(velocity) if velocity.x > 0.0));
        assert_eq!(laden(&fetch(), over_now + ae::Vec2::new(300.0 * fetch().lead_seconds, 0.0), target, walking), Laden::LetGo);
    }

    /// The fall from where it lets go is long enough to hurt: the thrown-item
    /// rule's bar for a coconut is a speed, and a fall of `let_go_over` at the
    /// gravity a loose item falls with is faster.
    #[test]
    fn a_coconut_let_go_arrives_faster_than_it_hurts_at() {
        let bar = crate::tuning::shipped().thrown_impacts.get(COCONUT).expect("a coconut hurts what it strikes").min_speed;
        let arrives = (2.0 * ambition_held_items::GROUND_ITEM_GRAVITY * fetch().let_go_over).sqrt();
        assert!(arrives > bar, "a coconut that falls {} arrives at {arrives}, under its bar of {bar}", fetch().let_go_over);
    }
}
