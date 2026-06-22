//! Tests for hit-event application to ECS actors/bosses/breakables and the
//! death-driven drop/explosion/split spawners.

use super::super::damage_drops::{
    drop_ability_pickup, drop_health_pickup, id_drops_health, spawn_death_explosion,
    spawn_split_offspring,
};
use super::*;
use crate::engine_core as ae;
use crate::features::ecs::enemy_component_snapshot;
use crate::features::{HitMode, HitTarget};
use bevy::prelude::{App, Update};

fn spawn_hostile_actor(app: &mut App) -> bevy::prelude::Entity {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut enemy = crate::features::ecs::enemy_clusters::EnemyClusterSeed::new(
        "kernel_guide".to_string(),
        "Kernel Guide".to_string(),
        aabb,
        crate::actor::EnemyBrain::Custom("medium_striker".into()),
        &[],
    );
    enemy.status.health = crate::actor::Health::new(5);
    let (identity, disposition, health, combat, intent, cooldowns) =
        enemy_component_snapshot(&enemy);
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("kernel_guide"),
            CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
            enemy.into_components(),
            identity,
            disposition,
            health,
            combat,
            intent,
            cooldowns,
        ))
        .id()
}

#[test]
fn victim_side_enemy_body_hit_does_not_damage_features() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app);
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    app.world_mut().write_message(HitEvent {
        volume: event_volume,
        damage: 1,
        source: HitSource::EnemyBody,
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });

    app.update();

    let health = app
        .world()
        .get::<ActorHealth>(actor_entity)
        .expect("hostile actor exists");
    assert_eq!(
        health.health.current, 5,
        "enemy body contact should not damage the enemy that emitted it"
    );
}

#[test]
fn enemy_charge_crash_is_processed_as_enemy_damage() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app);
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    app.world_mut().write_message(HitEvent {
        volume: event_volume,
        damage: 10,
        source: HitSource::EnemyChargeCrash,
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });

    app.update();

    let health = app
        .world()
        .get::<ActorHealth>(actor_entity)
        .expect("hostile actor exists");
    assert_eq!(
        health.health.current, 0,
        "enemy charge crash should damage and kill the crashing enemy"
    );
    let status = app
        .world()
        .get::<super::super::enemy_clusters::EnemyStatus>(actor_entity)
        .expect("hostile actor cluster status exists");
    assert!(
        !status.alive,
        "charge crash should mark the enemy dead through the normal kill path"
    );
}

