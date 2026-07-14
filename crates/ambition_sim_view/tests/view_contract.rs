//! The sim→view rebuild CONTRACT tests (moved here at the E4 mint — they
//! exercise gameplay_core sim entities THROUGH the observation-boundary
//! rebuilds, and the dev-dependency cycle means gameplay_core's own test
//! build sees a different type universe than the one this crate links).

use ambition_actors::features::{
    CenteredAabb, ChestFeature, Collected, FeatureId, FeatureName, FeatureSimEntity,
    FeatureVisualKind, PickupFeature,
};
use ambition_engine_core as ae;
use ambition_sim_view::{rebuild_feature_view_index, FeatureViewIndex};
use bevy::prelude::{App, Commands, Entity, IntoScheduleConfigs, Query, Update, With};

fn ambition_boss_catalog() -> ambition_actors::boss_encounter::BossCatalog {
    const ENCOUNTERS: &[&str] = &[
        include_str!("../../../game/ambition_content/assets/data/boss_encounters/clockwork_warden.ron"),
        include_str!("../../../game/ambition_content/assets/data/boss_encounters/mockingbird.ron"),
        include_str!("../../../game/ambition_content/assets/data/boss_encounters/gnu_ton_rider.ron"),
        include_str!("../../../game/ambition_content/assets/data/boss_encounters/smirking_behemoth_boss.ron"),
        include_str!("../../../game/ambition_content/assets/data/boss_encounters/flying_spaghetti_monster_boss.ron"),
        include_str!("../../../game/ambition_content/assets/data/boss_encounters/trex_boss.ron"),
        include_str!("../../../game/ambition_content/assets/data/boss_encounters/mode_collapse_boss.ron"),
        include_str!("../../../game/ambition_content/assets/data/boss_encounters/exploding_gradient_boss.ron"),
        include_str!("../../../game/ambition_content/assets/data/boss_encounters/overflow_boss.ron"),
    ];
    let fragment = ambition_actors::boss_encounter::BossCatalogFragment::from_ron(
        "view-contract",
        Some("clockwork_warden"),
        None::<String>,
        include_str!("../../../game/ambition_content/assets/data/boss_profiles.ron"),
        ENCOUNTERS,
        "{}",
        std::collections::BTreeMap::new(),
        std::collections::BTreeMap::new(),
    )
    .expect("view-contract boss fixture should parse");
    let mut registry = ambition_actors::boss_encounter::BossCatalogRegistry::default();
    registry.register(fragment).unwrap();
    registry.assemble().unwrap()
}

#[test]
fn boss_classifies_as_boss_not_the_actor_enemy_fallback() {
    let boss_catalog = ambition_boss_catalog();
    // Regression: bosses carry the shared actor read-models (`ActorDisposition`,
    // `BodyCombat`, …) synced by `sync_boss_actor_components`. The
    // view-index `actors` query keys on those, so without a
    // `Without<BossConfig>` exclusion a boss matches the actor family — which is
    // inserted BEFORE the boss family (first-wins priority) — and renders as the
    // generic enemy fallback sprite (a big goblin), invisible, instead of its
    // boss sheet. This pins the exclusion that the deleted `ActorRuntime` tag
    // used to provide implicitly.
    let boss_body = ae::Aabb::new(ae::Vec2::new(500.0, 500.0), ae::Vec2::new(80.0, 120.0));
    let boss = ambition_actors::features::BossClusterScratch::new(
        &boss_catalog,
        "gnu_ton_rider",
        "GNU-ton",
        boss_body,
        ambition_entity_catalog::placements::BossBrain::Dormant,
    );
    let (identity, disposition, combat, intent, cooldowns) =
        ambition_actors::features::boss_component_snapshot(
            boss.as_ref(),
            &ambition_characters::brain::BossAttackState::default(),
            &boss.health,
            &ambition_characters::actor::BodyCombat::default(),
        );

    let mut app = App::new();
    app.init_resource::<FeatureViewIndex>();
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("gnu_ton_rider"),
        FeatureName::new("GNU-ton"),
        CenteredAabb::from_aabb(boss_body),
        boss.into_components(),
        ambition_characters::brain::BossAttackState::default(),
        (identity, disposition, combat, intent, cooldowns),
    ));
    app.add_systems(Update, rebuild_feature_view_index);
    app.update();

    let index = app.world().resource::<FeatureViewIndex>();
    let view = index
        .get("gnu_ton_rider")
        .expect("the boss must have a feature view");
    assert_eq!(
        view.kind,
        ambition_combat::FeatureVisualKind::Actor,
        "a boss is an actor like every other (got {:?})",
        view.kind,
    );
    assert!(
        view.visible,
        "a live boss must produce a VISIBLE view from its own boss family — not be \
         shadowed by the invisible generic-actor fallback (its ActorConfig is absent)",
    );
}

