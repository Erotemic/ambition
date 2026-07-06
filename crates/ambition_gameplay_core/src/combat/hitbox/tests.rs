//! Unit tests for hitbox AABB resolution and the despawn/overlap lifecycle.

use super::*;
use bevy::prelude::*;

fn dummy_entity() -> Entity {
    Entity::from_raw_u32(42).expect("nonzero raw entity index")
}

/// FollowOwner anchor re-resolves position each tick: moving
/// the owner moves the hitbox without per-frame component update.
#[test]
fn follow_owner_hitbox_aabb_tracks_owner_position() {
    let hitbox = Hitbox {
        owner: dummy_entity(),
        source: ActorFaction::Enemy,
        anchor: HitboxAnchor::FollowOwner {
            local_offset: ae::Vec2::new(-20.0, 0.0),
        },
        half_extent: ae::Vec2::new(10.0, 10.0),
        shape: None,
        facing: 1.0,
        damage: 1,
        knockback_strength: 0.0,
        knockback_growth: 0.0,
        launch_dir: None,
        knock_x: 0.0,
        frame_down: ae::Vec2::new(0.0, 1.0),
    };
    let aabb_a = hitbox.world_aabb(ae::Vec2::new(100.0, 100.0));
    let aabb_b = hitbox.world_aabb(ae::Vec2::new(200.0, 100.0));
    assert_eq!(aabb_a.center(), ae::Vec2::new(80.0, 100.0));
    assert_eq!(aabb_b.center(), ae::Vec2::new(180.0, 100.0));
    // Half-extent translates into a full-size AABB; the local
    // offset doesn't change shape.
    assert_eq!(aabb_a.half_size(), ae::Vec2::new(10.0, 10.0));
}

/// World anchor is a fixed world rectangle regardless of owner.
#[test]
fn world_anchor_hitbox_ignores_owner_position() {
    let hitbox = Hitbox {
        owner: dummy_entity(),
        source: ActorFaction::Boss,
        anchor: HitboxAnchor::World {
            center: ae::Vec2::new(500.0, 600.0),
        },
        half_extent: ae::Vec2::new(40.0, 40.0),
        shape: None,
        facing: 1.0,
        damage: 1,
        knockback_strength: 0.0,
        knockback_growth: 0.0,
        launch_dir: None,
        knock_x: 0.0,
        frame_down: ae::Vec2::new(0.0, 1.0),
    };
    let aabb_a = hitbox.world_aabb(ae::Vec2::new(0.0, 0.0));
    let aabb_b = hitbox.world_aabb(ae::Vec2::new(9999.0, 9999.0));
    assert_eq!(aabb_a.center(), ae::Vec2::new(500.0, 600.0));
    assert_eq!(aabb_b.center(), ae::Vec2::new(500.0, 600.0));
}

/// `tick_and_despawn_hitboxes` advances `remaining_s` by
/// `world_time.sim_dt()` and despawns when it hits zero. A
/// short-lifetime hitbox should not survive a single tick at
/// the default 1/60s sim dt.
fn make_app_with_sim_dt(sim_dt: f32) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<WorldTime>();
    // WorldTime::default() leaves scaled_dt = 0, which would
    // freeze every gameplay timer; bump it so the despawn
    // assertions actually advance the lifetime.
    let mut world_time = app.world_mut().resource_mut::<WorldTime>();
    world_time.scaled_dt = sim_dt;
    world_time.raw_dt = sim_dt;
    app
}

#[test]
fn tick_and_despawn_drops_expired_hitboxes() {
    let mut app = make_app_with_sim_dt(0.05);
    app.add_systems(Update, tick_and_despawn_hitboxes);
    let hitbox = app
        .world_mut()
        .spawn((
            Hitbox {
                owner: dummy_entity(),
                source: ActorFaction::Enemy,
                anchor: HitboxAnchor::FollowOwner {
                    local_offset: ae::Vec2::ZERO,
                },
                half_extent: ae::Vec2::new(10.0, 10.0),
                shape: None,
                facing: 1.0,
                damage: 1,
                knockback_strength: 0.0,
                knockback_growth: 0.0,
                launch_dir: None,
                knock_x: 0.0,
                frame_down: ae::Vec2::new(0.0, 1.0),
            },
            HitboxLifetime { remaining_s: 0.01 },
            HitboxHits::default(),
        ))
        .id();
    // 50ms sim_dt burns through the 10ms lifetime in one tick.
    app.update();
    assert!(
        app.world().get_entity(hitbox).is_err(),
        "hitbox entity should be despawned after lifetime expired",
    );
}

