//! Generated sandbox room data.
//!
//! The engine models room geometry as named blocks. The Bevy sandbox decides
//! how to draw each block; the engine only cares about collision semantics.

use crate::actor::{Actor, BossBrain, EnemyBrain, KinematicPath};
use crate::combat::DamageVolume;
use crate::debug::{DebugLabel, DestinationLabel};
use crate::geometry::{aabb_from_min_size, Aabb, AabbExt};
use crate::interaction::{Breakable, Chest, Interactable, Pickup};
use crate::Vec2;

/// Upgrade tier required to blink through a blink wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlinkWallTier {
    /// Intended to be passable by an early blink-phasing upgrade.
    Soft,
    /// Intended to remain blocked until a stronger blink-phasing upgrade.
    Hard,
}

/// Collision/gameplay meaning of a generated world block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockKind {
    /// Full collision on both axes, and also a hard blocker for blink pathing.
    Solid,
    /// Full collision on both axes, but blink pathing may pass through it when
    /// the player has the matching blink-through upgrade. The destination still
    /// must be open space.
    BlinkWall { tier: BlinkWallTier },
    /// Landing platform: only solid when the player crosses from above.
    OneWay,
    /// Reset surface. Hitting this returns the player to spawn.
    Hazard,
    /// Pogo target that refreshes movement resources when struck downward.
    PogoOrb,
    /// Momentum-conversion surface. It applies a fixed impulse on touch.
    Rebound { impulse: Vec2 },
}

/// One piece of generated room geometry.
#[derive(Clone, Debug)]
pub struct Block {
    pub name: String,
    pub aabb: Aabb,
    pub kind: BlockKind,
}

impl Block {
    pub fn solid(name: impl Into<String>, min: Vec2, size: Vec2) -> Self {
        Self {
            name: name.into(),
            aabb: aabb_from_min_size(min, size),
            kind: BlockKind::Solid,
        }
    }

    pub fn blink_wall(name: impl Into<String>, min: Vec2, size: Vec2, tier: BlinkWallTier) -> Self {
        Self {
            name: name.into(),
            aabb: aabb_from_min_size(min, size),
            kind: BlockKind::BlinkWall { tier },
        }
    }

    pub fn one_way(name: impl Into<String>, min: Vec2, size: Vec2) -> Self {
        Self {
            name: name.into(),
            aabb: aabb_from_min_size(min, size),
            kind: BlockKind::OneWay,
        }
    }

    pub fn hazard(name: impl Into<String>, min: Vec2, size: Vec2) -> Self {
        Self {
            name: name.into(),
            aabb: aabb_from_min_size(min, size),
            kind: BlockKind::Hazard,
        }
    }

    pub fn pogo_orb(name: impl Into<String>, center: Vec2, radius: f32) -> Self {
        Self {
            name: name.into(),
            aabb: Aabb::new(center, Vec2::new(radius, radius)),
            kind: BlockKind::PogoOrb,
        }
    }

    pub fn rebound(name: impl Into<String>, min: Vec2, size: Vec2, impulse: Vec2) -> Self {
        Self {
            name: name.into(),
            aabb: aabb_from_min_size(min, size),
            kind: BlockKind::Rebound { impulse },
        }
    }
}

/// Data-first room object wrapper used by future sandbox/story content.
///
/// Blocks remain the collision tile/fixture language. Room objects are authored
/// entities layered on top of room geometry: hazards, interactables, pickups,
/// chests, breakables, enemy/boss spawns, kinematic paths, and debug labels.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomObject {
    pub id: String,
    pub name: String,
    pub aabb: Aabb,
    pub kind: RoomObjectKind,
}

impl RoomObject {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: Aabb,
        kind: RoomObjectKind,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            aabb,
            kind,
        }
    }
}

/// Reusable taxonomy for authored room entities.
#[derive(Clone, Debug, PartialEq)]
pub enum RoomObjectKind {
    DamageVolume(DamageVolume),
    Interactable(Interactable),
    Pickup(Pickup),
    Chest(Chest),
    Breakable(Breakable),
    Actor(Actor),
    EnemySpawn(EnemyBrain),
    BossSpawn(BossBrain),
    KinematicPath(KinematicPath),
    DebugLabel(DebugLabel),
    DestinationLabel(DestinationLabel),
}

/// Complete generated room spec.
#[derive(Clone, Debug)]
pub struct World {
    pub name: String,
    pub size: Vec2,
    pub spawn: Vec2,
    pub blocks: Vec<Block>,
    pub objects: Vec<RoomObject>,
}

/// First collision along a swept body path.
#[derive(Clone, Copy, Debug)]
pub struct SweepHit<'a> {
    pub block: &'a Block,
    /// Normalized time along the requested delta, in `[0, 1]`.
    pub time_of_impact: f32,
}

impl World {
    pub fn new(name: impl Into<String>, size: Vec2, spawn: Vec2, blocks: Vec<Block>) -> Self {
        Self {
            name: name.into(),
            size,
            spawn,
            blocks,
            objects: Vec::new(),
        }
    }

    pub fn with_objects(mut self, objects: Vec<RoomObject>) -> Self {
        self.objects = objects;
        self
    }

    /// True if `body` overlaps any block accepted by `predicate`.
    pub fn body_overlaps_any<F>(&self, body: Aabb, mut predicate: F) -> bool
    where
        F: FnMut(&Block) -> bool,
    {
        self.blocks
            .iter()
            .any(|block| predicate(block) && body.strict_intersects(block.aabb))
    }

    /// Return the earliest Parry-backed swept-AABB hit for `body` moving by `delta`.
    ///
    /// The predicate lets callers ask different gameplay questions from the same
    /// geometry routine: player movement solids, blink blockers, one-way landing
    /// tests, spawn blockers, and enemy collision can all share this path.
    pub fn first_body_sweep<F>(
        &self,
        body: Aabb,
        delta: Vec2,
        mut predicate: F,
    ) -> Option<SweepHit<'_>>
    where
        F: FnMut(&Block) -> bool,
    {
        let mut best: Option<SweepHit<'_>> = None;
        for block in &self.blocks {
            if !predicate(block) {
                continue;
            }
            let Some(time_of_impact) = body.sweep_time_of_impact(delta, block.aabb) else {
                continue;
            };
            if best.map_or(true, |hit| time_of_impact < hit.time_of_impact) {
                best = Some(SweepHit {
                    block,
                    time_of_impact,
                });
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_new_starts_without_authored_objects() {
        let world = World::new(
            "test",
            Vec2::new(100.0, 80.0),
            Vec2::new(20.0, 20.0),
            Vec::new(),
        );
        assert!(world.objects.is_empty());
    }
}
