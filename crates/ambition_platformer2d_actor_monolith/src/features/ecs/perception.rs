//! Gameplay construction of controller-neutral [`WorldView`] values.
//!
//! `ambition_characters::perception` owns the pure view/memory types and tactical
//! queries; this module collects live solids, bodies, and projectiles. Perception
//! is body-generic and hostility is relational through `FactionRelations`, not a
//! player/enemy branch. Peer/projectile snapshots are collected before mutable
//! per-body updates to avoid conflicting world borrows.

use ae::AabbExt;
use ambition_platformer2d_core as ae;

use ambition_characters::actor::ActorFaction;
use ambition_characters::perception::{
    BodyPhase, PerceivedActor, PerceivedPortal, PerceivedProjectile, PerceivedSolid, SelfView,
    SolidKind, StageView, Viewport, WorldView,
};

use ambition_combat::targeting::FactionRelations;

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
///   the nearest hostile ANYWHERE (the global [`ActorTarget`](ambition_combat::components::ActorTarget)
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

impl Perception {
    /// How far this body's terrain, projectile and line-of-fire queries reach.
    ///
    /// An omniscient body still gets a tactical view — it has to, or it could
    /// not aim — and it gets it at the same default extent an ordinary actor
    /// sees at. What omniscience buys is [`Self::knows_bodies_anywhere`].
    pub fn tactical_extent(self) -> ae::Vec2 {
        match self {
            Perception::Omniscient => DEFAULT_VIEWPORT_HALF,
            Perception::Sighted { viewport_half } => viewport_half,
        }
    }

    /// Does this body know where every hostile is, regardless of distance?
    ///
    /// ⛔ The BOUNDARY between the two modes, and the reason it is a method
    /// rather than a `matches!` at each site: a brain that reads `view.actors`
    /// and a target derivation that reads `ActorTarget` must agree about which
    /// bodies this one can see, and they disagreed for as long as this was
    /// decided in two places.
    pub fn knows_bodies_anywhere(self) -> bool {
        matches!(self, Perception::Omniscient)
    }
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
    /// What this body's shared burst button would DO if pressed this tick —
    /// see [`ambition_characters::perception::SelfView::burst`].
    pub burst: ae::BurstManeuver,
    pub can_shield: bool,
    /// Mid-air jumps left (`jump.air_jumps_available`). The recovery budget.
    pub air_jumps_left: u8,
    /// What this body is doing, and how long is left of it — the no-cheat
    /// contract's "move phase / animation state" (fighter-brain.md §1).
    pub phase: BodyPhase,
    pub phase_remaining: f32,
    pub invulnerable: bool,
    /// Falling out of a launch — the next landing is a knockdown unless it is
    /// teched. Read off this body's OWN peer row, like `phase`, so it cannot
    /// know itself more precisely than its opponents know it.
    pub tumbling: bool,
    /// The smash-percent axis (CM1) and its denominator.
    pub damage_taken: i32,
    pub health_max: i32,
    /// The capture relationship, as a fact rather than a component. Read off
    /// `CapturedBy` by the system that holds the queries and handed to the brain
    /// — the same shape `grudge` beside it takes, and for the same reason: a
    /// pure decision must not reach into the ECS to ask.
    pub captured: bool,
    /// How long it has been held, in scaled seconds. `0.0` when free.
    pub captured_for: f32,
    pub holding_captive: bool,
    pub pummels_landed: u8,
    /// This viewer's per-entity GRUDGE, if any (`ActorAggression.grudge`). A grudge
    /// makes ONE exact body a foe even when it shares the viewer's faction — the
    /// mechanism behind two same-faction NPCs dueling. Carried here so
    /// `hostile_to_self` matches `select_actor_targets`' foe set (faction-hostile OR
    /// grudge), not faction alone; without it a grudge-duelist would perceive no
    /// target. `None` for a body with no personal feud.
    pub grudge: Option<bevy::prelude::Entity>,
    /// This viewer's match TEAM, when it is seated in one.
    ///
    /// perception resolved hostility from FACTION alone, and the damage rule
    /// stopped doing that when `damage_lands_between` gave teams precedence. In
    /// a free-for-all — every seat its own team, which is what the smash demo
    /// authors — `faction_for` alternates Player/Enemy by seat index, so seats 0
    /// and 2 share a faction on different teams: they could damage each other
    /// and could not SEE each other. A brain with no perceived foe stands still,
    /// and every component that would explain the stillness is present and
    /// correct.
    pub team: Option<ambition_combat::targeting::MatchTeam>,
}

