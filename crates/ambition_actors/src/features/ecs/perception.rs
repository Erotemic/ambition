//! Body-generic [`WorldView`] builder — the gameplay-layer half of the world-out
//! port (architecture roadmap S4).
//!
//! [`ambition_characters::perception`] owns the headless, controller-neutral
//! *value* ([`WorldView`] / [`WorldMemory`]) and its pure tactical queries; this
//! module owns the **construction** — reading real solids, other actor bodies, and
//! live projectiles out of the gameplay world and packing them into the view.
//!
//! ### Body-generic by construction (guardrail #1)
//!
//! [`build_world_view`] takes a [`PerceptionBody`] — the minimal description of
//! **any** body (player-robot, Perfect Cell-ular Automaton, NPC, boss) — never an
//! `CharacterBrain`-keyed or `"player"`-keyed input. Perception "for the player" is a
//! brain driving the player-robot body through this same function, so when S5/S6
//! land there is no enemy-only path to undo. Hostility is resolved **relationally**
//! against [`FactionRelations`] (the S3e seam), not by a player-vs-enemy branch.
//!
//! The peer / projectile lists are pre-collected before the per-body loop (the
//! same shape the crowding pass uses), so a body perceives the others without a
//! second mutable borrow of the actor query.

use ae::AabbExt;
use ambition_engine_core as ae;

use ambition_characters::actor::ActorFaction;
use ambition_characters::perception::{
    BodyPhase, PerceivedActor, PerceivedPortal, PerceivedProjectile, PerceivedSolid, SelfView,
    SolidKind, StageView, Viewport, WorldView,
};

use crate::combat::targeting::FactionRelations;

/// Default viewport half-extent (world px) — the AI analogue of the human's
/// screen. Generous so a body perceives approaching threats with room to react;
/// a per-body override rides in [`Perception::Sighted`] for a character that wants
/// keener or duller senses.
pub const DEFAULT_VIEWPORT_HALF: ae::Vec2 = ae::Vec2::new(480.0, 320.0);

/// A body's PERCEPTION policy — HOW it learns where its foe is. Perception is
/// UNIVERSAL: targeting always flows through this typed, per-body policy, never
/// through an implicit "did the perception resource exist this run?" fallback. A
/// body without the component reads as the default, [`Perception::Omniscient`], so
/// omniscience is a deliberate BASIC mode, not a degraded path.
///
/// The two modes are a spectrum from primal to refined:
/// - [`Omniscient`](Self::Omniscient) — the BASIC perception: the body simply KNOWS
///   the nearest hostile ANYWHERE (the global [`ActorTarget`](crate::combat::components::ActorTarget)
///   `select_actor_targets` maintains). No viewport, no line-of-sight, no forgetting.
///   A boss has this — it is relentless, you cannot juke it — and it is what any body
///   defaults to before it is given senses, so a fixture that wires up no perception
///   still targets correctly through the same `ActorTarget` every body carries.
/// - [`Sighted`](Self::Sighted) — the body perceives only within `viewport_half` and
///   pursues a foe that left it from [`PerceptionMemory`] (invariant I6). Ordinary
///   actors have this: they can lose sight of you, be juked, and give up. This is the
///   world-out [`WorldView`] port ([`build_world_view`]).
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub enum Perception {
    /// Knows the nearest hostile anywhere (reads the global `ActorTarget`).
    Omniscient,
    /// Sees within `viewport_half`; blind beyond it (+ memory pursuit).
    Sighted { viewport_half: ae::Vec2 },
}

impl Default for Perception {
    /// Omniscience is the basic perception — the mode a body has until it is granted
    /// bounded senses.
    fn default() -> Self {
        Perception::Omniscient
    }
}

