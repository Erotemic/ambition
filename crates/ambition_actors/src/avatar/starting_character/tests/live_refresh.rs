use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::ActionSet;
use ambition_engine_core::BodyAbilities;
use ambition_platformer_primitives::markers::PrimaryPlayer;
use bevy::prelude::*;

use crate::actor::PlayerEntity;
use crate::combat::moveset::ActorMoveset;
use crate::features::{MomentumMotion, MotionModel};

#[derive(Resource, Default)]
struct AbilityChangeObservations(u32);

fn observe_body_ability_changes(
    changed: Query<(), Changed<BodyAbilities>>,
    mut observations: ResMut<AbilityChangeObservations>,
) {
    observations.0 += changed.iter().count() as u32;
}

/// Production-shaped regression for the two change-detection edges that used
/// to reset Sanic's surface follower every frame:
///
/// 1. an unchanged inspector mirror must not mark `BodyAbilities` changed;
/// 2. a real ability edit on an Authored persona must not reapply movement
///    identity or erase `MomentumMotion`'s persistent riding state.
#[test]
fn live_ability_sync_does_not_rederive_authored_movement_identity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(super::test_catalog());
    app.init_resource::<ambition_dev_tools::dev_tools::EditableAbilitySet>();
    app.init_resource::<ambition_dev_tools::dev_tools::EditableMovementTuning>();
    app.init_resource::<AbilityChangeObservations>();
    app.add_systems(
        Update,
        (
            ambition_dev_tools::sync_live_player_dev_edits_system,
            super::super::apply_worn_character_gameplay,
            observe_body_ability_changes,
        )
            .chain(),
    );

    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            WornCharacter::new("sanic"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_engine_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    ambition_engine_core::Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();

    app.update();
    app.world_mut()
        .resource_mut::<AbilityChangeObservations>()
        .0 = 0;
    let riding = ambition_engine_core::SurfaceMotion::Riding {
        on: ambition_engine_core::SurfaceRef::Chain(7),
        s: 123.0,
        v_t: 456.0,
    };
    {
        let mut model = app.world_mut().get_mut::<MotionModel>(entity).unwrap();
        match &mut *model {
            MotionModel::SurfaceMomentum(momentum) => momentum.state = riding,
            other => panic!("expected Sanic SurfaceMomentum after initial wear, got {other:?}"),
        }
    }

    // No inspector edit: merely querying the live body mutably must not create
    // a false Changed<BodyAbilities> edge.
    app.update();
    assert_eq!(
        app.world().resource::<AbilityChangeObservations>().0,
        0,
        "unchanged live dev resources must not mark BodyAbilities changed"
    );
    assert_riding_state(app.world(), entity, riding);

    // A real inspector ability edit does change BodyAbilities, but Sanic's
    // Authored persona does not derive its kit or movement from that source.
    app.world_mut()
        .resource_mut::<ambition_dev_tools::dev_tools::EditableAbilitySet>()
        .attack = false;
    app.update();
    assert_eq!(
        app.world().resource::<AbilityChangeObservations>().0,
        1,
        "the observer must see the genuine inspector edit"
    );
    assert_riding_state(app.world(), entity, riding);
}

fn assert_riding_state(
    world: &World,
    entity: Entity,
    expected: ambition_engine_core::SurfaceMotion,
) {
    let model = world.get::<MotionModel>(entity).unwrap();
    match model {
        MotionModel::SurfaceMomentum(MomentumMotion { state, .. }) => {
            assert_eq!(
                *state, expected,
                "ability synchronization must preserve the surface follower's persistent state"
            );
        }
        other => panic!("expected Sanic SurfaceMomentum, got {other:?}"),
    }
}

/// A CROSS-model runtime re-wear (momentum persona → axis persona) preserves
/// every shared body fact (world position, velocity, facing) and initializes
/// ONLY the destination policy's private state — the ADR 0024 §7 swap
/// invariant, exercised through the production worn-character seam and
/// therefore independent of who controls the body (the system never reads a
/// controller).
#[test]
fn cross_model_rewear_preserves_shared_state_and_initializes_axis_private_state() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(super::test_catalog());
    app.add_systems(Update, super::super::apply_worn_character_gameplay);

    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            WornCharacter::new("sanic"),
            MotionModel::default(),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            ambition_engine_core::BodyKinematics::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    ambition_engine_core::Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
        ))
        .id();
    app.update();
    assert!(
        matches!(
            app.world().get::<MotionModel>(entity).unwrap(),
            MotionModel::SurfaceMomentum(_)
        ),
        "wearing sanic selects the momentum policy"
    );

    // Live shared state + stale would-be axis maneuver state accumulated while
    // riding as sanic.
    let pose = ambition_engine_core::BodyKinematics {
        pos: ambition_engine_core::Vec2::new(321.0, 654.0),
        vel: ambition_engine_core::Vec2::new(900.0, -50.0),
        size: ambition_engine_core::Vec2::new(24.0, 40.0),
        facing: -1.0,
    };
    *app.world_mut()
        .get_mut::<ambition_engine_core::BodyKinematics>(entity)
        .unwrap() = pose;
    app.world_mut()
        .get_mut::<ambition_engine_core::BodyGroundState>(entity)
        .unwrap()
        .coyote_timer = 0.1;
    app.world_mut()
        .get_mut::<ambition_engine_core::BodyWallState>(entity)
        .unwrap()
        .wall_clinging = true;
    app.world_mut()
        .get_mut::<ambition_engine_core::BodyDashState>(entity)
        .unwrap()
        .charges_available = 2;

    // Re-wear an axis persona (the default protagonist).
    app.world_mut()
        .entity_mut(entity)
        .insert(WornCharacter::new("player"));
    app.update();

    assert!(
        matches!(
            app.world().get::<MotionModel>(entity).unwrap(),
            MotionModel::AxisSwept(_)
        ),
        "re-wearing the protagonist selects the axis policy"
    );
    assert_eq!(
        *app.world()
            .get::<ambition_engine_core::BodyKinematics>(entity)
            .unwrap(),
        pose,
        "world pose, velocity, and facing survive the swap untouched"
    );
    assert_eq!(
        app.world()
            .get::<ambition_engine_core::BodyGroundState>(entity)
            .unwrap()
            .coyote_timer,
        0.0,
        "the axis destination begins with NO imported coyote grace"
    );
    assert!(
        !app.world()
            .get::<ambition_engine_core::BodyWallState>(entity)
            .unwrap()
            .wall_clinging,
        "no imported wall engagement"
    );
    assert_eq!(
        app.world()
            .get::<ambition_engine_core::BodyDashState>(entity)
            .unwrap()
            .charges_available,
        2,
        "body RESOURCES (dash charges) are shared facts and survive"
    );
}
