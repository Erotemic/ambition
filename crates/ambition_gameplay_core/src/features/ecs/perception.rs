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
    PerceivedActor, PerceivedPortal, PerceivedProjectile, PerceivedSolid, SelfView, SolidKind,
    Viewport, WorldView,
};

use crate::combat::targeting::FactionRelations;

/// Default viewport half-extent (world px) — the AI analogue of the human's
/// screen. Generous so a body perceives approaching threats with room to react;
/// a per-body override can come later if a character wants keener or duller
/// senses.
pub const DEFAULT_VIEWPORT_HALF: ae::Vec2 = ae::Vec2::new(480.0, 320.0);

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
pub fn collect_perception_peers(
    mut peers: bevy::prelude::ResMut<PerceptionPeers>,
    bodies: bevy::prelude::Query<(
        bevy::prelude::Entity,
        Option<&crate::features::FeatureId>,
        &crate::actor::BodyKinematics,
        &ambition_characters::actor::BodyHealth,
        &ActorFaction,
    )>,
) {
    peers.0.clear();
    for (entity, id, kin, health, faction) in &bodies {
        peers.0.push(PerceptionPeer {
            entity,
            id: id
                .map(|f| f.as_str().to_string())
                .unwrap_or_else(|| format!("e{}", entity.index())),
            pos: kin.pos,
            vel: kin.vel,
            facing: kin.facing,
            half_extent: kin.size,
            faction: *faction,
            alive: health.alive(),
            on_ground: false,
            shield_raised: false,
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
        (&crate::actor::BodyKinematics, &crate::projectile::ProjectileGameplay, &ActorFaction),
        bevy::prelude::With<crate::enemy_projectile::EnemyProjectile>,
    >,
    live_pool: bevy::prelude::Query<
        (&crate::actor::BodyKinematics, &crate::projectile::ProjectileGameplay),
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
/// [`ensure_perception_memory`].
#[derive(bevy::prelude::Component, Default)]
pub struct PerceptionMemory(pub ambition_characters::perception::WorldMemory);

/// Attach a default [`PerceptionMemory`] to every non-boss brained actor that lacks
/// one, so the perceived-target derivation always has a belief store to pursue from.
/// Runs before the brain tick. Matches `tick_actor_brains`' own body set (brained,
/// non-player, non-boss).
///
/// The `Without<BossConfig>` here is documented POLICY, not a parallel-system
/// carve-out (§A7): the player brain doesn't perceive-target (it steers from
/// controller input), and a boss now perceives its foe through the SAME world-out
/// port (`tick_boss_brains_system`), but with an ARENA-WIDE viewport (half-extent =
/// the whole world) — so it never loses sight of the player and needs no off-screen
/// belief store to pursue from. A boss that wanted a bounded viewport (and thus
/// off-screen pursuit) would drop this exclusion and gain memory here; today none do.
pub fn ensure_perception_memory(
    mut commands: bevy::prelude::Commands,
    bodies: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<ambition_characters::brain::Brain>,
            bevy::prelude::With<crate::features::FeatureSimEntity>,
            bevy::prelude::Without<crate::actor::PlayerEntity>,
            bevy::prelude::Without<crate::combat::boss_clusters::BossConfig>,
            bevy::prelude::Without<PerceptionMemory>,
        ),
    >,
) {
    for entity in &bodies {
        commands.entity(entity).insert(PerceptionMemory::default());
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
mod tests {
    use super::*;

    fn body(pos: ae::Vec2, faction: ActorFaction) -> PerceptionBody {
        PerceptionBody {
            pos,
            vel: ae::Vec2::ZERO,
            facing: 1.0,
            half_extent: ae::Vec2::new(12.0, 18.0),
            faction,
            gravity_down: ae::Vec2::new(0.0, 1.0),
            on_ground: true,
            aerial: false,
            alive: true,
            can_fire: true,
            can_blink: false,
            can_dash: false,
            can_shield: false,
            grudge: None,
        }
    }

    fn peer(id: &str, pos: ae::Vec2, faction: ActorFaction) -> PerceptionPeer {
        PerceptionPeer {
            entity: bevy::prelude::Entity::PLACEHOLDER,
            id: id.to_string(),
            pos,
            vel: ae::Vec2::ZERO,
            facing: -1.0,
            half_extent: ae::Vec2::new(12.0, 18.0),
            faction,
            alive: true,
            on_ground: true,
            shield_raised: false,
        }
    }

    /// A real room: a floor and a wall between two combatants standing on it.
    fn arena_world() -> ae::World {
        let blocks = vec![
            ae::Block::solid(
                "floor",
                ae::Vec2::new(-500.0, 200.0),
                ae::Vec2::new(1000.0, 40.0),
            ),
            // A wall at x≈300, between a body at x=100 and one at x=500.
            ae::Block::solid(
                "wall",
                ae::Vec2::new(292.0, 40.0),
                ae::Vec2::new(16.0, 160.0),
            ),
        ];
        ae::World::new(
            "perception_arena",
            ae::Vec2::new(1000.0, 400.0),
            ae::Vec2::new(100.0, 180.0),
            blocks,
        )
    }

    /// Body-generic + relational: an Enemy body and a Boss body, made mutually
    /// hostile, each perceive the other as a hostile target — and the SAME builder
    /// runs for a Player-faction body (guardrail #1: no enemy-only path).
    #[test]
    fn builds_relational_view_for_any_faction() {
        let mut relations = FactionRelations::default();
        relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
        let world = arena_world();

        let enemy = body(ae::Vec2::new(100.0, 180.0), ActorFaction::Enemy);
        let peers = vec![peer("pca", ae::Vec2::new(180.0, 180.0), ActorFaction::Boss)];
        let view = build_world_view(
            &enemy,
            &peers,
            &[],
            &[],
            &world,
            &relations,
            DEFAULT_VIEWPORT_HALF,
            0.0,
        );
        // The Boss peer is in view and resolved hostile to the Enemy viewer.
        assert_eq!(view.actors.len(), 1);
        assert!(view.actors[0].hostile_to_self);
        assert_eq!(view.nearest_hostile().map(|a| a.id.as_str()), Some("pca"));
        // The floor + wall are clipped into the local terrain.
        assert!(
            view.terrain.iter().any(|s| s.kind == SolidKind::Solid),
            "the real floor/wall geometry is carried into the view"
        );

        // The exact same function builds a view for a PLAYER-faction body — the
        // player-robot body perceives identically (no player-centric branch). It
        // sees an Npc peer (which neither faction fights by default), and resolves
        // it as NOT a target — proving hostility is data, not the viewer's type.
        let player = body(ae::Vec2::new(100.0, 180.0), ActorFaction::Player);
        let npc_peers = vec![peer(
            "bystander",
            ae::Vec2::new(180.0, 180.0),
            ActorFaction::Npc,
        )];
        let player_view = build_world_view(
            &player,
            &npc_peers,
            &[],
            &[],
            &world,
            &relations,
            DEFAULT_VIEWPORT_HALF,
            0.0,
        );
        assert_eq!(player_view.actors.len(), 1);
        assert!(!player_view.actors[0].hostile_to_self);
        assert_eq!(player_view.nearest_hostile().count_or_none(), 0);
    }

    /// §A7 grudge: a SAME-faction peer the viewer holds a grudge against is
    /// perceived as hostile (so `nearest_hostile` finds a grudge-duel opponent that
    /// faction hostility alone would miss) — matching `select_actor_targets`' foe set.
    #[test]
    fn a_grudge_makes_a_same_faction_peer_hostile() {
        let relations = FactionRelations::default(); // no Npc↔Npc hostility
        let world = arena_world();
        // Two distinct real entity handles from a throwaway ECS world.
        let mut ecs = bevy::prelude::World::new();
        let foe_entity = ecs.spawn_empty().id();
        let other_entity = ecs.spawn_empty().id();
        // Two same-faction NPCs; without a grudge neither is a foe.
        let mut viewer = body(ae::Vec2::new(100.0, 180.0), ActorFaction::Npc);
        let mut foe = peer("duel_foe", ae::Vec2::new(180.0, 180.0), ActorFaction::Npc);
        foe.entity = foe_entity;

        // No grudge → the same-faction peer is NOT a target.
        let view = build_world_view(
            &viewer,
            std::slice::from_ref(&foe),
            &[],
            &[],
            &world,
            &relations,
            DEFAULT_VIEWPORT_HALF,
            0.0,
        );
        assert_eq!(view.actors.len(), 1);
        assert!(!view.actors[0].hostile_to_self, "same faction, no grudge → not a foe");
        assert!(view.nearest_hostile().is_none());

        // Grudge against that exact entity → it becomes the perceived hostile.
        viewer.grudge = Some(foe_entity);
        let view = build_world_view(
            &viewer,
            std::slice::from_ref(&foe),
            &[],
            &[],
            &world,
            &relations,
            DEFAULT_VIEWPORT_HALF,
            0.0,
        );
        assert!(view.actors[0].hostile_to_self, "the grudge entity is a foe");
        assert_eq!(
            view.nearest_hostile().map(|a| a.id.as_str()),
            Some("duel_foe"),
            "nearest_hostile resolves the grudge opponent (the duel mechanism)"
        );
        // A grudge against a DIFFERENT entity does not implicate this peer.
        viewer.grudge = Some(other_entity);
        let view = build_world_view(
            &viewer,
            std::slice::from_ref(&foe),
            &[],
            &[],
            &world,
            &relations,
            DEFAULT_VIEWPORT_HALF,
            0.0,
        );
        assert!(!view.actors[0].hostile_to_self, "a grudge against someone else spares this peer");
    }

    /// Line-of-fire over the REAL clipped geometry: a wall between two bodies
    /// blocks the shot; an unobstructed shot is clear. This is the query reusing
    /// the same solids the physics collides against.
    #[test]
    fn line_of_fire_uses_real_clipped_terrain() {
        let relations = FactionRelations::default();
        let world = arena_world();
        let shooter = body(ae::Vec2::new(100.0, 120.0), ActorFaction::Enemy);
        let view = build_world_view(
            &shooter,
            &[],
            &[],
            &[],
            &world,
            &relations,
            DEFAULT_VIEWPORT_HALF,
            0.0,
        );
        // Target on the far side of the x≈300 wall, same height → blocked.
        assert!(!view.line_of_fire(ae::Vec2::new(500.0, 120.0)));
        // Target straight up (clear of floor + wall) → in line of fire.
        assert!(view.line_of_fire(ae::Vec2::new(100.0, 60.0)));
    }

    /// A body only perceives what is inside its viewport — a peer far outside is
    /// not in the actor list (it would instead be retained by `WorldMemory`).
    #[test]
    fn peers_outside_viewport_are_not_perceived() {
        let mut relations = FactionRelations::default();
        relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
        let world = arena_world();
        let viewer = body(ae::Vec2::new(100.0, 180.0), ActorFaction::Enemy);
        // Far beyond DEFAULT_VIEWPORT_HALF.x = 480.
        let peers = vec![peer(
            "far",
            ae::Vec2::new(2000.0, 180.0),
            ActorFaction::Boss,
        )];
        let view = build_world_view(
            &viewer,
            &peers,
            &[],
            &[],
            &world,
            &relations,
            DEFAULT_VIEWPORT_HALF,
            0.0,
        );
        assert!(view.actors.is_empty(), "an out-of-viewport peer is unseen");
    }

    /// A hostile projectile in view is flagged as a threat; a same-side one is not.
    #[test]
    fn projectile_threat_resolved_relationally() {
        let relations = FactionRelations::default();
        let world = arena_world();
        let player = body(ae::Vec2::new(100.0, 180.0), ActorFaction::Player);
        let shots = vec![
            // Enemy shot near the player → threatens the player.
            PerceptionProjectile {
                pos: ae::Vec2::new(160.0, 180.0),
                vel: ae::Vec2::new(-200.0, 0.0),
                damage: 1,
                faction: ActorFaction::Enemy,
            },
            // Player's own shot → does not threaten the player.
            PerceptionProjectile {
                pos: ae::Vec2::new(160.0, 180.0),
                vel: ae::Vec2::new(200.0, 0.0),
                damage: 1,
                faction: ActorFaction::Player,
            },
        ];
        let view = build_world_view(
            &player,
            &[],
            &shots,
            &[],
            &world,
            &relations,
            DEFAULT_VIEWPORT_HALF,
            0.0,
        );
        assert_eq!(view.projectiles.len(), 2);
        assert_eq!(
            view.projectiles
                .iter()
                .filter(|p| p.hostile_to_self)
                .count(),
            1
        );
        assert_eq!(view.incoming_threats().count(), 1);
    }

    /// Portals are clipped to the viewport and the paired exit is resolvable from
    /// the perceived value (the data S5 routes through).
    #[test]
    fn portals_in_view_link_to_their_pair() {
        let relations = FactionRelations::default();
        let world = arena_world();
        let viewer = body(ae::Vec2::new(100.0, 180.0), ActorFaction::Enemy);
        let near = PerceptionPortal {
            pos: ae::Vec2::new(140.0, 180.0),
            normal: ae::Vec2::new(-1.0, 0.0),
            half_extent: ae::Vec2::new(4.0, 24.0),
            channel_key: 3,
        };
        let near_pair = PerceptionPortal {
            pos: ae::Vec2::new(260.0, 180.0),
            normal: ae::Vec2::new(1.0, 0.0),
            half_extent: ae::Vec2::new(4.0, 24.0),
            channel_key: 3,
        };
        // Far outside DEFAULT_VIEWPORT_HALF.x = 480 — clipped out.
        let far = PerceptionPortal {
            pos: ae::Vec2::new(3000.0, 180.0),
            normal: ae::Vec2::new(0.0, -1.0),
            half_extent: ae::Vec2::new(24.0, 4.0),
            channel_key: 5,
        };
        let view = build_world_view(
            &viewer,
            &[],
            &[],
            &[near, near_pair, far],
            &world,
            &relations,
            DEFAULT_VIEWPORT_HALF,
            0.0,
        );
        assert_eq!(
            view.portals.len(),
            2,
            "the far portal is clipped out of view"
        );
        let entry = view
            .portals
            .iter()
            .find(|p| p.pos == ae::Vec2::new(140.0, 180.0))
            .unwrap();
        assert_eq!(
            view.linked_portal(entry).map(|p| p.pos),
            Some(ae::Vec2::new(260.0, 180.0)),
            "entering one aperture resolves to its same-channel exit"
        );
    }

    /// §A7 peers-wiring: `collect_perception_peers` snapshots EVERY body (player,
    /// actor, boss — all carry `BodyKinematics`) into the resource `build_world_view`
    /// reads, with its source `Entity` (so a viewer excludes itself). A body without a
    /// `FeatureId` still gets a stable non-empty id.
    #[test]
    fn collect_perception_peers_snapshots_every_body() {
        use ambition_characters::actor::{BodyHealth, Health};
        use bevy::prelude::*;

        let mut app = App::new();
        app.init_resource::<PerceptionPeers>();
        app.add_systems(Update, collect_perception_peers);
        let kin = |x: f32| crate::actor::BodyKinematics {
            pos: ae::Vec2::new(x, 20.0),
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(14.0, 22.0),
            facing: 1.0,
        };
        let alice = app
            .world_mut()
            .spawn((
                crate::features::FeatureId::new("alice"),
                kin(10.0),
                BodyHealth::new(Health::new(5)),
                ActorFaction::Enemy,
            ))
            .id();
        // No FeatureId → the snapshot derives a stable entity id.
        let bob = app
            .world_mut()
            .spawn((kin(90.0), BodyHealth::new(Health::new(5)), ActorFaction::Boss))
            .id();
        app.update();

        let peers = app.world().resource::<PerceptionPeers>();
        assert_eq!(peers.0.len(), 2, "every body is snapshotted");
        let a = peers.0.iter().find(|p| p.entity == alice).unwrap();
        assert_eq!(a.id, "alice");
        assert_eq!(a.pos, ae::Vec2::new(10.0, 20.0));
        assert_eq!(a.faction, ActorFaction::Enemy);
        assert!(a.alive);
        let b = peers.0.iter().find(|p| p.entity == bob).unwrap();
        assert!(!b.id.is_empty(), "a FeatureId-less body still gets a stable id");
    }

    /// §A7 projectiles-wiring: `collect_perception_projectiles` snapshots BOTH pools —
    /// the `enemy_projectile` pool reading its own `ActorFaction`, the `LiveProjectile`
    /// pool defaulting to Player — with pos/vel/damage from the shared
    /// `BodyKinematics` + `ProjectileGameplay`.
    #[test]
    fn collect_perception_projectiles_snapshots_both_pools() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.init_resource::<PerceptionProjectiles>();
        app.add_systems(Update, collect_perception_projectiles);
        let kin = |x: f32| crate::actor::BodyKinematics {
            pos: ae::Vec2::new(x, 0.0),
            vel: ae::Vec2::new(-100.0, 0.0),
            size: ae::Vec2::new(8.0, 8.0),
            facing: -1.0,
        };
        let game = |dmg: i32| crate::projectile::ProjectileGameplay {
            age: 0.0,
            max_lifetime: 2.0,
            gravity: 0.0,
            damage: dmg,
            bounces_remaining: 0,
            world_hit: crate::projectile::WorldHitPolicy::ExpireOnContact,
        };
        app.world_mut().spawn((
            crate::enemy_projectile::EnemyProjectile,
            kin(200.0),
            game(3),
            ActorFaction::Enemy,
        ));
        app.world_mut()
            .spawn((crate::projectile::LiveProjectile, kin(50.0), game(2)));
        app.update();

        let shots = app.world().resource::<PerceptionProjectiles>();
        assert_eq!(shots.0.len(), 2, "both pools snapshotted");
        assert!(
            shots.0.iter().any(|p| p.faction == ActorFaction::Enemy && p.damage == 3),
            "the enemy-pool shot carries its own faction + damage"
        );
        assert!(
            shots.0.iter().any(|p| p.faction == ActorFaction::Player && p.damage == 2),
            "the live-pool shot defaults to Player"
        );
    }
}

#[cfg(test)]
trait CountOrNone {
    /// Tiny test helper: count an `Option<&T>` as 0 or 1 without importing extra
    /// machinery — keeps the `nearest_hostile() == None` assertion terse.
    fn count_or_none(self) -> usize;
}

#[cfg(test)]
impl<T> CountOrNone for Option<T> {
    fn count_or_none(self) -> usize {
        self.map(|_| 1).unwrap_or(0)
    }
}
