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
//! reachability); `ambition_actors` owns the body-generic **builder** that
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

use ae::AabbExt;
use ambition_engine_core as ae;

use crate::actor::ActorFaction;

/// A world-space rectangular region a body can perceive — the AI analogue of the
/// human's screen (invariant I5). Axis-aligned, so it is gravity-independent
/// (invariant I10): rotating gravity does not rotate what a body can see.
#[derive(Clone, Copy, Debug, Default)]
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

/// What a body is DOING this tick, as a human reads it off the animation.
///
/// The no-cheat contract (`docs/planning/engine/fighter-brain.md` §1) lists
/// *"move phase/animation state"* among the things the view may carry, because a
/// human sees the windup and knows the punish window. This is that field. It is a
/// perception vocabulary, deliberately independent of `ambition_combat`'s
/// `AttackPhase` — the view crate sits BELOW combat, and a brain reads what it
/// can see, not the combat model's internals. The gameplay-layer builder maps.
///
/// Discriminated in the order a resolver should test them: a body in hitstun is
/// not shielding, and a body mid-swing is not neutral.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BodyPhase {
    /// Free: no attack in flight, no hitstun, no guard raised.
    #[default]
    Neutral,
    /// Reeling from a hit — input authority reduced or gone. L1's `Advantage`
    /// when it is the OPPONENT, `Disadvantage` when it is self.
    Hitstun,
    /// Committed to an attack, hitbox not yet live. The punish window.
    AttackStartup,
    /// Hitbox live. Do not walk into it.
    AttackActive,
    /// Attack over, still locked in endlag. The other punish window.
    AttackRecovery,
    /// Guard raised.
    Shielding,
}

impl BodyPhase {
    /// Any part of a swing — startup, active, or recovery.
    pub fn is_attacking(self) -> bool {
        matches!(
            self,
            Self::AttackStartup | Self::AttackActive | Self::AttackRecovery
        )
    }

    /// Committed and unable to answer: the frames a punish lands in. Active is
    /// NOT punishable — that is where the hitbox is.
    pub fn is_punishable(self) -> bool {
        matches!(
            self,
            Self::AttackStartup | Self::AttackRecovery | Self::Hitstun
        )
    }
}

/// The stage a fight happens on — the geometry a human reads off the whole
/// screen, not just their viewport. L1's `Recovery` (self offstage) and
/// `EdgeGuard` (opponent recovering) are undecidable without it.
///
/// Not viewport-clipped, and that is not a cheat: a Smash player can see the
/// blastzones. `bounds` is the room's world AABB — the envelope CC3's invariant 3
/// polices, so "offstage" here means exactly what "out of bounds" means there.
#[derive(Clone, Copy, Debug)]
pub struct StageView {
    /// The room's full extent in world px.
    pub bounds: ae::Aabb,
}

impl Default for StageView {
    /// The **empty** stage (inverted bounds), so every point is offstage. That is
    /// the honest reading of "no stage was supplied": a brain classifying
    /// `Recovery` from it is not lulled into thinking it is standing on ground.
    /// A zero-size box at the origin would have been subtly worse — the origin,
    /// and only the origin, would have read as safe.
    fn default() -> Self {
        Self {
            bounds: ae::Aabb {
                min: ae::Vec2::splat(f32::INFINITY),
                max: ae::Vec2::splat(f32::NEG_INFINITY),
            },
        }
    }
}

impl StageView {
    /// Is this point outside the stage envelope? The `Recovery` predicate.
    pub fn offstage(&self, p: ae::Vec2) -> bool {
        p.x < self.bounds.min.x
            || p.x > self.bounds.max.x
            || p.y < self.bounds.min.y
            || p.y > self.bounds.max.y
    }

    /// Distance from `p` to the nearest stage edge (0 when already outside).
    /// The corner-pressure feature L2 scores stage position risk with.
    pub fn distance_to_edge(&self, p: ae::Vec2) -> f32 {
        if self.offstage(p) {
            return 0.0;
        }
        (p.x - self.bounds.min.x)
            .min(self.bounds.max.x - p.x)
            .min(p.y - self.bounds.min.y)
            .min(self.bounds.max.y - p.y)
    }
}

