use super::*;
use ambition_combat::components::{CenteredAabb, FeatureId, FeatureName};
use ambition_encounter::switches::{SwitchFeature, SwitchOn};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use bevy::prelude::{App, NextState, Update};

fn spawn_interaction_player(app: &mut App, pos: ae::Vec2) -> Entity {
    let scratch = crate::avatar::primary_player_scratch(pos, ae::AbilitySet::sandbox_all());
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        scratch,
        ambition_characters::actor::Health::new(10),
    );
    let player = app.world_mut().spawn(bundle).id();
    // The interact buffer is SLOT state now (published from the device); prime
    // the primary controller's slot so the system sees a live buffered interact.
    app.world_mut()
        .get_resource_or_insert_with(ambition_characters::control::SlotInteractionState::default)
        .primary_mut()
        .interact_buffer_timer = 0.15;
    player
}

/// Like [`spawn_interaction_player`], but also gives the home avatar the canonical
/// `WornCharacter` identity `simulation_world` attaches in production — so dialogue
/// speaks as the ENTITY's worn character, not an app-local resource.
fn spawn_interaction_player_wearing(app: &mut App, pos: ae::Vec2, worn: &str) -> Entity {
    let player = spawn_interaction_player(app, pos);
    app.world_mut()
        .entity_mut(player)
        .insert(ambition_characters::actor::WornCharacter::new(worn));
    player
}

#[test]
fn buffered_interact_toggles_an_adjacent_switch() {
    let center = ae::Vec2::new(100.0, 100.0);
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_dialog::DialogState::default());
    //  the AUTHORITY travels with the read-model. `interact_ecs_actors_and_
    // switches` opens a conversation in the simulation and shows it in the UI,
    // so a fixture with only the second half fails Bevy's param validation.
    //  NOT solved by making the param `Option`: that waiver would answer "may
    // this be absent" when the question is who OWNS registering it, and in
    // production the feature plugin does.
    app.init_resource::<ambition_conversation::ActiveConversation>();
    app.init_resource::<ambition_dialog::DialogueNodeIndex>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        <crate::avatar::StartingCharacter>::default(),
    );
    app.insert_resource(NextState::<
        ambition_platformer2d_shared_tangle::schedule::GameMode,
    >::default());
    app.add_message::<SetFlagRequested>();
    app.add_message::<QuestAdvanceRequested>();
    app.add_message::<SwitchActivated>();
    app.add_message::<VfxMessage>();
    spawn_interaction_player(&mut app, center);

    let switch = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("gate_switch"),
            FeatureName::new("Gate Switch"),
            CenteredAabb::from_center_size(center, ae::Vec2::new(24.0, 24.0)),
            SwitchFeature::new(ambition_encounter::SwitchActivation {
                id: "gate_switch".into(),
                action: "open".into(),
                target_encounter: String::new(),
            }),
            SwitchOn(false),
        ))
        .id();

    //  the box is a PROJECTION now, so the projection has to run. The
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

    assert!(
        app.world().get::<SwitchOn>(switch).unwrap().0,
        "a buffered interact on an adjacent switch should toggle it on"
    );
}

