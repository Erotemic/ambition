//! The sim→view rebuild CONTRACT tests (moved here at the E4 mint — they
//! exercise gameplay_core sim entities THROUGH the observation-boundary
//! rebuilds, and the dev-dependency cycle means gameplay_core's own test
//! build sees a different type universe than the one this crate links).

use ambition_combat::components::FeatureId;
use ambition_combat::components::FeatureName;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::CenteredAabb;
use ambition_platformer2d_items::pickups::Collected;
use ambition_platformer2d_items::pickups::PickupFeature;
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_sim_view::{FeatureViewIndex, rebuild_feature_view_index};
use ambition_world_items::containers::ChestFeature;
use bevy::prelude::{App, Update};

fn ambition_boss_catalog() -> ambition_boss_encounter::BossCatalog {
    let sources = ambition_boss_encounter::ambition_encounter_sources();
    let encounters: Vec<&str> = sources.iter().map(String::as_str).collect();
    let fragment = ambition_boss_encounter::BossCatalogFragment::from_ron(
        "view-contract",
        Some("clockwork_warden"),
        None::<String>,
        include_str!("../../../game/ambition_content/assets/data/boss_profiles.ron"),
        &encounters,
        "{}",
        Default::default(),
    )
    .expect("view-contract boss fixture should parse");
    let mut registry = ambition_boss_encounter::BossCatalogRegistry::default();
    registry.register(fragment).unwrap();
    registry.assemble().unwrap()
}

#[test]
fn boss_classifies_as_boss_not_the_actor_enemy_fallback() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let boss_catalog = ambition_boss_catalog();
    // The view-index `actors` query keys on those, so without a `Without<BossConfig>` exclusion a
    // boss matches the actor family — which is inserted BEFORE the boss family (first-wins
    // priority) — and renders as the generic enemy fallback sprite (a big goblin), invisible,
    // instead of its boss sheet.
    let boss_body = ae::Aabb::new(ae::Vec2::new(500.0, 500.0), ae::Vec2::new(80.0, 120.0));
    let boss = ambition_boss_encounter::BossClusterScratch::new(&sprite_content,
        &boss_catalog,
        "gnu_ton_rider",
        "GNU-ton",
        boss_body,
        ambition_entity_catalog::placements::BossBrain::Dormant,
    );
    let combat = ambition_characters::actor::BodyCombat::default();
    let (identity, disposition) =
        ambition_boss_encounter::ecs::boss_component_snapshot(boss.as_ref());

    let mut app = App::new();
    app.init_resource::<FeatureViewIndex>();
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("gnu_ton_rider"),
        FeatureName::new("GNU-ton"),
        CenteredAabb::from_aabb(boss_body),
        boss.into_components(),
        ambition_characters::brain::BossAttackState::default(),
        (identity, disposition, combat),
    ));
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.add_systems(Update, rebuild_feature_view_index);
    app.update();

    let index = app.world().resource::<FeatureViewIndex>();
    let view = index
        .get("gnu_ton_rider")
        .expect("the boss must have a feature view");
    assert_eq!(
        view.kind,
        ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
        "a boss is an actor like every other (got {:?})",
        view.kind,
    );
    assert!(
        view.visible,
        "a live boss must produce a VISIBLE view from its own boss family — not be \
         shadowed by the invisible generic-actor fallback (its ActorConfig is absent)",
    );
}

/// A same-frame pickup collection (drops the pickup entity into the `Collected` state) must be
/// reflected in `FeatureViewIndex` in that same `app.update()`.
#[test]
fn feature_view_index_reflects_same_frame_pickup_collection() {
    let center = ae::Vec2::new(64.0, 64.0);
    let mut app = App::new();
    app.insert_resource(FeatureViewIndex::default());
    let pickup_entity = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("hp_pickup"),
            FeatureName::new("Health"),
            CenteredAabb::from_center_size(center, ae::Vec2::new(12.0, 12.0)),
            PickupFeature::new(ambition_interaction::Pickup::new(
                "hp_pickup",
                ambition_interaction::PickupKind::Health { amount: 1 },
            )),
        ))
        .id();
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.add_systems(Update, rebuild_feature_view_index);
    app.update();
    assert!(
        app.world()
            .resource::<FeatureViewIndex>()
            .get("hp_pickup")
            .is_some_and(|v| v.visible),
        "uncollected pickup must report visible"
    );
    // Now mark it Collected and rebuild on the next tick — the
    // index must drop `visible` immediately, not a frame later.
    app.world_mut().entity_mut(pickup_entity).insert(Collected);
    app.update();
    assert!(
        app.world()
            .resource::<FeatureViewIndex>()
            .get("hp_pickup")
            .is_some_and(|v| !v.visible),
        "collected pickup must report not visible in the rebuild tick"
    );
}

/// Duplicate ids across feature families must resolve to the
/// first-priority family in the legacy linear-scan order
/// (pickup → chest → breakable → switch → actor → hazard → boss).
/// A naive `HashMap::insert` would flip the rendered kind to
/// whichever family was iterated last.
#[test]
fn feature_view_index_first_write_wins_on_duplicate_ids() {
    let pos = ae::Vec2::new(0.0, 0.0);
    let mut app = App::new();
    app.insert_resource(FeatureViewIndex::default());
    // Pickup wins under the legacy linear-scan priority.
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("dup_id"),
        FeatureName::new("Pickup"),
        CenteredAabb::from_center_size(pos, ae::Vec2::new(8.0, 8.0)),
        PickupFeature::new(ambition_interaction::Pickup::new(
            "dup_id",
            ambition_interaction::PickupKind::Health { amount: 1 },
        )),
    ));
    // Same id, different family — must NOT shadow the pickup.
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("dup_id"),
        FeatureName::new("Chest"),
        CenteredAabb::from_center_size(pos, ae::Vec2::new(16.0, 16.0)),
        ChestFeature::new(ambition_interaction::Chest::new("dup_id", None)),
    ));
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.add_systems(Update, rebuild_feature_view_index);
    app.update();
    let view = app
        .world()
        .resource::<FeatureViewIndex>()
        .get("dup_id")
        .copied()
        .expect("duplicate id must resolve to one of the two");
    assert_eq!(
        view.kind,
        FeatureVisualKind::Pickup,
        "first-write-wins priority must keep the pickup view (not the chest)"
    );
}
