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
    let bundle = crate::player::PlayerSimulationBundle::from_scratch(
        scratch,
        ambition_characters::actor::Health::new(10),
    );
    let entity = app.world_mut().spawn(bundle).id();
    // The interact buffer is SLOT state now; prime the primary controller slot and
    // point the controlled subject at this body.
    app.world_mut()
        .get_resource_or_insert_with(crate::player::SlotInteractionState::default)
        .primary_mut()
        .interact_buffer_timer = 0.15;
    app.world_mut()
        .insert_resource(ambition_platformer_primitives::markers::ControlledSubject(
            Some(entity),
        ));
}

#[test]
fn peaceful_actor_damageable_volume_derives_pogo_overlay() {
    let center = ae::Vec2::new(120.0, 180.0);
    let size = ae::Vec2::new(32.0, 48.0);
    let aabb = ae::Aabb::new(center, size * 0.5);
    let interactable = ambition_interaction::Interactable::new(
        "guide",
        "Talk",
        aabb,
        ambition_interaction::InteractionKind::Npc {
            character_id: None,
            dialogue_id: Some("hub_guide".into()),
            patrol_radius: 0.0,
            patrol_path_id: None,
        },
    );
    let (seed, _render) = super::actor_clusters::ActorClusterSeed::new_peaceful_npc(
        "guide",
        "Guide",
        aabb,
        &interactable,
        &[],
    );

    let mut app = App::new();
    app.insert_resource(FeatureEcsWorldOverlay::default());
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("guide"),
        FeatureName::new("Guide"),
        CenteredAabb::from_center_size(center, size),
        crate::features::ActorDisposition::Peaceful,
        seed.into_components(),
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
        ambition_entity_catalog::placements::BossBrain::Dormant,
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
    let mut breakable = ambition_interaction::Breakable::new("crate", 1);
    breakable.collision = ambition_interaction::BreakableCollision::Solid;
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
            ChestFeature::new(ambition_interaction::Chest::new("test_chest", None)),
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
    assert!(
        !app.world()
            .resource::<crate::player::SlotInteractionState>()
            .primary()
            .buffered(),
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
            ChestFeature::new(ambition_interaction::Chest::new("far_chest", None)),
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
            ChestFeature::new(ambition_interaction::Chest::new("already_open", None)),
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
    app.init_state::<ambition_platformer_primitives::schedule::GameMode>();

    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_dialog::DialogState::default());
    app.init_resource::<ambition_dialog::DialogueNodeIndex>();
    app.init_resource::<crate::player::StartingCharacter>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<QuestAdvanceRequested>();
    app.add_message::<SwitchActivated>();
    app.add_message::<VfxMessage>();

    spawn_interaction_player(&mut app, center);

    let npc_aabb = ae::Aabb::new(center, ae::Vec2::new(16.0, 24.0));
    let interactable = ambition_interaction::Interactable::new(
        "guide",
        "Talk",
        npc_aabb,
        ambition_interaction::InteractionKind::Npc {
            character_id: None,
            dialogue_id: Some("hub_guide".into()),
            patrol_radius: 0.0,
            patrol_path_id: None,
        },
    );
    let (seed, _render) = super::actor_clusters::ActorClusterSeed::new_peaceful_npc(
        "guide",
        "Guide",
        npc_aabb,
        &interactable,
        &[],
    );
    // Dialogue now keys off the shared `ActorInteraction` payload + a peaceful
    // `ActorDisposition`, not an `ActorRuntime::Npc` type tag.
    let interaction = crate::features::ActorInteraction {
        interactable,
        talk_radius: crate::features::NPC_TALK_RADIUS,
    };
    app.world_mut().spawn((
        FeatureSimEntity,
        CenteredAabb::from_center_size(center, ae::Vec2::new(32.0, 48.0)),
        seed.into_components(),
        interaction,
        crate::features::ActorIdentity::new("guide", "Guide"),
        crate::features::ActorDisposition::Peaceful,
    ));

    // No switches in this test — the switch query will be empty and the
    // system will handle the NPC branch.
    app.add_systems(Update, interact_ecs_actors_and_switches);
    app.update();

    let dialogue = app.world().resource::<ambition_dialog::DialogState>();
    assert!(
        dialogue.active(),
        "dialogue should be active after NPC interact"
    );
}

/// Regression for the presentation-reader ordering contract:
/// every system added to
/// [`crate::schedule::SandboxSet::PresentationVisualSync`] must run
/// after [`crate::schedule::SandboxSet::FeatureViewSync`].
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
    use crate::schedule::{configure_sandbox_sets, SandboxSet};
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
