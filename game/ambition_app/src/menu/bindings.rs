//! System-menu rows from the owning participant's live bindings.

use ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog;
use ambition_platformer2d::input::{
    device_picture, ActionBindings, ActiveDevice, GamepadStyle, PhysicalControl, SeatActiveDevices,
    SeatBindings,
};
use ambition_platformer2d::inventory_ui::InventoryMenu;
use ambition_platformer2d::settings_menu::system::{
    LayoutPage, LayoutSpot, RebindRow, SystemMenuModel,
};
use bevy::prelude::*;

/// A device's published layout (`input_device_*_layout.ron`, the renderer's
/// `input_glyphs` target): where each control sits in the device picture at
/// its full size, keyed by Bevy's control name.
#[derive(Asset, TypePath, serde::Deserialize, Clone, Debug)]
pub(crate) struct DeviceLayout {
    pub size: (f32, f32),
    pub controls: std::collections::BTreeMap<String, LayoutControl>,
}

/// One control's place in a [`DeviceLayout`]. The file also states each
/// control's glyph, shape and box, which this page does not read.
#[derive(serde::Deserialize, Clone, Debug)]
pub(crate) struct LayoutControl {
    pub center: (f32, f32),
}

/// The layout of every device a seat can hold, loaded once, kept alive, by
/// the device's picture path.
#[derive(Resource, Default)]
pub(crate) struct DeviceLayouts(Vec<(String, Handle<DeviceLayout>)>);

/// Load the device layouts the controller-layout screen reads. Only in an app
/// that loads assets; a headless one has no picture to draw.
pub(crate) fn install_device_layouts(app: &mut App) {
    if !app.world().contains_resource::<AssetServer>() {
        return;
    }
    app.add_plugins(bevy_common_assets::ron::RonAssetPlugin::<DeviceLayout>::new(&["ron"]))
        .init_resource::<DeviceLayouts>()
        .add_systems(Startup, load_device_layouts);
}

fn load_device_layouts(
    asset_server: Res<AssetServer>,
    catalog: Option<Res<Platformer2dAssetCatalog>>,
    mut layouts: ResMut<DeviceLayouts>,
) {
    let devices = [
        ActiveDevice::Keyboard,
        ActiveDevice::Gamepad(GamepadStyle::XboxLike),
        ActiveDevice::Gamepad(GamepadStyle::PlayStation),
        ActiveDevice::Gamepad(GamepadStyle::Switch),
        ActiveDevice::Gamepad(GamepadStyle::Generic),
    ];
    for device in devices {
        let (picture, layout) = device_picture(device);
        let path = catalog
            .as_deref()
            .and_then(|catalog| catalog.try_image_path_for_load(&layout))
            .unwrap_or(layout);
        layouts.0.push((picture, asset_server.load::<DeviceLayout>(path)));
    }
}

/// An action's name as words: `MoveDown` reads "Move Down".
fn spaced_words(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut previous_lower = false;
    for c in name.chars() {
        if c.is_uppercase() && previous_lower {
            out.push(' ');
        }
        previous_lower = c.is_lowercase() || c.is_ascii_digit();
        out.push(c);
    }
    out
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MenuBindings<'w, 's> {
    owners: Query<'w, 's, &'static InventoryMenu>,
    bindings: Option<Res<'w, SeatBindings>>,
    devices: Option<Res<'w, SeatActiveDevices>>,
    // The controller-layout screen's pictures; absent in a headless app.
    layouts: (
        Option<Res<'w, DeviceLayouts>>,
        Option<Res<'w, Assets<DeviceLayout>>>,
        Option<Res<'w, Platformer2dAssetCatalog>>,
    ),
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
        let rows = ambition_platformer2d::input::rebind_rows(bindings, device)
            .map(|row| {
                let action = ambition_platformer2d::input::action_name(&row.action);
                RebindRow {
                    label: action,
                    action: row.action,
                    binding: row.binding,
                    also: row.also,
                }
            })
            .collect();
        let model = model.with_rebind(rows);
        match self.layout_page(bindings, device) {
            Some(page) => model.with_layout(page),
            None => model,
        }
    }

    /// The device picture with each action this seat binds on that device
    /// named at its control. `None` until the device's layout has loaded.
    fn layout_page(&self, bindings: &ActionBindings, device: ActiveDevice) -> Option<LayoutPage> {
        let (handles, assets, catalog) = &self.layouts;
        let (picture, _) = device_picture(device);
        let handle = handles
            .as_deref()?
            .0
            .iter()
            .find(|(path, _)| *path == picture)
            .map(|(_, handle)| handle)?;
        let layout = assets.as_deref()?.get(handle)?;
        let image = catalog
            .as_deref()
            .and_then(|catalog| catalog.try_image_path_for_load(&picture))
            .unwrap_or(picture);
        let keyboard = device.draws_keyboard_glyphs();
        // Each control the device has, and the actions on it, in binding order.
        let mut on: Vec<(String, Vec<String>)> = Vec::new();
        for (name, controls) in bindings.all() {
            // The menu's own keys share the gameplay buttons; the page is for play.
            if ambition_platformer2d::input::action_named(name)
                .is_some_and(|action| action.is_menu_only())
            {
                continue;
            }
            let name = spaced_words(name);
            for control in controls {
                let of_device = matches!(
                    (control, keyboard),
                    (PhysicalControl::Key(_), true) | (PhysicalControl::Button(_), false)
                );
                let Some(key) = control.layout_name().filter(|_| of_device) else { continue };
                if !layout.controls.contains_key(&key) {
                    continue;
                }
                match on.iter_mut().find(|(control, _)| *control == key) {
                    Some((_, actions)) if !actions.contains(&name) => actions.push(name.clone()),
                    Some(_) => {}
                    None => on.push((key, vec![name.clone()])),
                }
            }
        }
        let spots = on
            .into_iter()
            .map(|(key, actions)| LayoutSpot {
                label: actions.join(", "),
                center: layout.controls[&key].center,
            })
            .collect();
        Some(LayoutPage {
            image,
            size: layout.size,
            spots,
        })
    }
}

#[cfg(all(test, any(feature = "bevy_ui_menu", feature = "kaleidoscope_menu")))]
mod tests {
    use super::*;
    use crate::menu::kaleidoscope_app::{capture_armed_rebind, RebindCapture};
    use ambition_platformer2d::input::{
        BindingRecipe, BindingSources, InputParticipant, KeyboardPreset, ParticipantId,
        PhysicalControl, InputAction as Action,
    };
    use ambition_platformer2d::persistence::settings::UserSettings;
    use leafwing_input_manager::prelude::{ActionState, InputMap};

    fn arm_jump(app: &mut App, menu: Entity) {
        app.world_mut().get_mut::<RebindCapture>(menu).unwrap().arm(Action::Jump);
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
            let recipe = ambition_input_opinions::recipe(ambition_platformer2d::input::PresetId::ArrowsZxc).with_sources(
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
        arm_jump(&mut app, menus[1]);
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

        arm_jump(&mut app, menus[1]);
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

        arm_jump(&mut app, menus[0]);
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
        arm_jump(&mut app, menus[1]);
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
