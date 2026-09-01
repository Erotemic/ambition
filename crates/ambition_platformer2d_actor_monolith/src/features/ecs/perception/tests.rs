use super::*;

fn body(pos: ae::Vec2, faction: ActorFaction) -> PerceptionBody {
    PerceptionBody {
        // A fixture hands in a peer list it built itself, with no row for the
        // viewer; there is nobody to exclude.
        viewer: None,
        captured: false,
        captured_for: 0.0,
        holding_captive: false,
        pummels_landed: 0,
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
        burst: ae::BurstManeuver::None,
        can_shield: false,
        // A fresh body with its air game intact; the fixture is about what a
        // viewer SEES, and a recovery budget of zero would be a different test.
        air_jumps_left: 1,
        team: None,
        phase: BodyPhase::Neutral,
        phase_remaining: 0.0,
        invulnerable: false,
        tumbling: false,
        damage_taken: 0,
        health_max: 100,
        grudge: None,
    }
}

fn peer(id: &str, pos: ae::Vec2, faction: ActorFaction) -> PerceptionPeer {
    PerceptionPeer {
        team: None,
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
        tumbling: false,
        ledge_hanging: false,
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
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
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
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
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
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
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
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
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
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
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
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
        0.0,
    );
    // Target on the far side of the x≈300 wall, same height → blocked.
    assert!(!view.line_of_fire(ae::Vec2::new(500.0, 120.0)));
    // Target straight up (clear of floor + wall) → in line of fire.
    assert!(view.line_of_fire(ae::Vec2::new(100.0, 60.0)));
}

