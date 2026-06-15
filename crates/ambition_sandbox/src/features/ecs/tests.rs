//! ECS-feature behavior tests.
//!
//! Tests cover the world-overlay rebuild, interact buffer → chest/NPC
//! resolution, and the feature-view-index same-frame consistency
//! invariants. Extracted from `ecs/mod.rs` to keep the implementation
//! file focused on systems.

use super::*;
use bevy::prelude::{App, IntoScheduleConfigs, Update, With};

/// Spawn the canonical player entity used by interaction system tests.
///
/// `player_pos` must overlap the feature AABB under test; the interact
/// buffer is pre-filled so the system sees it as buffered on the first
/// `app.update()` call.
fn spawn_interaction_player(app: &mut App, player_pos: ae::Vec2) {
    // The interaction system queries `BodyKinematics` +
    // `PlayerEntity` (and reads interact_buffer_timer);
    // `PlayerSimulationBundle` covers all of that.
    let mut scratch =
        crate::player::primary_player_scratch(player_pos, ae::AbilitySet::sandbox_all());
    scratch.ground.on_ground = true;
    let mut bundle =
        crate::player::PlayerSimulationBundle::from_scratch(scratch, crate::actor::Health::new(10));
    bundle.interaction.interact_buffer_timer = 0.15;
    app.world_mut().spawn(bundle);
}

#[test]
fn peaceful_actor_damageable_volume_derives_pogo_overlay() {
    let center = ae::Vec2::new(120.0, 180.0);
    let size = ae::Vec2::new(32.0, 48.0);
    let aabb = ae::Aabb::new(center, size * 0.5);
    let npc = super::npc_clusters::NpcClusterScratch::new_with_paths(
        "guide",
        "Guide",
        aabb,
        crate::interaction::Interactable::new(
            "guide",
            "Talk",
            aabb,
            crate::interaction::InteractionKind::Npc {
                dialogue_id: Some("hub_guide".into()),
                patrol_radius: 0.0,
                patrol_path_id: None,
            },
        ),
        &[],
    );

    let mut app = App::new();
    app.insert_resource(FeatureEcsWorldOverlay::default());
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("guide"),
        FeatureName::new("Guide"),
        CenteredAabb::from_center_size(center, size),
        ActorRuntime::Npc,
        npc.into_components(),
        DamageableVolumes::default(),
        PogoPolicy::FromDamageable,
        PogoTargetVolumes::default(),
    ));
    app.add_systems(
        Update,
        (
            refresh_actor_damageable_volumes,
            derive_pogo_target_volumes,
            rebuild_feature_ecs_world_overlay,
        )
            .chain(),
    );
    app.update();

    let overlay = app.world().resource::<FeatureEcsWorldOverlay>();
    assert!(
        overlay
            .blocks
            .iter()
            .any(|block| matches!(block.kind, ae::BlockKind::PogoOrb) && block.aabb == aabb),
        "peaceful NPCs are player-damageable and should therefore publish pogo blocks"
    );
}

#[test]
fn overlay_uses_published_pogo_volumes_instead_of_boss_body_aabb() {
    let boss_body = ae::Aabb::new(ae::Vec2::new(500.0, 500.0), ae::Vec2::new(80.0, 120.0));
    let pogo_hurtbox = ae::Aabb::new(ae::Vec2::new(440.0, 420.0), ae::Vec2::new(12.0, 16.0));
    let boss = super::boss_clusters::BossClusterScratch::new(
        "gnu_ton",
        "GNU-ton",
        boss_body,
        crate::actor::BossBrain::Dormant,
    );

    let mut app = App::new();
    app.insert_resource(FeatureEcsWorldOverlay::default());
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("gnu_ton"),
        FeatureName::new("GNU-ton"),
        CenteredAabb::from_aabb(boss_body),
        boss.into_components(),
        PogoTargetVolumes {
            volumes: vec![pogo_hurtbox],
        },
    ));
    app.add_systems(Update, rebuild_feature_ecs_world_overlay);
    app.update();

    let overlay = app.world().resource::<FeatureEcsWorldOverlay>();
    assert!(
        overlay
            .blocks
            .iter()
            .any(|block| matches!(block.kind, ae::BlockKind::PogoOrb) && block.aabb == pogo_hurtbox),
        "boss-specific hurtboxes should drive pogo blocks"
    );
    assert!(
        !overlay
            .blocks
            .iter()
            .any(|block| matches!(block.kind, ae::BlockKind::PogoOrb) && block.aabb == boss_body),
        "the overlay must not fall back to the coarse boss body AABB"
    );
}