#[test]
fn player_slash_damages_and_can_kill_a_hostile_actor() {
    // The core attack loop through the unified HitEvent path: a
    // player slash (attacker-side source) reduces a hostile
    // actor's HP, and enough damage routes through the normal kill
    // path. Complements the enemy-side tests above.
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app); // HP 5
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));

    // First slash: 2 damage → 3 HP, still alive.
    app.world_mut().write_message(HitEvent {
        volume: event_volume,
        damage: 2,
        source: HitSource::PlayerSlash { knock_x: 120.0 },
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<ActorHealth>(actor_entity)
            .unwrap()
            .health
            .current,
        3,
        "a 2-damage player slash should bring the 5-HP enemy to 3"
    );
    assert!(
        app.world()
            .get::<super::super::enemy_clusters::EnemyStatus>(actor_entity)
            .unwrap()
            .alive,
        "the enemy should still be alive after one slash"
    );

    // Lethal slash: 5 damage → dead through the normal kill path.
    app.world_mut().write_message(HitEvent {
        volume: event_volume,
        damage: 5,
        source: HitSource::PlayerSlash { knock_x: 120.0 },
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<ActorHealth>(actor_entity)
            .unwrap()
            .health
            .current,
        0,
        "a lethal slash should bring the enemy to 0 HP"
    );
    assert!(
        !app.world()
            .get::<super::super::enemy_clusters::EnemyStatus>(actor_entity)
            .unwrap()
            .alive,
        "the killed enemy should be marked dead"
    );
}

/// Shared setup for the cling-break tests: spawn a hostile actor, make it a
/// surface-walker clung to a LEFT wall (outward normal +x), then slash it.
fn slash_clung_surface_walker(cling_breaks_on_hit: bool) -> (App, bevy::prelude::Entity) {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let actor = spawn_hostile_actor(&mut app); // HP 5 — survives one slash
    {
        let mut cfg = app
            .world_mut()
            .get_mut::<super::super::enemy_clusters::EnemyConfig>(actor)
            .unwrap();
        cfg.tuning.surface_walker = true;
        cfg.tuning.cling_breaks_on_hit = cling_breaks_on_hit;
    }
    {
        let mut surf = app
            .world_mut()
            .get_mut::<crate::features::ActorSurfaceState>(actor)
            .unwrap();
        surf.on_ground = true;
        surf.surface_normal = ae::Vec2::new(1.0, 0.0);
    }
    app.world_mut().write_message(HitEvent {
        volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)),
        damage: 1,
        source: HitSource::PlayerSlash { knock_x: 0.0 },
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();
    (app, actor)
}

#[test]
fn struck_cling_breaker_loses_its_surface_and_falls() {
    // The puppy-slug "panic on hit": a struck surface-walker authored
    // `cling_breaks_on_hit` leaves its surface and peels away along that surface
    // normal. It keeps the last contact normal while airborne; the surface-walk
    // integration reorients it to the active acceleration frame when it next lands.
    let (app, actor) = slash_clung_surface_walker(true);
    let surf = app
        .world()
        .get::<crate::features::ActorSurfaceState>(actor)
        .unwrap();
    assert!(
        !surf.on_ground,
        "a struck cling-breaker should leave its surface and fall"
    );
    assert_eq!(
        surf.surface_normal,
        ae::Vec2::new(1.0, 0.0),
        "detaching preserves the last contact normal until gravity-relative landing"
    );
    let kin = app
        .world()
        .get::<super::super::enemy_clusters::BodyKinematics>(actor)
        .unwrap();
    assert!(
        kin.vel.x > 0.0,
        "it peels away along the +x wall normal, got vel {:?}",
        kin.vel
    );
}

#[test]
fn struck_surface_walker_holds_on_when_cling_does_not_break() {
    // Crawlers authored `cling_breaks_on_hit: false` keep clinging when struck —
    // their surface state is untouched by the hit.
    let (app, actor) = slash_clung_surface_walker(false);
    let surf = app
        .world()
        .get::<crate::features::ActorSurfaceState>(actor)
        .unwrap();
    assert!(surf.on_ground, "a non-breaking crawler keeps its footing");
    assert_eq!(
        surf.surface_normal,
        ae::Vec2::new(1.0, 0.0),
        "and stays oriented to its wall"
    );
}

#[test]
fn player_slash_shatters_a_breakable() {
    // Completes the attacker-side hit matrix: a player slash on a
    // 1-HP breakable shatters it through apply_feature_hit_events.
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(20.0, 20.0));
    let breakable = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("crate"),
            FeatureName::new("crate"),
            CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
            BreakableFeature::new(crate::interaction::Breakable::new("crate", 1)),
        ))
        .id();
    assert!(!app
        .world()
        .get::<BreakableFeature>(breakable)
        .unwrap()
        .broken());

    app.world_mut().write_message(HitEvent {
        volume: aabb,
        damage: 2,
        source: HitSource::PlayerSlash { knock_x: 0.0 },
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();

    assert!(
        app.world()
            .get::<BreakableFeature>(breakable)
            .unwrap()
            .broken(),
        "a player slash should shatter a 1-HP breakable"
    );

    // Shattering a crate drops one collectible coin.
    let mut q = app.world_mut().query::<&PickupFeature>();
    let coins = q
        .iter(app.world())
        .filter(|p| matches!(p.kind(), crate::interaction::PickupKind::Currency { .. }))
        .count();
    assert_eq!(coins, 1, "shattering a crate drops one coin");
}

#[test]
fn enemy_defeat_drops_a_collectible_currency_coin() {
    let mut app = App::new();
    app.add_systems(Update, |mut c: Commands| {
        drop_currency_coin(&mut c, "goblin_1", ae::Vec2::new(40.0, 50.0), ENEMY_BOUNTY);
    });
    app.update();
    let mut q = app.world_mut().query::<(&PickupFeature, &FeatureId)>();
    let rows: Vec<(crate::interaction::PickupKind, String)> = q
        .iter(app.world())
        .map(|(p, id)| (p.kind().clone(), id.as_str().to_string()))
        .collect();
    assert_eq!(rows.len(), 1, "exactly one coin dropped");
    assert_eq!(rows[0].1, "coin:goblin_1", "coin id is keyed to the enemy");
    assert_eq!(
        rows[0].0,
        crate::interaction::PickupKind::Currency {
            amount: ENEMY_BOUNTY
        },
        "the drop is a currency coin worth the bounty",
    );
}

#[test]
fn defeated_boss_drops_its_signature_ability() {
    use crate::features::BossBehaviorProfile;
    // Each boss's reward ability is content data (`boss_profiles.ron`):
    // verify the authored pairings and that each resolves to a real catalog
    // item. Read off the RON-loaded profile by id — the engine names none.
    let expect: &[(&str, Option<&str>)] = &[
        ("flying_spaghetti_monster_boss", Some("blink")),
        ("trex_boss", Some("grapple")),
        ("gnu_ton", Some("fireball")),
        ("clockwork_warden", Some("markrecall")),
        ("mockingbird", None),
        ("smirking_behemoth_boss", None),
    ];
    for (id, ability) in expect {
        let profile = BossBehaviorProfile::from_data(id);
        assert_eq!(
            profile.reward_ability.as_deref(),
            *ability,
            "{id} reward ability drifted from boss_profiles.ron",
        );
        if let Some(a) = ability {
            assert!(
                crate::items::Item::from_dialog_id(a).is_some(),
                "boss {id} -> ability {a} must be a real catalog item",
            );
        }
    }

    // The drop spawns a single collectible Ability pickup.
    let mut app = App::new();
    app.add_systems(Update, |mut c: Commands| {
        drop_ability_pickup(
            &mut c,
            "trex_boss",
            ae::Vec2::new(10.0, 20.0),
            "grapple",
            "Grapple",
        );
    });
    app.update();
    let mut q = app.world_mut().query::<&PickupFeature>();
    let kinds: Vec<crate::interaction::PickupKind> =
        q.iter(app.world()).map(|p| p.kind().clone()).collect();
    assert_eq!(kinds.len(), 1, "one ability pickup dropped");
    assert_eq!(
        kinds[0],
        crate::interaction::PickupKind::Ability {
            ability_id: "grapple".to_string()
        },
    );
}

#[test]
fn boss_signature_gauntlets_map_to_real_wielded_held_items() {
    use crate::abilities::ranged::{beam, meteor, sentry, shockwave, volley, vortex};
    use crate::abilities::traversal::dive;
    use crate::features::BossBehaviorProfile;
    // Signature gauntlets are content data (`boss_profiles.ron`): each must
    // resolve to a real held-item spec so the dropped GroundItem is
    // pick-up-able. Read off the RON-loaded profile by id — so the reward is
    // intrinsically keyed on the boss's REAL behavior id (the old
    // `"smirking_behemoth"` vs `"smirking_behemoth_boss"` mis-key, where a
    // literal-keyed lookup silently never fired, can no longer happen). The
    // expected values pin the RON against the ability id consts so the two
    // can't drift apart.
    let expect: &[(&str, Option<&str>)] = &[
        ("trex_boss", Some(shockwave::SHOCKWAVE_ID)),
        ("mockingbird", Some(volley::VOLLEY_ID)),
        ("smirking_behemoth_boss", Some(beam::BEAM_ID)),
        ("mode_collapse_boss", Some(vortex::VORTEX_ID)),
        ("exploding_gradient_boss", Some(sentry::SENTRY_ID)),
        ("overflow_boss", Some(dive::DIVE_ID)),
        ("gnu_ton", Some(meteor::METEOR_ID)),
        ("clockwork_warden", None),
        ("flying_spaghetti_monster_boss", None),
    ];
    let mut gauntlets = 0;
    let mut abilities = 0;
    for (id, gauntlet) in expect {
        let profile = BossBehaviorProfile::from_data(id);
        assert_eq!(
            profile.signature_gauntlet.as_deref(),
            *gauntlet,
            "{id} signature gauntlet drifted from boss_profiles.ron",
        );
        if let Some(g) = profile.signature_gauntlet.as_deref() {
            assert!(
                crate::brain::held_item_by_id(g).is_some(),
                "boss {id} -> gauntlet {g} must be a registered held item",
            );
            gauntlets += 1;
        }
        if profile.reward_ability.is_some() {
            abilities += 1;
        }
    }
    // trex + mockingbird + smirking + mode_collapse + exploding_gradient +
    // overflow + gnu_ton each arm a wielded gauntlet (seven "learn its
    // attack" drops; trex and gnu_ton also grant a catalog ability).
    assert_eq!(gauntlets, 7, "seven bosses drop a signature gauntlet");
    // FSM(blink) + trex(grapple) + gnu(fireball) + clockwork(markrecall).
    assert_eq!(abilities, 4, "four bosses grant a catalog ability");
}

#[test]
fn exploding_mite_blast_is_a_player_damaging_enemy_hitbox() {
    let mut app = App::new();
    app.add_systems(Update, |mut c: Commands| {
        spawn_death_explosion(&mut c, Entity::PLACEHOLDER, ae::Vec2::new(50.0, 60.0));
    });
    app.update();
    let mut q = app.world_mut().query::<&crate::features::Hitbox>();
    let boxes: Vec<crate::features::Hitbox> = q.iter(app.world()).cloned().collect();
    assert_eq!(boxes.len(), 1, "the mite's death spawns one blast hitbox");
    assert_eq!(
        boxes[0].source,
        crate::features::ActorFaction::Enemy,
        "enemy-faction → the blast damages the player, not other mites (no chain)",
    );
    assert_eq!(boxes[0].damage, EXPLODER_BLAST_DAMAGE);
    if let crate::features::HitboxAnchor::World { center } = boxes[0].anchor {
        assert_eq!(
            center,
            ae::Vec2::new(50.0, 60.0),
            "the blast centers on the corpse"
        );
    } else {
        panic!("the blast should be world-anchored at the death site");
    }
}

#[test]
fn dividing_mite_splits_into_two_hostile_offspring_on_death() {
    let mut app = App::new();
    app.add_systems(Update, |mut c: Commands| {
        spawn_split_offspring(&mut c, "divider_1", ae::Vec2::new(100.0, 100.0));
    });
    app.update();
    let mut q = app.world_mut().query::<&crate::features::ActorFaction>();
    let factions: Vec<crate::features::ActorFaction> = q.iter(app.world()).cloned().collect();
    assert_eq!(
        factions.len(),
        2,
        "a dividing mite splits into exactly two offspring"
    );
    assert!(
        factions
            .iter()
            .all(|f| *f == crate::features::ActorFaction::Enemy),
        "the offspring are hostile (Enemy faction), not player-allies",
    );
}

#[test]
fn enemy_health_drop_is_deterministic_and_spawns_a_heart() {
    // The gate is a pure function of the id, so the headless sim is reproducible.
    assert_eq!(id_drops_health("goblin_42"), id_drops_health("goblin_42"));
    // The drop spawns one collectible Health pickup.
    let mut app = App::new();
    app.add_systems(Update, |mut c: Commands| {
        drop_health_pickup(&mut c, "any", ae::Vec2::ZERO, ENEMY_HEALTH_DROP);
    });
    app.update();
    let mut q = app.world_mut().query::<&PickupFeature>();
    let kinds: Vec<crate::interaction::PickupKind> =
        q.iter(app.world()).map(|p| p.kind().clone()).collect();
    assert_eq!(kinds.len(), 1, "one heart dropped");
    assert!(
        matches!(kinds[0], crate::interaction::PickupKind::Health { .. }),
        "the drop is a health pickup",
    );
}

#[test]
fn an_armed_enemy_archetype_resolves_a_weapon_to_drop() {
    // The defeat branch's weapon drop keys off `held_item_spec()`; the pirate
    // carries a gun-sword, so a defeated pirate drops one.
    let spec = crate::features::enemies::test_spec("pirate_on_shark").held_item_spec();
    assert!(spec.is_some(), "PirateOnShark carries a weapon");
    assert_eq!(spec.unwrap().id.as_str(), "gun_sword");
}