/// A hitbox with `remaining_s` larger than one tick should
/// stay alive after a single update.
#[test]
fn tick_and_despawn_keeps_live_hitboxes() {
    let mut app = make_app_with_sim_dt(0.05);
    app.add_systems(Update, tick_and_despawn_hitboxes);
    let hitbox = app
        .world_mut()
        .spawn((
            Hitbox {
                owner: dummy_entity(),
                source: ActorFaction::Enemy,
                anchor: HitboxAnchor::FollowOwner {
                    local_offset: ae::Vec2::ZERO,
                },
                half_extent: ae::Vec2::new(10.0, 10.0),
                shape: None,
                facing: 1.0,
                damage: 1,
                knockback_strength: 0.0,
                knockback_growth: 0.0,
                launch_dir: None,
                knock_x: 0.0,
                frame_down: ae::Vec2::new(0.0, 1.0),
            },
            HitboxLifetime { remaining_s: 5.0 },
            HitboxHits::default(),
        ))
        .id();
    app.update();
    assert!(
        app.world().get_entity(hitbox).is_ok(),
        "hitbox with multi-second lifetime should survive a single tick",
    );
}

/// `spawn_melee_hitbox` populates a freshly-spawned entity with
/// the three expected components — pinned so a future
/// `Bundle`-ification of the spawn doesn't drop the `HitboxHits`
/// hit-once tracker by accident.
///
/// The helper takes `&mut Commands`; drive it through a one-off
/// system so Bevy provides a real Commands handle (and flushes
/// the queue automatically when the system finishes).
#[test]
fn spawn_melee_hitbox_attaches_full_component_set() {
    #[derive(Resource, Default)]
    struct SpawnedHitbox(Option<Entity>);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<SpawnedHitbox>();
    let owner = dummy_entity();
    app.add_systems(
        Update,
        move |mut commands: Commands, mut store: ResMut<SpawnedHitbox>| {
            if store.0.is_some() {
                return;
            }
            let entity = spawn_melee_hitbox(
                &mut commands,
                owner,
                ActorFaction::Enemy,
                ae::Vec2::new(-20.0, 0.0),
                ae::Vec2::new(20.0, 14.0),
                3,
                1.5,
                0.0,
                0.42,
                ae::Vec2::new(0.0, 1.0),
            );
            store.0 = Some(entity);
        },
    );
    app.update();
    let spawned = app
        .world()
        .resource::<SpawnedHitbox>()
        .0
        .expect("spawn_melee_hitbox should return an Entity");
    let entity = app.world().entity(spawned);
    let hitbox = entity.get::<Hitbox>().expect("Hitbox missing");
    assert_eq!(hitbox.damage, 3);
    assert!((hitbox.knockback_strength - 1.5).abs() < f32::EPSILON);
    match hitbox.anchor {
        HitboxAnchor::FollowOwner { local_offset } => {
            assert_eq!(local_offset, ae::Vec2::new(-20.0, 0.0));
        }
        _ => panic!("expected FollowOwner anchor"),
    }
    let lifetime = entity.get::<HitboxLifetime>().expect("Lifetime missing");
    assert!((lifetime.remaining_s - 0.42).abs() < f32::EPSILON);
    assert!(
        entity.get::<HitboxHits>().is_some(),
        "HitboxHits hit-once tracker should be attached by default",
    );
}

#[derive(Resource, Default)]
struct CapturedHits(Vec<HitEvent>);

