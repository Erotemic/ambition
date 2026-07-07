//! Generated sandbox room data.
//!
//! The engine models room geometry as named blocks. The Bevy sandbox decides
//! how to draw each block; the engine only cares about collision semantics.

use crate::geometry::{aabb_from_min_size, Aabb, AabbExt};
use crate::Vec2;

/// Upgrade tier required to blink through a blink wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BlinkWallTier {
    /// Intended to be passable by an early blink-phasing upgrade.
    Soft,
    /// Intended to remain blocked until a stronger blink-phasing upgrade.
    Hard,
}

/// Collision/gameplay meaning of a generated world block.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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

impl BlockKind {
    /// True for authored surfaces that a downward pogo strike may
    /// bounce from.
    ///
    /// Keep this policy centralized so the control-phase pogo helper
    /// and the melee/attack-phase pogo helper cannot drift apart.
    pub fn is_pogo_target(self) -> bool {
        matches!(self, Self::PogoOrb | Self::Rebound { .. })
    }
}

/// One piece of generated room geometry.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Block {
    /// Durable geometry identity (collision-and-ccd.md §3.6). `name` stays the
    /// human label; `id` is what `WorldDelta` ops / the CC6 portal host ref /
    /// traces name. Fixture constructors default to `GeoSource::Anon`; the IR
    /// emission paths assign real sources.
    pub id: crate::geo_id::GeoId,
    pub name: String,
    pub aabb: Aabb,
    pub kind: BlockKind,
    /// Per-frame displacement of this solid (a moving platform's `last_delta`;
    /// `ZERO` for static geometry — the common case). The collision sweep carries
    /// any body resting on the block by this, so "riding a moving platform" is an
    /// emergent property of standing on a moving solid — uniform across every body
    /// (player, clone, enemy, slug), with no per-actor wiring. A static solid is
    /// just the `velocity == ZERO` degenerate case.
    pub velocity: Vec2,
}

impl Block {
    /// The block's exterior boundary as a closed, exterior-rideable
    /// [`SurfaceChain`] (positive shoelace: outward normals under the shared
    /// `n = (t.y, -t.x)` winding rule). This is what makes a solid block a
    /// SURFACE to the momentum solver — one riding model for authored chains
    /// and ordinary room geometry. Carries the block's `velocity`, so riding
    /// a moving block composes exactly like riding a moving chain.
    pub fn boundary_chain(&self) -> SurfaceChain {
        let min = self.aabb.min;
        let max = self.aabb.max;
        let mut chain = SurfaceChain::closed_loop(
            self.name.clone(),
            vec![
                Vec2::new(min.x, min.y), // top-left (y grows downward)
                Vec2::new(max.x, min.y), // top-right
                Vec2::new(max.x, max.y), // bottom-right
                Vec2::new(min.x, max.y), // bottom-left
            ],
        );
        chain.velocity = self.velocity;
        chain
    }

    pub fn solid(name: impl Into<String>, min: Vec2, size: Vec2) -> Self {
        Self {
            id: crate::geo_id::GeoId::anon(),
            name: name.into(),
            aabb: aabb_from_min_size(min, size),
            kind: BlockKind::Solid,
            velocity: Vec2::ZERO,
        }
    }

    pub fn blink_wall(name: impl Into<String>, min: Vec2, size: Vec2, tier: BlinkWallTier) -> Self {
        Self {
            id: crate::geo_id::GeoId::anon(),
            name: name.into(),
            aabb: aabb_from_min_size(min, size),
            kind: BlockKind::BlinkWall { tier },
            velocity: Vec2::ZERO,
        }
    }

    pub fn one_way(name: impl Into<String>, min: Vec2, size: Vec2) -> Self {
        Self {
            id: crate::geo_id::GeoId::anon(),
            name: name.into(),
            aabb: aabb_from_min_size(min, size),
            kind: BlockKind::OneWay,
            velocity: Vec2::ZERO,
        }
    }

