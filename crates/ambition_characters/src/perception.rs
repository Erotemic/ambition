//! Controller-neutral per-body perception.
//!
//! [`WorldView`] contains the body's current visible world; [`WorldMemory`]
//! retains decaying last-known observations outside the viewport. The value
//! types live with brains while ECS construction lives in the gameplay layer.
//! Geometry is world-space and gravity-independent; [`SelfView::gravity_down`]
//! lets brains project into a body-local frame when needed.

use ae::AabbExt;
use ambition_platformer2d_core as ae;

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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageView {
    /// The room's full extent in world px.
    pub bounds: ae::Aabb,
}

impl Default for StageView {
    /// The empty stage (inverted bounds), so every point is offstage. That is
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

    /// Was a stage supplied at all? [`Self::default`] is the inverted box, for
    /// which `offstage` is true EVERYWHERE — honest for "am I safe" (the answer
    /// is no), and a trap for "did I just die" (the answer would be yes, at the
    /// origin, on tick zero). A consumer that turns `offstage` into a KO has to
    /// ask this first.
    pub fn is_known(&self) -> bool {
        self.bounds.min.x <= self.bounds.max.x && self.bounds.min.y <= self.bounds.max.y
    }

    /// Horizontal room from `p` to the stage edge in direction `dir` (0 when
    /// already outside).
    ///
    /// [`Self::distance_to_edge`] answers *"how exposed am I"* — the nearest
    /// edge in any direction, which is what a risk score wants. This answers
    /// *"how far can I go THAT way"*, which is what a RETREAT asks, and the two
    /// disagree exactly at a ledge the body is standing beside rather than
    /// backed against.
    pub fn room_toward(&self, p: ae::Vec2, dir: f32) -> f32 {
        if self.offstage(p) {
            return 0.0;
        }
        if dir >= 0.0 {
            self.bounds.max.x - p.x
        } else {
            p.x - self.bounds.min.x
        }
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

/// One other actor perceived in the viewport. Controller-neutral: just the
/// facts a brain needs to decide, with hostility already resolved relationally
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
    /// True iff the viewing body's faction is hostile to this actor's faction
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
    /// Falling out of a launch, so this body's next landing is a knockdown
    /// unless it techs. Visible from across the stage, which is why a watcher
    /// gets it and not only the body itself.
    pub tumbling: bool,
    /// HANGING ON A LEDGE — not climbing out of one, hanging on it.
    ///
    /// ⭐ THE EDGE-GUARD WINDOW, and without this fact nothing could see it. A
    /// body on the ledge is off the stage and has to come back through you: it
    /// cannot walk, cannot shield, and every way out of the hang is a committed
    /// animation on a clock. That is the most punishable state in the genre, and
    /// it read as ORDINARY NEUTRAL here — `phase` is `Neutral`, the hang is
    /// inside the room's box so `offstage` is false, and `is_punishable` only
    /// knows about swings and hitstun.
    pub ledge_hanging: bool,
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
    /// True iff this projectile's firing faction is hostile to the viewer
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
}

/// A solid block clipped into the viewport — the local terrain a brain reasons
/// over. Carries the same `ae::Aabb` the body physically collides against, so
/// tactical queries reuse the real geometry rather than a parallel sensor.
#[derive(Clone, Copy, Debug)]
pub struct PerceivedSolid {
    pub aabb: ae::Aabb,
    pub kind: SolidKind,
}

/// A portal aperture perceived in the viewport — the data a brain needs to
/// route through it (invariant I10 / S5's portal navigation). Plain data: no
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

/// The viewing body's own state — kinematics plus per-capability availability
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
    /// WHAT THE SHARED BURST BUTTON WOULD DO IF PRESSED THIS TICK.
    ///
    /// Dodge and dash are ONE input; which one a press produces is decided by the body's
    /// current state — grounded or not, dodge cooldown, air-dodge budget and endlag, dash
    /// charges — and not by which abilities the body owns. `apply_dodge` declines on cooldown
    /// WITHOUT consuming the buffered press, so `apply_dash` takes it: a brain reading
    /// capabilities decides *I am dodging* and the body dashes.
    ///
    ///  [`ambition_platformer2d_core::resolve_burst_maneuver`] is the one rule,
    /// and this field is its answer. The brain is handed a fact.
    pub burst: ambition_platformer2d_core::BurstManeuver,
    /// `0` on the ground is normal — the count refreshes on landing.
    pub air_jumps_left: u8,
    /// Reactive guard available (capability present).
    pub can_shield: bool,
    /// What self is doing. `Disadvantage` (§1's L1 state) is `Hitstun` here.
    pub phase: BodyPhase,
    /// Seconds left in `phase` (see [`PerceivedActor::phase_remaining`]).
    pub phase_remaining: f32,
    /// Self is in i-frames.
    pub invulnerable: bool,
    /// Falling out of a launch: the next landing is a knockdown unless it is
    /// teched. See [`PerceivedActor::tumbling`] — a body cannot know this about
    /// itself more precisely than a watcher knows it.
    pub tumbling: bool,
    /// Self's accumulated damage — the smash-percent axis (CM1).
    pub damage_taken: i32,
    /// Self's max health. `0` = unknown.
    pub health_max: i32,
    /// Somebody is holding this body. Its ordinary options are gone; what it
    /// can do about that is escape, which does not exist yet.
    ///
    /// a READ MODEL, not the authority. `CapturedBy` is the relationship
    /// and it lives in combat; a brain that queried it directly would be a
    /// second reader of ECS state from inside a pure decision, which is the
    /// thing this whole perception layer exists to prevent.
    pub captured: bool,
    /// How long it has been held, in scaled seconds. `0.0` when free.
    pub captured_for: f32,
    /// This body is holding somebody. Its ordinary options are gone too, and
    /// the ones it has instead are the capture context: pummel, or throw.
    pub holding_captive: bool,
    /// `0` when it holds nobody. The capture policy's whole input today.
    pub pummels_landed: u8,
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
    /// How much floor is left in a direction, from the solid underfoot.
    ///
    /// `None` means no supporting solid was perceived — an AIRBORNE body, or a
    /// view whose terrain was never built. Neither is a ledge question, and
    /// reading "I cannot see the floor" as "the floor ends here" would freeze
    /// every brain in a composition that does not build terrain.
    ///
    /// On an enclosed room the room's edge and the floor's edge coincide, which is why nothing
    /// needed this until the smash stage — the first room in this engine you can walk out of.
    /// On a platform stage a body at the very edge of the floor is still 110px from the room
    /// boundary, so every "am I cornered" question answered against the stage says no while the
    /// fighter walks into the sky.
    ///
    /// One authority, because L1 asks it to classify and L2 asks it to score, and
    /// two implementations of "where does the floor end" would drift the moment
    /// one of them learned about one-way platforms.
    pub fn floor_ahead(&self, toward: f32) -> Option<f32> {
        let support = self.supporting_floor()?;
        let me = &self.self_view;
        Some(if toward >= 0.0 {
            support.max.x - me.pos.x
        } else {
            me.pos.x - support.min.x
        })
    }