/// One **other** actor perceived in the viewport. Controller-neutral: just the
/// facts a brain needs to decide, with hostility already resolved **relationally**
/// (non-player-centric) at build time, so the brain reads `hostile_to_self`
/// instead of pattern-matching factions.
#[derive(Clone, Debug, Default)]
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
    /// What it is doing, as read off the animation. The punish window.
    pub phase: BodyPhase,
    /// Seconds left in `phase`, where the body knows (windup / active / hitstun).
    /// `0.0` for `Neutral` and for phases with no authored clock. Frame data is
    /// public knowledge; a player who studied the character has this number.
    pub phase_remaining: f32,
    /// Currently in i-frames (post-hit invulnerability). Visible: the body flashes.
    pub invulnerable: bool,
    /// Accumulated damage — the smash-percent axis (CM1). Kill potential scales
    /// off it, so L2 cannot score a finisher without it.
    pub damage_taken: i32,
    /// This body's max health, so `damage_taken` normalizes. `0` = unknown.
    pub health_max: i32,
}

impl PerceivedActor {
    /// `damage_taken / health_max`, clamped to `0..=1`. `0.0` when max is unknown.
    pub fn damage_frac(&self) -> f32 {
        if self.health_max <= 0 {
            return 0.0;
        }
        (self.damage_taken as f32 / self.health_max as f32).clamp(0.0, 1.0)
    }
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

/// A portal aperture perceived in the viewport — the data a brain needs to
/// **route through it** (invariant I10 / S5's portal navigation). Plain data: no
/// dependency on the portal crate, so the perception value stays headless and
/// the builder (gameplay layer) converts the live `PlacedPortal` into this.
#[derive(Clone, Copy, Debug)]
pub struct PerceivedPortal {
    /// Aperture center on the surface (world px).
    pub pos: ae::Vec2,
    /// Unit outward normal of the surface the aperture sits on (±x / ±y).
    pub normal: ae::Vec2,
    /// Oriented half-extent of the opening (world px).
    pub half_extent: ae::Vec2,
    /// Stable key identifying which pair this aperture belongs to. The linked
    /// exit is the other portal with the same key — a brain entering one emerges
    /// at the other. (The gameplay builder derives this from `PortalChannel`.)
    pub channel_key: u64,
}

/// The viewing body's own state — kinematics plus **per-capability availability**
/// (what it can actually do right now, the body-enforced floor of invariant I3).
#[derive(Clone, Copy, Debug, Default)]
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
    /// What self is doing. `Disadvantage` (§1's L1 state) is `Hitstun` here.
    pub phase: BodyPhase,
    /// Seconds left in `phase` (see [`PerceivedActor::phase_remaining`]).
    pub phase_remaining: f32,
    /// Self is in i-frames.
    pub invulnerable: bool,
    /// Self's accumulated damage — the smash-percent axis (CM1).
    pub damage_taken: i32,
    /// Self's max health. `0` = unknown.
    pub health_max: i32,
}

impl SelfView {
    /// Acceleration frame defining this body's local side/down axes.
    pub fn acceleration_frame(&self) -> ae::AccelerationFrame {
        ae::AccelerationFrame::new(self.gravity_down)
    }

    /// `damage_taken / health_max`, clamped to `0..=1`. `0.0` when max is unknown.
    pub fn damage_frac(&self) -> f32 {
        if self.health_max <= 0 {
            return 0.0;
        }
        (self.damage_taken as f32 / self.health_max as f32).clamp(0.0, 1.0)
    }
}

/// Everything a body perceives this tick — the headless, controller-neutral
/// world-out value (invariant I5). Built per body, any faction.
#[derive(Clone, Debug, Default)]
pub struct WorldView {
    pub self_view: SelfView,
    pub viewport: Viewport,
    /// The whole stage, NOT viewport-clipped — a fighter can see the blastzones.
    pub stage: StageView,
    /// Other actors inside the viewport (self excluded).
    pub actors: Vec<PerceivedActor>,
    /// Projectiles inside the viewport.
    pub projectiles: Vec<PerceivedProjectile>,
    /// Local solid terrain clipped to the viewport.
    pub terrain: Vec<PerceivedSolid>,
    /// Portal apertures inside the viewport (for S5 routing).
    pub portals: Vec<PerceivedPortal>,
    /// Sim time (scaled clock seconds) this view was taken.
    pub sim_time: f32,
}