#[test]
fn interact_lands_on_the_controlled_subject_not_the_vacated_home_avatar() {
    use ambition_platformer2d_core::BodyKinematics;
    use ambition_platformer2d_shared_tangle::markers::ControlledSubject;

    let home_pos = ae::Vec2::new(0.0, 0.0);
    let subject_pos = ae::Vec2::new(600.0, 0.0);

    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_dialog::DialogState::default());
    //  the AUTHORITY travels with the read-model. `interact_ecs_actors_and_
    // switches` opens a conversation in the simulation and shows it in the UI,
    // so a fixture with only the second half fails Bevy's param validation.
    //  NOT solved by making the param `Option`: that waiver would answer "may
    // this be absent" when the question is who OWNS registering it, and in
    // production the feature plugin does.
    app.init_resource::<ambition_conversation::ActiveConversation>();
    app.init_resource::<ambition_dialog::DialogueNodeIndex>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        <crate::avatar::StartingCharacter>::default(),
    );
    app.insert_resource(NextState::<
        ambition_platformer2d_shared_tangle::schedule::GameMode,
    >::default());
    app.add_message::<SetFlagRequested>();
    app.add_message::<QuestAdvanceRequested>();
    app.add_message::<SwitchActivated>();
    app.add_message::<VfxMessage>();

    // Slot-0 input surface: the home avatar, far from the switch, with a
    // buffered interact press from the device.
    spawn_interaction_player(&mut app, home_pos);

    // The possessed body the player is DRIVING, standing on the switch.
    let subject = app
        .world_mut()
        .spawn(BodyKinematics {
            pos: subject_pos,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(24.0, 40.0),
            facing: 1.0,
        })
        .id();
    app.insert_resource(ControlledSubject(Some(subject)));

    // A switch next to the DRIVEN body...
    let near_subject = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("subject_switch"),
            FeatureName::new("Subject Switch"),
            CenteredAabb::from_center_size(subject_pos, ae::Vec2::new(24.0, 24.0)),
            SwitchFeature::new(ambition_encounter::SwitchActivation {
                id: "subject_switch".into(),
                action: "open".into(),
                target_encounter: String::new(),
            }),
            SwitchOn(false),
        ))
        .id();

    // ...and a decoy next to the vacated home avatar, which must NOT fire.
    let near_home = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new("home_switch"),
            FeatureName::new("Home Switch"),
            CenteredAabb::from_center_size(home_pos, ae::Vec2::new(24.0, 24.0)),
            SwitchFeature::new(ambition_encounter::SwitchActivation {
                id: "home_switch".into(),
                action: "open".into(),
                target_encounter: String::new(),
            }),
            SwitchOn(false),
        ))
        .id();

    //  the box is a PROJECTION now, so the projection has to run. The
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

    assert!(
        app.world().get::<SwitchOn>(near_subject).unwrap().0,
        "interact should activate the switch next to the CONTROLLED body"
    );
    assert!(
        !app.world().get::<SwitchOn>(near_home).unwrap().0,
        "interact must NOT reach the switch next to the vacated home avatar"
    );
}

/// Spawn a talkable Hall-style pedestal: a peaceful actor that IS a catalog
/// character (`character_id`) and offers a dialogue node.
fn spawn_pedestal(app: &mut App, pos: ae::Vec2, character_id: &str, dialogue_id: &str) -> Entity {
    let interactable = ambition_interaction::Interactable::new(
        "hall_pedestal_placement",
        "Talk",
        ae::Aabb::new(pos, ae::Vec2::new(24.0, 40.0)),
        ambition_interaction::InteractionKind::Npc {
            character_id: Some(character_id.to_string()),
            dialogue_id: Some(dialogue_id.to_string()),
            patrol_radius: 0.0,
            patrol_path_id: None,
            brain_override: None,
        },
    );
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            CenteredAabb::from_center_size(pos, ae::Vec2::new(24.0, 40.0)),
            ActorDisposition::Peaceful,
            ActorIdentity::new("hall_pedestal_placement", "Player"),
            ActorInteraction {
                interactable,
                talk_radius: 40.0,
            },
        ))
        .id()
}

fn dialogue_app(nodes: &[&str]) -> App {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_dialog::DialogState::default());
    //  the AUTHORITY travels with the read-model. `interact_ecs_actors_and_
    // switches` opens a conversation in the simulation and shows it in the UI,
    // so a fixture with only the second half fails Bevy's param validation.
    //  NOT solved by making the param `Option`: that waiver would answer "may
    // this be absent" when the question is who OWNS registering it, and in
    // production the feature plugin does.
    app.init_resource::<ambition_conversation::ActiveConversation>();
    let mut index = ambition_dialog::DialogueNodeIndex::default();
    index.populate(nodes.iter().map(|n| (*n).to_string()));
    app.insert_resource(index);
    app.insert_resource(NextState::<
        ambition_platformer2d_shared_tangle::schedule::GameMode,
    >::default());
    app.add_message::<SetFlagRequested>();
    app.add_message::<QuestAdvanceRequested>();
    app.add_message::<SwitchActivated>();
    app.add_message::<VfxMessage>();
    app
}