/// The viewing body, described generically (any faction). Built for the
/// player-robot body exactly as for an enemy (guardrail #1) — this struct names
/// no character type.
pub struct PerceptionBody {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub facing: f32,
    pub half_extent: ae::Vec2,
    pub faction: ActorFaction,
    /// Local gravity direction (unit) — carried so a brain can reason frame-local.
    pub gravity_down: ae::Vec2,
    pub on_ground: bool,
    pub aerial: bool,
    pub alive: bool,
    pub can_fire: bool,
    pub can_blink: bool,
    pub can_dash: bool,
    pub can_shield: bool,
    /// What this body is doing, and how long is left of it — the no-cheat
    /// contract's "move phase / animation state" (fighter-brain.md §1).
    pub phase: BodyPhase,
    pub phase_remaining: f32,
    pub invulnerable: bool,
    /// The smash-percent axis (CM1) and its denominator.
    pub damage_taken: i32,
    pub health_max: i32,
    /// This viewer's per-entity GRUDGE, if any (`ActorAggression.grudge`). A grudge
    /// makes ONE exact body a foe even when it shares the viewer's faction — the
    /// mechanism behind two same-faction NPCs dueling. Carried here so
    /// `hostile_to_self` matches `select_actor_targets`' foe set (faction-hostile OR
    /// grudge), not faction alone; without it a grudge-duelist would perceive no
    /// target. `None` for a body with no personal feud.
    pub grudge: Option<bevy::prelude::Entity>,
}

/// A candidate other-body the viewer may perceive. Pre-collected (id +
/// kinematics + faction + body-state) before the per-body loop.
#[derive(Clone)]
pub struct PerceptionPeer {
    /// The source body's `Entity` — so the viewer can excludes itself AND resolve a
    /// per-entity grudge against this exact body (grudge is keyed by `Entity`, not id).
    pub entity: bevy::prelude::Entity,
    pub id: String,
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub facing: f32,
    pub half_extent: ae::Vec2,
    pub faction: ActorFaction,
    pub alive: bool,
    pub on_ground: bool,
    pub shield_raised: bool,
    /// Move phase + its remaining seconds, as a watcher reads it off the
    /// animation. This is what lets a brain punish a whiffed swing.
    pub phase: BodyPhase,
    pub phase_remaining: f32,
    pub invulnerable: bool,
    /// The smash-percent axis (CM1) and its denominator — kill potential.
    pub damage_taken: i32,
    pub health_max: i32,
}

/// A live projectile the viewer may perceive. `faction` is the **firer's**
/// faction; the builder resolves whether it threatens the viewer relationally.
pub struct PerceptionProjectile {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub damage: i32,
    pub faction: ActorFaction,
}

/// A portal aperture the viewer may perceive. `channel_key` is the stable pair
/// identity the builder derives from the live `PortalChannel`, so the perceived
/// value can find the linked exit without depending on the portal crate.
pub struct PerceptionPortal {
    pub pos: ae::Vec2,
    pub normal: ae::Vec2,
    pub half_extent: ae::Vec2,
    pub channel_key: u64,
}

/// Per-frame snapshot of EVERY live body's peer data, refreshed by
/// [`collect_perception_peers`] BEFORE the per-body view build so a body perceives
/// the others without a second (mutable-aliasing) borrow of the actor query. Each
/// [`PerceptionPeer`] carries its source `Entity` so a viewer excludes ITSELF (and
/// resolves grudges) when building its own view.
#[derive(bevy::prelude::Resource, Default)]
pub struct PerceptionPeers(pub Vec<PerceptionPeer>);

/// Collect the peer snapshot from every live body — player, actor, AND boss all
/// carry [`BodyKinematics`], so ONE query spans them (guardrail #1: no per-type
/// path). §A7: this POPULATES the peers channel `build_world_view` reads, so
/// `WorldView`'s `nearest_hostile` / `hostiles` / `incoming_threats` are live — and
/// non-boss brains now TARGET through it (they perceive their foe, not the omniscient
/// `ActorTarget`). Each peer carries its source `Entity` so a viewer excludes ITSELF
/// and resolves a per-entity grudge. `on_ground` / `shield_raised` are left `false`
/// for now (no consumer reads them; wire them when a brain needs them).
/// Read a body's **move phase** off its live combat state — the one place the
/// perception vocabulary ([`BodyPhase`]) is mapped from the sim's.
///
/// The `ambition_combat` swing clock is the authority while a swing is in flight;
/// `BodyCombat`'s hitstun timer wins over it, because a body knocked out of its
/// own attack is reeling, not attacking. Shield is last: you cannot guard while
/// reeling or swinging.
///
/// Returns `(phase, seconds_remaining_in_phase)`. The remaining clock is `0.0`
/// where the sim keeps none (recovery has no dedicated timer today — CM7's
/// frame-data table is what will give it one).
pub fn body_phase(
    combat: Option<&ambition_characters::actor::BodyCombat>,
    melee: Option<&ambition_combat::components::BodyMelee>,
    shield: Option<&ae::BodyShieldState>,
) -> (BodyPhase, f32) {
    if let Some(c) = combat {
        if c.hitstun_timer > 0.0 || c.recoil_lock_timer > 0.0 {
            return (BodyPhase::Hitstun, c.hitstun_timer.max(c.recoil_lock_timer));
        }
    }
    if let Some(m) = melee {
        match m.phase() {
            Some(ambition_combat::AttackPhase::Startup) => {
                return (BodyPhase::AttackStartup, m.windup_remaining())
            }
            Some(ambition_combat::AttackPhase::Active) => {
                return (BodyPhase::AttackActive, m.active_remaining())
            }
            Some(ambition_combat::AttackPhase::Recovery) => {
                return (BodyPhase::AttackRecovery, 0.0)
            }
            None => {}
        }
    }
    if shield.is_some_and(|s| s.active) {
        return (BodyPhase::Shielding, 0.0);
    }
    (BodyPhase::Neutral, 0.0)
}

