//! Physical axis sources. The binding catalog selects their actions.

use serde::{Deserialize, Serialize};
use crate::OverrideDeviceClass;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AxisSource {
    KeyboardDirections,
    PadDirections,
    LeftStick,
    RightStick,
    LeftTrigger,
    RightTrigger,
    TouchStick,
}

impl AxisSource {
    pub const fn device_class(self) -> OverrideDeviceClass {
        match self {
            Self::KeyboardDirections => OverrideDeviceClass::Keyboard,
            Self::TouchStick => OverrideDeviceClass::Touch,
            _ => OverrideDeviceClass::Gamepad,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::KeyboardDirections => "Direction keys",
            Self::PadDirections => "D-pad",
            Self::LeftStick => "L-Stick",
            Self::RightStick => "R-Stick",
            Self::LeftTrigger => "LT / L2",
            Self::RightTrigger => "RT / R2",
            Self::TouchStick => "Touch stick",
        }
    }

    #[cfg(feature = "input")]
    pub(crate) fn kind(self) -> leafwing_input_manager::InputControlKind {
        use leafwing_input_manager::InputControlKind;
        match self {
            Self::LeftTrigger | Self::RightTrigger => InputControlKind::Axis,
            _ => InputControlKind::DualAxis,
        }
    }
}

#[cfg(feature = "input")]
mod adapter {
    use bevy::prelude::{App, Entity, Reflect, World};
    use leafwing_input_manager::clashing_inputs::BasicInputs;
    use leafwing_input_manager::prelude::{serde_typetag, Buttonlike, GamepadControlAxis, GamepadStick, InputMap, UserInput, VirtualDPad};
    use leafwing_input_manager::user_input::updating::CentralInputStore;
    use leafwing_input_manager::InputControlKind;
    use serde::{Deserialize, Serialize};
    use crate::{BindingTable, InputAction as Action, InputProfile};
    use super::AxisSource;
    use leafwing_input_manager::prelude::RegisterUserInput;

    /// A direction can have zero or more physical buttons. The same rows
    /// supply its held value and its edge. Empty groups are always released.
    #[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
    struct DirectionButtons(Vec<Box<dyn Buttonlike>>);

    pub(crate) fn register(app: &mut App) {
        app.register_buttonlike_input::<DirectionButtons>();
    }

    impl UserInput for DirectionButtons {
        fn kind(&self) -> InputControlKind { InputControlKind::Button }
        fn decompose(&self) -> BasicInputs { BasicInputs::Composite(self.0.clone()) }
    }

    #[serde_typetag]
    impl Buttonlike for DirectionButtons {
        fn get_pressed(&self, store: &CentralInputStore, pad: Entity) -> Option<bool> {
            Some(self.0.iter().any(|button| button.pressed(store, pad)))
        }
        fn press_as_gamepad(&self, world: &mut World, gamepad: Option<Entity>) {
            if let Some(button) = self.0.first() { button.press_as_gamepad(world, gamepad); }
        }
        fn release_as_gamepad(&self, world: &mut World, gamepad: Option<Entity>) {
            for button in &self.0 { button.release_as_gamepad(world, gamepad); }
        }
    }

    impl AxisSource {
        pub(crate) fn install(self, action: Action, table: &BindingTable, map: &mut InputMap<Action>) {
            match self {
                Self::KeyboardDirections | Self::PadDirections => {
                    let class = self.device_class();
                    let direction = |action| DirectionButtons(table.rows().iter().filter(|row| {
                        row.profile == InputProfile::Gameplay && row.action == action
                            && crate::table::control_class(row.control.as_ref()) == Some(class)
                    }).map(|row| row.control.clone()).collect());
                    if action == Action::Move {
                        map.insert_dual_axis(action, VirtualDPad::new(direction(Action::MoveUp),
                            direction(Action::MoveDown), direction(Action::MoveLeft), direction(Action::MoveRight)));
                    }
                }
                Self::LeftStick => { map.insert_dual_axis(action, GamepadStick::LEFT); }
                Self::RightStick => { map.insert_dual_axis(action, GamepadStick::RIGHT); }
                Self::LeftTrigger => { map.insert_axis(action, GamepadControlAxis::LEFT_Z); }
                Self::RightTrigger => { map.insert_axis(action, GamepadControlAxis::RIGHT_Z); }
                Self::TouchStick => { map.insert_dual_axis(action, crate::touch_device::TouchVirtualStick); }
            }
        }
    }
}

#[cfg(feature = "input")]
pub(crate) use adapter::register;
