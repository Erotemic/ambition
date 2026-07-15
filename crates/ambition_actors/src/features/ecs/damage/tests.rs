//! Tests for hit-event application to ECS actors/bosses/breakables and the
//! death-driven drop/explosion/split spawners.

use super::super::damage_drops::{
    drop_ability_pickup, drop_health_pickup, id_drops_health, spawn_death_explosion,
    spawn_split_offspring,
};
use super::*;
use crate::features::ecs::enemy_component_snapshot;
use crate::features::{HitMode, HitTarget};
use ambition_characters::actor::BodyHealth;
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use bevy::prelude::{App, Update};

fn spawn_hostile_actor(app: &mut App) -> bevy::prelude::Entity {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut enemy = crate::features::ecs::actor_clusters::ActorClusterSeed::new(
        "kernel_guide".to_string(),
        "Kernel Guide".to_string(),
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom("medium_striker".into()),
        &[],
    );
    enemy.health =
        ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(5));
    let (identity, disposition, combat, intent, cooldowns) = enemy_component_snapshot(&enemy);
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("kernel_guide"),
            CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
            enemy.into_components(),
            crate::features::MotionModel::default(),
            identity,
            disposition,
            combat,
            intent,
            cooldowns,
        ))
        .id()
}

#[test]
fn victim_side_enemy_body_hit_does_not_damage_features() {
    let mut app = App::new();
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app);
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    app.world_mut().write_message(HitEvent {
        volume: event_volume.into(),
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
        .get::<BodyHealth>(actor_entity)
        .expect("hostile actor exists");
    assert_eq!(
        health.health.current, 5,
        "enemy body contact should not damage the enemy that emitted it"
    );
}

#[test]
fn enemy_charge_crash_is_processed_as_enemy_damage() {
    let mut app = App::new();
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app);
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    app.world_mut().write_message(HitEvent {
        volume: event_volume.into(),
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
        .get::<BodyHealth>(actor_entity)
        .expect("hostile actor exists");
    assert_eq!(
        health.health.current, 0,
        "enemy charge crash should damage and kill the crashing enemy"
    );
    let health = app
        .world()
        .get::<ambition_characters::actor::BodyHealth>(actor_entity)
        .expect("hostile actor cluster health exists");
    assert!(
        !health.alive(),
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
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app); // HP 5
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));

    // First slash: 2 damage → 3 HP, still alive.
    app.world_mut().write_message(HitEvent {
        volume: event_volume.into(),
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
            .get::<BodyHealth>(actor_entity)
            .unwrap()
            .health
            .current,
        3,
        "a 2-damage player slash should bring the 5-HP enemy to 3"
    );
    assert!(
        app.world().get::<BodyHealth>(actor_entity).unwrap().alive(),
        "the enemy should still be alive after one slash"
    );

    // Two DISTINCT slashes: in real play ~0.2 s+ separates them, so the actor's
    // post-hit i-frame (`ACTOR_DAMAGE_IFRAME_S`) has elapsed by the second swing.
    // This minimal app runs no integration tick to decay it, so clear it here to
    // model the gap between attacks (without it, the i-frame correctly gates the
    // back-to-back second hit — that is the regression fix working).
    app.world_mut()
        .get_mut::<ambition_characters::actor::BodyCombat>(actor_entity)
        .unwrap()
        .damage_invuln_timer = 0.0;

    // Lethal slash: 5 damage → dead through the normal kill path.
    app.world_mut().write_message(HitEvent {
        volume: event_volume.into(),
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
            .get::<BodyHealth>(actor_entity)
            .unwrap()
            .health
            .current,
        0,
        "a lethal slash should bring the enemy to 0 HP"
    );
    assert!(
        !app.world().get::<BodyHealth>(actor_entity).unwrap().alive(),
        "the killed enemy should be marked dead"
    );
}

#[test]
fn a_sustained_overlap_lands_one_hit_per_iframe_window_not_one_per_frame() {
    // Regression (Jon, 2026-06-27): a body pinned in a damaging volume — a lingering
    // attack volume, body contact, or a dialog-locked body next to an enemy — used to
    // re-register a hit (damage + sound + particles) EVERY frame, because actors had
    // no post-hit i-frame (the player did). With the body-generic
    // `ActorStatus::damage_invuln_timer`, the SAME hit fired twice with the window
    // still hot lands exactly once. (This minimal app runs no integration tick, so the
    // window never decays between the two updates — exactly the sustained-overlap case.)
    let mut app = App::new();
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let actor_entity = spawn_hostile_actor(&mut app); // HP 5
    let event_volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let slash = || HitEvent {
        volume: event_volume.into(),
        damage: 2,
        source: HitSource::PlayerSlash { knock_x: 120.0 },
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    };

    app.world_mut().write_message(slash());
    app.update();
    let hp_after_first = app
        .world()
        .get::<BodyHealth>(actor_entity)
        .unwrap()
        .health
        .current;
    assert_eq!(hp_after_first, 3, "first hit lands (5 → 3)");

    // Second identical hit while the i-frame is still hot (no tick decayed it):
    // ignored, so HP is unchanged — the sustained-overlap stream is collapsed.
    app.world_mut().write_message(slash());
    app.update();
    assert_eq!(
        app.world()
            .get::<BodyHealth>(actor_entity)
            .unwrap()
            .health
            .current,
        3,
        "a re-hit within the i-frame window must be ignored (no per-frame stream)"
    );
}

/// Shared setup for the cling-break tests: spawn a hostile actor, make it an
/// adhesive crawler clung to a LEFT wall (outward normal +x), then slash it.
fn slash_clung_surface_walker(cling_breaks_on_hit: bool) -> (App, bevy::prelude::Entity) {
    let mut app = App::new();
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let actor = spawn_hostile_actor(&mut app); // HP 5 — survives one slash
    {
        let mut cfg = app
            .world_mut()
            .get_mut::<super::super::actor_clusters::ActorConfig>(actor)
            .unwrap();
        cfg.tuning.surface_walker = true;
        cfg.tuning.cling_breaks_on_hit = cling_breaks_on_hit;
    }
    {
        // The crawler POLICY with a live attachment — the explicit model the
        // spawn selector installs for `surface_walker` archetypes.
        let mut model = app
            .world_mut()
            .get_mut::<crate::features::MotionModel>(actor)
            .unwrap();
        *model = crate::features::MotionModel::AdhesiveCrawler(ae::AdhesiveCrawlerMotion {
            params: ae::CrawlerParams::default(),
            state: ae::CrawlerState::attached(ae::Vec2::new(1.0, 0.0)),
        });
    }
    {
        app.world_mut()
            .get_mut::<crate::actor::BodyGroundState>(actor)
            .unwrap()
            .on_ground = true;
        app.world_mut()
            .get_mut::<crate::features::ActorSurfaceState>(actor)
            .unwrap()
            .surface_normal = ae::Vec2::new(1.0, 0.0);
    }
    app.world_mut().write_message(HitEvent {
        volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)).into(),
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
        !app.world()
            .get::<crate::actor::BodyGroundState>(actor)
            .unwrap()
            .on_ground,
        "a struck cling-breaker should leave its surface and fall"
    );
    assert_eq!(
        surf.surface_normal,
        ae::Vec2::new(1.0, 0.0),
        "detaching preserves the last contact normal until gravity-relative landing"
    );
    let kin = app
        .world()
        .get::<super::super::actor_clusters::BodyKinematics>(actor)
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
    assert!(
        app.world()
            .get::<crate::actor::BodyGroundState>(actor)
            .unwrap()
            .on_ground,
        "a non-breaking crawler keeps its footing"
    );
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
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
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
            BreakableFeature::new(ambition_interaction::Breakable::new("crate", 1)),
        ))
        .id();
    assert!(!app
        .world()
        .get::<BreakableFeature>(breakable)
        .unwrap()
        .broken());

    app.world_mut().write_message(HitEvent {
        volume: aabb.into(),
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
        .filter(|p| matches!(p.kind(), ambition_interaction::PickupKind::Currency { .. }))
        .count();
    assert_eq!(coins, 1, "shattering a crate drops one coin");
}

#[test]
fn enemy_defeat_drops_a_collectible_currency_coin() {
    let mut app = App::new();
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.add_systems(Update, |mut c: Commands| {
        drop_currency_coin(
            &mut c,
            ambition_platformer_primitives::lifecycle::SessionSpawnScope::UNSCOPED,
            "goblin_1",
            ae::Vec2::new(40.0, 50.0),
            ENEMY_BOUNTY,
        );
    });
    app.update();
    let mut q = app.world_mut().query::<(&PickupFeature, &FeatureId)>();
    let rows: Vec<(ambition_interaction::PickupKind, String)> = q
        .iter(app.world())
        .map(|(p, id)| (p.kind().clone(), id.as_str().to_string()))
        .collect();
    assert_eq!(rows.len(), 1, "exactly one coin dropped");
    assert_eq!(rows[0].1, "coin:goblin_1", "coin id is keyed to the enemy");
    assert_eq!(
        rows[0].0,
        ambition_interaction::PickupKind::Currency {
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
        ("gnu_ton_rider", Some("fireball")),
        ("clockwork_warden", Some("markrecall")),
        ("mockingbird", None),
        ("smirking_behemoth_boss", None),
    ];
    for (id, ability) in expect {
        let profile =
            BossBehaviorProfile::from_data(crate::boss_encounter::test_boss_catalog(), id);
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
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.add_systems(Update, |mut c: Commands| {
        drop_ability_pickup(
            &mut c,
            ambition_platformer_primitives::lifecycle::SessionSpawnScope::UNSCOPED,
            "trex_boss",
            ae::Vec2::new(10.0, 20.0),
            "grapple",
            "Grapple",
        );
    });
    app.update();
    let mut q = app.world_mut().query::<&PickupFeature>();
    let kinds: Vec<ambition_interaction::PickupKind> =
        q.iter(app.world()).map(|p| p.kind().clone()).collect();
    assert_eq!(kinds.len(), 1, "one ability pickup dropped");
    assert_eq!(
        kinds[0],
        ambition_interaction::PickupKind::Ability {
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
        ("gnu_ton_rider", Some(meteor::METEOR_ID)),
        ("clockwork_warden", None),
        ("flying_spaghetti_monster_boss", None),
    ];
    let mut gauntlets = 0;
    let mut abilities = 0;
    for (id, gauntlet) in expect {
        let profile =
            BossBehaviorProfile::from_data(crate::boss_encounter::test_boss_catalog(), id);
        assert_eq!(
            profile.signature_gauntlet.as_deref(),
            *gauntlet,
            "{id} signature gauntlet drifted from boss_profiles.ron",
        );
        if let Some(g) = profile.signature_gauntlet.as_deref() {
            assert!(
                ambition_characters::brain::held_item_by_id(g).is_some(),
                "boss {id} -> gauntlet {g} must be a registered held item",
            );
            gauntlets += 1;
        }
        if profile.reward_ability.is_some() {
            abilities += 1;
        }
    }
    // trex + mockingbird + smirking + mode_collapse + exploding_gradient +
    // overflow + the gnu_ton rider each arm a wielded gauntlet (seven "learn its
    // attack" drops; trex and the rider also grant a catalog ability).
    assert_eq!(gauntlets, 7, "seven bosses drop a signature gauntlet");
    // FSM(blink) + trex(grapple) + gnu(fireball) + clockwork(markrecall).
    assert_eq!(abilities, 4, "four bosses grant a catalog ability");
}

#[test]
fn exploding_mite_blast_is_a_player_damaging_enemy_hitbox() {
    let mut app = App::new();
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.add_systems(Update, |mut c: Commands| {
        spawn_death_explosion(
            &mut c,
            ambition_platformer_primitives::lifecycle::SessionSpawnScope::UNSCOPED,
            Entity::PLACEHOLDER,
            ae::Vec2::new(50.0, 60.0),
        );
    });
    app.update();
    let mut q = app.world_mut().query::<&crate::features::Hitbox>();
    let boxes: Vec<crate::features::Hitbox> = q.iter(app.world()).cloned().collect();
    assert_eq!(boxes.len(), 1, "the mite's death spawns one blast hitbox");
    assert_eq!(
        boxes[0].source,
        ambition_vfx::HitSide::Enemy,
        "enemy side -> the blast damages the player, not other mites (no chain)",
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
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.add_systems(
        Update,
        |mut c: Commands,
         catalog: bevy::prelude::Res<
            ambition_characters::actor::character_catalog::CharacterCatalog,
        >,
         roster: bevy::prelude::Res<crate::features::CharacterRoster>| {
            spawn_split_offspring(
                &mut c,
                &catalog,
                &roster,
                ambition_platformer_primitives::lifecycle::SessionSpawnScope::UNSCOPED,
                "divider_1",
                ae::Vec2::new(100.0, 100.0),
            );
        },
    );
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
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.add_systems(Update, |mut c: Commands| {
        drop_health_pickup(
            &mut c,
            ambition_platformer_primitives::lifecycle::SessionSpawnScope::UNSCOPED,
            "any",
            ae::Vec2::ZERO,
            ENEMY_HEALTH_DROP,
        );
    });
    app.update();
    let mut q = app.world_mut().query::<&PickupFeature>();
    let kinds: Vec<ambition_interaction::PickupKind> =
        q.iter(app.world()).map(|p| p.kind().clone()).collect();
    assert_eq!(kinds.len(), 1, "one heart dropped");
    assert!(
        matches!(kinds[0], ambition_interaction::PickupKind::Health { .. }),
        "the drop is a health pickup",
    );
}

#[test]
fn an_armed_enemy_archetype_resolves_a_weapon_to_drop() {
    // The defeat branch's weapon drop keys off `held_item_spec()`; the shark
    // rider carries a gun-sword, so a defeated rider drops one.
    let spec = crate::features::enemies::test_spec("pirate_shark_rider").held_item_spec();
    assert!(spec.is_some(), "the shark rider carries a weapon");
    assert_eq!(spec.unwrap().id.as_str(), "gun_sword");
}

// ── S3c: body-enforced reactive block ───────────────────────────────────────
//
// The shield is a body capability: the controller only sets `shield_held` (which
// the resolver lands on `status.shield_raised`, gated by `caps.can_shield`); the
// BODY negates a guarded hit from the side it faces. These drive the REAL actor
// damage system (`apply_feature_hit_events` → `apply_actor_hit`), so they prove
// the enforcement, not a mocked rule. A possessing human and an AI brain block
// identically because both only feed `shield_held` (invariants I2/I3).

/// Spawn a hostile actor with the shield capability, body facing +x (right),
/// 5 HP, with its guard raised iff `shield_raised`.
fn spawn_shielding_actor(app: &mut App, shield_raised: bool) -> bevy::prelude::Entity {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut enemy = crate::features::ecs::actor_clusters::ActorClusterSeed::new(
        "guard".to_string(),
        "Guard".to_string(),
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom(
            "cellular_automaton_fighter".into(),
        ),
        &[],
    );
    enemy.health =
        ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(5));
    enemy.kin.facing = 1.0;
    // The damage path reads the body's ONE shield component (`BodyShieldState`) —
    // set it directly, the way the pipeline shield limb would. (The `shield`
    // movement capability gates whether the pipeline RAISES the guard; the
    // resolver itself only reads the resulting `shield.active`.)
    enemy.body.0.shield.active = shield_raised;
    let (identity, disposition, combat, intent, cooldowns) = enemy_component_snapshot(&enemy);
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("guard"),
            CenteredAabb::from_center_size(aabb.center(), aabb.half_size() * 2.0),
            enemy.into_components(),
            crate::features::MotionModel::default(),
            identity,
            disposition,
            combat,
            intent,
            cooldowns,
        ))
        .id()
}

fn shield_test_app() -> App {
    let mut app = App::new();
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);
    app
}

/// A player slash whose hitbox is centered at `center` (must overlap the actor's
/// body AABB to land), dealing `damage`.
fn slash_at(center: ae::Vec2, damage: i32) -> HitEvent {
    HitEvent {
        volume: ae::Aabb::new(center, ae::Vec2::new(32.0, 40.0)).into(),
        damage,
        source: HitSource::PlayerSlash { knock_x: 200.0 },
        attacker: None,
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    }
}

fn actor_hp(app: &App, entity: bevy::prelude::Entity) -> i32 {
    app.world()
        .get::<BodyHealth>(entity)
        .expect("actor exists")
        .health
        .current
}

#[test]
fn raised_shield_negates_a_hit_from_the_faced_side() {
    let mut app = shield_test_app();
    let actor = spawn_shielding_actor(&mut app, true);
    // Body faces +x; the slash comes from the front (+x). The hitbox is wide
    // enough to overlap the body at the origin while its center sits forward.
    app.world_mut()
        .write_message(slash_at(ae::Vec2::new(14.0, 0.0), 2));
    app.update();
    assert_eq!(
        actor_hp(&app, actor),
        5,
        "a guarded hit from the faced side must be fully negated by the body"
    );
}

#[test]
fn a_lowered_shield_does_not_block() {
    let mut app = shield_test_app();
    let actor = spawn_shielding_actor(&mut app, false);
    app.world_mut()
        .write_message(slash_at(ae::Vec2::new(14.0, 0.0), 2));
    app.update();
    assert_eq!(
        actor_hp(&app, actor),
        3,
        "with the guard down the same front hit lands full damage"
    );
}

#[test]
fn a_raised_shield_does_not_guard_the_back() {
    let mut app = shield_test_app();
    let actor = spawn_shielding_actor(&mut app, true);
    // Body faces +x; this hit comes from BEHIND (-x). You can't guard your back.
    app.world_mut()
        .write_message(slash_at(ae::Vec2::new(-14.0, 0.0), 2));
    app.update();
    assert_eq!(
        actor_hp(&app, actor),
        3,
        "a hit from behind the guard still lands — the block is directional"
    );
}

// ── §A2 step 6: a struck actor rides the shared knockback resolution ─────────

/// A knockback-carrying hit (an aggressor swing pre-resolved to an actor
/// victim) launches the actor through `resolved_body_knockback_velocity` —
/// away from the source along its frame's side, rising against its gravity —
/// exactly the resolution a player victim gets.
#[test]
fn a_knockback_carrying_hit_launches_the_actor_like_a_player() {
    let mut app = shield_test_app();
    let victim = spawn_hostile_actor(&mut app);
    let feel = crate::time::feel::SandboxFeelTuning::default();
    // Attacker to the LEFT of the victim (victim at origin): expect a launch
    // toward +x with the feel-tuned enemy knockback, rising (world -y).
    app.world_mut().write_message(HitEvent {
        volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(40.0, 50.0)).into(),
        damage: 2,
        source: HitSource::EnemyAttack,
        attacker: None,
        target: HitTarget::Actor(victim),
        mode: HitMode::Knockback,
        knockback: Some(crate::features::HitKnockback {
            dir: 1.0,
            strength: 1.0,
            source_pos: ae::Vec2::new(-40.0, 0.0),
            impact_pos: ae::Vec2::ZERO,
            launch_dir: None,
        }),
        ignored_targets: Vec::new(),
    });
    app.update();
    let kin = app
        .world()
        .get::<super::super::actor_clusters::BodyKinematics>(victim)
        .unwrap();
    let expected = ae::Vec2::new(feel.enemy_knockback_x, -feel.enemy_knockback_y);
    assert!(
        (kin.vel - expected).length() < 1e-3,
        "actor knockback should be the shared feel-tuned resolution, got {:?} want {expected:?}",
        kin.vel
    );
    // §A2 step 7: the launch also arms the shared stagger set on `BodyCombat`,
    // exactly like the player's knockback path.
    let combat = app.world().get::<BodyCombat>(victim).unwrap();
    assert!(
        combat.hitstun_timer > 0.0 && combat.recoil_lock_timer > 0.0 && combat.hitstop_timer > 0.0,
        "a knockback hit arms hitstun/recoil/hitstop on the struck body: {combat:?}"
    );
}

