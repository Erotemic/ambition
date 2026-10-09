//! WORLD-ACCEPTANCE, first slice: Alice's note reaches Bob by one road.
//!
//! The intro quest `intro_cartography_route` reads two flags. Each flag had a
//! second writer: a `PickupSpawn` beside each NPC set it with no note handed
//! over. Now the hand-over in the dialogue is the only writer, so the quest
//! cannot advance unless the note is in the bag and then in Bob's hands.
//!
//! The test drives the shipped shell session with real key presses: Interact
//! starts the conversation and `MenuSelect` (Enter) takes the first choice.

use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::engine_core::BodyKinematics;
use bevy::prelude::{App, Entity, KeyCode};
use leafwing_input_manager::prelude::Buttonlike;

use crate::neighbor_prefetch_prepares_rooms::{alice as primary_body, cross, room_of};

pub(crate) const QUEST: &str = "intro_cartography_route";
pub(crate) const NOTE_FLAG: &str = "alice_route_note_carried";
pub(crate) const SURVEY_FLAG: &str = "bob_field_survey_received";

/// The shipped shell session, for a player who has seen the hub's intro: the
/// intro cutscene holds the seat's input while it plays.
pub(crate) fn a_returning_players_session() -> App {
    a_returning_players_session_with_pads(0)
}

/// [`a_returning_players_session`] with `pads` gamepads connected before the
/// gameplay route starts. Ambition seats one player per connected pad, and the
/// session opens its handles when it starts (`declare_ambition_seating`).
pub(crate) fn a_returning_players_session_with_pads(pads: usize) -> App {
    use ambition_app::app::{build_visible_app, VisibleRenderMode};
    use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.finish();
    app.update();
    for _ in 0..pads {
        app.world_mut().spawn(bevy::input::gamepad::Gamepad::default());
        app.update();
    }
    crate::common::the_hub_intro_has_already_played(app.world_mut());
    app.world_mut().write_message(ShellCommand::ReplaceWith {
        route: ShellRouteId::new("ambition_gameplay"),
        request: None,
    });
    crate::common::step_until_route_is_active_and_settled(&mut app, "ambition_gameplay");
    app
}

pub(crate) fn tap(app: &mut App, key: KeyCode) {
    Buttonlike::press(&key, app.world_mut());
    app.update();
    Buttonlike::release(&key, app.world_mut());
    app.update();
}

pub(crate) fn npc(app: &mut App, id: &str) -> Entity {
    let world = app.world_mut();
    world
        .query::<(Entity, &WornCharacter)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == id)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("the room stages `{id}`"))
}

pub(crate) fn put_body_at(app: &mut App, body: Entity, at: ambition_platformer2d::engine_core::Vec2) {
    use ambition_platformer2d::engine_core as ae;
    let mut state = bevy::ecs::system::SystemState::<
        bevy::prelude::Query<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel)>,
    >::new(app.world_mut());
    let mut bodies = state.get_mut(app.world_mut()).expect("the body query is valid");
    let (mut clusters, mut model) = bodies.get_mut(body).expect("the player has a body");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
    state.apply(app.world_mut());
}

pub(crate) fn dialog_active(app: &App) -> bool {
    app.world()
        .resource::<ambition_platformer2d::dialog::DialogState>()
        .active()
}

pub(crate) fn first_choice(app: &App) -> Option<String> {
    app.world()
        .resource::<ambition_platformer2d::dialog::DialogState>()
        .options()
        .first()
        .map(|choice| choice.label.clone())
}