/// A candidate other-body the viewer may perceive. Pre-collected (id +
/// kinematics + faction + body-state) before the per-body loop.
#[derive(Clone)]
pub struct PerceptionPeer {
    /// This peer's match team, when it is seated in one. Paired with
    /// [`PerceptionBody::team`]; both are needed or the relation is undecidable.
    pub team: Option<ambition_combat::targeting::MatchTeam>,
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
    /// Falling out of a launch, so the next landing is a knockdown unless it is
    /// teched. Visible from across the stage — a tumbling body is tumbling in
    /// plain sight — which is why it rides the peer row rather than being a
    /// private fact a body knows about itself.
    pub tumbling: bool,
    /// Hanging on a ledge, in plain sight, for the same reason — see
    /// [`ambition_characters::perception::PerceivedActor::ledge_hanging`].
    pub ledge_hanging: bool,
    /// The smash-percent axis (CM1) and its denominator — kill potential.
    pub damage_taken: i32,
    pub health_max: i32,
}

/// A live projectile the viewer may perceive. `faction` + `team` are the frozen
/// firing side when the projectile has one. No faction means an intentionally
/// ownerless/environmental shot, which the builder treats as indiscriminate.
/// Team precedence matches projectile damage whenever both sides are seated.
pub struct PerceptionProjectile {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub damage: i32,
    pub faction: Option<ActorFaction>,
    /// Frozen match team from the firing body, when the shot belongs to a
    /// seated combatant. `None` for unseated and ownerless shots.
    pub team: Option<ambition_combat::targeting::MatchTeam>,
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
/// Read a body's move phase off its live combat state — the one place the
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
                return (BodyPhase::AttackStartup, m.windup_remaining());
            }
            Some(ambition_combat::AttackPhase::Active) => {
                return (BodyPhase::AttackActive, m.active_remaining());
            }
            Some(ambition_combat::AttackPhase::Recovery) => {
                return (BodyPhase::AttackRecovery, 0.0);
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
        Option<&ae::BodyGroundState>,
        Option<&ae::BodyShieldState>,
        Option<&ambition_characters::actor::BodyCombat>,
        Option<&ambition_combat::components::BodyMelee>,
        // The match team, so hostility can follow the same precedence the DAMAGE
        // rule follows. `None` for anything not seated in a match, which is most
        // of the world.
        Option<&ambition_combat::targeting::MatchTeam>,
        // The published motion facts, for the two a watcher can see: tumbling,
        // and hanging on a ledge.
        Option<&ae::BodyMotionFacts>,
    )>,
) {
    peers.0.clear();
    for (entity, id, kin, health, faction, ground, shield, combat, melee, team, facts) in &bodies {
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
            tumbling: facts.is_some_and(|f| f.tumbling),
            // HANGING, not climbing: a body already pulling itself up has left
            // the edge, which is the same distinction `resolve_ledge_trumps`
            // draws and for the same reason — it is no longer contesting one.
            ledge_hanging: facts.is_some_and(|f| f.ledge.is_some_and(|ledge| !ledge.climbing)),
            invulnerable: body_invulnerable(combat),
            damage_taken: health.damage_taken(),
            health_max: health.max(),
            team: team.cloned(),
        });
    }
}

