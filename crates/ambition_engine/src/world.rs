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

/// Authored water volume tuning. The simulation reads this when the
/// player is inside the AABB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterVolumeSpec {
    /// Multiplier on gravity while submerged (1.0 = normal, 0.25 ≈
    /// "floaty"). Default 0.30.
    pub gravity_scale: f32,
    /// Linear drag coefficient applied to vel each tick while
    /// submerged. 0.0 = no drag, 1.0 = instant stop. Default 0.08.
    pub drag: f32,
    /// Cap on vertical fall speed inside the water. Default 220.
    pub max_fall_speed: f32,
    /// Per-press upward impulse applied when jump is pressed while
    /// submerged AND the player has the `swim` ability. Mario-style:
    /// each press is one stroke; repeated presses rise. Default 240.
    pub swim_up_impulse: f32,
}

impl Default for WaterVolumeSpec {
    fn default() -> Self {
        Self {
            gravity_scale: 0.30,
            drag: 0.08,
            max_fall_speed: 220.0,
            swim_up_impulse: 240.0,
        }
    }
}

/// Visual / gameplay flavor of a water region. Backend stays
/// source-agnostic: the runtime only cares about the kind for things
/// like obscuring vision (Murky) or unique tuning. Authoring layer
/// chooses entities or IntGrid per-room based on shape needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterKind {
    /// Mostly transparent. Player and submerged geometry stay visible.
    Clear,
    /// Opaque-ish; hides what's under the surface.
    Murky,
}

/// One axis-aligned water region on the world grid. Multiple regions
/// may exist in the same room; queries return the first that contains
/// the player AABB.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterRegion {
    pub aabb: Aabb,
    pub kind: WaterKind,
    pub spec: WaterVolumeSpec,
}

impl WaterRegion {
    pub fn new(aabb: Aabb, kind: WaterKind, spec: WaterVolumeSpec) -> Self {
        Self { aabb, kind, spec }
    }
}

/// Snapshot of "the player's relationship to water" for one frame.
/// Movement queries this rather than touching the underlying region
/// list, so future water sources (entity, IntGrid, generated) all
/// look identical to the simulator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterContact {
    pub kind: WaterKind,
    pub region_aabb: Aabb,
    /// Top edge of the water region in world coordinates. Lower
    /// y-values mean higher in screen space.
    pub surface_y: f32,
    /// 0.0 ≈ player AABB just dipping into the surface;
    /// 1.0 = player fully submerged (top of body below surface).
    pub submersion: f32,
    pub spec: WaterVolumeSpec,
}

/// Visual / gameplay flavor of a climbable surface. Backend stays
/// source-agnostic: movement only reads `kind` for behavior tweaks
/// (vine sway, ladder-rung snap). Authoring layer chooses entities or
/// IntGrid per-room based on shape needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClimbableKind {
    /// Rigid ladder; vertical + minor horizontal movement allowed.
    Ladder,
    /// Climbable wall surface (rock face, ivy). Same mechanics, the
    /// kind exists so sprites / sfx can branch.
    Wall,
    /// Hanging vine. Allows pendulum-style sway in a future patch;
    /// for now mechanically identical to Ladder.
    Vine,
}

/// Authored tuning for a climbable region. Mirrors `WaterVolumeSpec`
/// so authoring layers can opt into per-region tuning when the
/// default needs an override.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClimbableSpec {
    /// Vertical climb speed (px/sec). Default 180 — slower than walk
    /// (≈ 360) so climbing reads as a deliberate movement choice.
    pub climb_speed: f32,
    /// Horizontal-movement scale while climbing. 1.0 = full air speed,
    /// 0.0 = totally locked. Default 0.25 — ladders allow tiny strafe
    /// to align with rungs but don't let the player fly horizontally.
    pub strafe_factor: f32,
}

impl Default for ClimbableSpec {
    fn default() -> Self {
        Self {
            climb_speed: 180.0,
            strafe_factor: 0.25,
        }
    }
}

/// One axis-aligned climbable region. Multiple regions may exist in
/// the same room; queries return the first that contains the player
/// AABB.
#[derive(Clone, Debug, PartialEq)]
pub struct ClimbableRegion {
    pub aabb: Aabb,
    pub kind: ClimbableKind,
    pub spec: ClimbableSpec,
}

