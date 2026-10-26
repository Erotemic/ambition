//! Observe a reset through the production runtime phase graph.
use ambition_combat::components::{FeatureId, FeatureName};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::CenteredAabb;
use ambition_platformer2d_items::pickups::PickupFeature;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_sim_view::{FeatureViewIndex, rebuild_feature_view_index};
use bevy::prelude::{App, Commands, Entity, IntoScheduleConfigs, Query, Update, With};

/// The test stands in a minimal reset-shaped system that despawns
/// the pre-reset pickup and spawns a new one, then asserts the
/// FeatureViewIndex reflects the new id after `app.update()`. It runs in
/// `Platformer2dSimulationPhase::ResetProcessing`, the phase the
/// in-simulation reset systems use, to pin the ordering.
#[test]
fn feature_view_index_reflects_same_frame_reset_spawn() {
    use crate::schedule::configure_platformer2d_simulation_phases;
    use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase;

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
    configure_platformer2d_simulation_phases(&mut app);
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.world_mut()
        .spawn(ambition_platformer2d_shared_tangle::lifecycle::SessionRoot(
            ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(0),
        ));
    app.add_systems(
        Update,
        (
            fake_reset_system.in_set(Platformer2dSimulationPhase::ResetProcessing),
            rebuild_feature_view_index.in_set(Platformer2dSimulationPhase::FeatureViewSync),
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