    /// The solid I am standing on, as its full box — the authority behind
    /// [`Self::floor_ahead`], exposed because a body's *simulated* future needs
    /// the floor's EXTENT and not just the distance to one of its edges (the
    /// fighter brain's rollout walks a shadow body around and has to know when it
    /// has run out of ground).
    /// The floor a body would land on, standing or not.
    ///
    /// [`Self::supporting_floor`] answers only within about a body-height of
    /// the feet, because its question is *"what am I standing on"*. An AIRBORNE
    /// body has no supporting floor by that definition and therefore went blind
    /// about the platform exactly during recovery — the one moment the platform's
    /// extent decides whether it lives.
    ///
    /// This asks the other question: *"what is under me, if anything."* The
    /// nearest solid below the body's footprint, at any distance. `None` means
    /// there is genuinely nothing beneath — which for a fighter over the
    /// blastzone is the true and useful answer.
    /// Is this perceived solid something a body can STAND on?
    ///
    /// ⛔⛔ `BlinkWall` WAS MISSING, and [`SolidKind::BlinkWall`]'s own doc says
    /// it should not be: *"full collision, blocks sight and movement (a brain
    /// without the blink-through upgrade treats it as `Solid`)"*. Three floor
    /// questions each spelled `Solid | OneWay` inline and all three disagreed
    /// with that sentence — so a fighter standing on a platform contributed as
    /// a blink-passable block reported `ground=true supported=false
    /// floor_edge=None`, and every ledge question in the brain read through the
    /// false one.
    ///
    /// ⛔⛔ THIS COULD NOT LAND ALONE, and that is worth knowing before touching
    /// it again. The block in question was the SMASH DEMO'S RESPAWN PLATFORM,
    /// which was rebuilt under the protected fighter every tick — so making it
    /// visible handed the rollout a floor defined as *"wherever I am"*, whose
    /// perceived edge is a constant 48px however far the body walks. Every verb
    /// was then judged to walk off it and vetoed, every tick, and the ladder's
    /// level 6 regressed to its exact pre-fix numbers. The platform is placed
    /// ONCE now (`ambition_demo_smash::hold_the_respawn_platforms`), and the
    /// two changes were measured together.
    ///
    /// ⚠ `Hazard` is NOT ground and stays out: it *"does not block sight or
    /// movement"*, so nothing stands on it.
    fn is_standable(kind: SolidKind) -> bool {
        matches!(
            kind,
            SolidKind::Solid | SolidKind::OneWay | SolidKind::BlinkWall
        )
    }

