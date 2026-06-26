//! `WorldView` + `WorldMemory` — the **world-out** port (architecture roadmap S4).
//!
//! The unified-control architecture
//! (`docs/planning/fighter-capability-and-motor-unification.md`) gives every body
//! two ports: **intent-in** (a controller attempts an `ActorControlFrame`, the
//! body enforces) and **world-out** (the body produces a controller-neutral,
//! *headless* view of what it can perceive). This module is the world-out port.
//!
//! Two values:
//!
//! - [`WorldView`] — everything in the body's **viewport** this tick: other
//!   actors (pos / vel / facing / disposition / body-state), projectiles
//!   (pos / vel / threat), local terrain / solids, and `self` (kinematics +
//!   per-capability availability). The AI analogue of the human's screen
//!   (invariant I5), built **per body, any faction** — the same construction for
//!   the player-robot body as for the Perfect Cell-ular Automaton (guardrail #1).
//! - [`WorldMemory`] — the per-controller belief that **outlives the viewport**
//!   (invariant I6): last-known positions of actors that have left view, with a
//!   confidence that decays over time, so a controller can pursue a target that
//!   went off-screen instead of forgetting it the instant it leaves the frame.
//!
//! ### Why these types live here (and not in the gameplay crate)
//!
//! `WorldView` is what a **brain consumes**, exactly like [`crate::brain::BrainSnapshot`]:
//! the type belongs next to the brains, the *construction* (which reads the ECS
//! world) belongs in the gameplay layer. So this module owns the headless,
//! controller-neutral **value** and its pure tactical queries (line-of-fire,
//! reachability); `ambition_gameplay_core` owns the body-generic **builder** that
//! fills it from real solids / actors / projectiles. Zero rendering dependency
//! (invariant I5) and zero Bevy-world dependency here — just plain data over
//! [`ambition_engine_core`] geometry, so it is replay-deterministic and trivially
//! assertable in a headless harness.
//!
//! ### Frame-agnostic (invariant I10)
//!
//! The viewport is an axis-aligned world-space region, so it is gravity-independent
//! by construction. Tactical queries are world-space segment/sweep tests over the
//! same solids the body physically collides against — no `-y`-is-up assumption.
//! `self`'s gravity direction is carried in [`SelfView::gravity_down`] so a brain
//! that wants body-local reasoning can project into the acceleration frame.

use std::collections::HashMap;

use ambition_engine_core as ae;
use ae::AabbExt;

use crate::actor::ActorFaction;

/// A world-space rectangular region a body can perceive — the AI analogue of the
/// human's screen (invariant I5). Axis-aligned, so it is gravity-independent
/// (invariant I10): rotating gravity does not rotate what a body can see.
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    /// Center of the region (world px) — normally the body's position.
    pub center: ae::Vec2,
    /// Half-width / half-height of the region (world px).
    pub half_extent: ae::Vec2,
}

impl Viewport {
    /// A viewport of the given half-extent centered on `center`.
    pub fn around(center: ae::Vec2, half_extent: ae::Vec2) -> Self {
        Self {
            center,
            half_extent,
        }
    }

    /// Whether a world point is inside the viewport (inclusive of the edge).
    pub fn contains(&self, p: ae::Vec2) -> bool {
        (p.x - self.center.x).abs() <= self.half_extent.x
            && (p.y - self.center.y).abs() <= self.half_extent.y
    }

    /// The viewport as an [`ae::Aabb`], for overlap tests against block geometry.
    pub fn as_aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.center, self.half_extent)
    }
}

/// One **other** actor perceived in the viewport. Controller-neutral: just the
/// facts a brain needs to decide, with hostility already resolved **relationally**
/// (non-player-centric) at build time, so the brain reads `hostile_to_self`
/// instead of pattern-matching factions.
#[derive(Clone, Debug)]
pub struct PerceivedActor {
    /// Stable actor id (matches the body's config id) — the key [`WorldMemory`]
    /// remembers it under.
    pub id: String,
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub facing: f32,
    /// Half-extent of the perceived body's collision box (world px).
    pub half_extent: ae::Vec2,
    pub faction: ActorFaction,
    /// True iff the **viewing** body's faction is hostile to this actor's faction
    /// (resolved against `FactionRelations` at build time). The relational,
    /// non-player-centric "is this a target" signal (invariants behind S3e).
    pub hostile_to_self: bool,
    pub alive: bool,
    pub on_ground: bool,
    /// Whether this body currently has its reactive guard raised (S3c) — a brain
    /// can read it to avoid throwing into a block.
    pub shield_raised: bool,
}

