//! System-menu rows from the owning participant's live bindings.

use ambition_platformer2d::input::{ActionBindings, ActiveDevice, SeatActiveDevices, SeatBindings};
use ambition_platformer2d::inventory_ui::InventoryMenu;
use ambition_platformer2d::settings_menu::system::{RebindRow, SystemMenuModel};
use bevy::prelude::*;

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MenuBindings<'w, 's> {
    owners: Query<'w, 's, &'static InventoryMenu>,
    bindings: Option<Res<'w, SeatBindings>>,
    devices: Option<Res<'w, SeatActiveDevices>>,
}

impl MenuBindings<'_, '_> {
    pub(crate) fn state(&self, menu: Entity) -> Option<(&ActionBindings, ActiveDevice)> {
        let owner = self.owners.get(menu).ok()?.owner.slot();
        let device = self
            .devices
            .as_deref()
            .map_or_else(ActiveDevice::default, |devices| devices.for_seat(owner));
        let device = if device.draws_keyboard_glyphs() {
            ActiveDevice::Keyboard
        } else {
            ActiveDevice::Gamepad(device.gamepad_style())
        };
        Some((self.bindings.as_deref()?.for_seat(owner), device))
    }

    pub(crate) fn attach(&self, model: SystemMenuModel, menu: Entity) -> SystemMenuModel {
        let Some((bindings, device)) = self.state(menu) else {
            return model;
        };
        let rows = bindings
            .all()
            .filter_map(|(name, controls)| {
                let action = ambition_platformer2d::input::action_named(name)?;
                let control = bindings
                    .control_for(&action, device)
                    .or_else(|| controls.first());
                Some(RebindRow {
                    action: name.to_string(),
                    label: name.to_string(),
                    binding: bindings.label_for(&action, device).unwrap_or_default(),
                    also: control
                        .and_then(ambition_platformer2d::input::bindable)
                        .map_or_else(Vec::new, |control| {
                            ambition_platformer2d::input::also_bound_to(bindings, &action, control)
                        }),
                })
            })
            .collect();
        model.with_rebind(rows)
    }
}

#[cfg(all(test, any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu")))]
mod tests {
    use super::*;
    use crate::menu::kaleidoscope_app::{capture_armed_rebind, RebindCapture};
    use ambition_platformer2d::input::{
        BindingRecipe, BindingSources, InputParticipant, KeyboardPreset, ParticipantId,
        PhysicalControl, Platformer2dInputActionMonolith as Action,
    };
    use ambition_platformer2d::persistence::settings::UserSettings;
    use leafwing_input_manager::prelude::{ActionState, InputMap};

    fn arm_jump(app: &mut App, menu: Entity, owner: ParticipantId) {
        let row = app
            .world()
            .resource::<SeatBindings>()
            .for_seat(owner.slot())
            .all()
            .position(|(name, _)| name == "Jump")
            .unwrap();
        app.world_mut()
            .get_mut::<RebindCapture>(menu)
            .unwrap()
            .arm(row);
        app.update();
    }

    fn clear_presses(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        for mut pad in app
            .world_mut()
            .query::<&mut Gamepad>()
            .iter_mut(app.world_mut())
        {
            pad.digital_mut().clear();
        }
    }