fn capture_hits(mut reader: MessageReader<HitEvent>, mut cap: ResMut<CapturedHits>) {
    for e in reader.read() {
        cap.0.push(e.clone());
    }
}

/// The unification keystone: a **Player-faction** hitbox (a wielded boss
/// AOE) emits exactly one attacker-side Volume `HitEvent` that
/// `apply_feature_hit_events` then resolves against enemies/bosses — the
/// same primitive a Boss-faction hitbox uses to hit the player.
#[test]
fn player_faction_hitbox_emits_an_attacker_side_feature_hit() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());
    let owner = app
        .world_mut()
        .spawn(ae::CenteredAabb::new(
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(12.0, 16.0),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            owner,
            source: ActorFaction::Player,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::new(200.0, 80.0),
            },
            half_extent: ae::Vec2::new(60.0, 30.0),
            shape: None,
            facing: 1.0,
            damage: 5,
            knockback_strength: 1.0,
            knockback_growth: 0.0,
            launch_dir: None,
            knock_x: 0.0,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
    app.update();
    let cap = app.world().resource::<CapturedHits>();
    assert_eq!(
        cap.0.len(),
        1,
        "player AOE emits exactly one feature-damaging hit"
    );
    assert!(
        matches!(cap.0[0].source, HitSource::PlayerSlash { .. }),
        "carries an attacker-side player source so apply_feature_hit_events applies it"
    );
    assert!(cap.0[0].source.is_attacker_side());
    assert!(
        matches!(cap.0[0].target, HitTarget::Volume),
        "volume hit (every overlapping actor/boss)"
    );
    assert_eq!(cap.0[0].damage, 5);
}

// ── S3e: relational actor-vs-actor melee ────────────────────────────────────

use crate::features::FactionRelations;

/// Spawn an Enemy-source hitbox at `center` (World anchor) dealing `damage`, plus
/// an actor victim of `victim_faction` overlapping it. Returns (app, victim).
fn arena_hitbox_app(relations: FactionRelations, victim_faction: ActorFaction) -> (App, Entity) {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.init_resource::<CapturedHits>();
    app.insert_resource(relations);
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());
    let owner = app
        .world_mut()
        .spawn(ae::CenteredAabb::new(
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(12.0, 16.0),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            owner,
            source: ActorFaction::Enemy,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::new(100.0, 100.0),
            },
            half_extent: ae::Vec2::new(30.0, 30.0),
            shape: None,
            facing: 1.0,
            damage: 4,
            knockback_strength: 0.0,
            knockback_growth: 0.0,
            launch_dir: None,
            knock_x: 0.0,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
    let victim = app
        .world_mut()
        .spawn((
            ae::CenteredAabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(14.0, 20.0)),
            victim_faction,
            // Every body carries the vulnerability trio (§A1 slice 3) — the
            // victim query is no longer `Option` over them.
            crate::actor::BodyOffense::default(),
            crate::actor::BodyDodgeState::default(),
            crate::actor::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
        ))
        .id();
    (app, victim)
}

/// An Enemy swing damages a Boss-faction body when the relations matrix marks
/// them mutually hostile (a spectator arena). The hit is PRE-RESOLVED to that
/// exact body via `HitTarget::Actor`, so the actor-damage consumer lands it
/// without the bipartite player/enemy assumption.
#[test]
fn enemy_hitbox_damages_a_relationally_hostile_actor() {
    let mut relations = FactionRelations::default();
    relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
    let (mut app, victim) = arena_hitbox_app(relations, ActorFaction::Boss);
    app.update();
    let cap = &app.world().resource::<CapturedHits>().0;
    assert_eq!(cap.len(), 1, "one relational actor-vs-actor hit");
    assert_eq!(
        cap[0].target,
        HitTarget::Actor(victim),
        "pre-resolved to the hostile body"
    );
    assert!(matches!(cap[0].source, HitSource::EnemyAttack));
    assert_eq!(cap[0].damage, 4);
}