/// Per-frame snapshot of every live projectile, refreshed by
/// [`collect_perception_projectiles`] before the per-body view build (same shape as
/// [`PerceptionPeers`]). No source `Entity` is needed — a projectile is never its own
/// viewer.
#[derive(bevy::prelude::Resource, Default)]
pub struct PerceptionProjectiles(pub Vec<PerceptionProjectile>);

/// Collect one snapshot row for every live projectile.
///
/// `LiveProjectile` is occurrence identity; `ProjectileAllegiance` is the frozen
/// combat side. The old player/enemy spawn-family markers are deliberately not
/// part of perception: an open-visual projectile can be friendly, and a named
/// projectile can be hostile after a reflect. An unstamped environmental shot
/// carries `None`, matching the stepper's indiscriminate ownerless semantics.
pub fn collect_perception_projectiles(
    mut out: bevy::prelude::ResMut<PerceptionProjectiles>,
    live: bevy::prelude::Query<
        (
            &crate::actor::BodyKinematics,
            &ambition_projectiles::ProjectileGameplay,
            Option<&crate::projectile::ProjectileAllegiance>,
        ),
        bevy::prelude::With<ambition_projectiles::LiveProjectile>,
    >,
) {
    out.0.clear();
    for (kin, game, allegiance) in &live {
        out.0.push(PerceptionProjectile {
            pos: kin.pos,
            vel: kin.vel,
            damage: game.damage,
            faction: allegiance.map(|side| side.faction),
            team: allegiance.and_then(|side| side.team.clone()),
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
///
/// The belief is preserved across the possession rather than decayed by it, and re-enters use the
/// moment an AI brain returns to the body.
///
/// This is where ordinary actors OPT IN to sighted perception — they can be juked,
/// lose sight of a foe, and give up. Everything WITHOUT a [`Perception`] component
/// defaults to [`Perception::Omniscient`] (the basic mode), which is documented
/// POLICY, not a parallel-system carve-out (§A7):
/// - the player brain steers from controller input and never perceive-targets;
/// - a boss is relentless — it knows where you are in its arena (omniscience is
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
            bevy::prelude::Without<ambition_boss_encounter::BossConfig>,
            // ⭐ A FIGHTER SEATED IN A MATCH IS NOT AN EXPLORATION ACTOR, and
            // bounded senses are an exploration mechanic: being juked, losing a
            // foe and giving up are things a room full of enemies is FOR. A
            // platform fighter has none of them — both fighters are on screen
            // for the whole match and each always knows where the other is —
            // so a match fighter keeps the basic `Omniscient` mode.
            //
            // ⛔ THIS EXCLUSION IS LOAD-BEARING, and the measurement that put it
            // here is worth more than the rule: `DEFAULT_VIEWPORT_HALF.x` is
            // 480 and the smash platform is 480 wide, so two fighters that
            // drifted apart went permanently blind to each other while still
            // standing on the same stage. Over a sixteen-character mirror
            // sweep, six characters' median gap sat between 491 and 515 px —
            // with NOTHING between 295 and 491 — and three of them threw four
            // moves in a minute and dealt no damage at all.
            //
            // Safe because seating is ATOMIC: `realize_seat` and the
            // `MatchSeat` insert share one command flush, so a fighter is never
            // observable without its seat and can never be granted bounded
            // senses in the window before it.
            bevy::prelude::Without<crate::character_runtime::MatchSeat>,
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
/// The terrain carried into the view is clipped from the same `world.blocks`
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
    perception: Perception,
    sim_time: f32,
) -> WorldView {
    // THE TACTICAL EXTENT — how far this body's line-of-fire, terrain and
    // projectile queries reach. Both policies have one; they differ in whether
    // the ACTOR channel below respects it.
    let viewport = Viewport::around(body.pos, perception.tactical_extent());

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
        burst: body.burst,
        can_shield: body.can_shield,
        air_jumps_left: body.air_jumps_left,
        phase: body.phase,
        phase_remaining: body.phase_remaining,
        invulnerable: body.invulnerable,
        tumbling: body.tumbling,
        damage_taken: body.damage_taken,
        health_max: body.health_max,
        captured: body.captured,
        captured_for: body.captured_for,
        holding_captive: body.holding_captive,
        pummels_landed: body.pummels_landed,
    };

    // The stage is NOT viewport-clipped: a fighter can see the blastzones. It is
    // the same envelope CC3's invariant 3 polices, so "offstage" here and "out of
    // bounds" there are the same predicate.
    let stage = StageView {
        bounds: ae::aabb_from_min_size(ae::Vec2::ZERO, world.size),
    };

    // ⭐ THE ACTOR CHANNEL IS NOT CLIPPED FOR AN OMNISCIENT BODY, and this is
    // what `Perception::Omniscient` has always claimed to mean — *"the body
    // simply KNOWS the nearest hostile ANYWHERE"*. Until this line, it did not:
    // the view was built at `DEFAULT_VIEWPORT_HALF` whatever the policy said,
    // and only the `ActorTarget` derivation ignored the box. That was a safe
    // compromise for exactly as long as every brain reached its foe through
    // `ActorTarget` — and the fighter brain does not. It reads `view.actors`,
    // so an "omniscient" fighter was sighted at 480px and nothing said so.
    //
    // Terrain, projectiles and pickups keep the tactical extent under both
    // policies: omniscience is a claim about where BODIES are, not a licence to
    // fold a whole room's geometry into one tick's line-of-fire query.
    let actors = peers
        .iter()
        .filter(|p| perception.knows_bodies_anywhere() || viewport.contains(p.pos))
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
            // THE SAME PRECEDENCE `damage_lands_between` USES, through the
            // same `team_allows_damage` authority: when both bodies are seated,
            // the team relation decides and factions have nothing to say. A
            // grudge still overrides, exactly as it does for damage.
            hostile_to_self: match ambition_combat::targeting::team_allows_damage(
                body.team.as_ref(),
                p.team.as_ref(),
            ) {
                Some(allowed) => allowed || body.grudge == Some(p.entity),
                None => {
                    relations.is_hostile(body.faction, p.faction) || body.grudge == Some(p.entity)
                }
            },
            alive: p.alive,
            on_ground: p.on_ground,
            shield_raised: p.shield_raised,
            phase: p.phase,
            phase_remaining: p.phase_remaining,
            invulnerable: p.invulnerable,
            tumbling: p.tumbling,
            ledge_hanging: p.ledge_hanging,
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
            // A sided projectile threatens me iff its frozen firing side is
            // hostile to mine. An ownerless/environmental projectile is
            // indiscriminate, matching the damage stepper.
            hostile_to_self: match pr.faction {
                None => true,
                Some(faction) => match ambition_combat::targeting::team_allows_damage(
                    pr.team.as_ref(),
                    body.team.as_ref(),
                ) {
                    Some(allowed) => allowed,
                    None => relations.is_hostile(faction, body.faction),
                },
            },
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
        // a bonk-only block is not terrain to a brain. It blocks neither
        // sight nor a straight path — nothing can walk into it or stand on it —
        // so perceiving it as ground would route a patrol over a surface that
        // will not hold it.
        ae::BlockKind::BonkOnly | ae::BlockKind::PogoOrb | ae::BlockKind::Rebound { .. } => None,
    }
}

#[cfg(test)]
mod tests;

/// Project a live actor body into the perception input its own world view is
/// built from.
///
/// the projection belongs beside the type, not inside the tick. This was
/// sixty lines of struct literal in the middle of `tick_actor_brains`, between a
/// snapshot build and a brain call, which is what made "what does a body know
/// about itself" a question you answered by reading a decision loop. Every field
/// here is a read of one authority — the body's own clusters, its peer row, its
/// resolved frame — and the comments on them are the record of which authority
/// won each argument.
#[allow(clippy::too_many_arguments)]
pub(crate) fn perception_body_for(
    body: &super::actor_clusters::ActorClusterQueryDataReadOnlyItem<'_, '_>,
    faction: ActorFaction,
    gravity_down: ae::Vec2,
    action_set: Option<&ambition_characters::brain::ActionSet>,
    // This body's OWN row in the shared peer snapshot, so it cannot know itself
    // more precisely than its opponents know it.
    self_peer: Option<&PerceptionPeer>,
    aggression: Option<&ambition_combat::components::ActorAggression>,
    // NOT `Option`, per ADR 0024 §1 ("absence is never a policy and no outer
    // query may interpret a missing component as axis-swept"). The `None` arm of
    // the old signature did precisely that, and it was invisible to
    // `engine.movement-model-is-never-optional` because that rule matches the
    // spelling `Option<&MotionModel>`, not `Option<&ae::MotionModel>`.
    motion_model: &ae::MotionModel,
    // The capture relationship, resolved by the caller. Passed rather than
    // queried for the reason the whole signature is passed: this function reads
    // authorities the caller already holds, and a lookup here would be a second
    // reader of a relationship the combat layer owns.
    capture: ambition_combat::capture::systems::CaptureFacts,
) -> PerceptionBody {
    // the fallback below reads a PRESENT non-axis model (a crawler has no
    // air-dodge window, so "no window open, no endlag" is the honest answer for
    // one) — never a missing component, which the signature now forbids.
    let axis_motion = match motion_model {
        ae::MotionModel::AxisSwept(axis) => *axis,
        _ => ae::AxisSweptMotion::default(),
    };
    let burst_maneuver = ae::resolve_burst_maneuver(
        body.abilities,
        body.ground,
        body.dodge,
        &axis_motion.state,
        body.dash,
        axis_motion.params,
    );
    PerceptionBody {
        captured: capture.captured,
        captured_for: capture.captured_for,
        holding_captive: capture.holding_captive,
        pummels_landed: capture.pummels_landed,
        pos: body.kin.pos,
        vel: body.kin.vel,
        facing: body.kin.facing,
        // FB1: was `body.kin.size` — the FULL size handed to a HALF
        // extent. `WorldView::reachable` swept a box twice the body.
        half_extent: body.kin.size * 0.5,
        faction,
        gravity_down,
        on_ground: body.ground.on_ground,
        aerial: body.surface.gravity_scale <= 0.001,
        alive: body.health.alive(),
        can_fire: action_set.is_some_and(|a| a.ranged.is_some()),
        // Movement capability is read off the body's own
        // `AbilitySet` — the single authority every body
        // shares — not a parallel `CombatCapabilities` mirror.
        can_blink: body.abilities.abilities.blink,
        // THIS WAS `abilities.dash` / `abilities.dodge`,
        // AND A CAPABILITY IS NOT AN AVAILABILITY. Dodge and
        // dash are one button; which maneuver a press produces
        // is decided by the body's live state, and `apply_dodge`
        // declines on cooldown WITHOUT consuming the buffered
        // press so `apply_dash` takes it. A brain reading the
        // two flags decided "dodge" and the body dashed. The
        // kernel resolves it now and perception carries the
        // answer, so there is one rule rather than a driver
        // re-deriving the kernel's precedence from outside.
        burst: burst_maneuver,
        can_shield: body.abilities.abilities.shield,
        // The same counter `actor_movement` spends; a brain
        // planning a recovery reads the body's real budget,
        // not an assumption about what a fighter usually has.
        air_jumps_left: body.jump.air_jumps_available,
        phase: self_peer.map(|p| p.phase).unwrap_or_default(),
        phase_remaining: self_peer.map_or(0.0, |p| p.phase_remaining),
        invulnerable: self_peer.is_some_and(|p| p.invulnerable),
        tumbling: self_peer.is_some_and(|p| p.tumbling),
        damage_taken: body.health.damage_taken(),
        health_max: body.health.max(),
        // A grudge makes ONE same-faction body a foe (the duel
        // mechanism); carry it so this body's `nearest_hostile`
        // matches the foe `select_actor_targets` would pick.
        grudge: aggression.and_then(|a| a.grudge),
        // Read off this body's OWN peer row rather than a
        // fresh query, exactly like `phase` above — one
        // derivation, so a body cannot disagree with the rest
        // of the world about which team it is on.
        team: self_peer.and_then(|p| p.team.clone()),
    }
}

/// Where this body BELIEVES its target is, after seeing and remembering.
///
/// Sight and memory are one answer, so they are one call: the belief store is
/// updated from what the body just saw, and the target it then reports may come
/// from either. Splitting them let a caller update memory and forget to consult
/// it, or consult it without updating.
///
/// `None` means the policy does not override — an `Omniscient` body already
/// carries the global target and simply knows. `Some(None)` means a sighted body
/// perceives nobody, which is a real answer (idle), not a missing one.
pub(crate) fn believed_target(
    policy: Perception,
    view: &ambition_characters::perception::WorldView,
    mut memory: Option<&mut PerceptionMemory>,
    dt: f32,
) -> Option<Option<ae::Vec2>> {
    if let Some(mem) = memory.as_deref_mut() {
        mem.0.update(view, dt);
    }
    match policy {
        Perception::Omniscient => None,
        // The nearest foe IN VIEW, or when none is visible the most-confident foe
        // the body REMEMBERS — pursuit of one that left the viewport (invariant
        // I6).
        Perception::Sighted { .. } => Some(view.nearest_hostile().map(|a| a.pos).or_else(|| {
            memory
                .as_deref()
                .and_then(|m| m.0.last_known_hostile().map(|r| r.pos))
        })),
    }
}

/// What a body can perceive this tick, as one parameter.
///
/// The three channels a brain's world-out view needs are collected by three
/// systems that run before it: [`PerceptionPeers`] (the other bodies),
/// [`PerceptionProjectiles`] (the live shots), and the hostility table that says
/// which of those are enemies. A consumer needs all three or none of them —
/// building a view from two of the three is not a smaller view, it is a wrong
/// one — which is what makes this one concept rather than a bundle.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PerceivedWorld<'w, 's> {
    peers: Option<bevy::prelude::Res<'w, PerceptionPeers>>,
    projectiles: Option<bevy::prelude::Res<'w, PerceptionProjectiles>>,
    relations: Option<bevy::prelude::Res<'w, FactionRelations>>,
    /// A borrowable all-peaceful table, so [`Self::relations`] can hand out a
    /// reference whether or not the live resource exists. Never written.
    empty_relations: bevy::prelude::Local<'s, FactionRelations>,
}

impl PerceivedWorld<'_, '_> {
    /// The live hostility table, or an all-peaceful one when none is registered.
    pub fn relations(&self) -> &FactionRelations {
        self.relations.as_deref().unwrap_or(&self.empty_relations)
    }

    /// Every peer this viewer perceives — that is, all of them but itself.
    pub fn peers_seen_by(&self, viewer: bevy::prelude::Entity) -> Vec<PerceptionPeer> {
        self.peers
            .as_ref()
            .map(|peers| {
                peers
                    .0
                    .iter()
                    .filter(|peer| peer.entity != viewer)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// This body's OWN peer row.
    ///
    /// a body reads its own move phase and i-frames from the same snapshot
    /// every opponent reads them from, so it cannot know itself more precisely
    /// than it is known.
    pub fn peer(&self, body: bevy::prelude::Entity) -> Option<&PerceptionPeer> {
        self.peers
            .as_ref()
            .and_then(|peers| peers.0.iter().find(|peer| peer.entity == body))
    }

    /// The live shots in flight.
    pub fn projectiles(&self) -> &[PerceptionProjectile] {
        self.projectiles.as_ref().map_or(&[], |p| p.0.as_slice())
    }
}
