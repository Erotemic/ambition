//! Unit coverage for the session-teardown resource reset.

use bevy::prelude::*;

use ambition_platformer_primitives::lifecycle::{SessionScopeId, SessionScopeRetired};

use super::*;
use crate::abilities::traversal::possession::PossessionState;
use crate::boss_encounter::BossEncounterRegistry;
use crate::control::SlotInteractionState;
use crate::encounter::{EncounterRegistry, SwitchActivation, SwitchActivationQueue};
use crate::SandboxSimState;
use ambition_world::collision::MovingPlatformSet;

fn app_with_populated_mirrors() -> App {
    let mut app = App::new();
    app.add_message::<SessionScopeRetired>();
    app.init_resource::<MovingPlatformSet>();
    app.init_resource::<PossessionState>();
    app.init_resource::<ambition_platformer_primitives::markers::ControlledSubject>();
    app.init_resource::<EncounterRegistry>();
    app.init_resource::<crate::encounter::EncounterView>();
    app.init_resource::<BossEncounterRegistry>();
    app.init_resource::<ambition_persistence::quest::QuestRegistry>();
    app.init_resource::<SandboxSimState>();
    app.init_resource::<SlotInteractionState>();
    app.init_resource::<SwitchActivationQueue>();
    app.add_systems(Update, reset_session_scoped_resources_on_retire);

    // Populate the mirrors with distinctive session-A state.
    app.world_mut().resource_mut::<MovingPlatformSet>().0.push(
        ambition_world::platforms::MovingPlatformState::from_authored(
            ambition_engine_core::Vec2::new(10.0, 20.0),
            ambition_engine_core::Vec2::new(32.0, 8.0),
            48.0,
            30.0,
        ),
    );
    let ghost = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<PossessionState>().possessed = Some(ghost);
    app.world_mut()
        .resource_mut::<EncounterRegistry>()
        .ids
        .insert("wave_a".to_owned(), ghost);
    app.world_mut()
        .resource_mut::<SandboxSimState>()
        .room_transition_cooldown = 5.0;
    app.world_mut()
        .resource_mut::<SlotInteractionState>()
        .primary_mut()
        .interact_buffer_timer = 0.75;
    app.world_mut()
        .resource_mut::<SwitchActivationQueue>()
        .0
        .push(SwitchActivation {
            id: "session_a_switch".to_owned(),
            action: "reset".to_owned(),
            target_encounter: "session_a_encounter".to_owned(),
        });
    app
}

#[test]
fn retirement_clears_every_session_scoped_mirror() {
    let mut app = app_with_populated_mirrors();

    // No retirement yet: mirrors keep their session-A state.
    app.update();
    assert_eq!(app.world().resource::<MovingPlatformSet>().0.len(), 1);
    assert!(app
        .world()
        .resource::<PossessionState>()
        .possessed
        .is_some());
    assert!(!app.world().resource::<EncounterRegistry>().ids.is_empty());
    assert!(
        app.world()
            .resource::<SlotInteractionState>()
            .primary()
            .buffered()
    );
    assert_eq!(
        app.world().resource::<SwitchActivationQueue>().0.len(),
        1
    );

    // Retire the scope; the mirrors reset the same frame.
    app.world_mut()
        .write_message(SessionScopeRetired(SessionScopeId(0)));
    app.update();

    assert!(
        app.world().resource::<MovingPlatformSet>().0.is_empty(),
        "moving-platform mirror still holds session-A platforms after teardown"
    );
    assert_eq!(
        app.world().resource::<PossessionState>().possessed,
        None,
        "possession still points at a despawned session-A body after teardown"
    );
    assert!(
        app.world().resource::<EncounterRegistry>().ids.is_empty(),
        "encounter index still maps ids to dead session-A entities after teardown"
    );
    assert_eq!(
        app.world()
            .resource::<SandboxSimState>()
            .room_transition_cooldown,
        SandboxSimState::default().room_transition_cooldown,
        "transient room state carried across teardown"
    );
    assert!(
        !app.world()
            .resource::<SlotInteractionState>()
            .primary()
            .buffered(),
        "slot-level interaction buffer carried across teardown"
    );
    assert!(
        app.world().resource::<SwitchActivationQueue>().0.is_empty(),
        "pending switch activation carried across teardown"
    );
}

#[test]
fn no_retirement_leaves_mirrors_untouched() {
    let mut app = app_with_populated_mirrors();
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(app.world().resource::<MovingPlatformSet>().0.len(), 1);
    assert!(app
        .world()
        .resource::<PossessionState>()
        .possessed
        .is_some());
}