/// Same-faction actors don't fight: an Enemy swing does not hit another Enemy
/// even with the arena relation set (it only adds Enemy ↔ Boss).
#[test]
fn enemy_hitbox_ignores_a_same_faction_actor() {
    let mut relations = FactionRelations::default();
    relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Boss, true);
    let (mut app, _victim) = arena_hitbox_app(relations, ActorFaction::Enemy);
    app.update();
    assert!(
        app.world().resource::<CapturedHits>().0.is_empty(),
        "no friendly fire — an Enemy is not hostile to another Enemy"
    );
}

/// Damage is PHYSICAL, not relational: an Enemy swing damages a DIFFERENT-faction
/// (Boss) body even with default relations — no targeting hostility required.
/// Targeting (who a brain aims at) is relational; a hit that LANDS deals damage to
/// any non-ally. (Friendly fire is off by default, so a SAME-faction body is spared
/// — see `enemy_hitbox_ignores_a_same_faction_actor`.)
#[test]
fn actor_vs_actor_damage_is_physical_for_different_factions() {
    let (mut app, victim) = arena_hitbox_app(FactionRelations::default(), ActorFaction::Boss);
    app.update();
    let cap = &app.world().resource::<CapturedHits>().0;
    assert_eq!(
        cap.len(),
        1,
        "a different-faction body is hit regardless of relations (physical damage)"
    );
    assert_eq!(cap[0].target, HitTarget::Actor(victim));
}

/// Spawn an Enemy-source hitbox over a vulnerable player; relations decide
/// whether the player is hit. Returns (app, player).
fn enemy_hitbox_over_player_app(relations: FactionRelations) -> (App, Entity) {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.init_resource::<CapturedHits>();
    app.insert_resource(relations);
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());
    let owner = app
        .world_mut()
        .spawn(ae::CenteredAabb::new(
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(12.0, 16.0),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            owner,
            source: ActorFaction::Enemy,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::new(100.0, 100.0),
            },
            half_extent: ae::Vec2::new(30.0, 30.0),
            shape: None,
            facing: 1.0,
            damage: 3,
            knockback_strength: 0.0,
            knockback_growth: 0.0,
            launch_dir: None,
            knock_x: 0.0,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
    let player = app
        .world_mut()
        .spawn((
            crate::actor::PlayerEntity,
            ActorFaction::Player,
            crate::actor::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
            // The published combat footprint every body carries (§A6).
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(28.0, 46.0),
            ),
            crate::actor::BodyOffense::default(),
            crate::actor::BodyDodgeState::default(),
            crate::actor::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
        ))
        .id();
    (app, player)
}

/// Default (combat-baseline) relations keep Enemy hostile to Player, so an enemy
/// swing over the player lands — ordinary play is unchanged.
#[test]
fn enemy_hitbox_hits_the_player_by_default() {
    let (mut app, player) = enemy_hitbox_over_player_app(FactionRelations::default());
    app.update();
    let cap = &app.world().resource::<CapturedHits>().0;
    assert_eq!(cap.len(), 1, "the player takes the hit by default");
    assert_eq!(cap[0].target, HitTarget::Player(player));
    assert!(matches!(cap[0].source, HitSource::EnemyAttack));
}

/// Damage is physical, so an Enemy swing that OVERLAPS the player hits them even
/// when the Enemy is NOT hostile to Player (a duel combatant whose stray catches
/// the observer). Sparing the observer is a TARGETING property (the duelist won't
/// AIM at them), NOT a damage one — clearing hostility no longer makes the player
/// damage-immune. The player is only spared by friendly fire (same faction) or by
/// being out of range.
#[test]
fn enemy_hitbox_hits_a_non_targeted_player_strays_are_physical() {
    let mut relations = FactionRelations::default();
    relations.set_mutual_hostile(ActorFaction::Enemy, ActorFaction::Player, false);
    let (mut app, player) = enemy_hitbox_over_player_app(relations);
    app.update();
    let cap = &app.world().resource::<CapturedHits>().0;
    assert_eq!(
        cap.len(),
        1,
        "a cross-faction swing over the player lands even with no targeting hostility"
    );
    assert_eq!(cap[0].target, HitTarget::Player(player));
}