    pub fn floor_below(&self) -> Option<ae::Aabb> {
        let me = &self.self_view;
        let feet = me.pos.y + me.half_extent.y;
        self.terrain
            .iter()
            .filter(|solid| Self::is_standable(solid.kind))
            .filter(|solid| {
                solid.aabb.min.x <= me.pos.x + me.half_extent.x
                    && solid.aabb.max.x >= me.pos.x - me.half_extent.x
                    && solid.aabb.min.y >= feet - me.half_extent.y
            })
            .min_by(|a, b| (a.aabb.min.y - feet).total_cmp(&(b.aabb.min.y - feet)))
            .map(|solid| solid.aabb)
    }

    pub fn supporting_floor(&self) -> Option<ae::Aabb> {
        let me = &self.self_view;
        let feet = me.pos.y + me.half_extent.y;
        let support = self
            .terrain
            .iter()
            .filter(|solid| Self::is_standable(solid.kind))
            .filter(|solid| {
                // the body's FOOTPRINT, not its centre. This compared
                // `me.pos.x` against the solid's span, so a body standing on the
                // very lip of a platform — centre a few px past the edge, feet
                // still on it, `on_ground` still true — matched NO solid, and
                // `floor_ahead` answered `None`: *"I cannot see a floor"*. Every
                // ledge question in the brain reads through here, so the fighter
                // went blind about the edge at exactly the position where the
                // edge is the only thing that matters.
                //
                //     x=496 ... floor_edge=Some(34.0)   <- sees the lip
                //     x=537 ... floor_edge=None         <- ON the lip, blind
                //
                // The platform ends at x=530. Being past the edge is not the
                // absence of an edge; it is a NEGATIVE distance to one, and
                // `floor_ahead` reports it as such now.
                solid.aabb.min.x <= me.pos.x + me.half_extent.x
                    && solid.aabb.max.x >= me.pos.x - me.half_extent.x
                    && solid.aabb.min.y >= feet - me.half_extent.y
                    && solid.aabb.min.y <= feet + me.half_extent.y * 2.0
            })
            .min_by(|a, b| {
                (a.aabb.min.y - feet)
                    .abs()
                    .total_cmp(&(b.aabb.min.y - feet).abs())
            })?;
        Some(support.aabb)
    }

    /// Is there anything below me to land on?
    ///
    /// The top of the highest solid under the body's footprint, or `None` when
    /// there is nothing under it at all. Unlike [`Self::supporting_floor`] this
    /// does not care how far below: a body at the top of a jump is over its
    /// platform, and a body that has walked off the lip is over nothing, and
    /// those are different situations no matter what height either is at.
    ///
    /// it exists because "offstage" was a question about the ROOM.
    /// `StageView` is the room box, so on a platform stage a fighter that walked
    /// off the lip was still *inside the stage* for another hundred pixels of
    /// falling — L1 kept classifying `Neutral`, kept offering `Retreat`, and the
    /// verb that means "get back" was not on the list until the body had left
    /// the room. Having somewhere to land is the question recovery is actually
    /// about.
    pub fn ground_below(&self) -> Option<f32> {
        let me = &self.self_view;
        let feet = me.pos.y + me.half_extent.y;
        self.terrain
            .iter()
            .filter(|solid| Self::is_standable(solid.kind))
            .filter(|solid| {
                solid.aabb.min.x <= me.pos.x + me.half_extent.x
                    && solid.aabb.max.x >= me.pos.x - me.half_extent.x
                    // Below the feet, in the gravity sense this view is written
                    // in (+y down). A solid the body is already inside counts:
                    // it is still something to stand on.
                    && solid.aabb.max.y >= feet
            })
            .map(|solid| solid.aabb.min.y)
            .min_by(f32::total_cmp)
    }