/// Wearing a DIFFERENT character: an ordinary conversation, on the node the
/// pedestal authored.
#[test]
fn a_visitor_gets_the_pedestals_ordinary_node() {
    let center = ae::Vec2::new(100.0, 100.0);
    let mut app = dialogue_app(&["hall_player", "hall_player__self"]);
    spawn_interaction_player_wearing(&mut app, center, "goblin");
    spawn_pedestal(&mut app, center, "player_robot_v3", "hall_player");

    //  the box is a PROJECTION now, so the projection has to run. The
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

    let state = app.world().resource::<ambition_dialog::DialogState>();
    assert!(state.active());
    assert_eq!(state.dialogue_id(), "hall_player");
}

/// Wearing the pedestal's OWN character, with a self branch authored: the
/// engine enters that branch instead.
#[test]
fn wearing_the_pedestals_character_enters_the_self_branch() {
    let center = ae::Vec2::new(100.0, 100.0);
    let mut app = dialogue_app(&["hall_player", "hall_player__self"]);
    spawn_interaction_player_wearing(&mut app, center, "player_robot_v3");
    spawn_pedestal(&mut app, center, "player_robot_v3", "hall_player");

    //  the box is a PROJECTION now, so the projection has to run. The
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

    let state = app.world().resource::<ambition_dialog::DialogState>();
    assert!(state.active());
    assert_eq!(
        state.dialogue_id(),
        "hall_player__self",
        "the speaker IS the listener, and content authored a self branch"
    );
}

/// The engine default. Wearing the pedestal's character with NO self branch
/// authored: the interaction never happens — and leaves no trace. Not a
/// dialogue that opens and closes, not a consumed press, not a quest event.
#[test]
fn self_talk_without_a_self_branch_is_suppressed_without_a_trace() {
    let center = ae::Vec2::new(100.0, 100.0);
    let mut app = dialogue_app(&["hall_player"]);
    spawn_interaction_player_wearing(&mut app, center, "player_robot_v3");
    spawn_pedestal(&mut app, center, "player_robot_v3", "hall_player");

    // Pre-poison: if the system returns early for the WRONG reason, these
    // stay as-set and the assertions below would pass vacuously.
    app.world_mut()
        .resource_mut::<GameplayBanner>()
        .show("sentinel", 9.0);

    //  the box is a PROJECTION now, so the projection has to run. The
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

    let world = app.world();
    assert!(
        !world.resource::<ambition_dialog::DialogState>().active(),
        "no conversation may open"
    );
    assert_eq!(
        world.resource::<GameplayBanner>().text.as_str(),
        "sentinel",
        "no banner may be shown — the interaction did not happen"
    );
    assert!(
        world
            .resource::<ambition_characters::control::SlotInteractionState>()
            .primary()
            .buffered(),
        "the buffered press is NOT consumed: the player may still interact \
         with something else"
    );
    let quests = world.resource::<bevy::ecs::message::Messages<QuestAdvanceRequested>>();
    assert_eq!(quests.len(), 0, "no `NpcTalked` may fire");
    let flags = world.resource::<bevy::ecs::message::Messages<SetFlagRequested>>();
    assert_eq!(flags.len(), 0, "no `..._talked` flag may be set");
}

/// An index that never saw a compiled Yarn project (headless, RL, the frames
/// before the runner spawns) must not swallow interactions.
#[test]
fn an_unpopulated_node_index_never_suppresses() {
    let center = ae::Vec2::new(100.0, 100.0);
    let mut app = dialogue_app(&[]);
    app.insert_resource(ambition_dialog::DialogueNodeIndex::default());
    spawn_interaction_player_wearing(&mut app, center, "player_robot_v3");
    spawn_pedestal(&mut app, center, "player_robot_v3", "hall_player");

    //  the box is a PROJECTION now, so the projection has to run. The
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

    let state = app.world().resource::<ambition_dialog::DialogState>();
    assert!(state.active(), "not knowing is not grounds for suppressing");
    assert_eq!(state.dialogue_id(), "hall_player");
}

// The geometry half was already right — `interact_lands_on_the_controlled_
// subject_not_the_vacated_home_avatar` above pins it. Two halves were not: the interaction POSE
// was written unconditionally to whatever carried `PrimaryPlayer`, and the buffered press was
// read from and cleared on SLOT 0 whichever seat was actually driving.

