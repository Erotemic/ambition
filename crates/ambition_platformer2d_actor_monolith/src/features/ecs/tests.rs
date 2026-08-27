//! ECS-feature behavior tests.
//!
//! Tests cover the world-overlay rebuild, interact buffer → chest/NPC
//! resolution, and the feature-view-index same-frame consistency
//! invariants. Extracted from `ecs/mod.rs` to keep the implementation
//! file focused on systems.

use super::*;
use ambition_combat::events::{GameplayBanner, SetFlagRequested};
use ambition_encounter::switches::SwitchActivated;
use bevy::prelude::{App, IntoScheduleConfigs, Update};

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
        crate::avatar::primary_player_scratch(player_pos, ae::AbilitySet::sandbox_all());
    scratch.ground.on_ground = true;
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        scratch,
        ambition_characters::actor::Health::new(10),
    );
    // Production home avatars always wear a catalog character
    // (`simulation_world` attaches `WornCharacter`); dialogue speaks as the
    // worn identity, so the fixture mirrors that instead of leaning on a
    // process-global default speaker.
    let entity = app
        .world_mut()
        .spawn((
            bundle,
            ambition_characters::actor::WornCharacter::new("player_robot"),
        ))
        .id();
    // The interact buffer is SLOT state now; prime the primary controller slot and
    // point the controlled subject at this body.
    app.world_mut()
        .get_resource_or_insert_with(ambition_characters::control::SlotInteractionState::default)
        .primary_mut()
        .interact_buffer_timer = 0.15;
    app.world_mut().insert_resource(
        ambition_platformer2d_shared_tangle::markers::ControlledSubject(Some(entity)),
    );
}

#[test]
fn combat_body_pogo_geometry_stays_entity_side() {
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
            brain_override: None,
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
    let body = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("guide"),
            FeatureName::new("Guide"),
            CenteredAabb::from_center_size(center, size),
            crate::features::ActorDisposition::Peaceful,
            seed.into_components(),
            DamageableVolumes::default(),
            PogoPolicy::FromDamageable,
            PogoTargetVolumes::default(),
        ))
        .id();
    app.add_systems(
        Update,
        (
            refresh_body_damageable_volumes,
            derive_pogo_target_volumes,
            rebuild_feature_ecs_world_overlay,
        )
            .chain(),
    );
    app.update();

    let pogo = app
        .world()
        .get::<PogoTargetVolumes>(body)
        .expect("combat body should publish pogo affordance geometry");
    assert_eq!(
        pogo.volumes,
        vec![aabb],
        "damageable => pogoable remains body data"
    );

    let overlay = app.world().resource::<FeatureEcsWorldOverlay>();
    assert!(
        overlay
            .blocks
            .iter()
            .all(|block| !matches!(block.kind, ae::BlockKind::PogoOrb)),
        "ordinary combat bodies must retain entity identity instead of becoming anonymous world pogo blocks"
    );
}

#[test]
fn explicit_pogo_contributor_lowers_published_world_surface() {
    let coarse_body = ae::Aabb::new(ae::Vec2::new(500.0, 500.0), ae::Vec2::new(80.0, 120.0));
    let pogo_surface = ae::Aabb::new(ae::Vec2::new(440.0, 420.0), ae::Vec2::new(12.0, 16.0));

    let mut app = App::new();
    app.insert_resource(FeatureEcsWorldOverlay::default());
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("moving_pogo_surface"),
        FeatureName::new("moving pogo surface"),
        CenteredAabb::from_aabb(coarse_body),
        PogoTargetContributor,
        PogoTargetVolumes {
            volumes: vec![pogo_surface],
        },
    ));
    app.add_systems(Update, rebuild_feature_ecs_world_overlay);
    app.update();

    let overlay = app.world().resource::<FeatureEcsWorldOverlay>();
    assert!(
        overlay.blocks.iter().any(|block| {
            matches!(block.kind, ae::BlockKind::PogoOrb) && block.aabb == pogo_surface
        }),
        "an explicit world contributor should lower its published pogo surface"
    );
    assert!(
        !overlay.blocks.iter().any(|block| {
            matches!(block.kind, ae::BlockKind::PogoOrb) && block.aabb == coarse_body
        }),
        "explicit world pogo uses its published surface, not a coarse body envelope"
    );
}