/// A same-frame pickup collection (drops the pickup entity into
/// the `Collected` state) must be reflected in `FeatureViewIndex`
/// in that same `app.update()`. Regression guard for the
/// previously-stale index that lived in `PresentationSync` —
/// pickups, switches, and encounter mobs all mutate in sets that
/// run AFTER `CoreSimulation`, so a rebuild in `PresentationSync`
/// would have published last frame's view.
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

/// Regression for the `ResetProcessing` ordering bug:
/// `process_sandbox_reset_request` despawns every feature entity
/// and spawns the start room's feature set. If `ResetProcessing`
/// runs unordered relative to `FeatureViewSync`, the cache on the
/// reset frame can either still hold the pre-reset id (stale) or
/// miss the post-reset id entirely (empty). Joining
/// `ResetProcessing` into the chain BEFORE `FeatureViewSync`
/// guarantees same-frame consistency.
///
/// The test stands in a minimal reset-shaped system that despawns
/// the pre-reset pickup and spawns a new one, then asserts the
/// FeatureViewIndex reflects the new id after `app.update()`. The
/// real `process_sandbox_reset_request` runs in
/// `SandboxSet::ResetProcessing`; we use the same `.in_set` to
/// pin the ordering.
#[test]
fn feature_view_index_reflects_same_frame_reset_spawn() {
    use ambition_actors::schedule::configure_sandbox_sets;
    use ambition_platformer_primitives::schedule::SandboxSet;

    fn fake_reset_system(mut commands: Commands, existing: Query<Entity, With<FeatureSimEntity>>) {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        commands.spawn((
            FeatureSimEntity,
            FeatureId::new("post_reset_pickup"),
            FeatureName::new("Post-Reset Health"),
            CenteredAabb::from_center_size(ae::Vec2::new(20.0, 20.0), ae::Vec2::new(12.0, 12.0)),
            PickupFeature::new(ambition_interaction::Pickup::new(
                "post_reset_pickup",
                ambition_interaction::PickupKind::Health { amount: 1 },
            )),
        ));
    }

    let mut app = App::new();
    app.insert_resource(FeatureViewIndex::default());
    // Pre-reset pickup with a different id — must be gone from
    // the index after the reset+rebuild on the same tick.
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("pre_reset_pickup"),
        FeatureName::new("Pre-Reset Health"),
        CenteredAabb::from_center_size(ae::Vec2::ZERO, ae::Vec2::new(12.0, 12.0)),
        PickupFeature::new(ambition_interaction::Pickup::new(
            "pre_reset_pickup",
            ambition_interaction::PickupKind::Health { amount: 1 },
        )),
    ));
    configure_sandbox_sets(&mut app);
    app.world_mut().spawn(ambition_platformer_primitives::lifecycle::SessionRoot(
        ambition_platformer_primitives::lifecycle::SessionScopeId(0),
    ));
    app.add_systems(
        Update,
        (
            fake_reset_system.in_set(SandboxSet::ResetProcessing),
            rebuild_feature_view_index.in_set(SandboxSet::FeatureViewSync),
        ),
    );

    app.update();

    let index = app.world().resource::<FeatureViewIndex>();
    assert!(
        index.get("pre_reset_pickup").is_none(),
        "pre-reset feature must be gone from the index on the reset frame"
    );
    assert!(
        index.get("post_reset_pickup").is_some(),
        "post-reset feature must be present on the reset frame — the cache \
         rebuild must run AFTER ResetProcessing, not in parallel with it"
    );
}