    pub fn hazard(name: impl Into<String>, min: Vec2, size: Vec2) -> Self {
        Self {
            id: crate::geo_id::GeoId::anon(),
            name: name.into(),
            aabb: aabb_from_min_size(min, size),
            kind: BlockKind::Hazard,
            velocity: Vec2::ZERO,
        }
    }

    pub fn pogo_orb(name: impl Into<String>, center: Vec2, radius: f32) -> Self {
        Self {
            id: crate::geo_id::GeoId::anon(),
            name: name.into(),
            aabb: Aabb::new(center, Vec2::new(radius, radius)),
            kind: BlockKind::PogoOrb,
            velocity: Vec2::ZERO,
        }
    }

    pub fn rebound(name: impl Into<String>, min: Vec2, size: Vec2, impulse: Vec2) -> Self {
        Self {
            id: crate::geo_id::GeoId::anon(),
            name: name.into(),
            aabb: aabb_from_min_size(min, size),
            kind: BlockKind::Rebound { impulse },
            velocity: Vec2::ZERO,
        }
    }
}

/// Authored water volume tuning. The simulation reads this when the
/// player is inside the AABB.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WaterKind {
    /// Mostly transparent. Player and submerged geometry stay visible.
    Clear,
    /// Opaque-ish; hides what's under the surface.
    Murky,
}

/// One axis-aligned water region on the world grid. Multiple regions
/// may exist in the same room; queries return the first that contains
/// the player AABB.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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

/// Gameplay meaning of a [`SurfaceChain`]. Deliberately tiny — semantics grow
/// when content demands them (design-balance: knobs when use cases land).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SurfaceKind {
    /// A rideable ground surface (slopes, hills, loop tracks).
    Ground,
}

/// The local frame of a chain at an arc-length position: where you are, which
/// way the surface runs, which way is off the surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceFrame {
    pub point: Vec2,
    /// Unit tangent along increasing arc length.
    pub tangent: Vec2,
    /// Unit outward normal — `(t.y, -t.x)`, the side a body rides on. Matches
    /// the [`crate::collision_semantics::Contact`] winding (a floor chain
    /// authored left→right has tangent `(1,0)` and normal `(0,-1)` = up).
    pub normal: Vec2,
    /// Index of the segment this frame lies on.
    pub segment: usize,
}

/// The first richer-than-AABB world primitive (fable review 2026-07-05, AJ10
/// layer 2): a polyline surface a momentum body can ride along — slopes,
/// hills, valleys, and (when `closed`) full loops.
///
/// Conventions:
/// - One-sided by winding: bodies ride the `+normal` side, where
///   `normal = (t.y, -t.x)`. Author floors left→to→right (normals up, with y
///   growing downward). A rideable loop INTERIOR is a closed chain with
///   negative shoelace [`SurfaceChain::signed_area`].
/// - Normals are DERIVED, never authored — a validator checks the geometry
///   instead ([`SurfaceChain::validate`]), so inverted normals /
///   discontinuous joins can't masquerade as physics bugs.
/// - `velocity` is the chain's own per-frame motion (a moving surface's
///   `last_delta`, like [`Block::velocity`]); contact carry falls out of the
///   contact frame, not a special case.
/// - Chains are collision geometry ONLY for bodies that opt in (the
///   surface-momentum motion model). The axis-swept AABB path never sees
///   them — AABB stays the protected fast path.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SurfaceChain {
    pub name: String,
    /// Polyline vertices. For a `closed` chain the last point connects back
    /// to the first (do NOT duplicate the first point at the end).
    pub points: Vec<Vec2>,
    pub closed: bool,
    pub kind: SurfaceKind,
    /// Per-frame displacement of this surface (`ZERO` for static geometry).
    pub velocity: Vec2,
}

impl SurfaceChain {
    pub fn open(name: impl Into<String>, points: Vec<Vec2>) -> Self {
        Self {
            name: name.into(),
            points,
            closed: false,
            kind: SurfaceKind::Ground,
            velocity: Vec2::ZERO,
        }
    }

    pub fn closed_loop(name: impl Into<String>, points: Vec<Vec2>) -> Self {
        Self {
            name: name.into(),
            points,
            closed: true,
            kind: SurfaceKind::Ground,
            velocity: Vec2::ZERO,
        }
    }