impl WorldView {
    /// Is self outside the stage envelope? L1's `Recovery` predicate.
    pub fn self_offstage(&self) -> bool {
        self.stage.offstage(self.self_view.pos)
    }

    /// Is `actor` outside the stage envelope? L1's `EdgeGuard` predicate — the
    /// opponent is recovering, and this is the moment to take the stock.
    pub fn actor_offstage(&self, actor: &PerceivedActor) -> bool {
        self.stage.offstage(actor.pos)
    }

    /// Hostile, alive actors in view — the candidate targets, relationally
    /// resolved (non-player-centric).
    pub fn hostiles(&self) -> impl Iterator<Item = &PerceivedActor> {
        self.actors.iter().filter(|a| a.hostile_to_self && a.alive)
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

    /// The exit aperture linked to `portal` — the other portal on the same
    /// channel, if it too is in view. A brain entering `portal` emerges here, so
    /// this is what it routes toward when chasing a target across an aperture.
    pub fn linked_portal(&self, portal: &PerceivedPortal) -> Option<&PerceivedPortal> {
        self.portals
            .iter()
            .find(|p| p.channel_key == portal.channel_key && p.pos != portal.pos)
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
        self.actors
            .retain(|_, m| m.confidence >= Self::FORGET_BELOW);
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

/// **A view a brain is allowed to read.** The no-cheat contract, made a type.
///
/// `docs/planning/engine/fighter-brain.md` §3's humanity checks ask for a test
/// that *"the delay buffer is on the ONLY read path"*. A test can be forgotten,
/// and a grep lint can be argued with. This cannot: `Perceived` has a private
/// field and only [`DelayedPerception::perceive`] constructs one, so a brain layer
/// that wanted to read the LIVE world would have to change this file to do it.
///
/// It derefs to the view, so reading is free. Minting is not.
#[derive(Clone, Copy, Debug)]
pub struct Perceived<'a>(&'a WorldView);

impl std::ops::Deref for Perceived<'_> {
    type Target = WorldView;
    fn deref(&self) -> &WorldView {
        self.0
    }
}

impl<'a> Perceived<'a> {
    /// Mint a `Perceived` from a view WITHOUT any latency. The name is the
    /// documentation: this is the frame-perfect path, and it exists for RL rigs,
    /// replay determinism fixtures, and the unit tests of the brain layers
    /// themselves — never for a shipped difficulty (§1.3: *"Level 9 = small
    /// numbers, never zero"*).
    ///
    /// FB4's profile loader is the only production caller, and only for a row whose
    /// `reaction_ms` is zero, which no shipped row has.
    pub fn cheating(view: &'a WorldView) -> Self {
        Self(view)
    }
}

/// **The perception delay-buffer** — the no-cheat contract's reaction latency,
/// made structural (`docs/planning/engine/fighter-brain.md` §1.3, §5).
///
/// A brain that reads the live view reacts in zero milliseconds, which no human
/// does. `FighterBrainProfile.reaction_ms` says how late the brain should see the
/// world; this is the thing that makes it so. Wrap the ONE view read: the
/// gameplay layer `observe`s each tick's fresh view, and every L1/L2/L3 code path
/// reads `perceive()`.
///
/// **Warm-up is deliberately stale, never fresh.** Before the buffer fills, it
/// returns the oldest view it holds — so a brain spawned mid-fight reacts *more*
/// slowly than its profile for a few ticks, never faster. The failure direction
/// matters: a buffer that returned the live view while filling would be a
/// same-tick perceive→act cheat exactly at the moment a fight starts, which is
/// the moment FB4's humanity check is looking at.
///
/// `delay_ticks == 0` is a legal profile (a frame-perfect brain, for RL rigs and
/// regression fixtures) and returns the live view. Shipped difficulty rows never
/// use it — §1.3: *"Level 9 = small numbers, never zero."*
#[derive(Clone, Debug, Default)]
pub struct DelayedPerception {
    /// Oldest first. Length is capped at `delay_ticks + 1`.
    buf: std::collections::VecDeque<WorldView>,
    delay_ticks: usize,
}

impl DelayedPerception {
    /// A buffer that shows the world `delay_ticks` ticks late.
    pub fn new(delay_ticks: usize) -> Self {
        Self {
            buf: std::collections::VecDeque::with_capacity(delay_ticks + 1),
            delay_ticks,
        }
    }