/// A slash with no `HitKnockback` payload folds its `knock_x` impulse into the
/// same resolution: dir from the impulse sign, standard strength.
#[test]
fn a_slash_knock_x_folds_into_the_shared_knockback_resolution() {
    let mut app = shield_test_app();
    let victim = spawn_hostile_actor(&mut app);
    let feel = crate::time::feel::SandboxFeelTuning::default();
    // Slash volume centered on the victim (side derivation degenerates) with a
    // -x impulse: the stored dir carries the launch side.
    app.world_mut()
        .write_message(slash_at(ae::Vec2::new(0.0, 0.0), 1));
    app.update();
    let kin = app
        .world()
        .get::<super::super::actor_clusters::BodyKinematics>(victim)
        .unwrap();
    // `slash_at` carries knock_x: 200.0 → dir +1, strength 1.0.
    let expected = ae::Vec2::new(feel.enemy_knockback_x, -feel.enemy_knockback_y);
    assert!(
        (kin.vel - expected).length() < 1e-3,
        "slash knockback should ride the shared resolution, got {:?} want {expected:?}",
        kin.vel
    );
}

// ── S3e: relational actor-vs-actor damage application ────────────────────────

/// A `HitTarget::Actor(victim)` event (the pre-resolved actor-vs-actor hit an
/// Enemy/Boss swing emits) damages EXACTLY that actor, even though its source is
/// the victim-side `EnemyAttack` — and never spills onto other overlapping actors.
#[test]
fn an_actor_targeted_hit_damages_only_the_named_actor() {
    let mut app = shield_test_app();
    // Two hostile actors at the same spot (overlapping). HP 5 each.
    let victim = spawn_hostile_actor(&mut app);
    let bystander = spawn_hostile_actor(&mut app);
    app.world_mut().write_message(HitEvent {
        volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(40.0, 50.0)).into(),
        damage: 2,
        // Victim-side source, yet the Actor target routes it to the actor consumer.
        source: HitSource::EnemyAttack,
        attacker: None,
        target: HitTarget::Actor(victim),
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();
    assert_eq!(
        actor_hp(&app, victim),
        3,
        "the named actor takes the relational hit"
    );
    assert_eq!(
        actor_hp(&app, bystander),
        5,
        "an overlapping non-target actor is untouched (pre-resolved, not broadcast)"
    );
}

/// The player-melee one-hit-per-target dedup must PERSIST on the attacker's
/// `MovePlayback` (the moveset read-model swing is wiped each frame). This drives
/// `apply_feature_hit_events` directly: a `PlayerSlash` Volume hit whose attacker
/// carries a live melee move must (a) land, and (b) fold the struck target's key
/// onto `MovePlayback.hit_targets` so the next tick's emit ignores it.
#[test]
fn a_player_slash_folds_the_struck_target_onto_the_move_accumulator() {
    use crate::combat::moveset::{simple_melee, MovePlayback, SimpleMeleeParams};
    let mut app = App::new();
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(Update, apply_feature_hit_events);

    let attacker = app
        .world_mut()
        .spawn(MovePlayback::new(
            simple_melee(&SimpleMeleeParams::default()),
            1.0,
        ))
        .id();
    let enemy = spawn_hostile_actor(&mut app); // HP 5, box at origin
    let volume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    app.world_mut().write_message(HitEvent {
        volume: volume.into(),
        damage: 2,
        source: HitSource::PlayerSlash { knock_x: 0.0 },
        attacker: Some(attacker),
        target: HitTarget::Volume,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.update();

    assert_eq!(
        app.world().get::<BodyHealth>(enemy).unwrap().health.current,
        3,
        "the slash lands (5 -> 3)"
    );
    let acc = app
        .world()
        .get::<MovePlayback>(attacker)
        .unwrap()
        .hit_targets
        .clone();
    assert!(
        acc.iter().any(|k| k.starts_with("enemy:")),
        "the struck target must be folded onto MovePlayback.hit_targets so the \
         next active tick ignores it; got {acc:?}"
    );
}

/// END-TO-END isolation: a moveset player's FollowOwner strike emits a Volume
/// HitEvent EVERY active tick; the projection + fold-back must collapse them to
/// ONE landed hit. Victim i-frames are cleared each tick so the ONLY thing that
/// can dedup is the `MovePlayback.hit_targets` accumulator (the projection copies
/// it onto the swing → `apply_hitbox_damage` emits it as ignored_targets). If the
/// accumulator fails to persist, the enemy drains every tick.
#[test]
fn a_moveset_player_strike_hits_a_target_once_across_a_multi_tick_window() {
    use crate::combat::moveset::{
        project_moveset_melee_to_body_melee, simple_melee, MovePlayback, MovesetMelee,
        SimpleMeleeParams,
    };
    use bevy::prelude::IntoScheduleConfigs;
    fn clear_iframes(mut q: bevy::prelude::Query<&mut ambition_characters::actor::BodyCombat>) {
        for mut c in &mut q {
            c.damage_invuln_timer = 0.0;
        }
    }
    let mut app = App::new();
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<ActorStimulus>();
    app.add_systems(
        Update,
        (
            clear_iframes,
            project_moveset_melee_to_body_melee,
            crate::features::apply_hitbox_damage,
            apply_feature_hit_events,
        )
            .chain(),
    );

    let player = app
        .world_mut()
        .spawn((
            MovePlayback::new(simple_melee(&SimpleMeleeParams::default()), 1.0),
            MovesetMelee,
            crate::features::BodyMelee::default(),
            ambition_engine_core::BodyKinematics {
                pos: ae::Vec2::ZERO,
                size: ae::Vec2::new(20.0, 40.0),
                facing: 1.0,
                ..Default::default()
            },
            crate::features::MotionModel::default(),
            ambition_engine_core::CenteredAabb::from_center_size(
                ae::Vec2::ZERO,
                ae::Vec2::new(20.0, 40.0),
            ),
        ))
        .id();
    app.world_mut().spawn((
        ambition_vfx::Hitbox {
            owner: player,
            source: ambition_vfx::HitSide::Player,
            anchor: ambition_vfx::HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::ZERO,
            },
            half_extent: ae::Vec2::new(24.0, 40.0),
            shape: None,
            facing: 1.0,
            damage: 2,
            knockback_strength: 0.0,
            knockback_growth: 0.0,
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
        },
        ambition_vfx::HitboxHits::default(),
    ));
    let enemy = spawn_hostile_actor(&mut app); // HP 5 at origin

    for _ in 0..6 {
        app.update();
    }
    assert_eq!(
        app.world().get::<BodyHealth>(enemy).unwrap().health.current,
        3,
        "a multi-tick player strike must hit once (5 -> 3), not many times"
    );
}