    pub fn segment_count(&self) -> usize {
        if self.points.len() < 2 {
            0
        } else if self.closed {
            self.points.len()
        } else {
            self.points.len() - 1
        }
    }

    /// Endpoints of segment `i` (wraps for the closing segment).
    pub fn segment(&self, i: usize) -> (Vec2, Vec2) {
        let a = self.points[i % self.points.len()];
        let b = self.points[(i + 1) % self.points.len()];
        (a, b)
    }

    pub fn segment_length(&self, i: usize) -> f32 {
        let (a, b) = self.segment(i);
        (b - a).length()
    }

    pub fn tangent(&self, i: usize) -> Vec2 {
        let (a, b) = self.segment(i);
        (b - a).normalize_or_zero()
    }

    /// Outward normal of segment `i`: the tangent rotated by the shared
    /// winding rule `n = (t.y, -t.x)`.
    pub fn normal(&self, i: usize) -> Vec2 {
        let t = self.tangent(i);
        Vec2::new(t.y, -t.x)
    }

    pub fn total_length(&self) -> f32 {
        (0..self.segment_count())
            .map(|i| self.segment_length(i))
            .sum()
    }

    /// The surface frame at arc length `s`. `s` WRAPS on a closed chain and
    /// CLAMPS to the ends of an open one (falling off an open end is the
    /// solver's job, not the geometry's).
    pub fn frame_at(&self, s: f32) -> SurfaceFrame {
        let total = self.total_length();
        debug_assert!(total > 0.0, "frame_at on a degenerate chain");
        let mut s = if self.closed {
            s.rem_euclid(total)
        } else {
            s.clamp(0.0, total)
        };
        let count = self.segment_count();
        for i in 0..count {
            let len = self.segment_length(i);
            if s <= len || i == count - 1 {
                let (a, b) = self.segment(i);
                let t = self.tangent(i);
                let f = if len > 0.0 {
                    (s / len).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                return SurfaceFrame {
                    point: a + (b - a) * f,
                    tangent: t,
                    normal: Vec2::new(t.y, -t.x),
                    segment: i,
                };
            }
            s -= len;
        }
        unreachable!("segment walk covers the arc length");
    }

    /// Project `p` onto the chain: returns `(arc_length, signed_distance)`
    /// of the closest point, where `signed_distance > 0` means `p` is on the
    /// rideable (`+normal`) side of that segment.
    pub fn project(&self, p: Vec2) -> (f32, f32) {
        let mut best: Option<(f32, f32, f32)> = None; // (|d|, s, signed d)
        let mut arc = 0.0;
        for i in 0..self.segment_count() {
            let (a, b) = self.segment(i);
            let ab = b - a;
            let len_sq = ab.length_squared();
            let t = if len_sq > 0.0 {
                ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let closest = a + ab * t;
            let d = (p - closest).length();
            let signed = (p - closest).dot(self.normal(i));
            let s = arc + ab.length() * t;
            if best.is_none_or(|(bd, _, _)| d < bd) {
                best = Some((d, s, signed));
            }
            arc += ab.length();
        }
        let (_, s, signed) = best.expect("project on a chain with segments");
        (s, signed)
    }

    /// Shoelace signed area of a CLOSED chain (0 for open chains). With the
    /// engine's y-down screen coordinates and the `n = (t.y, -t.x)` winding,
    /// a NEGATIVE area means the normals face the enclosed interior — the
    /// authoring for a rideable loop inside.
    pub fn signed_area(&self) -> f32 {
        if !self.closed || self.points.len() < 3 {
            return 0.0;
        }
        let mut twice_area = 0.0;
        for i in 0..self.points.len() {
            let a = self.points[i];
            let b = self.points[(i + 1) % self.points.len()];
            twice_area += a.x * b.y - b.x * a.y;
        }
        twice_area * 0.5
    }

    /// Authoring validation (the pragmatic tier from `spatial-model.md`:
    /// catch the geometry that would masquerade as physics bugs). Returns
    /// human-readable problems; empty = valid.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let min_points = if self.closed { 3 } else { 2 };
        if self.points.len() < min_points {
            problems.push(format!(
                "chain '{}': needs at least {min_points} points ({} authored)",
                self.name,
                self.points.len()
            ));
            return problems;
        }
        if self
            .points
            .iter()
            .any(|p| !p.x.is_finite() || !p.y.is_finite())
        {
            problems.push(format!("chain '{}': non-finite point", self.name));
            return problems;
        }
        for i in 0..self.segment_count() {
            if self.segment_length(i) < 1.0e-3 {
                problems.push(format!(
                    "chain '{}': segment {i} is degenerate (zero length) — joins must share \
                     a single vertex, not duplicate it",
                    self.name
                ));
            }
        }
        if self.closed && self.points.first() == self.points.last() && self.points.len() > 1 {
            problems.push(format!(
                "chain '{}': closed chain duplicates its first point at the end — the closing \
                 segment is implicit",
                self.name
            ));
        }
        // Self-intersection: any two non-adjacent segments crossing makes
        // support ambiguous. O(n²) — validation-time only.
        let count = self.segment_count();
        for i in 0..count {
            for j in (i + 2)..count {
                if self.closed && i == 0 && j == count - 1 {
                    continue; // adjacent through the wrap
                }
                let (a1, a2) = self.segment(i);
                let (b1, b2) = self.segment(j);
                if segments_cross(a1, a2, b1, b2) {
                    problems.push(format!(
                        "chain '{}': segments {i} and {j} cross — self-intersecting surface",
                        self.name
                    ));
                }
            }
        }
        problems
    }
}

/// Strict proper-crossing test (shared endpoints / collinear touches don't count).
fn segments_cross(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2) -> bool {
    fn orient(a: Vec2, b: Vec2, c: Vec2) -> f32 {
        (b - a).perp_dot(c - a)
    }
    let d1 = orient(b1, b2, a1);
    let d2 = orient(b1, b2, a2);
    let d3 = orient(a1, a2, b1);
    let d4 = orient(a1, a2, b2);
    (d1 * d2 < 0.0) && (d3 * d4 < 0.0)
}

/// Complete generated room spec.
///
/// Engine-side `World` carries only simulation primitives: blocks,
/// source-agnostic water + climbable regions, surface chains, and the room's
/// nominal size / spawn / display name. Authored entities (hazards, pickups,
/// chests, enemies, bosses, NPCs, switches, labels) live on the
/// sandbox-side `RoomSpec` in per-family Vecs — see
/// `crate::rooms::RoomSpec` in `ambition_actors`. The engine has no
/// authored-entity IR.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct World {
    pub name: String,
    pub size: Vec2,
    pub spawn: Vec2,
    pub blocks: Vec<Block>,
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
    /// Rideable surface chains (fable review 2026-07-05 AJ10). EMPTY for
    /// every AABB-only room — the zero-chain case takes the existing fast
    /// paths untouched; only surface-momentum bodies ever read this list.
    pub chains: Vec<SurfaceChain>,
}