/// True while the body is in post-hit i-frames — visible, because it flashes.
fn body_invulnerable(combat: Option<&ambition_characters::actor::BodyCombat>) -> bool {
    combat.is_some_and(|c| c.damage_invuln_timer > 0.0)
}

pub fn collect_perception_peers(
    mut peers: bevy::prelude::ResMut<PerceptionPeers>,
    bodies: bevy::prelude::Query<(
        bevy::prelude::Entity,
        Option<&crate::features::FeatureId>,
        &crate::actor::BodyKinematics,
        &ambition_characters::actor::BodyHealth,
        &ActorFaction,
        // FB1: `on_ground` / `shield_raised` used to be hardcoded `false` here —
        // the view LIED about every peer. A brain that read them (and FB1's L1
        // classifier will) would think nobody was ever grounded or guarding.
        Option<&ae::BodyGroundState>,
        Option<&ae::BodyShieldState>,
        Option<&ambition_characters::actor::BodyCombat>,
        Option<&ambition_combat::components::BodyMelee>,
    )>,
) {
    peers.0.clear();
    for (entity, id, kin, health, faction, ground, shield, combat, melee) in &bodies {
        let (phase, phase_remaining) = body_phase(combat, melee, shield);
        peers.0.push(PerceptionPeer {
            entity,
            id: id
                .map(|f| f.as_str().to_string())
                .unwrap_or_else(|| format!("e{}", entity.index())),
            pos: kin.pos,
            vel: kin.vel,
            facing: kin.facing,
            // FB1: this was `kin.size` — the FULL body size passed as a HALF
            // extent, so every peer read as twice its real box. `BodyKinematics`
            // keeps full size (`aabb()` halves it); the view's contract is halves.
            half_extent: kin.size * 0.5,
            faction: *faction,
            alive: health.alive(),
            on_ground: ground.is_some_and(|g| g.on_ground),
            shield_raised: shield.is_some_and(|s| s.active),
            phase,
            phase_remaining,
            invulnerable: body_invulnerable(combat),
            damage_taken: health.damage_taken(),
            health_max: health.max(),
        });
    }
}

/// Per-frame snapshot of every live projectile, refreshed by
/// [`collect_perception_projectiles`] before the per-body view build (same shape as
/// [`PerceptionPeers`]). No source `Entity` is needed — a projectile is never its own
/// viewer.
#[derive(bevy::prelude::Resource, Default)]
pub struct PerceptionProjectiles(pub Vec<PerceptionProjectile>);

