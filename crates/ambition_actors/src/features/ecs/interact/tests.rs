//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::features::{
    CenteredAabb, FeatureId, FeatureName, FeatureSimEntity, SwitchFeature, SwitchOn,
};
use ambition_engine_core as ae;
use bevy::prelude::{App, NextState, Update};

fn spawn_interaction_player(app: &mut App, pos: ae::Vec2) {
    let scratch = crate::avatar::primary_player_scratch(pos, ae::AbilitySet::sandbox_all());
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        scratch,
        ambition_characters::actor::Health::new(10),
    );
    app.world_mut().spawn(bundle);
    // The interact buffer is SLOT state now (published from the device); prime
    // the primary controller's slot so the system sees a live buffered interact.
    app.world_mut()
        .get_resource_or_insert_with(crate::control::SlotInteractionState::default)
        .primary_mut()
        .interact_buffer_timer = 0.15;
}

#[test]
fn buffered_interact_toggles_an_adjacent_switch() {
    let center = ae::Vec2::new(100.0, 100.0);
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_dialog::DialogState::default());
    app.init_resource::<ambition_dialog::DialogueNodeIndex>();
    app.init_resource::<crate::avatar::StartingCharacter>();
    app.insert_resource(NextState::<
        ambition_platformer_primitives::schedule::GameMode,
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
            SwitchFeature::new(crate::encounter::SwitchActivation {
                id: "gate_switch".into(),
                action: "open".into(),
                target_encounter: String::new(),
            }),
            SwitchOn(false),
        ))
        .id();

    app.add_systems(Update, interact_ecs_actors_and_switches);
    app.update();

    assert!(
        app.world().get::<SwitchOn>(switch).unwrap().0,
        "a buffered interact on an adjacent switch should toggle it on"
    );
}

#[test]
fn interact_lands_on_the_controlled_subject_not_the_vacated_home_avatar() {
    use crate::actor::BodyKinematics;
    use ambition_platformer_primitives::markers::ControlledSubject;

    let home_pos = ae::Vec2::new(0.0, 0.0);
    let subject_pos = ae::Vec2::new(600.0, 0.0);

    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_dialog::DialogState::default());
    app.init_resource::<ambition_dialog::DialogueNodeIndex>();
    app.init_resource::<crate::avatar::StartingCharacter>();
    app.insert_resource(NextState::<
        ambition_platformer_primitives::schedule::GameMode,
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
            SwitchFeature::new(crate::encounter::SwitchActivation {
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
            SwitchFeature::new(crate::encounter::SwitchActivation {
                id: "home_switch".into(),
                action: "open".into(),
                target_encounter: String::new(),
            }),
            SwitchOn(false),
        ))
        .id();

    app.add_systems(Update, interact_ecs_actors_and_switches);
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

fn dialogue_app(worn: &str, nodes: &[&str]) -> App {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_dialog::DialogState::default());
    let mut index = ambition_dialog::DialogueNodeIndex::default();
    index.populate(nodes.iter().map(|n| (*n).to_string()));
    app.insert_resource(index);
    app.insert_resource(crate::avatar::StartingCharacter::new(worn));
    app.insert_resource(NextState::<
        ambition_platformer_primitives::schedule::GameMode,
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
    let mut app = dialogue_app("goblin", &["hall_player", "hall_player__self"]);
    spawn_interaction_player(&mut app, center);
    spawn_pedestal(&mut app, center, "player", "hall_player");

    app.add_systems(Update, interact_ecs_actors_and_switches);
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
    let mut app = dialogue_app("player", &["hall_player", "hall_player__self"]);
    spawn_interaction_player(&mut app, center);
    spawn_pedestal(&mut app, center, "player", "hall_player");

    app.add_systems(Update, interact_ecs_actors_and_switches);
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
    let mut app = dialogue_app("player", &["hall_player"]);
    spawn_interaction_player(&mut app, center);
    spawn_pedestal(&mut app, center, "player", "hall_player");

    // Pre-poison: if the system returns early for the WRONG reason, these
    // stay as-set and the assertions below would pass vacuously.
    app.world_mut()
        .resource_mut::<GameplayBanner>()
        .show("sentinel", 9.0);

    app.add_systems(Update, interact_ecs_actors_and_switches);
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
            .resource::<crate::control::SlotInteractionState>()
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
    let mut app = dialogue_app("player", &[]);
    app.insert_resource(ambition_dialog::DialogueNodeIndex::default());
    spawn_interaction_player(&mut app, center);
    spawn_pedestal(&mut app, center, "player", "hall_player");

    app.add_systems(Update, interact_ecs_actors_and_switches);
    app.update();

    let state = app.world().resource::<ambition_dialog::DialogState>();
    assert!(state.active(), "not knowing is not grounds for suppressing");
    assert_eq!(state.dialogue_id(), "hall_player");
}

/// A body with a character identity speaks as that character, not as its
/// placement. This is what makes `$speaker_is_self` fire at the Hall.
#[test]
fn character_identity_beats_placement_identity() {
    let interactable = ambition_interaction::Interactable::new(
        "some_ldtk_placement_iid",
        "Talk",
        ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)),
        ambition_interaction::InteractionKind::Npc {
            character_id: Some("player".into()),
            dialogue_id: Some("hall_player".into()),
            patrol_radius: 0.0,
            patrol_path_id: None,
        },
    );
    let interaction = ActorInteraction {
        interactable,
        talk_radius: 40.0,
    };
    let identity = ActorIdentity::new("some_ldtk_placement_iid", "Player");
    assert_eq!(
        dialogue_identity(Some(&interaction), Some(&identity)).as_deref(),
        Some("player"),
    );
    // A body with no character identity falls back to its placement.
    assert_eq!(
        dialogue_identity(None, Some(&identity)).as_deref(),
        Some("some_ldtk_placement_iid"),
    );
    // The home avatar has neither; the caller supplies its worn character.
    assert_eq!(dialogue_identity(None, None), None);
}