/// One projectile perceived in the viewport. `hostile_to_self` is the threat
/// filter: a projectile fired by a faction hostile to the viewer can damage it.
#[derive(Clone, Copy, Debug)]
pub struct PerceivedProjectile {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub damage: i32,
    /// True iff this projectile's firing faction is hostile to the **viewer**
    /// (i.e. it can hurt me). Resolved relationally at build time.
    pub hostile_to_self: bool,
}

impl PerceivedProjectile {
    /// Whether this projectile is closing on `target` (its velocity has a
    /// positive component along `target - pos`). A cheap "incoming" test the
    /// brain uses to decide whether to dodge.
    pub fn is_closing_on(&self, target: ae::Vec2) -> bool {
        self.vel.dot(target - self.pos) > 0.0
    }
}

/// The perceived kind of a solid, distilled from the engine's `BlockKind` to the
/// facts perception cares about. Drives the tactical queries: which solids block
/// sight (line-of-fire) versus which block a body's path (reachability).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolidKind {
    /// Full collision both axes; blocks sight and movement.
    Solid,
    /// Blink-through wall: full collision, blocks sight and movement (a brain
    /// without the blink-through upgrade treats it as `Solid`).
    BlinkWall,
    /// Landing platform: solid only when crossed from the gravity side. Does not
    /// block sight; treated as passable for a coarse reachability test.
    OneWay,
    /// Reset/damage surface — does not block sight or movement, but a brain may
    /// want to avoid pathing through it.
    Hazard,
}

impl SolidKind {
    /// Whether a solid of this kind blocks line-of-sight / line-of-fire. Full
    /// collision surfaces block; thin platforms and hazards do not.
    pub fn blocks_sight(self) -> bool {
        matches!(self, Self::Solid | Self::BlinkWall)
    }

    /// Whether a solid of this kind blocks a body's straight-line path (coarse
    /// reachability). Same set as sight today; `OneWay` directionality is left to
    /// a finer query when a brain needs it.
    pub fn blocks_path(self) -> bool {
        matches!(self, Self::Solid | Self::BlinkWall)
    }
}

/// A solid block clipped into the viewport — the local terrain a brain reasons
/// over. Carries the **same** `ae::Aabb` the body physically collides against, so
/// tactical queries reuse the real geometry rather than a parallel sensor.
#[derive(Clone, Copy, Debug)]
pub struct PerceivedSolid {
    pub aabb: ae::Aabb,
    pub kind: SolidKind,
}

/// The viewing body's own state — kinematics plus **per-capability availability**
/// (what it can actually do right now, the body-enforced floor of invariant I3).
#[derive(Clone, Copy, Debug)]
pub struct SelfView {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub facing: f32,
    pub half_extent: ae::Vec2,
    /// Local gravity direction (unit). Frame-agnostic reasoning projects against
    /// this; defaults to screen-down `(0, 1)`.
    pub gravity_down: ae::Vec2,
    pub on_ground: bool,
    /// Gravity-free free-mover (a flyer): the brain steers 2D velocity directly.
    pub aerial: bool,
    pub alive: bool,
    pub faction: ActorFaction,
    /// Ranged attack available this tick (cooldown elapsed + capability present).
    pub can_fire: bool,
    /// Blink available this tick (capability + cooldown).
    pub can_blink: bool,
    /// Dash available this tick (capability + cooldown).
    pub can_dash: bool,
    /// Reactive guard available (capability present).
    pub can_shield: bool,
}

impl SelfView {
    /// Acceleration frame defining this body's local side/down axes.
    pub fn acceleration_frame(&self) -> ae::AccelerationFrame {
        ae::AccelerationFrame::new(self.gravity_down)
    }
}

/// Everything a body perceives this tick — the headless, controller-neutral
/// world-out value (invariant I5). Built per body, any faction.
#[derive(Clone, Debug)]
pub struct WorldView {
    pub self_view: SelfView,
    pub viewport: Viewport,
    /// Other actors inside the viewport (self excluded).
    pub actors: Vec<PerceivedActor>,
    /// Projectiles inside the viewport.
    pub projectiles: Vec<PerceivedProjectile>,
    /// Local solid terrain clipped to the viewport.
    pub terrain: Vec<PerceivedSolid>,
    /// Sim time (scaled clock seconds) this view was taken.
    pub sim_time: f32,
}