/// First collision along a swept body path.
#[derive(Clone, Copy, Debug)]
pub struct SweepHit<'a> {
    pub block: &'a Block,
    /// Normalized time along the requested delta, in `[0, 1]`.
    pub time_of_impact: f32,
    /// Contact normal reported for the moving body by the underlying shape cast.
    pub normal1: Vec2,
}

impl World {
    pub fn new(name: impl Into<String>, size: Vec2, spawn: Vec2, blocks: Vec<Block>) -> Self {
        Self {
            name: name.into(),
            size,
            spawn,
            blocks,
            water_regions: Vec::new(),
            climbable_regions: Vec::new(),
            chains: Vec::new(),
        }
    }

    /// Builder-style setter for surface chains. Mirrors `with_water_regions`
    /// so every authoring source (LDtk entity, generated IR, native RON room)
    /// flows through one entry point.
    pub fn with_chains(mut self, chains: Vec<SurfaceChain>) -> Self {
        self.chains = chains;
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
        // AMBITION_REVIEW(discrete_ok): water is an ENTER/EXIT state region, not
        // a first-TOI trigger. A thick region can't be tunnelled (the body is
        // inside it for many frames); the ONLY tunnel risk is a region thinner
        // than one frame's travel, which `thin_region_warnings` flags at
        // authoring time — CC2 §3.3 sweeps the AUTHORING, not this per-frame
        // read (which RL steps millions of times).
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
        // AMBITION_REVIEW(discrete_ok): climbable is an ENTER/EXIT state region
        // (same rationale as `water_at`). Thin-strip tunnels are an authoring
        // defect `thin_region_warnings` catches, not a per-frame sweep concern.
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

    /// The first block matching `predicate` that `body` overlaps, if any. Used to
    /// read the surface a body is resting on — e.g. a moving platform's `velocity`,
    /// so the sweep can carry the rider.
    pub fn first_overlapping_block<F>(&self, body: Aabb, mut predicate: F) -> Option<&Block>
    where
        F: FnMut(&Block) -> bool,
    {
        self.blocks
            .iter()
            .find(|block| predicate(block) && body.strict_intersects(block.aabb))
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
            let Some(sweep_hit) = body.sweep_hit(delta, block.aabb) else {
                continue;
            };
            if best.is_none_or(|hit| sweep_hit.time_of_impact < hit.time_of_impact) {
                best = Some(SweepHit {
                    block,
                    time_of_impact: sweep_hit.time_of_impact,
                    normal1: sweep_hit.normal1,
                });
            }
        }
        best
    }