/// Collect the projectile snapshot from BOTH live pools (§A7 projectiles slice). The
/// two pools carry faction DIFFERENTLY (only projectiles carry `ProjectileGameplay`,
/// so it selects them): an `enemy_projectile` reads its own `ActorFaction` component;
/// a `projectile` `LiveProjectile` has none (the unified stepper attributes via its
/// owner), so it is snapshotted as `Player` — the live pool is the player/charge path,
/// and mixed-faction reflected shots are a refinement for when a dodging brain actually
/// reads `incoming_threats` (no consumer today, so this is additive + behavior-neutral).
pub fn collect_perception_projectiles(
    mut out: bevy::prelude::ResMut<PerceptionProjectiles>,
    enemy_pool: bevy::prelude::Query<
        (
            &crate::actor::BodyKinematics,
            &crate::projectile::ProjectileGameplay,
            &ActorFaction,
        ),
        bevy::prelude::With<crate::enemy_projectile::EnemyProjectile>,
    >,
    live_pool: bevy::prelude::Query<
        (
            &crate::actor::BodyKinematics,
            &crate::projectile::ProjectileGameplay,
        ),
        bevy::prelude::With<crate::projectile::LiveProjectile>,
    >,
) {
    out.0.clear();
    for (kin, game, faction) in &enemy_pool {
        out.0.push(PerceptionProjectile {
            pos: kin.pos,
            vel: kin.vel,
            damage: game.damage,
            faction: *faction,
        });
    }
    for (kin, game) in &live_pool {
        out.0.push(PerceptionProjectile {
            pos: kin.pos,
            vel: kin.vel,
            damage: game.damage,
            faction: ActorFaction::Player,
        });
    }
}

/// Per-body persistent world-belief (invariant I6): a brained body's [`WorldMemory`]
/// — the last-known positions of foes that have left its viewport, with a decaying
/// confidence — so a brain can PURSUE a target that went off-screen instead of
/// forgetting it the instant it leaves the frame. Updated each tick by
/// [`crate::features::ecs::actors::tick_actor_brains`] from the body's fresh
/// [`WorldView`], then read for the perceived target when nothing hostile is in view.
///
/// A component (not a resource) so it lives + dies with the body — no manual pruning
/// of despawned entities. Attached to every non-boss brained actor by
/// [`ensure_perception`].
#[derive(bevy::prelude::Component, Default)]
pub struct PerceptionMemory(pub ambition_characters::perception::WorldMemory);

/// Grant SIGHTED perception to every non-boss brained actor that lacks it: a
/// [`Perception::Sighted`] policy (bounded viewport + memory pursuit) AND the
/// [`PerceptionMemory`] belief store it pursues from. Runs before the brain tick.
/// Matches `tick_actor_brains`' own body set (brained, non-player, non-boss).
///
/// This is where ordinary actors OPT IN to sighted perception — they can be juked,
/// lose sight of a foe, and give up. Everything WITHOUT a [`Perception`] component
/// defaults to [`Perception::Omniscient`] (the basic mode), which is documented
/// POLICY, not a parallel-system carve-out (§A7):
/// - the **player** brain steers from controller input and never perceive-targets;
/// - a **boss** is relentless — it knows where you are in its arena (omniscience is
///   its perception, the `ActorTarget` read every body carries), so it needs no
///   viewport or belief store. A boss that wanted bounded, juke-able senses would drop
///   this `Without<BossConfig>` exclusion and be granted `Sighted` + memory here;
///   today none do.
///
/// Because the missing component reads as `Omniscient`, there is NO "perception
/// resource missing" fallback anywhere: the target derivation branches on this typed
/// policy, and a fixture that wires up no perception simply gets the basic mode.
pub fn ensure_perception(
    mut commands: bevy::prelude::Commands,
    bodies: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<ambition_characters::brain::Brain>,
            bevy::prelude::With<crate::features::FeatureSimEntity>,
            bevy::prelude::Without<crate::actor::PlayerEntity>,
            bevy::prelude::Without<crate::features::ecs::boss_clusters::BossConfig>,
            // Missing memory ⟺ missing perception (both attached together below), so
            // this one gate nets bodies that lack either.
            bevy::prelude::Without<PerceptionMemory>,
        ),
    >,
) {
    for entity in &bodies {
        commands.entity(entity).insert((
            Perception::Sighted {
                viewport_half: DEFAULT_VIEWPORT_HALF,
            },
            PerceptionMemory::default(),
        ));
    }
}