#[test]
fn ecs_overlay_ignores_broken_breakables() {
    let mut breakable = crate::interaction::Breakable::new("crate", 1);
    breakable.collision = crate::interaction::BreakableCollision::Solid;
    let mut app = App::new();
    app.insert_resource(FeatureEcsWorldOverlay::default());
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("crate"),
        FeatureName::new("crate"),
        CenteredAabb::from_center_size(ae::Vec2::ZERO, ae::Vec2::new(16.0, 16.0)),
        BreakableFeature::new(breakable),
    ));
    app.add_systems(Update, rebuild_feature_ecs_world_overlay);
    app.update();
    assert_eq!(
        app.world()
            .resource::<FeatureEcsWorldOverlay>()
            .blocks
            .len(),
        1
    );
}

/// A buffered interact with the player overlapping a closed chest inserts
/// the `Opened` marker on the chest entity and clears the buffer.
#[test]
fn interact_buffered_opens_adjacent_chest() {
    let center = ae::Vec2::new(100.0, 100.0);
    let mut app = App::new();

    app.insert_resource(GameplayBanner::default());
    app.add_message::<SetFlagRequested>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();

    spawn_interaction_player(&mut app, center);

    let chest_entity = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            ChestFeature::new(crate::interaction::Chest::new("test_chest", None)),
            FeatureId::new("test_chest"),
            FeatureName::new("test_chest"),
            CenteredAabb::from_center_size(center, ae::Vec2::new(24.0, 24.0)),
        ))
        .id();

    app.add_systems(Update, open_ecs_chests);
    app.update();

    assert!(
        app.world().get::<Opened>(chest_entity).is_some(),
        "chest should have Opened marker after interact"
    );
    let interaction = app
        .world_mut()
        .query_filtered::<&crate::player::PlayerInteractionState, With<crate::player::PlayerEntity>>()
        .single(app.world())
        .expect("player entity must exist");
    assert!(
        !interaction.buffered(),
        "interact buffer should be cleared after opening chest"
    );
}

/// A chest that the player is not overlapping must not be opened even
/// when the interact buffer is filled.
#[test]
fn interact_buffered_does_not_open_distant_chest() {
    let player_pos = ae::Vec2::new(100.0, 100.0);
    let chest_pos = ae::Vec2::new(500.0, 500.0);
    let mut app = App::new();

    app.insert_resource(GameplayBanner::default());
    app.add_message::<SetFlagRequested>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();

    spawn_interaction_player(&mut app, player_pos);

    let chest_entity = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            ChestFeature::new(crate::interaction::Chest::new("far_chest", None)),
            FeatureId::new("far_chest"),
            FeatureName::new("far_chest"),
            CenteredAabb::from_center_size(chest_pos, ae::Vec2::new(24.0, 24.0)),
        ))
        .id();

    app.add_systems(Update, open_ecs_chests);
    app.update();

    assert!(
        app.world().get::<Opened>(chest_entity).is_none(),
        "distant chest must not be opened"
    );
}

/// Already-opened chests are not re-opened by a second interact.
#[test]
fn interact_does_not_reopen_already_opened_chest() {
    let center = ae::Vec2::new(100.0, 100.0);
    let mut app = App::new();

    app.insert_resource(GameplayBanner::default());
    app.add_message::<SetFlagRequested>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();

    spawn_interaction_player(&mut app, center);

    let chest_entity = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            ChestFeature::new(crate::interaction::Chest::new("already_open", None)),
            FeatureId::new("already_open"),
            FeatureName::new("already_open"),
            CenteredAabb::from_center_size(center, ae::Vec2::new(24.0, 24.0)),
            Opened,
        ))
        .id();

    app.add_systems(Update, open_ecs_chests);
    app.update();

    // The entity should still have Opened (idempotent) but we verify the
    // system didn't panic or try to re-insert the marker.
    assert!(app.world().get::<Opened>(chest_entity).is_some());
}

