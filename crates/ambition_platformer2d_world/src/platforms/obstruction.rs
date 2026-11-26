//! Crushing C3 (crushing-and-obstruction-response.md): a moving platform's
//! advance is a proposal. Before it commits, the bodies its leading face
//! would push are read: each can be pushed as far as its own clearance
//! against the room's solids, and no further without being overconstrained.
//! A yielding surface keeps the part of its move that overconstrains no
//! body; a limited one pushes up to its allowance past that; an unstoppable
//! one keeps its schedule (the body takes the response, stage C4). The
//! accepted move is the one the platform records (`last_delta`), so riders
//! are carried by what happened.
//!
//! Pairwise and box-shaped: a body pushed into another body is not chained,
//! the room's solids are its blocks (not its lines), and a plank or a body
//! read only through a portal is not an obstruction here.

use ambition_platformer2d_core::{self as ae, Aabb, Vec2};

/// A body a platform may push, as the advance reads it.
#[derive(Clone, Copy, Debug)]
pub struct Obstacle {
    pub aabb: Aabb,
}

/// The first time `t` (in units of `delta`, `t >= 0`) at which `moving`,
/// translated by `delta * t`, touches `target`; `Some(0.0)` when they
/// already overlap. `None` when it never does.
pub fn time_of_contact(moving: Aabb, delta: Vec2, target: Aabb) -> Option<f32> {
    let mut enter = 0.0_f32;
    let mut exit = f32::INFINITY;
    for axis in 0..2 {
        let (lo, hi) = (moving.min[axis], moving.max[axis]);
        let (tlo, thi) = (target.min[axis], target.max[axis]);
        let d = delta[axis];
        if d == 0.0 {
            // Touching faces do not push: the body must be strictly across.
            if hi <= tlo || lo >= thi { return None; }
            continue;
        }
        let (a, b) = ((tlo - hi) / d, (thi - lo) / d);
        let (near, far) = if a < b { (a, b) } else { (b, a) };
        enter = enter.max(near);
        exit = exit.min(far);
    }
    (enter <= exit && exit > 0.0).then_some(enter.max(0.0))
}

/// How deep `moving` already is in `target` along the direction of `delta`
/// (0 when they do not overlap).
fn penetration(moving: Aabb, delta: Vec2, target: Aabb) -> f32 {
    let overlap = (moving.max.min(target.max) - moving.min.max(target.min)).max(Vec2::ZERO);
    if overlap.x <= 0.0 || overlap.y <= 0.0 { return 0.0; }
    let dir = delta.normalize_or_zero();
    (overlap * dir.abs()).length()
}

/// The fraction of `delta` a platform whose box is `platform` may take
/// before it pushes any obstacle past `allowance_px` of overconstraint,
/// given the room's `solids` the obstacle is pushed against. 1.0 when the
/// whole move is admitted.
pub fn accepted_fraction(
    platform: Aabb,
    delta: Vec2,
    allowance_px: f32,
    obstacles: &[Obstacle],
    solids: &[Aabb],
) -> f32 {
    let length = delta.length();
    if !(length > 0.0) || !length.is_finite() { return 1.0; }
    let mut accepted = 1.0_f32;
    for obstacle in obstacles {
        let Some(reach) = time_of_contact(platform, delta, obstacle.aabb) else { continue };
        if reach >= 1.0 { continue; }
        // How far the body can go before something stops it.
        let clearance = solids.iter()
            .filter_map(|solid| time_of_contact(obstacle.aabb, delta, *solid))
            .fold(f32::INFINITY, f32::min);
        // The overconstraint it already made on earlier ticks spends the
        // allowance: a limit is a depth, not a depth per tick.
        let spent = penetration(platform, delta, obstacle.aabb);
        accepted = accepted.min(reach + clearance + (allowance_px - spent).max(0.0) / length);
    }
    accepted.clamp(0.0, 1.0)
}

/// A body a moving host carries through a portal on it (crushing C5): when
/// the host moves its mouth by `delta`, the part of the body through the pair
/// moves by the pair's map of `-delta` in the exit's chart, against the
/// solids there. Published by the portal layer before platforms advance.
#[derive(Clone, Debug)]
pub struct CarriedObstruction {
    /// The host platform's collision identity (its block's `GeoId`, which a
    /// portal's face host names).
    pub platform: ae::GeoId,
    pub body: bevy_ecs::entity::Entity,
    /// The pair's linear map from the mouth's chart to the exit's, by
    /// columns (the images of +x and +y).
    pub map: [Vec2; 2],
    /// The part of the body through the pair, in the exit's chart.
    pub mapped: Aabb,
    /// The exit chart's solids that part is pushed against.
    pub solids: Vec<Aabb>,
    /// The exit chart's one-way supports, which stop the part only when it
    /// is pushed along gravity onto them.
    pub one_ways: Vec<Aabb>,
    /// Gravity's direction in the exit chart.
    pub gravity_dir: Vec2,
}