    /// CC2 §3.3 (the sweep law, authoring half): water + climbable regions are
    /// read DISCRETELY per frame (`water_at`/`climbable_at`) because they are
    /// ENTER/EXIT state regions, not first-TOI triggers — sweeping them every
    /// frame would cost the RL loop dearly for no gameplay gain. The ONE way a
    /// discrete region read can silently miss is a region thinner than a fast
    /// body's single-frame travel along the thin axis (the body starts one side
    /// and ends the other, never sampled inside). So the sweep moves to
    /// AUTHORING: this flags any region whose smaller dimension is under
    /// [`MIN_STATE_REGION_THICKNESS`], the floor below which a
    /// [`MAX_EXPECTED_BODY_SPEED`] body could tunnel it in a 60 Hz frame.
    /// Authors thicken the strip (or, for a genuinely thin trigger, convert the
    /// reader to a swept check). Non-fatal — returns human-readable warnings.
    pub fn thin_region_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let flag = |kind: &str, aabb: Aabb, out: &mut Vec<String>| {
            let thickness = aabb.width().min(aabb.height());
            if thickness < MIN_STATE_REGION_THICKNESS {
                out.push(format!(
                    "{kind} region at ({:.0}, {:.0}) is {thickness:.0}px thin — under the \
                     {MIN_STATE_REGION_THICKNESS:.0}px floor a fast body can tunnel in one \
                     frame; thicken it or make the reader swept (CC2 §3.3)",
                    aabb.center().x,
                    aabb.center().y,
                ));
            }
        };
        for region in &self.water_regions {
            flag("water", region.aabb, &mut warnings);
        }
        for region in &self.climbable_regions {
            flag("climbable", region.aabb, &mut warnings);
        }
        warnings
    }
}

/// The fastest sustained body speed CC2's authoring validator plans for —
/// comfortably above [`crate::movement::tuning::FLIGHT_TERMINAL_SPEED`] (760)
/// to cover dash / Sanic-momentum bursts. Blink is a discrete teleport handled
/// by its own swept path, so it is deliberately NOT the reference here.
pub const MAX_EXPECTED_BODY_SPEED: f32 = 1560.0;

/// Minimum thickness (px) an ENTER/EXIT state region (water / climbable) must
/// have so a [`MAX_EXPECTED_BODY_SPEED`] body cannot tunnel it in one 60 Hz
/// frame: `MAX_EXPECTED_BODY_SPEED / 60 = 26px`.
pub const MIN_STATE_REGION_THICKNESS: f32 = MAX_EXPECTED_BODY_SPEED / 60.0;

