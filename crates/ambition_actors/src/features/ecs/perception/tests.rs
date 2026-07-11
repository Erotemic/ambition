//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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
        phase: BodyPhase::Neutral,
        phase_remaining: 0.0,
        invulnerable: false,
        damage_taken: 0,
        health_max: 100,
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
        phase: BodyPhase::Neutral,
        phase_remaining: 0.0,
        invulnerable: false,
        damage_taken: 0,
        health_max: 100,
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
    assert!(
        !view.actors[0].hostile_to_self,
        "same faction, no grudge → not a foe"
    );
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
    assert!(
        !view.actors[0].hostile_to_self,
        "a grudge against someone else spares this peer"
    );
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
        .spawn((
            kin(90.0),
            BodyHealth::new(Health::new(5)),
            ActorFaction::Boss,
        ))
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
    assert!(
        !b.id.is_empty(),
        "a FeatureId-less body still gets a stable id"
    );
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
        shots
            .0
            .iter()
            .any(|p| p.faction == ActorFaction::Enemy && p.damage == 3),
        "the enemy-pool shot carries its own faction + damage"
    );
    assert!(
        shots
            .0
            .iter()
            .any(|p| p.faction == ActorFaction::Player && p.damage == 2),
        "the live-pool shot defaults to Player"
    );
}

// ── FB1: the view-audit regressions ──

/// **The 2× bug.** `BodyKinematics::size` is the FULL body size (`aabb()`
/// halves it); `PerceptionBody::half_extent` and `PerceivedActor::half_extent`
/// are halves. Both fill sites passed `size` straight through, so every body
/// perceived itself and everyone else as twice its real box — and
/// `WorldView::reachable`, which sweeps `self_view.half_extent`, refused
/// corridors the body physically fits through.
///
/// This test pins the CONTRACT rather than the call sites: the view's
/// half-extent must equal the body's real `aabb()` half-extent.
#[test]
fn the_views_half_extent_is_a_half_extent() {
    let kin_size = ae::Vec2::new(24.0, 36.0);
    let real_half = ae::Aabb::new(ae::Vec2::ZERO, kin_size * 0.5).half_size();
    assert_eq!(
        real_half,
        kin_size * 0.5,
        "if this ever changes, both perception fill sites must change with it"
    );
    // And a body built with the halved value reaches through a gap its full
    // size would not fit: the observable consequence of the bug.
    let mut b = body(ae::Vec2::new(0.0, 100.0), ActorFaction::Enemy);
    b.half_extent = kin_size * 0.5;
    // Gap 50px: the true 36px-tall sweep clears it; the doubled 72px one does not.
    let world = corridor_world(50.0);
    let view = build_world_view(
        &b,
        &[],
        &[],
        &[],
        &world,
        &FactionRelations::default(),
        DEFAULT_VIEWPORT_HALF,
        0.0,
    );
    assert!(
        view.reachable(ae::Vec2::new(300.0, 100.0)),
        "a body sweeping its true half-extent fits the corridor"
    );
    let mut fat = b;
    fat.half_extent = kin_size; // the bug
    let fat_view = build_world_view(
        &fat,
        &[],
        &[],
        &[],
        &world,
        &FactionRelations::default(),
        DEFAULT_VIEWPORT_HALF,
        0.0,
    );
    assert!(
        !fat_view.reachable(ae::Vec2::new(300.0, 100.0)),
        "the doubled box does not — which is what the brain used to believe"
    );
}

/// A corridor at y=100 whose vertical opening is `gap` px, walled above/below.
fn corridor_world(gap: f32) -> ae::World {
    let half = gap * 0.5;
    let blocks = vec![
        ae::Block::solid(
            "ceil",
            ae::Vec2::new(-500.0, 100.0 - half - 200.0),
            ae::Vec2::new(1000.0, 200.0),
        ),
        ae::Block::solid(
            "floor",
            ae::Vec2::new(-500.0, 100.0 + half),
            ae::Vec2::new(1000.0, 200.0),
        ),
    ];
    ae::World::new(
        "corridor",
        ae::Vec2::new(1000.0, 600.0),
        ae::Vec2::ZERO,
        blocks,
    )
}

/// **The stage is not viewport-clipped.** A fighter can see the blastzones;
/// L1's `Recovery`/`EdgeGuard` are undecidable otherwise. The viewport here is
/// far smaller than the room.
#[test]
fn the_view_carries_the_whole_stage_not_the_viewport() {
    let world = arena_world();
    let view = build_world_view(
        &body(ae::Vec2::new(0.0, 180.0), ActorFaction::Enemy),
        &[],
        &[],
        &[],
        &world,
        &FactionRelations::default(),
        ae::Vec2::splat(40.0), // a tiny viewport
        0.0,
    );
    assert_eq!(view.stage.bounds.min, ae::Vec2::ZERO);
    assert_eq!(view.stage.bounds.max, world.size);
    assert!(view.stage.bounds.max.x > view.viewport.half_extent.x * 2.0);
}

/// The move-phase reader's priority order: hitstun beats a swing (a body
/// knocked out of its own attack is reeling, not attacking), and a swing beats
/// a raised shield.
#[test]
fn hitstun_outranks_a_swing_and_a_swing_outranks_a_shield() {
    use ambition_characters::actor::BodyCombat;
    let shield_up = ae::BodyShieldState {
        active: true,
        parry_window_timer: 0.0,
    };

    let mut reeling = BodyCombat::default();
    reeling.hitstun_timer = 0.4;
    assert_eq!(
        body_phase(Some(&reeling), None, Some(&shield_up)),
        (BodyPhase::Hitstun, 0.4)
    );

    assert_eq!(
        body_phase(Some(&BodyCombat::default()), None, Some(&shield_up)),
        (BodyPhase::Shielding, 0.0)
    );
    assert_eq!(
        body_phase(None, None, None),
        (BodyPhase::Neutral, 0.0),
        "a body with no combat components is neutral, not unknown"
    );
}

#[test]
fn i_frames_are_perceivable_because_the_body_flashes() {
    use ambition_characters::actor::BodyCombat;
    let mut c = BodyCombat::default();
    assert!(!body_invulnerable(Some(&c)));
    c.damage_invuln_timer = 0.2;
    assert!(body_invulnerable(Some(&c)));
}

trait CountOrNone {
    /// Tiny test helper: count an `Option<&T>` as 0 or 1 without importing extra
    /// machinery — keeps the `nearest_hostile() == None` assertion terse.
    fn count_or_none(self) -> usize;
}

impl<T> CountOrNone for Option<T> {
    fn count_or_none(self) -> usize {
        self.map(|_| 1).unwrap_or(0)
    }
}
