//! Leafwing input kinds for the raw virtual touch device. No default actions.

use bevy::ecs::system::lifetimeless::SRes;
use bevy::ecs::system::StaticSystemParam;
use bevy::prelude::{Entity, Reflect, ResMut, Vec2, World};
use leafwing_input_manager::buttonlike::ButtonValue;
use leafwing_input_manager::clashing_inputs::BasicInputs;
use leafwing_input_manager::prelude::updating::{CentralInputStore, UpdatableInput};
use leafwing_input_manager::prelude::{serde_typetag, Buttonlike, DualAxislike, UserInput};
use leafwing_input_manager::InputControlKind;
use serde::{Deserialize, Serialize};

use crate::touch_state::{MobileTouchState, TouchActionButton, TouchButton, TouchStickDirection};

/// Stick deflection past this magnitude is a held direction. Same threshold as
/// the gamepad's `GamepadControlDirection` bindings (`STICK_DIRECTION_THRESHOLD`
/// in the presets), so touch and pad flicks give the same press edges.
const DIRECTION_THRESHOLD: f32 = 0.5;

/// One on-screen touch button as a bindable virtual button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub struct TouchVirtualButton(pub TouchActionButton);

impl UserInput for TouchVirtualButton {
    fn kind(&self) -> InputControlKind {
        InputControlKind::Button
    }

    fn decompose(&self) -> BasicInputs {
        BasicInputs::Simple(Box::new(*self))
    }
}

impl UpdatableInput for TouchVirtualButton {
    type SourceData = SRes<MobileTouchState>;

    fn compute(
        mut central_input_store: ResMut<CentralInputStore>,
        source_data: StaticSystemParam<Self::SourceData>,
    ) {
        for action in ALL_TOUCH_BUTTONS {
            let held = touch_button_state(&source_data, action).held;
            central_input_store
                .update_buttonlike(TouchVirtualButton(action), ButtonValue::from_pressed(held));
        }
    }
}

#[serde_typetag]
impl Buttonlike for TouchVirtualButton {
    fn get_pressed(&self, input_store: &CentralInputStore, _gamepad: Entity) -> Option<bool> {
        input_store.pressed(self)
    }

    /// Test/mocking seam: press the underlying touch state directly.
    fn press(&self, world: &mut World) {
        set_touch_button(world, self.0, true);
    }

    fn release(&self, world: &mut World) {
        set_touch_button(world, self.0, false);
    }
}

/// The touch move stick as a bindable dual-axis. Published in leafwing's
/// +Y-up convention (the touch state stores +Y-down screen space), matching
/// `GamepadStick::LEFT` so every downstream reader treats them identically.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub struct TouchVirtualStick;

impl UserInput for TouchVirtualStick {
    fn kind(&self) -> InputControlKind {
        InputControlKind::DualAxis
    }

    fn decompose(&self) -> BasicInputs {
        BasicInputs::Composite(vec![
            Box::new(TouchStickDirection::Up),
            Box::new(TouchStickDirection::Down),
            Box::new(TouchStickDirection::Left),
            Box::new(TouchStickDirection::Right),
        ])
    }
}

impl UpdatableInput for TouchVirtualStick {
    type SourceData = SRes<MobileTouchState>;

    fn compute(
        mut central_input_store: ResMut<CentralInputStore>,
        source_data: StaticSystemParam<Self::SourceData>,
    ) {
        let state = source_data.0;
        central_input_store
            .update_dualaxislike(TouchVirtualStick, Vec2::new(state.move_x, -state.move_y));
    }
}

#[serde_typetag]
impl DualAxislike for TouchVirtualStick {
    fn get_axis_pair(&self, input_store: &CentralInputStore, _gamepad: Entity) -> Option<Vec2> {
        input_store.pair(self)
    }

    /// Test/mocking seam: drive the underlying touch stick (value arrives in
    /// leafwing's +Y-up convention and is stored back as +Y-down).
    fn set_axis_pair(&self, world: &mut World, value: Vec2) {
        let mut state = world.resource_mut::<MobileTouchState>();
        state.0.move_x = value.x;
        state.0.move_y = -value.y;
    }
}

/// The touch stick's cardinal directions as threshold buttons: the source of
/// the discrete `MoveLeft/Right/Up/Down` edges (leafwing derives the edge from
/// the held transition, as for a pad stick). Leafwing convention: `Up` means
/// the stick is pushed up.

impl UserInput for TouchStickDirection {
    fn kind(&self) -> InputControlKind {
        InputControlKind::Button
    }

    fn decompose(&self) -> BasicInputs {
        BasicInputs::Simple(Box::new(*self))
    }
}

