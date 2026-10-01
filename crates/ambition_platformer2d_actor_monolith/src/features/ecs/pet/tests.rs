use super::*;
use ambition_vfx::vfx::VfxInRoom;
use ambition_characters::actor::character_catalog::{CharacterCatalog, CharacterCatalogData};
use ambition_combat::components::ActorIdentity;
use ambition_combat::components::ActorDisposition;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
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
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.add_message::<PetRequested>();
    app.add_message::<VfxInRoom>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_systems(
        Update,
        // The production order: the pet's chain in `GameplayEffects`.
        (apply_pet_requests, advance_pet_beats, project_pet_holds).chain(),
    );
    app
}

/// The mark the pet's walk steers the petter to.
fn walk_mark(app: &App, petter: Entity) -> Option<ae::Vec2> {
    app.world()
        .get::<CommandedMove>(petter)
        .map(|command| command.target)
}

/// Stand the petter on its walk's mark. This fixture runs no integrator, so
/// the walk itself is the composed game's to witness
/// (`talking_to_the_dog_offers_a_pet_that_holds_both_still_until_it_ends`).
fn arrive(app: &mut App, petter: Entity) {
    let mark = walk_mark(app, petter).expect("the petter is walking to a mark");
    app.world_mut().get_mut::<BodyKinematics>(petter).unwrap().pos.x = mark.x;
}

/// A hit that moves `body`: the recoil lock a strike opens.
fn knock(app: &mut App, body: Entity) {
    app.world_mut()
        .entity_mut(body)
        .entry::<BodyCombat>()
        .or_default()
        .get_mut()
        .recoil_lock_timer = 0.3;
}

fn hearts(app: &App) -> usize {
    app.world()
        .resource::<bevy::ecs::message::Messages<VfxInRoom>>()
        .iter_current_update_messages().map(|m| &m.vfx)
        .filter(|message| matches!(message, VfxMessage::Hearts { .. }))
        .count()
}

const PLAYER: &str = "player";

fn sim_id(name: &str) -> SimId {
    SimId::placement(name)
}

fn spawn_player(app: &mut App, pos: ae::Vec2) -> Entity {
    let scratch = crate::avatar::primary_player_scratch(pos, ae::AbilitySet::sandbox_all());
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        scratch,
        ambition_characters::actor::Health::new(10),
    );
    app.world_mut().spawn(bundle).insert(sim_id(PLAYER)).id()
}