    #[test]
    fn rebind_capture_follows_the_owners_devices_and_changes_only_that_profile() {
        let mut app = App::new();
        app.init_resource::<SeatBindings>()
            .init_resource::<UserSettings>()
            .init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(
            Update,
            (
                ambition_platformer2d::runtime::input::sync_primary_recipe_from_settings,
                ambition_platformer2d::input::rebuild_maps_from_recipes,
                ambition_platformer2d::input::publish_seat_bindings,
            )
                .chain(),
        );
        app.add_systems(PostUpdate, capture_armed_rebind);
        let pads = [0, 1].map(|_| app.world_mut().spawn(Gamepad::default()).id());
        let people = [ParticipantId::PRIMARY, ParticipantId::SECONDARY].map(|owner| {
            let recipe = BindingRecipe::preset(KeyboardPreset::arrows_zxc().id).with_sources(
                if owner == ParticipantId::PRIMARY {
                    BindingSources::KeyboardOnly
                } else {
                    BindingSources::GamepadOnly
                },
            );
            let mut map = recipe.build();
            if owner == ParticipantId::SECONDARY {
                map.set_gamepad(pads[0]);
            }
            app.world_mut()
                .spawn((
                    InputParticipant::with_id(owner),
                    recipe,
                    map,
                    ActionState::<Action>::default(),
                ))
                .id()
        });
        let menus = [ParticipantId::PRIMARY, ParticipantId::SECONDARY].map(|owner| {
            app.world_mut()
                .spawn((
                    crate::menu::MenuInstance,
                    InventoryMenu { owner },
                    ambition_platformer2d::inventory_ui::InventoryUiState {
                        visible: true,
                        ..default()
                    },
                ))
                .id()
        });
        app.update();
        arm_jump(&mut app, menus[1], ParticipantId::SECONDARY);
        clear_presses(&mut app);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F13);
        app.world_mut()
            .get_mut::<Gamepad>(pads[1])
            .unwrap()
            .digital_mut()
            .press(GamepadButton::North);
        app.update();
        assert!(app
            .world()
            .get::<RebindCapture>(menus[1])
            .unwrap()
            .armed()
            .is_some());
        assert!(app
            .world()
            .resource::<UserSettings>()
            .controls
            .binding_overrides
            .is_empty());
        clear_presses(&mut app);
        app.world_mut()
            .get_mut::<Gamepad>(pads[0])
            .unwrap()
            .digital_mut()
            .press(GamepadButton::East);
        app.update();
        clear_presses(&mut app);
        app.update();
        assert_eq!(
            app.world()
                .resource::<SeatBindings>()
                .for_seat(1)
                .controls(&Action::Jump),
            &[PhysicalControl::Button(GamepadButton::East)]
        );
        assert!(app
            .world()
            .get::<RebindCapture>(menus[1])
            .unwrap()
            .armed()
            .is_none());
        assert!(app
            .world()
            .resource::<UserSettings>()
            .controls
            .binding_overrides
            .is_empty());
        assert!(app
            .world()
            .get::<BindingRecipe>(people[0])
            .unwrap()
            .overrides
            .is_empty());

        arm_jump(&mut app, menus[1], ParticipantId::SECONDARY);
        app.world_mut()
            .get_mut::<InputMap<Action>>(people[1])
            .unwrap()
            .set_gamepad(pads[1]);
        clear_presses(&mut app);
        app.world_mut()
            .get_mut::<Gamepad>(pads[0])
            .unwrap()
            .digital_mut()
            .press(GamepadButton::North);
        app.update();
        assert!(app
            .world()
            .get::<RebindCapture>(menus[1])
            .unwrap()
            .armed()
            .is_some());
        clear_presses(&mut app);
        app.world_mut()
            .get_mut::<Gamepad>(pads[1])
            .unwrap()
            .digital_mut()
            .press(GamepadButton::West);
        app.update();
        clear_presses(&mut app);
        app.update();
        assert_eq!(
            app.world()
                .resource::<SeatBindings>()
                .for_seat(1)
                .controls(&Action::Jump),
            &[PhysicalControl::Button(GamepadButton::West)]
        );

        arm_jump(&mut app, menus[0], ParticipantId::PRIMARY);
        clear_presses(&mut app);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F14);
        app.update();
        clear_presses(&mut app);
        app.update();
        assert_eq!(
            app.world()
                .resource::<SeatBindings>()
                .for_seat(0)
                .controls(&Action::Jump),
            &[PhysicalControl::Key(KeyCode::F14)]
        );
        assert_eq!(
            app.world()
                .resource::<SeatBindings>()
                .for_seat(1)
                .controls(&Action::Jump),
            &[PhysicalControl::Button(GamepadButton::West)]
        );
        assert_eq!(
            app.world()
                .resource::<UserSettings>()
                .controls
                .binding_overrides
                .len(),
            1
        );
        arm_jump(&mut app, menus[1], ParticipantId::SECONDARY);
        app.world_mut()
            .get_mut::<ambition_platformer2d::inventory_ui::InventoryUiState>(menus[1])
            .unwrap()
            .visible = false;
        app.update();
        assert!(app
            .world()
            .get::<RebindCapture>(menus[1])
            .unwrap()
            .armed()
            .is_none());
    }
}