impl UpdatableInput for TouchStickDirection {
    type SourceData = SRes<MobileTouchState>;

    fn compute(
        mut central_input_store: ResMut<CentralInputStore>,
        source_data: StaticSystemParam<Self::SourceData>,
    ) {
        let state = source_data.0;
        // Touch state is +Y-down: pushing the stick up is negative move_y.
        let held = [
            (
                TouchStickDirection::Up,
                -state.move_y >= DIRECTION_THRESHOLD,
            ),
            (
                TouchStickDirection::Down,
                state.move_y >= DIRECTION_THRESHOLD,
            ),
            (
                TouchStickDirection::Left,
                -state.move_x >= DIRECTION_THRESHOLD,
            ),
            (
                TouchStickDirection::Right,
                state.move_x >= DIRECTION_THRESHOLD,
            ),
        ];
        for (direction, held) in held {
            central_input_store.update_buttonlike(direction, ButtonValue::from_pressed(held));
        }
    }
}

#[serde_typetag]
impl Buttonlike for TouchStickDirection {
    fn get_pressed(&self, input_store: &CentralInputStore, _gamepad: Entity) -> Option<bool> {
        input_store.pressed(self)
    }

    /// Test/mocking seam: deflect the underlying touch stick fully in this
    /// direction (release recenters that axis).
    fn press(&self, world: &mut World) {
        let mut state = world.resource_mut::<MobileTouchState>();
        match self {
            TouchStickDirection::Up => state.0.move_y = -1.0,
            TouchStickDirection::Down => state.0.move_y = 1.0,
            TouchStickDirection::Left => state.0.move_x = -1.0,
            TouchStickDirection::Right => state.0.move_x = 1.0,
        }
    }

    fn release(&self, world: &mut World) {
        let mut state = world.resource_mut::<MobileTouchState>();
        match self {
            TouchStickDirection::Up | TouchStickDirection::Down => state.0.move_y = 0.0,
            TouchStickDirection::Left | TouchStickDirection::Right => state.0.move_x = 0.0,
        }
    }
}

pub const ALL_TOUCH_BUTTONS: [TouchActionButton; 13] = [
    TouchActionButton::Jump,
    TouchActionButton::Attack,
    TouchActionButton::Special,
    TouchActionButton::Burst,
    TouchActionButton::Blink,
    TouchActionButton::Interact,
    TouchActionButton::Projectile,
    TouchActionButton::FlyToggle,
    TouchActionButton::Shield,
    TouchActionButton::Grab,
    TouchActionButton::Modifier,
    TouchActionButton::Start,
    TouchActionButton::Reset,
];

fn touch_button_state(state: &MobileTouchState, action: TouchActionButton) -> TouchButton {
    match action {
        TouchActionButton::Jump => state.0.jump,
        TouchActionButton::Attack => state.0.attack,
        TouchActionButton::Special => state.0.special,
        TouchActionButton::Burst => state.0.burst,
        TouchActionButton::Blink => state.0.blink,
        TouchActionButton::Interact => state.0.interact,
        TouchActionButton::Projectile => state.0.projectile,
        TouchActionButton::FlyToggle => state.0.fly_toggle,
        TouchActionButton::Shield => state.0.shield,
        TouchActionButton::Grab => state.0.grab,
        TouchActionButton::Modifier => state.0.modifier,
        TouchActionButton::Start => state.0.start,
        TouchActionButton::Reset => state.0.reset,
    }
}

fn set_touch_button(world: &mut World, action: TouchActionButton, held: bool) {
    let mut state = world.resource_mut::<MobileTouchState>();
    let button = match action {
        TouchActionButton::Jump => &mut state.0.jump,
        TouchActionButton::Attack => &mut state.0.attack,
        TouchActionButton::Special => &mut state.0.special,
        TouchActionButton::Burst => &mut state.0.burst,
        TouchActionButton::Blink => &mut state.0.blink,
        TouchActionButton::Interact => &mut state.0.interact,
        TouchActionButton::Projectile => &mut state.0.projectile,
        TouchActionButton::FlyToggle => &mut state.0.fly_toggle,
        TouchActionButton::Shield => &mut state.0.shield,
        TouchActionButton::Grab => &mut state.0.grab,
        TouchActionButton::Modifier => &mut state.0.modifier,
        TouchActionButton::Start => &mut state.0.start,
        TouchActionButton::Reset => &mut state.0.reset,
    };
    let was_held = button.held;
    *button = TouchButton {
        held,
        pressed_this_frame: held && !was_held,
        released_this_frame: !held && was_held,
    };
}

