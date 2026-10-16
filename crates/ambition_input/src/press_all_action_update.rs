//! Leafwing's `update_action_state`, without the clash pass where it does nothing.
//!
//! `InputMap::process_actions` (leafwing-input-manager 0.21) ends with
//! `handle_clashes`, which builds every pair of buttonlike actions and splits
//! each binding into new `Vec`s, on every call. Under `ClashStrategy::PressAll`
//! a clash removes nothing (`resolve_clash` returns `None` on every path), so
//! the pass has no effect. Measured (`tests/leafwing_allocations.rs`): one call
//! for a seat of this game makes 9,611 allocations, 9,522 of them in
//! `handle_clashes`; the system runs twice a frame (frame and fixed loop), and
//! this was three quarters of a standalone game's allocations.
//!
//! [`install_press_all_action_update`] removes leafwing's two
//! `update_action_state::<A>` systems and adds [`update_action_state_press_all`]
//! in their places. Under any other strategy it calls `process_actions`, so the
//! result is the same for every strategy. [`actions_without_clash_pass`] is the
//! first part of `process_actions`, copied; `a_seat_resolves_as_leafwing_does`
//! compares the two. Remove this module when leafwing skips the pass itself.

use bevy::app::{App, PreUpdate, RunFixedMainLoop, RunFixedMainLoopSystems};
use bevy::ecs::schedule::ScheduleCleanupPolicy;
use bevy::input::gamepad::Gamepad;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::*;
use leafwing_input_manager::buttonlike::ButtonValue;
use leafwing_input_manager::input_map::{UpdatedActions, UpdatedValue};
use leafwing_input_manager::plugin::InputManagerSystem;
use leafwing_input_manager::prelude::{ActionState, ClashStrategy, InputMap};
use leafwing_input_manager::systems::{swap_to_fixed_update, update_action_state};
use leafwing_input_manager::user_input::gamepad::find_gamepad;
use leafwing_input_manager::user_input::updating::CentralInputStore;
use leafwing_input_manager::Actionlike;

/// Replace leafwing's `update_action_state::<A>` in both of its places. Call it
/// after `InputManagerPlugin::<A>` is added.
pub fn install_press_all_action_update<A: Actionlike>(app: &mut App) {
    for removed in [
        app.remove_systems_in_set(PreUpdate, update_action_state::<A>, ScheduleCleanupPolicy::RemoveSystemsOnly),
        app.remove_systems_in_set(
            RunFixedMainLoop,
            update_action_state::<A>,
            ScheduleCleanupPolicy::RemoveSystemsOnly,
        ),
    ] {
        assert_eq!(
            removed.ok(),
            Some(1),
            "leafwing's update_action_state is not where this module expects it; \
             check the leafwing version",
        );
    }
    app.add_systems(PreUpdate, update_action_state_press_all::<A>.in_set(InputManagerSystem::Update));
    app.add_systems(
        RunFixedMainLoop,
        update_action_state_press_all::<A>
            .after(swap_to_fixed_update::<A>)
            .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
    );
}

/// `leafwing_input_manager::systems::update_action_state`, with the clash pass
/// only where it can change the result.
pub fn update_action_state_press_all<A: Actionlike>(
    input_store: Res<CentralInputStore>,
    clash_strategy: Res<ClashStrategy>,
    mut gamepads: Query<Entity, With<Gamepad>>,
    mut query: Query<(&mut ActionState<A>, &InputMap<A>)>,
) {
    for (mut action_state, input_map) in query.iter_mut() {
        let updated = if *clash_strategy == ClashStrategy::PressAll {
            actions_without_clash_pass(input_map, Some(gamepads.reborrow()), &input_store)
        } else {
            input_map.process_actions(Some(gamepads.reborrow()), &input_store, *clash_strategy)
        };
        action_state.update(updated);
    }
}