/// Build the headless [`WorldView`] for `body` from real world geometry, the
/// pre-collected peers/projectiles, and the relational faction matrix.
///
/// The terrain carried into the view is clipped from the **same** `world.blocks`
/// the body physically collides against (caller passes the derived collision
/// world — moving platforms + ECS overlays already folded in), so the view's
/// line-of-fire / reachability queries reuse the real geometry, never a parallel
/// sensor.
#[allow(clippy::too_many_arguments)]
pub fn build_world_view(
    body: &PerceptionBody,
    peers: &[PerceptionPeer],
    projectiles: &[PerceptionProjectile],
    portals: &[PerceptionPortal],
    world: &ae::World,
    relations: &FactionRelations,
    viewport_half: ae::Vec2,
    sim_time: f32,
) -> WorldView {
    let viewport = Viewport::around(body.pos, viewport_half);

    let self_view = SelfView {
        pos: body.pos,
        vel: body.vel,
        facing: body.facing,
        half_extent: body.half_extent,
        gravity_down: body.gravity_down,
        on_ground: body.on_ground,
        aerial: body.aerial,
        alive: body.alive,
        faction: body.faction,
        can_fire: body.can_fire,
        can_blink: body.can_blink,
        can_dash: body.can_dash,
        can_shield: body.can_shield,
        phase: body.phase,
        phase_remaining: body.phase_remaining,
        invulnerable: body.invulnerable,
        damage_taken: body.damage_taken,
        health_max: body.health_max,
    };

    // The stage is NOT viewport-clipped: a fighter can see the blastzones. It is
    // the same envelope CC3's invariant 3 polices, so "offstage" here and "out of
    // bounds" there are the same predicate.
    let stage = StageView {
        bounds: ae::aabb_from_min_size(ae::Vec2::ZERO, world.size),
    };

    let actors = peers
        .iter()
        .filter(|p| viewport.contains(p.pos))
        .map(|p| PerceivedActor {
            id: p.id.clone(),
            pos: p.pos,
            vel: p.vel,
            facing: p.facing,
            half_extent: p.half_extent,
            faction: p.faction,
            // A foe by faction (`FactionRelations`) OR by a personal grudge against
            // this exact body — the SAME two-part rule `select_actor_targets` uses, so
            // `nearest_hostile` sees a same-faction grudge-duel opponent (which faction
            // hostility alone would miss).
            hostile_to_self: relations.is_hostile(body.faction, p.faction)
                || body.grudge == Some(p.entity),
            alive: p.alive,
            on_ground: p.on_ground,
            shield_raised: p.shield_raised,
            phase: p.phase,
            phase_remaining: p.phase_remaining,
            invulnerable: p.invulnerable,
            damage_taken: p.damage_taken,
            health_max: p.health_max,
        })
        .collect();

    let projectiles = projectiles
        .iter()
        .filter(|pr| viewport.contains(pr.pos))
        .map(|pr| PerceivedProjectile {
            pos: pr.pos,
            vel: pr.vel,
            damage: pr.damage,
            // A projectile threatens me iff its firer's faction is hostile to mine.
            hostile_to_self: relations.is_hostile(pr.faction, body.faction),
        })
        .collect();

    let viewport_aabb = viewport.as_aabb();
    let terrain = world
        .blocks
        .iter()
        .filter_map(|b| perceived_solid_kind(b.kind).map(|kind| (b, kind)))
        .filter(|(b, _)| b.aabb.strict_intersects(viewport_aabb))
        .map(|(b, kind)| PerceivedSolid { aabb: b.aabb, kind })
        .collect();

    let portals = portals
        .iter()
        .filter(|p| viewport.contains(p.pos))
        .map(|p| PerceivedPortal {
            pos: p.pos,
            normal: p.normal,
            half_extent: p.half_extent,
            channel_key: p.channel_key,
        })
        .collect();

    WorldView {
        self_view,
        viewport,
        stage,
        actors,
        projectiles,
        terrain,
        portals,
        sim_time,
    }
}

/// Distill an engine `BlockKind` to the perception `SolidKind`, or `None` for
/// blocks perception doesn't model as terrain (pogo / rebound surfaces — they
/// don't block sight or a straight path).
fn perceived_solid_kind(kind: ae::BlockKind) -> Option<SolidKind> {
    match kind {
        ae::BlockKind::Solid => Some(SolidKind::Solid),
        ae::BlockKind::BlinkWall { .. } => Some(SolidKind::BlinkWall),
        ae::BlockKind::OneWay => Some(SolidKind::OneWay),
        ae::BlockKind::Hazard => Some(SolidKind::Hazard),
        ae::BlockKind::PogoOrb | ae::BlockKind::Rebound { .. } => None,
    }
}

#[cfg(test)]
mod tests;
