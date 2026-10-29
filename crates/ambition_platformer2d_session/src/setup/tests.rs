use super::*;
use ambition_characters::actor::BodyHealth;
use ambition_characters::prepared::PreparedCharacterRegistry;
use ambition_characters::session_cast::ActiveSessionCast;
use ambition_platformer2d_shared_tangle::lifecycle::{
    SoleLiveRoom, insert_live_room_component, insert_session_world_component,
};

#[derive(Resource, Default)]
struct BuiltBody(Option<SimulationWorld>);

fn build_body(sprite_content: bevy::prelude::Res<ambition_sprite_sheet::SpriteContent>,
    mut commands: Commands,
    setup: LiveInitialBodySetup,
    world: SoleLiveRoom<RoomGeometry>,
    mut built: ResMut<BuiltBody>,
) {
    built.0 = Some(setup.spawn(&sprite_content, &mut commands, SessionSpawnScope::UNSCOPED, &world, "alpha"));
}

fn app(policy: crate::initial_body::InitialBodyPolicy) -> App {
    let mut app = App::new();
    app.insert_resource(ambition_sprite_sheet::SpriteContent::default());
    app.init_resource::<ae::ActiveMovementTuning>()
        .init_resource::<BuiltBody>();
    insert_session_world_component(app.world_mut(), policy);
    insert_live_room_component(
        app.world_mut(),
        RoomGeometry(ae::World::new(
            "initial room",
            ae::Vec2::new(640.0, 480.0),
            ae::Vec2::new(96.0, 96.0),
            Vec::new(),
        )),
    );
    app.add_systems(Update, build_body);
    app
}

fn cast(health: i32) -> PreparedCharacterRegistry {
    let mut app = App::new();
    app.insert_resource(ambition_sprite_sheet::SpriteContent::default());
    let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
        "alpha", "Alpha", "fixture",
    );
    definition.vitals.max_health = Some(health);
    ambition_characters::prepared::stage_authored_character(
        &mut app,
        definition,
        &Default::default(),
    )
    .unwrap();
    ambition_characters::prepared::close_preparation_barrier(app.world_mut());
    app.world().resource::<PreparedCharacterRegistry>().clone()
}

#[test]
fn live_initial_body_uses_the_sessions_frozen_cast() {
    let mut app = app(crate::initial_body::InitialBodyPolicy::SpawnCharacter(
        crate::initial_body::StartingCharacter::new("alpha"),
    ));
    insert_session_world_component(
        app.world_mut(),
        (
            crate::initial_body::HomeBodyResources::default(),
            ambition_platformer2d_actor_monolith::avatar::HomeBodyAbilities::default(),
        ),
    );
    app.insert_resource(ActiveSessionCast(Some(cast(17))));
    app.insert_resource(cast(99));
    app.update();
    let body = app
        .world()
        .resource::<BuiltBody>()
        .0
        .as_ref()
        .expect("the initializer ran")
        .player
        .expect("the policy builds a body");
    assert_eq!(app.world().get::<BodyHealth>(body).unwrap().max(), 17);
}

#[test]
fn no_initial_body_needs_no_cast_or_home_body_grants() {
    let mut app = app(crate::initial_body::InitialBodyPolicy::NoInitialBody);
    app.update();
    assert!(
        app.world()
            .resource::<BuiltBody>()
            .0
            .as_ref()
            .expect("the initializer ran")
            .player
            .is_none()
    );
    let world = app.world_mut();
    assert_eq!(world.query_filtered::<Entity, With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>>().iter(world).count(), 0);
}