#[test]
fn explicit_pogo_contributor_without_published_surface_uses_its_envelope() {
    let coarse_surface = ae::Aabb::new(ae::Vec2::new(300.0, 260.0), ae::Vec2::new(24.0, 10.0));

    let mut app = App::new();
    app.insert_resource(FeatureEcsWorldOverlay::default());
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("plain_rebound_surface"),
        FeatureName::new("plain rebound surface"),
        CenteredAabb::from_aabb(coarse_surface),
        PogoTargetContributor,
    ));
    app.add_systems(Update, rebuild_feature_ecs_world_overlay);
    app.update();

    let overlay = app.world().resource::<FeatureEcsWorldOverlay>();
    let pogo_blocks: Vec<_> = overlay
        .blocks
        .iter()
        .filter(|block| matches!(block.kind, ae::BlockKind::PogoOrb))
        .collect();
    assert_eq!(
        pogo_blocks.len(),
        1,
        "one contributor lowers one fallback surface"
    );
    assert_eq!(pogo_blocks[0].aabb, coarse_surface);
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
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<crate::avatar::PlayerHealRequested>();

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
            .resource::<ambition_characters::control::SlotInteractionState>()
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
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<crate::avatar::PlayerHealRequested>();

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
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<crate::avatar::PlayerHealRequested>();

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
    app.init_state::<ambition_platformer2d_shared_tangle::schedule::GameMode>();

    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_dialog::DialogState::default());
    // the AUTHORITY travels with the read-model. `interact_ecs_actors_and_
    // switches` opens a conversation in the simulation and shows it in the UI,
    // so a fixture with only the second half fails Bevy's param validation.
    // NOT solved by making the param `Option`: that waiver would answer "may
    // this be absent" when the question is who OWNS registering it, and in
    // production the feature plugin does.
    app.init_resource::<ambition_conversation::ActiveConversation>();
    app.init_resource::<ambition_dialog::DialogueNodeIndex>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        <crate::avatar::StartingCharacter>::default(),
    );
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
            brain_override: None,
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
    // the box is a PROJECTION now, so the projection has to run. The
    // interaction system decides that a conversation exists; the presentation
    // half opens the runner from that, outside the sim schedule. A fixture that
    // ran only the first would be asserting on a text box nothing was left to
    // open.
    app.add_systems(
        Update,
        (
            interact_ecs_actors_and_switches,
            ambition_conversation::project_the_dialog_ui_from_the_conversation,
        )
            .chain(),
    );
    app.update();

    let dialogue = app.world().resource::<ambition_dialog::DialogState>();
    assert!(
        dialogue.active(),
        "dialogue should be active after NPC interact"
    );
}

/// Regression for the presentation-reader ordering contract:
/// every system added to
/// [`crate::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync`] must run
/// after [`crate::schedule::Platformer2dSimulationPhaseMonolith::FeatureViewSync`].
///
/// Structural check: inspect the actual Bevy schedule graph rather than depend on the
/// executor's behavior with two otherwise-unordered systems. `.after()` between sets becomes a
/// directed edge in `Schedule::graph().dependency()`, and the edge is materialized eagerly by
/// `configure_sets` — we don't have to run the schedule or rely on any declaration-order
/// fallback.
#[test]
fn presentation_visual_sync_runs_after_feature_view_sync() {
    use crate::schedule::{
        configure_platformer2d_simulation_phases, Platformer2dSimulationPhaseMonolith,
    };
    use bevy::ecs::schedule::{NodeId, Schedules};
    use bevy::prelude::{IntoScheduleConfigs, Update};

    let mut app = App::new();
    configure_platformer2d_simulation_phases(&mut app);
    // Touch both sets with an empty system each so they're
    // actually registered as nodes (configure_sets alone is
    // enough to register the relationship, but a no-op .in_set
    // also makes the intent explicit).
    app.add_systems(
        Update,
        (
            (|| {}).in_set(Platformer2dSimulationPhaseMonolith::FeatureViewSync),
            (|| {}).in_set(Platformer2dSimulationPhaseMonolith::PresentationVisualSync),
        ),
    );

    let schedules = app.world().resource::<Schedules>();
    let schedule = schedules
        .get(Update)
        .expect("Update schedule must exist after configure_platformer2d_simulation_phases");
    let graph = schedule.graph();
    let fvs_key = graph
        .system_sets
        .get_key(Platformer2dSimulationPhaseMonolith::FeatureViewSync.intern())
        .expect("FeatureViewSync must be a registered SystemSet");
    let pvs_key = graph
        .system_sets
        .get_key(Platformer2dSimulationPhaseMonolith::PresentationVisualSync.intern())
        .expect("PresentationVisualSync must be a registered SystemSet");
    let edge_present = graph
        .dependency()
        .graph()
        .contains_edge(NodeId::Set(fvs_key), NodeId::Set(pvs_key));
    assert!(
        edge_present,
        "schedule dependency graph must carry an edge \
         FeatureViewSync -> PresentationVisualSync (set in \
         configure_platformer2d_simulation_phases). Without it, presentation \
         systems can read a stale FeatureViewIndex on any frame \
         that mutates feature state (pickups, switches, encounter \
         spawns, save sync, sandbox reset)."
    );
}