impl ClimbableRegion {
    pub fn new(aabb: Aabb, kind: ClimbableKind, spec: ClimbableSpec) -> Self {
        Self { aabb, kind, spec }
    }

    /// Convenience: ladder with default spec.
    pub fn ladder(aabb: Aabb) -> Self {
        Self::new(aabb, ClimbableKind::Ladder, ClimbableSpec::default())
    }
}

/// Snapshot of "the player's relationship to a climbable surface" for
/// one frame. Mirrors `WaterContact`'s shape so the simulator's
/// climbable handling can stay symmetric.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClimbableContact {
    pub kind: ClimbableKind,
    pub region_aabb: Aabb,
    /// Top edge of the climbable region. Used by movement to detect
    /// "stepping off the top of a ladder onto solid ground".
    pub top_y: f32,
    /// Bottom edge — used to detect "dropping off the bottom".
    pub bottom_y: f32,
    /// X-center of the region. Movement may snap the player to this
    /// for ladder-rung-style alignment.
    pub center_x: f32,
    pub spec: ClimbableSpec,
}

/// Complete generated room spec.
#[derive(Clone, Debug)]
pub struct World {
    pub name: String,
    pub size: Vec2,
    pub spawn: Vec2,
    pub blocks: Vec<Block>,
    pub objects: Vec<RoomObject>,
    /// Source-agnostic water regions. Authoring may come from LDtk
    /// entities, an LDtk IntGrid water layer, or generated content.
    /// Movement only reads this list (via `water_at`), never the
    /// upstream sources.
    pub water_regions: Vec<WaterRegion>,
    /// Source-agnostic climbable regions (ladders, vines, climbable
    /// walls). Same authoring contract as `water_regions`: the
    /// simulator only queries `climbable_at`, never iterates this list
    /// directly.
    pub climbable_regions: Vec<ClimbableRegion>,
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
            water_regions: Vec::new(),
            climbable_regions: Vec::new(),
        }
    }

    pub fn with_objects(mut self, objects: Vec<RoomObject>) -> Self {
        self.objects = objects;
        self
    }

    pub fn with_water_regions(mut self, regions: Vec<WaterRegion>) -> Self {
        self.water_regions = regions;
        self
    }

    /// Builder-style setter for climbable regions. Mirrors
    /// `with_water_regions`; preferred over reaching into
    /// `world.climbable_regions` directly so future authoring sources
    /// (LDtk IntGrid, LDtk entity, generated) flow through one entry
    /// point.
    pub fn with_climbable_regions(mut self, regions: Vec<ClimbableRegion>) -> Self {
        self.climbable_regions = regions;
        self
    }

    /// Return the first water region intersecting `body`, with
    /// derived submersion + surface metrics. `None` when out of
    /// water. Source-agnostic: callers must not iterate
    /// `water_regions` directly.
    pub fn water_at(&self, body: Aabb) -> Option<WaterContact> {
        let region = self
            .water_regions
            .iter()
            .find(|r| r.aabb.strict_intersects(body))?;
        let surface_y = region.aabb.top();
        let body_h = body.height().max(1.0);
        // y grows downward in this engine: a body whose top equals
        // the surface is barely dipping in (submersion ≈ 0); a body
        // whose top is below the surface by its full height is fully
        // submerged (submersion = 1).
        let depth_into_water = (body.top() - surface_y).max(0.0);
        let submersion = (depth_into_water / body_h).clamp(0.0, 1.0);
        Some(WaterContact {
            kind: region.kind,
            region_aabb: region.aabb,
            surface_y,
            submersion,
            spec: region.spec,
        })
    }

    /// Return the first climbable region intersecting `body`, with
    /// derived top/bottom/center metrics. `None` when the player is
    /// not touching any climbable. Source-agnostic: callers must not
    /// iterate `climbable_regions` directly so future authoring
    /// sources (LDtk IntGrid, LDtk entity, generated) all look
    /// identical to the simulator. Mirrors `water_at`.
    pub fn climbable_at(&self, body: Aabb) -> Option<ClimbableContact> {
        let region = self
            .climbable_regions
            .iter()
            .find(|r| r.aabb.strict_intersects(body))?;
        Some(ClimbableContact {
            kind: region.kind,
            region_aabb: region.aabb,
            top_y: region.aabb.top(),
            bottom_y: region.aabb.bottom(),
            center_x: 0.5 * (region.aabb.min.x + region.aabb.max.x),
            spec: region.spec,
        })
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
            if best.is_none_or(|hit| time_of_impact < hit.time_of_impact) {
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

    #[test]
    fn body_overlaps_any_uses_predicate() {
        let world = World::new(
            "test",
            Vec2::new(200.0, 200.0),
            Vec2::new(50.0, 50.0),
            vec![
                Block::solid("wall", Vec2::new(50.0, 50.0), Vec2::new(20.0, 20.0)),
                Block::hazard("spike", Vec2::new(100.0, 50.0), Vec2::new(20.0, 20.0)),
            ],
        );
        let body = Aabb::new(Vec2::new(60.0, 60.0), Vec2::new(5.0, 5.0));
        // Predicate matches the wall — overlap detected.
        assert!(world.body_overlaps_any(body, |b| matches!(b.kind, BlockKind::Solid)));
        // Predicate matches only hazards — no overlap because the body
        // is over the wall, not the hazard.
        assert!(!world.body_overlaps_any(body, |b| matches!(b.kind, BlockKind::Hazard)));
    }

    #[test]
    fn first_body_sweep_picks_earliest_hit() {
        let world = World::new(
            "test",
            Vec2::new(500.0, 500.0),
            Vec2::new(10.0, 10.0),
            vec![
                Block::solid("near", Vec2::new(50.0, 0.0), Vec2::new(10.0, 100.0)),
                Block::solid("far", Vec2::new(200.0, 0.0), Vec2::new(10.0, 100.0)),
            ],
        );
        let body = Aabb::new(Vec2::new(20.0, 50.0), Vec2::new(5.0, 5.0));
        let hit = world.first_body_sweep(body, Vec2::new(300.0, 0.0), |_| true);
        let hit = hit.expect("sweep should hit something with two walls in path");
        assert_eq!(hit.block.name, "near");
        assert!(hit.time_of_impact >= 0.0 && hit.time_of_impact <= 1.0);
    }

    #[test]
    fn first_body_sweep_returns_none_when_predicate_filters_all() {
        let world = World::new(
            "test",
            Vec2::new(500.0, 500.0),
            Vec2::new(10.0, 10.0),
            vec![Block::solid(
                "wall",
                Vec2::new(50.0, 0.0),
                Vec2::new(10.0, 100.0),
            )],
        );
        let body = Aabb::new(Vec2::new(20.0, 50.0), Vec2::new(5.0, 5.0));
        // Predicate rejects every block — sweep finds nothing.
        let hit = world.first_body_sweep(body, Vec2::new(300.0, 0.0), |_| false);
        assert!(hit.is_none());
    }

    #[test]
    fn water_at_returns_none_outside_any_region() {
        let world = World::new(
            "test",
            Vec2::new(500.0, 500.0),
            Vec2::new(10.0, 10.0),
            Vec::new(),
        );
        let body = Aabb::new(Vec2::new(50.0, 50.0), Vec2::new(5.0, 5.0));
        assert!(world.water_at(body).is_none());
    }

    #[test]
    fn water_at_reports_full_submersion_for_a_body_below_the_surface() {
        // Aabb::new is (center, half_size). Water region: center
        // (200, 200), half (100, 100) → min=(100,100), max=(300,300).
        // top()=100. Body: center (200, 200), half (10, 10) →
        // top=190. depth = 190 - 100 = 90. Body height = 20.
        // submersion = 90 / 20 = 4.5, clamps to 1.0.
        let mut world = World::new(
            "test",
            Vec2::new(500.0, 500.0),
            Vec2::new(10.0, 10.0),
            Vec::new(),
        );
        world.water_regions.push(WaterRegion::new(
            Aabb::new(Vec2::new(200.0, 200.0), Vec2::new(100.0, 100.0)),
            WaterKind::Clear,
            WaterVolumeSpec::default(),
        ));
        let body = Aabb::new(Vec2::new(200.0, 200.0), Vec2::new(10.0, 10.0));
        let contact = world.water_at(body).expect("body sits inside water");
        assert!(
            (contact.submersion - 1.0).abs() < 1e-3,
            "expected full submersion clamp; got {}",
            contact.submersion
        );
        assert_eq!(contact.kind, WaterKind::Clear);
    }

    #[test]
    fn water_at_reports_zero_submersion_at_the_surface() {
        // Water region top at y=100 (center 200, half 100). Body
        // centered (200, 110), half (10, 10) → top=100 (exactly at
        // the surface), bottom=120. depth = 0, submersion = 0.
        let mut world = World::new(
            "test",
            Vec2::new(500.0, 500.0),
            Vec2::new(10.0, 10.0),
            Vec::new(),
        );
        world.water_regions.push(WaterRegion::new(
            Aabb::new(Vec2::new(200.0, 200.0), Vec2::new(100.0, 100.0)),
            WaterKind::Clear,
            WaterVolumeSpec::default(),
        ));
        let body = Aabb::new(Vec2::new(200.0, 110.0), Vec2::new(10.0, 10.0));
        let contact = world.water_at(body).expect("body straddles surface");
        assert!(
            (contact.submersion - 0.0).abs() < 1e-3,
            "expected zero submersion at surface; got {}",
            contact.submersion
        );
    }

    #[test]
    fn climbable_at_returns_none_outside_any_region() {
        let world = World::new(
            "test",
            Vec2::new(500.0, 500.0),
            Vec2::new(10.0, 10.0),
            Vec::new(),
        );
        let body = Aabb::new(Vec2::new(50.0, 50.0), Vec2::new(5.0, 5.0));
        assert!(world.climbable_at(body).is_none());
    }

    #[test]
    fn climbable_at_reports_first_intersecting_region() {
        // Two ladders side-by-side. Body sits inside the second
        // (`right`); query should return that region's metrics, not
        // the first.
        let left =
            ClimbableRegion::ladder(Aabb::new(Vec2::new(100.0, 200.0), Vec2::new(20.0, 100.0)));
        let right =
            ClimbableRegion::ladder(Aabb::new(Vec2::new(300.0, 200.0), Vec2::new(20.0, 100.0)));
        let world = World::new(
            "test",
            Vec2::new(500.0, 500.0),
            Vec2::new(10.0, 10.0),
            Vec::new(),
        )
        .with_climbable_regions(vec![left, right]);
        let body = Aabb::new(Vec2::new(305.0, 220.0), Vec2::new(10.0, 16.0));
        let contact = world.climbable_at(body).expect("body inside right ladder");
        assert!(
            (contact.center_x - 300.0).abs() < f32::EPSILON,
            "expected right-ladder center_x=300, got {}",
            contact.center_x
        );
        assert!(
            (contact.top_y - 100.0).abs() < f32::EPSILON,
            "expected top_y=100 (center 200 - half 100), got {}",
            contact.top_y
        );
        assert!(
            (contact.bottom_y - 300.0).abs() < f32::EPSILON,
            "expected bottom_y=300 (center 200 + half 100), got {}",
            contact.bottom_y
        );
        assert_eq!(contact.kind, ClimbableKind::Ladder);
    }

    #[test]
    fn climbable_kind_supports_ladder_wall_vine_variants() {
        // Compile-time check that all three kinds can be constructed
        // and round-trip through ClimbableRegion::new. The variants
        // exist so future authoring layers can drop in without a
        // breaking enum change.
        let aabb = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let ladder = ClimbableRegion::new(aabb, ClimbableKind::Ladder, ClimbableSpec::default());
        let wall = ClimbableRegion::new(aabb, ClimbableKind::Wall, ClimbableSpec::default());
        let vine = ClimbableRegion::new(aabb, ClimbableKind::Vine, ClimbableSpec::default());
        assert_eq!(ladder.kind, ClimbableKind::Ladder);
        assert_eq!(wall.kind, ClimbableKind::Wall);
        assert_eq!(vine.kind, ClimbableKind::Vine);
    }

    #[test]
    fn climbable_spec_defaults_match_design_intent() {
        // Default spec: 180 px/sec climb, 0.25 strafe factor.
        // Ladder is faster than fall (32 px/16ms ≈ 2 frames) but
        // slower than walk (~360 px/sec) so the player can plausibly
        // beat a falling enemy to the next rung but can't speed-run
        // ladders.
        let spec = ClimbableSpec::default();
        assert!(
            (spec.climb_speed - 180.0).abs() < f32::EPSILON,
            "default climb_speed should be 180 (got {})",
            spec.climb_speed
        );
        assert!(
            (spec.strafe_factor - 0.25).abs() < f32::EPSILON,
            "default strafe_factor should be 0.25 (got {})",
            spec.strafe_factor
        );
    }
}
