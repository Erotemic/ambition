//! Confirm prompts for the primary menu on the host frame clock.

use super::*;
use ambition_platformer2d::input::{InputSet, ParticipantId};
use ambition_platformer2d::inventory_ui::{InventoryInputContextPlugin, InventoryMenu, InventoryUiState};
use ambition_platformer2d::platformer::schedule::{GameMode, SimScheduleExt};
use ambition_platformer2d::sim_view::{ControlContextKind, ControlPrompt};

fn prompt_fixture() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameMode>();
    app.init_resource::<ControlPrompt>();
    let items = ambition_platformer2d::items::test_item_catalog();
    app.insert_resource(items.clone());
    let mut owned = OwnedItems::default();
    owned.grant(items, Item::GunSword, 1);
    owned.grant(items, Item::HealthCell, 1);
    app.insert_resource(owned);
    let mut cursor = KaleidoscopeCursor::default();
    cursor.mark_keyboard(MenuFocus::Item(Item::ALL.iter().position(|&i| i == Item::GunSword).unwrap()));
    let primary = app.world_mut().spawn((
        crate::menu::MenuInstance,
        InventoryMenu { owner: ParticipantId::PRIMARY },
        InventoryUiState { visible: true, ..default() },
        cursor,
    )).id();
    let mut secondary_cursor = KaleidoscopeCursor::default();
    secondary_cursor.mark_keyboard(MenuFocus::Item(Item::ALL.iter().position(|&i| i == Item::HealthCell).unwrap()));
    app.world_mut().spawn((
        crate::menu::MenuInstance,
        InventoryMenu { owner: ParticipantId::SECONDARY },
        InventoryUiState { visible: true, ..default() },
        secondary_cursor,
    ));
    app.world_mut().resource_mut::<NextState<GameMode>>().set(GameMode::Paused);
    (app, primary)
}

#[test]
fn the_shared_prompt_follows_the_primary_menu_and_clears_when_it_retires() {
    let (mut app, primary) = prompt_fixture();
    app.set_sim_schedule(Update);
    app.add_systems(Update, ambition_platformer2d::sim_view::rebuild_control_prompt
        .in_set(ambition_platformer2d::sim_view::ControlPromptRebuilt));
    super::install_menu_confirm_provider(&mut app);
    app.update();
    let prompt = app.world().resource::<ControlPrompt>();
    assert_eq!(prompt.context, ControlContextKind::Menu);
    assert_eq!(prompt.menu_confirm.as_deref(), Some("Equip"));

    app.world_mut().get_mut::<InventoryUiState>(primary).unwrap().close();
    app.update();
    assert_eq!(app.world().resource::<ControlPrompt>().menu_confirm.as_deref(), Some("Select"));

    app.world_mut().get_mut::<InventoryUiState>(primary).unwrap().visible = true;
    app.update();
    assert_eq!(app.world().resource::<ControlPrompt>().menu_confirm.as_deref(), Some("Equip"));
    app.world_mut().entity_mut(primary).despawn();
    app.update();
    assert_eq!(app.world().resource::<ControlPrompt>().menu_confirm.as_deref(), Some("Select"));
}

#[test]
fn an_inventory_context_gets_its_verb_without_a_simulation_tick() {
    use ambition_platformer2d::input::participant::{
        resolve_active_input_context, InputParticipant, ParticipantContexts, SeatInputContexts,
    };
    let (mut app, primary) = prompt_fixture();
    app.set_sim_schedule(FixedUpdate);
    app.init_resource::<SeatInputContexts>();
    app.world_mut().spawn((InputParticipant::primary(), ParticipantContexts::default()));
    app.add_plugins(InventoryInputContextPlugin);
    app.configure_sets(Update, (
        InputSet::ResolveContext, InputSet::Route, InputSet::PublishCues, InputSet::Consume,
    ).chain());
    app.add_systems(Update, resolve_active_input_context
        .after(InputSet::ResolveContext).before(InputSet::Route));
    app.add_systems(Update, ambition_platformer2d::sim_view::publish_frontend_context_prompt
        .after(InputSet::PublishCues).before(InputSet::Consume));
    super::install_menu_confirm_provider(&mut app);
    app.update();
    assert_eq!(app.world().resource::<ControlPrompt>().menu_confirm.as_deref(), Some("Equip"));

    app.world_mut().get_mut::<KaleidoscopeCursor>(primary).unwrap()
        .mark_keyboard(MenuFocus::Item(Item::ALL.iter().position(|&i| i == Item::HealthCell).unwrap()));
    app.update();
    assert_eq!(app.world().resource::<ControlPrompt>().menu_confirm.as_deref(), Some("Use"));
}
