use ambition_characters::actor::WornCharacter;
use ambition_characters::brain::ActionSet;
use ambition_engine_core::{
    BodyAbilities, BodyBlinkState, BodyDashState, BodyFlightState, BodyJumpState,
};
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
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
            BodyFlightState::default(),
            BodyBlinkState::default(),
            BodyDashState::default(),
            BodyJumpState::default(),
        ))
        .id();

    app.update();
    app.world_mut()
        .resource_mut::<AbilityChangeObservations>()
        .0 = 0;
    let riding = ambition_engine_core::surface::SurfaceMotion::Riding {
        on: ambition_engine_core::surface::SurfaceRef::Chain(7),
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
    expected: ambition_engine_core::surface::SurfaceMotion,
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
