//! Authored pickup motion plans stepped by the engine.
//!
//! [`ItemMotionPlan`] covers emergence, travel, gravity, bounce, and wall turns
//! without giving collectibles actor brains or body clusters. Resolution uses the
//! shared world/AABB geometry but only the axis-separated contact behavior items
//! need. Motion writes the authoritative [`WorldItem::pos`].

use bevy::prelude::*;

use super::world_item::WorldItem;
use ambition_platformer2d_core::{self as ae, AabbExt};

/// What a pickup does once it is in the world. Pure numbers — a game states
/// one of these and never writes a stepper.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemMotionPlan {
    /// Rise out of whatever produced it before anything else happens: how far,
    /// and over how long. `None` means it is simply there.
    ///
    /// The rise ignores geometry ON PURPOSE — a pickup emerging from a block
    /// starts INSIDE that block, so a rise that collided would be stuck against
    /// the thing it is climbing out of.
    pub emerge: Option<ItemEmerge>,
    /// Travel speed along the surface, px/s. `0.0` is a pickup that stays put
    /// once it has emerged.
    pub speed: f32,
    /// Which way it sets off: `+1` right, `-1` left.
    pub facing: f32,
    /// Downward pull once travelling, px/s². `0.0` floats.
    pub gravity: f32,
    pub bounce: f32,
    /// Turn around when something stops it. The same rule bodies follow, stated
    /// here rather than shared, because a pickup has no `ActorTuning` to hold it.
    pub turns_at_walls: bool,
}

/// The rise out of a block: how far against gravity, and over how long.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemEmerge {
    pub distance: f32,
    pub seconds: f32,
}

impl ItemMotionPlan {
    /// Sits where it was put. The behaviour of every pickup that has no plan, so
    /// this exists for a game that wants to say so explicitly.
    pub fn still() -> Self {
        Self {
            emerge: None,
            speed: 0.0,
            facing: 1.0,
            gravity: 0.0,
            bounce: 0.0,
            turns_at_walls: false,
        }
    }

    /// A walker: falls, follows the ground, turns at walls. The mushroom.
    pub fn walker(speed: f32) -> Self {
        Self {
            emerge: None,
            speed,
            facing: 1.0,
            gravity: DEFAULT_ITEM_GRAVITY,
            bounce: 0.0,
            turns_at_walls: true,
        }
    }

    /// A bouncer: a walker that keeps its bounce instead of settling. The
    /// star. `restitution` is the fraction of impact speed it gives back.
    pub fn bouncer(speed: f32, restitution: f32) -> Self {
        Self {
            bounce: restitution,
            ..Self::walker(speed)
        }
    }

    /// Rise out of the thing that produced it first.
    pub fn emerging(mut self, distance: f32, seconds: f32) -> Self {
        self.emerge = Some(ItemEmerge { distance, seconds });
        self
    }

    /// Set off the other way.
    pub fn facing(mut self, facing: f32) -> Self {
        self.facing = if facing < 0.0 { -1.0 } else { 1.0 };
        self
    }
}

/// The pull a pickup falls under unless a game says otherwise. Deliberately
/// close to a body's, so a mushroom and the player read as being in the same
/// world.
pub const DEFAULT_ITEM_GRAVITY: f32 = 900.0;

/// A pickup's motion: the authored plan plus where it has got to.
///
/// One component rather than plan-and-cursor as two, because a cursor without
/// its plan is meaningless and a plan on a pickup that is not being stepped is a
/// lie about what will happen.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ItemMotion {
    pub plan: ItemMotionPlan,
    /// Seconds spent rising so far; past `emerge.seconds` the rise is done.
    emerged_for: f32,
    /// Current travel velocity. Zero until the rise finishes.
    vel: ae::Vec2,
    /// Which way it is going now — starts at the plan's facing and flips at
    /// walls, so the plan stays the authored intent and this is the live fact.
    facing: f32,
}

impl ItemMotion {
    pub fn new(plan: ItemMotionPlan) -> Self {
        Self {
            plan,
            emerged_for: 0.0,
            vel: ae::Vec2::ZERO,
            facing: plan.facing,
        }
    }

    /// Still climbing out of whatever produced it.
    pub fn emerging(&self) -> bool {
        self.plan
            .emerge
            .is_some_and(|rise| self.emerged_for < rise.seconds)
    }

    /// Seconds spent rising so far — rollback state, hence readable.
    pub fn emerged_for(&self) -> f32 {
        self.emerged_for
    }

    pub fn facing(&self) -> f32 {
        self.facing
    }