/// The resources `interact_ecs_actors_and_switches` needs, and nothing else.
fn interaction_app() -> App {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_dialog::DialogState::default());
    app.init_resource::<ambition_conversation::ActiveConversation>();
    app.init_resource::<ambition_dialog::DialogueNodeIndex>();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        <crate::avatar::StartingCharacter>::default(),
    );
    app.insert_resource(NextState::<
        ambition_platformer2d_shared_tangle::schedule::GameMode,
    >::default());
    app.add_message::<SetFlagRequested>();
    app.add_message::<QuestAdvanceRequested>();
    app.add_message::<SwitchActivated>();
    app.add_message::<VfxMessage>();
    //  the SEAT is spawned on the body, because it IS the input road. `ActingParticipant`
    // answers *which seat drives this body* off `DrivingParticipant`, and a fixture whose
    // bodies carried no seat would hand every reader `None` — which `acting_slot` turns into
    // `PRIMARY`.
    //
    //  the possession reconcile is deliberately NOT here: it only moves the
    // primary seat between a home avatar and a possessed body, and no possession
    // happens in this fixture.
    app.add_systems(Update, interact_ecs_actors_and_switches);
    app
}

fn spawn_switch(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new(id),
            FeatureName::new(id),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(24.0, 24.0)),
            SwitchFeature::new(ambition_encounter::SwitchActivation {
                id: id.into(),
                action: "open".into(),
                target_encounter: String::new(),
            }),
            SwitchOn(false),
        ))
        .id()
}

/// A body a seat is driving: kinematics, a pose to play, and the seat that says
/// WHOSE body it is.
fn spawn_driven_body(app: &mut App, pos: ae::Vec2, slot: u8) -> Entity {
    app.world_mut()
        .spawn((
            ambition_platformer2d_core::BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ambition_characters::actor::BodyAnimFacts::default(),
            ambition_characters::control::DrivingParticipant(
                ambition_characters::control::PlayerSlot(slot),
            ),
        ))
        .id()
}

fn buffer_interact(app: &mut App, slot: u8, secs: f32) {
    let mut state = app
        .world_mut()
        .get_resource_or_insert_with(ambition_characters::control::SlotInteractionState::default);
    state
        .get_mut(ambition_characters::control::PlayerSlot(slot))
        .expect("slot is in range")
        .interact_buffer_timer = secs;
}

fn buffered_secs(app: &App, slot: u8) -> f32 {
    app.world()
        .resource::<ambition_characters::control::SlotInteractionState>()
        .get(ambition_characters::control::PlayerSlot(slot))
        .interact_buffer_timer
}

/// The body that acted plays the pose; the body left behind plays nothing.
#[test]
fn the_interact_pose_lands_on_the_body_that_acted() {
    use ambition_platformer2d_shared_tangle::markers::ControlledSubject;

    let home_pos = ae::Vec2::new(0.0, 0.0);
    let subject_pos = ae::Vec2::new(600.0, 0.0);

    let mut app = interaction_app();
    let home = spawn_interaction_player(&mut app, home_pos);
    let subject = spawn_driven_body(&mut app, subject_pos, 0);
    app.insert_resource(ControlledSubject(Some(subject)));
    let switch = spawn_switch(&mut app, "subject_switch", subject_pos);

    app.update();

    assert!(
        app.world().get::<SwitchOn>(switch).unwrap().0,
        "the interaction did not happen at all, so neither pose assertion below \
         could have failed"
    );
    assert!(
        app.world()
            .get::<ambition_characters::actor::BodyAnimFacts>(subject)
            .unwrap()
            .interact_anim_timer
            > 0.0,
        "the possessed body did the interacting and plays no reach-and-open pose"
    );
    assert_eq!(
        app.world()
            .get::<ambition_characters::actor::BodyAnimFacts>(home)
            .unwrap()
            .interact_anim_timer,
        0.0,
        "the VACATED home avatar played the interaction animation: the pose was \
         written to whatever carries `PrimaryPlayer` rather than to the body that acted"
    );
}