/// The active room's authored static spatial geometry — collision blocks,
/// water/climbable regions, bounds, spawn — exposed as a Bevy resource wrapping
/// [`World`].
///
/// (Formerly `GameWorld`; renamed because the old name named what it *wasn't*
/// — disambiguation from `bevy::ecs::World` — rather than what it is.)
///
/// This is the authored BASE, write-once-per-room: it is replaced wholesale at
/// a room boundary (load / reset / LDtk hot-reload), not mutated incrementally
/// mid-room. The collision the simulation actually sweeps against is a per-frame
/// derived *view* over this base plus dynamic overlay contributions (moving
/// platforms, ECS solids, portal carves) — built sandbox-side by the world
/// overlay (`world_with_sandbox_solids` in `ambition_actors`).
/// `RoomGeometry` is the geometry; the view is what you collide against.
#[derive(bevy_ecs::resource::Resource, Clone)]
pub struct RoomGeometry(pub World);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_new_starts_without_water_or_climbable_regions() {
        let world = World::new(
            "test",
            Vec2::new(100.0, 80.0),
            Vec2::new(20.0, 20.0),
            Vec::new(),
        );
        // Engine `World` no longer carries authored entities — those
        // live on the sandbox-side `RoomSpec`. Pin that the engine
        // starts with empty region lists too so future authors don't
        // re-add an authored-entity Vec without thinking about the
        // sandbox/engine boundary.
        assert!(world.water_regions.is_empty());
        assert!(world.climbable_regions.is_empty());
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
    fn chain_winding_matches_the_contact_convention() {
        // A floor authored left->right: tangent (1,0), normal (0,-1) = up in
        // y-down screen coordinates — identical to Contact::tangent's rule.
        let floor = SurfaceChain::open(
            "floor",
            vec![Vec2::new(0.0, 100.0), Vec2::new(200.0, 100.0)],
        );
        assert_eq!(floor.tangent(0), Vec2::new(1.0, 0.0));
        assert_eq!(floor.normal(0), Vec2::new(0.0, -1.0));
        assert_eq!(floor.total_length(), 200.0);
    }