/// `InputMap::process_actions` of leafwing-input-manager 0.21 up to its clash
/// pass: the value of each action from its bindings.
pub fn actions_without_clash_pass<A: Actionlike>(
    map: &InputMap<A>,
    gamepads: Option<Query<Entity, With<Gamepad>>>,
    input_store: &CentralInputStore,
) -> UpdatedActions<A> {
    let mut updated_actions = UpdatedActions::default();
    let gamepad = map.gamepad().unwrap_or(find_gamepad(gamepads));

    for (action, input_bindings) in map.iter_buttonlike() {
        let mut final_state: Option<(bool, f32)> = None;
        for binding in input_bindings {
            let pressed = binding.pressed(input_store, gamepad);
            let value = binding.value(input_store, gamepad);
            final_state = Some(match final_state {
                Some((existing_state, existing_value)) => (existing_state || pressed, existing_value.max(value)),
                None => (pressed, value),
            });
        }
        if let Some((pressed, value)) = final_state {
            updated_actions.insert(action.clone(), UpdatedValue::Button(ButtonValue { pressed, value }));
        }
    }

    for (action, input_bindings) in map.iter_axislike() {
        let mut final_value = None;
        for binding in input_bindings {
            if let Some(value) = binding.get_value(input_store, gamepad) {
                final_value = Some(final_value.unwrap_or(0.0) + value);
            }
        }
        if let Some(final_value) = final_value {
            updated_actions.insert(action.clone(), UpdatedValue::Axis(final_value));
        }
    }

    for (action, input_bindings) in map.iter_dual_axislike() {
        let mut final_value = None;
        for binding in input_bindings {
            if let Some(axis_pair) = binding.get_axis_pair(input_store, gamepad) {
                final_value = Some(final_value.unwrap_or(Vec2::ZERO) + axis_pair);
            }
        }
        if let Some(final_value) = final_value {
            updated_actions.insert(action.clone(), UpdatedValue::DualAxis(final_value));
        }
    }

    for (action, input_bindings) in map.iter_triple_axislike() {
        let mut final_value = Vec3::ZERO;
        for binding in input_bindings {
            final_value += binding.axis_triple(input_store, gamepad);
        }
        updated_actions.insert(action.clone(), UpdatedValue::TripleAxis(final_value));
    }

    updated_actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BindingRecipe, Platformer2dInputActionMonolith as Action, PresetId};
    use leafwing_input_manager::plugin::InputManagerPlugin;

    /// The copy gives what `process_actions` gives under `PressAll`, with keys
    /// held that make two actions share a key (a clash `PressAll` keeps).
    #[test]
    fn a_seat_resolves_as_leafwing_does() {
        let mut app = App::new();
        app.add_plugins((bevy::time::TimePlugin, bevy::input::InputPlugin, InputManagerPlugin::<Action>::default()));
        for preset in [PresetId::ArrowsZxc, PresetId::WasdJkl] {
            let map = BindingRecipe::preset(preset).build();
            for keys in [
                vec![],
                vec![KeyCode::KeyZ],
                vec![KeyCode::KeyX, KeyCode::ArrowLeft, KeyCode::ShiftLeft],
                vec![KeyCode::KeyJ, KeyCode::KeyK, KeyCode::KeyA, KeyCode::Space],
                vec![KeyCode::KeyC, KeyCode::KeyV, KeyCode::ArrowDown, KeyCode::ArrowUp],
            ] {
                {
                    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                    input.release_all();
                    for key in &keys {
                        input.press(*key);
                    }
                }
                app.update();
                let store = app.world().resource::<CentralInputStore>();
                assert_eq!(
                    actions_without_clash_pass(&map, None, store),
                    map.process_actions(None, store, ClashStrategy::PressAll),
                    "{preset:?} with {keys:?} held",
                );
            }
        }
    }

    /// Installed, the replacement is the only `update_action_state` in both
    /// schedules, and an App still updates.
    #[test]
    fn the_replacement_takes_both_places() {
        let mut app = App::new();
        app.add_plugins((bevy::time::TimePlugin, bevy::input::InputPlugin, InputManagerPlugin::<Action>::default()));
        install_press_all_action_update::<Action>(&mut app);
        app.update();
        assert_eq!(
            app.remove_systems_in_set(PreUpdate, update_action_state::<Action>, ScheduleCleanupPolicy::RemoveSystemsOnly)
                .ok()
                .unwrap_or(0),
            0,
        );
    }
}