impl WorldView {
    /// Hostile, alive actors in view — the candidate targets, relationally
    /// resolved (non-player-centric).
    pub fn hostiles(&self) -> impl Iterator<Item = &PerceivedActor> {
        self.actors
            .iter()
            .filter(|a| a.hostile_to_self && a.alive)
    }

    /// Nearest hostile, alive actor in view, by straight-line distance from self.
    pub fn nearest_hostile(&self) -> Option<&PerceivedActor> {
        self.hostiles().min_by(|a, b| {
            let da = a.pos.distance_squared(self.self_view.pos);
            let db = b.pos.distance_squared(self.self_view.pos);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Hostile projectiles closing on self — the dodge candidates.
    pub fn incoming_threats(&self) -> impl Iterator<Item = &PerceivedProjectile> {
        let me = self.self_view.pos;
        self.projectiles
            .iter()
            .filter(move |p| p.hostile_to_self && p.is_closing_on(me))
    }

    /// Whether self has a clear line of fire to `to` — no sight-blocking solid
    /// between the body and the point. Reuses the real collision geometry (the
    /// same `ae::Aabb`s and the same swept-intersection primitive the physics
    /// uses), so "can I shoot it" agrees with "can a shot physically get there".
    pub fn line_of_fire(&self, to: ae::Vec2) -> bool {
        !segment_blocked(
            self.self_view.pos,
            to,
            ae::Vec2::splat(SIGHT_PROBE_HALF),
            &self.terrain,
            SolidKind::blocks_sight,
        )
    }

    /// Whether self can travel in a straight line to `to` without a solid in the
    /// way — a coarse reachability test sweeping the body's own collision box.
    /// (A finer query — jumps, one-way directionality — is a brain-stage refinement.)
    pub fn reachable(&self, to: ae::Vec2) -> bool {
        !segment_blocked(
            self.self_view.pos,
            to,
            self.self_view.half_extent,
            &self.terrain,
            SolidKind::blocks_path,
        )
    }
}

/// Half-extent of the thin probe used for the line-of-fire ray. Non-zero so the
/// swept-AABB primitive (parry shape-cast) is well-conditioned; small enough that
/// it behaves like a ray for sight purposes.
const SIGHT_PROBE_HALF: f32 = 0.5;

/// Sweep a box of `probe_half` from `from` to `to` and report whether any solid
/// matching `pred` is hit before the end of the segment. Uses the SAME
/// [`AabbExt::sweep_hit`] primitive (parry shape-cast) the physics step uses, over
/// the SAME block AABBs — never a parallel sensor.
fn segment_blocked(
    from: ae::Vec2,
    to: ae::Vec2,
    probe_half: ae::Vec2,
    terrain: &[PerceivedSolid],
    pred: impl Fn(SolidKind) -> bool,
) -> bool {
    let probe = ae::Aabb::new(from, probe_half);
    let delta = to - from;
    terrain.iter().filter(|s| pred(s.kind)).any(|s| {
        probe
            .sweep_hit(delta, s.aabb)
            .map(|hit| hit.time_of_impact < 1.0)
            .unwrap_or(false)
    })
}

/// What a controller believes about an actor it has seen — the unit of
/// [`WorldMemory`]. Position is the last *directly perceived* position; a brain
/// pursuing a vanished target heads here.
#[derive(Clone, Copy, Debug)]
pub struct RememberedActor {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub faction: ActorFaction,
    pub hostile_to_self: bool,
    /// Sim time the actor was last directly in view.
    pub last_seen: f32,
    /// Belief confidence in `[0, 1]`: `1.0` while in view, decaying once it
    /// leaves (invariant I6). A brain weights pursuit by this.
    pub confidence: f32,
}

/// The per-controller belief that outlives the viewport (invariant I6). Keyed by
/// actor id. Refreshed for everything currently seen, decayed for everything that
/// has left view, and forgotten once confidence falls below a floor.
///
/// Pure: `update` is a function of the previous memory + the current view + dt, so
/// it is replay-deterministic and assertable headless without a running app.
#[derive(Clone, Debug, Default)]
pub struct WorldMemory {
    actors: HashMap<String, RememberedActor>,
}

impl WorldMemory {
    /// Confidence half-life (seconds) once an actor leaves the viewport: every
    /// `DECAY_HALF_LIFE_S` of not-seeing it, confidence halves.
    pub const DECAY_HALF_LIFE_S: f32 = 3.0;
    /// Drop a remembered actor once confidence falls below this — fully forgotten.
    pub const FORGET_BELOW: f32 = 0.05;

    /// Fold this tick's view into memory: decay everything not currently seen,
    /// forget what has faded, then refresh everything in view to full confidence.
    pub fn update(&mut self, view: &WorldView, dt: f32) {
        let now = view.sim_time;
        let decay = 0.5_f32.powf((dt / Self::DECAY_HALF_LIFE_S).max(0.0));
        // Decay the unseen. (Iterating then inserting below is two disjoint
        // phases, so there's no borrow conflict.)
        for (id, mem) in self.actors.iter_mut() {
            if !view.actors.iter().any(|a| &a.id == id) {
                mem.confidence *= decay;
                // Dead-reckon the last-known position by its last-known velocity
                // so a pursuing brain heads where the target was going, not where
                // it last stood. Cheap, and self-correcting the moment it's re-seen.
                mem.pos += mem.vel * dt;
            }
        }
        self.actors.retain(|_, m| m.confidence >= Self::FORGET_BELOW);
        // Refresh everything in view to full confidence.
        for a in &view.actors {
            self.actors.insert(
                a.id.clone(),
                RememberedActor {
                    pos: a.pos,
                    vel: a.vel,
                    faction: a.faction,
                    hostile_to_self: a.hostile_to_self,
                    last_seen: now,
                    confidence: 1.0,
                },
            );
        }
    }

    /// What we remember about a specific actor, if anything.
    pub fn get(&self, id: &str) -> Option<&RememberedActor> {
        self.actors.get(id)
    }

    /// The most-confident remembered **hostile** — the target a brain pursues when
    /// none is currently in view (invariant I6: "move towards the last known
    /// position of the player to look for them").
    pub fn last_known_hostile(&self) -> Option<&RememberedActor> {
        self.actors
            .values()
            .filter(|m| m.hostile_to_self)
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// How many actors are currently remembered (in view or fading).
    pub fn len(&self) -> usize {
        self.actors.len()
    }

    /// Whether memory is empty.
    pub fn is_empty(&self) -> bool {
        self.actors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn self_view_at(pos: ae::Vec2, faction: ActorFaction) -> SelfView {
        SelfView {
            pos,
            vel: ae::Vec2::ZERO,
            facing: 1.0,
            half_extent: ae::Vec2::new(10.0, 16.0),
            gravity_down: ae::Vec2::new(0.0, 1.0),
            on_ground: true,
            aerial: false,
            alive: true,
            faction,
            can_fire: true,
            can_blink: false,
            can_dash: false,
            can_shield: false,
        }
    }

    fn perceived(id: &str, pos: ae::Vec2, faction: ActorFaction, hostile: bool) -> PerceivedActor {
        PerceivedActor {
            id: id.to_string(),
            pos,
            vel: ae::Vec2::ZERO,
            facing: 1.0,
            half_extent: ae::Vec2::new(10.0, 16.0),
            faction,
            hostile_to_self: hostile,
            alive: true,
            on_ground: true,
            shield_raised: false,
        }
    }

    fn wall(center: ae::Vec2, half: ae::Vec2) -> PerceivedSolid {
        PerceivedSolid {
            aabb: ae::Aabb::new(center, half),
            kind: SolidKind::Solid,
        }
    }

    #[test]
    fn viewport_contains_is_axis_aligned() {
        let v = Viewport::around(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(50.0, 30.0));
        assert!(v.contains(ae::Vec2::new(140.0, 120.0)));
        assert!(!v.contains(ae::Vec2::new(160.0, 100.0))); // outside x
        assert!(!v.contains(ae::Vec2::new(100.0, 140.0))); // outside y
    }

    #[test]
    fn nearest_hostile_picks_closest_alive_foe() {
        let view = WorldView {
            self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
            viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(500.0)),
            actors: vec![
                perceived("far", ae::Vec2::new(300.0, 0.0), ActorFaction::Boss, true),
                perceived("near", ae::Vec2::new(80.0, 0.0), ActorFaction::Boss, true),
                perceived("ally", ae::Vec2::new(20.0, 0.0), ActorFaction::Enemy, false),
            ],
            projectiles: vec![],
            terrain: vec![],
            sim_time: 0.0,
        };
        assert_eq!(view.nearest_hostile().map(|a| a.id.as_str()), Some("near"));
        // The closer-but-non-hostile ally is ignored.
        assert_eq!(view.hostiles().count(), 2);
    }

    #[test]
    fn line_of_fire_blocked_by_wall_clear_otherwise() {
        // Self at origin, target straight right at (200, 0). A wall at x=100 blocks.
        let view = WorldView {
            self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
            viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(500.0)),
            actors: vec![],
            projectiles: vec![],
            terrain: vec![wall(ae::Vec2::new(100.0, 0.0), ae::Vec2::new(8.0, 40.0))],
            sim_time: 0.0,
        };
        assert!(
            !view.line_of_fire(ae::Vec2::new(200.0, 0.0)),
            "a wall between self and target blocks the shot"
        );
        // A target above the wall (clear sky) is in line of fire.
        assert!(
            view.line_of_fire(ae::Vec2::new(200.0, -200.0)),
            "a path that misses the wall is clear"
        );
    }

    #[test]
    fn reachable_false_through_solid() {
        let view = WorldView {
            self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
            viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(500.0)),
            actors: vec![],
            projectiles: vec![],
            terrain: vec![wall(ae::Vec2::new(100.0, 0.0), ae::Vec2::new(20.0, 80.0))],
            sim_time: 0.0,
        };
        assert!(!view.reachable(ae::Vec2::new(200.0, 0.0)));
        assert!(view.reachable(ae::Vec2::new(0.0, -200.0)));
    }

    #[test]
    fn incoming_threats_only_hostile_and_closing() {
        let me = ae::Vec2::ZERO;
        let view = WorldView {
            self_view: self_view_at(me, ActorFaction::Enemy),
            viewport: Viewport::around(me, ae::Vec2::splat(500.0)),
            actors: vec![],
            projectiles: vec![
                // hostile, closing (to the left, toward me from the right)
                PerceivedProjectile {
                    pos: ae::Vec2::new(100.0, 0.0),
                    vel: ae::Vec2::new(-200.0, 0.0),
                    damage: 1,
                    hostile_to_self: true,
                },
                // hostile, receding
                PerceivedProjectile {
                    pos: ae::Vec2::new(100.0, 0.0),
                    vel: ae::Vec2::new(200.0, 0.0),
                    damage: 1,
                    hostile_to_self: true,
                },
                // closing but friendly
                PerceivedProjectile {
                    pos: ae::Vec2::new(-100.0, 0.0),
                    vel: ae::Vec2::new(200.0, 0.0),
                    damage: 1,
                    hostile_to_self: false,
                },
            ],
            terrain: vec![],
            sim_time: 0.0,
        };
        assert_eq!(view.incoming_threats().count(), 1);
    }

    #[test]
    fn memory_retains_target_after_it_leaves_view() {
        // Tick 1: a hostile is in view → memorized at full confidence.
        let mut mem = WorldMemory::default();
        let in_view = WorldView {
            self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
            viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(300.0)),
            actors: vec![perceived(
                "boss",
                ae::Vec2::new(100.0, 0.0),
                ActorFaction::Boss,
                true,
            )],
            projectiles: vec![],
            terrain: vec![],
            sim_time: 0.0,
        };
        mem.update(&in_view, 1.0 / 60.0);
        assert_eq!(mem.get("boss").map(|m| m.confidence), Some(1.0));

        // Now it leaves view: empty actor list, several ticks pass. The target is
        // still remembered (decaying), so a brain can pursue its last-known spot.
        let mut empty = in_view.clone();
        empty.actors.clear();
        for i in 0..30 {
            empty.sim_time = (i as f32 + 1.0) / 60.0;
            mem.update(&empty, 1.0 / 60.0);
        }
        let remembered = mem
            .last_known_hostile()
            .expect("the hostile that left view is still pursued");
        assert!(
            remembered.confidence < 1.0 && remembered.confidence > WorldMemory::FORGET_BELOW,
            "confidence decays but the target is not yet forgotten: {}",
            remembered.confidence
        );
        assert_eq!(remembered.faction, ActorFaction::Boss);
    }

    #[test]
    fn memory_forgets_after_long_absence() {
        let mut mem = WorldMemory::default();
        let mut view = WorldView {
            self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
            viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(300.0)),
            actors: vec![perceived(
                "ghost",
                ae::Vec2::new(50.0, 0.0),
                ActorFaction::Boss,
                true,
            )],
            projectiles: vec![],
            terrain: vec![],
            sim_time: 0.0,
        };
        mem.update(&view, 1.0 / 60.0);
        view.actors.clear();
        // ~20s of absence at the 3s half-life takes confidence well below the floor.
        for _ in 0..40 {
            mem.update(&view, 0.5);
        }
        assert!(
            mem.is_empty(),
            "a target unseen for many half-lives is forgotten"
        );
    }
}
