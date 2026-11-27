//! Capture state for menus that assign a physical control to an action.

use crate::InputAction;
use bevy::prelude::Component;

/// The action stays armed across changes to the displayed list.
#[derive(Component, Default)]
pub struct RebindCapture {
    action: Option<InputAction>,
    #[cfg(feature = "input")]
    just_armed: bool,
    #[cfg(feature = "input")]
    axis_ready: bool,
}

impl RebindCapture {
    pub fn arm(&mut self, action: InputAction) {
        *self = Self {
            action: Some(action),
            #[cfg(feature = "input")]
            just_armed: true,
            #[cfg(feature = "input")]
            axis_ready: false,
        };
    }

    pub fn armed(&self) -> Option<InputAction> {
        self.action
    }

    pub fn cancel(&mut self) {
        *self = Self::default();
    }
}

#[cfg(feature = "input")]
mod device {
    use super::RebindCapture;
    use crate::{
        settings::ControlSettings, AxisSource, BindingRecipe, InputAction, InputParticipant,
        ParticipantId, PhysicalControl,
    };
    use bevy::ecs::system::SystemParam;
    use bevy::input::gamepad::GamepadAxis;
    use bevy::prelude::*;
    use leafwing_input_manager::prelude::InputMap;

    /// Read raw controls from the devices that belong to the selected participant.
    #[derive(SystemParam)]
    pub struct RebindInputs<'w, 's> {
        keys: Option<Res<'w, ButtonInput<KeyCode>>>,
        pads: Query<'w, 's, &'static Gamepad>,
        participants: Query<
            'w,
            's,
            (
                &'static InputParticipant,
                &'static InputMap<InputAction>,
                &'static mut BindingRecipe,
            ),
        >,
    }

    impl RebindInputs<'_, '_> {
        /// Apply the next compatible press. Call once per frame after menu input.
        /// The primary participant saves the override in client settings.
        /// Other participants store it in their current recipes.
        pub fn poll(
            &mut self,
            owner: ParticipantId,
            capture: &mut RebindCapture,
            mut settings: Mut<ControlSettings>,
        ) {
            let Some(action) = capture.action else {
                return;
            };
            // The press that opens capture can still be in the input state.
            if std::mem::take(&mut capture.just_armed) {
                return;
            }
            let mut participants = self
                .participants
                .iter_mut()
                .filter(|(participant, _, _)| participant.id == owner);
            let Some((_, input, mut recipe)) = participants.next() else {
                capture.cancel();
                return;
            };
            if participants.next().is_some() {
                capture.cancel();
                return;
            }

            let keyboard = self
                .keys
                .as_deref()
                .filter(|_| recipe.sources.admits_keyboard());
            let pad = input
                .gamepad()
                .and_then(|entity| self.pads.get(entity).ok())
                .filter(|_| recipe.sources.admits_gamepad());
            let mut pressed = crate::pressed_controls_this_frame(
                keyboard,
                pad.into_iter()
                    .flat_map(|pad| pad.get_just_pressed().copied()),
            );
            if let Some(pad) = pad {
                CaptureAxes::read(pad).append(capture, &mut pressed);
            }
            if let Some(binding) = crate::capture(&action, pressed) {
                if owner == ParticipantId::PRIMARY {
                    settings.set_binding_override(binding);
                } else {
                    recipe.set_binding_override(binding);
                }
                capture.cancel();
            }
        }
    }

    struct CaptureAxes([(AxisSource, f32); 4]);

    impl CaptureAxes {
        fn read(pad: &Gamepad) -> Self {
            use GamepadAxis as Axis;
            let value = |axis| pad.get(axis).unwrap_or(0.0).abs();
            Self([
                (
                    AxisSource::LeftStick,
                    value(Axis::LeftStickX).max(value(Axis::LeftStickY)),
                ),
                (
                    AxisSource::RightStick,
                    value(Axis::RightStickX).max(value(Axis::RightStickY)),
                ),
                (AxisSource::LeftTrigger, value(Axis::LeftZ)),
                (AxisSource::RightTrigger, value(Axis::RightZ)),
            ])
        }

        fn append(self, capture: &mut RebindCapture, pressed: &mut Vec<PhysicalControl>) {
            if self.0.iter().all(|(_, value)| *value < 0.25) {
                capture.axis_ready = true;
            }
            if capture.axis_ready {
                pressed.extend(
                    self.0
                        .into_iter()
                        .filter(|(_, value)| *value > 0.7)
                        .map(|(axis, _)| PhysicalControl::Axis(axis)),
                );
            }
        }
    }
}
#[cfg(feature = "input")]
pub use device::RebindInputs;