    pub fn velocity(&self) -> ae::Vec2 {
        self.vel
    }
}

/// Which face of the pickup's box the world stopped, resolving one axis.
struct AxisResolve {
    /// Where it ended up after being pushed out.
    pos: ae::Vec2,
    /// Something stopped it along the axis it moved.
    blocked: bool,
}

/// Move `pos` by `delta` along ONE axis and push back out of any solid it
/// entered. Axis-separated so a pickup sliding along a floor is not stopped by
/// the floor's vertical face, which is the classic corner-snag.
fn move_axis(world: &ae::World, pos: ae::Vec2, half: ae::Vec2, delta: ae::Vec2) -> AxisResolve {
    let mut moved = pos + delta;
    let mut blocked = false;
    // Deterministic: `world.blocks` is an ordered slice and every push is
    // idempotent, so the result does not depend on which block is found first.
    for block in &world.blocks {
        if !matches!(block.kind, ae::BlockKind::Solid) {
            continue;
        }
        let aabb = ae::Aabb::new(moved, half);
        if !block.aabb.strict_intersects(aabb) {
            continue;
        }
        blocked = true;
        let b = block.aabb;
        if delta.x > 0.0 {
            moved.x = b.center().x - b.half_size().x - half.x;
        } else if delta.x < 0.0 {
            moved.x = b.center().x + b.half_size().x + half.x;
        } else if delta.y > 0.0 {
            moved.y = b.center().y - b.half_size().y - half.y;
        } else if delta.y < 0.0 {
            moved.y = b.center().y + b.half_size().y + half.y;
        }
    }
    AxisResolve {
        pos: moved,
        blocked,
    }
}

/// Step every pickup that has a plan.
///
/// Runs before the collect pass, so a pickup is collected where it IS this tick
/// rather than where it was last one — a fast star would otherwise be
/// collectable from a box it has already left.
pub fn step_item_motion(
    time: Res<ambition_time::WorldTime>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    mut items: Query<(&mut WorldItem, &mut ItemMotion)>,
) {
    let dt = time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    for (mut item, mut motion) in &mut items {
        step_one_item(&world.0, &mut item, &mut motion, dt);
    }
}

/// One pickup, one tick. Split out so the system and its tests drive the same
/// code — a test that re-implemented the resolve would agree with itself and
/// with nothing else.
fn step_one_item(world: &ae::World, item: &mut WorldItem, motion: &mut ItemMotion, dt: f32) {
    {
        // ── The rise ──────────────────────────────────────────────────────
        // Screen up is -y. No collision: it is inside the block it is leaving.
        if let Some(rise) = motion.plan.emerge {
            if motion.emerged_for < rise.seconds {
                // CLAMP THE TIME CONSUMED, not the per-tick fraction.
                //
                // `(dt / seconds).min(1.0)` bounds a single step at the whole rise, which only
                // matters for a rise shorter than one tick — and says nothing about the LAST
                // tick of a normal one. The item then began its travel from somewhere nobody
                // wrote down.
                //
                // Taking `min(remaining)` makes the fractions sum to exactly
                // one, so total displacement is exactly `rise.distance` for any
                // duration and any timestep.
                let remaining = (rise.seconds - motion.emerged_for).max(0.0);
                let used = dt.min(remaining);
                let fraction = used / rise.seconds.max(1e-4);
                item.pos.y -= rise.distance * fraction;
                motion.emerged_for += used;
                return;
            }
        }

        // ── The travel ────────────────────────────────────────────────────
        let plan = motion.plan;
        motion.vel.y += plan.gravity * dt;
        motion.vel.x = motion.facing * plan.speed;

        let half = item.half_extent;
        let horizontal = move_axis(world, item.pos, half, ae::Vec2::new(motion.vel.x * dt, 0.0));
        item.pos = horizontal.pos;
        if horizontal.blocked && plan.turns_at_walls {
            motion.facing = -motion.facing;
        }

        let falling = motion.vel.y > 0.0;
        let vertical = move_axis(world, item.pos, half, ae::Vec2::new(0.0, motion.vel.y * dt));
        item.pos = vertical.pos;
        if vertical.blocked {
            // Landing is the only place `bounce` is read: a pickup that gives
            // nothing back settles and walks, one that gives most of it back
            // keeps hopping down the level.
            motion.vel.y = if falling && plan.bounce > 0.0 {
                -motion.vel.y * plan.bounce
            } else {
                0.0
            };
        }
    }
}

#[cfg(test)]
mod tests;