/// The AOE fires once, not every tick of its lifetime — the owner doubles
/// as a fired-sentinel in `HitboxHits`.
#[test]
fn player_faction_hitbox_only_fires_once() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());
    let owner = app
        .world_mut()
        .spawn(ae::CenteredAabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)))
        .id();
    app.world_mut().spawn((
        Hitbox {
            owner,
            source: ActorFaction::Player,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::ZERO,
            },
            half_extent: ae::Vec2::splat(40.0),
            shape: None,
            facing: 1.0,
            damage: 3,
            knockback_strength: 0.0,
            knockback_growth: 0.0,
            launch_dir: None,
            knock_x: 0.0,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 1.0 },
        HitboxHits::default(),
    ));
    app.update();
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<CapturedHits>().0.len(),
        1,
        "the AOE emits its hit once across multiple live ticks"
    );
}

/// The unified player MELEE strike: a Player-faction FollowOwner hitbox owned by
/// a body that has NO `CenteredAabb` (the player carries `BodyKinematics`) emits a
/// `PlayerSlash` Volume hit each active tick, carrying the strike's signed
/// `knock_x`, gated on the owner having an armed `MeleeSwing`. This is the path
/// `advance_attack` now spawns through (replacing the per-frame Volume emit) — it
/// pins owner-pos-via-kinematics + knock_x carriage + the swing gate.
#[test]
fn player_followowner_melee_strike_emits_player_slash_with_knock_x() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());

    let view = crate::combat::AttackView {
        pos: ae::Vec2::new(100.0, 100.0),
        size: ae::Vec2::new(20.0, 40.0),
        facing: 1.0,
        on_ground: true,
        wall_clinging: false,
        dash_timer: 0.0,
        abilities_directional_primary: true,
    };
    let spec = crate::combat::attack_spec_from_view(&view, crate::combat::AttackIntent::Forward);

    // Owner = a player-like body: BodyKinematics (NOT CenteredAabb) + armed swing.
    let owner = app
        .world_mut()
        .spawn((
            crate::actor::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                ..Default::default()
            },
            crate::features::BodyMelee {
                swing: Some(crate::features::MeleeSwing::new(spec)),
                ..Default::default()
            },
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            owner,
            source: ActorFaction::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::new(30.0, 0.0),
            },
            half_extent: ae::Vec2::new(20.0, 20.0),
            shape: None,
            facing: 1.0,
            damage: 4,
            knockback_strength: 0.0,
            knockback_growth: 0.0,
            launch_dir: None,
            knock_x: 250.0,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));

    app.update();
    let cap = app.world().resource::<CapturedHits>();
    assert_eq!(cap.0.len(), 1, "the player FollowOwner strike emits a hit");
    assert!(
        matches!(cap.0[0].source, HitSource::PlayerSlash { knock_x } if (knock_x - 250.0).abs() < 0.01),
        "carries the strike's signed knock_x (was {:?})",
        cap.0[0].source,
    );
    assert!(matches!(cap.0[0].target, HitTarget::Volume));
    assert_eq!(cap.0[0].damage, 4);
}

/// No armed swing on the owner ⇒ a Player FollowOwner hitbox deals NO damage (the
/// swing is the strike's authority; a stray hitbox with no swing is inert).
#[test]
fn player_followowner_strike_without_a_swing_is_inert() {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, (apply_hitbox_damage, capture_hits).chain());
    let owner = app
        .world_mut()
        .spawn((
            crate::actor::BodyKinematics::default(),
            crate::features::BodyMelee::default(), // swing = None
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            owner,
            source: ActorFaction::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::ZERO,
            },
            half_extent: ae::Vec2::splat(20.0),
            shape: None,
            facing: 1.0,
            damage: 4,
            knockback_strength: 0.0,
            knockback_growth: 0.0,
            launch_dir: None,
            knock_x: 250.0,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
    app.update();
    assert_eq!(
        app.world().resource::<CapturedHits>().0.len(),
        0,
        "a Player FollowOwner hitbox with no armed swing emits nothing"
    );
}