    #[test]
    fn chain_frame_at_wraps_closed_and_clamps_open() {
        // A 100x100 square loop traversed so its normals face the INTERIOR
        // (rideable inside): floor L->R, up the right wall, R->L along the
        // ceiling, down the left wall. Negative shoelace area by convention.
        let square = SurfaceChain::closed_loop(
            "loop",
            vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(100.0, 0.0),
                Vec2::new(100.0, -100.0),
                Vec2::new(0.0, -100.0),
            ],
        );
        assert_eq!(square.segment_count(), 4);
        assert_eq!(square.total_length(), 400.0);
        assert!(square.signed_area() < 0.0, "interior-rideable winding");
        // Interior-facing normals on every segment: floor up, right wall
        // leftward, ceiling down, left wall rightward.
        assert_eq!(square.normal(0), Vec2::new(0.0, -1.0));
        assert_eq!(square.normal(1), Vec2::new(-1.0, 0.0));
        assert_eq!(square.normal(2), Vec2::new(0.0, 1.0));
        assert_eq!(square.normal(3), Vec2::new(1.0, 0.0));
        // Arc length wraps: s = 450 is s = 50, halfway along the floor.
        let f = square.frame_at(450.0);
        assert_eq!(f.segment, 0);
        assert!((f.point - Vec2::new(50.0, 0.0)).length() < 1e-4);
        // Negative s wraps backward onto the left wall.
        let back = square.frame_at(-50.0);
        assert_eq!(back.segment, 3);
        // An open chain clamps instead.
        let open = SurfaceChain::open("ramp", vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, -50.0)]);
        let end = open.frame_at(1.0e6);
        assert!((end.point - Vec2::new(100.0, -50.0)).length() < 1e-3);
    }

    #[test]
    fn chain_project_reports_arc_and_rideable_side() {
        let floor = SurfaceChain::open(
            "floor",
            vec![Vec2::new(0.0, 100.0), Vec2::new(200.0, 100.0)],
        );
        // A point ABOVE the floor (y < 100 in y-down coords) is on the
        // rideable +normal side.
        let (s, d) = floor.project(Vec2::new(50.0, 90.0));
        assert!((s - 50.0).abs() < 1e-4);
        assert!(d > 0.0, "above the floor is the rideable side (d = {d})");
        let (_, below) = floor.project(Vec2::new(50.0, 110.0));
        assert!(below < 0.0, "below the floor is the solid side");
    }

    #[test]
    fn chain_validate_catches_authoring_hazards() {
        // Too few points.
        assert!(!SurfaceChain::open("p", vec![Vec2::ZERO])
            .validate()
            .is_empty());
        // Degenerate segment (duplicated join vertex).
        let dup = SurfaceChain::open(
            "dup",
            vec![
                Vec2::ZERO,
                Vec2::new(50.0, 0.0),
                Vec2::new(50.0, 0.0),
                Vec2::new(100.0, 0.0),
            ],
        );
        assert!(dup.validate().iter().any(|p| p.contains("degenerate")));
        // Closed chain duplicating its first point at the end.
        let closed_dup = SurfaceChain::closed_loop(
            "ring",
            vec![
                Vec2::ZERO,
                Vec2::new(100.0, 0.0),
                Vec2::new(100.0, -100.0),
                Vec2::ZERO,
            ],
        );
        assert!(closed_dup
            .validate()
            .iter()
            .any(|p| p.contains("closing segment is implicit") || p.contains("degenerate")));
        // Self-intersection (a bowtie).
        let bowtie = SurfaceChain::open(
            "bowtie",
            vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(100.0, -100.0),
                Vec2::new(100.0, 0.0),
                Vec2::new(0.0, -100.0),
            ],
        );
        assert!(bowtie.validate().iter().any(|p| p.contains("cross")));
        // A healthy ramp validates clean.
        let ramp = SurfaceChain::open(
            "ramp",
            vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(100.0, -30.0),
                Vec2::new(200.0, -40.0),
            ],
        );
        assert!(ramp.validate().is_empty(), "{:?}", ramp.validate());
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
    fn thin_region_warnings_flags_tunnelable_regions_and_passes_thick_ones() {
        // A thick water pool and a thin climbable strip (a 6px-wide vertical
        // vine — under the 26px floor a fast body tunnels it in one frame).
        let world = World::new(
            "test",
            Vec2::new(500.0, 500.0),
            Vec2::new(10.0, 10.0),
            Vec::new(),
        )
        .with_water_regions(vec![WaterRegion::new(
            Aabb::new(Vec2::new(200.0, 200.0), Vec2::new(100.0, 40.0)),
            WaterKind::Clear,
            WaterVolumeSpec::default(),
        )])
        .with_climbable_regions(vec![ClimbableRegion::ladder(Aabb::new(
            Vec2::new(300.0, 200.0),
            Vec2::new(3.0, 100.0),
        ))]);
        let warnings = world.thin_region_warnings();
        assert_eq!(warnings.len(), 1, "only the thin vine warns: {warnings:?}");
        assert!(
            warnings[0].contains("climbable") && warnings[0].contains("tunnel"),
            "the warning names the thin climbable region: {warnings:?}"
        );
        // The floor is derived from the max expected body speed at 60 Hz.
        assert!((MIN_STATE_REGION_THICKNESS - 26.0).abs() < 1.0);
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

#[cfg(test)]
mod pogo_policy_tests {
    use super::*;

    #[test]
    fn pogo_target_policy_is_authored_pogo_or_rebound_only() {
        let rebound = BlockKind::Rebound {
            impulse: Vec2::ZERO,
        };
        let blink_wall = BlockKind::BlinkWall {
            tier: BlinkWallTier::Soft,
        };

        assert!(BlockKind::PogoOrb.is_pogo_target());
        assert!(rebound.is_pogo_target());
        assert!(!BlockKind::Solid.is_pogo_target());
        assert!(!BlockKind::OneWay.is_pogo_target());
        assert!(!blink_wall.is_pogo_target());
        assert!(!BlockKind::Hazard.is_pogo_target());
    }
}