    /// Convert a profile's `reaction_ms` into ticks at the sim's rate, rounding to
    /// nearest. At 60 Hz: 150 ms → 9 ticks (level 9), 500 ms → 30 (level 1).
    pub fn from_reaction_ms(reaction_ms: f32, tick_hz: f32) -> Self {
        let ticks = if tick_hz > 0.0 && reaction_ms > 0.0 {
            (reaction_ms * tick_hz / 1000.0).round().max(0.0) as usize
        } else {
            0
        };
        Self::new(ticks)
    }

    /// How many ticks late this buffer shows the world.
    pub fn delay_ticks(&self) -> usize {
        self.delay_ticks
    }

    /// Feed this tick's live view. Call exactly once per sim tick, from the
    /// gameplay layer — the brain never calls it.
    pub fn observe(&mut self, view: WorldView) {
        self.buf.push_back(view);
        while self.buf.len() > self.delay_ticks + 1 {
            self.buf.pop_front();
        }
    }

    /// What the brain is allowed to read: the view from `delay_ticks` ticks ago,
    /// or the oldest one held if the buffer has not filled yet. `None` only before
    /// the first `observe`.
    ///
    /// Returns a [`Perceived`], not a `&WorldView`. That is the enforcement: §3's
    /// humanity check asks a test to *"assert the delay buffer is on the ONLY read
    /// path"*, and a type that only this method can mint makes the assertion
    /// unnecessary. A brain layer cannot accept a live view, because it cannot name
    /// one.
    pub fn perceive(&self) -> Option<Perceived<'_>> {
        self.buf.front().map(Perceived)
    }

    /// Ticks currently buffered. `delay_ticks + 1` once warm.
    pub fn buffered(&self) -> usize {
        self.buf.len()
    }

    /// True once `perceive()` is returning a view exactly `delay_ticks` old.
    pub fn warm(&self) -> bool {
        self.buf.len() == self.delay_ticks + 1
    }

