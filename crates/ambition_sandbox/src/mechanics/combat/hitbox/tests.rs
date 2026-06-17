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
        damage: 1,
        knockback_strength: 0.0,
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
        damage: 1,
        knockback_strength: 0.0,
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
                damage: 1,
                knockback_strength: 0.0,
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
                damage: 1,
                knockback_strength: 0.0,
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
                0.42,
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
        .spawn(crate::features::CenteredAabb::new(
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
            damage: 5,
            knockback_strength: 1.0,
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
        .spawn(crate::features::CenteredAabb::new(
            ae::Vec2::ZERO,
            ae::Vec2::splat(8.0),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            owner,
            source: ActorFaction::Player,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::ZERO,
            },
            half_extent: ae::Vec2::splat(40.0),
            damage: 3,
            knockback_strength: 0.0,
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