/// The bodies moving hosts carry through their portals this tick.
#[derive(bevy_ecs::prelude::Resource, Clone, Debug, Default)]
pub struct CarriedObstructions(pub Vec<CarriedObstruction>);

impl CarriedObstructions {
    pub fn of(&self, platform: &ae::GeoId) -> Vec<&CarriedObstruction> {
        self.0.iter().filter(|carried| &carried.platform == platform).collect()
    }
}

/// The fraction of `delta` a host may take before a body it carries through
/// a portal is pushed past `allowance_px` against the exit's solids.
pub fn accepted_through(delta: Vec2, allowance_px: f32, carried: &[&CarriedObstruction]) -> f32 {
    let mut accepted = 1.0_f32;
    for through in carried {
        let push = -(through.map[0] * delta.x + through.map[1] * delta.y);
        let length = push.length();
        if !(length > 0.0) || !length.is_finite() { continue; }
        let onto_one_ways = push.normalize_or_zero().dot(through.gravity_dir) > 0.5;
        let one_ways = through.one_ways.iter().filter(|_| onto_one_ways);
        let clearance = through.solids.iter().chain(one_ways)
            .filter_map(|solid| time_of_contact(through.mapped, push, *solid))
            .fold(f32::INFINITY, f32::min);
        accepted = accepted.min(clearance + allowance_px / length);
    }
    accepted.clamp(0.0, 1.0)
}

/// The boxes a body pushed along `delta` is stopped by: the room's full
/// solids, and a one-way support when the push is along gravity onto its
/// top (a one-way stops nothing from below or the side).
pub fn pushing_solids(world: &ae::World, delta: Vec2, gravity_dir: Vec2, except: &ae::GeoId) -> Vec<Aabb> {
    let mut solids = full_solids(world, Some(except));
    if delta.normalize_or_zero().dot(gravity_dir) > 0.5 {
        solids.extend(one_way_supports(world, Some(except)));
    }
    solids
}

/// The room's full-collision blocks but `except`.
pub fn full_solids(world: &ae::World, except: Option<&ae::GeoId>) -> Vec<Aabb> {
    world.blocks.iter()
        .filter(|block| Some(&block.id) != except && ae::collision_semantics::is_full_collision_surface(block.kind))
        .map(|block| block.aabb)
        .collect()
}

/// The room's one-way supports but `except`.
pub fn one_way_supports(world: &ae::World, except: Option<&ae::GeoId>) -> Vec<Aabb> {
    world.blocks.iter()
        .filter(|block| Some(&block.id) != except && block.kind == ae::BlockKind::OneWay)
        .map(|block| block.aabb)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb(min: Vec2, size: Vec2) -> Aabb { Aabb::new(min + size * 0.5, size * 0.5) }

    #[test]
    fn contact_time_is_the_first_touch_along_the_move() {
        let moving = aabb(Vec2::new(0.0, 0.0), Vec2::splat(10.0));
        let target = aabb(Vec2::new(0.0, 14.0), Vec2::splat(10.0));
        assert_eq!(time_of_contact(moving, Vec2::new(0.0, 8.0), target), Some(0.5));
        assert_eq!(time_of_contact(moving, Vec2::new(0.0, -8.0), target), None, "moving away");
        assert_eq!(time_of_contact(moving, Vec2::new(0.0, 8.0), aabb(Vec2::new(10.0, 14.0), Vec2::splat(10.0))), None,
            "beside it, touching faces only");
    }

    /// A press descending 6 px onto a body 2 px under it, the body on a floor:
    /// a yielding press keeps 2 px of its move, a press allowed 3 px of
    /// overconstraint keeps 5, and with the floor gone the whole move.
    #[test]
    fn the_accepted_move_stops_where_the_body_has_no_room() {
        let press = aabb(Vec2::new(0.0, 0.0), Vec2::new(100.0, 20.0));
        let body = Obstacle { aabb: aabb(Vec2::new(40.0, 22.0), Vec2::new(20.0, 40.0)) };
        let floor = aabb(Vec2::new(0.0, 62.0), Vec2::new(100.0, 20.0));
        let down = Vec2::new(0.0, 6.0);
        assert!((accepted_fraction(press, down, 0.0, &[body], &[floor]) - 2.0 / 6.0).abs() < 1e-6);
        assert!((accepted_fraction(press, down, 3.0, &[body], &[floor]) - 5.0 / 6.0).abs() < 1e-6);
        assert_eq!(accepted_fraction(press, down, 0.0, &[body], &[]), 1.0);
        assert_eq!(accepted_fraction(press, -down, 0.0, &[body], &[floor]), 1.0, "rising away");
    }
}