/// ⭐ AN OMNISCIENT BODY PERCEIVES A PEER ANYWHERE, which is what the policy has
/// always claimed and did not do: the view was built at the default extent under
/// both modes, and only the `ActorTarget` derivation ignored the box. A brain
/// that reaches its foe through `view.actors` — the fighter brain does — saw an
/// "omniscient" body as sighted at 480px.
///
/// ⛔ The twin below is the other half of the same rule and they must be read
/// together: same viewer, same peer, same distance, opposite policy, opposite
/// answer. A change that made this pass by clipping nothing at all would take
/// `peers_outside_viewport_are_not_perceived` down with it.
#[test]
fn an_omniscient_body_perceives_a_peer_far_outside_its_tactical_extent() {
    let mut relations = FactionRelations::default();
    relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
    let world = arena_world();
    let viewer = body(ae::Vec2::new(100.0, 180.0), ActorFaction::Enemy);
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
        Perception::Omniscient,
        0.0,
    );
    assert_eq!(
        view.actors.len(),
        1,
        "an omniscient body knows where every hostile is, at any distance"
    );
    assert!(
        view.nearest_hostile().is_some(),
        "and the brain reaches it through the view, not only through ActorTarget"
    );
    // The TACTICAL extent is unchanged — omniscience is a claim about where
    // bodies are, not a licence to fold a whole room's geometry into one tick.
    assert_eq!(view.viewport.half_extent, DEFAULT_VIEWPORT_HALF);
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
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
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
            faction: Some(ActorFaction::Enemy),
            team: None,
        },
        // Player's own shot → does not threaten the player.
        PerceptionProjectile {
            pos: ae::Vec2::new(160.0, 180.0),
            vel: ae::Vec2::new(200.0, 0.0),
            damage: 1,
            faction: Some(ActorFaction::Player),
            team: None,
        },
        // Environmental/ownerless shot → indiscriminate, matching damage routing.
        PerceptionProjectile {
            pos: ae::Vec2::new(140.0, 180.0),
            vel: ae::Vec2::new(-50.0, 0.0),
            damage: 1,
            faction: None,
            team: None,
        },
    ];
    // Same authored faction but a different match team: team authority
    // outranks faction exactly as projectile damage does.
    let mut team_player = body(ae::Vec2::new(100.0, 180.0), ActorFaction::Player);
    team_player.team = Some(ambition_combat::targeting::MatchTeam::new("blue"));
    let team_shot = PerceptionProjectile {
        pos: ae::Vec2::new(150.0, 180.0),
        vel: ae::Vec2::ZERO,
        damage: 1,
        faction: Some(ActorFaction::Player),
        team: Some(ambition_combat::targeting::MatchTeam::new("red")),
    };
    let team_view = build_world_view(
        &team_player,
        &[],
        &[team_shot],
        &[],
        &world,
        &relations,
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
        0.0,
    );
    assert!(
        team_view.projectiles[0].hostile_to_self,
        "different match teams make the projectile threatening even when factions match"
    );

    let view = build_world_view(
        &player,
        &[],
        &shots,
        &[],
        &world,
        &relations,
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
        0.0,
    );
    assert_eq!(view.projectiles.len(), 3);
    assert_eq!(
        view.projectiles
            .iter()
            .filter(|p| p.hostile_to_self)
            .count(),
        2
    );
    assert_eq!(
        view.incoming_threats().count(),
        2,
        "both the hostile enemy shot and the ownerless environmental shot are closing; \
         the friendly shot is receding and must not be a dodge candidate",
    );
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
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
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
    let kin = |x: f32| ambition_platformer2d_core::BodyKinematics {
        pos: ae::Vec2::new(x, 20.0),
        vel: ae::Vec2::ZERO,
        size: ae::Vec2::new(14.0, 22.0),
        facing: 1.0,
    };
    let alice = app
        .world_mut()
        .spawn((
            ambition_combat::components::FeatureId::new("alice"),
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

/// §A7 projectiles-wiring: `collect_perception_projectiles` snapshots the single
/// live-projectile occurrence family exactly once and reads side from the frozen
/// allegiance rather than from the shot's presentation vocabulary.
#[test]
fn collect_perception_projectiles_snapshots_live_projectiles_once_with_frozen_side() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<PerceptionProjectiles>();
    app.add_systems(Update, collect_perception_projectiles);
    let kin = |x: f32| ambition_platformer2d_core::BodyKinematics {
        pos: ae::Vec2::new(x, 0.0),
        vel: ae::Vec2::new(-100.0, 0.0),
        size: ae::Vec2::new(8.0, 8.0),
        facing: -1.0,
    };
    let game = |dmg: i32| ambition_projectiles::ProjectileGameplay {
        age: 0.0,
        max_lifetime: 2.0,
        gravity: 0.0,
        damage: dmg,
        bounces_remaining: 0,
        world_hit: ambition_projectiles::WorldHitPolicy::ExpireOnContact,
        accel: ae::Vec2::ZERO,
        hits_cleared_on_leg: 0,
    };
    app.world_mut().spawn((
        ambition_projectiles::LiveProjectile,
        kin(200.0),
        game(3),
        crate::projectile::ProjectileAllegiance {
            faction: ActorFaction::Enemy,
            team: Some(ambition_combat::targeting::MatchTeam::new("red")),
        },
    ));
    app.world_mut()
        .spawn((ambition_projectiles::LiveProjectile, kin(50.0), game(2)));
    app.update();

    let shots = app.world().resource::<PerceptionProjectiles>();
    assert_eq!(shots.0.len(), 2, "one row per live projectile");
    assert!(
        shots.0.iter().any(|p| {
            p.faction == Some(ActorFaction::Enemy)
                && p.team.as_ref().is_some_and(|team| team.as_str() == "red")
                && p.damage == 3
        }),
        "the sided shot carries its frozen allegiance"
    );
    assert!(
        shots
            .0
            .iter()
            .any(|p| p.faction.is_none() && p.team.is_none() && p.damage == 2),
        "an ownerless shot remains explicitly unsided rather than defaulting to Player"
    );
}

// ── FB1: the view-audit regressions ──

/// Both fill sites passed `size` straight through, so every body perceived itself and everyone
/// else as twice its real box.
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
    let mut b = body(ae::Vec2::new(0.0, 100.0), ActorFaction::Enemy);
    b.half_extent = kin_size * 0.5;
    let world = corridor_world(50.0);
    let view = build_world_view(
        &b,
        &[],
        &[],
        &[],
        &world,
        &FactionRelations::default(),
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
        0.0,
    );
    // The observable consequence, on the value the builder actually published:
    // a HALF, never the full size, and never doubled on the way through.
    assert_eq!(
        view.self_view.half_extent, real_half,
        "the builder published {:?} for a body whose real half-extent is \
         {real_half:?} — the 2x bug is back",
        view.self_view.half_extent
    );
    // And it is a half of the SIZE, which is the mistake that was made: a fill
    // site passing `size` straight through would produce this value instead.
    assert_ne!(
        view.self_view.half_extent, kin_size,
        "poison: the view is carrying the FULL body size as a half-extent"
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

/// The stage is not viewport-clipped. A fighter can see the blastzones;
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
        Perception::Sighted {
            viewport_half: ae::Vec2::splat(40.0), // a tiny viewport
        },
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
        ..Default::default()
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

/// Two seats on different teams are foes even when they share a faction.
///
/// `damage_lands_between` gives the TEAM relation precedence over factions;
/// perception did not, so it disagreed with the damage rule about who the enemy
/// is. In a free-for-all — every seat its own team, which is what a 4-player
/// smash authors — `faction_for` alternates Player/Enemy by seat index, so seats
/// 0 and 2 share a faction on different teams. They could hit each other and
/// could not SEE each other, and a brain with no perceived foe stands still.
#[test]
fn a_different_team_is_hostile_even_on_the_same_faction() {
    use ambition_combat::targeting::MatchTeam;

    let world = arena_world();
    let relations = FactionRelations::default();

    let mut me = body(ae::Vec2::new(0.0, 0.0), ActorFaction::Player);
    me.team = Some(MatchTeam::new("seat 1"));
    let mut them = peer("them", ae::Vec2::new(50.0, 0.0), ActorFaction::Player);
    them.team = Some(MatchTeam::new("seat 3"));

    let view = build_world_view(
        &me,
        &[them.clone()],
        &[],
        &[],
        &world,
        &relations,
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
        0.0,
    );
    assert!(
        view.actors[0].hostile_to_self,
        "a different team is a foe; the damage rule already says so"
    );

    // And the converse: the same team is NOT, whatever the factions say.
    let mut ally = peer("ally", ae::Vec2::new(50.0, 0.0), ActorFaction::Enemy);
    ally.team = Some(MatchTeam::new("seat 1"));
    let view = build_world_view(
        &me,
        &[ally],
        &[],
        &[],
        &world,
        &relations,
        Perception::Sighted {
            viewport_half: DEFAULT_VIEWPORT_HALF,
        },
        0.0,
    );
    assert!(
        !view.actors[0].hostile_to_self,
        "a teammate is not a target, and reading factions instead would make \
         every 2v2 a free-for-all in the brain's eyes"
    );
}

/// ⛔⛔ **A VIEWER MUST NOT PERCEIVE ITSELF.**
///
/// This is the entire job the deleted `peers_seen_by` was doing, and deleting it
/// is what removed ~16.8k struct-and-`String` clones per tick from the hall.
/// Every viewer now reads the SAME shared snapshot — which contains its own row
/// — and self-exclusion is one comparison against `PerceptionBody::viewer`
/// inside `build_world_view`.
///
/// Get it wrong and a body perceives itself as another actor: `nearest_hostile`
/// can return the viewer, a grudge-holder becomes hostile to itself, and every
/// distance query has a zero in it. Nothing panics. The brains simply act on a
/// world with a duplicate of themselves in it.
mod a_viewer_is_not_its_own_peer {
    use super::*;
    use bevy::prelude::*;

    /// Two REAL entities, because `Entity::PLACEHOLDER` is what every other
    /// fixture in this file uses and a test where viewer and peer are the same
    /// placeholder cannot tell exclusion from an empty list.
    fn two_entities() -> (Entity, Entity) {
        let mut world = World::new();
        (world.spawn_empty().id(), world.spawn_empty().id())
    }

    #[test]
    fn the_viewers_own_row_is_excluded_from_the_shared_snapshot() {
        let (me, other) = two_entities();
        let world = arena_world();
        let relations = FactionRelations::default();

        let mut viewer = body(ae::Vec2::new(100.0, 180.0), ActorFaction::Enemy);
        viewer.viewer = Some(me);

        // The shared snapshot, exactly as `PerceivedWorld::peers()` hands it
        // over: it contains the viewer.
        let mut my_row = peer("me", ae::Vec2::new(100.0, 180.0), ActorFaction::Enemy);
        my_row.entity = me;
        let mut their_row = peer("them", ae::Vec2::new(180.0, 180.0), ActorFaction::Player);
        their_row.entity = other;
        let snapshot = vec![my_row, their_row];

        let view = build_world_view(
            &viewer,
            &snapshot,
            &[],
            &[],
            &world,
            &relations,
            Perception::Omniscient,
            0.0,
        );

        assert_eq!(
            view.actors.len(),
            1,
            "the viewer's own row must not reach the brain; got {:?}",
            view.actors.iter().map(|a| &a.id).collect::<Vec<_>>()
        );
        assert_eq!(view.actors[0].id, "them");
    }

    /// Premise guard: exclusion must remove ONE row, not the population.
    ///
    /// Without this, a filter that dropped everything — or a `viewer` that
    /// matched every peer because they all share `Entity::PLACEHOLDER` — would
    /// pass the arm above.
    #[test]
    fn a_viewer_with_no_row_in_the_snapshot_excludes_nobody() {
        let (me, other) = two_entities();
        let world = arena_world();
        let relations = FactionRelations::default();

        let mut viewer = body(ae::Vec2::new(100.0, 180.0), ActorFaction::Enemy);
        viewer.viewer = Some(me);

        let mut their_row = peer("them", ae::Vec2::new(180.0, 180.0), ActorFaction::Player);
        their_row.entity = other;

        let view = build_world_view(
            &viewer,
            &[their_row],
            &[],
            &[],
            &world,
            &relations,
            Perception::Omniscient,
            0.0,
        );
        assert_eq!(view.actors.len(), 1, "nobody to exclude, so nobody is");
    }
}