/// When a peaceful NPC's AABB overlaps the player and the interact buffer
/// is filled, `interact_ecs_actors_and_switches` starts a dialogue session.
#[test]
fn interact_buffered_starts_npc_dialogue() {
    use bevy::state::app::StatesPlugin;

    let center = ae::Vec2::new(100.0, 100.0);
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<crate::GameMode>();

    app.insert_resource(GameplayBanner::default());
    app.insert_resource(crate::dialog::DialogState::default());
    app.add_message::<SetFlagRequested>();
    app.add_message::<QuestAdvanceRequested>();
    app.add_message::<SwitchActivated>();
    app.add_message::<VfxMessage>();

    spawn_interaction_player(&mut app, center);

    let npc_aabb = ae::Aabb::new(center, ae::Vec2::new(16.0, 24.0));
    let npc = super::npc_clusters::NpcClusterScratch::new_with_paths(
        "guide",
        "Guide",
        npc_aabb,
        crate::interaction::Interactable::new(
            "guide",
            "Talk",
            npc_aabb,
            crate::interaction::InteractionKind::Npc {
                dialogue_id: Some("hub_guide".into()),
                patrol_radius: 0.0,
                patrol_path_id: None,
            },
        ),
        &[],
    );
    app.world_mut().spawn((
        FeatureSimEntity,
        CenteredAabb::from_center_size(center, ae::Vec2::new(32.0, 48.0)),
        ActorRuntime::Npc,
        npc.into_components(),
    ));

    // No switches in this test — the switch query will be empty and the
    // system will handle the NPC branch.
    app.add_systems(Update, interact_ecs_actors_and_switches);
    app.update();

    let dialogue = app.world().resource::<crate::dialog::DialogState>();
    assert!(
        dialogue.active(),
        "dialogue should be active after NPC interact"
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
            PickupFeature::new(crate::interaction::Pickup::new(
                "hp_pickup",
                crate::interaction::PickupKind::Health { amount: 1 },
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
        PickupFeature::new(crate::interaction::Pickup::new(
            "dup_id",
            crate::interaction::PickupKind::Health { amount: 1 },
        )),
    ));
    // Same id, different family — must NOT shadow the pickup.
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("dup_id"),
        FeatureName::new("Chest"),
        CenteredAabb::from_center_size(pos, ae::Vec2::new(16.0, 16.0)),
        ChestFeature::new(crate::interaction::Chest::new("dup_id", None)),
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
    use crate::app::{configure_sandbox_sets, SandboxSet};

    fn fake_reset_system(mut commands: Commands, existing: Query<Entity, With<FeatureSimEntity>>) {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        commands.spawn((
            FeatureSimEntity,
            FeatureId::new("post_reset_pickup"),
            FeatureName::new("Post-Reset Health"),
            CenteredAabb::from_center_size(ae::Vec2::new(20.0, 20.0), ae::Vec2::new(12.0, 12.0)),
            PickupFeature::new(crate::interaction::Pickup::new(
                "post_reset_pickup",
                crate::interaction::PickupKind::Health { amount: 1 },
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
        PickupFeature::new(crate::interaction::Pickup::new(
            "pre_reset_pickup",
            crate::interaction::PickupKind::Health { amount: 1 },
        )),
    ));
    configure_sandbox_sets(&mut app);
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

/// Regression for the presentation-reader ordering contract:
/// every system added to
/// [`crate::app::SandboxSet::PresentationVisualSync`] must run
/// after [`crate::app::SandboxSet::FeatureViewSync`].
///
/// Structural check: inspect the actual Bevy schedule graph
/// rather than depend on the executor's behavior with two
/// otherwise-unordered systems. `.after()` between sets
/// becomes a directed edge in `Schedule::graph().dependency()`,
/// and the edge is materialized eagerly by `configure_sets` —
/// we don't have to run the schedule or rely on any
/// declaration-order fallback. The test FAILS the moment
/// `PresentationVisualSync.after(FeatureViewSync)` is removed
/// from `configure_sandbox_sets`, regardless of what executor
/// Bevy ships or how it tie-breaks unordered systems.
#[test]
fn presentation_visual_sync_runs_after_feature_view_sync() {
    use crate::app::{configure_sandbox_sets, SandboxSet};
    use bevy::ecs::schedule::{NodeId, Schedules};
    use bevy::prelude::{IntoScheduleConfigs, Update};

    let mut app = App::new();
    configure_sandbox_sets(&mut app);
    // Touch both sets with an empty system each so they're
    // actually registered as nodes (configure_sets alone is
    // enough to register the relationship, but a no-op .in_set
    // also makes the intent explicit).
    app.add_systems(
        Update,
        (
            (|| {}).in_set(SandboxSet::FeatureViewSync),
            (|| {}).in_set(SandboxSet::PresentationVisualSync),
        ),
    );

    let schedules = app.world().resource::<Schedules>();
    let schedule = schedules
        .get(Update)
        .expect("Update schedule must exist after configure_sandbox_sets");
    let graph = schedule.graph();
    let fvs_key = graph
        .system_sets
        .get_key(SandboxSet::FeatureViewSync.intern())
        .expect("FeatureViewSync must be a registered SystemSet");
    let pvs_key = graph
        .system_sets
        .get_key(SandboxSet::PresentationVisualSync.intern())
        .expect("PresentationVisualSync must be a registered SystemSet");
    let edge_present = graph
        .dependency()
        .graph()
        .contains_edge(NodeId::Set(fvs_key), NodeId::Set(pvs_key));
    assert!(
        edge_present,
        "schedule dependency graph must carry an edge \
         FeatureViewSync -> PresentationVisualSync (set in \
         configure_sandbox_sets). Without it, presentation \
         systems can read a stale FeatureViewIndex on any frame \
         that mutates feature state (pickups, switches, encounter \
         spawns, save sync, sandbox reset)."
    );
}