/// The conversation's `<<pet>>`: the player pets `petted`.
fn ask_for_a_pet(app: &mut App, petted: &str) {
    app.world_mut().write_message(PetRequested {
        petter: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(sim_id(PLAYER), None),
        petted: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(sim_id(petted), None),
    });
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
            sim_id(character_id),
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

/// The pet walks the petter to the petted body's front, and pets on arrival.
///
/// The control is the arrival: the gesture, the facing and the hearts. The
/// subject is the request's own tick: the petter has not moved (the pet used
/// to write its position to the mark in one tick), it walks toward the mark,
/// and both bodies are held while it does.
#[test]
fn a_pet_walks_the_petter_to_the_front_and_pets_on_arrival() {
    let mut app = app();
    let dog_at = ae::Vec2::new(100.0, 100.0);
    let start = dog_at + ae::Vec2::new(30.0, 0.0);
    let player = spawn_player(&mut app, start);
    let dog = spawn_character(&mut app, dog_at, "good_dog");

    ask_for_a_pet(&mut app, "good_dog");
    app.update();

    let player_kin = app.world().get::<BodyKinematics>(player).unwrap();
    assert_eq!(player_kin.pos, start, "the petter was moved to the mark in one tick");
    let mark = walk_mark(&app, player).expect("the petter walks to the dog's front");
    assert!(mark.x > dog_at.x + 32.0, "the mark is the dog's front, on the petter's side");
    assert_eq!(
        (anim(&app, player).petting, anim(&app, dog).petted),
        (false, false),
        "the gesture waits for the walk"
    );
    assert!(
        gesture_held(&app, player) && gesture_held(&app, dog),
        "both are held while the petter walks"
    );
    assert_eq!(
        app.world().get::<BodyKinematics>(dog).unwrap().facing,
        1.0,
        "the dog turns to the petter at once"
    );

    arrive(&mut app, player);
    app.update();

    assert!(anim(&app, player).petting, "the player pets");
    assert!(anim(&app, dog).petted, "the dog is petted");
    assert_eq!(
        app.world().get::<PetBeat>(player).map(|beat| beat.stage),
        Some(PetStage::Gesture { remaining: PET_SECONDS }),
        "the beat owns the gesture's length"
    );
    assert!(walk_mark(&app, player).is_none(), "the walk is over");
    assert!(
        gesture_held(&app, player) && gesture_held(&app, dog),
        "both are held still"
    );
    assert_eq!(
        app.world().get::<BodyKinematics>(player).unwrap().facing,
        -1.0,
        "the player faces the dog"
    );
    assert_eq!(hearts(&app), 1, "the pet shows hearts");
}

/// A hit that moves the petter during the walk ends the pet: the walk stops,
/// both bodies are let go, and arriving later pets nothing. The control is
/// the same walk with no hit, which pets on arrival (above).
#[test]
fn a_hit_during_the_walk_interrupts_the_pet() {
    let mut app = app();
    let dog_at = ae::Vec2::new(100.0, 100.0);
    let player = spawn_player(&mut app, dog_at + ae::Vec2::new(30.0, 0.0));
    let dog = spawn_character(&mut app, dog_at, "good_dog");
    ask_for_a_pet(&mut app, "good_dog");
    app.update();
    let mark = walk_mark(&app, player).expect("the walk started");

    knock(&mut app, player);
    app.update();

    assert!(walk_mark(&app, player).is_none(), "the walk went on after the hit");
    assert!(app.world().get::<PetBeat>(player).is_none(), "the pet went on after the hit");
    assert!(
        !gesture_held(&app, player) && !gesture_held(&app, dog),
        "the interrupted pet kept a body held"
    );
    app.world_mut().get_mut::<BodyKinematics>(player).unwrap().pos.x = mark.x;
    app.update();
    assert_eq!(
        (anim(&app, player).petting, anim(&app, dog).petted),
        (false, false),
        "an interrupted pet started its gesture anyway"
    );
}

/// A hit during the gesture stops it on both bodies.
#[test]
fn a_hit_during_the_gesture_interrupts_the_pet() {
    let mut app = app();
    let dog_at = ae::Vec2::new(100.0, 100.0);
    let player = spawn_player(&mut app, dog_at + ae::Vec2::new(30.0, 0.0));
    let dog = spawn_character(&mut app, dog_at, "good_dog");
    ask_for_a_pet(&mut app, "good_dog");
    app.update();
    arrive(&mut app, player);
    app.update();
    assert!(anim(&app, dog).petted, "control: the gesture plays");

    knock(&mut app, dog);
    app.update();

    assert_eq!(
        (anim(&app, player).petting, anim(&app, dog).petted),
        (false, false),
        "the gesture played on through a hit"
    );
    assert!(
        !gesture_held(&app, player) && !gesture_held(&app, dog),
        "the interrupted gesture kept a body held"
    );
}

/// A walk that cannot arrive (a wall, a ledge) gives up and lets both go.
#[test]
fn a_walk_that_does_not_arrive_gives_up() {
    let mut app = app();
    let dog_at = ae::Vec2::new(100.0, 100.0);
    let player = spawn_player(&mut app, dog_at + ae::Vec2::new(30.0, 0.0));
    let dog = spawn_character(&mut app, dog_at, "good_dog");
    ask_for_a_pet(&mut app, "good_dog");
    app.update();
    assert!(gesture_held(&app, player), "control: the walk holds the petter");

    // No integrator runs here, so the petter never moves.
    for _ in 0..(60.0 * (1.0 + 60.0 / PET_WALK_SPEED)) as usize + 2 {
        app.update();
    }

    assert!(app.world().get::<PetBeat>(player).is_none(), "a blocked walk never gave up");
    assert!(
        !gesture_held(&app, player) && !gesture_held(&app, dog),
        "a blocked walk kept a body held"
    );
}

#[test]
fn a_character_without_a_petting_row_is_not_petted() {
    let mut app = app();
    let at = ae::Vec2::new(100.0, 100.0);
    let player = spawn_player(&mut app, at + ae::Vec2::new(30.0, 0.0));
    let shopkeeper = spawn_character(&mut app, at, "shopkeeper");

    ask_for_a_pet(&mut app, "shopkeeper");
    app.update();

    assert!(!anim(&app, player).petting);
    assert!(!anim(&app, shopkeeper).petted);
    assert!(!gesture_held(&app, player));
}

/// A pet asked for while one is running does not restart it.
#[test]
fn a_second_pet_during_the_first_changes_nothing() {
    let mut app = app();
    let dog_at = ae::Vec2::new(100.0, 100.0);
    let player = spawn_player(&mut app, dog_at + ae::Vec2::new(30.0, 0.0));
    let dog = spawn_character(&mut app, dog_at, "good_dog");
    ask_for_a_pet(&mut app, "good_dog");
    app.update();
    arrive(&mut app, player);
    app.update();
    let stage = |app: &App| app.world().get::<PetBeat>(player).map(|beat| beat.stage);
    let Some(PetStage::Gesture { remaining }) = stage(&app) else {
        panic!("control: the first pet is in its gesture");
    };

    ask_for_a_pet(&mut app, "good_dog");
    app.update();

    assert_eq!(
        stage(&app),
        Some(PetStage::Gesture { remaining: remaining - 1.0 / 60.0 }),
        "the second pet restarted the running one"
    );
    assert!(anim(&app, dog).petted);
}

/// The projection lets go of the gesture's own hold bit and no other.
#[test]
fn the_gesture_hold_lets_go_when_the_pet_ends_and_only_its_own_bit() {
    let mut app = App::new();
    app.add_systems(Update, project_pet_holds);
    let body = app
        .world_mut()
        .spawn((
            BodyAnimFacts::default(),
            PetBeat {
                petted: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(sim_id("good_dog"), None),
                mark_x: 0.0,
                side: 1.0,
                stage: PetStage::Gesture { remaining: 1.0 },
            },
            ControlHolds::only(ControlHold::Conversation),
        ))
        .id();

    app.update();
    assert!(gesture_held(&app, body), "a running pet holds the body");
    assert!(anim(&app, body).petting, "and poses it");

    app.world_mut().entity_mut(body).remove::<PetBeat>();
    app.update();
    let holds = app
        .world()
        .get::<ControlHolds>(body)
        .expect("the conversation still holds it");
    assert!(!holds.holds(ControlHold::Gesture), "the pet let go");
    assert!(!anim(&app, body).petting, "the pose went with the beat");
    assert!(
        holds.holds(ControlHold::Conversation),
        "and left the conversation's hold alone"
    );
}

/// A pet in its gesture, for the despawn arms below.
fn a_pet_in_its_gesture() -> (App, Entity, Entity) {
    let mut app = app();
    let dog_at = ae::Vec2::new(100.0, 100.0);
    let player = spawn_player(&mut app, dog_at + ae::Vec2::new(30.0, 0.0));
    let dog = spawn_character(&mut app, dog_at, "good_dog");
    ask_for_a_pet(&mut app, "good_dog");
    app.update();
    arrive(&mut app, player);
    app.update();
    assert!(
        anim(&app, player).petting && anim(&app, dog).petted,
        "control: the gesture plays on both bodies"
    );
    assert!(gesture_held(&app, player) && gesture_held(&app, dog), "control: both are held");
    (app, player, dog)
}

/// The petted body goes away during the gesture: the petter is let go on
/// the next tick, and nothing of the pet is left on it.
#[test]
fn the_petter_is_let_go_when_the_petted_body_goes_away() {
    let (mut app, player, dog) = a_pet_in_its_gesture();
    app.world_mut().despawn(dog);
    app.update();
    assert!(app.world().get::<PetBeat>(player).is_none(), "the beat outlived the dog");
    assert!(!anim(&app, player).petting, "the petter kept petting nothing");
    assert!(!gesture_held(&app, player), "the petter kept its hold");
}

/// The petter goes away during the gesture: the petted body is let go on the
/// next tick. It has no pet state of its own to run down.
#[test]
fn the_petted_body_is_let_go_when_the_petter_goes_away() {
    let (mut app, player, dog) = a_pet_in_its_gesture();
    app.world_mut().despawn(player);
    app.update();
    assert!(!anim(&app, dog).petted, "the dog kept being petted by nobody");
    assert!(!gesture_held(&app, dog), "the dog kept its hold");
}