    /// Drop every buffered view (respawn, room change, match reset). The brain
    /// goes blind for one tick rather than acting on a view of the old room.
    pub fn clear(&mut self) {
        self.buf.clear();
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
            ..Default::default()
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
            ..Default::default()
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
            portals: vec![],
            sim_time: 0.0,
            ..Default::default()
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
            portals: vec![],
            sim_time: 0.0,
            ..Default::default()
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
            portals: vec![],
            sim_time: 0.0,
            ..Default::default()
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
            portals: vec![],
            sim_time: 0.0,
            ..Default::default()
        };
        assert_eq!(view.incoming_threats().count(), 1);
    }

    #[test]
    fn linked_portal_finds_the_paired_exit() {
        let blue_a = PerceivedPortal {
            pos: ae::Vec2::new(50.0, 0.0),
            normal: ae::Vec2::new(-1.0, 0.0),
            half_extent: ae::Vec2::new(4.0, 24.0),
            channel_key: 7,
        };
        let blue_b = PerceivedPortal {
            pos: ae::Vec2::new(300.0, 0.0),
            normal: ae::Vec2::new(1.0, 0.0),
            half_extent: ae::Vec2::new(4.0, 24.0),
            channel_key: 7,
        };
        let orange = PerceivedPortal {
            pos: ae::Vec2::new(150.0, 0.0),
            normal: ae::Vec2::new(0.0, -1.0),
            half_extent: ae::Vec2::new(24.0, 4.0),
            channel_key: 9,
        };
        let view = WorldView {
            self_view: self_view_at(ae::Vec2::ZERO, ActorFaction::Enemy),
            viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(500.0)),
            actors: vec![],
            projectiles: vec![],
            terrain: vec![],
            portals: vec![blue_a, blue_b, orange],
            sim_time: 0.0,
            ..Default::default()
        };
        // Entering blue_a emerges at blue_b (same channel, other aperture).
        assert_eq!(view.linked_portal(&blue_a).map(|p| p.pos), Some(blue_b.pos));
        // The orange aperture has no pair in view → no linked exit.
        assert!(view.linked_portal(&orange).is_none());
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
            portals: vec![],
            sim_time: 0.0,
            ..Default::default()
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
            portals: vec![],
            sim_time: 0.0,
            ..Default::default()
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

    // ── FB1: the perception delay buffer (the no-cheat contract's §1.3) ──

    fn view_at_time(t: f32) -> WorldView {
        WorldView {
            sim_time: t,
            ..Default::default()
        }
    }

    #[test]
    fn a_zero_delay_buffer_shows_the_live_view() {
        let mut d = DelayedPerception::new(0);
        d.observe(view_at_time(1.0));
        assert_eq!(d.perceive().map(|v| v.sim_time), Some(1.0));
        d.observe(view_at_time(2.0));
        assert_eq!(d.perceive().map(|v| v.sim_time), Some(2.0));
        assert!(d.warm());
    }

    /// The whole point: at level 9 the brain sees the world 9 ticks ago.
    #[test]
    fn a_warm_buffer_shows_the_world_exactly_delay_ticks_late() {
        let mut d = DelayedPerception::new(3);
        for t in 0..10 {
            d.observe(view_at_time(t as f32));
        }
        assert!(d.warm());
        assert_eq!(
            d.perceive().map(|v| v.sim_time),
            Some(6.0),
            "tick 9 observed; the brain must be looking at tick 9-3"
        );
        assert_eq!(d.buffered(), 4);
    }

    /// **The failure direction that matters.** While filling, the buffer returns
    /// the STALEST view it holds, never a fresher one. A brain spawned mid-fight
    /// reacts more slowly than its profile for a few ticks — it never gets a
    /// same-tick perceive→act cheat at the exact moment a fight begins, which is
    /// the moment FB4's humanity check is watching.
    #[test]
    fn warming_up_is_stale_never_fresh() {
        let mut d = DelayedPerception::new(5);
        assert!(d.perceive().is_none(), "blind before the first observe");
        for t in 0..5 {
            d.observe(view_at_time(t as f32));
            assert!(!d.warm());
            let seen = d.perceive().expect("something buffered").sim_time;
            let age = t as f32 - seen;
            assert!(
                age <= d.delay_ticks() as f32,
                "never fresher than the profile"
            );
            assert_eq!(seen, 0.0, "the oldest held view, not the newest");
        }
    }

    #[test]
    fn reaction_ms_converts_at_the_sim_rate() {
        // The doc's ladder endpoints, at 60 Hz.
        assert_eq!(
            DelayedPerception::from_reaction_ms(150.0, 60.0).delay_ticks(),
            9
        );
        assert_eq!(
            DelayedPerception::from_reaction_ms(500.0, 60.0).delay_ticks(),
            30
        );
        // A frame-perfect brain is a legal profile (RL rigs, regression fixtures).
        assert_eq!(
            DelayedPerception::from_reaction_ms(0.0, 60.0).delay_ticks(),
            0
        );
    }

    /// A respawn or room change invalidates every buffered view. The brain goes
    /// blind for a tick rather than acting on a picture of the old room.
    #[test]
    fn clearing_blinds_the_brain_rather_than_stranding_a_stale_room() {
        let mut d = DelayedPerception::new(2);
        for t in 0..5 {
            d.observe(view_at_time(t as f32));
        }
        d.clear();
        assert!(d.perceive().is_none());
        d.observe(view_at_time(100.0));
        assert_eq!(d.perceive().map(|v| v.sim_time), Some(100.0));
    }

    // ── FB1: stage geometry, the L1 classifier's missing input ──

    fn stage_400() -> StageView {
        StageView {
            bounds: ae::Aabb::new(ae::Vec2::new(200.0, 200.0), ae::Vec2::new(200.0, 200.0)),
        }
    }

    #[test]
    fn offstage_is_outside_the_rooms_envelope() {
        let s = stage_400();
        assert!(!s.offstage(ae::Vec2::new(200.0, 200.0)));
        assert!(
            !s.offstage(ae::Vec2::new(0.0, 0.0)),
            "the corner is on-stage"
        );
        assert!(s.offstage(ae::Vec2::new(-1.0, 200.0)), "past the left edge");
        assert!(
            s.offstage(ae::Vec2::new(200.0, 401.0)),
            "under the blastzone"
        );
    }

    #[test]
    fn distance_to_edge_is_zero_offstage_and_measures_corner_pressure_on_it() {
        let s = stage_400();
        assert_eq!(s.distance_to_edge(ae::Vec2::new(-50.0, 200.0)), 0.0);
        assert_eq!(s.distance_to_edge(ae::Vec2::new(200.0, 200.0)), 200.0);
        assert_eq!(
            s.distance_to_edge(ae::Vec2::new(10.0, 200.0)),
            10.0,
            "cornered: 10px from the left wall"
        );
    }

    /// The two L1 predicates the stage exists to serve: `Recovery` (self offstage)
    /// and `EdgeGuard` (the opponent is).
    #[test]
    fn the_view_answers_recovery_and_edgeguard() {
        let mut view = WorldView {
            self_view: self_view_at(ae::Vec2::new(-40.0, 200.0), ActorFaction::Enemy),
            stage: stage_400(),
            actors: vec![perceived(
                "foe",
                ae::Vec2::new(200.0, 200.0),
                ActorFaction::Player,
                true,
            )],
            ..Default::default()
        };
        assert!(view.self_offstage(), "self is past the left blastzone");
        assert!(!view.actor_offstage(&view.actors[0]));

        view.self_view.pos = ae::Vec2::new(200.0, 200.0);
        view.actors[0].pos = ae::Vec2::new(200.0, 500.0);
        assert!(!view.self_offstage());
        assert!(view.actor_offstage(&view.actors[0]), "they are recovering");
    }

    /// A default view has an EMPTY stage, so every point is offstage. The first
    /// draft used a zero-size box at the origin, which made the origin — and only
    /// the origin — read as on-stage. That is exactly the kind of quiet lie the
    /// view must not tell.
    #[test]
    fn a_stageless_view_reads_as_entirely_offstage() {
        assert!(WorldView::default().self_offstage());
        let s = StageView::default();
        assert!(s.offstage(ae::Vec2::ZERO));
        assert!(s.offstage(ae::Vec2::new(1e6, -1e6)));
        assert_eq!(s.distance_to_edge(ae::Vec2::ZERO), 0.0);
    }

    // ── FB1: the damage meter (CM1's smash-percent axis) ──

    #[test]
    fn damage_frac_normalizes_and_survives_an_unknown_max() {
        let mut a = perceived("foe", ae::Vec2::ZERO, ActorFaction::Player, true);
        a.health_max = 200;
        a.damage_taken = 50;
        assert_eq!(a.damage_frac(), 0.25);
        a.health_max = 0;
        assert_eq!(a.damage_frac(), 0.0, "unknown max reads as undamaged");
    }

    #[test]
    fn phase_classification_names_the_punish_windows() {
        assert!(BodyPhase::AttackStartup.is_punishable());
        assert!(BodyPhase::AttackRecovery.is_punishable());
        assert!(BodyPhase::Hitstun.is_punishable());
        assert!(
            !BodyPhase::AttackActive.is_punishable(),
            "the hitbox is out — walking in is not a punish"
        );
        assert!(!BodyPhase::Neutral.is_punishable());
        assert!(BodyPhase::AttackActive.is_attacking());
        assert!(!BodyPhase::Shielding.is_attacking());
    }
}