    /// The nearer of the two floor edges, or `None` when there is no floor.
    pub fn floor_edge_distance(&self) -> Option<f32> {
        match (self.floor_ahead(1.0), self.floor_ahead(-1.0)) {
            (Some(right), Some(left)) => Some(right.min(left)),
            _ => None,
        }
    }
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
    /// `BTreeMap`, not `HashMap` (ADR 0023). [`WorldMemory::last_known_hostile`]
    /// takes the `max_by` confidence over these, and two hostiles both in view are
    /// both at confidence `1.0` — so the tie is broken by iteration order. Under
    /// `RandomState` that is the process seed, and the enemy chases a different
    /// player on every run of the same binary on the same inputs.
    actors: std::collections::BTreeMap<String, RememberedActor>,
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
        // ⛔⛔ THIS MEMBERSHIP TEST WAS A LINEAR SCAN OF THE WHOLE VIEW, PER
        // REMEMBERED ACTOR. `view.actors.iter().any(|a| &a.id == id)` is
        // O(remembered x seen) with a `String` comparison inside, and both terms
        // are the crowd size: at 113 perceived peers that is ~12,800 string
        // compares per actor per tick, ~1.6 MILLION across 130 actors. Measured
        // 2026-09-01 with the tactical extent widened to simulate a crowded
        // room, `WorldMemory::update` was the single largest symbol in the whole
        // profile at 12.89%, with the `BTreeMap` work beneath it at 4.09%.
        //
        // ⭐ SORTED BORROWED KEYS, NOT A HASH SET. The ids are borrowed from the
        // view, so this clones no `String`s; and it is sorted rather than hashed
        // because ADR 0023 keeps this type on `BTreeMap` for determinism, and
        // reaching for a `HashSet` here would put process-seeded iteration back
        // into the one function that was deliberately kept free of it — even
        // though only membership is asked. A sorted slice cannot regress that way.
        let mut seen: Vec<&str> = view.actors.iter().map(|a| a.id.as_str()).collect();
        seen.sort_unstable();

        // Decay the unseen. (Iterating then inserting below is two disjoint
        // phases, so there's no borrow conflict.)
        for (id, mem) in self.actors.iter_mut() {
            if seen.binary_search(&id.as_str()).is_err() {
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

    /// The most-confident remembered hostile — the target a brain pursues when
    /// none is currently in view (invariant I6: "move towards the last known
    /// position of the player to look for them").
    /// The most-confidently-remembered hostile. Ties break on the greatest actor id
    /// — `max_by` keeps the last maximum, and a `BTreeMap` walks ids in order — so
    /// two foes at equal confidence resolve the same way on every run. That is not a
    /// tiebreak anyone would *choose*; it is a tiebreak that EXISTS, which is the
    /// whole requirement (ADR 0023).
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

    /// Every remembered actor, in id order. For deterministic GGRS checksums, and
    /// deterministic by construction now that the map is a `BTreeMap`.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &RememberedActor)> {
        self.actors.iter().map(|(id, m)| (id.as_str(), m))
    }

    /// Rebuild from a snapshot blob. The only way to construct a `WorldMemory` other
    /// than by [`WorldMemory::update`], and named for its one caller.
    pub fn from_snapshot(entries: impl IntoIterator<Item = (String, RememberedActor)>) -> Self {
        Self {
            actors: entries.into_iter().collect(),
        }
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

/// A view a brain is allowed to read. The no-cheat contract, made a type.
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

/// The perception delay-buffer — the no-cheat contract's reaction latency,
/// made structural (`docs/planning/engine/fighter-brain.md` §1.3, §5).
///
/// A brain that reads the live view reacts in zero milliseconds, which no human
/// does. `FighterBrainProfile.reaction_ms` says how late the brain should see the
/// world; this is the thing that makes it so. Wrap the ONE view read: the
/// gameplay layer `observe`s each tick's fresh view, and every L1/L2/L3 code path
/// reads `perceive()`.
///
/// Warm-up is deliberately stale, never fresh. Before the buffer fills, it returns the oldest
/// view it holds — so a brain spawned mid-fight reacts *more* slowly than its profile for a few
/// ticks, never faster.
///
/// Shipped difficulty rows never use it — §1.3: *"Level 9 = small numbers, never zero."*
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
mod tests;