/// A second seat interacts with its OWN press.
///
///  the discriminating half is what is left behind: seat 0's press is still
/// buffered afterwards, because seat 1 spent seat 1's.
#[test]
fn a_second_seat_spends_its_own_buffered_interact() {
    use ambition_platformer2d_shared_tangle::markers::ControlledSubject;

    let subject_pos = ae::Vec2::new(600.0, 0.0);
    let mut app = interaction_app();
    // The home avatar exists and is holding a press of its own, which must
    // survive untouched.
    spawn_interaction_player(&mut app, ae::Vec2::new(0.0, 0.0));
    buffer_interact(&mut app, 0, 0.15);
    buffer_interact(&mut app, 1, 0.15);

    let subject = spawn_driven_body(&mut app, subject_pos, 1);
    app.insert_resource(ControlledSubject(Some(subject)));
    let switch = spawn_switch(&mut app, "subject_switch", subject_pos);

    app.update();

    assert!(
        app.world().get::<SwitchOn>(switch).unwrap().0,
        "seat 1's body was in reach with a live buffered press and nothing happened"
    );
    assert_eq!(
        buffered_secs(&app, 1),
        0.0,
        "the acting seat's press was not spent"
    );
    assert_eq!(
        buffered_secs(&app, 0),
        0.15,
        "seat 1's interaction consumed SEAT 0's buffered press — the system read and \
         cleared slot 0 whichever seat was actually driving, so a co-op partner's \
         queued interact vanished when somebody else opened a door"
    );
}

/// The negative direction, with positive evidence that the road is live.
#[test]
fn a_seat_that_pressed_nothing_does_not_interact_on_another_seats_press() {
    use ambition_platformer2d_shared_tangle::markers::ControlledSubject;

    let subject_pos = ae::Vec2::new(600.0, 0.0);
    let mut app = interaction_app();
    spawn_interaction_player(&mut app, ae::Vec2::new(0.0, 0.0));
    buffer_interact(&mut app, 0, 0.15);
    buffer_interact(&mut app, 1, 0.0);

    let subject = spawn_driven_body(&mut app, subject_pos, 1);
    app.insert_resource(ControlledSubject(Some(subject)));
    let switch = spawn_switch(&mut app, "subject_switch", subject_pos);

    app.update();

    assert!(
        !app.world().get::<SwitchOn>(switch).unwrap().0,
        "seat 0's press worked a switch that only seat 1's body was standing on"
    );

    //  the road is live: the same fixture, with seat 1 pressing, DOES fire.
    // Without this the assertion above would also pass on a world where the
    // press could never reach the simulation at all.
    buffer_interact(&mut app, 1, 0.15);
    app.update();
    assert!(
        app.world().get::<SwitchOn>(switch).unwrap().0,
        "seat 1's own press did not work its own switch either, so the assertion \
         above proved nothing about WHOSE press was read"
    );
}

/// ⭐⭐ TWO DRIVEN BODIES EACH FLIP THEIR OWN SWITCH, IN ONE TICK.
///
/// ⛔⛔ THIS SYSTEM RESOLVED ONE `ControlledSubject`, so on a couch stage the
/// second seat could stand on a switch and press interact forever. The gesture
/// half was already per-body — `ActingParticipant` keys the buffered interact
/// off the body's own driving slot — and only the SUBJECT was singular.
///
/// ⛔ AND THE SWITCH LOOP'S `return` ENDED THE SYSTEM. "Once we flip one we
/// stop" is right PER BODY and wrong for the population: seat a flipping its
/// switch would have stopped seat b flipping a different one. It is a `break`
/// out of the switch loop now.
#[test]
fn two_driven_bodies_each_flip_their_own_switch() {
    use ambition_characters::control::{PlayerSlot, SlotInteractionState};

    let mut app = interaction_app();
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
    {
        let mut gestures = app
            .world_mut()
            .get_resource_or_insert_with(SlotInteractionState::default);
        gestures.primary_mut().interact_buffer_timer = 0.5;
        if let Some(second) = gestures.get_mut(PlayerSlot(1)) {
            second.interact_buffer_timer = 0.5;
        }
    }

    let _a = spawn_driven_body(&mut app, ae::Vec2::new(100.0, 100.0), 0);
    let _b = spawn_driven_body(&mut app, ae::Vec2::new(900.0, 100.0), 1);
    let switch_a = spawn_switch(&mut app, "switch_a", ae::Vec2::new(100.0, 100.0));
    let switch_b = spawn_switch(&mut app, "switch_b", ae::Vec2::new(900.0, 100.0));

    app.update();

    assert!(
        app.world().get::<SwitchOn>(switch_a).is_some_and(|on| on.0),
        "seat a's switch stayed off"
    );
    assert!(
        app.world().get::<SwitchOn>(switch_b).is_some_and(|on| on.0),
        "seat b's switch stayed off"
    );
}
