use super::*;
use ambition_characters::actor::character_catalog::{CharacterCatalog, CharacterCatalogData};
use ambition_combat::components::ActorIdentity;
use ambition_platformer2d_core as ae;
use bevy::prelude::{App, Update};

/// A catalog with one pettable character and one that is not.
fn catalog() -> CharacterCatalog {
    let data: CharacterCatalogData = ron::from_str(
        r#"(
            brain_presets: { "idle": StandStill },
            action_set_presets: { "peaceful": (move_style: Walk) },
            characters: {
                "good_dog": (
                    display_name: "Good Dog",
                    spritesheet: "sprites/dog.png",
                    manifest: "sprites/dog.ron",
                    tier: MainHall,
                    body_kind: Standard,
                    composition: None,
                    default_brain: "idle",
                    default_action_set: "peaceful",
                    petting: Some((sound: Some("npc.dog.woof"))),
                ),
                "shopkeeper": (
                    display_name: "Shopkeeper",
                    spritesheet: "sprites/shop.png",
                    manifest: "sprites/shop.ron",
                    tier: MainHall,
                    body_kind: Standard,
                    composition: None,
                    default_brain: "idle",
                    default_action_set: "peaceful",
                ),
            },
        )"#,
    )
    .expect("the fixture catalog parses");
    CharacterCatalog::from_data(data)
}

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(catalog());
    app.add_message::<VfxMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_systems(
        Update,
        (pet_pettable_characters, project_gesture_holds).chain(),
    );
    app
}

/// The player, with a buffered Interact press.
fn spawn_player(app: &mut App, pos: ae::Vec2) -> Entity {
    let scratch = crate::avatar::primary_player_scratch(pos, ae::AbilitySet::sandbox_all());
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        scratch,
        ambition_characters::actor::Health::new(10),
    );
    let player = app.world_mut().spawn(bundle).id();
    app.world_mut()
        .get_resource_or_insert_with(ambition_characters::control::SlotInteractionState::default)
        .primary_mut()
        .interact_buffer_timer = 0.15;
    player
}

/// A peaceful catalog character standing at `pos`, facing away from +x.
fn spawn_character(app: &mut App, pos: ae::Vec2, character_id: &str) -> Entity {
    let size = ae::Vec2::new(64.0, 48.0);
    let interactable = ambition_interaction::Interactable::new(
        "placement",
        "Greet",
        ae::Aabb::new(pos, size),
        ambition_interaction::InteractionKind::Npc {
            character_id: Some(character_id.to_string()),
            dialogue_id: None,
            patrol_radius: 0.0,
            patrol_path_id: None,
            brain_override: None,
        },
    );
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            CenteredAabb::from_center_size(pos, size),
            ActorDisposition::Peaceful,
            ActorIdentity::new("placement", character_id),
            ActorInteraction { interactable },
            BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size,
                facing: -1.0,
            },
            BodyAnimFacts::default(),
        ))
        .id()
}

fn anim(app: &App, body: Entity) -> BodyAnimFacts {
    app.world()
        .get::<BodyAnimFacts>(body)
        .expect("a body")
        .clone()
}

fn gesture_held(app: &App, body: Entity) -> bool {
    app.world()
        .get::<ControlHolds>(body)
        .is_some_and(|holds| holds.holds(ControlHold::Gesture))
}

#[test]
fn interact_beside_a_pettable_character_pets_it() {
    let mut app = app();
    let dog_at = ae::Vec2::new(100.0, 100.0);
    let player = spawn_player(&mut app, dog_at + ae::Vec2::new(30.0, 0.0));
    let dog = spawn_character(&mut app, dog_at, "good_dog");

    app.update();

    assert_eq!(
        anim(&app, player).pet_anim_timer,
        PET_SECONDS,
        "the player pets"
    );
    assert_eq!(
        anim(&app, dog).petted_anim_timer,
        PET_SECONDS,
        "the dog is petted"
    );
    assert!(
        gesture_held(&app, player) && gesture_held(&app, dog),
        "both are held still"
    );
    let world = app.world();
    let player_kin = world.get::<BodyKinematics>(player).unwrap();
    let dog_kin = world.get::<BodyKinematics>(dog).unwrap();
    assert_eq!(
        dog_kin.facing, 1.0,
        "the dog turns to the player, on its right"
    );
    assert_eq!(player_kin.facing, -1.0, "the player faces the dog");
    assert!(
        player_kin.pos.x > dog_at.x + 32.0,
        "the player stands at the dog's front"
    );
    assert_eq!(
        world
            .resource::<ambition_characters::control::SlotInteractionState>()
            .primary()
            .interact_buffer_timer,
        0.0,
        "the pet spends the press, so no conversation opens with it"
    );
    let hearts = world
        .resource::<bevy::ecs::message::Messages<VfxMessage>>()
        .iter_current_update_messages()
        .filter(|message| matches!(message, VfxMessage::Hearts { .. }))
        .count();
    assert_eq!(hearts, 1, "the pet shows hearts");
}

#[test]
fn a_character_without_a_petting_row_is_not_petted() {
    let mut app = app();
    let at = ae::Vec2::new(100.0, 100.0);
    let player = spawn_player(&mut app, at + ae::Vec2::new(30.0, 0.0));
    let shopkeeper = spawn_character(&mut app, at, "shopkeeper");

    app.update();

    assert_eq!(anim(&app, player).pet_anim_timer, 0.0);
    assert_eq!(anim(&app, shopkeeper).petted_anim_timer, 0.0);
    assert!(!gesture_held(&app, player));
    assert!(
        app.world()
            .resource::<ambition_characters::control::SlotInteractionState>()
            .primary()
            .interact_buffer_timer
            > 0.0,
        "the press is left for the conversation"
    );
}

#[test]
fn the_gesture_hold_lets_go_when_the_pet_ends_and_only_its_own_bit() {
    let mut app = App::new();
    app.add_systems(Update, project_gesture_holds);
    let body = app
        .world_mut()
        .spawn((
            BodyAnimFacts {
                pet_anim_timer: 1.0,
                ..Default::default()
            },
            ControlHolds::only(ControlHold::Conversation),
        ))
        .id();

    app.update();
    assert!(gesture_held(&app, body), "a running pet holds the body");

    app.world_mut()
        .get_mut::<BodyAnimFacts>(body)
        .unwrap()
        .pet_anim_timer = 0.0;
    app.update();
    let holds = app
        .world()
        .get::<ControlHolds>(body)
        .expect("the conversation still holds it");
    assert!(!holds.holds(ControlHold::Gesture), "the pet let go");
    assert!(
        holds.holds(ControlHold::Conversation),
        "and left the conversation's hold alone"
    );
}