/// Stand beside `npc_id`, press Interact, and take the first choice when its
/// label is `choice`. Then press through the rest of the conversation.
pub(crate) fn talk_and_choose(app: &mut App, npc_id: &str, choice: &str) {
    let npc = npc(app, npc_id);
    let at = app.world().get::<BodyKinematics>(npc).expect("a live NPC").pos;
    let body = primary_body(app);
    put_body_at(app, body, at);
    for _ in 0..10 {
        app.update();
    }
    tap(app, KeyCode::KeyF);
    let mut opened = false;
    for _ in 0..30 {
        app.update();
        if dialog_active(app) {
            opened = true;
            break;
        }
    }
    assert!(opened, "Interact beside `{npc_id}` did not open its dialogue");
    let mut offered = None;
    for _ in 0..20 {
        if let Some(label) = first_choice(app) {
            offered = Some(label);
            break;
        }
        tap(app, KeyCode::Enter);
    }
    assert_eq!(offered.as_deref(), Some(choice), "`{npc_id}` did not offer `{choice}` first");
    tap(app, KeyCode::Enter);
    for _ in 0..20 {
        if !dialog_active(app) {
            break;
        }
        tap(app, KeyCode::Enter);
    }
    assert!(!dialog_active(app), "the conversation with `{npc_id}` did not end");
    for _ in 0..5 {
        app.update();
    }
}

pub(crate) fn flag(app: &App, id: &str) -> bool {
    app.world()
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data()
        .flag(id)
}

pub(crate) fn bag(app: &App, dialog_id: &str) -> u32 {
    let item = ambition_platformer2d::items::item_catalog(app.world())
        .item_by_dialog_id(dialog_id)
        .unwrap_or_else(|| panic!("the catalog names `{dialog_id}`"));
    app.world()
        .resource::<ambition_platformer2d::item::OwnedItems>()
        .count(item)
}

pub(crate) fn quest_step(app: &App) -> Option<u8> {
    app.world()
        .resource::<ambition_content::quest::QuestRegistry>()
        .get(QUEST)
        .map(|state| state.step)
}

/// ⭐ THE NOTE IS THE ROAD. Each fact the quest reads is written by the
/// hand-over that moves the note, and by nothing the player walks into.
///
/// Premise: in each relay room, standing where the old pickup was, for long
/// enough to touch it, writes no flag. Then Alice's choice puts one note in the
/// bag and advances the quest one step. Bob's choice takes the note, gives the
/// survey, and advances it again.
#[test]
fn the_note_travels_from_alice_to_bob() {
    let mut app = a_returning_players_session();
    let body = primary_body(&mut app);
    cross(&mut app, body, PlayerSlot(0), "alice_relay");
    assert_eq!(room_of(&app, body).as_deref(), Some("alice_relay"));
    let start_step = quest_step(&app).expect("the intro quest starts by itself");

    // Where the old pickup of the note was (`alice_relay`, px 400,696).
    put_body_at(&mut app, body, ambition_platformer2d::engine_core::Vec2::new(412.0, 684.0));
    for _ in 0..30 {
        app.update();
    }
    assert!(!flag(&app, NOTE_FLAG), "the note's flag was set with no note handed over");

    talk_and_choose(&mut app, "npc_alice", "Take the note.");
    assert_eq!(
        (bag(&app, "sealednote"), flag(&app, NOTE_FLAG), quest_step(&app)),
        (1, true, Some(start_step + 1)),
        "(notes in the bag, note flag, quest step) after Alice's choice"
    );

    cross(&mut app, body, PlayerSlot(0), "bob_relay");
    // Where the old pickup of the survey was (`bob_relay`, px 880,584).
    put_body_at(&mut app, body, ambition_platformer2d::engine_core::Vec2::new(892.0, 572.0));
    for _ in 0..30 {
        app.update();
    }
    assert!(!flag(&app, SURVEY_FLAG), "the survey's flag was set with no note handed over");

    talk_and_choose(&mut app, "npc_bob", "Hand him the sealed note.");
    assert_eq!(
        (
            bag(&app, "sealednote"),
            bag(&app, "fieldsurvey"),
            flag(&app, SURVEY_FLAG),
            quest_step(&app)
        ),
        (0, 1, true, Some(start_step + 2)),
        "(notes, surveys, survey flag, quest step) after Bob's choice"
    );
}
